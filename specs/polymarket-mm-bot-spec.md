# Polymarket Market-Making Bot - Technical Specification

**Version**: 1.0.0  
**Last Updated**: 2026-01-05  
**Target Platform**: Polymarket Prediction Markets  
**Purpose**: Automated market-making with ML-driven strategy optimization  
**Spec Format**: Github Speckit Compatible

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Architecture](#2-architecture)
3. [Core Components](#3-core-components)
4. [Data Models](#4-data-models)
5. [API Specifications](#5-api-specifications)
6. [Configuration](#6-configuration)
7. [Deployment](#7-deployment)
8. [Implementation Phases](#8-implementation-phases)
9. [Testing Strategy](#9-testing-strategy)
10. [Monitoring & Observability](#10-monitoring--observability)
11. [Security & Risk Management](#11-security--risk-management)
12. [ML Integration](#12-ml-integration)

---

## 1. System Overview

### 1.1 Project Goals

Build a low-latency, automated market-making bot for Polymarket that:
- Executes trades with <100ms latency
- Supports 10-50 concurrent markets
- Collects comprehensive data for ML model training
- Operates hands-off with robust risk management
- Integrates with local Ollama models for intelligent decision-making

### 1.2 Key Requirements

**Functional Requirements**:
- Real-time market data consumption via WebSocket
- Automated bid/ask quote generation and placement
- Multi-market support with independent strategies per market
- Position and risk management with circuit breakers
- Historical data collection for backtesting and ML training
- Configuration-driven behavior (no code changes for adjustments)
- Paper trading mode for safe testing

**Non-Functional Requirements**:
- **Latency**: Order placement <100ms from signal to exchange
- **Reliability**: 99.9% uptime
- **Scalability**: Handle 50+ markets simultaneously
- **Data Integrity**: Zero data loss, all events logged
- **Recoverability**: Automatic restart and state recovery
- **Observability**: Real-time metrics and alerting

### 1.3 Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Core Engine | Rust | Low latency, memory safety, excellent async |
| Strategy Backtesting | Python | Rich data science ecosystem |
| Database | PostgreSQL + TimescaleDB | Reliable, time-series optimized |
| WebSocket Client | tokio-tungstenite | High-performance async Rust |
| Configuration | TOML + config crate | Type-safe, hierarchical configs |
| Monitoring | Prometheus + Grafana | Industry standard, rich ecosystem |
| ML Framework | Ollama | Local LLM hosting, privacy-preserving |
| Container Runtime | Docker + Docker Compose | Consistent environments |
| Message Queue | Redis (optional) | Order queueing, caching |

---

## 2. Architecture

### 2.1 System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Configuration Layer                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ TOML Configs │  │  Web UI      │  │ Hot Reload   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Core Trading Engine (Rust)                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Strategy   │  │     Risk     │  │   Position   │     │
│  │   Manager    │  │   Manager    │  │   Tracker    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │     OMS      │  │  Market Data │  │   Logger     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              Exchange Integration Layer                      │
│  ┌──────────────────────────────────────────────────┐      │
│  │         Polymarket CLOB API Connector             │      │
│  │  ┌─────────────┐  ┌─────────────┐               │      │
│  │  │  WebSocket  │  │  REST API   │               │      │
│  │  │   Client    │  │   Client    │               │      │
│  │  └─────────────┘  └─────────────┘               │      │
│  └──────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   Data & Analytics Layer                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  PostgreSQL  │  │  TimescaleDB │  │   Parquet    │     │
│  │  (Relational)│  │ (Time-Series)│  │   (Export)   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                  ML Training Pipeline (Python)               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Feature    │  │    Ollama    │  │  Backtest    │     │
│  │ Engineering  │  │   Training   │  │   Engine     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              Monitoring & Alerting Layer                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Prometheus  │  │   Grafana    │  │  PagerDuty   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow

```
Market Data Flow:
Polymarket → WebSocket → Parser → Orderbook Manager → Strategy Engine → OMS → Exchange

Order Flow:
Strategy Signal → Risk Check → OMS Queue → Rate Limiter → Exchange API → Fill Handler → Database

ML Training Flow:
Database → Feature Extractor → Training Pipeline → Ollama → Model Evaluation → Deployment
```

### 2.3 Module Dependencies

```
main
├── config (loads TOML)
├── market_data
│   ├── websocket_client
│   ├── orderbook_manager
│   └── market_state
├── strategy
│   ├── base_strategy (trait)
│   ├── basic_mm
│   ├── adaptive_spread
│   └── ml_enhanced
├── oms
│   ├── order_manager
│   ├── order_queue
│   └── rate_limiter
├── risk
│   ├── pre_trade_checks
│   ├── position_limiter
│   └── circuit_breaker
├── database
│   ├── postgres_client
│   └── schema
└── monitoring
    ├── metrics
    └── logger
```

---

## 3. Core Components

### 3.1 Market Data Handler

**Module**: `src/market_data/`

**Responsibilities**:
- Establish and maintain WebSocket connection to Polymarket CLOB
- Parse incoming market data messages
- Maintain accurate local orderbook state
- Emit market data events to strategy engine
- Handle reconnection logic with exponential backoff

**Key Interfaces**:

```rust
// src/market_data/mod.rs

pub trait MarketDataHandler: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, market_ids: Vec<String>) -> Result<()>;
    async fn get_orderbook(&self, market_id: &str) -> Option<OrderBook>;
    fn stream(&self) -> Receiver<MarketDataEvent>;
}

pub enum MarketDataEvent {
    OrderBookUpdate(OrderBookUpdate),
    Trade(Trade),
    Ticker(Ticker),
    ConnectionStatus(ConnectionStatus),
}

pub struct OrderBook {
    pub market_id: String,
    pub timestamp: i64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub sequence: u64,
}

pub struct PriceLevel {
    pub price: Decimal,
    pub size: Decimal,
}
```

**Configuration**:
```toml
[market_data]
websocket_url = "wss://clob.polymarket.com/ws"
reconnect_delay_ms = 1000
max_reconnect_attempts = 10
heartbeat_interval_ms = 30000
orderbook_depth = 20  # Number of price levels to track
```

**Performance Targets**:
- Message processing: <5ms per message
- Orderbook update latency: <10ms
- Reconnection time: <2 seconds

**Error Handling**:
- Automatic reconnection on disconnect
- State recovery from last known sequence
- Logging all connection issues
- Alert on prolonged disconnection (>30s)

### 3.2 Strategy Engine

**Module**: `src/strategy/`

**Responsibilities**:
- Implement market-making logic
- Generate bid/ask quotes based on market conditions
- Manage inventory and position risk
- Adapt to changing market dynamics
- Integrate ML model predictions (optional)

**Key Interfaces**:

```rust
// src/strategy/base_strategy.rs

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    
    async fn on_market_data(&mut self, event: &MarketDataEvent) -> Result<Vec<Order>>;
    
    async fn on_fill(&mut self, fill: &Fill) -> Result<Vec<Order>>;
    
    async fn on_timer(&mut self) -> Result<Vec<Order>>;
    
    fn get_position(&self, market_id: &str) -> Position;
    
    fn calculate_quotes(&self, orderbook: &OrderBook, position: &Position) -> Quotes;
}

pub struct Quotes {
    pub bid_price: Decimal,
    pub bid_size: Decimal,
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub confidence: f64,
}

pub struct Position {
    pub market_id: String,
    pub size: Decimal,  // Positive = long, negative = short
    pub avg_entry_price: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
}
```

**Strategy Implementations**:

#### 3.2.1 Basic Market Making Strategy

```rust
// src/strategy/basic_mm.rs

pub struct BasicMarketMaker {
    config: BasicMMConfig,
    positions: HashMap<String, Position>,
}

pub struct BasicMMConfig {
    pub base_spread_bps: u32,      // Base spread in basis points
    pub quote_size: Decimal,        // Size of each quote
    pub max_position: Decimal,      // Maximum position per market
    pub skew_factor: f64,           // How much to skew quotes based on inventory
}

impl BasicMarketMaker {
    fn calculate_quotes(&self, orderbook: &OrderBook, position: &Position) -> Quotes {
        let mid_price = orderbook.mid_price();
        let spread = mid_price * self.config.base_spread_bps / 10000;
        
        // Calculate inventory skew
        let position_ratio = position.size / self.config.max_position;
        let skew = position_ratio * self.config.skew_factor;
        
        // Adjust quotes based on inventory
        let bid_price = mid_price - (spread / 2) * (1.0 + skew);
        let ask_price = mid_price + (spread / 2) * (1.0 - skew);
        
        Quotes {
            bid_price,
            bid_size: self.config.quote_size,
            ask_price,
            ask_size: self.config.quote_size,
            confidence: 1.0,
        }
    }
}
```

**Configuration**:
```toml
[strategy.basic_mm]
enabled = true
base_spread_bps = 500           # 5% spread
quote_size = 10                 # $10 per side
max_position = 100              # $100 max position
skew_factor = 0.5               # 50% skew at max position
quote_refresh_interval_ms = 2000
cancel_on_disconnect = true
```

#### 3.2.2 Adaptive Spread Strategy

```rust
// src/strategy/adaptive_spread.rs

pub struct AdaptiveSpreadMM {
    config: AdaptiveConfig,
    volatility_tracker: VolatilityTracker,
    positions: HashMap<String, Position>,
}

pub struct AdaptiveConfig {
    pub min_spread_bps: u32,
    pub max_spread_bps: u32,
    pub volatility_lookback_seconds: u64,
    pub spread_adjustment_factor: f64,
}

impl AdaptiveSpreadMM {
    fn calculate_dynamic_spread(&self, market_id: &str) -> Decimal {
        let volatility = self.volatility_tracker.get_volatility(market_id);
        let base_spread = self.config.min_spread_bps;
        let vol_adjustment = volatility * self.config.spread_adjustment_factor;
        
        let spread = base_spread + vol_adjustment;
        spread.clamp(self.config.min_spread_bps, self.config.max_spread_bps)
    }
}
```

**Configuration**:
```toml
[strategy.adaptive_spread]
enabled = false
min_spread_bps = 300            # 3% minimum
max_spread_bps = 1000           # 10% maximum
volatility_lookback_seconds = 300
spread_adjustment_factor = 100
inventory_adjustment_enabled = true
```

#### 3.2.3 ML-Enhanced Strategy

```rust
// src/strategy/ml_enhanced.rs

pub struct MLEnhancedMM {
    base_strategy: Box<dyn Strategy>,
    ml_predictor: OllamaPredictor,
    config: MLConfig,
}

pub struct MLConfig {
    pub ollama_endpoint: String,
    pub model_name: String,
    pub inference_timeout_ms: u64,
    pub min_confidence_threshold: f64,
    pub fallback_to_base: bool,
}

impl MLEnhancedMM {
    async fn get_ml_prediction(&self, features: &MarketFeatures) -> Result<MLPrediction> {
        let prompt = self.build_prediction_prompt(features);
        let response = self.ml_predictor.predict(prompt).await?;
        
        MLPrediction::from_response(response)
    }
    
    fn build_prediction_prompt(&self, features: &MarketFeatures) -> String {
        format!(
            "Given market conditions:\n\
             - Current spread: {:.4}\n\
             - Volatility: {:.4}\n\
             - Order book imbalance: {:.4}\n\
             - Recent volume: {}\n\
             - My position: {}\n\
             Predict optimal bid/ask spreads and sizes. Return JSON.",
            features.spread,
            features.volatility,
            features.imbalance,
            features.volume,
            features.position_size
        )
    }
}
```

**Configuration**:
```toml
[strategy.ml_enhanced]
enabled = false                 # Start disabled!
ollama_endpoint = "http://localhost:11434"
model_name = "market-maker-v1"
inference_timeout_ms = 500
min_confidence_threshold = 0.7
fallback_to_base = true
base_strategy = "adaptive_spread"
```

### 3.3 Order Management System (OMS)

**Module**: `src/oms/`

**Responsibilities**:
- Queue and manage order lifecycle
- Place orders via Polymarket API
- Track order states (pending, open, filled, cancelled)
- Handle order amendments and cancellations
- Implement rate limiting to avoid API bans
- Reconcile exchange state with local state

**Key Interfaces**:

```rust
// src/oms/order_manager.rs

pub struct OrderManager {
    exchange_client: ExchangeClient,
    rate_limiter: RateLimiter,
    order_tracker: OrderTracker,
    config: OMSConfig,
}

pub struct OMSConfig {
    pub max_orders_per_second: u32,
    pub max_pending_orders: usize,
    pub order_timeout_seconds: u64,
    pub enable_order_amendment: bool,
}

impl OrderManager {
    pub async fn submit_order(&mut self, order: NewOrder) -> Result<OrderId> {
        // Rate limit check
        self.rate_limiter.acquire().await?;
        
        // Submit to exchange
        let order_id = self.exchange_client.place_order(order).await?;
        
        // Track locally
        self.order_tracker.add(order_id, order);
        
        Ok(order_id)
    }
    
    pub async fn cancel_order(&mut self, order_id: OrderId) -> Result<()> {
        self.exchange_client.cancel_order(order_id).await?;
        self.order_tracker.mark_cancelled(order_id);
        Ok(())
    }
    
    pub async fn cancel_all_orders(&mut self, market_id: Option<String>) -> Result<()> {
        // Emergency cancel
    }
    
    pub fn get_open_orders(&self, market_id: &str) -> Vec<Order> {
        // Return current open orders
    }
}

pub struct Order {
    pub id: String,
    pub market_id: String,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub filled_size: Decimal,
    pub status: OrderStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

pub enum OrderStatus {
    Pending,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

pub enum Side {
    Buy,
    Sell,
}
```

**Configuration**:
```toml
[oms]
max_orders_per_second = 5
max_pending_orders = 100
order_timeout_seconds = 300
enable_order_amendment = true
retry_on_failure = true
max_retry_attempts = 3
```

**Order Flow**:
1. Strategy generates order signal
2. Risk checks validate order
3. OMS queues order with rate limiting
4. Order submitted to exchange via API
5. Order ID returned and tracked locally
6. Fill updates received via WebSocket
7. Position and PnL updated

### 3.4 Risk Management System

**Module**: `src/risk/`

**Responsibilities**:
- Pre-trade risk validation
- Position size limits
- Maximum exposure limits
- Loss limits (daily/weekly)
- Circuit breakers for abnormal conditions
- Kill switch functionality

**Key Interfaces**:

```rust
// src/risk/risk_manager.rs

pub struct RiskManager {
    config: RiskConfig,
    position_tracker: PositionTracker,
    pnl_tracker: PnLTracker,
    circuit_breaker: CircuitBreaker,
}

pub struct RiskConfig {
    pub max_position_per_market: Decimal,
    pub max_total_exposure: Decimal,
    pub max_order_size: Decimal,
    pub max_daily_loss: Decimal,
    pub max_weekly_loss: Decimal,
    pub min_spread_bps: u32,
    pub circuit_breaker_loss_pct: f64,
}

impl RiskManager {
    pub fn validate_order(&self, order: &NewOrder, position: &Position) -> Result<()> {
        // Check position limits
        self.check_position_limit(order, position)?;
        
        // Check exposure limits
        self.check_exposure_limit(order)?;
        
        // Check loss limits
        self.check_loss_limits()?;
        
        // Check spread minimum
        self.check_spread(order)?;
        
        // Check circuit breaker
        self.circuit_breaker.check_status()?;
        
        Ok(())
    }
    
    fn check_position_limit(&self, order: &NewOrder, position: &Position) -> Result<()> {
        let new_position = position.size + order.size_delta();
        
        if new_position.abs() > self.config.max_position_per_market {
            return Err(RiskError::PositionLimitExceeded);
        }
        
        Ok(())
    }
    
    fn check_loss_limits(&self) -> Result<()> {
        let daily_loss = self.pnl_tracker.get_daily_pnl();
        if daily_loss < -self.config.max_daily_loss {
            return Err(RiskError::DailyLossLimitExceeded);
        }
        
        Ok(())
    }
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitBreakerState,
}

pub enum CircuitBreakerState {
    Normal,
    Triggered { reason: String, triggered_at: i64 },
}
```

**Configuration**:
```toml
[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
max_order_size = 20.0
max_daily_loss = 50.0
max_weekly_loss = 150.0
min_spread_bps = 200  # 2% minimum spread
circuit_breaker_loss_pct = 0.10  # 10% of capital

[risk.circuit_breaker]
enabled = true
triggers = [
    { type = "daily_loss", threshold = 50.0 },
    { type = "loss_rate", pct = 0.05, window_minutes = 15 },
    { type = "connection_loss", duration_seconds = 60 },
]
manual_reset_required = true
```

**Circuit Breaker Logic**:

```rust
impl CircuitBreaker {
    pub fn check_and_trigger(&mut self, event: &RiskEvent) -> bool {
        match event {
            RiskEvent::DailyLossThreshold(loss) => {
                if *loss > self.config.daily_loss_threshold {
                    self.trigger("Daily loss limit exceeded");
                    return true;
                }
            }
            RiskEvent::RapidLoss { pct, duration } => {
                if *pct > self.config.loss_rate_pct {
                    self.trigger("Rapid loss detected");
                    return true;
                }
            }
            RiskEvent::ConnectionLoss(duration) => {
                if *duration > self.config.connection_loss_threshold {
                    self.trigger("Prolonged connection loss");
                    return true;
                }
            }
        }
        false
    }
    
    fn trigger(&mut self, reason: &str) {
        self.state = CircuitBreakerState::Triggered {
            reason: reason.to_string(),
            triggered_at: Utc::now().timestamp(),
        };
        
        // Cancel all orders immediately
        // Send alerts
        // Stop trading
    }
}
```

### 3.5 Position Tracker

**Module**: `src/risk/position_tracker.rs`

**Responsibilities**:
- Track positions across all markets
- Calculate realized and unrealized PnL
- Maintain average entry prices
- Reconcile with exchange positions

**Key Interfaces**:

```rust
pub struct PositionTracker {
    positions: HashMap<String, Position>,
    pnl_history: Vec<PnLSnapshot>,
}

impl PositionTracker {
    pub fn update_on_fill(&mut self, fill: &Fill) {
        let position = self.positions.entry(fill.market_id.clone())
            .or_insert(Position::default());
        
        position.update_with_fill(fill);
    }
    
    pub fn calculate_unrealized_pnl(&self, market_prices: &HashMap<String, Decimal>) -> Decimal {
        self.positions.iter()
            .map(|(market_id, position)| {
                let current_price = market_prices.get(market_id).unwrap_or(&Decimal::ZERO);
                position.calculate_unrealized_pnl(*current_price)
            })
            .sum()
    }
    
    pub fn get_total_exposure(&self) -> Decimal {
        self.positions.values()
            .map(|p| p.size.abs() * p.mark_price)
            .sum()
    }
}
```

### 3.6 Database Layer

**Module**: `src/database/`

**Responsibilities**:
- Persist all market data
- Store order and fill history
- Record strategy decisions
- Log PnL snapshots
- Export data for ML training

**Schema Design**:

```sql
-- Markets tracking
CREATE TABLE markets (
    id VARCHAR(100) PRIMARY KEY,
    question TEXT NOT NULL,
    end_date TIMESTAMPTZ,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Orderbook snapshots (TimescaleDB hypertable)
CREATE TABLE orderbook_snapshots (
    id BIGSERIAL,
    market_id VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    bids JSONB NOT NULL,
    asks JSONB NOT NULL,
    mid_price DECIMAL(18, 8),
    spread DECIMAL(10, 6),
    PRIMARY KEY (id, timestamp)
);
SELECT create_hypertable('orderbook_snapshots', 'timestamp');

-- Trades (TimescaleDB hypertable)
CREATE TABLE trades (
    id BIGSERIAL,
    market_id VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    price DECIMAL(18, 8) NOT NULL,
    size DECIMAL(18, 8) NOT NULL,
    side VARCHAR(10) NOT NULL,
    PRIMARY KEY (id, timestamp)
);
SELECT create_hypertable('trades', 'timestamp');

-- Our orders
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    order_id VARCHAR(100) UNIQUE NOT NULL,
    market_id VARCHAR(100) NOT NULL,
    side VARCHAR(10) NOT NULL,
    order_type VARCHAR(20) NOT NULL,
    price DECIMAL(18, 8) NOT NULL,
    size DECIMAL(18, 8) NOT NULL,
    filled_size DECIMAL(18, 8) DEFAULT 0,
    status VARCHAR(20) NOT NULL,
    strategy_name VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    cancelled_at TIMESTAMPTZ,
    filled_at TIMESTAMPTZ
);
CREATE INDEX idx_orders_market_status ON orders(market_id, status);
CREATE INDEX idx_orders_created_at ON orders(created_at);

-- Fill events
CREATE TABLE fills (
    id BIGSERIAL PRIMARY KEY,
    order_id VARCHAR(100) NOT NULL,
    market_id VARCHAR(100) NOT NULL,
    side VARCHAR(10) NOT NULL,
    price DECIMAL(18, 8) NOT NULL,
    size DECIMAL(18, 8) NOT NULL,
    fee DECIMAL(18, 8),
    timestamp TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(order_id)
);
CREATE INDEX idx_fills_timestamp ON fills(timestamp);

-- Position snapshots (TimescaleDB hypertable)
CREATE TABLE position_snapshots (
    id BIGSERIAL,
    timestamp TIMESTAMPTZ NOT NULL,
    market_id VARCHAR(100) NOT NULL,
    size DECIMAL(18, 8) NOT NULL,
    avg_entry_price DECIMAL(18, 8),
    mark_price DECIMAL(18, 8),
    realized_pnl DECIMAL(18, 8),
    unrealized_pnl DECIMAL(18, 8),
    PRIMARY KEY (id, timestamp)
);
SELECT create_hypertable('position_snapshots', 'timestamp');

-- Strategy decisions (for ML training)
CREATE TABLE strategy_decisions (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    market_id VARCHAR(100) NOT NULL,
    strategy_name VARCHAR(50) NOT NULL,
    
    -- Input features
    features JSONB NOT NULL,  -- All market features at decision time
    
    -- Decision
    decision JSONB NOT NULL,  -- bid/ask prices, sizes, confidence
    
    -- Outcome (updated after fills)
    outcome JSONB,  -- fills received, pnl impact
    
    -- Model metadata
    model_version VARCHAR(50),
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    outcome_updated_at TIMESTAMPTZ
);
CREATE INDEX idx_strategy_decisions_timestamp ON strategy_decisions(timestamp);
CREATE INDEX idx_strategy_decisions_market ON strategy_decisions(market_id);

-- PnL summary (materialized view, refreshed periodically)
CREATE MATERIALIZED VIEW pnl_summary AS
SELECT
    DATE_TRUNC('day', timestamp) as date,
    market_id,
    SUM(realized_pnl) as daily_realized_pnl,
    AVG(unrealized_pnl) as avg_unrealized_pnl,
    MAX(mark_price) as high_price,
    MIN(mark_price) as low_price
FROM position_snapshots
GROUP BY date, market_id;

CREATE UNIQUE INDEX ON pnl_summary(date, market_id);

-- System events log
CREATE TABLE system_events (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    message TEXT,
    details JSONB
);
CREATE INDEX idx_system_events_timestamp ON system_events(timestamp);
CREATE INDEX idx_system_events_type ON system_events(event_type);
```

**Database Client**:

```rust
// src/database/client.rs

pub struct DatabaseClient {
    pool: PgPool,
}

impl DatabaseClient {
    pub async fn insert_orderbook_snapshot(&self, snapshot: &OrderBookSnapshot) -> Result<()> {
        sqlx::query!(
            "INSERT INTO orderbook_snapshots 
             (market_id, timestamp, bids, asks, mid_price, spread)
             VALUES ($1, $2, $3, $4, $5, $6)",
            snapshot.market_id,
            snapshot.timestamp,
            snapshot.bids_json,
            snapshot.asks_json,
            snapshot.mid_price,
            snapshot.spread
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn insert_order(&self, order: &Order) -> Result<()> {
        // Insert order
    }
    
    pub async fn update_order_status(&self, order_id: &str, status: OrderStatus) -> Result<()> {
        // Update order
    }
    
    pub async fn insert_strategy_decision(&self, decision: &StrategyDecision) -> Result<i64> {
        // Log strategy decision for ML
    }
    
    pub async fn get_daily_pnl(&self) -> Result<Decimal> {
        // Calculate today's PnL
    }
}
```

---

## 4. Data Models

### 4.1 Core Data Structures

```rust
// src/models.rs

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: String,
    pub question: String,
    pub end_date: Option<i64>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub market_id: String,
    pub timestamp: i64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub sequence: u64,
}

impl OrderBook {
    pub fn mid_price(&self) -> Decimal {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => (bid.price + ask.price) / Decimal::from(2),
            _ => Decimal::ZERO,
        }
    }
    
    pub fn spread(&self) -> Decimal {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => ask.price - bid.price,
            _ => Decimal::ZERO,
        }
    }
    
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }
    
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
    
    pub fn imbalance(&self) -> f64 {
        let bid_volume: Decimal = self.bids.iter().map(|l| l.size).sum();
        let ask_volume: Decimal = self.asks.iter().map(|l| l.size).sum();
        let total = bid_volume + ask_volume;
        
        if total == Decimal::ZERO {
            return 0.0;
        }
        
        ((bid_volume - ask_volume) / total).to_f64().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub market_id: String,
    pub timestamp: i64,
    pub price: Decimal,
    pub size: Decimal,
    pub side: Side,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub market_id: String,
    pub size: Decimal,
    pub avg_entry_price: Decimal,
    pub mark_price: Decimal,
    pub realized_pnl: Decimal,
}

impl Position {
    pub fn unrealized_pnl(&self) -> Decimal {
        (self.mark_price - self.avg_entry_price) * self.size
    }
    
    pub fn update_with_fill(&mut self, fill: &Fill) {
        let prev_size = self.size;
        let new_size = prev_size + fill.size_delta();
        
        if new_size.abs() > prev_size.abs() {
            // Increasing position - update avg entry
            let prev_value = prev_size * self.avg_entry_price;
            let new_value = fill.size_delta() * fill.price;
            self.avg_entry_price = (prev_value + new_value) / new_size;
        } else if new_size.signum() != prev_size.signum() {
            // Closing or flipping position - realize PnL
            let pnl = (fill.price - self.avg_entry_price) * fill.size_delta().abs();
            self.realized_pnl += pnl;
            
            if new_size != Decimal::ZERO {
                self.avg_entry_price = fill.price;
            }
        }
        
        self.size = new_size;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub market_id: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Decimal,
    pub size: Decimal,
    pub filled_size: Decimal,
    pub status: OrderStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn sign(&self) -> Decimal {
        match self {
            Side::Buy => Decimal::ONE,
            Side::Sell => -Decimal::ONE,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: String,
    pub market_id: String,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub fee: Decimal,
    pub timestamp: i64,
}

impl Fill {
    pub fn size_delta(&self) -> Decimal {
        self.size * self.side.sign()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketFeatures {
    pub market_id: String,
    pub timestamp: i64,
    
    // Price metrics
    pub mid_price: Decimal,
    pub spread: Decimal,
    pub spread_bps: f64,
    
    // Orderbook metrics
    pub imbalance: f64,
    pub bid_depth: Decimal,
    pub ask_depth: Decimal,
    
    // Volatility metrics
    pub volatility_1m: f64,
    pub volatility_5m: f64,
    pub volatility_15m: f64,
    
    // Volume metrics
    pub volume_1m: Decimal,
    pub volume_5m: Decimal,
    pub trade_count_1m: u32,
    
    // Position info
    pub current_position: Decimal,
    pub position_ratio: f64,  // position / max_position
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDecision {
    pub timestamp: i64,
    pub market_id: String,
    pub strategy_name: String,
    pub features: MarketFeatures,
    pub decision: DecisionOutput,
    pub model_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutput {
    pub bid_price: Decimal,
    pub bid_size: Decimal,
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub confidence: f64,
    pub reasoning: Option<String>,
}
```

---

## 5. API Specifications

### 5.1 Polymarket CLOB API Integration

**Base URLs**:
- WebSocket: `wss://clob.polymarket.com/ws`
- REST API: `https://clob.polymarket.com/api/v1`

**Authentication**:
```rust
// Using API key authentication
struct PolymarketAuth {
    api_key: String,
    api_secret: String,
}

impl PolymarketAuth {
    fn sign_request(&self, method: &str, path: &str, body: &str) -> String {
        // HMAC-SHA256 signature
        let message = format!("{}{}{}", method, path, body);
        hmac_sha256(&self.api_secret, message.as_bytes())
    }
}
```

**WebSocket Connection**:

```rust
// Connection flow
async fn connect_websocket() -> Result<WebSocketStream> {
    let url = "wss://clob.polymarket.com/ws";
    let (ws_stream, _) = connect_async(url).await?;
    
    // Authenticate
    let auth_msg = json!({
        "type": "auth",
        "apiKey": api_key,
        "signature": signature,
        "timestamp": timestamp
    });
    ws_stream.send(Message::Text(auth_msg.to_string())).await?;
    
    Ok(ws_stream)
}

// Subscribe to markets
async fn subscribe_to_markets(ws: &mut WebSocketStream, market_ids: &[String]) {
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": ["orderbook", "trades"],
        "markets": market_ids
    });
    ws.send(Message::Text(subscribe_msg.to_string())).await?;
}
```

**Message Types**:

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "orderbook")]
    OrderBook {
        market_id: String,
        bids: Vec<[String; 2]>,  // [price, size]
        asks: Vec<[String; 2]>,
        timestamp: i64,
        sequence: u64,
    },
    
    #[serde(rename = "trade")]
    Trade {
        market_id: String,
        price: String,
        size: String,
        side: String,
        timestamp: i64,
    },
    
    #[serde(rename = "order_update")]
    OrderUpdate {
        order_id: String,
        status: String,
        filled_size: String,
    },
}
```

**REST API Endpoints**:

```rust
// Place order
POST /orders
{
    "market_id": "string",
    "side": "buy" | "sell",
    "type": "limit" | "market",
    "price": "string",  // decimal as string
    "size": "string"
}

// Cancel order
DELETE /orders/{order_id}

// Get open orders
GET /orders?market_id={market_id}&status=open

// Get order book
GET /markets/{market_id}/orderbook

// Get trades
GET /markets/{market_id}/trades?limit=100
```

**Rate Limits**:
- REST API: 10 requests/second per API key
- WebSocket: No hard limit, but avoid excessive subscriptions
- Order placement: 5 orders/second recommended

### 5.2 Internal HTTP API (for monitoring/control)

```rust
// Optional: expose HTTP API for monitoring and control

use axum::{Router, Json};
use axum::routing::{get, post};

pub fn create_api_router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(get_status))
        .route("/positions", get(get_positions))
        .route("/pnl", get(get_pnl))
        .route("/orders", get(get_orders))
        .route("/circuit-breaker/reset", post(reset_circuit_breaker))
        .route("/stop", post(stop_trading))
}

async fn health_check() -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "healthy",
        uptime_seconds: get_uptime(),
    })
}

async fn get_status() -> Json<SystemStatus> {
    Json(SystemStatus {
        connected: is_connected(),
        trading: is_trading(),
        markets_active: get_active_markets(),
        circuit_breaker: get_circuit_breaker_state(),
    })
}
```

---

## 6. Configuration

### 6.1 Configuration File Structure

```toml
# config/production.toml

[general]
environment = "production"
log_level = "info"
data_dir = "/var/lib/mm-bot"

[exchange]
api_key = "${POLYMARKET_API_KEY}"
api_secret = "${POLYMARKET_API_SECRET}"
clob_websocket = "wss://clob.polymarket.com/ws"
clob_rest_api = "https://clob.polymarket.com/api/v1"
request_timeout_seconds = 10
max_reconnect_attempts = 10

[market_data]
websocket_url = "${exchange.clob_websocket}"
reconnect_delay_ms = 1000
heartbeat_interval_ms = 30000
orderbook_depth = 20
snapshot_interval_ms = 1000

[strategy]
active = "basic_mm"  # Which strategy to use
quote_refresh_interval_ms = 2000
cancel_stale_orders = true
stale_order_threshold_seconds = 30

[strategy.basic_mm]
base_spread_bps = 500
quote_size = 10.0
max_position = 100.0
skew_factor = 0.5

[strategy.adaptive_spread]
min_spread_bps = 300
max_spread_bps = 1000
volatility_lookback_seconds = 300
spread_adjustment_factor = 100.0

[strategy.ml_enhanced]
enabled = false
base_strategy = "adaptive_spread"
ollama_endpoint = "http://localhost:11434"
model_name = "market-maker-v1"
inference_timeout_ms = 500
min_confidence_threshold = 0.7
fallback_to_base = true

[oms]
max_orders_per_second = 5
max_pending_orders = 100
order_timeout_seconds = 300
enable_order_amendment = true
retry_on_failure = true
max_retry_attempts = 3

[risk]
# CRITICAL: Start with very conservative limits!
max_position_per_market = 100.0
max_total_exposure = 500.0
max_order_size = 20.0
max_daily_loss = 50.0
max_weekly_loss = 150.0
min_spread_bps = 200
paper_trading_mode = true  # KEEP TRUE initially

[risk.circuit_breaker]
enabled = true
daily_loss_threshold = 50.0
loss_rate_pct = 0.05
loss_rate_window_minutes = 15
connection_loss_threshold_seconds = 60
manual_reset_required = true

[markets]
# Start with empty, manually add markets after testing
enabled = []
blacklist = []
auto_discover = false

[database]
postgres_url = "${DATABASE_URL}"
max_connections = 10
connection_timeout_seconds = 30
enable_ssl = true

[database.retention]
orderbook_snapshots_days = 30
trades_days = 90
orders_days = 365
strategy_decisions_days = 90

[monitoring]
prometheus_port = 9090
enable_metrics = true
enable_tracing = true

[monitoring.alerts]
pagerduty_key = "${PAGERDUTY_KEY}"
slack_webhook = "${SLACK_WEBHOOK}"
alert_on_circuit_breaker = true
alert_on_connection_loss = true
alert_on_daily_loss_threshold = true

[ml]
ollama_endpoint = "http://localhost:11434"
training_enabled = false
training_schedule = "0 0 * * 0"  # Weekly on Sunday
min_training_samples = 10000
model_backup_dir = "/var/lib/mm-bot/models"

[logging]
console_output = true
file_output = true
log_file = "/var/log/mm-bot/app.log"
max_file_size_mb = 100
max_backup_files = 10
json_format = true
```

### 6.2 Configuration Loading

```rust
// src/config.rs

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub exchange: ExchangeConfig,
    pub market_data: MarketDataConfig,
    pub strategy: StrategyConfig,
    pub oms: OMSConfig,
    pub risk: RiskConfig,
    pub markets: MarketsConfig,
    pub database: DatabaseConfig,
    pub monitoring: MonitoringConfig,
    pub ml: MLConfig,
    pub logging: LoggingConfig,
}

impl AppConfig {
    pub fn load(environment: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default config
            .add_source(File::with_name("config/default"))
            // Add environment-specific config
            .add_source(File::with_name(&format!("config/{}", environment)).required(false))
            // Add environment variables with prefix "MM_BOT"
            .add_source(Environment::with_prefix("MM_BOT").separator("__"))
            .build()?;
        
        config.try_deserialize()
    }
    
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate configuration consistency
        if self.risk.max_order_size > self.risk.max_position_per_market {
            return Err(ValidationError::new("max_order_size cannot exceed max_position"));
        }
        
        if self.risk.max_position_per_market * self.markets.enabled.len() as f64
            > self.risk.max_total_exposure {
            return Err(ValidationError::new("Sum of max positions exceeds total exposure"));
        }
        
        // Warn if paper trading is disabled
        if !self.risk.paper_trading_mode {
            log::warn!("⚠️  PAPER TRADING MODE IS DISABLED - REAL MONEY AT RISK ⚠️");
        }
        
        Ok(())
    }
}

// Environment variable examples:
// MM_BOT__EXCHANGE__API_KEY=xxx
// MM_BOT__RISK__MAX_DAILY_LOSS=100
// MM_BOT__STRATEGY__ACTIVE=adaptive_spread
```

### 6.3 Hot Reloading Configuration

```rust
// Support hot-reloading for non-critical configs

pub struct ConfigWatcher {
    config_path: PathBuf,
    last_modified: SystemTime,
    reload_tx: Sender<AppConfig>,
}

impl ConfigWatcher {
    pub async fn watch(&mut self) {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            
            if let Ok(metadata) = fs::metadata(&self.config_path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > self.last_modified {
                        if let Ok(new_config) = AppConfig::load("production") {
                            log::info!("Configuration reloaded");
                            self.reload_tx.send(new_config).await.ok();
                            self.last_modified = modified;
                        }
                    }
                }
            }
        }
    }
}

// Only reload safe parameters:
// - Strategy parameters (spreads, sizes)
// - Monitoring settings
// - Logging settings
// 
// DO NOT reload:
// - Risk limits (require restart)
// - API credentials
// - Database connections
```

---

## 7. Deployment

### 7.1 Docker Setup

**Dockerfile**:

```dockerfile
# Dockerfile

FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mm-bot /usr/local/bin/mm-bot

# Create data directories
RUN mkdir -p /var/lib/mm-bot /var/log/mm-bot /etc/mm-bot

# Non-root user
RUN useradd -m -u 1000 mmbot && \
    chown -R mmbot:mmbot /var/lib/mm-bot /var/log/mm-bot

USER mmbot

ENTRYPOINT ["/usr/local/bin/mm-bot"]
CMD ["--config", "/etc/mm-bot/production.toml"]
```

**docker-compose.yml**:

```yaml
version: '3.8'

services:
  postgres:
    image: timescale/timescaledb:latest-pg15
    container_name: mm-bot-postgres
    environment:
      POSTGRES_DB: mm_bot
      POSTGRES_USER: mmuser
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      PGDATA: /var/lib/postgresql/data/pgdata
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./sql/schema.sql:/docker-entrypoint-initdb.d/01-schema.sql
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U mmuser"]
      interval: 10s
      timeout: 5s
      retries: 5

  mm-bot:
    build: .
    container_name: mm-bot
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      MM_BOT__DATABASE__POSTGRES_URL: "postgresql://mmuser:${DB_PASSWORD}@postgres:5432/mm_bot"
      MM_BOT__EXCHANGE__API_KEY: ${POLYMARKET_API_KEY}
      MM_BOT__EXCHANGE__API_SECRET: ${POLYMARKET_API_SECRET}
      RUST_LOG: info
    volumes:
      - ./config:/etc/mm-bot:ro
      - mm_bot_data:/var/lib/mm-bot
      - mm_bot_logs:/var/log/mm-bot
    restart: unless-stopped
    networks:
      - mm-bot-network
    ports:
      - "9090:9090"  # Prometheus metrics

  prometheus:
    image: prom/prometheus:latest
    container_name: mm-bot-prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    ports:
      - "9091:9090"
    networks:
      - mm-bot-network
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    container_name: mm-bot-grafana
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD}
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro
      - ./monitoring/grafana/datasources:/etc/grafana/provisioning/datasources:ro
    ports:
      - "3000:3000"
    networks:
      - mm-bot-network
    restart: unless-stopped

volumes:
  postgres_data:
  mm_bot_data:
  mm_bot_logs:
  prometheus_data:
  grafana_data:

networks:
  mm-bot-network:
    driver: bridge
```

**.env file**:

```bash
# .env (DO NOT COMMIT TO GIT)

# Database
DB_PASSWORD=changeme_secure_password

# Polymarket API
POLYMARKET_API_KEY=your_api_key_here
POLYMARKET_API_SECRET=your_api_secret_here

# Monitoring
GRAFANA_PASSWORD=changeme_secure_password
PAGERDUTY_KEY=your_pagerduty_key
SLACK_WEBHOOK=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
```

### 7.2 Cloud Deployment (AWS Example)

**Infrastructure as Code (Terraform)**:

```hcl
# infrastructure/main.tf

provider "aws" {
  region = "us-east-1"
}

# VPC and networking
resource "aws_vpc" "mm_bot" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name = "mm-bot-vpc"
  }
}

# EC2 instance for bot
resource "aws_instance" "mm_bot" {
  ami           = "ami-0c55b159cbfafe1f0"  # Ubuntu 22.04
  instance_type = "t3.medium"  # 2 vCPU, 4GB RAM

  vpc_security_group_ids = [aws_security_group.mm_bot.id]
  subnet_id              = aws_subnet.private.id

  user_data = file("user_data.sh")

  tags = {
    Name = "mm-bot-instance"
  }
}

# RDS for PostgreSQL
resource "aws_db_instance" "mm_bot" {
  identifier           = "mm-bot-db"
  engine               = "postgres"
  engine_version       = "15.3"
  instance_class       = "db.t3.micro"
  allocated_storage    = 20
  storage_encrypted    = true

  db_name  = "mm_bot"
  username = "mmuser"
  password = var.db_password

  vpc_security_group_ids = [aws_security_group.rds.id]
  db_subnet_group_name   = aws_db_subnet_group.mm_bot.name

  backup_retention_period = 7
  skip_final_snapshot     = false
  final_snapshot_identifier = "mm-bot-final-snapshot"

  tags = {
    Name = "mm-bot-database"
  }
}

# CloudWatch alarms
resource "aws_cloudwatch_metric_alarm" "high_cpu" {
  alarm_name          = "mm-bot-high-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CPUUtilization"
  namespace           = "AWS/EC2"
  period              = "120"
  statistic           = "Average"
  threshold           = "80"
  alarm_description   = "This metric monitors ec2 cpu utilization"
  alarm_actions       = [aws_sns_topic.alerts.arn]

  dimensions = {
    InstanceId = aws_instance.mm_bot.id
  }
}
```

**User data script**:

```bash
#!/bin/bash
# user_data.sh

# Update system
apt-get update && apt-get upgrade -y

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Install Docker Compose
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Create application directory
mkdir -p /opt/mm-bot
cd /opt/mm-bot

# Pull application code (from S3 or git)
aws s3 cp s3://your-bucket/mm-bot-deploy.tar.gz .
tar -xzf mm-bot-deploy.tar.gz

# Set environment variables from Secrets Manager
export POLYMARKET_API_KEY=$(aws secretsmanager get-secret-value --secret-id mm-bot/api-key --query SecretString --output text)
export POLYMARKET_API_SECRET=$(aws secretsmanager get-secret-value --secret-id mm-bot/api-secret --query SecretString --output text)
export DB_PASSWORD=$(aws secretsmanager get-secret-value --secret-id mm-bot/db-password --query SecretString --output text)

# Start services
docker-compose up -d

# Set up log rotation
cat > /etc/logrotate.d/mm-bot << EOF
/var/log/mm-bot/*.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    create 0640 mmbot mmbot
}
EOF
```

### 7.3 Deployment Checklist

**Pre-deployment**:
- [ ] API credentials obtained and tested
- [ ] Database schema initialized
- [ ] Configuration files created and validated
- [ ] Paper trading tested for minimum 30 days
- [ ] All risk limits configured conservatively
- [ ] Monitoring and alerting set up
- [ ] Backup and recovery procedures documented

**Deployment**:
- [ ] Build Docker image
- [ ] Push to container registry
- [ ] Deploy database
- [ ] Run database migrations
- [ ] Deploy application with paper trading enabled
- [ ] Verify connection to Polymarket
- [ ] Monitor for 24 hours in paper trading
- [ ] Review logs for errors

**Post-deployment**:
- [ ] Verify all metrics are reporting
- [ ] Test alert notifications
- [ ] Verify database is recording data
- [ ] Monitor performance for 1 week
- [ ] Document any issues
- [ ] Create runbook for common issues

---

## 8. Implementation Phases

### Phase 1: Foundation (Weeks 1-2)

**Goal**: Establish core infrastructure and paper trading capability

**Tasks**:
1. Set up development environment
   - Install Rust, Docker, PostgreSQL
   - Create project structure
   - Initialize git repository

2. Implement market data handler
   - WebSocket client for Polymarket
   - Message parsing
   - Orderbook management
   - Data validation

3. Create database schema
   - Design tables
   - Set up TimescaleDB
   - Create migration scripts

4. Build basic logging
   - Structured logging with tracing
   - Log to console and file
   - Error handling

5. Implement paper trading mode
   - Simulate order placement
   - Track virtual positions
   - Calculate virtual PnL

**Deliverables**:
- [ ] Working WebSocket connection to Polymarket
- [ ] Accurate orderbook maintenance
- [ ] Data persisted to database
- [ ] Paper trading mode functional
- [ ] Basic CLI for monitoring

**Success Criteria**:
- Zero data loss over 24-hour period
- Orderbook stays in sync with exchange
- All market events logged to database

### Phase 2: Trading Execution (Weeks 3-4)

**Goal**: Enable real order placement with conservative risk management

**Tasks**:
1. Implement OMS
   - Order placement via REST API
   - Order lifecycle management
   - Rate limiting
   - Fill tracking

2. Build risk management
   - Pre-trade checks
   - Position limits
   - Loss limits
   - Circuit breaker

3. Implement basic market making strategy
   - Fixed spread calculation
   - Quote generation
   - Inventory management

4. Create monitoring dashboard
   - Real-time metrics
   - Position display
   - PnL tracking

5. Set up alerting
   - Circuit breaker alerts
   - Connection loss alerts
   - Daily PnL alerts

**Deliverables**:
- [ ] Bot can place real orders
- [ ] Risk checks enforced before every trade
- [ ] Emergency stop mechanism working
- [ ] Monitoring dashboard operational
- [ ] Tested with $50-100 on ONE market

**Success Criteria**:
- All orders within risk limits
- Circuit breaker triggers correctly
- No unauthorized orders placed
- Alerts fire appropriately

### Phase 3: Strategy Enhancement (Weeks 5-8)

**Goal**: Improve strategy performance and scale to multiple markets

**Tasks**:
1. Implement adaptive spread strategy
   - Volatility calculation
   - Dynamic spread adjustment
   - Improved inventory management

2. Build backtesting framework
   - Historical data replay
   - Strategy simulation
   - Performance metrics

3. Optimize order placement
   - Reduce latency
   - Smart order routing
   - Order amendment logic

4. Scale to multiple markets
   - Parallel market processing
   - Per-market risk limits
   - Market selection criteria

5. Improve monitoring
   - Per-market metrics
   - Strategy performance tracking
   - Comparative analysis

**Deliverables**:
- [ ] Adaptive spread strategy implemented
- [ ] Backtesting shows positive results
- [ ] Trading 3-5 markets successfully
- [ ] Latency <100ms for order placement
- [ ] Comprehensive monitoring

**Success Criteria**:
- Positive PnL over 2-week period
- Fill rate >30%
- Uptime >99%
- No circuit breaker triggers from bugs

### Phase 4: ML Integration (Weeks 9-12)

**Goal**: Integrate machine learning for intelligent decision making

**Tasks**:
1. Design feature engineering pipeline
   - Extract market features
   - Calculate technical indicators
   - Prepare training data

2. Build training pipeline
   - Data export to Parquet
   - Ollama model training
   - Model evaluation
   - Backtesting with ML

3. Implement ML-enhanced strategy
   - Ollama inference integration
   - Confidence-based decision making
   - Fallback to rule-based

4. Set up A/B testing
   - Split markets between strategies
   - Track comparative performance
   - Statistical significance testing

5. Automate model retraining
   - Scheduled retraining
   - Model versioning
   - Automated deployment

**Deliverables**:
- [ ] 30+ days of training data collected
- [ ] ML model trained on historical data
- [ ] ML-enhanced strategy deployed
- [ ] A/B testing shows improvement
- [ ] Automated retraining pipeline

**Success Criteria**:
- ML strategy outperforms baseline
- Inference latency <500ms
- Model improves with more data
- Automated retraining works correctly

---

## 9. Testing Strategy

### 9.1 Unit Testing

```rust
// tests/unit/orderbook_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_mid_price() {
        let mut orderbook = OrderBook::new("TEST-MARKET");
        orderbook.add_bid(PriceLevel { price: dec!(0.50), size: dec!(100) });
        orderbook.add_ask(PriceLevel { price: dec!(0.52), size: dec!(100) });
        
        assert_eq!(orderbook.mid_price(), dec!(0.51));
    }

    #[test]
    fn test_orderbook_spread() {
        let mut orderbook = OrderBook::new("TEST-MARKET");
        orderbook.add_bid(PriceLevel { price: dec!(0.50), size: dec!(100) });
        orderbook.add_ask(PriceLevel { price: dec!(0.52), size: dec!(100) });
        
        assert_eq!(orderbook.spread(), dec!(0.02));
    }

    #[test]
    fn test_position_update() {
        let mut position = Position::default();
        
        // Open long position
        let fill = Fill {
            price: dec!(0.50),
            size: dec!(100),
            side: Side::Buy,
            ..Default::default()
        };
        position.update_with_fill(&fill);
        
        assert_eq!(position.size, dec!(100));
        assert_eq!(position.avg_entry_price, dec!(0.50));
        
        // Partially close
        let close_fill = Fill {
            price: dec!(0.52),
            size: dec!(50),
            side: Side::Sell,
            ..Default::default()
        };
        position.update_with_fill(&close_fill);
        
        assert_eq!(position.size, dec!(50));
        assert_eq!(position.realized_pnl, dec!(1.00));  // 50 * (0.52 - 0.50)
    }
}
```

### 9.2 Integration Testing

```rust
// tests/integration/trading_flow_test.rs

#[tokio::test]
async fn test_full_trading_flow() {
    // Set up test environment
    let config = load_test_config();
    let db = setup_test_database().await;
    
    // Create components
    let mut market_data = MockMarketDataHandler::new();
    let mut strategy = BasicMarketMaker::new(config.strategy);
    let mut oms = OrderManager::new(config.oms);
    let mut risk_manager = RiskManager::new(config.risk);
    
    // Simulate market data
    let orderbook = create_test_orderbook();
    market_data.send_orderbook_update(orderbook.clone()).await;
    
    // Strategy generates quotes
    let quotes = strategy.calculate_quotes(&orderbook, &Position::default());
    
    // Create orders
    let bid_order = NewOrder {
        market_id: "TEST".to_string(),
        side: Side::Buy,
        price: quotes.bid_price,
        size: quotes.bid_size,
    };
    
    // Risk check
    assert!(risk_manager.validate_order(&bid_order, &Position::default()).is_ok());
    
    // Submit order
    let order_id = oms.submit_order(bid_order).await.unwrap();
    assert!(!order_id.is_empty());
    
    // Verify order in database
    let saved_order = db.get_order(&order_id).await.unwrap();
    assert_eq!(saved_order.status, OrderStatus::Open);
    
    // Cleanup
    teardown_test_database(db).await;
}
```

### 9.3 Backtesting

```python
# scripts/backtest.py

import pandas as pd
from dataclasses import dataclass
from typing import List

@dataclass
class BacktestConfig:
    initial_capital: float
    max_position_per_market: float
    base_spread_bps: int
    quote_size: float

class Backtester:
    def __init__(self, config: BacktestConfig):
        self.config = config
        self.capital = config.initial_capital
        self.positions = {}
        self.trades = []
        self.pnl_history = []
    
    def run(self, market_data: pd.DataFrame) -> BacktestResults:
        """Run backtest on historical data"""
        
        for timestamp, row in market_data.iterrows():
            # Calculate quotes
            mid_price = (row['best_bid'] + row['best_ask']) / 2
            spread = mid_price * self.config.base_spread_bps / 10000
            
            bid_price = mid_price - spread / 2
            ask_price = mid_price + spread / 2
            
            # Simulate fills
            if row['trade_price'] <= bid_price:
                # We got filled on our bid
                self.execute_fill('buy', bid_price, self.config.quote_size, timestamp)
            
            if row['trade_price'] >= ask_price:
                # We got filled on our ask
                self.execute_fill('sell', ask_price, self.config.quote_size, timestamp)
            
            # Update PnL
            self.calculate_pnl(row['mid_price'], timestamp)
        
        return self.generate_results()
    
    def execute_fill(self, side: str, price: float, size: float, timestamp):
        """Simulate a fill"""
        self.trades.append({
            'timestamp': timestamp,
            'side': side,
            'price': price,
            'size': size
        })
        
        # Update position
        delta = size if side == 'buy' else -size
        market_id = 'TEST'  # Simplified
        
        if market_id not in self.positions:
            self.positions[market_id] = {'size': 0, 'avg_price': 0}
        
        pos = self.positions[market_id]
        new_size = pos['size'] + delta
        
        if abs(new_size) > abs(pos['size']):
            # Increasing position
            pos['avg_price'] = (pos['size'] * pos['avg_price'] + delta * price) / new_size
        
        pos['size'] = new_size
    
    def calculate_pnl(self, mark_price: float, timestamp):
        """Calculate current PnL"""
        total_pnl = 0
        
        for market_id, pos in self.positions.items():
            unrealized = (mark_price - pos['avg_price']) * pos['size']
            total_pnl += unrealized
        
        self.pnl_history.append({
            'timestamp': timestamp,
            'pnl': total_pnl
        })
    
    def generate_results(self) -> dict:
        """Generate backtest metrics"""
        pnl_series = pd.Series([p['pnl'] for p in self.pnl_history])
        
        return {
            'total_return': pnl_series.iloc[-1] / self.config.initial_capital,
            'sharpe_ratio': pnl_series.mean() / pnl_series.std() * (252 ** 0.5),
            'max_drawdown': (pnl_series / pnl_series.cummax() - 1).min(),
            'num_trades': len(self.trades),
            'win_rate': self.calculate_win_rate(),
        }
    
    def calculate_win_rate(self) -> float:
        """Calculate percentage of profitable trades"""
        if len(self.trades) < 2:
            return 0.0
        
        wins = 0
        for i in range(1, len(self.trades)):
            if self.trades[i]['price'] > self.trades[i-1]['price']:
                wins += 1
        
        return wins / (len(self.trades) - 1)

# Usage
if __name__ == '__main__':
    # Load historical data
    data = pd.read_parquet('data/market_history.parquet')
    
    # Configure backtest
    config = BacktestConfig(
        initial_capital=1000.0,
        max_position_per_market=100.0,
        base_spread_bps=500,
        quote_size=10.0
    )
    
    # Run backtest
    backtester = Backtester(config)
    results = backtester.run(data)
    
    print("Backtest Results:")
    print(f"Total Return: {results['total_return']:.2%}")
    print(f"Sharpe Ratio: {results['sharpe_ratio']:.2f}")
    print(f"Max Drawdown: {results['max_drawdown']:.2%}")
    print(f"Number of Trades: {results['num_trades']}")
    print(f"Win Rate: {results['win_rate']:.2%}")
```

### 9.4 Load Testing

```rust
// tests/load/latency_test.rs

#[tokio::test]
async fn test_order_placement_latency() {
    let mut oms = OrderManager::new(config);
    let mut latencies = Vec::new();
    
    for _ in 0..1000 {
        let start = Instant::now();
        
        let order = NewOrder {
            market_id: "TEST".to_string(),
            side: Side::Buy,
            price: dec!(0.50),
            size: dec!(10),
        };
        
        oms.submit_order(order).await.unwrap();
        
        let elapsed = start.elapsed();
        latencies.push(elapsed.as_millis());
    }
    
    // Calculate percentiles
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];
    
    println!("Latency P50: {}ms", p50);
    println!("Latency P95: {}ms", p95);
    println!("Latency P99: {}ms", p99);
    
    // Assert latency targets
    assert!(p95 < 100, "P95 latency exceeds 100ms");
    assert!(p99 < 200, "P99 latency exceeds 200ms");
}
```

---

## 10. Monitoring & Observability

### 10.1 Prometheus Metrics

```rust
// src/monitoring/metrics.rs

use prometheus::{
    Counter, Gauge, Histogram, IntGauge,
    register_counter, register_gauge, register_histogram, register_int_gauge,
};

lazy_static! {
    // Connection metrics
    pub static ref WEBSOCKET_CONNECTED: IntGauge = 
        register_int_gauge!("websocket_connected", "WebSocket connection status").unwrap();
    
    // Market data metrics
    pub static ref MARKET_DATA_MESSAGES: Counter = 
        register_counter!("market_data_messages_total", "Total market data messages received").unwrap();
    
    pub static ref ORDERBOOK_UPDATE_LATENCY: Histogram = 
        register_histogram!("orderbook_update_latency_ms", "Orderbook update latency").unwrap();
    
    // Trading metrics
    pub static ref ORDERS_PLACED: Counter = 
        register_counter!("orders_placed_total", "Total orders placed").unwrap();
    
    pub static ref ORDERS_FILLED: Counter = 
        register_counter!("orders_filled_total", "Total orders filled").unwrap();
    
    pub static ref ORDERS_CANCELLED: Counter = 
        register_counter!("orders_cancelled_total", "Total orders cancelled").unwrap();
    
    pub static ref ORDER_PLACEMENT_LATENCY: Histogram = 
        register_histogram!("order_placement_latency_ms", "Order placement latency").unwrap();
    
    // Position metrics
    pub static ref CURRENT_POSITION: Gauge = 
        register_gauge!("current_position", "Current position size").unwrap();
    
    pub static ref TOTAL_EXPOSURE: Gauge = 
        register_gauge!("total_exposure_usd", "Total exposure in USD").unwrap();
    
    // PnL metrics
    pub static ref REALIZED_PNL: Gauge = 
        register_gauge!("realized_pnl_usd", "Realized PnL in USD").unwrap();
    
    pub static ref UNREALIZED_PNL: Gauge = 
        register_gauge!("unrealized_pnl_usd", "Unrealized PnL in USD").unwrap();
    
    pub static ref DAILY_PNL: Gauge = 
        register_gauge!("daily_pnl_usd", "Daily PnL in USD").unwrap();
    
    // Risk metrics
    pub static ref CIRCUIT_BREAKER_STATUS: IntGauge = 
        register_int_gauge!("circuit_breaker_triggered", "Circuit breaker status").unwrap();
    
    pub static ref RISK_VIOLATIONS: Counter = 
        register_counter!("risk_violations_total", "Total risk limit violations").unwrap();
    
    // Strategy metrics
    pub static ref SPREAD_CAPTURE_RATE: Gauge = 
        register_gauge!("spread_capture_rate", "Percentage of spread captured").unwrap();
    
    pub static ref FILL_RATE: Gauge = 
        register_gauge!("fill_rate", "Order fill rate").unwrap();
}
```

**Prometheus Configuration**:

```yaml
# monitoring/prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'mm-bot'
    static_configs:
      - targets: ['mm-bot:9090']
    
  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres-exporter:9187']

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']

rule_files:
  - 'alerts.yml'
```

**Alert Rules**:

```yaml
# monitoring/alerts.yml

groups:
  - name: mm_bot_alerts
    interval: 30s
    rules:
      - alert: CircuitBreakerTriggered
        expr: circuit_breaker_triggered == 1
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Circuit breaker has been triggered"
          description: "Trading has been stopped due to risk conditions"
      
      - alert: HighDailyLoss
        expr: daily_pnl_usd < -40
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Daily loss approaching limit"
          description: "Daily PnL is {{ $value }} USD"
      
      - alert: WebSocketDisconnected
        expr: websocket_connected == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "WebSocket connection lost"
          description: "Connection to exchange has been lost"
      
      - alert: LowFillRate
        expr: fill_rate < 0.2
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "Fill rate is low"
          description: "Fill rate is {{ $value | humanizePercentage }}"
      
      - alert: HighOrderPlacementLatency
        expr: histogram_quantile(0.95, order_placement_latency_ms) > 200
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High order placement latency"
          description: "P95 latency is {{ $value }}ms"
```

### 10.2 Grafana Dashboard

```json
{
  "dashboard": {
    "title": "MM Bot Performance",
    "panels": [
      {
        "title": "Real-time PnL",
        "targets": [
          {
            "expr": "realized_pnl_usd + unrealized_pnl_usd"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Position Sizes",
        "targets": [
          {
            "expr": "current_position"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Order Placement Latency",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, order_placement_latency_ms)",
            "legendFormat": "p50"
          },
          {
            "expr": "histogram_quantile(0.95, order_placement_latency_ms)",
            "legendFormat": "p95"
          },
          {
            "expr": "histogram_quantile(0.99, order_placement_latency_ms)",
            "legendFormat": "p99"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Fill Rate",
        "targets": [
          {
            "expr": "rate(orders_filled_total[5m]) / rate(orders_placed_total[5m])"
          }
        ],
        "type": "gauge"
      },
      {
        "title": "Daily PnL",
        "targets": [
          {
            "expr": "daily_pnl_usd"
          }
        ],
        "type": "stat",
        "thresholds": [
          {"value": -40, "color": "red"},
          {"value": 0, "color": "yellow"},
          {"value": 10, "color": "green"}
        ]
      }
    ]
  }
}
```

### 10.3 Logging

```rust
// src/logging.rs

use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging(config: &LoggingConfig) {
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);
    
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, "mm-bot.log");
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .json();
    
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(tracing_subscriber::filter::LevelFilter::from_level(config.level))
        .init();
    
    info!("Logging initialized");
}

// Usage throughout the code:
info!(market_id = %market_id, "Connected to market");
debug!(order_id = %order_id, price = %price, "Order placed");
warn!(reason = %reason, "Risk check failed");
error!(error = %err, "Failed to connect to exchange");
```

---

## 11. Security & Risk Management

### 11.1 API Key Security

**Best Practices**:
- Never commit API keys to git
- Use environment variables or secrets management
- Rotate keys regularly
- Use read-only keys where possible
- Implement key encryption at rest

```rust
// src/security/credentials.rs

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

pub struct SecureCredentials {
    encrypted_api_key: Vec<u8>,
    encrypted_api_secret: Vec<u8>,
    cipher: Aes256Gcm,
}

impl SecureCredentials {
    pub fn new(encryption_key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(encryption_key.into());
        
        Self {
            encrypted_api_key: Vec::new(),
            encrypted_api_secret: Vec::new(),
            cipher,
        }
    }
    
    pub fn set_api_key(&mut self, api_key: &str) -> Result<()> {
        let nonce = Nonce::from_slice(b"unique nonce");
        self.encrypted_api_key = self.cipher
            .encrypt(nonce, api_key.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        Ok(())
    }
    
    pub fn get_api_key(&self) -> Result<String> {
        let nonce = Nonce::from_slice(b"unique nonce");
        let decrypted = self.cipher
            .decrypt(nonce, self.encrypted_api_key.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        Ok(String::from_utf8(decrypted)?)
    }
}
```

### 11.2 Input Validation

```rust
// src/validation.rs

pub fn validate_order(order: &NewOrder) -> Result<()> {
    // Validate market ID
    if order.market_id.is_empty() {
        return Err(ValidationError::EmptyMarketId);
    }
    
    // Validate price
    if order.price <= Decimal::ZERO || order.price > Decimal::from(1) {
        return Err(ValidationError::InvalidPrice);
    }
    
    // Validate size
    if order.size <= Decimal::ZERO {
        return Err(ValidationError::InvalidSize);
    }
    
    // Validate precision
    if order.price.scale() > 6 {
        return Err(ValidationError::ExcessivePrecision);
    }
    
    Ok(())
}
```

### 11.3 Kill Switch

```rust
// src/safety/kill_switch.rs

pub struct KillSwitch {
    active: Arc<AtomicBool>,
    triggered: Arc<AtomicBool>,
}

impl KillSwitch {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            triggered: Arc::new(AtomicBool::new(false)),
        }
    }
    
    pub fn trigger(&self) {
        warn!("🚨 KILL SWITCH ACTIVATED 🚨");
        self.triggered.store(true, Ordering::SeqCst);
        self.active.store(false, Ordering::SeqCst);
        
        // Cancel all orders
        // Close all positions (if configured)
        // Send alerts
    }
    
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
    
    pub fn reset(&self, confirmation: &str) -> Result<()> {
        if confirmation != "CONFIRM_RESET" {
            return Err(anyhow!("Invalid confirmation"));
        }
        
        info!("Kill switch reset");
        self.triggered.store(false, Ordering::SeqCst);
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }
}
```

---

## 12. ML Integration

### 12.1 Feature Engineering

```python
# scripts/features.py

import pandas as pd
import numpy as np

class FeatureEngineer:
    def __init__(self):
        self.lookback_windows = [60, 300, 900]  # 1min, 5min, 15min in seconds
    
    def extract_features(self, orderbook_df: pd.DataFrame, trades_df: pd.DataFrame) -> pd.DataFrame:
        """Extract features from raw market data"""
        
        features = pd.DataFrame(index=orderbook_df.index)
        
        # Price features
        features['mid_price'] = (orderbook_df['best_bid'] + orderbook_df['best_ask']) / 2
        features['spread'] = orderbook_df['best_ask'] - orderbook_df['best_bid']
        features['spread_bps'] = (features['spread'] / features['mid_price']) * 10000
        
        # Orderbook features
        features['imbalance'] = self.calculate_imbalance(orderbook_df)
        features['bid_depth'] = orderbook_df['bid_volume_total']
        features['ask_depth'] = orderbook_df['ask_volume_total']
        features['depth_imbalance'] = (features['bid_depth'] - features['ask_depth']) / (features['bid_depth'] + features['ask_depth'])
        
        # Volatility features
        for window in self.lookback_windows:
            features[f'volatility_{window}s'] = self.calculate_volatility(features['mid_price'], window)
        
        # Volume features
        for window in self.lookback_windows:
            features[f'volume_{window}s'] = self.calculate_rolling_volume(trades_df, window)
            features[f'trade_count_{window}s'] = self.calculate_trade_count(trades_df, window)
        
        # Trade flow features
        for window in self.lookback_windows:
            features[f'buy_volume_{window}s'] = self.calculate_directional_volume(trades_df, 'buy', window)
            features[f'sell_volume_{window}s'] = self.calculate_directional_volume(trades_df, 'sell', window)
        
        features['buy_sell_ratio'] = features['buy_volume_300s'] / (features['sell_volume_300s'] + 1e-8)
        
        # Price momentum
        for window in self.lookback_windows:
            features[f'returns_{window}s'] = features['mid_price'].pct_change(window)
        
        return features
    
    def calculate_imbalance(self, orderbook_df: pd.DataFrame) -> pd.Series:
        """Calculate orderbook imbalance"""
        bid_vol = orderbook_df['bid_volume_total']
        ask_vol = orderbook_df['ask_volume_total']
        return (bid_vol - ask_vol) / (bid_vol + ask_vol + 1e-8)
    
    def calculate_volatility(self, price: pd.Series, window: int) -> pd.Series:
        """Calculate rolling volatility"""
        returns = price.pct_change()
        return returns.rolling(window).std() * np.sqrt(window)
    
    def calculate_rolling_volume(self, trades_df: pd.DataFrame, window: int) -> pd.Series:
        """Calculate rolling trade volume"""
        return trades_df['size'].rolling(f'{window}s').sum()
    
    def calculate_trade_count(self, trades_df: pd.DataFrame, window: int) -> pd.Series:
        """Calculate rolling trade count"""
        return trades_df['size'].rolling(f'{window}s').count()
    
    def calculate_directional_volume(self, trades_df: pd.DataFrame, side: str, window: int) -> pd.Series:
        """Calculate volume for specific trade side"""
        side_trades = trades_df[trades_df['side'] == side]
        return side_trades['size'].rolling(f'{window}s').sum()
```

### 12.2 Model Training

```python
# scripts/train_model.py

import ollama
import json
from pathlib import Path

class MarketMakerTrainer:
    def __init__(self, ollama_host: str = "http://localhost:11434"):
        self.client = ollama.Client(host=ollama_host)
        self.model_name = "market-maker-v1"
    
    def prepare_training_data(self, features_df: pd.DataFrame, decisions_df: pd.DataFrame) -> list:
        """Prepare instruction-completion pairs for fine-tuning"""
        
        training_data = []
        
        for idx, row in features_df.iterrows():
            # Get corresponding decision
            decision = decisions_df.loc[idx]
            
            # Create instruction
            instruction = f"""You are a market-making bot. Given the following market conditions:
- Mid price: {row['mid_price']:.4f}
- Spread: {row['spread_bps']:.2f} bps
- Orderbook imbalance: {row['imbalance']:.4f}
- Volatility (5min): {row['volatility_300s']:.4f}
- Recent volume: {row['volume_300s']:.2f}
- Buy/sell ratio: {row['buy_sell_ratio']:.4f}

What bid and ask prices and sizes should you quote? Respond in JSON format."""

            # Create completion
            completion = json.dumps({
                "bid_price": float(decision['bid_price']),
                "bid_size": float(decision['bid_size']),
                "ask_price": float(decision['ask_price']),
                "ask_size": float(decision['ask_size']),
                "confidence": float(decision['confidence']),
                "reasoning": decision['reasoning']
            })
            
            training_data.append({
                "instruction": instruction,
                "completion": completion
            })
        
        return training_data
    
    def train_model(self, training_data: list, base_model: str = "llama3.2:latest"):
        """Fine-tune Ollama model"""
        
        # Save training data to file
        training_file = Path("data/training_data.jsonl")
        with open(training_file, 'w') as f:
            for item in training_data:
                f.write(json.dumps(item) + '\n')
        
        # Create modelfile
        modelfile = f"""FROM {base_model}

SYSTEM You are a sophisticated market-making bot. You analyze market conditions and provide optimal bid/ask quotes with reasoning.

PARAMETER temperature 0.3
PARAMETER top_p 0.9
"""
        
        # Create model
        print(f"Creating model {self.model_name}...")
        self.client.create(model=self.model_name, modelfile=modelfile)
        
        # Note: Actual fine-tuning would require more setup with Ollama
        # This is a simplified example
        
        print(f"Model {self.model_name} created successfully")
    
    def evaluate_model(self, test_features: pd.DataFrame) -> dict:
        """Evaluate model performance on test set"""
        
        predictions = []
        
        for idx, row in test_features.iterrows():
            # Generate prediction
            prompt = self.create_prediction_prompt(row)
            response = self.client.generate(model=self.model_name, prompt=prompt)
            
            try:
                pred = json.loads(response['response'])
                predictions.append(pred)
            except:
                predictions.append(None)
        
        # Calculate metrics
        metrics = self.calculate_metrics(predictions, test_features)
        
        return metrics
    
    def create_prediction_prompt(self, features: pd.Series) -> str:
        """Create prompt for model inference"""
        return f"""Given market conditions:
- Mid price: {features['mid_price']:.4f}
- Spread: {features['spread_bps']:.2f} bps
- Orderbook imbalance: {features['imbalance']:.4f}
- Volatility (5min): {features['volatility_300s']:.4f}
- Recent volume: {features['volume_300s']:.2f}

Provide optimal quotes in JSON format."""

# Usage
if __name__ == '__main__':
    # Load data
    features = pd.read_parquet('data/features.parquet')
    decisions = pd.read_parquet('data/decisions.parquet')
    
    # Split train/test
    train_size = int(len(features) * 0.8)
    train_features = features[:train_size]
    train_decisions = decisions[:train_size]
    test_features = features[train_size:]
    
    # Train model
    trainer = MarketMakerTrainer()
    training_data = trainer.prepare_training_data(train_features, train_decisions)
    trainer.train_model(training_data)
    
    # Evaluate
    metrics = trainer.evaluate_model(test_features)
    print("Evaluation metrics:", metrics)
```

### 12.3 Ollama Integration in Rust

```rust
// src/ml/ollama_client.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OllamaClient {
    client: Client,
    endpoint: String,
    model_name: String,
}

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    temperature: f32,
    top_p: f32,
    num_predict: i32,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
    done: bool,
}

impl OllamaClient {
    pub fn new(endpoint: String, model_name: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model_name,
        }
    }
    
    pub async fn predict(&self, features: &MarketFeatures) -> Result<MLPrediction> {
        let prompt = self.build_prompt(features);
        
        let request = GenerateRequest {
            model: self.model_name.clone(),
            prompt,
            stream: false,
            options: GenerateOptions {
                temperature: 0.3,
                top_p: 0.9,
                num_predict: 256,
            },
        };
        
        let response = self.client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&request)
            .timeout(Duration::from_millis(500))
            .send()
            .await?;
        
        let generate_response: GenerateResponse = response.json().await?;
        
        // Parse JSON response
        let prediction: MLPrediction = serde_json::from_str(&generate_response.response)?;
        
        Ok(prediction)
    }
    
    fn build_prompt(&self, features: &MarketFeatures) -> String {
        format!(
            "Given market conditions:\n\
             - Mid price: {:.4}\n\
             - Spread: {:.2} bps\n\
             - Orderbook imbalance: {:.4}\n\
             - Volatility (5min): {:.4}\n\
             - Recent volume: {:.2}\n\
             - Buy/sell ratio: {:.4}\n\n\
             Provide optimal quotes in JSON format.",
            features.mid_price,
            features.spread_bps,
            features.imbalance,
            features.volatility_5m,
            features.volume_5m,
            features.buy_sell_ratio
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct MLPrediction {
    pub bid_price: Decimal,
    pub bid_size: Decimal,
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub confidence: f64,
    pub reasoning: Option<String>,
}
```

---

## Appendix

### A. Glossary

- **Market Making**: Strategy of providing liquidity by placing both buy and sell orders
- **Spread**: Difference between bid and ask prices
- **Orderbook**: List of all buy and sell orders at different price levels
- **Fill**: Execution of an order
- **Position**: Current holding in a market (long or short)
- **PnL**: Profit and Loss
- **Inventory Risk**: Risk of holding positions that move against you
- **Adverse Selection**: Getting filled on orders at bad times
- **Circuit Breaker**: Automatic trading halt on certain conditions

### B. Resources

**Polymarket Documentation**:
- API Docs: https://docs.polymarket.com
- CLOB Documentation: https://docs.polymarket.com/clob

**Rust Resources**:
- Tokio async runtime: https://tokio.rs
- Rust book: https://doc.rust-lang.org/book/

**Trading Resources**:
- Investopedia: https://www.investopedia.com
- QuantConnect forums: https://www.quantconnect.com/forum

### C. Troubleshooting

**Common Issues**:

1. **WebSocket disconnections**
   - Check network connectivity
   - Verify API credentials
   - Implement exponential backoff

2. **Orders not filling**
   - Check spread is competitive
   - Verify order sizes are reasonable
   - Review market liquidity

3. **High latency**
   - Optimize network path to exchange
   - Review code for blocking operations
   - Consider using faster serialization

4. **Database performance**
   - Add indexes on frequently queried columns
   - Use connection pooling
   - Consider archiving old data

### D. Future Enhancements

1. **Multi-exchange support**
2. **Advanced hedging strategies**
3. **Reinforcement learning integration**
4. **Automated parameter optimization**
5. **Web-based configuration UI**
6. **Mobile alerts app**
7. **Integration with TradingView for charting**

---

## ⚠️ FINAL REMINDERS

1. **START WITH PAPER TRADING** - Minimum 30 days
2. **USE TINY AMOUNTS** - Start with $50-100 maximum
3. **UNDERSTAND EVERYTHING** - Don't automate what you don't understand
4. **MONITOR CONSTANTLY** - Especially in first weeks
5. **EXPECT LOSSES** - Consider them tuition
6. **RISK MANAGEMENT FIRST** - Always prioritize safety over profits
7. **READ DOCUMENTATION** - Study Polymarket API docs thoroughly
8. **JOIN COMMUNITIES** - Learn from other algorithmic traders
9. **KEEP LEARNING** - Markets evolve, strategies must adapt
10. **HAVE AN EXIT PLAN** - Know when to stop

**This spec provides a complete roadmap, but success requires dedication, learning, and careful risk management. Good luck!**
