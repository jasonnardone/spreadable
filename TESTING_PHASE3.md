# Testing Phase 3 - Market Data Collection

**Status**: Phase 3 implementation complete ✅
**Date**: 2026-01-05
**Next Session**: Continue with Phase 4 (OMS + Risk + Strategy)

---

## 🎯 What You Have Working

A complete market data collection system that:
- Connects to Polymarket WebSocket API
- Streams real-time orderbook updates
- Maintains local orderbook state with delta application
- Persists snapshots to TimescaleDB
- Handles reconnection with exponential backoff
- Supports multiple markets concurrently

**Files Implemented:**
- `src/market_data/websocket_client.rs` (427 lines)
- `src/market_data/orderbook_manager.rs` (362 lines)
- `src/market_data/service.rs` (166 lines)
- `src/main.rs` (updated with market data integration)

---

## 📋 Prerequisites

### 1. Install Rust (if not already installed)

```bash
# Windows (PowerShell/Git Bash)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should show 1.75 or higher
cargo --version
```

### 2. Install Docker Desktop

Download from: https://www.docker.com/products/docker-desktop

Verify:
```bash
docker --version
docker-compose --version
```

### 3. Get Polymarket API Credentials

1. Go to https://polymarket.com
2. Create an account (if needed)
3. Navigate to Settings → API
4. Generate API key and secret
5. **KEEP THESE SECRET** - never commit to git

---

## 🚀 Quick Start Testing

### Step 1: Set Up Environment

```bash
cd C:/code/spreadable

# Copy example environment file
cp .env.example .env

# Edit .env with your Polymarket credentials
notepad .env  # or use your preferred editor
```

**Edit `.env` file:**
```bash
# Replace with your actual Polymarket API credentials
POLYMARKET_API_KEY=your_actual_api_key_here
POLYMARKET_API_SECRET=your_actual_api_secret_here

# Database password (choose any secure password)
DB_PASSWORD=your_secure_password_here

# Database connection string (uses password above)
DATABASE_URL=postgresql://spreaduser:your_secure_password_here@localhost:5432/spreadable

# Logging
RUST_LOG=info,spreadable=debug
```

### Step 2: Start Database

```bash
# Start PostgreSQL with TimescaleDB
docker-compose up -d postgres

# Wait 15 seconds for database to initialize
# (First time only - creates tables, indexes, hypertables)

# Verify database is running
docker-compose ps
# Should show: spreadable-postgres Up (healthy)

# Check logs if needed
docker-compose logs postgres
```

### Step 3: Validate Configuration

```bash
# Validate config without running
cargo run --release -- --config config/development.toml --validate

# Expected output:
# ✓ Configuration loaded successfully
# ✓ Paper trading mode: ENABLED
# ✓ Active strategy: basic_mm
# ✓ Enabled markets: none (add via config)
```

**If this fails:**
- Check `RUST_LOG` in `.env` is set correctly
- Verify `.env` file exists and has correct format
- Ensure `config/development.toml` exists

### Step 4: Add Test Markets

Find a Polymarket market to monitor:

1. Go to https://polymarket.com
2. Click on any active market
3. Copy the market ID from URL:
   - URL: `https://polymarket.com/event/slug?market=0x1234567890abcdef...`
   - Copy: `0x1234567890abcdef...`

**Edit `config/development.toml`:**
```toml
[markets]
enabled = [
    "0x1234567890abcdef1234567890abcdef12345678"  # Replace with actual market ID
]
```

**Popular markets to test** (check polymarket.com for current markets):
- Presidential election markets
- Sports outcome markets
- Economic indicator markets

### Step 5: Run the Bot!

```bash
# Build and run in release mode
cargo run --release -- --config config/development.toml

# Or build first, then run (faster for repeated runs)
cargo build --release
./target/release/spreadable --config config/development.toml
```

---

## ✅ What to Expect (Success Indicators)

### Console Output

```
INFO  Spreadable Market-Making Bot v0.1.0
INFO  Configuration loaded from: config/development.toml
INFO  Paper trading mode: ENABLED
INFO  Starting Spreadable Market-Making Bot...
INFO  Connecting to database...
INFO  Database connection pool established: max_connections=10, min_connections=2
INFO  Running database migrations...
INFO  Database migrations complete
INFO  Database connection established
INFO  Initializing market data collection for 1 markets
INFO  Market data collection started
INFO  Spreadable is running in PAPER TRADING mode
INFO  Attempting WebSocket connection...
INFO  WebSocket connected to wss://clob.polymarket.com/ws
DEBUG Authentication message sent
INFO  WebSocket authentication successful
DEBUG Subscribed to market market_id=0x1234...
INFO  Market subscription confirmed market_id=0x1234...
INFO  Orderbook snapshot processed market_id=0x1234... bid_levels=45 ask_levels=48
INFO  Orderbook snapshot processed and persisted market_id=0x1234... best_bid=0.52 best_ask=0.54 spread_bps=384
DEBUG Orderbook delta applied market_id=0x1234... bid_levels=45 ask_levels=48
```

**Every few seconds, you should see:**
- Orderbook delta messages (incremental updates)
- Best bid/ask prices updating
- Spread in basis points

### Database Verification

Open a new terminal:

```bash
# Connect to database
docker-compose exec postgres psql -U spreaduser -d spreadable

# Check orderbook snapshots are being collected
SELECT
    market_id,
    COUNT(*) as snapshots,
    MAX(timestamp) as latest,
    MIN(timestamp) as earliest
FROM orderbook_snapshots
GROUP BY market_id;

# Expected output:
#              market_id              | snapshots |          latest           |         earliest
# ------------------------------------+-----------+---------------------------+---------------------------
#  0x1234567890abcdef1234567890abcdef |       142 | 2026-01-05 15:30:45+00    | 2026-01-05 15:20:12+00

# View recent snapshots with prices
SELECT
    timestamp,
    best_bid,
    best_ask,
    mid_price,
    spread_bps
FROM orderbook_snapshots
WHERE market_id = '0x1234567890abcdef1234567890abcdef12345678'
ORDER BY timestamp DESC
LIMIT 10;

# Exit
\q
```

**Success indicators:**
- Snapshot count increases over time
- Timestamps are recent (within last few minutes)
- Best bid < best ask (orderbook is valid)
- Spread is reasonable (usually 100-1000 bps for prediction markets)

---

## 🐛 Troubleshooting

### Bot Won't Start

**Error**: "Failed to load configuration"
```bash
# Check .env file exists
ls -la .env

# Verify .env format (no syntax errors)
cat .env

# Check config file is valid TOML
cargo run -- --config config/development.toml --validate
```

**Error**: "Database connection failed"
```bash
# Check PostgreSQL is running
docker-compose ps postgres

# Restart database if needed
docker-compose restart postgres

# Check DATABASE_URL in .env matches docker-compose.yml
# Default: postgresql://spreaduser:${DB_PASSWORD}@localhost:5432/spreadable
```

**Error**: "Failed to connect to database: password authentication failed"
- Ensure `DB_PASSWORD` in `.env` is set
- Ensure `DATABASE_URL` uses same password
- Check `docker-compose.yml` uses `${DB_PASSWORD}`

### WebSocket Connection Issues

**Error**: "WebSocket connection failed"
- Check internet connection
- Verify Polymarket API is up: https://clob.polymarket.com/health
- Check API credentials are correct in `.env`

**Error**: "Authentication failed"
- Double-check `POLYMARKET_API_KEY` and `POLYMARKET_API_SECRET`
- Ensure no extra spaces in `.env` file
- Verify credentials are valid (not expired)

**Logs show**: "WebSocket disconnected, will retry"
- This is NORMAL - automatic reconnection will happen
- Wait 1-60 seconds for reconnection
- If reconnection fails repeatedly, check API credentials

### No Data Being Collected

**Check markets are configured:**
```bash
# Verify markets section in config
grep -A 5 "\[markets\]" config/development.toml

# Should show:
# [markets]
# enabled = [
#     "0x..."
# ]
```

**Verify market ID is valid:**
- Go to polymarket.com
- Ensure market is **Active** (not Settled or Delisted)
- Copy exact market ID from URL

**Check logs for errors:**
```bash
# Look for error messages in output
# Search for lines containing "error" or "ERROR"
```

### Database Issues

**Error**: "Disk full"
```bash
# Check disk space
df -h

# Clean old snapshots if needed (keeps last 7 days)
docker-compose exec postgres psql -U spreaduser -d spreadable -c "
DELETE FROM orderbook_snapshots
WHERE timestamp < NOW() - INTERVAL '7 days';"
```

**No data in database:**
```bash
# Check if bot is running
ps aux | grep spreadable

# Check logs for database errors
# Look for "Failed to insert orderbook snapshot"
```

---

## 📊 Monitoring & Observability

### View Live Logs

```bash
# Follow logs in real-time
# (Run while bot is running in another terminal)

# Pretty format (easier to read)
tail -f logs/spreadable.log

# Or see all output
# (if you started bot without redirecting output)
```

### Grafana Dashboard (Optional - for Phase 5)

**Note**: Grafana is configured but not required for Phase 3 testing.

```bash
# Start monitoring stack (optional)
docker-compose up -d prometheus grafana

# Access Grafana
# URL: http://localhost:3000
# Username: admin
# Password: admin (change on first login)

# Dashboard: "Spreadable Market Maker - Overview"
```

### Database Statistics

```bash
# Connect to database
docker-compose exec postgres psql -U spreaduser -d spreadable

# Get snapshot counts per market
SELECT
    market_id,
    COUNT(*) as total_snapshots,
    MIN(timestamp) as first_snapshot,
    MAX(timestamp) as last_snapshot,
    EXTRACT(EPOCH FROM (MAX(timestamp) - MIN(timestamp))) as duration_seconds
FROM orderbook_snapshots
GROUP BY market_id;

# Get average spread by market (last hour)
SELECT
    market_id,
    AVG(spread_bps) as avg_spread_bps,
    MIN(spread_bps) as min_spread_bps,
    MAX(spread_bps) as max_spread_bps,
    COUNT(*) as samples
FROM orderbook_snapshots
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY market_id;

# Exit
\q
```

---

## 🧪 Testing Checklist

Before considering Phase 3 validated:

- [ ] Bot starts without errors
- [ ] WebSocket connects and authenticates
- [ ] Markets are subscribed successfully
- [ ] Orderbook snapshots are received
- [ ] Orderbook deltas are applied correctly
- [ ] Data is persisted to database
- [ ] Best bid is always < best ask
- [ ] Spread calculations are reasonable
- [ ] Bot handles CTRL+C gracefully (no errors)
- [ ] Bot reconnects after network interruption
- [ ] Multiple markets work simultaneously (if configured)
- [ ] Database snapshots accumulate over time
- [ ] No memory leaks (check with `docker stats`)

### Test Scenarios

**1. Normal Operation** (15 minutes)
- Start bot with 1-2 markets
- Let run for 15 minutes
- Verify continuous data collection
- Check database has 100+ snapshots per market

**2. Graceful Shutdown**
- Press CTRL+C
- Verify clean shutdown logs
- Restart and verify resumes correctly

**3. Network Interruption** (optional)
- Disconnect WiFi/network for 30 seconds
- Reconnect
- Verify bot reconnects automatically
- Check logs show exponential backoff

**4. Multi-Market** (optional)
- Add 3-5 markets to config
- Restart bot
- Verify all markets receive updates
- Check database has data for all markets

---

## 📈 Performance Expectations

**Normal Operation:**
- CPU: 5-15% (single market)
- Memory: 50-100 MB
- Database size: ~1-5 MB/hour/market (with compression)
- Latency: <10ms orderbook processing
- WebSocket: ~1-10 messages/second/market

**Red Flags:**
- CPU >50% sustained
- Memory growing continuously (leak)
- Database errors in logs
- Websocket disconnecting repeatedly
- Spread values seem wrong (negative, >10000 bps)

---

## 🔧 Cleanup (When Done Testing)

```bash
# Stop the bot
# (CTRL+C in terminal running the bot)

# Stop all Docker services
docker-compose down

# Or keep database data and just stop services
docker-compose stop

# To completely clean up (removes all data)
docker-compose down -v  # WARNING: Deletes database data
```

---

## ✨ Success! What You've Achieved

If all tests pass, you have:

✅ A working real-time market data collection system
✅ Live connection to Polymarket orderbook
✅ Accurate delta-based orderbook maintenance
✅ Persistent time-series data in TimescaleDB
✅ Automatic reconnection and error handling
✅ Multi-market support
✅ Complete observability (logs + database)

**This is a significant milestone!** 🎉

---

## 🚀 Next Steps (Phase 4)

When you return with fresh tokens:

**Phase 4: User Story 2 - Risk-Limited Trading**
- Order Management System (place/cancel orders)
- Risk controls (position limits, circuit breaker)
- Basic market-making strategy
- Position tracking and P&L calculation
- Paper trading mode (simulated fills)

**Estimated tokens needed**: 40-60k tokens
**Estimated time**: 30-45 minutes of implementation

---

## 📞 Support

If you encounter issues:

1. **Check logs**: Look for ERROR or WARN messages
2. **Verify setup**: Run through prerequisites again
3. **Database state**: Query orderbook_snapshots table
4. **Configuration**: Use `--validate` flag
5. **GitHub**: All code is at https://github.com/jasonnardone/spreadable

**Current branch**: `001-polymarket-mm-bot`
**Latest commit**: `6dcb9f8` - Complete Phase 3

---

## 📝 Notes for Next Session

**What's done:**
- ✅ Phase 1: Setup
- ✅ Phase 2: Foundation
- ✅ Phase 3: Market Data Collection

**What's next:**
- ⏳ Phase 4: OMS + Risk + Strategy
- ⏳ Phase 5: Multi-market scaling
- ⏳ Phase 6: ML integration
- ⏳ Phase 7: Production polish

**Key files to know:**
- `src/main.rs` - Application entry point
- `src/market_data/` - Market data collection
- `config/development.toml` - Configuration
- `.env` - Secrets (never commit!)
- `sql/schema.sql` - Database schema

Good luck testing! 🚀
