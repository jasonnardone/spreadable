# Quickstart Guide: Polymarket Market-Making Bot

**Feature**: Polymarket Market-Making Bot
**Date**: 2026-01-05
**Audience**: Operators deploying and running the market-making bot
**Time to Complete**: 30-45 minutes

## Overview

This guide walks you through setting up, configuring, and running the Polymarket market-making bot in paper trading mode. By the end, you'll have a running system collecting real-time market data and simulating trades without financial risk.

**Prerequisites**:
- Linux machine (Ubuntu 22.04+ recommended) or macOS
- Docker and Docker Compose installed
- Polymarket API credentials (API key + secret)
- Basic command-line familiarity
- 2GB free disk space
- Stable internet connection

## Part 1: Initial Setup (15 minutes)

### Step 1: Install Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # Verify: should show 1.75+

# Install Docker (if not already installed)
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh
docker --version  # Verify

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
docker-compose --version  # Verify
```

### Step 2: Clone and Build Project

```bash
# Navigate to project directory
cd spreadable

# Verify project structure
ls -la
# Should see: src/, tests/, config/, sql/, Cargo.toml, docker-compose.yml

# Build the project
cargo build --release
# This takes 5-10 minutes on first build

# Verify binary created
ls -lh target/release/spreadable
```

### Step 3: Configure Environment Variables

```bash
# Create environment file from template
cp .env.example .env

# Edit .env with your credentials
nano .env  # or vim, code, etc.
```

**Edit `.env` file**:
```bash
# Polymarket API Credentials
POLYMARKET_API_KEY=your_api_key_here
POLYMARKET_API_SECRET=your_api_secret_here

# Database Password
DB_PASSWORD=choose_secure_password_here

# Optional: Logging level
RUST_LOG=info  # Options: error, warn, info, debug, trace
```

**Security Note**: Never commit `.env` to version control. It's in `.gitignore` by default.

### Step 4: Start Database

```bash
# Start PostgreSQL + TimescaleDB
docker-compose up -d postgres

# Verify database is running
docker-compose ps
# Should show postgres status "Up"

# Wait for database to initialize (10-15 seconds)
sleep 15

# Initialize database schema
docker-compose exec postgres psql -U spreaduser -d spreadable -f /docker-entrypoint-initdb.d/schema.sql

# Verify tables created
docker-compose exec postgres psql -U spreaduser -d spreadable -c "\dt"
# Should list: markets, orderbook_snapshots, orders, positions, etc.
```

**Troubleshooting**:
- If postgres fails to start: Check port 5432 not already in use (`sudo lsof -i :5432`)
- If schema fails: Check `sql/schema.sql` exists and is valid SQL

---

## Part 2: Configuration (10 minutes)

### Step 5: Review and Customize Config

Open `config/development.toml` in your editor:

```bash
nano config/development.toml
```

**Key Settings to Review**:

```toml
[exchange]
# These are loaded from environment variables - don't change
api_key = "${POLYMARKET_API_KEY}"
api_secret = "${POLYMARKET_API_SECRET}"

[strategy]
active = "basic_mm"  # Start with basic market-making
quote_refresh_interval_ms = 2000  # Update quotes every 2 seconds

[strategy.basic_mm]
base_spread_bps = 500  # 5% spread (conservative)
quote_size = 10.0      # $10 per order
max_position = 100.0   # Max $100 position per market
skew_factor = 0.5      # Inventory skew adjustment

[risk]
max_position_per_market = 100.0  # $100 max per market
max_total_exposure = 500.0       # $500 total across all markets
max_order_size = 20.0            # $20 per order
max_daily_loss = 50.0            # $50 daily loss limit
min_spread_bps = 200             # 2% minimum spread
paper_trading_mode = true        # CRITICAL: Must be true initially

[risk.circuit_breaker]
enabled = true
daily_loss_threshold = 50.0  # Halt trading if daily loss hits $50
loss_rate_pct = 0.05         # Halt if 5% loss in 15 minutes

[markets]
enabled = []  # Start empty - add markets after testing

[database]
postgres_url = "${DATABASE_URL}"

[ml]
ollama_endpoint = "http://localhost:11434"
model_name = "market-maker-v1"
inference_enabled = false  # Start with ML disabled
```

**Configuration Rules**:
1. **Always start with `paper_trading_mode = true`**
2. Keep risk limits conservative initially
3. Start with `markets.enabled = []` (no markets)
4. Only modify after understanding each parameter

### Step 6: Validate Configuration

```bash
# Test config parsing
cargo run --release -- --config config/development.toml --validate

# Expected output:
# ✓ Configuration loaded successfully
# ✓ Paper trading mode: ENABLED
# ✓ Risk limits validated
# ✓ No markets configured (add via config)
```

**If validation fails**:
- Check TOML syntax (commas, quotes, brackets)
- Verify environment variables are set (`echo $POLYMARKET_API_KEY`)
- Ensure DATABASE_URL is correct (check .env)

---

## Part 3: First Run (5 minutes)

### Step 7: Start the Bot

```bash
# Run in foreground (see logs in real-time)
cargo run --release -- --config config/development.toml

# Expected startup logs:
# INFO spreadable: Starting Polymarket Market-Making Bot v0.1.0
# INFO spreadable::config: Loaded configuration from config/development.toml
# INFO spreadable::config: Paper trading mode: ENABLED
# INFO spreadable::database: Connected to PostgreSQL
# INFO spreadable::database: Running migrations...
# INFO spreadable::market_data: Initializing WebSocket client
# WARN spreadable::strategy: No markets enabled - bot will idle
# INFO spreadable::monitoring: Metrics server started on 0.0.0.0:9090
# INFO spreadable: All systems initialized successfully
```

**What's Happening**:
- Bot loads config and connects to database
- WebSocket client initializes but doesn't connect (no markets configured)
- Metrics server starts on port 9090
- Strategy engine waits for markets
- Everything runs but does nothing (no markets = no trading)

### Step 8: Verify Health

Open a new terminal and check the health endpoint:

```bash
curl http://localhost:9090/health

# Expected response:
{
  "status": "healthy",
  "timestamp": "2026-01-05T10:30:00Z",
  "checks": {
    "database": "connected",
    "websocket": "idle",
    "circuit_breaker": "standby"
  }
}
```

Check Prometheus metrics:

```bash
curl http://localhost:9090/metrics | grep websocket_connected

# Expected output:
# websocket_connected{market_id=""} 0
# (0 because no markets configured yet)
```

### Step 9: Add Your First Market (Paper Trading)

Stop the bot (`Ctrl+C`), then edit config:

```bash
nano config/development.toml
```

**Add a market** (example - replace with actual Polymarket market ID):
```toml
[markets]
enabled = [
    "0x1234567890abcdef1234567890abcdef12345678"  # Example market ID
]
```

**Finding Market IDs**:
1. Go to https://polymarket.com
2. Browse to a market you want to trade
3. URL will be like: `https://polymarket.com/event/slug?market=0x...`
4. Copy the hex string after `?market=`

**Restart the bot**:
```bash
cargo run --release -- --config config/development.toml

# New logs should appear:
# INFO spreadable::market_data: Subscribing to market 0x1234...
# INFO spreadable::market_data: WebSocket connected
# INFO spreadable::market_data: Received orderbook snapshot for market 0x1234...
# INFO spreadable::strategy: Calculating quotes for market 0x1234...
# INFO spreadable::oms: PAPER TRADE: Would submit BUY order at 0.48, size 10.0
# INFO spreadable::oms: PAPER TRADE: Would submit SELL order at 0.52, size 10.0
```

**Success Indicators**:
- `websocket_connected = 1`
- Orderbook updates logged every few seconds
- "PAPER TRADE" messages show quotes being calculated
- No actual orders submitted (paper trading mode)

---

## Part 4: Monitoring (5 minutes)

### Step 10: Start Monitoring Stack

```bash
# Start Prometheus + Grafana
docker-compose up -d prometheus grafana

# Verify services running
docker-compose ps

# Should show:
# - postgres: Up
# - prometheus: Up
# - grafana: Up
```

### Step 11: Access Grafana Dashboard

1. Open browser to `http://localhost:3000`
2. Login credentials:
   - Username: `admin`
   - Password: `admin` (change on first login)
3. Navigate to "Dashboards" → "Spreadable Market Maker"
4. You should see:
   - WebSocket connection status (green = connected)
   - Orderbook update rate (messages/second)
   - Current positions (should be 0 in paper trading)
   - Paper trade count (increments every quote_refresh_interval)

**Key Metrics to Watch**:
- **websocket_connected**: Must be 1 (green)
- **order_placement_latency_ms**: Should be <100ms
- **current_position**: Should stay 0 in paper trading
- **daily_pnl_usd**: Simulated P&L (not real money)

### Step 12: Check Database

Verify data is being collected:

```bash
# Connect to database
docker-compose exec postgres psql -U spreaduser -d spreadable

# Check orderbook snapshots
SELECT market_id, COUNT(*) as snapshot_count, MAX(timestamp) as latest
FROM orderbook_snapshots
GROUP BY market_id;

# Expected output:
#              market_id              | snapshot_count |          latest
# ------------------------------------+----------------+---------------------------
#  0x1234567890abcdef1234567890abcdef |             42 | 2026-01-05 10:35:00+00

# Check paper trades (simulated orders)
SELECT COUNT(*) FROM orders WHERE status = 'open';

# Check positions (should be empty or flat in paper trading)
SELECT * FROM positions;

# Exit psql
\q
```

---

## Part 5: Operational Tasks (5 minutes)

### Adding More Markets

```bash
# Stop bot (Ctrl+C)
nano config/development.toml

# Add more market IDs to enabled array
[markets]
enabled = [
    "0x1234567890abcdef1234567890abcdef12345678",
    "0xabcdefabcdefabcdefabcdefabcdefabcdef1234",
    "0x9876543210fedcba9876543210fedcba98765432"
]

# Restart
cargo run --release -- --config config/development.toml
```

**Scaling Considerations**:
- Start with 1-3 markets
- Add more gradually (5, 10, 20...)
- Monitor memory usage: `docker stats`
- Each market adds ~5-10MB RAM

### Adjusting Risk Limits

```toml
[risk]
max_position_per_market = 200.0  # Increase from 100 to 200
max_total_exposure = 1000.0      # Increase from 500 to 1000
max_daily_loss = 100.0           # Increase from 50 to 100
```

**Best Practices**:
- Increase limits gradually (2x increments)
- Monitor for 24 hours before next increase
- Never increase while circuit breaker triggered

### Changing Strategy

```toml
[strategy]
active = "adaptive_spread"  # Switch from basic_mm to adaptive

[strategy.adaptive_spread]
base_spread_bps = 300
volatility_multiplier = 1.5
max_spread_bps = 1000
```

**Strategy Options**:
- `basic_mm`: Fixed spread, simple inventory skew
- `adaptive_spread`: Dynamic spreads based on volatility
- `ml_enhanced`: Ollama-powered (requires ML setup)

---

## Part 6: Troubleshooting

### Bot Won't Start

**Symptom**: Crashes on startup

**Checklist**:
1. Verify environment variables: `env | grep POLYMARKET`
2. Check database running: `docker-compose ps postgres`
3. Test database connection: `docker-compose exec postgres psql -U spreaduser -d spreadable -c "SELECT 1;"`
4. Validate config: `cargo run -- --config config/development.toml --validate`
5. Check logs for specific error message

### WebSocket Disconnects

**Symptom**: `websocket_connected = 0` in metrics

**Causes**:
- Network connectivity issue
- Invalid API credentials
- Polymarket API outage
- Rate limiting

**Actions**:
1. Check internet connection: `ping 8.8.8.8`
2. Verify API credentials haven't expired
3. Check Polymarket status page
4. Wait for automatic reconnection (max 60 seconds)
5. If persists >5 minutes, restart bot

### Database Full

**Symptom**: "Disk full" error

**Solution**:
```bash
# Check disk usage
df -h

# Clean old orderbook snapshots (keeps last 7 days)
docker-compose exec postgres psql -U spreaduser -d spreadable -c "
DELETE FROM orderbook_snapshots
WHERE timestamp < NOW() - INTERVAL '7 days';"

# Compress old data (TimescaleDB)
docker-compose exec postgres psql -U spreaduser -d spreadable -c "
SELECT compress_chunk(i)
FROM show_chunks('orderbook_snapshots', older_than => INTERVAL '1 day') i;"
```

### Circuit Breaker Triggered

**Symptom**: Logs show "CIRCUIT BREAKER TRIGGERED"

**Causes**:
- Daily loss limit reached
- Rapid loss rate (5% in 15 minutes)
- WebSocket disconnected >60 seconds
- Manual trigger

**Actions**:
1. **DO NOT immediately reset**
2. Check logs for trigger reason
3. Review positions and P&L: `SELECT * FROM positions;`
4. Investigate root cause (bad strategy? market anomaly?)
5. Only reset after understanding issue
6. Reset: Edit config, set `circuit_breaker.enabled = false`, restart

---

## Part 7: Going to Production (Important!)

### Prerequisites for Live Trading

**Before setting `paper_trading_mode = false`**:

1. ✅ **30+ Days of Paper Trading**: Run continuously for at least 30 days
2. ✅ **Positive Simulated Results**: Verify strategy is profitable in simulation
3. ✅ **System Stability**: Confirm 99%+ uptime, no crashes
4. ✅ **Risk Limits Tested**: Circuit breaker has triggered and reset successfully in testing
5. ✅ **Monitoring Configured**: Alerts set up for critical metrics
6. ✅ **Capital Allocated**: Start with $50-100 maximum
7. ✅ **Backup Plan**: Kill switch accessible, able to cancel all orders manually

### Enabling Live Trading

```toml
[risk]
paper_trading_mode = false  # DANGER: This enables real money trading

# Start with very conservative limits
max_position_per_market = 50.0   # $50 (was 100 in paper trading)
max_total_exposure = 200.0       # $200 (was 500)
max_order_size = 10.0            # $10 (was 20)
max_daily_loss = 25.0            # $25 (was 50)
```

**First Live Run Checklist**:
- [ ] Reduce risk limits to 50% of paper trading values
- [ ] Enable circuit breaker
- [ ] Start with ONE market only
- [ ] Monitor continuously for first 2 hours
- [ ] Verify orders appear on Polymarket website
- [ ] Confirm fills update positions correctly
- [ ] Check actual P&L matches expectations

### Production Deployment

```bash
# Use production config
cargo run --release -- --config config/production.toml

# Or run as systemd service (recommended)
sudo cp monitoring/spreadable.service /etc/systemd/system/
sudo systemctl enable spreadable
sudo systemctl start spreadable
sudo systemctl status spreadable
```

**Production Monitoring**:
- Set up Grafana alerts (Slack, email, SMS)
- Monitor daily: positions, P&L, fill rates
- Weekly review: strategy performance, adjust parameters
- Keep detailed logs for compliance

---

## Summary

You've successfully:
- ✅ Installed and built the market-making bot
- ✅ Configured database and environment
- ✅ Started bot in paper trading mode
- ✅ Added markets and observed simulated trading
- ✅ Set up monitoring with Prometheus + Grafana
- ✅ Learned operational tasks (adding markets, adjusting limits)

**Next Steps**:
1. Run in paper trading for 30+ days
2. Analyze simulated performance
3. Tune strategy parameters based on data
4. When ready (and only when ready): enable live trading with minimal capital

**Safety Reminders**:
- Always start in paper trading mode
- Never skip the 30-day validation period
- Start live trading with tiny amounts ($50-100)
- Monitor constantly during first weeks
- Have kill switch ready at all times
- Understand everything before automating

**Support**:
- Logs: Check `target/release/spreadable.log`
- Metrics: `http://localhost:9090/metrics`
- Database: `docker-compose exec postgres psql -U spreaduser -d spreadable`
- Issues: Document and review before making changes

Happy (safe) trading! 🎯
