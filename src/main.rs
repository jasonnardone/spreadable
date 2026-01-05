// Spreadable - Automated Market-Making Bot for Polymarket
// Main entry point with async runtime and service orchestration

mod config;
mod database;
mod error;
mod logging;
mod types;

use config::AppConfig;
use database::DatabaseClient;
use error::{Result, SpreadableError};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let args = parse_args();

    // Load configuration
    let config = match AppConfig::load(&args.config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {e}");
            process::exit(1);
        }
    };

    // Initialize logging
    if let Err(e) = logging::init_logging(&config.logging) {
        eprintln!("Failed to initialize logging: {e}");
        process::exit(1);
    }

    info!("Spreadable Market-Making Bot v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration loaded from: {}", args.config_path.display());
    info!(
        "Paper trading mode: {}",
        if config.risk.paper_trading_mode {
            "ENABLED"
        } else {
            "DISABLED"
        }
    );

    // Validate configuration mode only
    if args.validate_only {
        info!("Configuration validation successful");
        info!("Paper trading mode: {}", config.risk.paper_trading_mode);
        info!("Active strategy: {}", config.strategy.active);
        info!(
            "Enabled markets: {}",
            if config.markets.enabled.is_empty() {
                "none (add via config)".to_string()
            } else {
                config.markets.enabled.len().to_string()
            }
        );
        return;
    }

    // Run the application
    if let Err(e) = run(config).await {
        error!("Application error: {}", e);
        process::exit(1);
    }
}

async fn run(config: AppConfig) -> Result<()> {
    info!("Starting Spreadable Market-Making Bot...");

    // Connect to database
    info!("Connecting to database...");
    let db_client = DatabaseClient::new(&config.database).await?;

    // Run database migrations
    db_client.run_migrations().await?;

    // Verify database connectivity
    db_client.health_check().await?;
    info!("Database connection established");

    // TODO: Initialize remaining components in Phase 3:
    // - WebSocket client for market data
    // - Strategy engine
    // - Order management system
    // - Risk manager
    // - Monitoring/metrics server

    info!("All systems initialized successfully");
    info!("Spreadable is running...");

    // Keep the application running
    // In Phase 3, this will be replaced with actual service loops
    tokio::signal::ctrl_c().await.map_err(|e| {
        SpreadableError::Internal(format!("Failed to wait for ctrl-c: {e}"))
    })?;

    info!("Shutdown signal received, cleaning up...");
    db_client.close().await;
    info!("Shutdown complete");

    Ok(())
}

struct Args {
    config_path: PathBuf,
    validate_only: bool,
}

fn parse_args() -> Args {
    let mut config_path = PathBuf::from("config/development.toml");
    let mut validate_only = false;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    config_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --config requires a path argument");
                    print_usage();
                    process::exit(1);
                }
            }
            "--validate" => {
                validate_only = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                print_usage();
                process::exit(1);
            }
        }
    }

    Args {
        config_path,
        validate_only,
    }
}

fn print_usage() {
    println!("Spreadable - Automated Market-Making Bot for Polymarket");
    println!();
    println!("USAGE:");
    println!("    spreadable [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --config, -c <PATH>    Path to configuration file");
    println!("                          (default: config/development.toml)");
    println!("    --validate            Validate configuration and exit");
    println!("    --help, -h            Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    spreadable --config config/production.toml");
    println!("    spreadable --validate");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_set() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        assert_eq!(version, "0.1.0");
    }
}
