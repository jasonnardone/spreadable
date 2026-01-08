# Spreadable - Polymarket Market-Making Bot

Automated market-making bot for Polymarket prediction markets with comprehensive risk controls and paper trading validation.

## Features

- **Real-time Market Data**: WebSocket-based orderbook streaming with <10ms processing latency
- **Multi-layer Risk Controls**: Position limits, circuit breakers, daily loss limits
- **Paper Trading**: 30+ day validation period before live deployment
- **Multi-market Support**: Scale from single market to 10-50 concurrent markets
- **ML Enhancement**: Optional Ollama integration for strategy optimization
- **Complete Observability**: Prometheus metrics + Grafana dashboards

## Project Status

**Current Phase**: Phase 1 - Setup ✅ COMPLETE

- [x] Project structure created
- [x] Rust toolchain configured (1.75+, strict lints)
- [x] Database schema (PostgreSQL + TimescaleDB)
- [x] Configuration templates
- [x] Docker Compose setup
- [x] Monitoring stack (Prometheus + Grafana)

**Next Phase**: Phase 2 - Foundational Components (shared types, logging, database client)

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Docker and Docker Compose
- Polymarket API credentials
- PostgreSQL 15+ (via Docker)

### Setup

1. **Configure environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your Polymarket API credentials and database password
   ```

2. **Start database**:
   ```bash
   docker-compose up -d postgres
   ```

3. **Build the project**:
   ```bash
   cargo build --release
   ```

4. **Run in paper trading mode**:
   ```bash
   cargo run --release -- --config config/development.toml
   ```

See [quickstart.md](specs/001-polymarket-mm-bot/quickstart.md) for detailed setup guide.

## Architecture

```
src/
├── main.rs                  # CLI entry point
├── config.rs                # TOML configuration
├── market_data/             # WebSocket client, orderbook management
├── strategy/                # Trading strategies (basic_mm, adaptive, ml)
├── oms/                     # Order management system
├── risk/                    # Risk limits, circuit breaker
├── database/                # PostgreSQL + TimescaleDB client
├── ml/                      # Ollama integration
└── monitoring/              # Prometheus metrics, health checks
```

## Configuration

Edit `config/development.toml`:

- **Risk Limits**: Position sizes, exposure limits, daily loss thresholds
- **Strategy**: Select active strategy and parameters
- **Markets**: List of enabled Polymarket market IDs
- **Paper Trading**: MUST be `true` for initial deployment

## Safety

⚠️ **CRITICAL SAFETY REQUIREMENTS**:

1. **Always start with `paper_trading_mode = true`**
2. **Run for 30+ days in paper trading before live deployment**
3. **Start with small position limits ($50-100 total exposure)**
4. **Monitor continuously during first weeks of live trading**
5. **Have kill switch ready to cancel all orders**

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_tests

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check
```

## Monitoring

- **Metrics**: http://localhost:9090/metrics (Prometheus format)
- **Health**: http://localhost:8080/health
- **Grafana**: http://localhost:3000 (admin/admin)

## Documentation

- [Specification](specs/001-polymarket-mm-bot/spec.md) - Feature requirements
- [Implementation Plan](specs/001-polymarket-mm-bot/plan.md) - Technical architecture
- [Quickstart Guide](specs/001-polymarket-mm-bot/quickstart.md) - Operator onboarding
- [Data Model](specs/001-polymarket-mm-bot/data-model.md) - Core entities and schemas
- [API Contracts](specs/001-polymarket-mm-bot/contracts/) - External and internal contracts

## Development

See [CLAUDE.md](CLAUDE.md) for development guidelines and tech stack details.

**Commands**:
```bash
cargo dev-check     # Run clippy with all lints
cargo dev-test      # Run all tests
cargo dev-build     # Build with all features
cargo release       # Release build
```

## License

MIT
