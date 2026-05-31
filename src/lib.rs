pub mod alert;
pub mod config;
pub mod data;
pub mod indicators;

use std::sync::Arc;

use config::load_config;
use data::{build_chart_url, extract_prices, YFResponse};
use indicators::Indicator;

pub use config::{
    BollingerBandsConfig, CrossoverConfig, DaemonConfig, IndicatorConfig, MacdConfig, RsiConfig,
    TickerConfig,
};
pub use data::MarketData;
pub use indicators::IndicatorResult;

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

pub async fn evaluate_market(config: Arc<DaemonConfig>, client: reqwest::Client) {
    println!("Beginning cycle evaluation across targets...");

    let mut total_indicators = 0u32;
    let mut total_triggered = 0u32;
    let mut total_failures = 0u32;

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
                total_failures += 1;
                continue;
            }
        };

        if let Ok(yf_resp) = res.json::<YFResponse>().await {
            if let Some(results) = yf_resp.chart.result {
                if let Some(quote) = results.first().map(|r| &r.indicators.quote) {
                    if let Some(raw_closes) = quote.first().map(|q| &q.close) {
                        let prices = extract_prices(raw_closes);
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

                            if result.triggered {
                                total_triggered += 1;
                            }

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

                        println!("{}", log_parts.join(" | "));
                    }
                }
            }
        }
    }

    println!(
        "Cycle complete — {} indicators evaluated, {} triggered, {} failures.",
        total_indicators, total_triggered, total_failures
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TickerConfig;

    #[tokio::test]
    async fn test_evaluate_market_handles_network_error() {
        let config = Arc::new(DaemonConfig {
            ntfy_url: "http://localhost:1".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
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
        evaluate_market(config, client).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Failed to load configuration")]
    async fn test_run_loop_invalid_path() {
        run_loop("/tmp/nonexistent-config.json").await;
    }
}
