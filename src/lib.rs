pub mod alert;
pub mod config;
pub mod data;
pub mod db;
pub mod indicators;
pub mod yahoo;

use std::sync::Arc;
use std::time::Duration;

use config::load_config;
use data::extract_ohlcv;
use db::PriceDb;
use indicators::Indicator;
use tokio::sync::Mutex;

pub use config::{
    BollingerBandsConfig, CrossoverConfig, DaemonConfig, IndicatorConfig, MacdConfig, RsiConfig,
    TickerConfig,
};
pub use data::{MarketData, OhlcvRow, extract_prices};
pub use indicators::IndicatorResult;
pub use yahoo::{YFQuote, build_chart_url};

fn resolve_db_path(config: &DaemonConfig) -> std::path::PathBuf {
    if let Some(ref path) = config.db_path {
        return std::path::PathBuf::from(path);
    }
    if let Ok(db_path) = std::env::var("DB_PATH")
        && !db_path.trim().is_empty()
    {
        return std::path::PathBuf::from(db_path.trim());
    }
    if let Ok(state_dir) = std::env::var("STATE_DIRECTORY") {
        let dir = std::path::PathBuf::from(state_dir);
        if !dir.as_os_str().is_empty() {
            return dir.join("data.db");
        }
    }
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        let dir = std::path::PathBuf::from(data_home).join("indicator-alert-daemon");
        return dir.join("data.db");
    }
    if let Ok(home) = std::env::var("HOME") {
        let dir = std::path::PathBuf::from(home).join(".local/share/indicator-alert-daemon");
        return dir.join("data.db");
    }
    std::path::PathBuf::from("data.db")
}

fn compute_retain_count(config: &DaemonConfig) -> i64 {
    let max_bars = config
        .tickers
        .iter()
        .flat_map(|t| t.indicators.iter())
        .map(|i| i.required_bars())
        .max()
        .unwrap_or(0) as i64;

    let months_bars = match config.interval_type.as_str() {
        "1d" => 36 * 30,
        "1wk" => 36 * 4,
        _ => 36 * 30,
    };

    let retain = std::cmp::min(max_bars * 2, months_bars);
    std::cmp::max(retain, 50)
}

async fn fetch_yahoo_chart(
    client: &reqwest::Client,
    url: &str,
    max_retries: u32,
    retry_base_delay_ms: u64,
) -> Result<yahoo::YFResponse, String> {
    let mut attempt = 0u32;
    loop {
        match client
            .get(url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
        {
            Ok(resp) => {
                return resp
                    .json::<yahoo::YFResponse>()
                    .await
                    .map_err(|e| format!("Failed to parse response: {e}"));
            }
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(format!("Network error after {max_retries} retries: {e}"));
                }
                let delay = Duration::from_millis(retry_base_delay_ms * 2u64.pow(attempt));
                tracing::warn!(
                    attempt,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Retrying Yahoo Finance fetch"
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

pub async fn run_loop(config_path: &str) {
    let config = load_config(config_path).expect("Failed to load configuration");
    let config = Arc::new(config);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let db_path = resolve_db_path(&config);
    let db = PriceDb::new(&db_path).expect("Failed to open database");
    let db = Arc::new(Mutex::new(db));

    tracing::info!(path = %db_path.display(), "Database opened");

    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Failed to install SIGTERM handler");
    let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .expect("Failed to install SIGINT handler");

    loop {
        tokio::select! {
            _ = term.recv() => {
                tracing::info!("Received SIGTERM, shutting down");
                break;
            }
            _ = int.recv() => {
                tracing::info!("Received SIGINT, shutting down");
                break;
            }
            _ = async {
                evaluate_market(config.clone(), client.clone(), &db).await;
                tokio::time::sleep(Duration::from_secs(config.frequency_seconds)).await;
            } => {}
        }
    }

    tracing::info!("Shutdown complete");
}

#[tracing::instrument(skip_all)]
pub async fn evaluate_market(
    config: Arc<DaemonConfig>,
    client: reqwest::Client,
    db: &Mutex<PriceDb>,
) {
    tracing::info!("Beginning cycle evaluation across targets");

    let mut total_indicators = 0u32;
    let mut total_triggered = 0u32;
    let mut total_failures = 0u32;

    for ticker in &config.tickers {
        tokio::time::sleep(Duration::from_millis(config.ticker_delay_ms)).await;

        let need_full_fetch = {
            let db = db.lock().await;
            db.latest_timestamp(&ticker.symbol, &config.interval_type)
                .unwrap_or(None)
                .is_none()
        };

        let url = build_chart_url(&ticker.symbol, &config.interval_type, need_full_fetch);

        let yf_resp = match fetch_yahoo_chart(
            &client,
            &url,
            config.max_retries,
            config.retry_base_delay_ms,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(symbol = %ticker.symbol, error = %e, "Failed to fetch chart data");
                total_failures += 1;
                continue;
            }
        };

        let results = match yf_resp.chart.result {
            Some(r) if !r.is_empty() => r,
            _ => {
                tracing::warn!(symbol = %ticker.symbol, "No chart data returned");
                total_failures += 1;
                continue;
            }
        };

        let chart_result = &results[0];
        let timestamps = &chart_result.timestamp;

        let quote = match chart_result.indicators.quote.first() {
            Some(q) => q,
            None => {
                tracing::warn!(symbol = %ticker.symbol, "No quote data returned");
                total_failures += 1;
                continue;
            }
        };

        let ohlcv_rows = extract_ohlcv(timestamps, quote);

        {
            let db = db.lock().await;
            if let Err(e) = db.insert_ohlcv(&ticker.symbol, &config.interval_type, &ohlcv_rows) {
                tracing::error!(symbol = %ticker.symbol, error = %e, "DB insert error");
            }
        }

        let prices = {
            let db = db.lock().await;
            db.load_close_prices(&ticker.symbol, &config.interval_type)
                .unwrap_or_default()
        };

        if prices.len() < 2 {
            continue;
        }

        let current_price = prices[prices.len() - 1];
        let mut log_parts = vec![
            format!("[{}]", ticker.symbol),
            format!("Price: ${:.2}", current_price),
        ];

        let market_data = MarketData {
            symbol: ticker.symbol.clone(),
            close_prices: prices,
            interval_type: config.interval_type.clone(),
        };

        for indicator_cfg in &ticker.indicators {
            total_indicators += 1;
            let result = indicator_cfg.evaluate(&market_data);
            log_parts.push(result.log_label);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let config_json = serde_json::to_string(indicator_cfg).unwrap_or_default();

            let should_alert = {
                let db = db.lock().await;

                let prev = db
                    .last_indicator_result(&ticker.symbol, indicator_cfg.name(), &config_json)
                    .unwrap_or(None);

                let _ = db.store_indicator_result(
                    &ticker.symbol,
                    indicator_cfg.name(),
                    &config_json,
                    now,
                    None,
                    None,
                    result.triggered,
                );

                result.triggered && prev.map(|(_, _, t)| !t).unwrap_or(true)
            };

            if should_alert {
                total_triggered += 1;
                if let Some(msg) = result.alert_message {
                    alert::send_alert(
                        &client,
                        &config.ntfy_url,
                        &ticker.symbol,
                        indicator_cfg.name(),
                        msg,
                    )
                    .await;
                }
            }
        }

        tracing::info!("{}", log_parts.join(" | "));
    }

    let retain = compute_retain_count(&config);
    {
        let db = db.lock().await;
        if let Err(e) = db.purge_old_data(retain) {
            tracing::error!(error = %e, "DB purge error");
        }
    }

    tracing::info!(
        "Cycle complete — {total_indicators} indicators evaluated, {total_triggered} triggered, {total_failures} failures"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TickerConfig;
    use tempfile::TempDir;

    fn test_config() -> DaemonConfig {
        DaemonConfig {
            ntfy_url: "https://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            db_path: None,
            ticker_delay_ms: 1500,
            max_retries: 3,
            retry_base_delay_ms: 500,
            tickers: vec![TickerConfig {
                symbol: "INVALID".to_string(),
                indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                    threshold: 35.0,
                    period: 14,
                })],
            }],
        }
    }

    #[test]
    fn test_resolve_db_path_from_config() {
        let mut config = test_config();
        config.db_path = Some("/tmp/test.db".to_string());
        let path = resolve_db_path(&config);
        assert_eq!(path, std::path::PathBuf::from("/tmp/test.db"));
    }

    #[test]
    fn test_resolve_db_path_xdg_fallback() {
        let config = test_config();
        let path = resolve_db_path(&config);
        // Should resolve to some path since HOME is set in test env
        assert!(path.ends_with("data.db"));
    }

    #[test]
    fn test_compute_retain_count_default() {
        let config = test_config();
        // RSI(14) -> required_bars = 42, *2 = 84, cap 1080 -> max(84, 50) = 84
        assert_eq!(compute_retain_count(&config), 84);
    }

    #[test]
    fn test_compute_retain_count_weekly() {
        let mut config = test_config();
        config.interval_type = "1wk".to_string();
        // 36 months * 4 weeks = 144 bars cap
        // RSI(14) -> 42, *2 = 84, cap 144 -> 84 (cap not hit)
        assert_eq!(compute_retain_count(&config), 84);
    }

    #[test]
    fn test_compute_retain_count_daily_cap_kicks_in() {
        let config = DaemonConfig {
            ntfy_url: "https://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            db_path: None,
            ticker_delay_ms: 1500,
            max_retries: 3,
            retry_base_delay_ms: 500,
            tickers: vec![TickerConfig {
                symbol: "T".to_string(),
                indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                    threshold: 30.0,
                    period: 1000,
                })],
            }],
        };
        // RSI(1000) -> 3000, *2 = 6000, cap 1080 -> 1080
        assert_eq!(compute_retain_count(&config), 1080);
    }

    #[test]
    fn test_compute_retain_count_minimum_floor() {
        let config = DaemonConfig {
            ntfy_url: "https://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            db_path: None,
            ticker_delay_ms: 1500,
            max_retries: 3,
            retry_base_delay_ms: 500,
            tickers: vec![TickerConfig {
                symbol: "T".to_string(),
                indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                    threshold: 30.0,
                    period: 2,
                })],
            }],
        };
        // RSI(2) -> required_bars = 6, *2 = 12, floor at 50
        assert_eq!(compute_retain_count(&config), 50);
    }

    #[tokio::test]
    async fn test_evaluate_market_handles_network_error() {
        let config = Arc::new(DaemonConfig {
            ntfy_url: "https://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            db_path: None,
            ticker_delay_ms: 1500,
            max_retries: 3,
            retry_base_delay_ms: 500,
            tickers: vec![TickerConfig {
                symbol: "INVALID".to_string(),
                indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                    threshold: 35.0,
                    period: 14,
                })],
            }],
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap();
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let db = PriceDb::new(&db_path).unwrap();
        let db = Arc::new(Mutex::new(db));
        evaluate_market(config, client, &db).await;
    }

    #[tokio::test]
    async fn test_evaluate_market_multiple_tickers() {
        let config = Arc::new(DaemonConfig {
            ntfy_url: "https://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            db_path: None,
            ticker_delay_ms: 1500,
            max_retries: 3,
            retry_base_delay_ms: 500,
            tickers: vec![
                TickerConfig {
                    symbol: "AAA".to_string(),
                    indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                        threshold: 30.0,
                        period: 14,
                    })],
                },
                TickerConfig {
                    symbol: "BBB".to_string(),
                    indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                        threshold: 40.0,
                        period: 14,
                    })],
                },
            ],
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap();
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let db = PriceDb::new(&db_path).unwrap();
        let db = Arc::new(Mutex::new(db));
        evaluate_market(config, client, &db).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Failed to load configuration")]
    async fn test_run_loop_invalid_path() {
        run_loop("/tmp/nonexistent-config.json").await;
    }

    #[tokio::test]
    async fn test_indicator_result_state_transition_alert_suppression() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let db = PriceDb::new(&db_path).unwrap();
        let db = Arc::new(Mutex::new(db));

        let cfg = IndicatorConfig::Rsi(RsiConfig {
            threshold: 30.0,
            period: 14,
        });
        let config_json = serde_json::to_string(&cfg).unwrap();

        // First evaluation with triggered=true
        {
            let db = db.lock().await;
            db.store_indicator_result("SYM", "RSI", &config_json, 100, None, None, true)
                .unwrap();
        }

        // prev_triggered should be true → new trigger should be suppressed
        let prev_triggered = {
            let db = db.lock().await;
            db.last_indicator_result("SYM", "RSI", &config_json)
                .unwrap()
                .map(|(_, _, t)| t)
        };
        assert_eq!(prev_triggered, Some(true));
        let should_alert = prev_triggered.map(|t| !t).unwrap_or(true);
        assert!(!should_alert);

        // Condition clears → store triggered=false
        {
            let db = db.lock().await;
            db.store_indicator_result("SYM", "RSI", &config_json, 200, None, None, false)
                .unwrap();
        }

        let prev_triggered = {
            let db = db.lock().await;
            db.last_indicator_result("SYM", "RSI", &config_json)
                .unwrap()
                .map(|(_, _, t)| t)
        };
        assert_eq!(prev_triggered, Some(false));

        // New trigger should fire (state change: false → true)
        let should_alert = prev_triggered.map(|t| !t).unwrap_or(true);
        assert!(should_alert);
    }
}
