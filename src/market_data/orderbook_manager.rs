// Orderbook manager - maintains local orderbook state
// Applies snapshots and deltas from WebSocket

use crate::error::{Result, SpreadableError};
use crate::market_data::websocket_client::{
    OrderbookChange, OrderbookDeltaMsg, OrderbookSnapshotMsg,
};
use crate::types::{MarketId, OrderBook, PriceLevel};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Manages orderbooks for multiple markets
pub struct OrderbookManager {
    orderbooks: Arc<RwLock<HashMap<MarketId, OrderBook>>>,
}

impl OrderbookManager {
    /// Create a new orderbook manager
    pub fn new() -> Self {
        Self {
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process orderbook snapshot (full state replacement)
    pub async fn process_snapshot(&self, snapshot: OrderbookSnapshotMsg) -> Result<OrderBook> {
        let market_id = MarketId::new(snapshot.market_id.clone());

        // Parse bids (sorted descending by price)
        let mut bids = Vec::new();
        for (price_str, size_str) in snapshot.bids {
            let price = Decimal::from_str(&price_str).map_err(|e| {
                SpreadableError::Internal(format!("Invalid bid price: {e}"))
            })?;
            let size = Decimal::from_str(&size_str).map_err(|e| {
                SpreadableError::Internal(format!("Invalid bid size: {e}"))
            })?;
            bids.push(PriceLevel { price, size });
        }
        bids.sort_by(|a, b| b.price.cmp(&a.price)); // Descending

        // Parse asks (sorted ascending by price)
        let mut asks = Vec::new();
        for (price_str, size_str) in snapshot.asks {
            let price = Decimal::from_str(&price_str).map_err(|e| {
                SpreadableError::Internal(format!("Invalid ask price: {e}"))
            })?;
            let size = Decimal::from_str(&size_str).map_err(|e| {
                SpreadableError::Internal(format!("Invalid ask size: {e}"))
            })?;
            asks.push(PriceLevel { price, size });
        }
        asks.sort_by(|a, b| a.price.cmp(&b.price)); // Ascending

        let orderbook = OrderBook {
            market_id: market_id.clone(),
            bids,
            asks,
            timestamp: DateTime::from_timestamp_millis(snapshot.timestamp)
                .unwrap_or_else(Utc::now),
        };

        // Store in map
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.insert(market_id.clone(), orderbook.clone());

        debug!(
            market_id = %market_id,
            bid_levels = orderbook.bids.len(),
            ask_levels = orderbook.asks.len(),
            "Orderbook snapshot processed"
        );

        Ok(orderbook)
    }

    /// Process orderbook delta (incremental update)
    pub async fn process_delta(&self, delta: OrderbookDeltaMsg) -> Result<OrderBook> {
        let market_id = MarketId::new(delta.market_id.clone());

        let mut orderbooks = self.orderbooks.write().await;

        let orderbook = orderbooks.get_mut(&market_id).ok_or_else(|| {
            SpreadableError::Internal(format!(
                "No orderbook snapshot for market: {}",
                market_id
            ))
        })?;

        // Apply each change
        for change in delta.changes {
            apply_change(orderbook, change)?;
        }

        // Update timestamp
        orderbook.timestamp =
            DateTime::from_timestamp_millis(delta.timestamp).unwrap_or_else(Utc::now);

        debug!(
            market_id = %market_id,
            bid_levels = orderbook.bids.len(),
            ask_levels = orderbook.asks.len(),
            "Orderbook delta applied"
        );

        Ok(orderbook.clone())
    }

    /// Get current orderbook for a market
    pub async fn get_orderbook(&self, market_id: &MarketId) -> Option<OrderBook> {
        let orderbooks = self.orderbooks.read().await;
        orderbooks.get(market_id).cloned()
    }

    /// Get all current orderbooks
    pub async fn get_all_orderbooks(&self) -> Vec<OrderBook> {
        let orderbooks = self.orderbooks.read().await;
        orderbooks.values().cloned().collect()
    }

    /// Clear orderbook for a market (on unsubscribe)
    pub async fn clear_orderbook(&self, market_id: &MarketId) {
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.remove(market_id);
        debug!(market_id = %market_id, "Orderbook cleared");
    }
}

impl Default for OrderbookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply a single orderbook change (add, update, or remove price level)
fn apply_change(orderbook: &mut OrderBook, change: OrderbookChange) -> Result<()> {
    let price = Decimal::from_str(&change.price)
        .map_err(|e| SpreadableError::Internal(format!("Invalid price: {e}")))?;

    let size = Decimal::from_str(&change.size)
        .map_err(|e| SpreadableError::Internal(format!("Invalid size: {e}")))?;

    let levels = match change.side.as_str() {
        "bid" => &mut orderbook.bids,
        "ask" => &mut orderbook.asks,
        _ => {
            warn!(side = %change.side, "Unknown orderbook side");
            return Ok(());
        }
    };

    if size == Decimal::ZERO {
        // Remove price level
        levels.retain(|level| level.price != price);
    } else if let Some(level) = levels.iter_mut().find(|l| l.price == price) {
        // Update existing level
        level.size = size;
    } else {
        // Insert new level
        levels.push(PriceLevel { price, size });

        // Re-sort (bids descending, asks ascending)
        match change.side.as_str() {
            "bid" => levels.sort_by(|a, b| b.price.cmp(&a.price)),
            "ask" => levels.sort_by(|a, b| a.price.cmp(&b.price)),
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_snapshot() {
        let manager = OrderbookManager::new();

        let snapshot = OrderbookSnapshotMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441600000,
            bids: vec![
                ("0.50".to_string(), "1000".to_string()),
                ("0.49".to_string(), "500".to_string()),
            ],
            asks: vec![
                ("0.51".to_string(), "800".to_string()),
                ("0.52".to_string(), "600".to_string()),
            ],
        };

        let orderbook = manager.process_snapshot(snapshot).await.unwrap();

        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);

        // Verify bids sorted descending
        assert!(orderbook.bids[0].price > orderbook.bids[1].price);

        // Verify asks sorted ascending
        assert!(orderbook.asks[0].price < orderbook.asks[1].price);

        // Verify mid price
        let mid = orderbook.mid_price().unwrap();
        assert_eq!(mid, Decimal::new(505, 3)); // (0.50 + 0.51) / 2 = 0.505
    }

    #[tokio::test]
    async fn test_process_delta_update_level() {
        let manager = OrderbookManager::new();

        // Initial snapshot
        let snapshot = OrderbookSnapshotMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441600000,
            bids: vec![("0.50".to_string(), "1000".to_string())],
            asks: vec![("0.51".to_string(), "800".to_string())],
        };

        manager.process_snapshot(snapshot).await.unwrap();

        // Update bid size
        let delta = OrderbookDeltaMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441601000,
            changes: vec![OrderbookChange {
                side: "bid".to_string(),
                price: "0.50".to_string(),
                size: "1200".to_string(), // Updated size
            }],
        };

        let orderbook = manager.process_delta(delta).await.unwrap();

        assert_eq!(orderbook.bids[0].size, Decimal::new(1200, 0));
    }

    #[tokio::test]
    async fn test_process_delta_remove_level() {
        let manager = OrderbookManager::new();

        // Initial snapshot
        let snapshot = OrderbookSnapshotMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441600000,
            bids: vec![
                ("0.50".to_string(), "1000".to_string()),
                ("0.49".to_string(), "500".to_string()),
            ],
            asks: vec![("0.51".to_string(), "800".to_string())],
        };

        manager.process_snapshot(snapshot).await.unwrap();

        // Remove bid level (size = 0)
        let delta = OrderbookDeltaMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441601000,
            changes: vec![OrderbookChange {
                side: "bid".to_string(),
                price: "0.50".to_string(),
                size: "0".to_string(),
            }],
        };

        let orderbook = manager.process_delta(delta).await.unwrap();

        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(orderbook.bids[0].price, Decimal::new(49, 2)); // 0.49
    }

    #[tokio::test]
    async fn test_process_delta_add_level() {
        let manager = OrderbookManager::new();

        // Initial snapshot
        let snapshot = OrderbookSnapshotMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441600000,
            bids: vec![("0.50".to_string(), "1000".to_string())],
            asks: vec![("0.51".to_string(), "800".to_string())],
        };

        manager.process_snapshot(snapshot).await.unwrap();

        // Add new bid level
        let delta = OrderbookDeltaMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441601000,
            changes: vec![OrderbookChange {
                side: "bid".to_string(),
                price: "0.48".to_string(), // New level
                size: "750".to_string(),
            }],
        };

        let orderbook = manager.process_delta(delta).await.unwrap();

        assert_eq!(orderbook.bids.len(), 2);

        // Verify still sorted descending
        assert!(orderbook.bids[0].price > orderbook.bids[1].price);
        assert_eq!(orderbook.bids[1].price, Decimal::new(48, 2)); // 0.48
    }

    #[tokio::test]
    async fn test_spread_calculation() {
        let manager = OrderbookManager::new();

        let snapshot = OrderbookSnapshotMsg {
            market_id: "0xtest".to_string(),
            timestamp: 1704441600000,
            bids: vec![("0.50".to_string(), "1000".to_string())],
            asks: vec![("0.52".to_string(), "800".to_string())],
        };

        let orderbook = manager.process_snapshot(snapshot).await.unwrap();

        let spread_bps = orderbook.spread_bps().unwrap();
        assert_eq!(spread_bps, 400); // (0.52 - 0.50) / 0.50 * 10000 = 400 bps (4%)
    }
}
