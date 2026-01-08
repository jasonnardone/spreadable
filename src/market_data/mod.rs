// Market data module
// WebSocket client, orderbook management, and market data service

mod orderbook_manager;
mod service;
mod websocket_client;

pub use service::MarketDataService;
