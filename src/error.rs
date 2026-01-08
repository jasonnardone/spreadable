// Error types for the application
// Uses thiserror for ergonomic error handling

#![allow(dead_code)]

use std::fmt;
use thiserror::Error;

/// Application-wide error type
#[derive(Error, Debug)]
pub enum SpreadableError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Database errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// WebSocket connection errors
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// API communication errors
    #[error("API error: {0}")]
    Api(String),

    /// Order validation errors
    #[error("Order validation error: {0}")]
    OrderValidation(String),

    /// Risk limit violations
    #[error("Risk limit violation: {0}")]
    RiskViolation(String),

    /// Circuit breaker triggered
    #[error("Circuit breaker triggered: {0}")]
    CircuitBreaker(String),

    /// Strategy errors
    #[error("Strategy error: {0}")]
    Strategy(String),

    /// ML inference errors
    #[error("ML inference error: {0}")]
    MlInference(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP client errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Generic internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, SpreadableError>;

/// WebSocket-specific error details
#[derive(Debug, Clone)]
pub struct WebSocketError {
    pub message: String,
    pub reconnect_attempt: u32,
    pub market_id: Option<String>,
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WebSocket error: {} (attempt {})",
            self.message, self.reconnect_attempt
        )
    }
}

/// Risk violation details
#[derive(Debug, Clone)]
pub struct RiskViolation {
    pub limit_type: RiskLimitType,
    pub limit_value: rust_decimal::Decimal,
    pub actual_value: rust_decimal::Decimal,
    pub market_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLimitType {
    MaxPositionPerMarket,
    MaxTotalExposure,
    MaxOrderSize,
    MaxDailyLoss,
    MinSpread,
}

impl fmt::Display for RiskLimitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxPositionPerMarket => write!(f, "max_position_per_market"),
            Self::MaxTotalExposure => write!(f, "max_total_exposure"),
            Self::MaxOrderSize => write!(f, "max_order_size"),
            Self::MaxDailyLoss => write!(f, "max_daily_loss"),
            Self::MinSpread => write!(f, "min_spread"),
        }
    }
}

impl fmt::Display for RiskViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: limit={}, actual={}",
            self.limit_type, self.limit_value, self.actual_value
        )
    }
}

impl From<RiskViolation> for SpreadableError {
    fn from(violation: RiskViolation) -> Self {
        Self::RiskViolation(violation.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_error_display() {
        let err = SpreadableError::Config("Invalid API key".to_string());
        assert_eq!(err.to_string(), "Configuration error: Invalid API key");
    }

    #[test]
    fn test_risk_violation_display() {
        let violation = RiskViolation {
            limit_type: RiskLimitType::MaxPositionPerMarket,
            limit_value: Decimal::new(100, 0),
            actual_value: Decimal::new(150, 0),
            market_id: Some("0xtest".to_string()),
        };

        assert!(violation
            .to_string()
            .contains("max_position_per_market: limit=100"));
    }

    #[test]
    fn test_websocket_error_display() {
        let ws_err = WebSocketError {
            message: "Connection failed".to_string(),
            reconnect_attempt: 3,
            market_id: Some("0xtest".to_string()),
        };

        assert!(ws_err.to_string().contains("attempt 3"));
    }
}
