use serde::Deserialize;

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
    pub open: Vec<Option<f64>>,
    pub high: Vec<Option<f64>>,
    pub low: Vec<Option<f64>>,
    pub close: Vec<Option<f64>>,
    pub volume: Vec<Option<f64>>,
}

pub fn build_chart_url(symbol: &str, interval: &str, need_full_range: bool) -> String {
    let range = if need_full_range { "6mo" } else { "1mo" };
    format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
        symbol, range, interval
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chart_url_full_range() {
        let url = build_chart_url("KOID", "1wk", true);
        assert!(url.contains("KOID"));
        assert!(url.contains("6mo"));
        assert!(url.contains("interval=1wk"));
    }

    #[test]
    fn test_build_chart_url_incremental_range() {
        let url = build_chart_url("KOID", "1d", false);
        assert!(url.contains("KOID"));
        assert!(url.contains("1mo"));
        assert!(url.contains("interval=1d"));
    }
}
