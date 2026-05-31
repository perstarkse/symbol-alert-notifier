#[tokio::main]
async fn main() {
    let config_path = std::env::args()
        .nth(1)
        .expect("Usage: indicator-alert-daemon <config.json>");
    indicator_alert_daemon::run_loop(&config_path).await;
}
