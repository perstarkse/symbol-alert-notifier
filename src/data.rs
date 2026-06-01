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
    pub open: Vec<Option<f64>>,
    pub high: Vec<Option<f64>>,
    pub low: Vec<Option<f64>>,
    pub close: Vec<Option<f64>>,
    pub volume: Vec<Option<f64>>,
}

pub fn extract_prices(raw_closes: &[Option<f64>]) -> Vec<f64> {
    raw_closes
        .iter()
        .filter_map(|&x| x)
        .filter(|&x| x != 0.0)
        .collect()
}

pub fn build_chart_url(symbol: &str, interval: &str, need_full_range: bool) -> String {
    let range = if need_full_range { "6mo" } else { "1mo" };
    format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
        symbol, range, interval
    )
}

pub fn calculate_price_change(current: f64, previous: f64) -> f64 {
    if previous == 0.0 {
        return 0.0;
    }
    ((current - previous) / previous) * 100.0
}

#[derive(Debug, Clone)]
pub struct OhlcvRow {
    pub ts: i64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    pub volume: Option<f64>,
}

pub fn extract_ohlcv(timestamps: &[i64], quote: &YFQuote) -> Vec<OhlcvRow> {
    timestamps
        .iter()
        .enumerate()
        .filter_map(|(i, &ts)| {
            let close = quote.close.get(i).copied().flatten()?;
            if close == 0.0 {
                return None;
            }
            Some(OhlcvRow {
                ts,
                open: quote.open.get(i).copied().flatten(),
                high: quote.high.get(i).copied().flatten(),
                low: quote.low.get(i).copied().flatten(),
                close,
                volume: quote.volume.get(i).copied().flatten(),
            })
        })
        .collect()
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
    fn test_calculate_price_change_division_by_zero_protection() {
        let result = calculate_price_change(100.0, 0.0);
        assert!((result - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_ohlcv_basic() {
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
        assert_eq!(rows[0].ts, 100);
        assert!((rows[0].close - 10.5).abs() < 0.001);
        assert!((rows[1].high.unwrap() - 13.0).abs() < 0.001);
        assert!((rows[2].volume.unwrap() - 3000.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_ohlcv_filters_zero_close() {
        let timestamps = vec![100, 200];
        let quote = YFQuote {
            open: vec![Some(10.0), Some(0.0)],
            high: vec![Some(12.0), Some(0.0)],
            low: vec![Some(9.0), Some(0.0)],
            close: vec![Some(10.5), Some(0.0)],
            volume: vec![Some(1000.0), Some(0.0)],
        };
        let rows = extract_ohlcv(&timestamps, &quote);
        assert_eq!(rows.len(), 1);
        assert!((rows[0].close - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_extract_ohlcv_handles_missing_data() {
        let timestamps = vec![100, 200, 300];
        let quote = YFQuote {
            open: vec![Some(10.0), None, Some(12.0)],
            high: vec![Some(12.0), None, Some(14.0)],
            low: vec![Some(9.0), None, Some(11.0)],
            close: vec![Some(10.5), None, Some(12.5)],
            volume: vec![Some(1000.0), None, Some(3000.0)],
        };
        let rows = extract_ohlcv(&timestamps, &quote);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ts, 100);
        assert_eq!(rows[1].ts, 300);
    }

    #[test]
    fn test_extract_ohlcv_empty_input() {
        let rows = extract_ohlcv(
            &[],
            &YFQuote {
                open: vec![],
                high: vec![],
                low: vec![],
                close: vec![],
                volume: vec![],
            },
        );
        assert!(rows.is_empty());
    }
}
