use crate::config::CrossoverConfig;
use crate::data::MarketData;
use crate::indicators::{Indicator, IndicatorResult};

impl Indicator for CrossoverConfig {
    fn evaluate(&self, data: &MarketData) -> IndicatorResult {
        let prices = &data.close_prices;
        if prices.len() < self.slow_period + 1 {
            return IndicatorResult {
                triggered: false,
                alert_message: None,
                log_label: "Crossover: N/A".to_string(),
            };
        }

        let current_price = prices[prices.len() - 1];

        let fast_current = sma(prices, self.fast_period);
        let slow_current = sma(prices, self.slow_period);

        let fast_prev = sma(&prices[..prices.len() - 1], self.fast_period);
        let slow_prev = sma(&prices[..prices.len() - 1], self.slow_period);

        let log_label = format!(
            "Crossover: fast={:.2} slow={:.2}",
            fast_current, slow_current
        );

        if fast_prev <= slow_prev && fast_current > slow_current {
            let msg = format!(
                "Asset Target Breached!\n\
                 Ticker: {}\n\
                 Price: ${:.2}\n\
                 MA Crossover ({}): Golden Cross!\n\
                 Fast MA ({}-period): ${:.2}\n\
                 Slow MA ({}-period): ${:.2}",
                data.symbol,
                current_price,
                data.interval_type,
                self.fast_period,
                fast_current,
                self.slow_period,
                slow_current,
            );
            IndicatorResult {
                triggered: true,
                alert_message: Some(msg),
                log_label,
            }
        } else if fast_prev >= slow_prev && fast_current < slow_current {
            let msg = format!(
                "Asset Target Breached!\n\
                 Ticker: {}\n\
                 Price: ${:.2}\n\
                 MA Crossover ({}): Death Cross!\n\
                 Fast MA ({}-period): ${:.2}\n\
                 Slow MA ({}-period): ${:.2}",
                data.symbol,
                current_price,
                data.interval_type,
                self.fast_period,
                fast_current,
                self.slow_period,
                slow_current,
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
        "MA Crossover"
    }
}

fn sma(prices: &[f64], period: usize) -> f64 {
    let len = prices.len();
    let slice = &prices[len.saturating_sub(period)..];
    slice.iter().sum::<f64>() / slice.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&prices, 3);
        assert!((result - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_sma_period_equals_length() {
        let prices = vec![10.0, 20.0, 30.0];
        let result = sma(&prices, 3);
        assert!((result - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_crossover_golden_cross() {
        // 40 stable at 100, then dip to 50, then rise to 110.
        // Fast SMA(5) drops below slow SMA(15) during dip, crosses above on rise.
        let mut prices = vec![100.0; 40];
        for v in [90.0, 80.0, 70.0, 60.0, 50.0] {
            prices.push(v);
        }
        for v in [60.0, 70.0, 80.0, 90.0, 100.0, 110.0] {
            prices.push(v);
        }

        let cfg = CrossoverConfig {
            fast_period: 5,
            slow_period: 15,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(result.alert_message.is_some());
        let msg = result.alert_message.unwrap();
        assert!(msg.contains("Golden Cross"));
    }

    #[test]
    fn test_crossover_death_cross() {
        // 40 stable at 100, then surge to 150, then drop to 90.
        // Fast SMA(5) rises above slow SMA(15) during surge, crosses below on drop.
        let mut prices = vec![100.0; 40];
        for v in [110.0, 120.0, 130.0, 140.0, 150.0] {
            prices.push(v);
        }
        for v in [140.0, 130.0, 120.0, 110.0, 100.0, 90.0] {
            prices.push(v);
        }

        let cfg = CrossoverConfig {
            fast_period: 5,
            slow_period: 15,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(result.alert_message.is_some());
        let msg = result.alert_message.unwrap();
        assert!(msg.contains("Death Cross"));
    }

    #[test]
    fn test_crossover_no_cross() {
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let cfg = CrossoverConfig {
            fast_period: 5,
            slow_period: 15,
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
    fn test_crossover_insufficient_data() {
        let prices = vec![100.0, 101.0, 102.0];
        let cfg = CrossoverConfig {
            fast_period: 5,
            slow_period: 15,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(!result.triggered);
    }
}
