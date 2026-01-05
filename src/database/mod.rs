// Database module
// PostgreSQL + TimescaleDB client with connection pooling

pub mod client;
pub mod queries;

pub use client::DatabaseClient;
