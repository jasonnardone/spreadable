# Spreadable - Polymarket Market-Making Bot Specification

**Version**: 1.0.0  
**Last Updated**: 2026-01-05  
**Purpose**: Automated market-making bot for Polymarket with ML-driven optimization

---

## 1. System Overview

### 1.1 Goals
- Low-latency automated trading (<100ms order placement)
- Support 10-50 concurrent markets
- ML integration with local Ollama models
- Comprehensive data collection for training
- Configuration-driven, hands-off operation

### 1.2 Technology Stack
- **Core**: Rust (tokio async)
- **Database**: PostgreSQL + TimescaleDB
- **ML**: Ollama (local LLM)
- **Deployment**: Docker + Docker Compose
- **Monitoring**: Prometheus + Grafana
- **Config**: TOML + environment variables

### 1.3 Architecture

```
Config Layer (TOML) → Core Engine (Rust) → Exchange API (WebSocket/REST)
                           ↓
                    Data Layer (PostgreSQL/TimescaleDB)
                           ↓
                    ML Pipeline (Python + Ollama)
                           ↓
                    Monitoring (Prometheus + Grafana)
```

---

## 2. Core Components

### 2.1 Market Data Handler
**Module**: `src/market_data/`

**Interface**:
```rust
pub trait MarketDataHandler {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, market_ids: Vec<String>) -> Result<()>;
    async fn get_orderbook(&self, market_id: &str) -> Option<OrderBook>;
    fn stream(&self) -> Receiver<MarketDataEvent>;
}

pub struct OrderBook {
    pub market_id: String,
    pub timestamp: i64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}
```

**Requirements**:
- WebSocket connection to Polymarket CLOB
- Maintain accurate local orderbook
- <10ms processing latency
- Automatic reconnection with exponential backoff

### 2.2 Strategy Engine
**Module**: `src/strategy/`

**Strategies**:
1. **BasicMM**: Fixed spread around mid-price with inventory skew
2. **AdaptiveSpread**: Dynamic spreads based on volatility
3. **MLEnhanced**: Ollama-powered predictions with fallback

**Interface**:
```rust
#[async_trait]
pub trait Strategy {
    fn name(&self) -> &str;
    async fn on_market_data(&mut self, event: &MarketDataEvent) -> Result<Vec<Order>>;
    fn calculate_quotes(&self, orderbook: &OrderBook, position: &Position) -> Quotes;
}

pub struct Quotes {
    pub bid_price: Decimal,
    pub bid_size: Decimal,
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub confidence: f64,
}
```

### 2.3 Order Management System (OMS)
**Module**: `src/oms/`

**Responsibilities**:
- Queue orders with rate limiting (5/second)
- Track order lifecycle (pending → open → filled)
- Handle cancellations and amendments
- Reconcile with exchange state

**Interface**:
```rust
pub struct OrderManager {
    pub async fn submit_order(&mut self, order: NewOrder) -> Result<OrderId>;
    pub async fn cancel_order(&mut self, order_id: OrderId) -> Result<()>;
    pub async fn cancel_all_orders(&mut self, market_id: Option<String>) -> Result<()>;
}
```

### 2.4 Risk Management
**Module**: `src/risk/`

**Checks**:
- Position limits per market
- Total exposure limits
- Daily/weekly loss limits
- Minimum spread enforcement
- Circuit breaker on rapid losses

**Interface**:
```rust
pub struct RiskManager {
    pub fn validate_order(&self, order: &NewOrder, position: &Position) -> Result<()>;
}

pub struct CircuitBreaker {
    pub fn check_and_trigger(&mut self, event: &RiskEvent) -> bool;
}
```

### 2.5 Database Layer
**Module**: `src/database/`

**Schema** (PostgreSQL + TimescaleDB):

```sql
-- Orderbook snapshots (hypertable)
CREATE TABLE orderbook_snapshots (
    market_id VARCHAR(100),
    timestamp TIMESTAMPTZ NOT NULL,
    bids JSONB,
    asks JSONB,
    mid_price DECIMAL(18,8),
    spread DECIMAL(10,6)
);

-- Orders
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    order_id VARCHAR(100) UNIQUE,
    market_id VARCHAR(100),
    side VARCHAR(10),
    price DECIMAL(18,8),
    size DECIMAL(18,8),
    status VARCHAR(20),
    created_at TIMESTAMPTZ
);

-- Position snapshots (hypertable)
CREATE TABLE position_snapshots (
    timestamp TIMESTAMPTZ NOT NULL,
    market_id VARCHAR(100),
    size DECIMAL(18,8),
    avg_entry_price DECIMAL(18,8),
    realized_pnl DECIMAL(18,8),
    unrealized_pnl DECIMAL(18,8)
);

-- Strategy decisions (for ML training)
CREATE TABLE strategy_decisions (
    timestamp TIMESTAMPTZ NOT NULL,
    market_id VARCHAR(100),
    features JSONB,  -- Market conditions
    decision JSONB,  -- Quote prices/sizes
    outcome JSONB,   -- Fills and PnL
    model_version VARCHAR(50)
);
```

---

## 3. Configuration

### 3.1 Main Config Structure
**File**: `config/production.toml`

```toml
[exchange]
api_key = "${POLYMARKET_API_KEY}"
api_secret = "${POLYMARKET_API_SECRET}"
clob_websocket = "wss://clob.polymarket.com/ws"
clob_rest_api = "https://clob.polymarket.com/api/v1"

[strategy]
active = "basic_mm"
quote_refresh_interval_ms = 2000

[strategy.basic_mm]
base_spread_bps = 500  # 5% spread
quote_size = 10.0
max_position = 100.0
skew_factor = 0.5

[oms]
max_orders_per_second = 5
max_pending_orders = 100

[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
max_order_size = 20.0
max_daily_loss = 50.0
min_spread_bps = 200
paper_trading_mode = true  # START WITH TRUE

[risk.circuit_breaker]
enabled = true
daily_loss_threshold = 50.0
loss_rate_pct = 0.05

[markets]
enabled = []  # Manually add after testing

[database]
postgres_url = "${DATABASE_URL}"

[ml]
ollama_endpoint = "http://localhost:11434"
model_name = "market-maker-v1"
inference_enabled = false  # Start false
```

---

## 4. ML Integration

### 4.1 Feature Engineering
**File**: `scripts/features.py`

**Extract these features**:
- Price: mid_price, spread, spread_bps
- Orderbook: imbalance, bid_depth, ask_depth
- Volatility: 1min, 5min, 15min rolling
- Volume: rolling trade volume, trade count
- Flow: buy vs sell volume ratio
- Momentum: price returns over windows

### 4.2 Ollama Integration
**File**: `src/ml/ollama_client.rs`

```rust
pub struct OllamaClient {
    pub async fn predict(&self, features: &MarketFeatures) -> Result<MLPrediction>;
}

pub struct MLPrediction {
    pub bid_price: Decimal,
    pub ask_price: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
    pub confidence: f64,
}
```

**Prompt Format**:
```
Given market conditions:
- Mid price: {mid_price}
- Spread: {spread_bps} bps
- Imbalance: {imbalance}
- Volatility: {volatility_5m}
- Volume: {volume_5m}

Provide optimal quotes in JSON format.
```

---

## 5. API Integration

### 5.1 Polymarket CLOB API

**WebSocket**: `wss://clob.polymarket.com/ws`  
**REST**: `https://clob.polymarket.com/api/v1`

**Authentication**: HMAC-SHA256 signed requests

**WebSocket Messages**:
```rust
pub enum WebSocketMessage {
    OrderBook {
        market_id: String,
        bids: Vec<[String; 2]>,
        asks: Vec<[String; 2]>,
        timestamp: i64,
    },
    Trade {
        market_id: String,
        price: String,
        size: String,
        side: String,
    },
}
```

**REST Endpoints**:
- `POST /orders` - Place order
- `DELETE /orders/{id}` - Cancel order
- `GET /orders?market_id={id}&status=open` - Get orders
- `GET /markets/{id}/orderbook` - Get orderbook

**Rate Limits**: 5 orders/second recommended

---

## 6. Deployment

### 6.1 Docker Setup

**docker-compose.yml**:
```yaml
version: '3.8'
services:
  postgres:
    image: timescale/timescaledb:latest-pg15
    environment:
      POSTGRES_DB: spreadable
      POSTGRES_USER: spreaduser
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  spreadable:
    build: .
    depends_on:
      - postgres
    environment:
      DATABASE_URL: postgresql://spreaduser:${DB_PASSWORD}@postgres:5432/spreadable
      POLYMARKET_API_KEY: ${POLYMARKET_API_KEY}
      POLYMARKET_API_SECRET: ${POLYMARKET_API_SECRET}
    volumes:
      - ./config:/etc/spreadable:ro
      - spreadable_data:/var/lib/spreadable
    ports:
      - "9090:9090"  # Metrics

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    ports:
      - "9091:9090"

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana

volumes:
  postgres_data:
  spreadable_data:
  grafana_data:
```

### 6.2 Dockerfile

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/spreadable /usr/local/bin/
RUN useradd -m spreaduser
USER spreaduser
ENTRYPOINT ["/usr/local/bin/spreadable"]
CMD ["--config", "/etc/spreadable/production.toml"]
```

---

## 7. Implementation Phases

### Phase 1: Foundation (Weeks 1-2)
**Deliverables**:
- WebSocket connection to Polymarket ✓
- Orderbook management ✓
- Database schema and logging ✓
- Paper trading mode ✓

**Success Criteria**:
- Zero data loss over 24 hours
- Orderbook stays in sync
- All events logged to database

### Phase 2: Trading Execution (Weeks 3-4)
**Deliverables**:
- Real order placement via API ✓
- Risk management system ✓
- Basic market-making strategy ✓
- Monitoring dashboard ✓

**Success Criteria**:
- All orders within risk limits
- Circuit breaker works correctly
- Test with $50-100 on ONE market

### Phase 3: Strategy Enhancement (Weeks 5-8)
**Deliverables**:
- Adaptive spread strategy ✓
- Backtesting framework ✓
- Multi-market support (3-5 markets) ✓
- Latency optimization (<100ms) ✓

**Success Criteria**:
- Positive PnL over 2 weeks
- Fill rate >30%
- Uptime >99%

### Phase 4: ML Integration (Weeks 9-12)
**Deliverables**:
- Feature engineering pipeline ✓
- Ollama model training ✓
- ML-enhanced strategy ✓
- Automated retraining ✓

**Success Criteria**:
- ML outperforms baseline
- Inference <500ms
- Automated retraining works

---

## 8. Monitoring

### 8.1 Key Metrics

**Prometheus Metrics**:
```rust
// Connection
websocket_connected

// Trading
orders_placed_total
orders_filled_total
order_placement_latency_ms

// Position & PnL
current_position
realized_pnl_usd
unrealized_pnl_usd
daily_pnl_usd

// Risk
circuit_breaker_triggered
risk_violations_total

// Performance
fill_rate
spread_capture_rate
```

### 8.2 Alerts

**Critical Alerts**:
- Circuit breaker triggered
- WebSocket disconnected >1min
- Daily loss >$40

**Warning Alerts**:
- Fill rate <20% for 15min
- Order latency p95 >200ms
- Daily loss approaching limit

---

## 9. Project Structure

```
spreadable/
├── Cargo.toml
├── docker-compose.yml
├── Dockerfile
├── .env
├── config/
│   ├── development.toml
│   ├── staging.toml
│   └── production.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── market_data/
│   │   ├── mod.rs
│   │   ├── websocket_client.rs
│   │   └── orderbook_manager.rs
│   ├── strategy/
│   │   ├── mod.rs
│   │   ├── base_strategy.rs
│   │   ├── basic_mm.rs
│   │   ├── adaptive_spread.rs
│   │   └── ml_enhanced.rs
│   ├── oms/
│   │   ├── mod.rs
│   │   ├── order_manager.rs
│   │   └── rate_limiter.rs
│   ├── risk/
│   │   ├── mod.rs
│   │   ├── risk_manager.rs
│   │   ├── position_tracker.rs
│   │   └── circuit_breaker.rs
│   ├── database/
│   │   ├── mod.rs
│   │   └── client.rs
│   ├── ml/
│   │   ├── mod.rs
│   │   └── ollama_client.rs
│   └── monitoring/
│       ├── mod.rs
│       └── metrics.rs
├── scripts/
│   ├── features.py
│   ├── train_model.py
│   └── backtest.py
├── sql/
│   ├── schema.sql
│   └── migrations/
├── monitoring/
│   ├── prometheus.yml
│   └── alerts.yml
└── tests/
    ├── unit/
    └── integration/
```

---

## 10. Setup Instructions

### 10.1 Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh

# Install Ollama
curl https://ollama.ai/install.sh | sh
ollama pull llama3.2:latest
```

### 10.2 Initial Setup
```bash
# Clone/create project
cd spreadable

# Create environment file
cat > .env << 'EOF'
POLYMARKET_API_KEY=your_key_here
POLYMARKET_API_SECRET=your_secret_here
DB_PASSWORD=secure_password
EOF

# Start database
docker-compose up -d postgres

# Initialize database
psql $DATABASE_URL -f sql/schema.sql

# Build project
cargo build --release

# Run in paper trading mode
cargo run --release -- --config config/production.toml
```

### 10.3 First Run Checklist
- [ ] API credentials configured
- [ ] Database initialized
- [ ] `paper_trading_mode = true` in config
- [ ] Risk limits set conservatively
- [ ] Monitoring dashboard accessible
- [ ] Test connection to Polymarket
- [ ] Verify data logging works

---

## 11. Safety & Risk

### 11.1 Critical Safety Rules
1. **ALWAYS start with paper trading** (30+ days minimum)
2. **Start with tiny capital** ($50-100 maximum)
3. **Conservative risk limits** (use defaults in config)
4. **Monitor constantly** first 2 weeks
5. **Circuit breaker enabled** at all times
6. **Have kill switch ready** (cancel all orders)

### 11.2 Risk Parameters (Start Values)
```toml
max_position_per_market = 100.0   # $100 max per market
max_total_exposure = 500.0        # $500 total
max_order_size = 20.0             # $20 per order
max_daily_loss = 50.0             # $50 daily loss limit
min_spread_bps = 200              # 2% minimum spread
base_spread_bps = 500             # 5% starting spread
```

### 11.3 Circuit Breaker Triggers
- Daily loss exceeds $50
- Loss rate >5% in 15min window
- WebSocket disconnection >60s
- Manual trigger via API

---

## 12. Key Data Models

```rust
pub struct OrderBook {
    pub market_id: String,
    pub timestamp: i64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

pub struct Position {
    pub market_id: String,
    pub size: Decimal,
    pub avg_entry_price: Decimal,
    pub realized_pnl: Decimal,
}

pub struct Order {
    pub id: String,
    pub market_id: String,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub status: OrderStatus,
}

pub enum Side { Buy, Sell }

pub enum OrderStatus {
    Pending,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}
```

---

## 13. Testing Strategy

### 13.1 Unit Tests
- Orderbook calculations (mid-price, spread, imbalance)
- Position tracking (PnL updates, average entry)
- Risk checks (position limits, loss limits)

### 13.2 Integration Tests
- Full trading flow (market data → strategy → order → fill)
- Database persistence
- Circuit breaker triggering

### 13.3 Backtesting
- Load historical orderbook data
- Simulate strategy execution
- Calculate metrics: return, Sharpe ratio, max drawdown, fill rate

### 13.4 Paper Trading
- **Required**: 30+ days minimum
- Track all metrics as if trading real money
- Verify all systems work correctly
- Build confidence before going live

---

## ⚠️ CRITICAL WARNINGS

1. **Trading involves significant financial risk** - You can lose all invested capital
2. **Start with paper trading** - Minimum 30 days, no exceptions
3. **Use tiny amounts** - $50-100 maximum when starting live
4. **Understand everything** - Don't automate what you don't understand
5. **Expect losses initially** - Consider them tuition
6. **Monitor constantly** - Especially first weeks
7. **Have kill switch ready** - Be able to stop instantly
8. **Check regulations** - Ensure compliance with your jurisdiction
9. **No guaranteed profits** - Good bots can still lose money
10. **Polymarket T&Cs** - Verify bot usage is permitted

---

## Appendix: Quick Reference

### Cargo Dependencies
```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = "0.21"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
rust_decimal = "1.33"
config = "0.13"
tracing = "0.1"
prometheus = "0.13"
reqwest = { version = "0.11", features = ["json"] }
anyhow = "1.0"
```

### Environment Variables
```bash
# Required
POLYMARKET_API_KEY=xxx
POLYMARKET_API_SECRET=xxx
DATABASE_URL=postgresql://user:pass@host:5432/db

# Optional
RUST_LOG=info
MM_BOT__RISK__PAPER_TRADING_MODE=true
```

### Common Commands
```bash
# Build
cargo build --release

# Run with config
cargo run --release -- --config config/production.toml

# Run tests
cargo test

# Check logs
tail -f /var/log/spreadable/app.log

# Monitor metrics
curl http://localhost:9090/metrics
```

---

**End of Specification**

Ready to build with `specify init` → Development → Testing → Deployment
