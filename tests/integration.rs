use indicator_alert_daemon::data::{
    build_chart_url, extract_ohlcv, extract_prices, OhlcvRow, YFQuote,
};
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
fn test_build_chart_url_format_full() {
    let url = build_chart_url("BOTZ", "1d", true);
    assert_eq!(
        url,
        "https://query1.finance.yahoo.com/v8/finance/chart/BOTZ?range=6mo&interval=1d"
    );
}

#[test]
fn test_build_chart_url_format_incremental() {
    let url = build_chart_url("BOTZ", "1d", false);
    assert_eq!(
        url,
        "https://query1.finance.yahoo.com/v8/finance/chart/BOTZ?range=1mo&interval=1d"
    );
}

#[test]
fn test_ohlcv_row_creation() {
    let row = OhlcvRow {
        ts: 1234567890,
        open: Some(100.0),
        high: Some(105.0),
        low: Some(99.0),
        close: 102.5,
        volume: Some(10000.0),
    };
    assert_eq!(row.ts, 1234567890);
    assert!((row.close - 102.5).abs() < 0.001);
}

#[test]
fn test_extract_ohlcv_from_quote() {
    let timestamps = vec![100, 200, 300];
    let quote = YFQuote {
        open: vec![Some(10.0), Some(11.0), Some(12.0)],
        high: vec![Some(12.0), Some(13.0), Some(14.0)],
        low: vec![Some(9.0), Some(10.0), Some(11.0)],
        close: vec![Some(10.5), Some(11.5), Some(12.5)],
        volume: vec![Some(1000.0), Some(2000.0), Some(3000.0)],
    };
    let rows = extract_ohlcv(&timestamps, &quote);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].ts, 200);
    assert!((rows[1].close - 11.5).abs() < 0.001);
}

#[test]
fn test_indicator_required_bars() {
    let rsi = IndicatorConfig::Rsi(RsiConfig {
        threshold: 30.0,
        period: 14,
    });
    assert_eq!(rsi.required_bars(), 42);

    let bb = IndicatorConfig::BollingerBands(BollingerBandsConfig {
        period: 20,
        stddev: 2.0,
    });
    assert_eq!(bb.required_bars(), 40);

    let macd = IndicatorConfig::Macd(MacdConfig {
        fast: 12,
        slow: 26,
        signal: 9,
    });
    assert_eq!(macd.required_bars(), 47);

    let cross = IndicatorConfig::Crossover(CrossoverConfig {
        fast_period: 5,
        slow_period: 15,
    });
    assert_eq!(cross.required_bars(), 30);
}
