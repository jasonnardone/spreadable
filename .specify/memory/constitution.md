<!--
=================================================================================
SYNC IMPACT REPORT
=================================================================================
Version Change: 1.0.0 (new constitution)
Modified Principles: N/A (initial creation)
Added Sections:
  - I. Code Quality Standards
  - II. Testing Discipline
  - III. User Experience Consistency
  - IV. Performance Requirements
  - V. Observability & Monitoring
  - Security Standards
  - Development Workflow
Removed Sections: N/A
Templates Requiring Updates:
  ✅ plan-template.md - Constitution Check section already references constitution.md
  ✅ spec-template.md - Requirements sections align with functional requirements
  ✅ tasks-template.md - Task categorization supports test-driven and quality gates
Follow-up TODOs: None
=================================================================================
-->

# Spreadable Constitution

## Core Principles

### I. Code Quality Standards

**All code MUST be production-ready and maintainable:**

- **Type Safety**: All Rust code MUST use strict type checking with no `unwrap()` calls in production paths; use proper error handling with `Result<T, E>` and `?` operator
- **Error Handling**: All errors MUST be propagated correctly; external API calls, database operations, and I/O MUST handle failures gracefully with exponential backoff where appropriate
- **Code Reviews**: All changes MUST pass review for correctness, security, and maintainability before merge
- **Documentation**: Public APIs and complex business logic MUST include inline documentation explaining purpose and edge cases
- **No Warnings**: Code MUST compile without warnings in production builds; use `#[allow(...)]` only when justified with comment

**Rationale**: Trading systems handle real money; bugs can cause direct financial loss. Strict quality standards prevent costly errors and ensure system reliability under production loads.

### II. Testing Discipline (NON-NEGOTIABLE)

**Test-driven approach MUST be followed for all critical paths:**

- **Unit Tests Required**: All business logic (strategies, risk calculations, PnL tracking) MUST have unit tests with >80% coverage
- **Integration Tests Required**: All external integrations (WebSocket connections, REST API calls, database operations) MUST have integration tests validating contracts
- **Contract Tests**: All public APIs and data models MUST have contract tests ensuring backward compatibility
- **Test Before Implementation**: For new features, tests MUST be written first, verified to FAIL, then implementation proceeds until tests pass (Red-Green-Refactor)
- **Paper Trading Validation**: All trading strategies MUST run successfully in paper trading mode for minimum 30 days before live deployment

**Focus Areas Requiring Integration Tests**:
- WebSocket connection lifecycle and reconnection logic
- Order placement, cancellation, and state reconciliation with exchange
- Database persistence and TimescaleDB hypertable operations
- Risk manager circuit breaker triggers
- ML model inference integration with fallback behavior

**Rationale**: Automated trading operates 24/7 without human supervision. Comprehensive tests are the only way to verify correctness across all edge cases and prevent silent failures that could drain capital.

### III. User Experience Consistency

**All user-facing interfaces MUST be predictable and reliable:**

- **Configuration Clarity**: All TOML configuration options MUST have clear descriptions, sensible defaults, and validation with helpful error messages
- **Logging Standards**: All significant events (order placement, fills, risk violations, circuit breaker triggers) MUST be logged at appropriate levels (ERROR, WARN, INFO) with structured context
- **Error Messages**: All user-facing errors MUST explain what went wrong and suggest remediation steps
- **API Stability**: Breaking changes to configuration schema or CLI interfaces MUST be avoided; when unavoidable, provide migration path
- **Monitoring Dashboards**: All critical metrics MUST be exposed via Prometheus and visualized in Grafana with meaningful alerts

**Rationale**: Market-making bots run with minimal human intervention. Clear feedback, consistent behavior, and comprehensive monitoring are essential for operators to maintain confidence and troubleshoot issues quickly.

### IV. Performance Requirements

**System MUST meet latency and throughput targets:**

- **Order Placement Latency**: End-to-end time from market data receipt to order submission MUST be <100ms at p95
- **Market Data Processing**: OrderBook updates MUST be processed in <10ms to maintain accurate state
- **Database Writes**: Asynchronous batch writes MUST be used for high-frequency data (orderbook snapshots, trades) to avoid blocking trading loop
- **Memory Efficiency**: System MUST operate within 512MB RAM for typical workloads (10-50 concurrent markets)
- **Rate Limiting**: Order submission MUST respect exchange rate limits (5 orders/second) with proper queuing and backpressure handling

**Performance Testing Required**:
- Load testing with simulated market data at 100 updates/second per market
- Latency profiling of critical path from WebSocket message to order placement
- Memory leak detection during 72-hour continuous runs
- Database write throughput validation under peak load

**Rationale**: Market-making profitability depends on speed. Slow systems miss opportunities and get adversely selected. Meeting performance targets ensures competitiveness and prevents technical failures during high-volatility periods.

### V. Observability & Monitoring

**All system behavior MUST be observable and traceable:**

- **Structured Logging**: All logs MUST include timestamps, correlation IDs (market_id, order_id), and structured context (JSON format where helpful)
- **Metrics Instrumentation**: All critical operations MUST emit Prometheus metrics: connection status, order counts, latency histograms, position snapshots, PnL updates
- **Distributed Tracing**: Long operations (strategy calculation, ML inference) MUST use tracing spans for profiling
- **Health Checks**: System MUST expose `/health` endpoint reporting WebSocket connection status, database connectivity, and circuit breaker state
- **Audit Trail**: All strategy decisions MUST be persisted to `strategy_decisions` table for post-hoc analysis and ML training

**Required Metrics**:
- `websocket_connected` (gauge): Connection status
- `orders_placed_total` (counter): Total orders submitted
- `order_placement_latency_ms` (histogram): Order submission latency
- `current_position` (gauge): Position size per market
- `daily_pnl_usd` (gauge): Realized + unrealized PnL
- `circuit_breaker_triggered` (counter): Emergency stop activations
- `fill_rate` (gauge): Percentage of orders filled

**Rationale**: Observability enables rapid debugging, performance optimization, and compliance verification. Without comprehensive monitoring, diagnosing production issues becomes guesswork and post-incident analysis is impossible.

## Security Standards

**All security-sensitive operations MUST follow best practices:**

- **Secrets Management**: API keys and database passwords MUST be stored in environment variables or secure vaults, NEVER committed to version control
- **Input Validation**: All external inputs (configuration files, WebSocket messages, REST API responses) MUST be validated and sanitized
- **Dependency Auditing**: All dependencies MUST be regularly audited with `cargo audit` and vulnerabilities addressed within 7 days for critical, 30 days for high severity
- **Least Privilege**: Database credentials MUST use read/write-specific roles; no root/superuser access in production
- **Rate Limiting**: All external API clients MUST implement rate limiting and circuit breakers to prevent accidental DoS

**Rationale**: Security breaches can result in loss of funds, API key compromise, or service disruption. Proactive security practices prevent exploits and ensure regulatory compliance.

## Development Workflow

**All development MUST follow this workflow:**

1. **Feature Specification**: New features MUST start with spec document defining user scenarios and functional requirements
2. **Implementation Planning**: Complex features MUST have implementation plan documenting technical approach and architecture decisions
3. **Test-First Development**: Tests MUST be written before implementation for all critical paths
4. **Code Review**: All changes MUST be reviewed for correctness, security, test coverage, and constitution compliance
5. **Pre-Production Validation**: Trading strategies MUST complete 30+ days of paper trading before live deployment
6. **Incremental Rollout**: New strategies MUST start with conservative risk limits ($50-100 max exposure) for first 2 weeks

**Constitution Compliance Review**:
- All PRs MUST verify adherence to testing discipline (tests present and passing)
- All PRs adding complexity MUST justify why simpler alternatives are insufficient
- All PRs affecting performance-critical paths MUST include latency measurements

**Rationale**: Disciplined development workflow prevents rushed changes, ensures quality gates are met, and reduces production incidents.

## Governance

**Constitution Enforcement**:
- This constitution supersedes all other development practices
- All pull requests MUST pass constitution compliance checks during code review
- Violations MUST be documented and justified or corrected before merge
- Any uncertainty about constitution requirements MUST be clarified before proceeding

**Amendment Process**:
- Constitution changes require rationale, impact analysis, and approval
- Version follows semantic versioning: MAJOR (principle removal/redefinition), MINOR (new principle/section), PATCH (clarifications)
- All amendments MUST include migration plan for affected code and documentation updates

**Version**: 1.0.0 | **Ratified**: 2026-01-05 | **Last Amended**: 2026-01-05
