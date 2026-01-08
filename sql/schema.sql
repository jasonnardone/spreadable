-- Spreadable Market-Making Bot Database Schema
-- PostgreSQL 15+ with TimescaleDB extension

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Markets table
CREATE TABLE IF NOT EXISTS markets (
    market_id TEXT PRIMARY KEY,
    question TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK (status IN ('Active', 'Settled', 'Delisted')),
    end_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_markets_status ON markets(status);
CREATE INDEX idx_markets_end_date ON markets(end_date) WHERE status = 'Active';

-- Orderbook snapshots (time-series data)
CREATE TABLE IF NOT EXISTS orderbook_snapshots (
    id BIGSERIAL,
    market_id TEXT NOT NULL REFERENCES markets(market_id),
    timestamp TIMESTAMPTZ NOT NULL,
    best_bid DECIMAL(10, 4),
    best_ask DECIMAL(10, 4),
    bid_size DECIMAL(18, 2),
    ask_size DECIMAL(18, 2),
    mid_price DECIMAL(10, 4),
    spread_bps INTEGER,
    PRIMARY KEY (id, timestamp)
);

-- Convert to TimescaleDB hypertable
SELECT create_hypertable('orderbook_snapshots', 'timestamp', if_not_exists => TRUE);

-- Enable compression on the hypertable
ALTER TABLE orderbook_snapshots SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'market_id'
);

-- Compression policy (compress data older than 1 day)
SELECT add_compression_policy('orderbook_snapshots', INTERVAL '1 day', if_not_exists => TRUE);

-- Retention policy (keep data for 90 days)
SELECT add_retention_policy('orderbook_snapshots', INTERVAL '90 days', if_not_exists => TRUE);

CREATE INDEX idx_orderbook_market_time ON orderbook_snapshots(market_id, timestamp DESC);

-- Orders table
CREATE TABLE IF NOT EXISTS orders (
    order_id UUID PRIMARY KEY,
    exchange_order_id TEXT UNIQUE,
    market_id TEXT NOT NULL REFERENCES markets(market_id),
    side TEXT NOT NULL CHECK (side IN ('Buy', 'Sell')),
    price DECIMAL(10, 4) NOT NULL CHECK (price > 0 AND price <= 1),
    size DECIMAL(18, 2) NOT NULL CHECK (size > 0),
    filled_size DECIMAL(18, 2) NOT NULL DEFAULT 0 CHECK (filled_size >= 0 AND filled_size <= size),
    status TEXT NOT NULL CHECK (status IN ('pending_submit', 'open', 'partially_filled', 'filled', 'cancelled', 'rejected', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,
    opened_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    paper_trading BOOLEAN NOT NULL DEFAULT TRUE,
    cancel_reason TEXT,
    rejection_reason TEXT,
    error_message TEXT
);

CREATE INDEX idx_orders_market ON orders(market_id);
CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_created ON orders(created_at DESC);
CREATE INDEX idx_orders_exchange ON orders(exchange_order_id) WHERE exchange_order_id IS NOT NULL;

-- Fills table (order executions)
CREATE TABLE IF NOT EXISTS fills (
    fill_id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(order_id),
    exchange_fill_id TEXT UNIQUE,
    price DECIMAL(10, 4) NOT NULL,
    size DECIMAL(18, 2) NOT NULL,
    fee DECIMAL(18, 6) NOT NULL DEFAULT 0,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fills_order ON fills(order_id);
CREATE INDEX idx_fills_timestamp ON fills(timestamp DESC);

-- Positions table
CREATE TABLE IF NOT EXISTS positions (
    market_id TEXT PRIMARY KEY REFERENCES markets(market_id),
    size DECIMAL(18, 2) NOT NULL DEFAULT 0,
    avg_entry_price DECIMAL(10, 4),
    realized_pnl DECIMAL(18, 6) NOT NULL DEFAULT 0,
    unrealized_pnl DECIMAL(18, 6) NOT NULL DEFAULT 0,
    total_pnl DECIMAL(18, 6) NOT NULL DEFAULT 0,
    trade_count INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Risk metrics table (time-series)
CREATE TABLE IF NOT EXISTS risk_metrics (
    id BIGSERIAL,
    timestamp TIMESTAMPTZ NOT NULL,
    total_exposure DECIMAL(18, 2) NOT NULL,
    max_position_size DECIMAL(18, 2) NOT NULL,
    daily_pnl DECIMAL(18, 6) NOT NULL,
    daily_loss DECIMAL(18, 6) NOT NULL,
    open_order_count INTEGER NOT NULL,
    active_market_count INTEGER NOT NULL,
    circuit_breaker_active BOOLEAN NOT NULL DEFAULT FALSE,
    circuit_breaker_reason TEXT,
    PRIMARY KEY (id, timestamp)
);

SELECT create_hypertable('risk_metrics', 'timestamp', if_not_exists => TRUE);

-- Enable compression on the hypertable
ALTER TABLE risk_metrics SET (
    timescaledb.compress
);

SELECT add_compression_policy('risk_metrics', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_retention_policy('risk_metrics', INTERVAL '90 days', if_not_exists => TRUE);

-- Strategy decisions table (for analysis)
CREATE TABLE IF NOT EXISTS strategy_decisions (
    id BIGSERIAL PRIMARY KEY,
    market_id TEXT NOT NULL REFERENCES markets(market_id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    strategy_name TEXT NOT NULL,
    bid_price DECIMAL(10, 4),
    ask_price DECIMAL(10, 4),
    bid_size DECIMAL(18, 2),
    ask_size DECIMAL(18, 2),
    spread_bps INTEGER,
    skew_factor DECIMAL(5, 4),
    decision TEXT NOT NULL CHECK (decision IN ('quote', 'skip', 'cancel_all')),
    reason TEXT,
    ml_confidence DECIMAL(5, 4)
);

CREATE INDEX idx_strategy_market_time ON strategy_decisions(market_id, timestamp DESC);
CREATE INDEX idx_strategy_timestamp ON strategy_decisions(timestamp DESC);

-- Circuit breaker events table
CREATE TABLE IF NOT EXISTS circuit_breaker_events (
    id BIGSERIAL PRIMARY KEY,
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reason TEXT NOT NULL,
    daily_loss DECIMAL(18, 6),
    loss_rate_pct DECIMAL(5, 4),
    total_exposure DECIMAL(18, 2),
    reset_at TIMESTAMPTZ,
    reset_by TEXT
);

CREATE INDEX idx_cb_triggered ON circuit_breaker_events(triggered_at DESC);

-- WebSocket connection events (for debugging)
CREATE TABLE IF NOT EXISTS websocket_events (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type TEXT NOT NULL CHECK (event_type IN ('connected', 'disconnected', 'subscribed', 'error', 'heartbeat_timeout')),
    market_id TEXT,
    error_message TEXT,
    reconnect_attempt INTEGER
);

CREATE INDEX idx_ws_events_timestamp ON websocket_events(timestamp DESC);
CREATE INDEX idx_ws_events_type ON websocket_events(event_type);

-- Functions for automatic timestamp updates
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updated_at columns
CREATE TRIGGER update_markets_updated_at
    BEFORE UPDATE ON markets
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_positions_updated_at
    BEFORE UPDATE ON positions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_orders_updated_at
    BEFORE UPDATE ON orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- View for current risk status
CREATE OR REPLACE VIEW current_risk_status AS
SELECT
    COALESCE(SUM(ABS(size * COALESCE(avg_entry_price, 0.5))), 0) as total_exposure,
    COALESCE(MAX(ABS(size * COALESCE(avg_entry_price, 0.5))), 0) as max_position_size,
    COALESCE(SUM(total_pnl), 0) as total_pnl,
    COUNT(*) as position_count,
    (SELECT COUNT(*) FROM orders WHERE status IN ('open', 'partially_filled')) as open_order_count,
    (SELECT COUNT(*) FROM markets WHERE status = 'Active') as active_market_count
FROM positions
WHERE ABS(size) > 0.01;

-- View for daily P&L
CREATE OR REPLACE VIEW daily_pnl AS
SELECT
    DATE(f.timestamp) as date,
    o.market_id,
    SUM(CASE WHEN o.side = 'Buy' THEN -f.price * f.size ELSE f.price * f.size END) as gross_pnl,
    SUM(f.fee) as total_fees,
    SUM(CASE WHEN o.side = 'Buy' THEN -f.price * f.size ELSE f.price * f.size END) - SUM(f.fee) as net_pnl,
    COUNT(*) as fill_count
FROM fills f
JOIN orders o ON f.order_id = o.order_id
WHERE f.timestamp >= CURRENT_DATE
GROUP BY DATE(f.timestamp), o.market_id;

-- Grant permissions (adjust user as needed)
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO spreaduser;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO spreaduser;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO spreaduser;
