# Polymarket API Contract

**Feature**: Polymarket Market-Making Bot
**Date**: 2026-01-05
**Purpose**: Document external Polymarket CLOB API contracts for WebSocket and REST integrations

## Overview

This document specifies the Polymarket Central Limit Order Book (CLOB) API contracts that the market-making bot depends on. These contracts define expected message formats, authentication requirements, and error handling for reliable integration.

## API Endpoints

### Base URLs

- **WebSocket**: `wss://clob.polymarket.com/ws`
- **REST API**: `https://clob.polymarket.com/api/v1`

### Authentication

**Method**: HMAC-SHA256 request signing

All REST requests and WebSocket subscriptions require authentication using API key and secret.

**Request Signing**:
```
Signature = HMAC-SHA256(secret, timestamp + method + path + body)

Headers:
  - X-API-KEY: <api_key>
  - X-SIGNATURE: <hex_encoded_signature>
  - X-TIMESTAMP: <unix_timestamp_ms>
```

**Example (Rust)**:
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn sign_request(secret: &str, timestamp: u64, method: &str, path: &str, body: &str) -> String {
    let message = format!("{}{}{}{}", timestamp, method, path, body);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
```

## WebSocket API

### Connection Lifecycle

1. **Connect**: `wss://clob.polymarket.com/ws`
2. **Authenticate**: Send auth message with API key + signature
3. **Subscribe**: Subscribe to market channels
4. **Receive**: Process orderbook updates and trade events
5. **Heartbeat**: Send ping every 30 seconds, expect pong
6. **Reconnect**: On disconnect, use exponential backoff (1s, 2s, 4s, 8s, max 60s)

### Message Formats

#### 1. Authentication Request

**Direction**: Client → Server
**When**: Immediately after WebSocket connection established

```json
{
  "type": "auth",
  "api_key": "your_api_key",
  "signature": "hmac_sha256_signature",
  "timestamp": 1704441600000
}
```

**Response**:
```json
{
  "type": "auth_response",
  "status": "success",
  "message": "Authenticated successfully"
}
```

**Error Response**:
```json
{
  "type": "error",
  "code": "AUTH_FAILED",
  "message": "Invalid signature"
}
```

---

#### 2. Subscribe to Market

**Direction**: Client → Server
**When**: After successful authentication

```json
{
  "type": "subscribe",
  "channel": "orderbook",
  "market_id": "0x1234567890abcdef"
}
```

**Response**:
```json
{
  "type": "subscribed",
  "channel": "orderbook",
  "market_id": "0x1234567890abcdef"
}
```

---

#### 3. Orderbook Update (Snapshot)

**Direction**: Server → Client
**When**: Immediately after subscription, full orderbook state

```json
{
  "type": "orderbook_snapshot",
  "market_id": "0x1234567890abcdef",
  "timestamp": 1704441600000,
  "bids": [
    ["0.52", "1000.00"],
    ["0.51", "500.00"],
    ["0.50", "250.00"]
  ],
  "asks": [
    ["0.53", "800.00"],
    ["0.54", "600.00"],
    ["0.55", "400.00"]
  ]
}
```

**Field Specifications**:
- `market_id`: Hex-encoded market identifier
- `timestamp`: Unix timestamp in milliseconds
- `bids`: Array of `[price, size]` tuples, sorted descending by price
- `asks`: Array of `[price, size]` tuples, sorted ascending by price
- All prices and sizes are strings to preserve precision

---

#### 4. Orderbook Update (Delta)

**Direction**: Server → Client
**When**: Continuously as orderbook changes

```json
{
  "type": "orderbook_delta",
  "market_id": "0x1234567890abcdef",
  "timestamp": 1704441601000,
  "changes": [
    {
      "side": "bid",
      "price": "0.52",
      "size": "1200.00"
    },
    {
      "side": "ask",
      "price": "0.53",
      "size": "0.00"
    }
  ]
}
```

**Field Specifications**:
- `changes`: Array of price level updates
- `side`: "bid" or "ask"
- `size`: "0.00" indicates price level removed

**Processing Logic**:
```rust
fn apply_delta(orderbook: &mut OrderBook, delta: OrderbookDelta) {
    for change in delta.changes {
        let levels = match change.side {
            Side::Bid => &mut orderbook.bids,
            Side::Ask => &mut orderbook.asks,
        };

        if change.size == Decimal::ZERO {
            // Remove price level
            levels.retain(|level| level.price != change.price);
        } else {
            // Update or insert price level
            if let Some(level) = levels.iter_mut().find(|l| l.price == change.price) {
                level.size = change.size;
            } else {
                levels.push(PriceLevel { price: change.price, size: change.size });
                // Re-sort: bids descending, asks ascending
                match change.side {
                    Side::Bid => levels.sort_by(|a, b| b.price.cmp(&a.price)),
                    Side::Ask => levels.sort_by(|a, b| a.price.cmp(&b.price)),
                }
            }
        }
    }
}
```

---

#### 5. Trade Event

**Direction**: Server → Client
**When**: A trade executes on the market

```json
{
  "type": "trade",
  "market_id": "0x1234567890abcdef",
  "timestamp": 1704441602000,
  "trade_id": "trade_abc123",
  "price": "0.525",
  "size": "100.00",
  "side": "buy"
}
```

**Field Specifications**:
- `side`: "buy" or "sell" (taker side)
- Used for volume tracking, not order fills (see REST API for fills)

---

#### 6. Heartbeat (Ping/Pong)

**Direction**: Bidirectional
**When**: Every 30 seconds to keep connection alive

**Client Ping**:
```json
{
  "type": "ping",
  "timestamp": 1704441630000
}
```

**Server Pong**:
```json
{
  "type": "pong",
  "timestamp": 1704441630000
}
```

**Timeout Handling**:
- If no pong received within 10 seconds of ping, assume stale connection
- Close and reconnect with exponential backoff

---

#### 7. Error Messages

**Direction**: Server → Client
**When**: Invalid request or internal error

```json
{
  "type": "error",
  "code": "INVALID_MARKET",
  "message": "Market ID not found: 0xinvalid",
  "timestamp": 1704441600000
}
```

**Common Error Codes**:
- `AUTH_FAILED`: Invalid API key or signature
- `INVALID_MARKET`: Market ID doesn't exist
- `RATE_LIMIT`: Too many requests
- `INTERNAL_ERROR`: Server-side error

---

## REST API

### Rate Limits

- **Orders**: 5 requests per second per API key
- **Market Data**: 10 requests per second per API key
- **Rate Limit Headers**:
  ```
  X-RateLimit-Limit: 5
  X-RateLimit-Remaining: 4
  X-RateLimit-Reset: 1704441660
  ```

### Endpoints

#### 1. Place Order

**Method**: `POST /orders`
**Purpose**: Submit a new limit order

**Request**:
```json
{
  "market_id": "0x1234567890abcdef",
  "side": "buy",
  "price": "0.52",
  "size": "100.00",
  "type": "limit",
  "time_in_force": "GTC"
}
```

**Request Fields**:
- `market_id`: Hex-encoded market identifier
- `side`: "buy" or "sell"
- `price`: Limit price as string
- `size`: Order quantity as string
- `type`: "limit" (market orders not supported for market-making)
- `time_in_force`: "GTC" (Good-Till-Cancel) or "IOC" (Immediate-Or-Cancel)

**Response (Success)**:
```json
{
  "order_id": "order_xyz789",
  "market_id": "0x1234567890abcdef",
  "side": "buy",
  "price": "0.52",
  "size": "100.00",
  "filled_size": "0.00",
  "status": "open",
  "created_at": 1704441600000
}
```

**Response (Error)**:
```json
{
  "error": {
    "code": "INSUFFICIENT_FUNDS",
    "message": "Insufficient balance to place order",
    "details": {
      "required": "52.00",
      "available": "50.00"
    }
  }
}
```

**Common Errors**:
- `INSUFFICIENT_FUNDS`: Not enough balance
- `INVALID_PRICE`: Price outside allowed range
- `MARKET_CLOSED`: Market not accepting orders
- `RATE_LIMIT`: Exceeded order rate limit

---

#### 2. Cancel Order

**Method**: `DELETE /orders/{order_id}`
**Purpose**: Cancel an existing order

**Request**: No body required

**Response (Success)**:
```json
{
  "order_id": "order_xyz789",
  "status": "cancelled",
  "cancelled_at": 1704441610000
}
```

**Response (Error)**:
```json
{
  "error": {
    "code": "ORDER_NOT_FOUND",
    "message": "Order not found or already cancelled"
  }
}
```

---

#### 3. Cancel All Orders

**Method**: `DELETE /orders`
**Purpose**: Cancel all open orders (optionally filtered by market)

**Request**:
```json
{
  "market_id": "0x1234567890abcdef"  // Optional, omit to cancel all markets
}
```

**Response**:
```json
{
  "cancelled_orders": [
    "order_xyz789",
    "order_abc123"
  ],
  "count": 2
}
```

---

#### 4. Get Order Status

**Method**: `GET /orders/{order_id}`
**Purpose**: Query current status of an order

**Response**:
```json
{
  "order_id": "order_xyz789",
  "market_id": "0x1234567890abcdef",
  "side": "buy",
  "price": "0.52",
  "size": "100.00",
  "filled_size": "50.00",
  "status": "partially_filled",
  "created_at": 1704441600000,
  "updated_at": 1704441605000,
  "fills": [
    {
      "fill_id": "fill_aaa111",
      "price": "0.52",
      "size": "30.00",
      "fee": "0.156",
      "timestamp": 1704441602000
    },
    {
      "fill_id": "fill_bbb222",
      "price": "0.52",
      "size": "20.00",
      "fee": "0.104",
      "timestamp": 1704441605000
    }
  ]
}
```

**Status Values**:
- `open`: Resting in orderbook, no fills
- `partially_filled`: Some fills, still open
- `filled`: Fully executed
- `cancelled`: Cancelled by user
- `rejected`: Rejected by exchange

---

#### 5. Get Open Orders

**Method**: `GET /orders?market_id={market_id}&status=open`
**Purpose**: List all open orders

**Query Parameters**:
- `market_id`: Filter by market (optional)
- `status`: Filter by status, typically "open" or "partially_filled"
- `limit`: Max results (default 50, max 500)

**Response**:
```json
{
  "orders": [
    {
      "order_id": "order_xyz789",
      "market_id": "0x1234567890abcdef",
      "side": "buy",
      "price": "0.52",
      "size": "100.00",
      "filled_size": "0.00",
      "status": "open",
      "created_at": 1704441600000
    },
    {
      "order_id": "order_abc123",
      "market_id": "0x1234567890abcdef",
      "side": "sell",
      "price": "0.54",
      "size": "100.00",
      "filled_size": "0.00",
      "status": "open",
      "created_at": 1704441601000
    }
  ],
  "count": 2
}
```

---

#### 6. Get Orderbook Snapshot

**Method**: `GET /markets/{market_id}/orderbook`
**Purpose**: Fetch current orderbook (REST alternative to WebSocket)

**Response**:
```json
{
  "market_id": "0x1234567890abcdef",
  "timestamp": 1704441600000,
  "bids": [
    ["0.52", "1000.00"],
    ["0.51", "500.00"]
  ],
  "asks": [
    ["0.53", "800.00"],
    ["0.54", "600.00"]
  ]
}
```

**Usage**: Primarily for initial orderbook fetch or WebSocket reconnection recovery

---

## Error Handling

### HTTP Status Codes

- `200 OK`: Success
- `400 Bad Request`: Invalid request format or parameters
- `401 Unauthorized`: Authentication failed
- `403 Forbidden`: API key doesn't have permission
- `404 Not Found`: Resource (order, market) not found
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server-side error
- `503 Service Unavailable`: Temporary outage

### Retry Strategy

**Retriable Errors** (with exponential backoff):
- `429 Too Many Requests`: Wait until rate limit reset
- `500 Internal Server Error`: Retry after 1s, 2s, 4s, max 3 attempts
- `503 Service Unavailable`: Retry after 5s, 10s, 20s, max 5 attempts
- Network errors (timeout, connection reset): Retry with backoff

**Non-Retriable Errors** (fail immediately):
- `400 Bad Request`: Fix request format
- `401 Unauthorized`: Check API credentials
- `404 Not Found`: Order/market doesn't exist

---

## Validation Rules

### Client-Side Pre-Submission Validation

Before submitting orders to Polymarket API:

1. **Price Validation**:
   - Must be > 0 and <= 1.0 (probabilities)
   - Round to 2 decimal places (e.g., 0.52, not 0.525)

2. **Size Validation**:
   - Must be > 0
   - Round to 2 decimal places
   - Check against available balance

3. **Spread Validation**:
   - Bid must be < Ask
   - Spread must be >= `min_spread_bps` from RiskConfig

4. **Rate Limiting**:
   - Enforce client-side 5 orders/second limit
   - Queue orders if rate exceeded

5. **Market Status**:
   - Only submit to Active markets
   - Don't submit to Settled or Delisted markets

---

## Testing Contracts

### Mock WebSocket Server (for integration tests)

```rust
// Example mock messages for testing
const MOCK_ORDERBOOK_SNAPSHOT: &str = r#"{
  "type": "orderbook_snapshot",
  "market_id": "0xtest",
  "timestamp": 1704441600000,
  "bids": [["0.50", "1000"]],
  "asks": [["0.51", "800"]]
}"#;

const MOCK_TRADE_EVENT: &str = r#"{
  "type": "trade",
  "market_id": "0xtest",
  "timestamp": 1704441601000,
  "trade_id": "trade_123",
  "price": "0.505",
  "size": "50",
  "side": "buy"
}"#;
```

### Contract Test Scenarios

1. **Valid Orderbook Parsing**: Deserialize all message types without errors
2. **Invalid Messages**: Handle malformed JSON gracefully
3. **Missing Fields**: Handle partial messages (log warning, don't crash)
4. **Delta Application**: Verify orderbook state after applying deltas
5. **Reconnection**: Simulate disconnect → reconnect → re-subscribe flow
6. **Rate Limiting**: Verify 429 responses trigger backoff
7. **Authentication**: Test signature generation matches expected values

---

## Conclusion

This contract documentation ensures reliable integration with the Polymarket CLOB API. Key takeaways:

- **Authentication**: HMAC-SHA256 signing required for all requests
- **WebSocket**: Primary data source for low-latency orderbook updates
- **REST API**: Used for order submission, cancellation, and queries
- **Error Handling**: Implement retry logic with exponential backoff for retriable errors
- **Validation**: Client-side validation prevents invalid API calls
- **Testing**: Contract tests ensure message format compatibility

Next: See `internal-events.md` for internal event bus contracts between system components.
