use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerConfig {
    pub symbol: String,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub ntfy_url: String,
    pub interval_type: String,
    pub frequency_seconds: u64,
    pub tickers: Vec<TickerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct YFResponse {
    pub chart: YFChart,
}

#[derive(Debug, Deserialize)]
pub struct YFChart {
    pub result: Option<Vec<YFChartResult>>,
}

#[derive(Debug, Deserialize)]
pub struct YFChartResult {
    pub timestamp: Vec<i64>,
    pub indicators: YFIndicators,
}

#[derive(Debug, Deserialize)]
pub struct YFIndicators {
    pub quote: Vec<YFQuote>,
}

#[derive(Debug, Deserialize)]
pub struct YFQuote {
    pub close: Vec<Option<f64>>,
}

pub fn calculate_rsi(prices: &[f64], period: usize) -> f64 {
    if prices.len() <= period {
        return 50.0;
    }
    let mut gains = 0.0;
    let mut losses = 0.0;

    for i in 1..=period {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains += change;
        } else {
            losses -= change;
        }
    }

    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;

    for i in (period + 1)..prices.len() {
        let change = prices[i] - prices[i - 1];
        let (c_gain, c_loss) = if change > 0.0 {
            (change, 0.0)
        } else {
            (0.0, -change)
        };
        avg_gain = (avg_gain * (period - 1) as f64 + c_gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + c_loss) / period as f64;
    }

    if avg_loss == 0.0 {
        return 100.0;
    }
    100.0 - (100.0 / (1.0 + (avg_gain / avg_loss)))
}

pub fn calculate_price_change(current: f64, previous: f64) -> f64 {
    ((current - previous) / previous) * 100.0
}

pub fn format_alert_message(
    ticker: &str,
    price: f64,
    chg_1d: f64,
    chg_7d: f64,
    rsi: f64,
    interval_type: &str,
    threshold: f64,
) -> String {
    format!(
        "Asset Target Breached!\n\
         Ticker: {}\n\
         Price: ${:.2}\n\
         1-Day Change: {:.2}%\n\
         7-Day Change: {:.2}%\n\
         RSI ({}): {:.1} (Target: <{:.1})",
        ticker, price, chg_1d, chg_7d, interval_type, rsi, threshold,
    )
}

pub fn load_config(path: &str) -> Result<DaemonConfig, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    std::io::Read::read_to_string(&mut file, &mut contents)?;
    let config: DaemonConfig = serde_json::from_str(&contents)?;
    Ok(config)
}

pub fn extract_prices(raw_closes: &[Option<f64>]) -> Vec<f64> {
    raw_closes
        .iter()
        .filter_map(|&x| x)
        .filter(|&x| x != 0.0)
        .collect()
}

pub fn evaluate_ticker(
    symbol: &str,
    threshold: f64,
    interval_type: &str,
    prices: &[f64],
) -> Option<String> {
    if prices.len() < 15 {
        return None;
    }

    let len = prices.len();
    let current_price = prices[len - 1];
    let chg_1d = calculate_price_change(current_price, prices[len - 2]);
    let chg_7d = if len >= 8 {
        calculate_price_change(current_price, prices[len - 8])
    } else {
        0.0
    };
    let rsi = calculate_rsi(prices, 14);

    if rsi < threshold {
        Some(format_alert_message(
            symbol,
            current_price,
            chg_1d,
            chg_7d,
            rsi,
            interval_type,
            threshold,
        ))
    } else {
        None
    }
}

pub fn build_chart_url(symbol: &str, interval: &str) -> String {
    format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=6mo&interval={}",
        symbol, interval
    )
}

pub async fn run_loop(config_path: &str) {
    let config = load_config(config_path).expect("Failed to load configuration");
    let config = Arc::new(config);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");
    loop {
        evaluate_market(config.clone(), client.clone()).await;
        tokio::time::sleep(std::time::Duration::from_secs(config.frequency_seconds)).await;
    }
}

pub async fn evaluate_market(
    config: Arc<DaemonConfig>,
    client: reqwest::Client,
) {
    println!("Beginning cycle evaluation across targets...");

    for ticker in &config.tickers {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        let url = build_chart_url(&ticker.symbol, &config.interval_type);

        let res = match client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => {
                println!("[{}] Network connection failed.", ticker.symbol);
                continue;
            }
        };

        if let Ok(yf_resp) = res.json::<YFResponse>().await {
            if let Some(results) = yf_resp.chart.result {
                if let Some(quote) = results.first().map(|r| &r.indicators.quote) {
                    if let Some(raw_closes) = quote.first().map(|q| &q.close) {
                        let prices = extract_prices(raw_closes);
                        if prices.len() < 15 {
                            continue;
                        }

                        let current_price = prices[prices.len() - 1];
                        let chg_1d = calculate_price_change(
                            current_price,
                            prices[prices.len() - 2],
                        );
                        let chg_7d = if prices.len() >= 8 {
                            calculate_price_change(
                                current_price,
                                prices[prices.len() - 8],
                            )
                        } else {
                            0.0
                        };

                        let rsi = calculate_rsi(&prices, 14);
                        println!(
                            "[{}] Price: ${:.2} | RSI: {:.1} | 1D: {:.1}% | 7D: {:.1}%",
                            ticker.symbol, current_price, rsi, chg_1d, chg_7d
                        );

                        if rsi < ticker.threshold {
                            let msg = format_alert_message(
                                &ticker.symbol,
                                current_price,
                                chg_1d,
                                chg_7d,
                                rsi,
                                &config.interval_type,
                                ticker.threshold,
                            );
                            let title = format!("Value Buy Alert: {}", ticker.symbol);

                            let _ = client
                                .post(&config.ntfy_url)
                                .header("Title", &title)
                                .header("Priority", "high")
                                .header("Tags", "chart_with_downwards_trend,moneybag")
                                .body(msg)
                                .send()
                                .await;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rsi_all_up() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_rsi_all_down() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_rsi_alternating() {
        let mut prices = Vec::new();
        for i in 0..100 {
            prices.push(100.0 + if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_calculate_rsi_short_input() {
        let prices = vec![10.0, 11.0, 12.0];
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_price_change_positive() {
        let result = calculate_price_change(110.0, 100.0);
        assert!((result - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_price_change_negative() {
        let result = calculate_price_change(90.0, 100.0);
        assert!((result - (-10.0)).abs() < 0.001);
    }

    #[test]
    fn test_calculate_price_change_zero() {
        let result = calculate_price_change(100.0, 100.0);
        assert!((result - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_prices_filters_none_and_zero() {
        let raw = vec![Some(10.0), None, Some(0.0), Some(20.0)];
        let prices = extract_prices(&raw);
        assert_eq!(prices, vec![10.0, 20.0]);
    }

    #[test]
    fn test_format_alert_message() {
        let msg = format_alert_message("KOID", 45.67, -2.34, 5.67, 30.5, "1wk", 35.0);
        assert!(msg.contains("KOID"));
        assert!(msg.contains("$45.67"));
        assert!(msg.contains("-2.34%"));
        assert!(msg.contains("5.67%"));
        assert!(msg.contains("30.5"));
        assert!(msg.contains("<35.0"));
    }

    #[test]
    fn test_evaluate_ticker_below_threshold() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();
        let result = evaluate_ticker("TEST", 35.0, "1wk", &prices);
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("TEST"));
    }

    #[test]
    fn test_evaluate_ticker_above_threshold() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let result = evaluate_ticker("TEST", 35.0, "1wk", &prices);
        assert!(result.is_none());
    }

    #[test]
    fn test_evaluate_ticker_insufficient_data() {
        let prices = vec![100.0, 101.0, 102.0];
        let result = evaluate_ticker("TEST", 35.0, "1wk", &prices);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        let config_data = r#"{
            "ntfy_url": "https://ntfy.sh/test",
            "interval_type": "1d",
            "frequency_seconds": 3600,
            "tickers": [
                {"symbol": "KOID", "threshold": 35.0}
            ]
        }"#;
        std::fs::write(&config_path, config_data).unwrap();
        let config = load_config(config_path.to_str().unwrap()).unwrap();
        assert_eq!(config.ntfy_url, "https://ntfy.sh/test");
        assert_eq!(config.interval_type, "1d");
        assert_eq!(config.frequency_seconds, 3600);
        assert_eq!(config.tickers.len(), 1);
        assert_eq!(config.tickers[0].symbol, "KOID");
        assert_eq!(config.tickers[0].threshold, 35.0);
    }

    #[test]
    fn test_build_chart_url() {
        let url = build_chart_url("KOID", "1wk");
        assert!(url.contains("KOID"));
        assert!(url.contains("6mo"));
        assert!(url.contains("interval=1wk"));
    }

    #[tokio::test]
    async fn test_evaluate_market_handles_network_error() {
        let config = Arc::new(DaemonConfig {
            ntfy_url: "http://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            tickers: vec![TickerConfig {
                symbol: "INVALID".to_string(),
                threshold: 35.0,
            }],
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap();
        evaluate_market(config, client).await;
    }

    #[tokio::test]
    async fn test_evaluate_market_multiple_tickers() {
        let config = Arc::new(DaemonConfig {
            ntfy_url: "http://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            tickers: vec![
                TickerConfig {
                    symbol: "AAA".to_string(),
                    threshold: 30.0,
                },
                TickerConfig {
                    symbol: "BBB".to_string(),
                    threshold: 40.0,
                },
            ],
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap();
        evaluate_market(config, client).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Failed to load configuration")]
    async fn test_run_loop_invalid_path() {
        run_loop("/tmp/nonexistent-config.json").await;
    }
}
