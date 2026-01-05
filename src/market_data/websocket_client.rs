// WebSocket client for Polymarket CLOB API
// Handles connection, authentication, subscriptions, and reconnection

use crate::config::ExchangeConfig;
use crate::error::{Result, SpreadableError};
use crate::types::MarketId;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};

type HmacSha256 = Hmac<Sha256>;

/// WebSocket client for market data streaming
pub struct WebSocketClient {
    config: Arc<ExchangeConfig>,
    event_tx: mpsc::UnboundedSender<WebSocketEvent>,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    pub fn new(
        config: Arc<ExchangeConfig>,
        event_tx: mpsc::UnboundedSender<WebSocketEvent>,
    ) -> Self {
        Self { config, event_tx }
    }

    /// Start the WebSocket client (runs until cancelled)
    pub async fn run(&self, markets: Vec<MarketId>) -> Result<()> {
        let mut reconnect_delay = Duration::from_millis(self.config.reconnect_interval_ms);
        let max_delay = Duration::from_millis(self.config.max_reconnect_interval_ms);
        let mut attempt = 0;

        loop {
            info!(attempt, "Attempting WebSocket connection...");

            match self.connect_and_run(&markets).await {
                Ok(()) => {
                    info!("WebSocket connection closed normally");
                    reconnect_delay = Duration::from_millis(self.config.reconnect_interval_ms);
                    attempt = 0;
                }
                Err(e) => {
                    attempt += 1;
                    error!(
                        error = %e,
                        attempt,
                        retry_in_secs = reconnect_delay.as_secs(),
                        "WebSocket connection failed"
                    );

                    // Notify listeners of disconnection
                    let _ = self.event_tx.send(WebSocketEvent::Disconnected {
                        attempt,
                        retry_in: reconnect_delay,
                    });

                    // Wait before reconnecting
                    sleep(reconnect_delay).await;

                    // Exponential backoff
                    reconnect_delay = std::cmp::min(reconnect_delay * 2, max_delay);
                }
            }
        }
    }

    async fn connect_and_run(&self, markets: &[MarketId]) -> Result<()> {
        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&self.config.websocket_url)
            .await
            .map_err(|e| SpreadableError::WebSocket(format!("Connection failed: {e}")))?;

        info!("WebSocket connected to {}", self.config.websocket_url);

        let (mut write, mut read) = ws_stream.split();

        // Authenticate
        let auth_msg = self.create_auth_message()?;
        write
            .send(Message::Text(auth_msg))
            .await
            .map_err(|e| SpreadableError::WebSocket(format!("Auth send failed: {e}")))?;

        debug!("Authentication message sent");

        // Wait for auth response
        if let Some(msg) = read.next().await {
            let msg = msg.map_err(|e| {
                SpreadableError::WebSocket(format!("Auth response error: {e}"))
            })?;

            if let Message::Text(text) = msg {
                let response: serde_json::Value = serde_json::from_str(&text)?;
                if response["type"] == "auth_response" && response["status"] == "success" {
                    info!("WebSocket authentication successful");
                } else {
                    return Err(SpreadableError::WebSocket(format!(
                        "Authentication failed: {text}"
                    )));
                }
            }
        }

        // Subscribe to markets
        for market_id in markets {
            let subscribe_msg = self.create_subscribe_message(market_id)?;
            write
                .send(Message::Text(subscribe_msg))
                .await
                .map_err(|e| {
                    SpreadableError::WebSocket(format!("Subscribe send failed: {e}"))
                })?;

            debug!(market_id = %market_id, "Subscribed to market");
        }

        // Notify connected
        let _ = self.event_tx.send(WebSocketEvent::Connected);

        // Set up heartbeat interval
        let mut heartbeat_interval =
            interval(Duration::from_millis(self.config.heartbeat_interval_ms));

        // Message loop
        loop {
            tokio::select! {
                // Incoming messages
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_message(&text).await {
                                error!(error = %e, "Failed to handle message");
                            }
                        }
                        Some(Ok(Message::Ping(_))) => {
                            debug!("Received ping");
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket closed by server");
                            break;
                        }
                        Some(Err(e)) => {
                            error!(error = %e, "WebSocket read error");
                            return Err(SpreadableError::WebSocket(format!("Read error: {e}")));
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }

                // Send heartbeat
                _ = heartbeat_interval.tick() => {
                    let ping_msg = self.create_ping_message();
                    if let Err(e) = write.send(Message::Text(ping_msg)).await {
                        error!(error = %e, "Failed to send heartbeat");
                        return Err(SpreadableError::WebSocket(format!("Heartbeat failed: {e}")));
                    }
                    debug!("Heartbeat sent");
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&self, text: &str) -> Result<()> {
        let value: serde_json::Value = serde_json::from_str(text)?;

        match value["type"].as_str() {
            Some("orderbook_snapshot") => {
                let snapshot: OrderbookSnapshotMsg = serde_json::from_str(text)?;
                let _ = self
                    .event_tx
                    .send(WebSocketEvent::OrderbookSnapshot(snapshot));
            }
            Some("orderbook_delta") => {
                let delta: OrderbookDeltaMsg = serde_json::from_str(text)?;
                let _ = self.event_tx.send(WebSocketEvent::OrderbookDelta(delta));
            }
            Some("trade") => {
                let trade: TradeMsg = serde_json::from_str(text)?;
                let _ = self.event_tx.send(WebSocketEvent::Trade(trade));
            }
            Some("pong") => {
                debug!("Received pong response");
            }
            Some("subscribed") => {
                info!(
                    market_id = value["market_id"].as_str().unwrap_or("unknown"),
                    "Market subscription confirmed"
                );
            }
            Some("error") => {
                warn!(
                    code = value["code"].as_str().unwrap_or("unknown"),
                    message = value["message"].as_str().unwrap_or("unknown"),
                    "WebSocket error message"
                );
            }
            _ => {
                debug!(message_type = value["type"].as_str(), "Unknown message type");
            }
        }

        Ok(())
    }

    fn create_auth_message(&self) -> Result<String> {
        let timestamp = Utc::now().timestamp_millis();
        let signature = self.sign_request(timestamp, "auth", "", "")?;

        let msg = serde_json::json!({
            "type": "auth",
            "api_key": self.config.api_key,
            "signature": signature,
            "timestamp": timestamp
        });

        Ok(msg.to_string())
    }

    fn create_subscribe_message(&self, market_id: &MarketId) -> Result<String> {
        let msg = serde_json::json!({
            "type": "subscribe",
            "channel": "orderbook",
            "market_id": market_id.as_str()
        });

        Ok(msg.to_string())
    }

    fn create_ping_message(&self) -> String {
        let timestamp = Utc::now().timestamp_millis();
        serde_json::json!({
            "type": "ping",
            "timestamp": timestamp
        })
        .to_string()
    }

    fn sign_request(
        &self,
        timestamp: i64,
        method: &str,
        path: &str,
        body: &str,
    ) -> Result<String> {
        let message = format!("{}{}{}{}", timestamp, method, path, body);

        let mut mac = HmacSha256::new_from_slice(self.config.api_secret.as_bytes())
            .map_err(|e| SpreadableError::Internal(format!("HMAC key error: {e}")))?;

        mac.update(message.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}

// ============================================================================
// WebSocket Events
// ============================================================================

#[derive(Debug, Clone)]
pub enum WebSocketEvent {
    Connected,
    Disconnected {
        attempt: u32,
        retry_in: Duration,
    },
    OrderbookSnapshot(OrderbookSnapshotMsg),
    OrderbookDelta(OrderbookDeltaMsg),
    Trade(TradeMsg),
}

// ============================================================================
// Message Types (from Polymarket API contract)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderbookSnapshotMsg {
    pub market_id: String,
    pub timestamp: i64,
    pub bids: Vec<(String, String)>, // [price, size]
    pub asks: Vec<(String, String)>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderbookDeltaMsg {
    pub market_id: String,
    pub timestamp: i64,
    pub changes: Vec<OrderbookChange>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderbookChange {
    pub side: String, // "bid" or "ask"
    pub price: String,
    pub size: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeMsg {
    pub market_id: String,
    pub timestamp: i64,
    pub trade_id: String,
    pub price: String,
    pub size: String,
    pub side: String, // "buy" or "sell"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_request() {
        let config = Arc::new(ExchangeConfig {
            websocket_url: "wss://test.com".to_string(),
            rest_api_url: "https://test.com".to_string(),
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            reconnect_interval_ms: 1000,
            max_reconnect_interval_ms: 60000,
            heartbeat_interval_ms: 30000,
            heartbeat_timeout_ms: 10000,
        });

        let (tx, _rx) = mpsc::unbounded_channel();
        let client = WebSocketClient::new(config, tx);

        let signature = client.sign_request(1704441600000, "auth", "", "");
        assert!(signature.is_ok());

        let sig = signature.unwrap();
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // SHA256 hex output is 64 chars
    }

    #[test]
    fn test_parse_orderbook_snapshot() {
        let json = r#"{
            "type": "orderbook_snapshot",
            "market_id": "0xtest",
            "timestamp": 1704441600000,
            "bids": [["0.50", "1000"]],
            "asks": [["0.51", "800"]]
        }"#;

        let snapshot: OrderbookSnapshotMsg = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.market_id, "0xtest");
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.asks.len(), 1);
    }

    #[test]
    fn test_parse_orderbook_delta() {
        let json = r#"{
            "type": "orderbook_delta",
            "market_id": "0xtest",
            "timestamp": 1704441601000,
            "changes": [
                {"side": "bid", "price": "0.52", "size": "1200.00"}
            ]
        }"#;

        let delta: OrderbookDeltaMsg = serde_json::from_str(json).unwrap();
        assert_eq!(delta.market_id, "0xtest");
        assert_eq!(delta.changes.len(), 1);
        assert_eq!(delta.changes[0].side, "bid");
    }
}
