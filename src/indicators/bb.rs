use crate::config::BollingerBandsConfig;
use crate::data::MarketData;
use crate::indicators::{Indicator, IndicatorResult};

impl Indicator for BollingerBandsConfig {
    fn evaluate(&self, data: &MarketData) -> IndicatorResult {
        let prices = &data.close_prices;
        if prices.len() < self.period {
            return IndicatorResult {
                triggered: false,
                alert_message: None,
                log_label: "BB: N/A".to_string(),
            };
        }

        let current_price = prices[prices.len() - 1];
        let (sma, upper, lower) = bollinger_bands(prices, self.period, self.stddev);
        let log_label = format!(
            "BB: ${:.2} (lower: ${:.2}, upper: ${:.2})",
            current_price, lower, upper
        );

        if current_price <= lower {
            let msg = format!(
                "Asset Target Breached!\n\
                 Ticker: {}\n\
                 Price: ${:.2}\n\
                 Bollinger Bands ({}): Lower band breakout!\n\
                 SMA: ${:.2}, Lower: ${:.2}, Upper: ${:.2}",
                data.symbol, current_price, data.interval_type, sma, lower, upper,
            );
            IndicatorResult {
                triggered: true,
                alert_message: Some(msg),
                log_label,
            }
        } else {
            IndicatorResult {
                triggered: false,
                alert_message: None,
                log_label,
            }
        }
    }

    fn name(&self) -> &str {
        "Bollinger Bands"
    }
}

fn bollinger_bands(prices: &[f64], period: usize, stddev: f64) -> (f64, f64, f64) {
    let len = prices.len();
    let slice = &prices[len.saturating_sub(period)..];

    let sma: f64 = slice.iter().sum::<f64>() / slice.len() as f64;
    let variance: f64 = slice.iter().map(|&x| (x - sma).powi(2)).sum::<f64>() / slice.len() as f64;
    let sd = variance.sqrt();
    let upper = sma + stddev * sd;
    let lower = sma - stddev * sd;
    (sma, upper, lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bb_constant_prices() {
        let prices = vec![50.0; 25];
        let (sma, upper, lower) = bollinger_bands(&prices, 20, 2.0);
        assert!((sma - 50.0).abs() < 0.001);
        assert!((upper - 50.0).abs() < 0.001);
        assert!((lower - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_bb_varied_prices() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
        let (sma, upper, lower) = bollinger_bands(&prices, 20, 2.0);
        assert!(upper > sma);
        assert!(lower < sma);
    }

    #[test]
    fn test_bb_indicator_lower_band_breakout() {
        let mut prices: Vec<f64> = (0..25).map(|_| 100.0).collect();
        prices.push(10.0);
        let cfg = BollingerBandsConfig {
            period: 20,
            stddev: 2.0,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(result.triggered);
        assert!(result.alert_message.is_some());
    }

    #[test]
    fn test_bb_indicator_no_breakout() {
        let prices: Vec<f64> = (0..25).map(|i| 100.0 + (i as f64).sin() * 2.0).collect();
        let cfg = BollingerBandsConfig {
            period: 20,
            stddev: 2.0,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(!result.triggered);
    }

    #[test]
    fn test_bb_insufficient_data() {
        let prices = vec![100.0, 101.0, 102.0];
        let cfg = BollingerBandsConfig {
            period: 20,
            stddev: 2.0,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(!result.triggered);
        assert!(result.alert_message.is_none());
    }
}
