use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub ntfy_url: String,
    pub interval_type: String,
    pub frequency_seconds: u64,
    pub tickers: Vec<TickerConfig>,
    #[serde(default)]
    pub db_path: Option<String>,
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
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &DaemonConfig) -> Result<(), Box<dyn std::error::Error>> {
    if config.ntfy_url.trim().is_empty() {
        return Err("ntfy_url must not be empty".into());
    }
    match config.interval_type.as_str() {
        "1d" | "1wk" => {}
        _ => {
            return Err(format!(
                "unsupported interval_type: '{}' (expected '1d' or '1wk')",
                config.interval_type
            )
            .into());
        }
    }
    if config.frequency_seconds == 0 {
        return Err("frequency_seconds must be > 0".into());
    }
    if config.tickers.is_empty() {
        return Err("at least one ticker is required".into());
    }
    for ticker in &config.tickers {
        if ticker.symbol.trim().is_empty() {
            return Err("ticker symbol must not be empty".into());
        }
        if ticker.indicators.is_empty() {
            return Err(format!(
                "ticker '{}' must have at least one indicator",
                ticker.symbol
            )
            .into());
        }
        for indicator in &ticker.indicators {
            match indicator {
                IndicatorConfig::Rsi(cfg) => {
                    if !(0.0..=100.0).contains(&cfg.threshold) {
                        return Err(format!(
                            "RSI threshold must be between 0 and 100, got {}",
                            cfg.threshold
                        )
                        .into());
                    }
                }
                IndicatorConfig::Crossover(cfg) => {
                    if cfg.fast_period >= cfg.slow_period {
                        return Err(format!(
                            "crossover fast_period ({}) must be less than slow_period ({})",
                            cfg.fast_period, cfg.slow_period
                        )
                        .into());
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
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

    fn valid_config() -> DaemonConfig {
        DaemonConfig {
            ntfy_url: "https://ntfy.sh/test".to_string(),
            interval_type: "1d".to_string(),
            frequency_seconds: 3600,
            db_path: None,
            tickers: vec![TickerConfig {
                symbol: "TEST".to_string(),
                indicators: vec![IndicatorConfig::Rsi(RsiConfig {
                    threshold: 30.0,
                    period: 14,
                })],
            }],
        }
    }

    #[test]
    fn test_validate_valid_config() {
        assert!(validate_config(&valid_config()).is_ok());
    }

    #[test]
    fn test_validate_empty_ntfy_url() {
        let mut cfg = valid_config();
        cfg.ntfy_url = "   ".to_string();
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("ntfy_url"));
    }

    #[test]
    fn test_validate_bad_interval() {
        let mut cfg = valid_config();
        cfg.interval_type = "1m".to_string();
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("interval_type"));
    }

    #[test]
    fn test_validate_zero_frequency() {
        let mut cfg = valid_config();
        cfg.frequency_seconds = 0;
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("frequency_seconds"));
    }

    #[test]
    fn test_validate_empty_tickers() {
        let mut cfg = valid_config();
        cfg.tickers = vec![];
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("ticker"));
    }

    #[test]
    fn test_validate_empty_symbol() {
        let mut cfg = valid_config();
        cfg.tickers[0].symbol = "".to_string();
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("symbol"));
    }

    #[test]
    fn test_validate_no_indicators() {
        let mut cfg = valid_config();
        cfg.tickers[0].indicators = vec![];
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("indicator"));
    }

    #[test]
    fn test_validate_rsi_threshold_out_of_range() {
        let mut cfg = valid_config();
        cfg.tickers[0].indicators = vec![IndicatorConfig::Rsi(RsiConfig {
            threshold: 150.0,
            period: 14,
        })];
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("RSI threshold"));
    }

    #[test]
    fn test_validate_crossover_periods_reversed() {
        let mut cfg = valid_config();
        cfg.tickers[0].indicators = vec![IndicatorConfig::Crossover(CrossoverConfig {
            fast_period: 30,
            slow_period: 10,
        })];
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("fast_period"));
    }
}
