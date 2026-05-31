#[tokio::main]
async fn main() {
    let config_path = std::env::args()
        .nth(1)
        .expect("Usage: rsi-alert-daemon <config.json>");
    rsi_alert_daemon::run_loop(&config_path).await;
}
