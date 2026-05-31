use crate::config::IndicatorConfig;
use crate::data::MarketData;

pub struct IndicatorResult {
    pub triggered: bool,
    pub alert_message: Option<String>,
    pub log_label: String,
}

pub trait Indicator: Send + Sync {
    fn evaluate(&self, data: &MarketData) -> IndicatorResult;
    fn name(&self) -> &str;
}

mod bb;
mod crossover;
mod macd;
mod rsi;

impl Indicator for IndicatorConfig {
    fn evaluate(&self, data: &MarketData) -> IndicatorResult {
        match self {
            IndicatorConfig::Rsi(cfg) => cfg.evaluate(data),
            IndicatorConfig::BollingerBands(cfg) => cfg.evaluate(data),
            IndicatorConfig::Macd(cfg) => cfg.evaluate(data),
            IndicatorConfig::Crossover(cfg) => cfg.evaluate(data),
        }
    }

    fn name(&self) -> &str {
        match self {
            IndicatorConfig::Rsi(_) => "RSI",
            IndicatorConfig::BollingerBands(_) => "Bollinger Bands",
            IndicatorConfig::Macd(_) => "MACD",
            IndicatorConfig::Crossover(_) => "MA Crossover",
        }
    }
}

pub use rsi::calculate_rsi;
