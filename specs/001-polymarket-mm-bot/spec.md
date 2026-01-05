# Feature Specification: Polymarket Market-Making Bot

**Feature Branch**: `001-polymarket-mm-bot`
**Created**: 2026-01-05
**Status**: Draft
**Input**: User description: "Create a feature spec based on the requirements in spreadable-spec-condensed.md"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Safe Market Data Collection (Priority: P1)

An operator needs to safely collect real-time market data from Polymarket without risking any capital, building confidence that the system accurately tracks market conditions before attempting any trading activity.

**Why this priority**: Data collection is the foundation of all trading activities. Without accurate, reliable market data, no trading decisions can be made. This is the minimal viable system that delivers immediate value by enabling market observation and strategy backtesting.

**Independent Test**: Can be fully tested by deploying the system in paper trading mode, verifying that orderbook data is collected continuously for 24 hours without data loss, and confirming all market events are logged to the database for later analysis.

**Acceptance Scenarios**:

1. **Given** the system is configured in paper trading mode with valid API credentials, **When** the operator starts the bot and subscribes to 3 Polymarket markets, **Then** the system maintains real-time orderbook state for all markets with updates processed in under 10 milliseconds
2. **Given** the WebSocket connection drops unexpectedly, **When** network connectivity is restored, **Then** the system automatically reconnects within 60 seconds and resumes data collection without manual intervention
3. **Given** the system has been running for 24 hours, **When** the operator queries the database, **Then** all orderbook snapshots, market events, and position updates are stored with no gaps in the timeline
4. **Given** the operator accesses the monitoring dashboard, **When** viewing system health metrics, **Then** connection status, data processing latency, and database write status are clearly displayed with real-time updates

---

### User Story 2 - Controlled Risk-Limited Trading (Priority: P2)

An operator needs to execute automated market-making trades on a single market with strict position and loss limits, ensuring the bot cannot exceed predefined risk thresholds even if market conditions change rapidly.

**Why this priority**: After validating data collection, the next critical step is proving the bot can execute trades safely. This story delivers the core value proposition (automated trading) while maintaining safety through comprehensive risk controls.

**Independent Test**: Can be fully tested by configuring the bot with conservative limits ($100 max position, $50 daily loss limit), enabling live trading on one low-volatility market with $50-100 capital, and verifying all orders stay within limits over a 48-hour period.

**Acceptance Scenarios**:

1. **Given** risk limits are configured (max $100 position per market, $20 max order size, $50 daily loss limit), **When** the bot calculates and submits orders, **Then** all orders are validated against limits before submission and rejected orders are logged with clear reasons
2. **Given** the bot is actively quoting on a market, **When** market conditions change and position approaches the limit, **Then** the bot adjusts quote sizes or stops quoting to prevent exceeding position limits
3. **Given** daily losses approach $40 (80% of $50 limit), **When** the next loss would exceed the limit, **Then** the circuit breaker triggers, all open orders are cancelled, and the operator receives an alert
4. **Given** orders are placed successfully, **When** they are filled by other market participants, **Then** positions are updated in real-time, profit/loss is calculated accurately, and all fills are logged to the database
5. **Given** the bot has been trading for one week, **When** the operator reviews performance metrics, **Then** fill rate, total profit/loss, daily PnL trends, and risk limit violations are clearly reported

---

### User Story 3 - Multi-Market Operations (Priority: P3)

An operator needs to run the bot across 10-20 markets simultaneously, diversifying trading activity and capturing opportunities across multiple prediction markets while maintaining independent risk controls per market.

**Why this priority**: After proving single-market safety and profitability, scaling to multiple markets increases revenue potential and reduces concentration risk. This represents the "production-ready" configuration for serious operation.

**Independent Test**: Can be fully tested by configuring 10 markets with individual position limits, running the bot for 72 hours, and verifying that each market operates independently (limits enforced per-market, fills in one market don't affect others).

**Acceptance Scenarios**:

1. **Given** the bot is configured with 10 active markets, **When** the system starts, **Then** WebSocket connections are established for all markets and orderbooks are maintained independently for each
2. **Given** multiple markets are active, **When** one market experiences high volatility, **Then** increased activity in that market does not degrade performance or latency in other markets
3. **Given** each market has a $100 position limit and $500 total exposure limit, **When** positions are accumulated across markets, **Then** per-market limits and total exposure limits are both enforced independently
4. **Given** the operator monitors the dashboard, **When** viewing multi-market status, **Then** all market connection states, current positions, and individual PnLs are displayed in a consolidated view

---

### User Story 4 - ML-Enhanced Strategy Optimization (Priority: P4)

An operator needs to leverage machine learning predictions to improve quote pricing and sizing decisions, potentially increasing profitability compared to simpler rule-based strategies while maintaining fallback behavior if ML inference fails.

**Why this priority**: This is an enhancement that may improve returns but is not required for core functionality. ML adds complexity and must be proven to outperform baseline strategies before production use.

**Independent Test**: Can be fully tested by training a model on 30 days of collected historical data, running the ML-enhanced strategy in paper trading mode for 14 days, and comparing fill rates and simulated PnL against the baseline strategy over the same period.

**Acceptance Scenarios**:

1. **Given** 30+ days of orderbook data and strategy decisions are stored in the database, **When** the operator runs the feature engineering pipeline, **Then** market features (price, spread, volatility, volume, imbalance) are calculated and exported for model training
2. **Given** the ML model is trained and deployed, **When** the bot requests quote predictions, **Then** the model returns suggested bid/ask prices and sizes within 500 milliseconds
3. **Given** the ML inference endpoint becomes unavailable, **When** the bot attempts to get predictions, **Then** the system automatically falls back to the baseline rule-based strategy without interrupting trading
4. **Given** the ML strategy has been running for 14 days in paper trading mode, **When** the operator compares performance metrics, **Then** the ML strategy demonstrates measurably higher fill rates or better PnL than the baseline with statistical significance

---

### Edge Cases

- What happens when the WebSocket connection experiences intermittent connectivity (frequent disconnects/reconnects within 60 seconds)?
- How does the system handle API rate limit errors when order submission exceeds 5 orders per second?
- What happens when the Polymarket API returns malformed orderbook data or unexpected message formats?
- How does the risk manager behave when position limits are exceeded due to unexpected fills during a reconnection window?
- What happens when database writes fail due to connectivity issues - are critical events buffered or lost?
- How does the circuit breaker handle edge cases like exactly meeting the loss threshold versus exceeding it?
- What happens when the system time and exchange timestamps are significantly out of sync?
- How does the bot handle markets that are paused, settled, or delisted during active trading?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST maintain real-time orderbook state for all subscribed markets with updates processed in under 10 milliseconds
- **FR-002**: System MUST automatically reconnect to Polymarket WebSocket API with exponential backoff when connections drop, with maximum 60-second recovery time
- **FR-003**: System MUST persist all orderbook snapshots, trade events, order lifecycle state changes, and position updates to a time-series database with no data loss
- **FR-004**: System MUST validate all outgoing orders against configured risk limits (max position per market, max total exposure, max order size, daily loss limit) before submission
- **FR-005**: System MUST reject any order that would violate risk limits and log the rejection reason with full context (market, current position, attempted order, limit violated)
- **FR-006**: System MUST trigger a circuit breaker that cancels all open orders and halts trading when daily loss limit is reached or loss rate exceeds 5% in 15 minutes
- **FR-007**: System MUST respect Polymarket API rate limits by queuing orders and submitting at maximum 5 orders per second
- **FR-008**: System MUST track order lifecycle states (pending, open, partially filled, filled, cancelled) and reconcile with exchange state
- **FR-009**: System MUST calculate and update positions in real-time as orders are filled, including average entry price, realized PnL, and unrealized PnL
- **FR-010**: System MUST expose health check endpoint reporting WebSocket connection status, database connectivity, current positions, and circuit breaker state
- **FR-011**: System MUST emit metrics (connection status, order counts, latency histograms, positions, PnL) in Prometheus format for monitoring
- **FR-012**: System MUST support paper trading mode where all trading logic executes normally but no real orders are submitted to the exchange
- **FR-013**: System MUST log all significant events (order placement, fills, risk violations, circuit breaker triggers) with structured context (timestamps, market IDs, order IDs, correlation data)
- **FR-014**: Operators MUST be able to configure all risk parameters, strategy settings, and market selections via configuration files without code changes
- **FR-015**: System MUST validate configuration files on startup and provide clear error messages for invalid or missing required parameters
- **FR-016**: System MUST calculate market-making quotes (bid/ask prices and sizes) based on current orderbook state and position using configurable strategies (basic spread, adaptive spread, ML-enhanced)
- **FR-017**: System MUST provide a kill switch mechanism that immediately cancels all open orders across all markets when triggered manually or by critical system errors
- **FR-018**: System MUST store all strategy decisions (market conditions, calculated quotes, outcomes) for post-trade analysis and ML model training

### Key Entities

- **Market**: Represents a Polymarket prediction market with unique identifier, current orderbook state (bids/asks), mid-price, spread, and subscription status
- **OrderBook**: Snapshot of bid and ask price levels for a specific market at a point in time, including timestamp, price levels with sizes, calculated mid-price and spread
- **Order**: A limit order with unique identifier, market, side (buy/sell), price, size, current status, submission timestamp, and fill history
- **Position**: Current trading position in a specific market including size (positive for long, negative for short), average entry price, realized profit/loss, unrealized profit/loss based on current market price
- **RiskLimit**: Configured constraints including max position per market, max total exposure across all markets, max order size, daily loss limit, minimum spread requirements
- **CircuitBreaker**: Risk control mechanism that monitors loss rates and triggers emergency trading halt, tracking trigger conditions, activation state, and manual override capability
- **StrategyDecision**: Record of a trading decision including market conditions (features), calculated quotes, execution outcome (fills, PnL change), and strategy version used
- **Metric**: Time-series measurement of system behavior including connection status, order latency, fill rates, positions, PnL, and operational health indicators

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: System operates continuously for 24 hours in paper trading mode with zero data loss and 100% orderbook update accuracy verified against exchange API snapshots
- **SC-002**: System processes market data updates in under 10 milliseconds at 99th percentile during normal market conditions (under 100 updates per second per market)
- **SC-003**: System submits orders with end-to-end latency under 100 milliseconds at 95th percentile measured from orderbook update receipt to API submission
- **SC-004**: All risk limits are enforced with 100% accuracy over 7 days of live trading - zero instances of position limits, exposure limits, or loss limits being violated
- **SC-005**: Circuit breaker activates within 1 second when loss thresholds are met, with all open orders successfully cancelled within 5 seconds
- **SC-006**: System achieves greater than 30% fill rate (percentage of submitted orders that get filled) when market-making on moderately liquid markets
- **SC-007**: System maintains 99% uptime over a 30-day period, measured as percentage of time with active WebSocket connection and ability to submit orders
- **SC-008**: Operators can deploy configuration changes (risk limit adjustments, strategy parameter changes, market list updates) by editing config files and restarting the bot in under 5 minutes
- **SC-009**: All critical events (orders, fills, risk violations, circuit breaker activations) are logged and queryable from the database with under 1 second query response time for 30-day historical data
- **SC-010**: Monitoring dashboard displays real-time system status with metrics updated every 5 seconds or less, enabling operators to assess system health at a glance
- **SC-011**: ML-enhanced strategy (when enabled) completes inference within 500 milliseconds and demonstrates statistically significant improvement in simulated PnL over baseline strategy across 14 days of paper trading

### Assumptions

- Polymarket API provides reliable WebSocket and REST endpoints with documented rate limits and message formats
- Operators have basic technical knowledge to configure TOML files, manage environment variables, and interpret log files
- Trading will initially focus on relatively liquid markets with consistent orderbook depth to ensure reasonable fill rates
- System will run on infrastructure with stable network connectivity and sufficient resources (512MB RAM, modern CPU)
- Operators will follow mandatory 30-day paper trading validation before transitioning to live trading with real capital
- Database (PostgreSQL with TimescaleDB) will be properly maintained with appropriate retention policies to manage storage growth from time-series data
- ML model training will be performed offline and models deployed as separate services accessible via HTTP endpoints
- Risk parameters will be set conservatively at initial deployment ($50-100 max capital at risk) and only increased after proven stable operation
