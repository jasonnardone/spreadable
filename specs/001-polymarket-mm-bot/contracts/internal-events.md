# Internal Events Contract

**Feature**: Polymarket Market-Making Bot
**Date**: 2026-01-05
**Purpose**: Define internal event bus contracts for communication between system components

## Overview

This document specifies the internal event types used for communication between components (market data handler, strategy engine, OMS, risk manager, database client). Events flow through async channels (tokio::mpsc) to maintain low-latency, non-blocking operation.

## Event Bus Architecture

```
MarketDataHandler → [MarketDataEvent] → StrategyEngine
                                      ↓
StrategyEngine → [TradingDecision] → RiskManager
                                   ↓
RiskManager → [ValidatedOrder] → OrderManager
                              ↓
OrderManager → [OrderEvent] → PositionTracker
                           ↓
PositionTracker → [PositionUpdate] → Database + Metrics
                                   ↓
RiskManager ← [RiskEvent] ← CircuitBreaker
```

## Event Types

### 1. MarketDataEvent

**Source**: MarketDataHandler (WebSocket client)
**Consumers**: StrategyEngine, Database
**Purpose**: Propagate orderbook updates and trades from Polymarket

```rust
#[derive(Debug, Clone)]
pub enum MarketDataEvent {
    OrderbookSnapshot {
        market_id: String,
        orderbook: OrderBook,
        timestamp: Timestamp,
    },
    OrderbookDelta {
        market_id: String,
        changes: Vec<PriceLevelChange>,
        timestamp: Timestamp,
    },
    Trade {
        market_id: String,
        trade_id: String,
        price: Decimal,
        size: Decimal,
        side: TradeSide,
        timestamp: Timestamp,
    },
    ConnectionStatus {
        market_id: String,
        connected: bool,
        timestamp: Timestamp,
    },
}

#[derive(Debug, Clone)]
pub struct PriceLevelChange {
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,  // 0 = remove level
}

#[derive(Debug, Clone, Copy)]
pub enum TradeSide {
    Buy,   // Taker bought
    Sell,  // Taker sold
}
```

**Delivery Guarantees**:
- **At-least-once**: Snapshots may be duplicated on reconnect
- **Ordered**: Deltas for same market are sequentially ordered
- **Buffered**: Channel capacity 1000 messages, drops oldest on overflow

**Validation**:
- `orderbook` must have valid mid_price and spread
- `changes` array must not be empty
- `timestamp` must be recent (< 60 seconds old)

---

### 2. TradingDecision

**Source**: StrategyEngine
**Consumers**: RiskManager
**Purpose**: Communicate strategy's desired quotes to risk manager for validation

```rust
#[derive(Debug, Clone)]
pub struct TradingDecision {
    pub decision_id: Uuid,
    pub market_id: String,
    pub strategy_name: String,
    pub timestamp: Timestamp,
    pub quotes: CalculatedQuotes,
    pub features: MarketFeatures,  // For audit trail
}

#[derive(Debug, Clone)]
pub struct CalculatedQuotes {
    pub bid_price: Decimal,
    pub bid_size: Decimal,
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub confidence: f64,  // 0.0 - 1.0
}

#[derive(Debug, Clone)]
pub struct MarketFeatures {
    pub mid_price: Decimal,
    pub spread_bps: u32,
    pub imbalance: f64,
    pub volatility_5m: f64,
    pub volume_5m: Decimal,
    pub current_position: Decimal,
}
```

**Constraints**:
- `bid_price < ask_price` (no crossed quotes)
- `bid_price, ask_price > 0` and `<= 1.0` (probability constraints)
- `bid_size, ask_size > 0`
- `confidence` between 0.0 and 1.0

**Flow**:
1. Strategy calculates quotes from orderbook + position
2. Emits TradingDecision event
3. Risk manager validates against limits
4. If valid → ValidatedOrder, if rejected → RiskEvent

---

### 3. ValidatedOrder

**Source**: RiskManager
**Consumers**: OrderManager
**Purpose**: Order that passed risk checks, ready for submission

```rust
#[derive(Debug, Clone)]
pub struct ValidatedOrder {
    pub order_id: Uuid,
    pub decision_id: Uuid,  // Links back to TradingDecision
    pub market_id: String,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub time_in_force: TimeInForce,
    pub validated_at: Timestamp,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeInForce {
    GTC,  // Good-Till-Cancel
    IOC,  // Immediate-Or-Cancel
}
```

**Guarantees**:
- Order has passed all risk checks (position limits, exposure, daily loss)
- Order is ready for immediate submission to exchange
- No further validation required

---

### 4. OrderEvent

**Source**: OrderManager
**Consumers**: PositionTracker, Database, Metrics
**Purpose**: Notify about order lifecycle changes and fills

```rust
#[derive(Debug, Clone)]
pub enum OrderEvent {
    Submitted {
        order_id: Uuid,
        exchange_order_id: Option<String>,
        market_id: String,
        side: OrderSide,
        price: Decimal,
        size: Decimal,
        timestamp: Timestamp,
    },
    Opened {
        order_id: Uuid,
        exchange_order_id: String,
        timestamp: Timestamp,
    },
    PartiallyFilled {
        order_id: Uuid,
        exchange_order_id: String,
        fill: Fill,
        total_filled: Decimal,
        remaining: Decimal,
        timestamp: Timestamp,
    },
    Filled {
        order_id: Uuid,
        exchange_order_id: String,
        fills: Vec<Fill>,
        total_filled: Decimal,
        avg_fill_price: Decimal,
        timestamp: Timestamp,
    },
    Cancelled {
        order_id: Uuid,
        exchange_order_id: Option<String>,
        reason: CancelReason,
        timestamp: Timestamp,
    },
    Rejected {
        order_id: Uuid,
        reason: String,
        timestamp: Timestamp,
    },
    Failed {
        order_id: Uuid,
        error: String,
        timestamp: Timestamp,
    },
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub fill_id: String,
    pub price: Decimal,
    pub size: Decimal,
    pub fee: Decimal,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub enum CancelReason {
    UserRequested,
    CircuitBreaker,
    RiskViolation,
    MarketClosed,
}
```

**Usage**:
- PositionTracker listens for PartiallyFilled + Filled to update positions
- Database persists all events for audit trail
- Metrics emits counters/histograms

**Delivery**:
- **At-least-once**: Critical events (Filled) may be duplicated
- **Idempotent Processing**: Consumers handle duplicates (check order_id)

---

### 5. PositionUpdate

**Source**: PositionTracker
**Consumers**: RiskManager, Database, Metrics
**Purpose**: Notify about position changes and P&L updates

```rust
#[derive(Debug, Clone)]
pub struct PositionUpdate {
    pub market_id: String,
    pub position: Position,
    pub trigger: PositionTrigger,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub market_id: String,
    pub size: Decimal,
    pub avg_entry_price: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub total_pnl: Decimal,
    pub trade_count: u64,
}

#[derive(Debug, Clone)]
pub enum PositionTrigger {
    Fill { order_id: Uuid, fill: Fill },
    MarkToMarket { new_mid_price: Decimal },
    DailyReset,
}
```

**Constraints**:
- `total_pnl = realized_pnl + unrealized_pnl` always
- `unrealized_pnl = (current_price - avg_entry_price) * size` when size != 0

**Flow**:
1. PositionTracker receives OrderEvent::Filled
2. Updates position calculations
3. Emits PositionUpdate
4. RiskManager checks against position limits
5. Database persists snapshot
6. Metrics updates gauges

---

### 6. RiskEvent

**Source**: RiskManager, CircuitBreaker
**Consumers**: OrderManager, Metrics, Database
**Purpose**: Communicate risk violations and circuit breaker triggers

```rust
#[derive(Debug, Clone)]
pub enum RiskEvent {
    OrderRejected {
        order_id: Uuid,
        decision_id: Uuid,
        violation: RiskViolation,
        timestamp: Timestamp,
    },
    CircuitBreakerTriggered {
        reason: CircuitBreakerReason,
        triggered_at: Timestamp,
    },
    CircuitBreakerReset {
        reset_at: Timestamp,
        reset_by: String,
    },
    PositionLimitWarning {
        market_id: String,
        current_position: Decimal,
        limit: Decimal,
        utilization_pct: f64,
        timestamp: Timestamp,
    },
    DailyLossWarning {
        current_loss: Decimal,
        limit: Decimal,
        utilization_pct: f64,
        timestamp: Timestamp,
    },
}

#[derive(Debug, Clone)]
pub enum RiskViolation {
    OrderSizeExceeded { order_size: Decimal, max: Decimal },
    PositionLimitExceeded { new_position: Decimal, max: Decimal },
    TotalExposureExceeded { total: Decimal, max: Decimal },
    DailyLossExceeded { daily_loss: Decimal, max: Decimal },
    SpreadTooNarrow { spread_bps: u32, min: u32 },
    CircuitBreakerActive,
}

#[derive(Debug, Clone)]
pub enum CircuitBreakerReason {
    DailyLossExceeded { daily_loss: Decimal },
    LossRateExceeded { loss_rate: f64, window: Duration },
    WebSocketDisconnected { duration: Duration },
    ManualTrigger { reason: String },
}
```

**Criticality**:
- **CircuitBreakerTriggered**: Highest priority, triggers immediate order cancellation
- **OrderRejected**: Log violation, don't submit order
- **Warnings**: Informational, log for monitoring

**Action on Receipt**:
- OrderManager: Cancel all orders on CircuitBreakerTriggered
- Database: Persist all events for compliance
- Metrics: Increment violation counters, trigger alerts

---

### 7. MetricEvent

**Source**: All components
**Consumers**: Metrics module, Prometheus exporter
**Purpose**: Emit measurements for observability

```rust
#[derive(Debug, Clone)]
pub struct MetricEvent {
    pub metric_name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Copy)]
pub enum MetricType {
    Counter,     // Monotonically increasing
    Gauge,       // Point-in-time value
    Histogram,   // Distribution
}
```

**Common Metrics**:
- `websocket_connected` (Gauge): 1 = connected, 0 = disconnected
- `orders_placed_total` (Counter): Total orders submitted
- `order_placement_latency_ms` (Histogram): Submission latency
- `current_position` (Gauge): Per-market position size
- `daily_pnl_usd` (Gauge): Realized + unrealized P&L
- `circuit_breaker_triggered` (Counter): CB activations
- `fill_rate` (Gauge): Percentage of orders filled

**Labels**:
- `market_id`: Identify which market
- `strategy`: Which strategy generated order
- `side`: Buy or sell

---

## Channel Configuration

### Channel Capacities

```rust
// High-frequency channels (orderbook updates)
const MARKET_DATA_CHANNEL_SIZE: usize = 1000;

// Medium-frequency channels (trading decisions, orders)
const TRADING_CHANNEL_SIZE: usize = 100;

// Low-frequency channels (risk events, position updates)
const RISK_CHANNEL_SIZE: usize = 50;

// Metrics (high volume but non-critical)
const METRICS_CHANNEL_SIZE: usize = 500;
```

**Overflow Behavior**:
- **MarketData**: Drop oldest (latest data more important)
- **Trading/Risk**: Block sender (backpressure to prevent order flood)
- **Metrics**: Drop newest (don't block critical path)

### Channel Creation Pattern

```rust
use tokio::sync::mpsc;

// Create channels
let (market_data_tx, market_data_rx) = mpsc::channel::<MarketDataEvent>(1000);
let (trading_decision_tx, trading_decision_rx) = mpsc::channel::<TradingDecision>(100);
let (validated_order_tx, validated_order_rx) = mpsc::channel::<ValidatedOrder>(100);
let (order_event_tx, order_event_rx) = mpsc::channel::<OrderEvent>(100);
let (position_update_tx, position_update_rx) = mpsc::channel::<PositionUpdate>(50);
let (risk_event_tx, risk_event_rx) = mpsc::channel::<RiskEvent>(50);
let (metric_event_tx, metric_event_rx) = mpsc::channel::<MetricEvent>(500);

// Pass senders to components
let market_data_handler = MarketDataHandler::new(market_data_tx.clone());
let strategy_engine = StrategyEngine::new(trading_decision_tx.clone());
let risk_manager = RiskManager::new(validated_order_tx, risk_event_tx.clone());
let order_manager = OrderManager::new(order_event_tx.clone());
let position_tracker = PositionTracker::new(position_update_tx);
```

---

## Error Handling

### Send Errors

**Scenario**: Receiver dropped (component crashed)

**Handling**:
```rust
if let Err(e) = tx.send(event).await {
    error!("Failed to send event: {}, receiver dropped", e);
    // Critical: If receiver is gone, component crashed
    // Options:
    // 1. Log error + continue (non-critical path)
    // 2. Trigger shutdown (critical component)
    // 3. Restart component (advanced)
}
```

**Policy**:
- **Critical Path** (OrderEvent → PositionTracker): Trigger shutdown
- **Non-Critical** (MetricEvent): Log warning, continue

### Receive Timeouts

**Scenario**: No events for extended period

**Handling**:
```rust
use tokio::time::timeout;

match timeout(Duration::from_secs(30), rx.recv()).await {
    Ok(Some(event)) => {
        // Process event
    }
    Ok(None) => {
        // Channel closed
        warn!("Event channel closed, shutting down");
        break;
    }
    Err(_) => {
        // Timeout - no events for 30 seconds
        // This may be normal (no trading activity)
        debug!("No events received in 30 seconds");
    }
}
```

---

## Testing Contracts

### Unit Test Patterns

```rust
#[tokio::test]
async fn test_market_data_event_flow() {
    let (tx, mut rx) = mpsc::channel::<MarketDataEvent>(10);

    // Send mock event
    let event = MarketDataEvent::OrderbookSnapshot {
        market_id: "test_market".to_string(),
        orderbook: create_mock_orderbook(),
        timestamp: Timestamp::now(),
    };

    tx.send(event.clone()).await.unwrap();

    // Receive and validate
    let received = rx.recv().await.unwrap();
    match received {
        MarketDataEvent::OrderbookSnapshot { market_id, .. } => {
            assert_eq!(market_id, "test_market");
        }
        _ => panic!("Expected OrderbookSnapshot"),
    }
}
```

### Integration Test Patterns

```rust
#[tokio::test]
async fn test_end_to_end_order_flow() {
    // Setup all channels
    let (market_data_tx, market_data_rx) = mpsc::channel(10);
    let (trading_decision_tx, trading_decision_rx) = mpsc::channel(10);
    // ... other channels

    // Spawn components
    tokio::spawn(async move {
        strategy_engine.run(market_data_rx, trading_decision_tx).await
    });
    tokio::spawn(async move {
        risk_manager.run(trading_decision_rx, validated_order_tx).await
    });
    // ... other components

    // Send mock market data
    market_data_tx.send(create_mock_snapshot()).await.unwrap();

    // Assert: Order event received
    let order_event = timeout(Duration::from_secs(1), order_event_rx.recv())
        .await
        .expect("Timeout waiting for order event")
        .expect("Channel closed");

    assert!(matches!(order_event, OrderEvent::Submitted { .. }));
}
```

---

## Observability

### Tracing Integration

All event handlers should use structured logging:

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(event), fields(market_id = %event.market_id()))]
async fn handle_market_data_event(event: MarketDataEvent) {
    info!("Processing market data event");

    match event {
        MarketDataEvent::OrderbookSnapshot { market_id, .. } => {
            info!("Received orderbook snapshot");
            // Process...
        }
        MarketDataEvent::ConnectionStatus { connected, .. } => {
            if !connected {
                warn!("Market data connection lost");
            } else {
                info!("Market data connection restored");
            }
        }
        _ => {}
    }
}
```

### Metrics for Event Bus

- `event_channel_capacity`: Gauge (current channel usage)
- `event_send_errors_total`: Counter (send failures)
- `event_processing_latency_ms`: Histogram (time from emit to process)

---

## Conclusion

This internal event contract ensures reliable, low-latency communication between system components. Key design decisions:

1. **Async Channels**: Non-blocking, high-throughput event delivery
2. **Typed Events**: Rust enums provide compile-time validation
3. **Bounded Capacity**: Prevents unbounded memory growth
4. **Backpressure**: Blocks senders when consumers can't keep up (for critical paths)
5. **Observability**: All events traced for debugging

Next: See `quickstart.md` for operator onboarding guide.
