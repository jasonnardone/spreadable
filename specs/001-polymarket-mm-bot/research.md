# Research: Polymarket Market-Making Bot

**Feature**: Polymarket Market-Making Bot
**Date**: 2026-01-05
**Purpose**: Document architectural decisions, technology choices, and best practices for automated trading system implementation

## Executive Summary

This document captures research findings and architectural decisions for building a low-latency, high-reliability market-making bot for Polymarket prediction markets. Key decisions prioritize safety (paper trading, circuit breakers), performance (async Rust, <100ms latency), and observability (structured logging, Prometheus metrics) to support automated 24/7 operation handling real financial transactions.

## 1. Core Technology Stack

### Decision: Rust 1.75+ with Tokio Async Runtime

**Rationale**:
- **Performance**: Rust provides zero-cost abstractions and compile-time guarantees needed for <100ms order placement latency targets
- **Safety**: Ownership system prevents data races and memory errors critical for financial applications handling real money
- **Async Ecosystem**: Tokio provides battle-tested async runtime for concurrent WebSocket connections, database writes, and HTTP requests
- **Low Latency**: No garbage collection pauses that could cause missed trading opportunities

**Alternatives Considered**:
- **Python**: Rejected due to GIL limiting concurrency, slower execution (10-100x), GC pauses incompatible with latency requirements
- **Go**: Rejected due to GC pauses (10-100ms), less strict type safety, weaker ecosystem for financial precision (Decimal types)
- **C++**: Rejected due to manual memory management complexity, longer development time, harder to maintain

**Best Practices**:
- Use `#![deny(unwrap_used)]` lint to prevent runtime panics in production
- Leverage `Result<T, E>` and `?` operator for error propagation
- Use `rust_decimal` crate for financial calculations (avoids floating point precision issues)
- Structure code as async state machines with clear lifecycle management

## 2. WebSocket Connection Management

### Decision: tokio-tungstenite with Exponential Backoff Reconnection

**Rationale**:
- **Reliability**: Market data is mission-critical; automatic reconnection prevents data loss during network blips
- **Backoff Strategy**: Exponential backoff (1s, 2s, 4s, 8s, max 60s) prevents thundering herd during Polymarket outages
- **Connection Pooling**: Single persistent connection per market reduces overhead, maintains low latency

**Implementation Pattern**:
```rust
// Reconnection loop with exponential backoff
let mut retry_delay = Duration::from_secs(1);
loop {
    match websocket_client.connect().await {
        Ok(_) => {
            retry_delay = Duration::from_secs(1); // Reset on success
            // Process messages...
        }
        Err(e) => {
            warn!("WebSocket connection failed: {}, retrying in {:?}", e, retry_delay);
            sleep(retry_delay).await;
            retry_delay = min(retry_delay * 2, Duration::from_secs(60));
        }
    }
}
```

**Best Practices**:
- Implement heartbeat/ping-pong to detect stale connections
- Buffer critical events during reconnection windows
- Log all connection state transitions for debugging
- Test reconnection logic with integration tests (simulate network failures)

## 3. Database Architecture

### Decision: PostgreSQL 15+ with TimescaleDB Extension

**Rationale**:
- **Time-Series Optimization**: TimescaleDB hypertables optimize storage and queries for orderbook snapshots (thousands per second)
- **ACID Guarantees**: Critical for financial data (order fills, PnL calculations) where data loss is unacceptable
- **JSON Support**: JSONB columns efficiently store variable-structure data (strategy features, market metadata)
- **Mature Ecosystem**: SQLx provides compile-time query validation, connection pooling, async support

**Schema Design**:
```sql
-- Hypertable for high-frequency orderbook data
CREATE TABLE orderbook_snapshots (
    market_id VARCHAR(100),
    timestamp TIMESTAMPTZ NOT NULL,
    bids JSONB,  -- [{price, size}, ...]
    asks JSONB,
    mid_price DECIMAL(18,8),
    spread DECIMAL(10,6)
);
SELECT create_hypertable('orderbook_snapshots', 'timestamp');

-- Regular table for orders (lower frequency, needs ACID)
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    order_id VARCHAR(100) UNIQUE,
    market_id VARCHAR(100),
    side VARCHAR(10),
    price DECIMAL(18,8),
    size DECIMAL(18,8),
    status VARCHAR(20),
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
);
CREATE INDEX idx_orders_market_status ON orders(market_id, status);
```

**Best Practices**:
- Use async batch writes to avoid blocking trading loop
- Implement connection pooling (min 5, max 20 connections)
- Set retention policies on hypertables (compress after 7 days, drop after 90 days)
- Monitor query performance with `EXPLAIN ANALYZE`

## 4. Risk Management Architecture

### Decision: Multi-Layer Risk Controls with Circuit Breaker Pattern

**Rationale**:
- **Defense in Depth**: Multiple independent checks (per-order, position, daily loss, rate of loss) prevent single point of failure
- **Circuit Breaker**: Automatic trading halt prevents runaway losses during market anomalies
- **Fail-Safe**: System defaults to safe state (cancel all orders, stop trading) on any critical error

**Risk Layer Hierarchy**:
1. **Pre-Trade Validation**: Check every order against limits before submission
   - Max position per market ($100 default)
   - Max total exposure across markets ($500 default)
   - Max order size ($20 default)
   - Minimum spread (200 bps default)

2. **Position Monitoring**: Real-time tracking of P&L
   - Update position on every fill
   - Calculate realized + unrealized P&L
   - Emit metrics for monitoring

3. **Circuit Breaker**: Emergency halt conditions
   - Daily loss exceeds threshold ($50 default)
   - Loss rate >5% in 15-minute window
   - WebSocket disconnected >60 seconds
   - Manual trigger via kill switch

**Implementation Pattern**:
```rust
pub struct RiskManager {
    config: RiskConfig,
    position_tracker: PositionTracker,
    circuit_breaker: CircuitBreaker,
}

impl RiskManager {
    pub fn validate_order(&self, order: &NewOrder, position: &Position) -> Result<(), RiskError> {
        // Layer 1: Per-order checks
        if order.size > self.config.max_order_size {
            return Err(RiskError::OrderSizeExceeded);
        }

        // Layer 2: Position checks
        let new_position = position.size + order.effective_size();
        if new_position.abs() > self.config.max_position_per_market {
            return Err(RiskError::PositionLimitExceeded);
        }

        // Layer 3: Circuit breaker check
        if self.circuit_breaker.is_triggered() {
            return Err(RiskError::CircuitBreakerActive);
        }

        Ok(())
    }
}
```

**Best Practices**:
- Test circuit breaker with integration tests (simulate loss scenarios)
- Log all risk violations with full context for auditing
- Make risk limits configurable without code changes
- Implement "dry-run" mode to test changes before production

## 5. Observability Strategy

### Decision: Structured Logging (tracing) + Prometheus Metrics + Grafana Dashboards

**Rationale**:
- **Debugging**: Structured logs with correlation IDs enable tracing requests across async boundaries
- **Real-Time Monitoring**: Prometheus metrics provide live system health visibility
- **Alerting**: Grafana alerts notify operators of critical issues (circuit breaker, loss limits)
- **Performance Analysis**: Histogram metrics identify latency bottlenecks

**Logging Strategy**:
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self), fields(market_id = %order.market_id, order_id = %order.id))]
async fn submit_order(&mut self, order: NewOrder) -> Result<OrderId> {
    info!("Submitting order: side={}, price={}, size={}",
          order.side, order.price, order.size);

    match self.api_client.submit(order).await {
        Ok(order_id) => {
            info!("Order submitted successfully");
            Ok(order_id)
        }
        Err(e) => {
            error!("Order submission failed: {}", e);
            Err(e.into())
        }
    }
}
```

**Key Metrics**:
```rust
// Connection status
websocket_connected: Gauge  // 1 = connected, 0 = disconnected

// Trading activity
orders_placed_total: Counter
orders_filled_total: Counter
order_placement_latency_ms: Histogram  // p50, p95, p99

// Positions & P&L
current_position: Gauge  // Per market
realized_pnl_usd: Gauge
unrealized_pnl_usd: Gauge
daily_pnl_usd: Gauge

// Risk events
circuit_breaker_triggered: Counter
risk_violations_total: Counter  // By type

// Performance
fill_rate: Gauge  // Percentage of orders filled
spread_capture_rate: Gauge  // Average spread captured
```

**Best Practices**:
- Use `#[instrument]` macro for automatic span creation
- Include correlation IDs in all log messages
- Set log levels appropriately (ERROR for failures, WARN for degraded, INFO for normal events)
- Create Grafana alerts for critical metrics (circuit breaker, daily loss approaching limit)

## 6. Rate Limiting Strategy

### Decision: Token Bucket Algorithm with Configurable Limit

**Rationale**:
- **Exchange Compliance**: Polymarket limits to 5 orders/second; client-side limiting prevents bans
- **Burst Handling**: Token bucket allows short bursts while maintaining average rate
- **Backpressure**: Queue orders when rate limit reached, avoid dropping

**Implementation Pattern**:
```rust
pub struct RateLimiter {
    tokens: Arc<Mutex<f64>>,
    max_tokens: f64,
    refill_rate: f64,  // tokens/second
    last_refill: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    pub async fn acquire(&self) -> Result<()> {
        loop {
            {
                let mut tokens = self.tokens.lock().await;
                let mut last_refill = self.last_refill.lock().await;

                // Refill tokens based on elapsed time
                let now = Instant::now();
                let elapsed = now.duration_since(*last_refill).as_secs_f64();
                *tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
                *last_refill = now;

                if *tokens >= 1.0 {
                    *tokens -= 1.0;
                    return Ok(());
                }
            }

            // Wait for next token
            sleep(Duration::from_millis(200)).await;
        }
    }
}
```

**Best Practices**:
- Set max_tokens to allow small bursts (e.g., 10 tokens, 5/sec refill)
- Log when rate limit is reached for tuning
- Make rate configurable per environment (higher for testing, strict for production)
- Implement metrics tracking queue depth and wait times

## 7. Testing Strategy

### Decision: Multi-Level Testing with TDD for Critical Paths

**Test Pyramid**:
1. **Unit Tests** (fast, isolated, >80% coverage target)
   - Orderbook calculations (mid-price, spread, imbalance)
   - Position tracking (P&L, average entry price)
   - Risk validation logic (limit checks)
   - Rate limiter token bucket behavior

2. **Integration Tests** (slower, external dependencies)
   - WebSocket lifecycle (connect, disconnect, reconnect)
   - Database persistence (writes, queries, hypertable compression)
   - Order flow end-to-end (submit → exchange → fill → position update)
   - Circuit breaker triggering

3. **Contract Tests** (API compatibility)
   - Polymarket WebSocket message parsing
   - REST API request/response formats
   - Internal event schemas

4. **Performance Tests** (latency validation)
   - Order placement latency (<100ms p95)
   - Orderbook update processing (<10ms p99)
   - Memory usage under load (<512MB for 50 markets)

**TDD Workflow for Critical Paths**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_manager_rejects_order_exceeding_position_limit() {
        // Red: Write failing test first
        let config = RiskConfig { max_position_per_market: 100.0, ..Default::default() };
        let risk_manager = RiskManager::new(config);
        let position = Position { size: Decimal::from(95), ..Default::default() };
        let order = NewOrder { size: Decimal::from(10), ..Default::default() };

        let result = risk_manager.validate_order(&order, &position);

        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), RiskError::PositionLimitExceeded);
    }

    // Green: Implement minimum code to make test pass
    // Refactor: Clean up implementation
}
```

**Best Practices**:
- Use `testcontainers` for integration tests requiring PostgreSQL
- Mock Polymarket API with `wiremock` for deterministic testing
- Run unit tests on every commit, integration tests on PR
- Use `criterion` for benchmarking latency-critical paths

## 8. Deployment Strategy

### Decision: Docker Compose for Development, Docker for Production

**Rationale**:
- **Consistency**: Same container runs in dev, staging, production
- **Isolation**: Each service (bot, postgres, prometheus, grafana) in separate container
- **Simplicity**: Docker Compose orchestrates multi-service setup for local development

**Docker Build Strategy**:
```dockerfile
# Multi-stage build for minimal production image
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

**Best Practices**:
- Use multi-stage builds to minimize image size (builder + runtime)
- Run as non-root user for security
- Mount config as read-only volume
- Use health checks in docker-compose.yml
- Tag images with git commit SHA for traceability

## 9. Configuration Management

### Decision: TOML Files + Environment Variables

**Rationale**:
- **Human-Readable**: TOML more readable than JSON/YAML for configuration
- **Type-Safe Parsing**: `config` crate validates at startup with helpful errors
- **Secrets Separation**: Environment variables for API keys, files for non-sensitive settings
- **Multi-Environment**: Separate files (development.toml, staging.toml, production.toml)

**Configuration Schema**:
```toml
[exchange]
api_key = "${POLYMARKET_API_KEY}"  # From environment
api_secret = "${POLYMARKET_API_SECRET}"
clob_websocket = "wss://clob.polymarket.com/ws"
clob_rest_api = "https://clob.polymarket.com/api/v1"

[strategy]
active = "basic_mm"  # basic_mm | adaptive_spread | ml_enhanced
quote_refresh_interval_ms = 2000

[strategy.basic_mm]
base_spread_bps = 500  # 5% spread
quote_size = 10.0
max_position = 100.0
skew_factor = 0.5

[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
max_order_size = 20.0
max_daily_loss = 50.0
min_spread_bps = 200
paper_trading_mode = true  # MUST start true

[risk.circuit_breaker]
enabled = true
daily_loss_threshold = 50.0
loss_rate_pct = 0.05  # 5% loss in 15min triggers

[markets]
enabled = []  # Start empty, add after testing

[database]
postgres_url = "${DATABASE_URL}"
max_connections = 20
min_connections = 5

[ml]
ollama_endpoint = "http://localhost:11434"
model_name = "market-maker-v1"
inference_enabled = false  # Start false
```

**Best Practices**:
- Validate configuration on startup, fail fast with clear errors
- Provide `.env.example` with dummy values
- Document all config options in inline comments
- Use different defaults per environment (verbose logging in dev, strict limits in prod)
- Version control config files (except production secrets)

## 10. Paper Trading Implementation

### Decision: Conditional Order Submission with Full Simulation

**Rationale**:
- **Risk-Free Validation**: Test all logic (data collection, strategy, risk) without financial risk
- **Realistic Testing**: Simulate full order lifecycle including fills
- **Mandatory Period**: 30+ days required before live trading per constitution
- **Performance Baseline**: Establish expected metrics (fill rate, latency) before real money

**Implementation Pattern**:
```rust
pub struct OrderManager {
    config: OMSConfig,
    api_client: PolymarketClient,
    paper_trading: bool,
}

impl OrderManager {
    pub async fn submit_order(&mut self, order: NewOrder) -> Result<OrderId> {
        // Full pre-trade validation (same as live)
        self.risk_manager.validate_order(&order, &self.position)?;

        if self.paper_trading {
            // Paper trading: simulate submission, generate fake order ID
            let order_id = format!("PAPER-{}", uuid::Uuid::new_v4());
            info!("PAPER TRADE: Would submit order: {:?}", order);

            // Simulate fill after delay (based on historical fill rates)
            self.simulate_fill(order_id.clone(), &order).await?;

            Ok(order_id)
        } else {
            // Live trading: actual API call
            let order_id = self.api_client.submit_order(order).await?;
            info!("LIVE TRADE: Order submitted: {}", order_id);
            Ok(order_id)
        }
    }

    async fn simulate_fill(&self, order_id: String, order: &NewOrder) {
        // Simulate random fill delay (1-10 seconds)
        sleep(Duration::from_secs(rand::random::<u64>() % 10 + 1)).await;

        // Simulate partial or full fill based on historical data
        let fill_pct = if rand::random::<f64>() < 0.7 { 1.0 } else { 0.5 };
        self.emit_fill_event(order_id, order.size * fill_pct).await;
    }
}
```

**Best Practices**:
- Make paper trading a top-level config flag, easy to toggle
- Log all paper trades clearly with "PAPER TRADE" prefix
- Track paper trading metrics separately for comparison with live
- Require explicit config change + restart to enable live trading (no hot-swap)

## Conclusion

These architectural decisions prioritize safety, performance, and observability for a system handling real financial transactions. The technology stack (Rust + Tokio + PostgreSQL + Prometheus) provides the foundation for low-latency, reliable operation. Multi-layer risk controls and mandatory paper trading ensure safe rollout. Comprehensive testing and monitoring enable confident production deployment.

**Next Steps**:
1. Proceed to Phase 1: Data Model Design
2. Define detailed entity schemas and relationships
3. Document API contracts (Polymarket integration, internal events)
4. Create quickstart guide for operator onboarding
