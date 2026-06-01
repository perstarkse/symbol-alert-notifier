#[tracing::instrument(skip(client, message))]
pub async fn send_alert(
    client: &reqwest::Client,
    ntfy_url: &str,
    symbol: &str,
    indicator_name: &str,
    message: String,
) {
    let title = format!("Alert: {} - {}", indicator_name, symbol);
    if let Err(e) = client
        .post(ntfy_url)
        .header("Title", &title)
        .header("Priority", "high")
        .header("Tags", "chart_with_downwards_trend,moneybag")
        .body(message)
        .send()
        .await
    {
        tracing::error!(
            symbol,
            indicator = indicator_name,
            error = %e,
            "Failed to send alert"
        );
    }
}
