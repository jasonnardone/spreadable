// Market data service - coordinates WebSocket, orderbook, and persistence
// Main entry point for market data collection

use crate::config::ExchangeConfig;
use crate::database::DatabaseClient;
use crate::error::Result;
use crate::market_data::orderbook_manager::OrderbookManager;
use crate::market_data::websocket_client::{WebSocketClient, WebSocketEvent};
use crate::types::MarketId;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Market data service
pub struct MarketDataService {
    orderbook_manager: Arc<OrderbookManager>,
    db_client: DatabaseClient,
    ws_event_rx: mpsc::UnboundedReceiver<WebSocketEvent>,
}

impl MarketDataService {
    /// Create a new market data service
    #[allow(unused_variables)]
    pub fn new(
        config: Arc<ExchangeConfig>,
        db_client: DatabaseClient,
        markets: Vec<MarketId>,
    ) -> (Self, WebSocketClient) {
        let orderbook_manager = Arc::new(OrderbookManager::new());

        // Create channel for WebSocket events
        let (ws_event_tx, ws_event_rx) = mpsc::unbounded_channel();

        // Create WebSocket client
        let ws_client = WebSocketClient::new(config, ws_event_tx);

        let service = Self {
            orderbook_manager,
            db_client,
            ws_event_rx,
        };

        (service, ws_client)
    }

    /// Run the market data service (event loop)
    pub async fn run(mut self) -> Result<()> {
        info!("Starting market data service...");

        while let Some(event) = self.ws_event_rx.recv().await {
            if let Err(e) = self.handle_event(event).await {
                error!(error = %e, "Failed to handle market data event");
            }
        }

        info!("Market data service stopped");
        Ok(())
    }

    async fn handle_event(&self, event: WebSocketEvent) -> Result<()> {
        match event {
            WebSocketEvent::Connected => {
                info!("WebSocket connected");
            }

            WebSocketEvent::Disconnected { attempt, retry_in } => {
                info!(
                    attempt,
                    retry_in_secs = retry_in.as_secs(),
                    "WebSocket disconnected, will retry"
                );
            }

            WebSocketEvent::OrderbookSnapshot(snapshot) => {
                // Process snapshot through orderbook manager
                let orderbook = self.orderbook_manager.process_snapshot(snapshot).await?;

                // Persist to database
                crate::database::queries::insert_orderbook_snapshot(
                    self.db_client.pool(),
                    &orderbook.market_id,
                    orderbook.timestamp,
                    orderbook.best_bid().map(|b| b.price),
                    orderbook.best_ask().map(|a| a.price),
                    orderbook.best_bid().map(|b| b.size),
                    orderbook.best_ask().map(|a| a.size),
                    orderbook.mid_price(),
                    orderbook.spread_bps(),
                )
                .await?;

                info!(
                    market_id = %orderbook.market_id,
                    best_bid = ?orderbook.best_bid().map(|b| b.price),
                    best_ask = ?orderbook.best_ask().map(|a| a.price),
                    spread_bps = ?orderbook.spread_bps(),
                    "Orderbook snapshot processed and persisted"
                );
            }

            WebSocketEvent::OrderbookDelta(delta) => {
                // Process delta through orderbook manager
                let orderbook = self.orderbook_manager.process_delta(delta).await?;

                // Persist updated state to database
                crate::database::queries::insert_orderbook_snapshot(
                    self.db_client.pool(),
                    &orderbook.market_id,
                    orderbook.timestamp,
                    orderbook.best_bid().map(|b| b.price),
                    orderbook.best_ask().map(|a| a.price),
                    orderbook.best_bid().map(|b| b.size),
                    orderbook.best_ask().map(|a| a.size),
                    orderbook.mid_price(),
                    orderbook.spread_bps(),
                )
                .await?;
            }

            WebSocketEvent::Trade(trade) => {
                info!(
                    market_id = %trade.market_id,
                    price = %trade.price,
                    size = %trade.size,
                    side = %trade.side,
                    "Trade event received"
                );
                // TODO: Process trade for volume metrics
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
    // Require database and mock WebSocket server
}
