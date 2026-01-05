// Database query implementations
// CRUD operations for markets, orders, positions, etc.

use crate::error::{Result, SpreadableError};
use crate::types::{
    CircuitBreakerReason, Fill, Market, MarketId, MarketStatus, Order, OrderId, OrderSide,
    OrderStatus, Position,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;

// ============================================================================
// Market Queries
// ============================================================================

/// Insert or update a market
pub async fn upsert_market(pool: &PgPool, market: &Market) -> Result<()> {
    let status_str = market.status.to_string();

    sqlx::query!(
        r#"
        INSERT INTO markets (market_id, question, description, status, end_date, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (market_id) DO UPDATE SET
            question = EXCLUDED.question,
            description = EXCLUDED.description,
            status = EXCLUDED.status,
            end_date = EXCLUDED.end_date,
            updated_at = EXCLUDED.updated_at
        "#,
        market.market_id.as_str(),
        market.question,
        market.description,
        status_str,
        market.end_date,
        market.created_at,
        market.updated_at,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    debug!(market_id = %market.market_id, "Upserted market");
    Ok(())
}

/// Get market by ID
pub async fn get_market(pool: &PgPool, market_id: &MarketId) -> Result<Option<Market>> {
    let row = sqlx::query!(
        r#"
        SELECT market_id, question, description, status, end_date, created_at, updated_at
        FROM markets
        WHERE market_id = $1
        "#,
        market_id.as_str()
    )
    .fetch_optional(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(row.map(|r| Market {
        market_id: MarketId::new(r.market_id),
        question: r.question,
        description: r.description,
        status: match r.status.as_str() {
            "Active" => MarketStatus::Active,
            "Settled" => MarketStatus::Settled,
            "Delisted" => MarketStatus::Delisted,
            _ => MarketStatus::Delisted, // Default to Delisted for unknown statuses
        },
        end_date: r.end_date,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }))
}

/// Get all active markets
pub async fn get_active_markets(pool: &PgPool) -> Result<Vec<Market>> {
    let rows = sqlx::query!(
        r#"
        SELECT market_id, question, description, status, end_date, created_at, updated_at
        FROM markets
        WHERE status = 'Active'
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(rows
        .into_iter()
        .map(|r| Market {
            market_id: MarketId::new(r.market_id),
            question: r.question,
            description: r.description,
            status: MarketStatus::Active,
            end_date: r.end_date,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

// ============================================================================
// Order Queries
// ============================================================================

/// Insert a new order
pub async fn insert_order(pool: &PgPool, order: &Order) -> Result<()> {
    let side_str = order.side.to_string();
    let status_str = order.status.to_string();

    sqlx::query!(
        r#"
        INSERT INTO orders (
            order_id, exchange_order_id, market_id, side, price, size, filled_size,
            status, created_at, submitted_at, opened_at, closed_at, paper_trading
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        order.order_id.0,
        order.exchange_order_id,
        order.market_id.as_str(),
        side_str,
        order.price,
        order.size,
        order.filled_size,
        status_str,
        order.created_at,
        order.submitted_at,
        order.opened_at,
        order.closed_at,
        order.paper_trading,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    debug!(order_id = %order.order_id, market_id = %order.market_id, "Inserted order");
    Ok(())
}

/// Update order status
pub async fn update_order_status(
    pool: &PgPool,
    order_id: &OrderId,
    status: OrderStatus,
    exchange_order_id: Option<&str>,
) -> Result<()> {
    let status_str = status.to_string();
    let now = Utc::now();

    sqlx::query!(
        r#"
        UPDATE orders
        SET status = $1,
            exchange_order_id = COALESCE($2, exchange_order_id),
            updated_at = $3,
            opened_at = CASE WHEN $1 = 'open' THEN $3 ELSE opened_at END,
            closed_at = CASE WHEN $1 IN ('filled', 'cancelled', 'rejected', 'failed') THEN $3 ELSE closed_at END
        WHERE order_id = $4
        "#,
        status_str,
        exchange_order_id,
        now,
        order_id.0,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    debug!(order_id = %order_id, status = %status, "Updated order status");
    Ok(())
}

/// Get order by ID
pub async fn get_order(pool: &PgPool, order_id: &OrderId) -> Result<Option<Order>> {
    let row = sqlx::query!(
        r#"
        SELECT order_id, exchange_order_id, market_id, side, price, size, filled_size,
               status, created_at, submitted_at, opened_at, closed_at, paper_trading
        FROM orders
        WHERE order_id = $1
        "#,
        order_id.0
    )
    .fetch_optional(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(row.map(|r| Order {
        order_id: OrderId(r.order_id),
        exchange_order_id: r.exchange_order_id,
        market_id: MarketId::new(r.market_id),
        side: match r.side.as_str() {
            "Buy" => OrderSide::Buy,
            "Sell" => OrderSide::Sell,
            _ => OrderSide::Buy,
        },
        price: r.price,
        size: r.size,
        filled_size: r.filled_size,
        status: match r.status.as_str() {
            "pending_submit" => OrderStatus::PendingSubmit,
            "open" => OrderStatus::Open,
            "partially_filled" => OrderStatus::PartiallyFilled,
            "filled" => OrderStatus::Filled,
            "cancelled" => OrderStatus::Cancelled,
            "rejected" => OrderStatus::Rejected,
            "failed" => OrderStatus::Failed,
            _ => OrderStatus::Failed,
        },
        created_at: r.created_at,
        submitted_at: r.submitted_at,
        opened_at: r.opened_at,
        closed_at: r.closed_at,
        paper_trading: r.paper_trading,
    }))
}

/// Get all open orders for a market
pub async fn get_open_orders(pool: &PgPool, market_id: &MarketId) -> Result<Vec<Order>> {
    let rows = sqlx::query!(
        r#"
        SELECT order_id, exchange_order_id, market_id, side, price, size, filled_size,
               status, created_at, submitted_at, opened_at, closed_at, paper_trading
        FROM orders
        WHERE market_id = $1 AND status IN ('open', 'partially_filled')
        ORDER BY created_at DESC
        "#,
        market_id.as_str()
    )
    .fetch_all(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(rows
        .into_iter()
        .map(|r| Order {
            order_id: OrderId(r.order_id),
            exchange_order_id: r.exchange_order_id,
            market_id: MarketId::new(r.market_id),
            side: match r.side.as_str() {
                "Buy" => OrderSide::Buy,
                "Sell" => OrderSide::Sell,
                _ => OrderSide::Buy,
            },
            price: r.price,
            size: r.size,
            filled_size: r.filled_size,
            status: match r.status.as_str() {
                "open" => OrderStatus::Open,
                "partially_filled" => OrderStatus::PartiallyFilled,
                _ => OrderStatus::Open,
            },
            created_at: r.created_at,
            submitted_at: r.submitted_at,
            opened_at: r.opened_at,
            closed_at: r.closed_at,
            paper_trading: r.paper_trading,
        })
        .collect())
}

// ============================================================================
// Position Queries
// ============================================================================

/// Upsert position
pub async fn upsert_position(pool: &PgPool, position: &Position) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO positions (
            market_id, size, avg_entry_price, realized_pnl, unrealized_pnl,
            total_pnl, trade_count, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (market_id) DO UPDATE SET
            size = EXCLUDED.size,
            avg_entry_price = EXCLUDED.avg_entry_price,
            realized_pnl = EXCLUDED.realized_pnl,
            unrealized_pnl = EXCLUDED.unrealized_pnl,
            total_pnl = EXCLUDED.total_pnl,
            trade_count = EXCLUDED.trade_count,
            updated_at = EXCLUDED.updated_at
        "#,
        position.market_id.as_str(),
        position.size,
        position.avg_entry_price,
        position.realized_pnl,
        position.unrealized_pnl,
        position.total_pnl,
        position.trade_count as i32,
        position.updated_at,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    debug!(market_id = %position.market_id, size = %position.size, "Upserted position");
    Ok(())
}

/// Get position by market ID
pub async fn get_position(pool: &PgPool, market_id: &MarketId) -> Result<Option<Position>> {
    let row = sqlx::query!(
        r#"
        SELECT market_id, size, avg_entry_price, realized_pnl, unrealized_pnl,
               total_pnl, trade_count, updated_at
        FROM positions
        WHERE market_id = $1
        "#,
        market_id.as_str()
    )
    .fetch_optional(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(row.map(|r| Position {
        market_id: MarketId::new(r.market_id),
        size: r.size,
        avg_entry_price: r.avg_entry_price,
        realized_pnl: r.realized_pnl,
        unrealized_pnl: r.unrealized_pnl,
        total_pnl: r.total_pnl,
        trade_count: r.trade_count as u64,
        updated_at: r.updated_at,
    }))
}

/// Get all non-flat positions
pub async fn get_all_positions(pool: &PgPool) -> Result<Vec<Position>> {
    let rows = sqlx::query!(
        r#"
        SELECT market_id, size, avg_entry_price, realized_pnl, unrealized_pnl,
               total_pnl, trade_count, updated_at
        FROM positions
        WHERE ABS(size) > 0.01
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(rows
        .into_iter()
        .map(|r| Position {
            market_id: MarketId::new(r.market_id),
            size: r.size,
            avg_entry_price: r.avg_entry_price,
            realized_pnl: r.realized_pnl,
            unrealized_pnl: r.unrealized_pnl,
            total_pnl: r.total_pnl,
            trade_count: r.trade_count as u64,
            updated_at: r.updated_at,
        })
        .collect())
}

// ============================================================================
// Fill Queries
// ============================================================================

/// Insert a fill
pub async fn insert_fill(pool: &PgPool, fill: &Fill) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO fills (fill_id, order_id, exchange_fill_id, price, size, fee, timestamp)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        fill.fill_id,
        fill.order_id.0,
        fill.exchange_fill_id,
        fill.price,
        fill.size,
        fill.fee,
        fill.timestamp,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    debug!(fill_id = %fill.fill_id, order_id = %fill.order_id, "Inserted fill");
    Ok(())
}

// ============================================================================
// Orderbook Snapshot Queries
// ============================================================================

/// Insert orderbook snapshot
pub async fn insert_orderbook_snapshot(
    pool: &PgPool,
    market_id: &MarketId,
    timestamp: DateTime<Utc>,
    best_bid: Option<Decimal>,
    best_ask: Option<Decimal>,
    bid_size: Option<Decimal>,
    ask_size: Option<Decimal>,
    mid_price: Option<Decimal>,
    spread_bps: Option<i32>,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO orderbook_snapshots (
            market_id, timestamp, best_bid, best_ask, bid_size, ask_size, mid_price, spread_bps
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        market_id.as_str(),
        timestamp,
        best_bid,
        best_ask,
        bid_size,
        ask_size,
        mid_price,
        spread_bps,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    Ok(())
}

// ============================================================================
// Circuit Breaker Queries
// ============================================================================

/// Insert circuit breaker event
pub async fn insert_circuit_breaker_event(
    pool: &PgPool,
    reason: CircuitBreakerReason,
    daily_loss: Option<Decimal>,
    loss_rate_pct: Option<Decimal>,
    total_exposure: Option<Decimal>,
) -> Result<()> {
    let reason_str = reason.to_string();

    sqlx::query!(
        r#"
        INSERT INTO circuit_breaker_events (
            triggered_at, reason, daily_loss, loss_rate_pct, total_exposure
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
        Utc::now(),
        reason_str,
        daily_loss,
        loss_rate_pct,
        total_exposure,
    )
    .execute(pool)
    .await
    .map_err(SpreadableError::Database)?;

    debug!(reason = %reason, "Inserted circuit breaker event");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running PostgreSQL instance
    // They are integration tests and should be run with:
    // cargo test --test integration_tests

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_market_crud() {
        // This would be implemented in integration tests
        // with proper database setup/teardown
    }
}
