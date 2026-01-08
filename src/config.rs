// Configuration loading and validation
// Loads from TOML file with environment variable substitution

use crate::error::{Result, SpreadableError};
use config::{Config, Environment, File};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub exchange: ExchangeConfig,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub markets: MarketsConfig,
    pub database: DatabaseConfig,
    pub monitoring: MonitoringConfig,
    pub logging: LoggingConfig,
    pub ml: MlConfig,
}

impl AppConfig {
    /// Load configuration from file with environment variable substitution
    pub fn load(config_path: impl AsRef<Path>) -> Result<Self> {
        let config = Config::builder()
            .add_source(File::from(config_path.as_ref()))
            .add_source(Environment::with_prefix("SPREADABLE").separator("_"))
            .build()
            .map_err(|e| SpreadableError::Config(format!("Failed to load config: {e}")))?;

        let mut app_config: Self = config
            .try_deserialize()
            .map_err(|e| SpreadableError::Config(format!("Failed to parse config: {e}")))?;

        // Override with direct environment variables if present (for convenience)
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            app_config.database.postgres_url = db_url;
        }
        if let Ok(api_key) = std::env::var("POLYMARKET_API_KEY") {
            app_config.exchange.api_key = api_key;
        }
        if let Ok(api_secret) = std::env::var("POLYMARKET_API_SECRET") {
            app_config.exchange.api_secret = api_secret;
        }

        app_config.validate()?;
        Ok(app_config)
    }

    /// Validate configuration constraints
    fn validate(&self) -> Result<()> {
        // Validate risk limits
        if self.risk.max_position_per_market <= Decimal::ZERO {
            return Err(SpreadableError::Config(
                "max_position_per_market must be > 0".to_string(),
            ));
        }

        if self.risk.max_total_exposure <= Decimal::ZERO {
            return Err(SpreadableError::Config(
                "max_total_exposure must be > 0".to_string(),
            ));
        }

        if self.risk.max_order_size <= Decimal::ZERO {
            return Err(SpreadableError::Config(
                "max_order_size must be > 0".to_string(),
            ));
        }

        if self.risk.max_order_size > self.risk.max_position_per_market {
            return Err(SpreadableError::Config(
                "max_order_size cannot exceed max_position_per_market".to_string(),
            ));
        }

        if self.risk.min_spread_bps < 0 {
            return Err(SpreadableError::Config(
                "min_spread_bps must be >= 0".to_string(),
            ));
        }

        // Validate strategy configuration
        self.strategy.validate()?;

        // Validate monitoring ports don't conflict
        if self.monitoring.metrics_bind_addr == self.monitoring.health_bind_addr {
            return Err(SpreadableError::Config(
                "metrics and health endpoints cannot use same address".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeConfig {
    pub websocket_url: String,
    pub rest_api_url: String,
    pub api_key: String,
    pub api_secret: String,
    pub reconnect_interval_ms: u64,
    pub max_reconnect_interval_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub heartbeat_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyConfig {
    pub active: String,
    pub quote_refresh_interval_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic_mm: Option<BasicMmConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_spread: Option<AdaptiveSpreadConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ml_enhanced: Option<MlEnhancedConfig>,
}

impl StrategyConfig {
    fn validate(&self) -> Result<()> {
        match self.active.as_str() {
            "basic_mm" => {
                if self.basic_mm.is_none() {
                    return Err(SpreadableError::Config(
                        "basic_mm strategy selected but not configured".to_string(),
                    ));
                }
            }
            "adaptive_spread" => {
                if self.adaptive_spread.is_none() {
                    return Err(SpreadableError::Config(
                        "adaptive_spread strategy selected but not configured".to_string(),
                    ));
                }
            }
            "ml_enhanced" => {
                if self.ml_enhanced.is_none() {
                    return Err(SpreadableError::Config(
                        "ml_enhanced strategy selected but not configured".to_string(),
                    ));
                }
            }
            _ => {
                return Err(SpreadableError::Config(format!(
                    "Unknown strategy: {}",
                    self.active
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BasicMmConfig {
    pub base_spread_bps: i32,
    pub quote_size: Decimal,
    pub max_position: Decimal,
    pub skew_factor: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdaptiveSpreadConfig {
    pub base_spread_bps: i32,
    pub volatility_multiplier: Decimal,
    pub max_spread_bps: i32,
    pub min_spread_bps: i32,
    pub quote_size: Decimal,
    pub max_position: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MlEnhancedConfig {
    pub ollama_endpoint: String,
    pub model_name: String,
    pub inference_timeout_ms: u64,
    pub fallback_strategy: String,
    pub confidence_threshold: Decimal,
    pub quote_size: Decimal,
    pub max_position: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskConfig {
    pub max_position_per_market: Decimal,
    pub max_total_exposure: Decimal,
    pub max_order_size: Decimal,
    pub max_daily_loss: Decimal,
    pub min_spread_bps: i32,
    pub paper_trading_mode: bool,
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub daily_loss_threshold: Decimal,
    pub loss_rate_pct: Decimal,
    pub loss_rate_window_secs: u64,
    pub websocket_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketsConfig {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub postgres_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub statement_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub metrics_bind_addr: String,
    pub health_bind_addr: String,
    pub metrics_interval_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file_path: String,
    pub file_rotation: String,
    pub file_max_size_mb: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MlConfig {
    pub ollama_endpoint: String,
    pub model_name: String,
    pub inference_enabled: bool,
    pub inference_timeout_ms: u64,
    pub max_retries: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_config() -> String {
        r#"
[exchange]
websocket_url = "wss://test.example.com/ws"
rest_api_url = "https://test.example.com/api/v1"
api_key = "test_key"
api_secret = "test_secret"
reconnect_interval_ms = 1000
max_reconnect_interval_ms = 60000
heartbeat_interval_ms = 30000
heartbeat_timeout_ms = 10000

[strategy]
active = "basic_mm"
quote_refresh_interval_ms = 2000

[strategy.basic_mm]
base_spread_bps = 500
quote_size = 10.0
max_position = 100.0
skew_factor = 0.5

[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
max_order_size = 20.0
max_daily_loss = 50.0
min_spread_bps = 200
paper_trading_mode = true

[risk.circuit_breaker]
enabled = true
daily_loss_threshold = 50.0
loss_rate_pct = 0.05
loss_rate_window_secs = 900
websocket_timeout_secs = 60

[markets]
enabled = []

[database]
postgres_url = "postgresql://test:test@localhost/test"
max_connections = 10
min_connections = 2
connection_timeout_secs = 30
idle_timeout_secs = 600
statement_timeout_ms = 5000

[monitoring]
metrics_bind_addr = "0.0.0.0:9090"
health_bind_addr = "0.0.0.0:8080"
metrics_interval_ms = 1000

[logging]
level = "info"
format = "json"
file_path = "logs/test.log"
file_rotation = "daily"
file_max_size_mb = 100

[ml]
ollama_endpoint = "http://localhost:11434"
model_name = "test-model"
inference_enabled = false
inference_timeout_ms = 500
max_retries = 3
"#
        .to_string()
    }

    #[test]
    fn test_load_valid_config() {
        let config_str = create_test_config();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = AppConfig::load(temp_file.path()).unwrap();

        assert_eq!(config.exchange.api_key, "test_key");
        assert_eq!(config.strategy.active, "basic_mm");
        assert_eq!(config.risk.paper_trading_mode, true);
        assert_eq!(config.markets.enabled.len(), 0);
    }

    #[test]
    fn test_invalid_risk_limits() {
        let mut config_str = create_test_config();
        // Set max_order_size > max_position_per_market
        config_str = config_str.replace(
            "max_order_size = 20.0",
            "max_order_size = 200.0",
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = AppConfig::load(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_order_size cannot exceed"));
    }

    #[test]
    fn test_missing_strategy_config() {
        let config_str = create_test_config()
            .replace("[strategy.basic_mm]", "# [strategy.basic_mm]")
            .replace("base_spread_bps = 500", "# base_spread_bps = 500")
            .replace("quote_size = 10.0", "# quote_size = 10.0")
            .replace("max_position = 100.0", "# max_position = 100.0")
            .replace("skew_factor = 0.5", "# skew_factor = 0.5");

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = AppConfig::load(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("basic_mm strategy selected but not configured"));
    }
}
