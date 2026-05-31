use indicator_alert_daemon::data::{build_chart_url, extract_prices};
use indicator_alert_daemon::indicators::{self, Indicator};
use indicator_alert_daemon::*;

#[test]
fn test_rsi_within_bounds() {
    let prices: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
    let rsi = indicators::calculate_rsi(&prices, 14);
    assert!(rsi >= 0.0);
    assert!(rsi <= 100.0);
}

#[test]
fn test_evaluate_multiple_indicators() {
    let cfg_rsi = IndicatorConfig::Rsi(RsiConfig {
        threshold: 35.0,
        period: 14,
    });

    let data_up = MarketData {
        symbol: "UP".to_string(),
        close_prices: (0..30).map(|i| 100.0 + i as f64).collect(),
        interval_type: "1d".to_string(),
    };
    let data_down = MarketData {
        symbol: "DOWN".to_string(),
        close_prices: (0..30).map(|i| 100.0 - i as f64).collect(),
        interval_type: "1d".to_string(),
    };

    let result_up = cfg_rsi.evaluate(&data_up);
    let result_down = cfg_rsi.evaluate(&data_down);

    assert!(!result_up.triggered);
    assert!(result_down.triggered);
}

#[test]
fn test_message_has_all_fields() {
    let cfg = IndicatorConfig::Rsi(RsiConfig {
        threshold: 35.0,
        period: 14,
    });
    let data = MarketData {
        symbol: "KOID".to_string(),
        close_prices: (0..30).map(|i| 100.0 - i as f64).collect(),
        interval_type: "1wk".to_string(),
    };
    let result = cfg.evaluate(&data);
    let msg = result.alert_message.expect("Should trigger");
    assert!(msg.contains("Asset Target Breached!"));
    assert!(msg.contains("Ticker: KOID"));
    assert!(msg.contains("RSI (1wk):"));
}

#[test]
fn test_extract_prices_handles_mixed_data() {
    let raw = vec![Some(100.0), None, Some(0.0), Some(200.0), Some(300.0), None];
    let prices = extract_prices(&raw);
    assert_eq!(prices, vec![100.0, 200.0, 300.0]);
}

#[test]
fn test_binary_fails_without_args() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_indicator-alert-daemon"))
        .output()
        .expect("Failed to run binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage:"),
        "Expected usage message, got: {stderr}"
    );
}

#[test]
fn test_binary_fails_with_bad_config() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_indicator-alert-daemon"))
        .arg("/tmp/nonexistent-bad-path.json")
        .output()
        .expect("Failed to run binary");
    assert!(!output.status.success());
}

#[test]
fn test_build_chart_url_format() {
    let url = build_chart_url("BOTZ", "1d");
    assert_eq!(
        url,
        "https://query1.finance.yahoo.com/v8/finance/chart/BOTZ?range=6mo&interval=1d"
    );
}
