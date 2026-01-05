# Implementation Plan: Polymarket Market-Making Bot

**Branch**: `001-polymarket-mm-bot` | **Date**: 2026-01-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-polymarket-mm-bot/spec.md`

## Summary

Build an automated market-making bot for Polymarket prediction markets that safely collects real-time orderbook data, executes trades within strict risk limits, and scales to 10-50 concurrent markets. The system prioritizes safety through paper trading validation, comprehensive risk controls (position limits, circuit breakers), and progressive rollout from single-market operation to multi-market production deployment. Technical approach emphasizes low-latency async processing (<100ms order placement), reliable WebSocket connectivity with automatic reconnection, and complete observability through structured logging and Prometheus metrics.

## Technical Context

**Language/Version**: Rust 1.75+ (stable channel, 2021 edition)
**Primary Dependencies**: tokio 1.35 (async runtime), tokio-tungstenite 0.21 (WebSocket), sqlx 0.7 (PostgreSQL client), serde/serde_json (serialization), rust_decimal 1.33 (financial precision), prometheus 0.13 (metrics), tracing 0.1 (structured logging), config 0.13 (TOML parsing), reqwest 0.11 (HTTP client)
**Storage**: PostgreSQL 15+ with TimescaleDB extension for time-series data (orderbook snapshots, positions, strategy decisions)
**Testing**: cargo test (unit), integration tests with testcontainers (PostgreSQL), contract tests for Polymarket API mocks, criterion for benchmarking latency-critical paths
**Target Platform**: Linux x86_64 server (Docker containers on Ubuntu 22.04+), amd64 architecture
**Project Type**: Single project (CLI application with background services)
**Performance Goals**: <100ms p95 order placement latency, <10ms p99 orderbook update processing, support 10-50 concurrent markets at 100 updates/sec/market, <512MB RAM footprint
**Constraints**: Must respect Polymarket rate limits (5 orders/second), operate within exchange-imposed latency requirements, maintain 99% uptime over 30-day periods
**Scale/Scope**: Handle 10-50 concurrent markets, process 5000+ orderbook updates/second aggregate, store 30+ days of historical data for ML training (~10GB), support 3 strategy types (basic, adaptive, ML-enhanced)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Code Quality Standards
- ✅ **Type Safety**: Rust's ownership system enforces strict type safety; will use `Result<T, E>` throughout with `?` operator, zero `unwrap()` in production paths
- ✅ **Unit Tests**: All business logic modules (strategy calculations, position tracking, risk validation) will have >80% coverage
- ✅ **Integration Tests**: WebSocket lifecycle, order submission/cancellation, database persistence, circuit breaker triggers all covered
- ✅ **Contract Tests**: Mock Polymarket API responses, validate orderbook parsing, order submission formats
- ✅ **Test-First**: Critical paths (risk limit enforcement, PnL calculations) will follow TDD with tests written before implementation
- ✅ **Paper Trading**: Mandatory 30-day validation period built into deployment workflow
- ✅ **Error Handling**: All external interactions (WebSocket, REST API, database) will use proper error types with context, exponential backoff for retries
- ✅ **Documentation**: Public trait methods and complex algorithms (PnL calculations, risk checks) will have rustdoc comments
- ✅ **No Warnings**: CI pipeline will enforce `#![deny(warnings)]` in production builds
- ✅ **Configuration Clarity**: TOML schema with clear field names, inline comments, validation on startup with helpful error messages
- ✅ **Logging Standards**: Structured logging with `tracing` crate, correlation IDs (market_id, order_id), ERROR/WARN/INFO levels
- ✅ **Error Messages**: User-facing errors include problem description + suggested remediation (e.g., "WebSocket disconnected → check network connectivity")
- ✅ **Monitoring Dashboards**: Prometheus metrics exposed on `/metrics` endpoint, Grafana dashboards provided in monitoring/ directory

### Performance Requirements
- ✅ **Order Placement Latency**: Architecture uses async pipelines to achieve <100ms p95 end-to-end
- ✅ **Market Data Processing**: Lock-free orderbook updates with channels, <10ms processing target
- ✅ **Database Writes**: Batch writes with tokio::spawn for async persistence, non-blocking trading loop
- ✅ **Memory Efficiency**: Bounded channels, periodic cleanup of filled orders, target <512MB for 50 markets
- ✅ **Rate Limiting**: Token bucket rate limiter enforcing 5 orders/second with backpressure

### Observability & Monitoring
- ✅ **Structured Logging**: All events include timestamps (UTC), correlation IDs, structured fields (JSON where applicable)
- ✅ **Metrics Instrumentation**: All required metrics implemented (websocket_connected, orders_placed_total, order_placement_latency_ms, current_position, daily_pnl_usd, circuit_breaker_triggered, fill_rate)
- ✅ **Health Checks**: `/health` endpoint reports connection status, database ping, circuit breaker state
- ✅ **Audit Trail**: strategy_decisions table stores all trading decisions with features, outcomes for ML training

### Security Standards
- ✅ **Secrets Management**: API keys loaded from environment variables only, never hardcoded
- ✅ **Input Validation**: All WebSocket messages validated with serde, malformed data rejected with logging
- ✅ **Dependency Auditing**: CI includes `cargo audit` check, vulnerabilities block merge
- ✅ **Rate Limiting**: Client-side rate limiting prevents accidental API abuse

### Gate Evaluation
**STATUS**: ✅ PASSED - All constitution requirements satisfied. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/001-polymarket-mm-bot/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   ├── polymarket-api.md
│   └── internal-events.md
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs                  # CLI entry point, configuration loading, service orchestration
├── config.rs                # TOML config parsing, validation, defaults
├── market_data/
│   ├── mod.rs              # Public market data module interface
│   ├── websocket_client.rs # Polymarket WebSocket connection, reconnection logic
│   └── orderbook_manager.rs # Maintain local orderbook state, mid-price calculation
├── strategy/
│   ├── mod.rs              # Strategy trait definition
│   ├── base_strategy.rs    # Common quote calculation utilities
│   ├── basic_mm.rs         # Fixed spread strategy with inventory skew
│   ├── adaptive_spread.rs  # Volatility-based dynamic spreads
│   └── ml_enhanced.rs      # Ollama integration with fallback to baseline
├── oms/
│   ├── mod.rs              # Order management public interface
│   ├── order_manager.rs    # Order submission, lifecycle tracking, reconciliation
│   └── rate_limiter.rs     # Token bucket rate limiting (5 orders/sec)
├── risk/
│   ├── mod.rs              # Risk management module interface
│   ├── risk_manager.rs     # Position/exposure/loss limit validation
│   ├── position_tracker.rs # Real-time position + PnL calculations
│   └── circuit_breaker.rs  # Emergency halt on loss thresholds
├── database/
│   ├── mod.rs              # Database client interface
│   └── client.rs           # SQLx connection pool, query builders
├── ml/
│   ├── mod.rs              # ML module interface
│   └── ollama_client.rs    # HTTP client for Ollama predictions
└── monitoring/
    ├── mod.rs              # Monitoring module interface
    └── metrics.rs          # Prometheus metric definitions and registration

tests/
├── contract/
│   ├── test_polymarket_api.rs      # Mock Polymarket responses, validate parsing
│   └── test_internal_events.rs     # Internal event format contracts
├── integration/
│   ├── test_websocket_lifecycle.rs # Connect, disconnect, reconnect scenarios
│   ├── test_order_flow.rs          # End-to-end order placement → fill
│   ├── test_database_persistence.rs # TimescaleDB writes, queries
│   └── test_circuit_breaker.rs     # Trigger conditions, cancellation behavior
└── unit/
    ├── test_orderbook.rs           # Mid-price, spread calculations
    ├── test_position.rs            # PnL tracking, average entry price
    ├── test_risk.rs                # Limit validation logic
    └── test_rate_limiter.rs        # Token bucket behavior

config/
├── development.toml        # Dev config (paper trading enabled, verbose logging)
├── staging.toml            # Staging config (paper trading, conservative limits)
└── production.toml         # Prod config template (live trading, strict limits)

sql/
├── schema.sql              # PostgreSQL + TimescaleDB schema initialization
└── migrations/             # Versioned schema changes

scripts/
├── features.py             # Feature engineering for ML training data
├── train_model.py          # Ollama model training script
└── backtest.py             # Strategy backtesting on historical data

monitoring/
├── prometheus.yml          # Prometheus scrape config
├── alerts.yml              # Alertmanager rules (circuit breaker, loss limits)
└── grafana-dashboard.json  # Pre-built dashboard for system health

docker-compose.yml          # PostgreSQL, Prometheus, Grafana services
Dockerfile                  # Multi-stage build for spreadable binary
Cargo.toml                  # Rust dependencies and workspace config
Cargo.lock                  # Locked dependency versions
.env.example                # Template for environment variables
README.md                   # Setup instructions, quickstart reference
```

**Structure Decision**: Selected Option 1 (Single Project) because this is a standalone CLI application with background services. All components (market data, trading engine, database client) run within a single Rust binary. The modular `src/` structure separates concerns (market_data, strategy, oms, risk) while maintaining tight integration for low-latency operation. Tests are organized by type (unit, integration, contract) to support TDD workflow and clear separation of test scopes.

## Complexity Tracking

No constitution violations - all checks passed. No complexity justifications required.
