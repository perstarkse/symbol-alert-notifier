use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub ntfy_url: String,
    pub interval_type: String,
    pub frequency_seconds: u64,
    pub tickers: Vec<TickerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerConfig {
    pub symbol: String,
    pub indicators: Vec<IndicatorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IndicatorConfig {
    Rsi(RsiConfig),
    BollingerBands(BollingerBandsConfig),
    Macd(MacdConfig),
    Crossover(CrossoverConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsiConfig {
    pub threshold: f64,
    #[serde(default = "default_rsi_period")]
    pub period: usize,
}

const fn default_rsi_period() -> usize {
    14
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBandsConfig {
    #[serde(default = "default_bb_period")]
    pub period: usize,
    #[serde(default = "default_bb_stddev")]
    pub stddev: f64,
}

const fn default_bb_period() -> usize {
    20
}

const fn default_bb_stddev() -> f64 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacdConfig {
    #[serde(default = "default_macd_fast")]
    pub fast: usize,
    #[serde(default = "default_macd_slow")]
    pub slow: usize,
    #[serde(default = "default_macd_signal")]
    pub signal: usize,
}

const fn default_macd_fast() -> usize {
    12
}

const fn default_macd_slow() -> usize {
    26
}

const fn default_macd_signal() -> usize {
    9
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossoverConfig {
    pub fast_period: usize,
    pub slow_period: usize,
}

pub fn load_config(path: &str) -> Result<DaemonConfig, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    Read::read_to_string(&mut file, &mut contents)?;
    let config: DaemonConfig = serde_json::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        let config_data = r#"{
            "ntfy_url": "https://ntfy.sh/test",
            "interval_type": "1d",
            "frequency_seconds": 3600,
            "tickers": [
                {
                    "symbol": "KOID",
                    "indicators": [
                        { "type": "rsi", "threshold": 35.0 }
                    ]
                }
            ]
        }"#;
        std::fs::write(&config_path, config_data).unwrap();
        let config = load_config(config_path.to_str().unwrap()).unwrap();
        assert_eq!(config.ntfy_url, "https://ntfy.sh/test");
        assert_eq!(config.tickers.len(), 1);
        assert_eq!(config.tickers[0].symbol, "KOID");
        assert_eq!(config.tickers[0].indicators.len(), 1);
    }

    #[test]
    fn test_deserialize_rsi_with_default_period() {
        let json = r#"{"type": "rsi", "threshold": 30.0}"#;
        let cfg: IndicatorConfig = serde_json::from_str(json).unwrap();
        match cfg {
            IndicatorConfig::Rsi(rsi) => {
                assert!((rsi.threshold - 30.0).abs() < 0.001);
                assert_eq!(rsi.period, 14);
            }
            _ => panic!("Expected RSI variant"),
        }
    }

    #[test]
    fn test_deserialize_rsi_with_custom_period() {
        let json = r#"{"type": "rsi", "threshold": 30.0, "period": 7}"#;
        let cfg: IndicatorConfig = serde_json::from_str(json).unwrap();
        match cfg {
            IndicatorConfig::Rsi(rsi) => {
                assert_eq!(rsi.period, 7);
            }
            _ => panic!("Expected RSI variant"),
        }
    }

    #[test]
    fn test_deserialize_bb_with_defaults() {
        let json = r#"{"type": "bollinger_bands"}"#;
        let cfg: IndicatorConfig = serde_json::from_str(json).unwrap();
        match cfg {
            IndicatorConfig::BollingerBands(bb) => {
                assert_eq!(bb.period, 20);
                assert!((bb.stddev - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected BollingerBands variant"),
        }
    }

    #[test]
    fn test_deserialize_macd_with_defaults() {
        let json = r#"{"type": "macd"}"#;
        let cfg: IndicatorConfig = serde_json::from_str(json).unwrap();
        match cfg {
            IndicatorConfig::Macd(m) => {
                assert_eq!(m.fast, 12);
                assert_eq!(m.slow, 26);
                assert_eq!(m.signal, 9);
            }
            _ => panic!("Expected Macd variant"),
        }
    }

    #[test]
    fn test_deserialize_crossover() {
        let json = r#"{"type": "crossover", "fast_period": 10, "slow_period": 30}"#;
        let cfg: IndicatorConfig = serde_json::from_str(json).unwrap();
        match cfg {
            IndicatorConfig::Crossover(c) => {
                assert_eq!(c.fast_period, 10);
                assert_eq!(c.slow_period, 30);
            }
            _ => panic!("Expected Crossover variant"),
        }
    }

    #[test]
    fn test_deserialize_multiple_indicators() {
        let json = r#"{
            "ntfy_url": "https://ntfy.sh/test",
            "interval_type": "1d",
            "frequency_seconds": 3600,
            "tickers": [
                {
                    "symbol": "TEST",
                    "indicators": [
                        { "type": "rsi", "threshold": 30.0 },
                        { "type": "bollinger_bands", "period": 15, "stddev": 2.5 }
                    ]
                }
            ]
        }"#;
        let config: DaemonConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.tickers[0].indicators.len(), 2);
        match &config.tickers[0].indicators[0] {
            IndicatorConfig::Rsi(rsi) => assert!((rsi.threshold - 30.0).abs() < 0.001),
            _ => panic!("Expected RSI"),
        }
        match &config.tickers[0].indicators[1] {
            IndicatorConfig::BollingerBands(bb) => {
                assert_eq!(bb.period, 15);
                assert!((bb.stddev - 2.5).abs() < 0.001);
            }
            _ => panic!("Expected BB"),
        }
    }
}
