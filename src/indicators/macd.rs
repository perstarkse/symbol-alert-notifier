use crate::config::MacdConfig;
use crate::data::MarketData;
use crate::indicators::{Indicator, IndicatorResult};

impl Indicator for MacdConfig {
    fn evaluate(&self, data: &MarketData) -> IndicatorResult {
        let prices = &data.close_prices;
        if prices.len() < self.slow + self.signal {
            return IndicatorResult {
                triggered: false,
                alert_message: None,
                log_label: "MACD: N/A".to_string(),
            };
        }

        let current_price = prices[prices.len() - 1];

        let macd_current = ema(prices, self.fast) - ema(prices, self.slow);
        let signal_current = calc_macd_signal(prices, self.fast, self.slow, self.signal);

        let prev = &prices[..prices.len() - 1];
        let macd_prev = ema(prev, self.fast) - ema(prev, self.slow);
        let signal_prev = calc_macd_signal(prev, self.fast, self.slow, self.signal);

        let log_label = format!("MACD: {:.2} Signal: {:.2}", macd_current, signal_current);

        let bullish = macd_prev <= signal_prev && macd_current > signal_current;
        let bearish = macd_prev >= signal_prev && macd_current < signal_current;

        if bullish {
            let msg = format!(
                "Asset Target Breached!\n\
                 Ticker: {}\n\
                 Price: ${:.2}\n\
                 MACD ({}): Bullish Crossover!\n\
                 MACD: {:.2}\n\
                 Signal: {:.2}",
                data.symbol, current_price, data.interval_type, macd_current, signal_current,
            );
            IndicatorResult {
                triggered: true,
                alert_message: Some(msg),
                log_label,
            }
        } else if bearish {
            let msg = format!(
                "Asset Target Breached!\n\
                 Ticker: {}\n\
                 Price: ${:.2}\n\
                 MACD ({}): Bearish Crossover!\n\
                 MACD: {:.2}\n\
                 Signal: {:.2}",
                data.symbol, current_price, data.interval_type, macd_current, signal_current,
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
        "MACD"
    }

    fn required_bars(&self) -> usize {
        self.slow + self.signal + self.fast
    }
}

fn ema(values: &[f64], period: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() < period {
        return values.iter().sum::<f64>() / values.len() as f64;
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema_value: f64 = values[..period].iter().sum::<f64>() / period as f64;
    for &v in &values[period..] {
        ema_value = (v - ema_value) * multiplier + ema_value;
    }
    ema_value
}

fn calc_macd_signal(prices: &[f64], fast: usize, slow: usize, signal: usize) -> f64 {
    let start = slow;
    let end = prices.len();
    let mut macd_vals = Vec::with_capacity(end - start + 1);
    for i in start..=end {
        let slice = &prices[..i];
        macd_vals.push(ema(slice, fast) - ema(slice, slow));
    }
    ema(&macd_vals, signal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_constant_values() {
        let values = vec![10.0; 20];
        let result = ema(&values, 10);
        assert!((result - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_ema_increasing_values() {
        let values: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let result = ema(&values, 10);
        assert!(result > 0.0);
    }

    #[test]
    fn test_ema_short_input() {
        let values = vec![1.0, 2.0, 3.0];
        let result = ema(&values, 10);
        assert!((result - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_macd_indicator_bullish_crossover() {
        let mut prices: Vec<f64> = vec![100.0; 40];
        prices.push(200.0);

        let cfg = MacdConfig {
            fast: 5,
            slow: 13,
            signal: 5,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(result.alert_message.is_some());
        let msg = result.alert_message.unwrap();
        assert!(msg.contains("Bullish Crossover"));
    }

    #[test]
    fn test_macd_indicator_runs() {
        let prices: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64).sin() * 2.0).collect();
        let cfg = MacdConfig {
            fast: 5,
            slow: 13,
            signal: 5,
        };
        let data = MarketData {
            symbol: "TEST".to_string(),
            close_prices: prices,
            interval_type: "1d".to_string(),
        };
        let result = cfg.evaluate(&data);
        assert!(result.log_label.contains("MACD:"));
    }

    #[test]
    fn test_macd_insufficient_data() {
        let prices = vec![100.0, 101.0, 102.0];
        let cfg = MacdConfig {
            fast: 12,
            slow: 26,
            signal: 9,
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

    #[test]
    fn test_calc_macd_signal_consistency() {
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();

        let _macd_line = ema(&prices, 12) - ema(&prices, 26);
        let signal = calc_macd_signal(&prices, 12, 26, 9);

        assert!(!signal.is_nan());
        assert!(!signal.is_infinite());
    }
}
