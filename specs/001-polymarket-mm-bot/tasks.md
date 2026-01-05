# Tasks: Polymarket Market-Making Bot

**Input**: Design documents from `/specs/001-polymarket-mm-bot/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Create project directory structure per implementation plan at C:/code/spreadable
- [ ] T002 Initialize Rust project with Cargo.toml dependencies (tokio, tokio-tungstenite, serde, serde_json, sqlx, rust_decimal, config, tracing, prometheus, reqwest, anyhow)
- [ ] T003 [P] Configure clippy and rustfmt in .cargo/config.toml with strict lints enabled
- [ ] T004 [P] Create .env.example file with required environment variables (POLYMARKET_API_KEY, POLYMARKET_API_SECRET, DATABASE_URL)
- [ ] T005 [P] Create docker-compose.yml with PostgreSQL TimescaleDB service, Prometheus, and Grafana
- [ ] T006 [P] Create Dockerfile with multi-stage build for production binary
- [ ] T007 [P] Create .gitignore file excluding .env, target/, and database volumes
- [ ] T008 Create config/development.toml with paper trading enabled and verbose logging
- [ ] T009 [P] Create config/staging.toml with paper trading and conservative limits
- [ ] T010 [P] Create config/production.toml template with live trading configuration
- [ ] T011 Create README.md with setup instructions and quickstart reference

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T012 Create sql/schema.sql with PostgreSQL and TimescaleDB schema (orderbook_snapshots, orders, position_snapshots, strategy_decisions tables)
- [ ] T013 Create sql/migrations/ directory structure for versioned schema changes
- [ ] T014 Implement src/config.rs with TOML parsing, validation, and environment variable substitution
- [ ] T015 [P] Create shared error types in src/error.rs with proper error propagation using anyhow::Result
- [ ] T016 [P] Create shared data models in src/types.rs (OrderBook, Order, Position, Side, OrderStatus enums)
- [ ] T017 Implement src/database/mod.rs with SQLx connection pool initialization
- [ ] T018 Implement src/database/client.rs with async query methods for all tables
- [ ] T019 [P] Implement src/monitoring/mod.rs with Prometheus metric registry
- [ ] T020 [P] Implement src/monitoring/metrics.rs with all required metrics (websocket_connected, orders_placed_total, order_placement_latency_ms, current_position, daily_pnl_usd, circuit_breaker_triggered, fill_rate)
- [ ] T021 Setup structured logging with tracing subscriber in src/main.rs
- [ ] T022 Implement health check endpoint handler at /health in src/main.rs
- [ ] T023 Create integration test setup with testcontainers for PostgreSQL in tests/integration/setup.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Safe Market Data Collection (Priority: P1) 🎯 MVP

**Goal**: Safely collect real-time market data from Polymarket without risking capital, building confidence in accurate market tracking

**Independent Test**: Deploy in paper trading mode, verify 24 hours of continuous orderbook data collection with zero data loss, confirm all events logged to database

### Tests for User Story 1 (TDD - Write These FIRST) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T024 [P] [US1] Contract test for Polymarket WebSocket orderbook message parsing in tests/contract/test_polymarket_api.rs
- [ ] T025 [P] [US1] Contract test for Polymarket REST API orderbook snapshot format in tests/contract/test_polymarket_api.rs
- [ ] T026 [P] [US1] Integration test for WebSocket connection lifecycle (connect, subscribe, receive updates) in tests/integration/test_websocket_lifecycle.rs
- [ ] T027 [P] [US1] Integration test for WebSocket reconnection with exponential backoff in tests/integration/test_websocket_lifecycle.rs
- [ ] T028 [P] [US1] Integration test for orderbook snapshot persistence to database in tests/integration/test_database_persistence.rs
- [ ] T029 [P] [US1] Unit test for orderbook mid-price and spread calculations in tests/unit/test_orderbook.rs
- [ ] T030 [P] [US1] Unit test for orderbook bid/ask level updates and sorting in tests/unit/test_orderbook.rs

### Implementation for User Story 1

- [ ] T031 [P] [US1] Create WebSocket message types in src/market_data/mod.rs (OrderBookMessage, TradeMessage, SubscriptionMessage)
- [ ] T032 [US1] Implement WebSocket client with connection management in src/market_data/websocket_client.rs
- [ ] T033 [US1] Implement WebSocket reconnection logic with exponential backoff (max 60 second delay) in src/market_data/websocket_client.rs
- [ ] T034 [US1] Implement market subscription protocol in src/market_data/websocket_client.rs
- [ ] T035 [US1] Create OrderBook struct with bid/ask levels in src/market_data/orderbook_manager.rs
- [ ] T036 [US1] Implement orderbook update processing (<10ms target) in src/market_data/orderbook_manager.rs
- [ ] T037 [US1] Implement mid-price and spread calculation methods in src/market_data/orderbook_manager.rs
- [ ] T038 [US1] Implement orderbook imbalance calculation in src/market_data/orderbook_manager.rs
- [ ] T039 [US1] Add database persistence for orderbook snapshots (async batched writes) in src/database/client.rs
- [ ] T040 [US1] Add metrics tracking for WebSocket connection status in src/market_data/websocket_client.rs
- [ ] T041 [US1] Add metrics tracking for orderbook update latency in src/market_data/orderbook_manager.rs
- [ ] T042 [US1] Add structured logging for connection events (connect, disconnect, reconnect) in src/market_data/websocket_client.rs
- [ ] T043 [US1] Add structured logging for orderbook updates with market_id correlation in src/market_data/orderbook_manager.rs
- [ ] T044 [US1] Create monitoring/prometheus.yml with scrape configuration
- [ ] T045 [US1] Create monitoring/grafana-dashboard.json with connection status and latency visualizations
- [ ] T046 [US1] Integrate WebSocket client into main event loop in src/main.rs
- [ ] T047 [US1] Add paper trading mode flag validation in src/config.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - system can collect and persist market data for 24+ hours

---

## Phase 4: User Story 2 - Controlled Risk-Limited Trading (Priority: P2)

**Goal**: Execute automated market-making trades on single market with strict position and loss limits ensuring bot cannot exceed risk thresholds

**Independent Test**: Configure bot with conservative limits ($100 max position, $50 daily loss), enable live trading on one low-volatility market with $50-100 capital, verify all orders stay within limits over 48 hours

### Tests for User Story 2 (TDD - Write These FIRST) ⚠️

- [ ] T048 [P] [US2] Contract test for Polymarket order placement API request format in tests/contract/test_polymarket_api.rs
- [ ] T049 [P] [US2] Contract test for Polymarket order cancellation API request format in tests/contract/test_polymarket_api.rs
- [ ] T050 [P] [US2] Integration test for complete order flow (submit → open → filled → position update) in tests/integration/test_order_flow.rs
- [ ] T051 [P] [US2] Integration test for circuit breaker triggering on daily loss limit in tests/integration/test_circuit_breaker.rs
- [ ] T052 [P] [US2] Integration test for circuit breaker cancelling all open orders in tests/integration/test_circuit_breaker.rs
- [ ] T053 [P] [US2] Unit test for position size limit validation in tests/unit/test_risk.rs
- [ ] T054 [P] [US2] Unit test for daily loss limit validation in tests/unit/test_risk.rs
- [ ] T055 [P] [US2] Unit test for max order size validation in tests/unit/test_risk.rs
- [ ] T056 [P] [US2] Unit test for total exposure limit validation in tests/unit/test_risk.rs
- [ ] T057 [P] [US2] Unit test for position PnL calculations (realized + unrealized) in tests/unit/test_position.rs
- [ ] T058 [P] [US2] Unit test for average entry price calculation on fills in tests/unit/test_position.rs
- [ ] T059 [P] [US2] Unit test for token bucket rate limiter (5 orders/sec) in tests/unit/test_rate_limiter.rs

### Implementation for User Story 2

- [ ] T060 [P] [US2] Create Order and NewOrder types in src/types.rs with all required fields
- [ ] T061 [P] [US2] Create Position struct with PnL tracking fields in src/types.rs
- [ ] T062 [P] [US2] Create RiskLimit configuration struct in src/config.rs
- [ ] T063 [US2] Implement RiskManager with limit validation methods in src/risk/risk_manager.rs
- [ ] T064 [US2] Implement order validation against position limits in src/risk/risk_manager.rs
- [ ] T065 [US2] Implement order validation against daily loss limits in src/risk/risk_manager.rs
- [ ] T066 [US2] Implement order validation against max order size in src/risk/risk_manager.rs
- [ ] T067 [US2] Implement PositionTracker with real-time position updates in src/risk/position_tracker.rs
- [ ] T068 [US2] Implement realized PnL calculation on order fills in src/risk/position_tracker.rs
- [ ] T069 [US2] Implement unrealized PnL calculation from current market price in src/risk/position_tracker.rs
- [ ] T070 [US2] Implement average entry price tracking in src/risk/position_tracker.rs
- [ ] T071 [US2] Implement CircuitBreaker with daily loss threshold monitoring in src/risk/circuit_breaker.rs
- [ ] T072 [US2] Implement CircuitBreaker with loss rate threshold (5% in 15min) in src/risk/circuit_breaker.rs
- [ ] T073 [US2] Implement emergency order cancellation in CircuitBreaker in src/risk/circuit_breaker.rs
- [ ] T074 [US2] Create RateLimiter with token bucket algorithm (5 tokens/sec) in src/oms/rate_limiter.rs
- [ ] T075 [US2] Implement OrderManager with order submission queue in src/oms/order_manager.rs
- [ ] T076 [US2] Implement Polymarket REST API client for order placement in src/oms/order_manager.rs
- [ ] T077 [US2] Implement Polymarket REST API client for order cancellation in src/oms/order_manager.rs
- [ ] T078 [US2] Implement order lifecycle state tracking (pending → open → filled) in src/oms/order_manager.rs
- [ ] T079 [US2] Implement order reconciliation with exchange state in src/oms/order_manager.rs
- [ ] T080 [US2] Add database persistence for orders table in src/database/client.rs
- [ ] T081 [US2] Add database persistence for position_snapshots table in src/database/client.rs
- [ ] T082 [US2] Create BasicMM strategy implementation in src/strategy/basic_mm.rs
- [ ] T083 [US2] Implement fixed spread calculation around mid-price in src/strategy/basic_mm.rs
- [ ] T084 [US2] Implement inventory skew adjustment based on position in src/strategy/basic_mm.rs
- [ ] T085 [US2] Implement quote size calculation with risk limits in src/strategy/basic_mm.rs
- [ ] T086 [US2] Implement Strategy trait definition in src/strategy/mod.rs
- [ ] T087 [US2] Add metrics for orders_placed_total counter in src/oms/order_manager.rs
- [ ] T088 [US2] Add metrics for order_placement_latency_ms histogram in src/oms/order_manager.rs
- [ ] T089 [US2] Add metrics for current_position gauge per market in src/risk/position_tracker.rs
- [ ] T090 [US2] Add metrics for daily_pnl_usd gauge in src/risk/position_tracker.rs
- [ ] T091 [US2] Add metrics for circuit_breaker_triggered counter in src/risk/circuit_breaker.rs
- [ ] T092 [US2] Add metrics for fill_rate gauge in src/oms/order_manager.rs
- [ ] T093 [US2] Add structured logging for order submissions with order_id correlation in src/oms/order_manager.rs
- [ ] T094 [US2] Add structured logging for order fills with PnL impact in src/risk/position_tracker.rs
- [ ] T095 [US2] Add structured logging for risk violations with rejection reasons in src/risk/risk_manager.rs
- [ ] T096 [US2] Add structured logging for circuit breaker triggers in src/risk/circuit_breaker.rs
- [ ] T097 [US2] Integrate RiskManager into order submission flow in src/main.rs
- [ ] T098 [US2] Integrate OrderManager with rate limiting into main event loop in src/main.rs
- [ ] T099 [US2] Integrate CircuitBreaker monitoring into main event loop in src/main.rs
- [ ] T100 [US2] Update Grafana dashboard with trading metrics (orders, fills, PnL, positions) in monitoring/grafana-dashboard.json
- [ ] T101 [US2] Create monitoring/alerts.yml with critical alerts (circuit breaker, daily loss limit)

**Checkpoint**: At this point, User Story 2 should be fully functional - system can safely trade on single market with enforced risk limits

---

## Phase 5: User Story 3 - Multi-Market Operations (Priority: P3)

**Goal**: Run bot across 10-20 markets simultaneously, diversifying trading activity while maintaining independent risk controls per market

**Independent Test**: Configure 10 markets with individual position limits, run bot for 72 hours, verify each market operates independently (limits enforced per-market, fills in one market don't affect others)

### Tests for User Story 3 (TDD - Write These FIRST) ⚠️

- [ ] T102 [P] [US3] Integration test for multi-market WebSocket subscriptions in tests/integration/test_websocket_lifecycle.rs
- [ ] T103 [P] [US3] Integration test for independent per-market position tracking in tests/integration/test_order_flow.rs
- [ ] T104 [P] [US3] Integration test for total exposure limit across markets in tests/integration/test_order_flow.rs
- [ ] T105 [P] [US3] Unit test for aggregated position exposure calculation in tests/unit/test_risk.rs
- [ ] T106 [P] [US3] Performance test for 50 markets at 100 updates/sec each (5000 total updates/sec) in tests/integration/test_performance.rs

### Implementation for User Story 3

- [ ] T107 [P] [US3] Extend WebSocket client to support multiple market subscriptions in src/market_data/websocket_client.rs
- [ ] T108 [US3] Implement concurrent orderbook manager for multiple markets in src/market_data/orderbook_manager.rs
- [ ] T109 [US3] Add per-market orderbook state isolation using HashMap<MarketId, OrderBook> in src/market_data/orderbook_manager.rs
- [ ] T110 [US3] Extend PositionTracker to track positions per market in src/risk/position_tracker.rs
- [ ] T111 [US3] Implement total exposure aggregation across all markets in src/risk/position_tracker.rs
- [ ] T112 [US3] Extend RiskManager to validate total exposure limit in src/risk/risk_manager.rs
- [ ] T113 [US3] Optimize orderbook processing with parallel tokio tasks per market in src/market_data/orderbook_manager.rs
- [ ] T114 [US3] Optimize order submission with concurrent processing per market in src/oms/order_manager.rs
- [ ] T115 [US3] Add per-market metrics labels to all gauges/counters in src/monitoring/metrics.rs
- [ ] T116 [US3] Update Grafana dashboard with multi-market consolidated view in monitoring/grafana-dashboard.json
- [ ] T117 [US3] Add market selection configuration to config files (enabled_markets list) in config/production.toml
- [ ] T118 [US3] Implement graceful shutdown with all markets cleanup in src/main.rs

**Checkpoint**: At this point, User Story 3 should be fully functional - system can safely operate across 10-20 markets with independent controls

---

## Phase 6: User Story 4 - ML-Enhanced Strategy Optimization (Priority: P4)

**Goal**: Leverage ML predictions to improve quote pricing and sizing decisions, increasing profitability vs rule-based strategies while maintaining fallback

**Independent Test**: Train model on 30 days of historical data, run ML-enhanced strategy in paper trading for 14 days, compare fill rates and simulated PnL against baseline strategy

### Tests for User Story 4 (TDD - Write These FIRST) ⚠️

- [ ] T119 [P] [US4] Contract test for Ollama inference API request/response format in tests/contract/test_ollama_api.rs
- [ ] T120 [P] [US4] Integration test for ML-enhanced strategy with successful inference in tests/integration/test_ml_strategy.rs
- [ ] T121 [P] [US4] Integration test for ML strategy fallback to baseline when Ollama unavailable in tests/integration/test_ml_strategy.rs
- [ ] T122 [P] [US4] Unit test for market feature extraction (price, spread, volatility, imbalance) in tests/unit/test_features.rs
- [ ] T123 [P] [US4] Unit test for ML prediction timeout handling (500ms limit) in tests/unit/test_ollama_client.rs

### Implementation for User Story 4

- [ ] T124 [P] [US4] Create MarketFeatures struct with all feature fields in src/types.rs
- [ ] T125 [P] [US4] Create MLPrediction struct for quote predictions in src/types.rs
- [ ] T126 [US4] Implement feature extraction from orderbook in src/strategy/base_strategy.rs
- [ ] T127 [US4] Implement rolling volatility calculation (1min, 5min, 15min windows) in src/strategy/base_strategy.rs
- [ ] T128 [US4] Implement rolling volume calculation in src/strategy/base_strategy.rs
- [ ] T129 [US4] Implement OllamaClient HTTP client in src/ml/ollama_client.rs
- [ ] T130 [US4] Implement ML prediction request formatting in src/ml/ollama_client.rs
- [ ] T131 [US4] Implement ML prediction response parsing in src/ml/ollama_client.rs
- [ ] T132 [US4] Implement prediction timeout handling (500ms max) in src/ml/ollama_client.rs
- [ ] T133 [US4] Create MLEnhancedStrategy implementation in src/strategy/ml_enhanced.rs
- [ ] T134 [US4] Implement fallback to BasicMM strategy on ML failure in src/strategy/ml_enhanced.rs
- [ ] T135 [US4] Implement confidence-based blending of ML and baseline quotes in src/strategy/ml_enhanced.rs
- [ ] T136 [US4] Add database persistence for strategy_decisions table in src/database/client.rs
- [ ] T137 [US4] Add strategy decision logging (features, quotes, outcomes) in src/strategy/ml_enhanced.rs
- [ ] T138 [US4] Create Python feature engineering script in scripts/features.py
- [ ] T139 [US4] Implement database query for historical orderbook data in scripts/features.py
- [ ] T140 [US4] Implement feature calculation pipeline in scripts/features.py
- [ ] T141 [US4] Implement feature export to CSV/parquet in scripts/features.py
- [ ] T142 [US4] Create Python model training script in scripts/train_model.py
- [ ] T143 [US4] Implement Ollama model fine-tuning logic in scripts/train_model.py
- [ ] T144 [US4] Create backtesting framework in scripts/backtest.py
- [ ] T145 [US4] Implement strategy comparison metrics (Sharpe ratio, max drawdown, fill rate) in scripts/backtest.py
- [ ] T146 [US4] Add ML configuration section to config files (ollama_endpoint, model_name, inference_enabled) in config/production.toml
- [ ] T147 [US4] Add metrics for ML inference latency in src/ml/ollama_client.rs
- [ ] T148 [US4] Add metrics for ML fallback counter in src/strategy/ml_enhanced.rs
- [ ] T149 [US4] Update README with ML setup instructions in README.md

**Checkpoint**: At this point, User Story 4 should be fully functional - ML strategy can enhance baseline performance with proven fallback behavior

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and production readiness

- [ ] T150 [P] Add benchmark tests for orderbook update processing (<10ms target) using criterion in benches/orderbook.rs
- [ ] T151 [P] Add benchmark tests for order placement latency (<100ms target) using criterion in benches/order_flow.rs
- [ ] T152 [P] Add comprehensive unit tests for all remaining utility functions (>80% coverage target) in tests/unit/
- [ ] T153 [P] Add integration test for 72-hour continuous operation in tests/integration/test_endurance.rs
- [ ] T154 [P] Add integration test for database write throughput under peak load in tests/integration/test_performance.rs
- [ ] T155 [P] Create kill switch implementation for emergency order cancellation in src/main.rs
- [ ] T156 [P] Add CLI flag for manual circuit breaker trigger in src/main.rs
- [ ] T157 [P] Add configuration validation with helpful error messages in src/config.rs
- [ ] T158 [P] Add HMAC-SHA256 authentication for Polymarket API in src/oms/order_manager.rs
- [ ] T159 [P] Implement comprehensive error messages with remediation suggestions in src/error.rs
- [ ] T160 [P] Add cargo audit to CI pipeline in .github/workflows/ci.yml
- [ ] T161 [P] Add cargo deny for license and security checks in .github/workflows/ci.yml
- [ ] T162 [P] Create GitHub Actions workflow for test suite in .github/workflows/ci.yml
- [ ] T163 [P] Create GitHub Actions workflow for Docker image build in .github/workflows/docker.yml
- [ ] T164 [P] Add deployment documentation in docs/deployment.md
- [ ] T165 [P] Add troubleshooting guide in docs/troubleshooting.md
- [ ] T166 [P] Add monitoring runbook in docs/runbook.md
- [ ] T167 [P] Update README with complete quickstart instructions
- [ ] T168 [P] Create example .env file with all required variables in .env.example
- [ ] T169 [P] Add config validation tests for all TOML files in tests/unit/test_config.rs
- [ ] T170 [P] Add memory leak detection test (72-hour run with memory profiling) in tests/integration/test_endurance.rs
- [ ] T171 [P] Optimize database queries with proper indexes in sql/schema.sql
- [ ] T172 [P] Implement database connection retry logic with exponential backoff in src/database/client.rs
- [ ] T173 [P] Add TimescaleDB retention policy configuration in sql/schema.sql
- [ ] T174 [P] Add security audit for secrets handling in code review checklist
- [ ] T175 Run full test suite and verify >80% unit test coverage
- [ ] T176 Run integration test suite with testcontainers and verify all pass
- [ ] T177 Run performance benchmarks and verify <100ms p95 order placement, <10ms p99 orderbook processing
- [ ] T178 Deploy to staging environment and complete 30-day paper trading validation
- [ ] T179 Review all constitution compliance requirements and document any deviations
- [ ] T180 Run quickstart.md validation to ensure new users can set up system successfully

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phases 3-6)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed) or sequentially by priority (P1 → P2 → P3 → P4)
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Requires US1 for market data but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Builds on US1 and US2 but independently testable
- **User Story 4 (P4)**: Can start after Foundational (Phase 2) - Requires US2 for strategy framework but independently testable

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD)
- Contract tests before integration tests
- Unit tests in parallel with implementation
- Data types before services
- Services before integration into main event loop
- Core implementation before metrics and logging enhancements
- Story complete and independently tested before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- All tests for a user story marked [P] can run in parallel (write them all first)
- Implementation tasks marked [P] within a story can run in parallel
- Once Foundational phase completes, different user stories can be worked on in parallel by different team members
- All Polish tasks marked [P] can run in parallel

---

## Implementation Strategy

### MVP First (User Story 1 + User Story 2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Data Collection)
4. **STOP and VALIDATE**: Test 24-hour continuous data collection
5. Complete Phase 4: User Story 2 (Risk-Limited Trading)
6. **STOP and VALIDATE**: Test 48-hour paper trading on single market
7. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Validate 24-hour data collection (MVP!)
3. Add User Story 2 → Test independently → Validate 48-hour paper trading
4. Add User Story 3 → Test independently → Validate 72-hour multi-market operation
5. Add User Story 4 → Test independently → Validate 14-day ML strategy comparison
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Data Collection)
   - Developer B: User Story 2 (Trading Engine) - can proceed in parallel with shared types from Foundational
   - Developer C: User Story 3 (Multi-Market) - starts after US1 and US2 complete
   - Developer D: User Story 4 (ML Enhancement) - can start in parallel with US3
3. Stories complete and integrate independently

---

## Testing Requirements Summary

### Unit Tests (>80% Coverage Required)

**Critical Paths Requiring TDD**:
- Position PnL calculations (realized + unrealized)
- Risk limit validation (position, loss, order size, exposure)
- Circuit breaker trigger logic
- Rate limiter token bucket behavior
- Orderbook calculations (mid-price, spread, imbalance)
- Average entry price tracking

**Coverage Target**: >80% line coverage for all business logic modules (strategy/, risk/, oms/)

### Integration Tests (All External Dependencies)

**Required Integration Test Coverage**:
- WebSocket connection lifecycle and reconnection
- Order placement, cancellation, and reconciliation with exchange
- Database persistence and TimescaleDB operations
- Circuit breaker triggering and order cancellation
- Multi-market concurrent operations
- ML inference with fallback behavior
- 72-hour endurance test with no memory leaks

### Contract Tests (API Boundaries)

**Required Contract Test Coverage**:
- Polymarket WebSocket message formats (orderbook, trades)
- Polymarket REST API request/response formats (orders, cancellations)
- Ollama inference API format
- Internal event formats between modules

### Performance Tests

**Required Performance Validation**:
- Orderbook update processing: <10ms at p99
- Order placement latency: <100ms at p95
- Database write throughput: 5000 updates/second sustained
- Memory footprint: <512MB for 50 concurrent markets
- 72-hour continuous operation without degradation

### Paper Trading Validation

**Mandatory 30-Day Period**:
- All strategies MUST complete 30 days of paper trading before live deployment
- Daily monitoring and metric collection required
- Success criteria: >99% uptime, no circuit breaker false positives, accurate position tracking

---

## Constitution Compliance Checklist

- ✅ All critical paths (risk validation, PnL calculations) using TDD approach
- ✅ Unit test coverage >80% for business logic modules
- ✅ Integration tests for all external dependencies (WebSocket, REST API, database)
- ✅ Contract tests for all API boundaries
- ✅ Performance tests for latency-critical paths
- ✅ 30-day paper trading validation before live deployment
- ✅ Strict type safety with Result<T, E> and no unwrap() in production
- ✅ Comprehensive error handling with context and remediation
- ✅ Structured logging with correlation IDs
- ✅ All required Prometheus metrics implemented
- ✅ Health check endpoint for observability
- ✅ Secrets management via environment variables
- ✅ Input validation for all external data
- ✅ Dependency auditing in CI pipeline
- ✅ Rate limiting to prevent API abuse
- ✅ Documentation for all public APIs and complex logic

---

## Notes

- [P] tasks = different files, no dependencies, can run in parallel
- [Story] label maps task to specific user story for traceability (US1, US2, US3, US4)
- Each user story should be independently completable and testable
- **CRITICAL**: Write all tests FIRST for each story, verify they FAIL, then implement until tests pass
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently before proceeding
- Paper trading mode MUST be enabled for all initial testing and validation
- Conservative risk limits ($50-100 max capital) required for first live trading attempts
- All constitution requirements MUST be met before production deployment
