# Data Model: Polymarket Market-Making Bot

**Feature**: Polymarket Market-Making Bot
**Date**: 2026-01-05
**Purpose**: Define core entities, relationships, state transitions, and validation rules

## Overview

This document specifies the data models for the market-making bot, including market data structures, trading entities (orders, positions), risk management state, and audit trail records. Models are designed for low-latency in-memory operation with async persistence to PostgreSQL + TimescaleDB.

## Core Entities

### 1. Market

Represents a Polymarket prediction market with current orderbook state.

**Fields**:
- `market_id`: String (unique identifier from Polymarket)
- `question`: String (market question, e.g., "Will BTC reach $100k by Dec 2024?")
- `status`: MarketStatus enum (Active, Paused, Settled, Delisted)
- `orderbook`: OrderBook (current bid/ask state)
- `last_update_time`: Timestamp (UTC, when orderbook last updated)
- `subscription_active`: Boolean (WebSocket subscription status)

**Relationships**:
- Has one OrderBook (current state)
- Has many Positions (one per market)
- Has many Orders (historical and active)
- Has many StrategyDecisions (audit trail)

**Invariants**:
- `market_id` must not be empty
- `last_update_time` must not be more than 60 seconds old when `subscription_active = true`
- `orderbook` must have at least one bid and one ask when `status = Active`

**State Transitions**:
```
[Initial] → Active (when subscribed)
Active → Paused (exchange pause)
Active → Settled (market resolved)
Active → Delisted (market removed)
Paused → Active (resume trading)
Settled → [Terminal]
Delisted → [Terminal]
```

---

### 2. OrderBook

Snapshot of bid and ask price levels at a point in time.

**Fields**:
- `market_id`: String (reference to Market)
- `timestamp`: Timestamp (UTC, when snapshot captured)
- `bids`: Vec<PriceLevel> (buy orders, sorted descending by price)
- `asks`: Vec<PriceLevel> (sell orders, sorted ascending by price)
- `mid_price`: Decimal (calculated as `(best_bid + best_ask) / 2`)
- `spread`: Decimal (calculated as `best_ask - best_bid`)
- `spread_bps`: u32 (spread in basis points, `(spread / mid_price) * 10000`)

**Nested Structures**:
```rust
pub struct PriceLevel {
    pub price: Decimal,  // Price point
    pub size: Decimal,   // Total size at this price
}
```

**Derived Calculations**:
- `best_bid`: First element of `bids` vector
- `best_ask`: First element of `asks` vector
- `imbalance`: `(bid_depth - ask_depth) / (bid_depth + ask_depth)` where depth is sum of sizes
- `bid_depth`: Sum of all bid sizes
- `ask_depth`: Sum of all ask sizes

**Invariants**:
- `bids` must be sorted descending by price (highest first)
- `asks` must be sorted ascending by price (lowest first)
- All prices and sizes must be positive
- `mid_price` must be between `best_bid` and `best_ask`
- `spread` must be non-negative

**Validation Rules**:
- Reject orderbook if `mid_price <= 0`
- Reject if `best_bid >= best_ask` (crossed book)
- Log warning if `spread_bps > 10000` (>100% spread indicates illiquid market)

---

### 3. Order

Represents a limit order in its lifecycle from submission to terminal state.

**Fields**:
- `id`: OrderId (internal UUID)
- `exchange_order_id`: Option<String> (Polymarket order ID, None if paper trading)
- `market_id`: String (reference to Market)
- `side`: OrderSide enum (Buy, Sell)
- `price`: Decimal (limit price)
- `size`: Decimal (order quantity)
- `filled_size`: Decimal (quantity filled so far, 0 initially)
- `status`: OrderStatus enum (see state machine below)
- `created_at`: Timestamp (UTC, when order created)
- `submitted_at`: Option<Timestamp> (when sent to exchange)
- `updated_at`: Timestamp (last status change)
- `fills`: Vec<Fill> (partial fills)

**Nested Structures**:
```rust
pub struct Fill {
    pub timestamp: Timestamp,
    pub price: Decimal,
    pub size: Decimal,
    pub fee: Decimal,
}

pub enum OrderSide {
    Buy,
    Sell,
}

pub enum OrderStatus {
    Pending,         // Created, awaiting submission
    Submitted,       // Sent to exchange, awaiting confirmation
    Open,            // Confirmed by exchange, resting in book
    PartiallyFilled, // Some fills, still open
    Filled,          // Fully filled
    Cancelled,       // Cancelled by user or system
    Rejected,        // Rejected by exchange or risk manager
    Failed,          // Submission failed (network, API error)
}
```

**State Transitions**:
```
[Initial] → Pending (order created)

Pending → Submitted (sent to exchange)
Pending → Rejected (failed pre-trade risk check)

Submitted → Open (exchange confirms)
Submitted → Failed (network error, API rejection)

Open → PartiallyFilled (first fill received)
Open → Filled (single fill for entire size)
Open → Cancelled (cancellation confirmed)

PartiallyFilled → PartiallyFilled (additional fills)
PartiallyFilled → Filled (final fill)
PartiallyFilled → Cancelled (cancelled with partial fill)

Filled → [Terminal]
Cancelled → [Terminal]
Rejected → [Terminal]
Failed → [Terminal]
```

**Invariants**:
- `filled_size <= size` always
- `fills.sum(size) == filled_size`
- Terminal states (Filled, Cancelled, Rejected, Failed) cannot transition
- `price > 0` and `size > 0`
- `status = PartiallyFilled` iff `0 < filled_size < size`
- `status = Filled` iff `filled_size == size`

**Validation Rules**:
- Reject order if `price <= 0` or `size <= 0`
- Reject if `size > max_order_size` from RiskConfig
- Ensure `side` matches intended position direction

---

### 4. Position

Current trading position in a specific market with P&L tracking.

**Fields**:
- `market_id`: String (reference to Market)
- `size`: Decimal (net position, positive = long, negative = short, 0 = flat)
- `avg_entry_price`: Decimal (volume-weighted average entry price)
- `realized_pnl`: Decimal (locked-in profit/loss from closed trades)
- `unrealized_pnl`: Decimal (mark-to-market P&L on open position)
- `total_pnl`: Decimal (realized + unrealized)
- `trade_count`: u64 (number of fills contributing to this position)
- `last_updated`: Timestamp (when position last changed)

**Derived Calculations**:
```rust
// Update on fill
fn update_position(&mut self, fill: &Fill, side: OrderSide, current_price: Decimal) {
    let fill_size_signed = match side {
        OrderSide::Buy => fill.size,
        OrderSide::Sell => -fill.size,
    };

    // Update avg entry price
    if self.size.signum() == fill_size_signed.signum() {
        // Adding to position
        self.avg_entry_price = (self.avg_entry_price * self.size + fill.price * fill.size)
                                / (self.size + fill_size_signed);
    } else if self.size.abs() < fill.size.abs() {
        // Closing position and reversing
        let pnl = (fill.price - self.avg_entry_price) * self.size;
        self.realized_pnl += pnl;
        self.avg_entry_price = fill.price;  // New entry price
    } else {
        // Reducing position
        let closed_size = fill.size.min(self.size.abs());
        let pnl = (fill.price - self.avg_entry_price) * closed_size * self.size.signum();
        self.realized_pnl += pnl;
    }

    self.size += fill_size_signed;
    self.trade_count += 1;

    // Update unrealized P&L
    self.unrealized_pnl = (current_price - self.avg_entry_price) * self.size;
    self.total_pnl = self.realized_pnl + self.unrealized_pnl;
    self.last_updated = Timestamp::now();
}
```

**Invariants**:
- `total_pnl = realized_pnl + unrealized_pnl` always
- `avg_entry_price > 0` when `size != 0`
- When `size = 0`, `unrealized_pnl = 0` (flat position has no unrealized)

**Validation Rules**:
- Validate position doesn't exceed `max_position_per_market` from RiskConfig
- Alert if total exposure across all markets exceeds `max_total_exposure`

---

### 5. RiskLimit

Configured constraints for risk management.

**Fields**:
- `max_position_per_market`: Decimal (max absolute position size per market, e.g., $100)
- `max_total_exposure`: Decimal (max sum of absolute positions across all markets, e.g., $500)
- `max_order_size`: Decimal (max size for single order, e.g., $20)
- `max_daily_loss`: Decimal (max cumulative loss per day, e.g., $50)
- `min_spread_bps`: u32 (minimum spread in basis points, e.g., 200 = 2%)
- `paper_trading_mode`: Boolean (if true, don't submit real orders)

**Invariants**:
- All Decimal fields must be positive
- `max_order_size <= max_position_per_market`
- `max_position_per_market * num_markets <= max_total_exposure` (approximately)

**Validation Logic**:
```rust
pub fn validate_order(
    &self,
    order: &Order,
    position: &Position,
    total_exposure: Decimal,
    daily_loss: Decimal
) -> Result<(), RiskError> {
    // Check 1: Order size
    if order.size > self.max_order_size {
        return Err(RiskError::OrderSizeExceeded {
            order_size: order.size,
            max: self.max_order_size,
        });
    }

    // Check 2: Position limit
    let new_position_size = match order.side {
        OrderSide::Buy => position.size + order.size,
        OrderSide::Sell => position.size - order.size,
    };
    if new_position_size.abs() > self.max_position_per_market {
        return Err(RiskError::PositionLimitExceeded {
            new_position: new_position_size,
            max: self.max_position_per_market,
        });
    }

    // Check 3: Total exposure
    let exposure_change = order.size;  // Approximate
    if total_exposure + exposure_change > self.max_total_exposure {
        return Err(RiskError::TotalExposureExceeded {
            total: total_exposure + exposure_change,
            max: self.max_total_exposure,
        });
    }

    // Check 4: Daily loss limit
    if daily_loss.abs() > self.max_daily_loss {
        return Err(RiskError::DailyLossExceeded {
            daily_loss,
            max: self.max_daily_loss,
        });
    }

    Ok(())
}
```

---

### 6. CircuitBreaker

Emergency trading halt mechanism.

**Fields**:
- `enabled`: Boolean (circuit breaker active)
- `triggered`: Boolean (currently in triggered state)
- `trigger_reason`: Option<TriggerReason> (why circuit breaker activated)
- `triggered_at`: Option<Timestamp> (when activated)
- `daily_loss_threshold`: Decimal (loss amount that triggers CB, e.g., $50)
- `loss_rate_threshold_pct`: f64 (loss rate in 15min that triggers, e.g., 0.05 = 5%)
- `loss_window_minutes`: u32 (time window for rate calculation, e.g., 15)

**Nested Structures**:
```rust
pub enum TriggerReason {
    DailyLossExceeded { daily_loss: Decimal },
    LossRateExceeded { loss_rate: f64, window: Duration },
    WebSocketDisconnected { duration: Duration },
    ManualTrigger { reason: String },
}
```

**State Transitions**:
```
[Standby] → Triggered (loss threshold exceeded)
Triggered → Standby (manual reset after investigation)
```

**Trigger Logic**:
```rust
pub fn check_and_trigger(&mut self, event: &RiskEvent) -> bool {
    if !self.enabled {
        return false;
    }

    let should_trigger = match event {
        RiskEvent::DailyLoss(loss) => loss.abs() >= self.daily_loss_threshold,
        RiskEvent::LossRate { rate, window } => {
            *rate >= self.loss_rate_threshold_pct
                && *window == Duration::from_minutes(self.loss_window_minutes)
        }
        RiskEvent::WebSocketDisconnected(duration) => *duration > Duration::from_secs(60),
        RiskEvent::ManualTrigger(_) => true,
    };

    if should_trigger && !self.triggered {
        self.triggered = true;
        self.trigger_reason = Some(event.into());
        self.triggered_at = Some(Timestamp::now());

        // Emit alert
        error!("CIRCUIT BREAKER TRIGGERED: {:?}", self.trigger_reason);

        // Cancel all orders (handled by OrderManager)
    }

    self.triggered
}
```

**Invariants**:
- When `triggered = true`, `trigger_reason` and `triggered_at` must be `Some`
- When `triggered = false`, system can submit orders (if other checks pass)

---

### 7. StrategyDecision

Audit trail of trading decisions for ML training and analysis.

**Fields**:
- `id`: UUID (unique identifier)
- `timestamp`: Timestamp (UTC, when decision made)
- `market_id`: String (which market)
- `strategy_name`: String (e.g., "basic_mm", "adaptive_spread", "ml_enhanced")
- `features`: JSON (market conditions at decision time)
  - `mid_price`: Decimal
  - `spread_bps`: u32
  - `imbalance`: f64
  - `volatility_5m`: f64
  - `volume_5m`: Decimal
  - `current_position`: Decimal
- `decision`: JSON (calculated quotes)
  - `bid_price`: Decimal
  - `bid_size`: Decimal
  - `ask_price`: Decimal
  - `ask_size`: Decimal
  - `confidence`: f64 (0.0-1.0)
- `outcome`: JSON (what happened after decision)
  - `orders_submitted`: u32
  - `orders_filled`: u32
  - `pnl_change`: Decimal (realized P&L from fills)
  - `fill_latency_ms`: Option<u64>
- `model_version`: Option<String> (for ML strategies, which model version used)

**Purpose**:
- Training data for ML model improvement
- Post-trade analysis to evaluate strategy performance
- Debugging strategy behavior in production

**Invariants**:
- `features` and `decision` are never null
- `outcome` populated after reasonable time window (e.g., 60 seconds)
- `model_version` required when `strategy_name = "ml_enhanced"`

---

### 8. Metric

Time-series measurement for observability.

**Fields**:
- `timestamp`: Timestamp (UTC, when measurement taken)
- `metric_name`: String (e.g., "order_placement_latency_ms")
- `metric_type`: MetricType enum (Counter, Gauge, Histogram)
- `value`: f64 (metric value)
- `labels`: HashMap<String, String> (dimensions, e.g., {"market_id": "abc123", "side": "buy"})

**Metric Types**:
```rust
pub enum MetricType {
    Counter,    // Monotonically increasing (e.g., orders_placed_total)
    Gauge,      // Point-in-time value (e.g., current_position)
    Histogram,  // Distribution (e.g., latency)
}
```

**Key Metrics**:
- `websocket_connected`: Gauge (1 = connected, 0 = disconnected)
- `orders_placed_total`: Counter (total orders submitted)
- `orders_filled_total`: Counter (total fills received)
- `order_placement_latency_ms`: Histogram (time from decision to submission)
- `current_position`: Gauge per market (current position size)
- `daily_pnl_usd`: Gauge (realized + unrealized P&L today)
- `circuit_breaker_triggered`: Counter (CB activation count)
- `fill_rate`: Gauge (percentage of orders filled)

---

## Entity Relationships

```
Market 1:1 OrderBook (current state)
Market 1:N Orders (all orders for this market)
Market 1:1 Position (current position)
Market 1:N StrategyDecisions (decision history)

Order N:1 Market
Order 1:N Fills (partial fills for this order)

Position 1:1 Market
Position updated by Order fills

RiskLimit applied to all Orders and Positions
CircuitBreaker monitors all Positions and Orders

StrategyDecision N:1 Market
StrategyDecision triggers Order creation

Metric tagged with market_id references Market
```

## Database Schema (PostgreSQL + TimescaleDB)

```sql
-- Markets (regular table, low frequency updates)
CREATE TABLE markets (
    market_id VARCHAR(100) PRIMARY KEY,
    question TEXT NOT NULL,
    status VARCHAR(20) NOT NULL,
    subscription_active BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Orderbook snapshots (hypertable for high frequency)
CREATE TABLE orderbook_snapshots (
    market_id VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    bids JSONB NOT NULL,
    asks JSONB NOT NULL,
    mid_price DECIMAL(18,8),
    spread DECIMAL(18,8),
    spread_bps INTEGER,
    PRIMARY KEY (market_id, timestamp)
);
SELECT create_hypertable('orderbook_snapshots', 'timestamp');
CREATE INDEX idx_orderbook_market ON orderbook_snapshots(market_id, timestamp DESC);

-- Orders (regular table with indexes)
CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    exchange_order_id VARCHAR(100),
    market_id VARCHAR(100) NOT NULL REFERENCES markets(market_id),
    side VARCHAR(10) NOT NULL,
    price DECIMAL(18,8) NOT NULL,
    size DECIMAL(18,8) NOT NULL,
    filled_size DECIMAL(18,8) DEFAULT 0,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    fills JSONB DEFAULT '[]'::jsonb
);
CREATE INDEX idx_orders_market_status ON orders(market_id, status, created_at DESC);
CREATE INDEX idx_orders_exchange_id ON orders(exchange_order_id) WHERE exchange_order_id IS NOT NULL;

-- Positions (regular table, one row per market)
CREATE TABLE positions (
    market_id VARCHAR(100) PRIMARY KEY REFERENCES markets(market_id),
    size DECIMAL(18,8) DEFAULT 0,
    avg_entry_price DECIMAL(18,8),
    realized_pnl DECIMAL(18,8) DEFAULT 0,
    unrealized_pnl DECIMAL(18,8) DEFAULT 0,
    total_pnl DECIMAL(18,8) DEFAULT 0,
    trade_count BIGINT DEFAULT 0,
    last_updated TIMESTAMPTZ DEFAULT NOW()
);

-- Position snapshots (hypertable for historical tracking)
CREATE TABLE position_snapshots (
    timestamp TIMESTAMPTZ NOT NULL,
    market_id VARCHAR(100) NOT NULL,
    size DECIMAL(18,8),
    avg_entry_price DECIMAL(18,8),
    realized_pnl DECIMAL(18,8),
    unrealized_pnl DECIMAL(18,8),
    total_pnl DECIMAL(18,8),
    PRIMARY KEY (market_id, timestamp)
);
SELECT create_hypertable('position_snapshots', 'timestamp');

-- Strategy decisions (hypertable for ML training data)
CREATE TABLE strategy_decisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL,
    market_id VARCHAR(100) NOT NULL REFERENCES markets(market_id),
    strategy_name VARCHAR(50) NOT NULL,
    features JSONB NOT NULL,
    decision JSONB NOT NULL,
    outcome JSONB,
    model_version VARCHAR(50)
);
SELECT create_hypertable('strategy_decisions', 'timestamp');
CREATE INDEX idx_strategy_decisions_market ON strategy_decisions(market_id, timestamp DESC);

-- Metrics (hypertable for observability)
CREATE TABLE metrics (
    timestamp TIMESTAMPTZ NOT NULL,
    metric_name VARCHAR(100) NOT NULL,
    metric_type VARCHAR(20) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    labels JSONB DEFAULT '{}'::jsonb
);
SELECT create_hypertable('metrics', 'timestamp');
CREATE INDEX idx_metrics_name_time ON metrics(metric_name, timestamp DESC);
```

## Conclusion

This data model provides the foundation for safe, observable market-making operations. Key design decisions:

1. **In-Memory First**: Core entities (OrderBook, Position) optimized for low-latency access
2. **Async Persistence**: TimescaleDB hypertables handle high-frequency writes without blocking trading
3. **Audit Trail**: StrategyDecisions and position_snapshots enable ML training and compliance
4. **State Machines**: Clear state transitions for Orders and CircuitBreaker prevent invalid states
5. **Validation**: Invariants and validation rules enforced at entity level prevent data corruption

The schema supports all user stories from P1 (data collection) through P4 (ML optimization) while maintaining strict risk controls and comprehensive observability.
