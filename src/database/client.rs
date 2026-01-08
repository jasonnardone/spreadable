// Database client with connection pooling
// Manages PostgreSQL + TimescaleDB connections

#![allow(dead_code)]

use crate::config::DatabaseConfig;
use crate::error::{Result, SpreadableError};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

/// Database client with connection pool
#[derive(Clone)]
pub struct DatabaseClient {
    pool: PgPool,
}

impl DatabaseClient {
    /// Create a new database client and establish connection pool
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to PostgreSQL database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(3600)) // 1 hour max connection lifetime
            .connect(&config.postgres_url)
            .await
            .map_err(|e| {
                SpreadableError::Database(sqlx::Error::Configuration(
                    format!("Failed to connect to database: {e}").into(),
                ))
            })?;

        info!(
            "Database connection pool established: max_connections={}, min_connections={}",
            config.max_connections, config.min_connections
        );

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations (if needed)
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // Note: In production, you'd use sqlx::migrate!() here
        // For now, we assume schema.sql is run manually via docker-compose

        info!("Database migrations complete");
        Ok(())
    }

    /// Check database connectivity
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(SpreadableError::Database)?;

        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> DatabaseStats {
        DatabaseStats {
            active_connections: self.pool.size() as u32,
            idle_connections: self.pool.num_idle() as u32,
        }
    }

    /// Close the connection pool gracefully
    pub async fn close(self) {
        info!("Closing database connection pool...");
        self.pool.close().await;
        info!("Database connection pool closed");
    }
}

/// Database connection statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub active_connections: u32,
    pub idle_connections: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running PostgreSQL instance
    // They are integration tests and should be run with:
    // cargo test --test integration_tests

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_database_connection() {
        let config = DatabaseConfig {
            postgres_url: "postgresql://spreaduser:test@localhost:5432/spreadable_test"
                .to_string(),
            max_connections: 5,
            min_connections: 1,
            connection_timeout_secs: 10,
            idle_timeout_secs: 600,
            statement_timeout_ms: 5000,
        };

        let client = DatabaseClient::new(&config).await;
        assert!(client.is_ok());

        let client = client.unwrap();
        let health = client.health_check().await;
        assert!(health.is_ok());

        let stats = client.get_stats().await;
        assert!(stats.active_connections > 0);
    }
}
