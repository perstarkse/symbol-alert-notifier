use crate::config::RsiConfig;
use crate::data::{calculate_price_change, MarketData};
use crate::indicators::{Indicator, IndicatorResult};

impl Indicator for RsiConfig {
    fn evaluate(&self, data: &MarketData) -> IndicatorResult {
        let prices = &data.close_prices;
        if prices.len() <= self.period {
            return IndicatorResult {
                triggered: false,
                alert_message: None,
                log_label: "RSI: N/A".to_string(),
            };
        }

        let rsi = calculate_rsi(prices, self.period);
        let len = prices.len();
        let current_price = prices[len - 1];
        let chg_1d = calculate_price_change(current_price, prices[len - 2]);
        let chg_7d = if len >= 8 {
            calculate_price_change(current_price, prices[len - 8])
        } else {
            0.0
        };

        let log_label = format!("RSI: {:.1}", rsi);

        if rsi < self.threshold {
            let msg = format!(
                "Asset Target Breached!\n\
                 Ticker: {}\n\
                 Price: ${:.2}\n\
                 1-Day Change: {:.2}%\n\
                 7-Day Change: {:.2}%\n\
                 RSI ({}): {:.1} (Target: <{:.1})",
                data.symbol, current_price, chg_1d, chg_7d, data.interval_type, rsi, self.threshold,
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
        "RSI"
    }
}

pub fn calculate_rsi(prices: &[f64], period: usize) -> f64 {
    if prices.len() <= period {
        return 50.0;
    }
    let mut gains = 0.0;
    let mut losses = 0.0;

    for i in 1..=period {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains += change;
        } else {
            losses -= change;
        }
    }

    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;

    for i in (period + 1)..prices.len() {
        let change = prices[i] - prices[i - 1];
        let (c_gain, c_loss) = if change > 0.0 {
            (change, 0.0)
        } else {
            (0.0, -change)
        };
        avg_gain = (avg_gain * (period - 1) as f64 + c_gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + c_loss) / period as f64;
    }

    if avg_loss == 0.0 {
        return 100.0;
    }
    100.0 - (100.0 / (1.0 + (avg_gain / avg_loss)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rsi_all_up() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_rsi_all_down() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_rsi_alternating() {
        let mut prices = Vec::new();
        for i in 0..100 {
            prices.push(100.0 + if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_calculate_rsi_short_input() {
        let prices = vec![10.0, 11.0, 12.0];
        let rsi = calculate_rsi(&prices, 14);
        assert!((rsi - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_rsi_within_bounds() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
        let rsi = calculate_rsi(&prices, 14);
        assert!(rsi >= 0.0);
        assert!(rsi <= 100.0);
    }

    #[test]
    fn test_rsi_indicator_below_threshold() {
        let cfg = RsiConfig {
            threshold: 35.0,
            period: 14,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: (0..30).map(|i| 100.0 - i as f64).collect(),
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(result.triggered);
        assert!(result.alert_message.is_some());
        assert!(result.alert_message.unwrap().contains("TEST"));
    }

    #[test]
    fn test_rsi_indicator_above_threshold() {
        let cfg = RsiConfig {
            threshold: 35.0,
            period: 14,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: (0..30).map(|i| 100.0 + i as f64).collect(),
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(!result.triggered);
        assert!(result.alert_message.is_none());
    }

    #[test]
    fn test_rsi_indicator_insufficient_data() {
        let cfg = RsiConfig {
            threshold: 35.0,
            period: 14,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: vec![100.0, 101.0, 102.0],
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(!result.triggered);
        assert!(result.alert_message.is_none());
    }
}
