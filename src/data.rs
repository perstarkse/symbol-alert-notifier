use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct MarketData {
    pub symbol: String,
    pub close_prices: Vec<f64>,
    pub interval_type: String,
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

pub fn extract_prices(raw_closes: &[Option<f64>]) -> Vec<f64> {
    raw_closes
        .iter()
        .filter_map(|&x| x)
        .filter(|&x| x != 0.0)
        .collect()
}

pub fn build_chart_url(symbol: &str, interval: &str) -> String {
    format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=6mo&interval={}",
        symbol, interval
    )
}

pub fn calculate_price_change(current: f64, previous: f64) -> f64 {
    ((current - previous) / previous) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_prices_filters_none_and_zero() {
        let raw = vec![Some(10.0), None, Some(0.0), Some(20.0)];
        let prices = extract_prices(&raw);
        assert_eq!(prices, vec![10.0, 20.0]);
    }

    #[test]
    fn test_build_chart_url() {
        let url = build_chart_url("KOID", "1wk");
        assert!(url.contains("KOID"));
        assert!(url.contains("6mo"));
        assert!(url.contains("interval=1wk"));
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
}
