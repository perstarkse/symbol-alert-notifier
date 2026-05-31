use rsi_alert_daemon::*;

#[test]
fn test_rsi_within_bounds() {
    let prices: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
    let rsi = calculate_rsi(&prices, 14);
    assert!(rsi >= 0.0);
    assert!(rsi <= 100.0);
}

#[test]
fn test_evaluate_multiple_tickers() {
    let prices_up: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
    let prices_down: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();

    let result_up = evaluate_ticker("UP", 35.0, "1d", &prices_up);
    let result_down = evaluate_ticker("DOWN", 35.0, "1d", &prices_down);

    assert!(result_up.is_none());
    assert!(result_down.is_some());
}

#[test]
fn test_message_has_all_fields() {
    let msg = format_alert_message("KOID", 45.67, -2.34, 5.67, 30.5, "1wk", 35.0);
    assert!(msg.contains("Asset Target Breached!"));
    assert!(msg.contains("Ticker: KOID"));
    assert!(msg.contains("Price: $45.67"));
    assert!(msg.contains("1-Day Change: -2.34%"));
    assert!(msg.contains("7-Day Change: 5.67%"));
    assert!(msg.contains("RSI (1wk): 30.5"));
    assert!(msg.contains("Target: <35.0"));
}

#[test]
fn test_extract_prices_handles_mixed_data() {
    let raw = vec![
        Some(100.0),
        None,
        Some(0.0),
        Some(200.0),
        Some(300.0),
        None,
    ];
    let prices = extract_prices(&raw);
    assert_eq!(prices, vec![100.0, 200.0, 300.0]);
}

#[test]
fn test_binary_fails_without_args() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rsi-alert-daemon"))
        .output()
        .expect("Failed to run binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:"), "Expected usage message, got: {stderr}");
}

#[test]
fn test_binary_fails_with_bad_config() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rsi-alert-daemon"))
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
