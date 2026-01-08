// Shared types used across all modules
// Defines core domain entities and enums

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Market Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

impl MarketId {
    #[must_use]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub market_id: MarketId,
    pub question: String,
    pub description: Option<String>,
    pub status: MarketStatus,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketStatus {
    Active,
    Settled,
    Delisted,
}

impl fmt::Display for MarketStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Settled => write!(f, "Settled"),
            Self::Delisted => write!(f, "Delisted"),
        }
    }
}

// ============================================================================
// OrderBook Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub market_id: MarketId,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp: DateTime<Utc>,
}

impl OrderBook {
    #[must_use]
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    #[must_use]
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }

    #[must_use]
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                Some((bid.price + ask.price) / Decimal::from(2))
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn spread_bps(&self) -> Option<i32> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) if bid.price > Decimal::ZERO => {
                let spread = ask.price - bid.price;
                let spread_bps = (spread / bid.price) * Decimal::from(10000);
                spread_bps.to_i32()
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub size: Decimal,
}

// ============================================================================
// Order Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

impl OrderId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub exchange_order_id: Option<String>,
    pub market_id: MarketId,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub filled_size: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub opened_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub paper_trading: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buy => write!(f, "Buy"),
            Self::Sell => write!(f, "Sell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    PendingSubmit,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Failed,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PendingSubmit => write!(f, "pending_submit"),
            Self::Open => write!(f, "open"),
            Self::PartiallyFilled => write!(f, "partially_filled"),
            Self::Filled => write!(f, "filled"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Rejected => write!(f, "rejected"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub fill_id: Uuid,
    pub order_id: OrderId,
    pub exchange_fill_id: Option<String>,
    pub price: Decimal,
    pub size: Decimal,
    pub fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Position Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub market_id: MarketId,
    pub size: Decimal,
    pub avg_entry_price: Option<Decimal>,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub total_pnl: Decimal,
    pub trade_count: u64,
    pub updated_at: DateTime<Utc>,
}

impl Position {
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.size.abs() < Decimal::new(1, 2) // < 0.01
    }

    #[must_use]
    pub fn is_long(&self) -> bool {
        self.size > Decimal::ZERO
    }

    #[must_use]
    pub fn is_short(&self) -> bool {
        self.size < Decimal::ZERO
    }

    pub fn update_unrealized_pnl(&mut self, current_price: Decimal) {
        if let Some(avg_price) = self.avg_entry_price {
            self.unrealized_pnl = (current_price - avg_price) * self.size;
            self.total_pnl = self.realized_pnl + self.unrealized_pnl;
        }
    }
}

// ============================================================================
// Strategy Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub market_id: MarketId,
    pub bid_price: Decimal,
    pub ask_price: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
    pub timestamp: DateTime<Utc>,
}

impl Quote {
    #[must_use]
    pub fn spread_bps(&self) -> Option<i32> {
        if self.bid_price > Decimal::ZERO {
            let spread = self.ask_price - self.bid_price;
            let spread_bps = (spread / self.bid_price) * Decimal::from(10000);
            spread_bps.to_i32()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyDecision {
    Quote,
    Skip,
    CancelAll,
}

// ============================================================================
// Risk Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    pub max_position_per_market: Decimal,
    pub max_total_exposure: Decimal,
    pub max_order_size: Decimal,
    pub max_daily_loss: Decimal,
    pub min_spread_bps: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerReason {
    DailyLossThreshold,
    LossRateExceeded,
    WebSocketTimeout,
    ManualTrigger,
}

impl fmt::Display for CircuitBreakerReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DailyLossThreshold => write!(f, "Daily loss threshold exceeded"),
            Self::LossRateExceeded => write!(f, "Loss rate exceeded"),
            Self::WebSocketTimeout => write!(f, "WebSocket timeout"),
            Self::ManualTrigger => write!(f, "Manual trigger"),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_mid_price() {
        let orderbook = OrderBook {
            market_id: MarketId::new("test".to_string()),
            bids: vec![PriceLevel {
                price: Decimal::new(50, 2), // 0.50
                size: Decimal::new(100, 0),
            }],
            asks: vec![PriceLevel {
                price: Decimal::new(52, 2), // 0.52
                size: Decimal::new(100, 0),
            }],
            timestamp: Utc::now(),
        };

        let mid = orderbook.mid_price().unwrap();
        assert_eq!(mid, Decimal::new(51, 2)); // 0.51
    }

    #[test]
    fn test_orderbook_spread_bps() {
        let orderbook = OrderBook {
            market_id: MarketId::new("test".to_string()),
            bids: vec![PriceLevel {
                price: Decimal::new(50, 2), // 0.50
                size: Decimal::new(100, 0),
            }],
            asks: vec![PriceLevel {
                price: Decimal::new(52, 2), // 0.52
                size: Decimal::new(100, 0),
            }],
            timestamp: Utc::now(),
        };

        let spread = orderbook.spread_bps().unwrap();
        assert_eq!(spread, 400); // 4% spread = 400 bps
    }

    #[test]
    fn test_quote_spread_bps() {
        let quote = Quote {
            market_id: MarketId::new("test".to_string()),
            bid_price: Decimal::new(48, 2), // 0.48
            ask_price: Decimal::new(52, 2), // 0.52
            bid_size: Decimal::new(10, 0),
            ask_size: Decimal::new(10, 0),
            timestamp: Utc::now(),
        };

        let spread = quote.spread_bps().unwrap();
        assert_eq!(spread, 833); // ~8.33% spread
    }

    #[test]
    fn test_position_unrealized_pnl() {
        let mut position = Position {
            market_id: MarketId::new("test".to_string()),
            size: Decimal::new(100, 0),
            avg_entry_price: Some(Decimal::new(50, 2)), // 0.50
            realized_pnl: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            total_pnl: Decimal::ZERO,
            trade_count: 1,
            updated_at: Utc::now(),
        };

        position.update_unrealized_pnl(Decimal::new(55, 2)); // 0.55

        // (0.55 - 0.50) * 100 = 5.00
        assert_eq!(position.unrealized_pnl, Decimal::new(5, 0));
        assert_eq!(position.total_pnl, Decimal::new(5, 0));
    }
}
