// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Panoptes Web Dashboard
//!
//! Standalone web server for the Panoptes dashboard interface.

use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error};

use panoptes::config::AppConfig;
use panoptes::db::Database;
use panoptes::Result;

#[derive(Parser, Debug)]
#[command(name = "panoptes-web")]
#[command(author = "Jonathan D. A. Jewell <hyperpolymath>")]
#[command(version = "3.0.0")]
#[command(about = "Panoptes Web Dashboard Server")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,

    /// Host to bind to
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// Port to listen on
    #[arg(short, long)]
    port: Option<u16>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Open browser automatically
    #[arg(long)]
    open: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    info!("Panoptes Web Dashboard v3.0.0");

    // Load config
    let mut config = AppConfig::load(&args.config)?;

    // Apply CLI overrides
    if let Some(host) = args.host {
        config.web.host = host;
    }
    if let Some(port) = args.port {
        config.web.port = port;
    }

    // Initialize database
    let db = Database::open(&config.database.path)?;
    info!("Database: {}", config.database.path);

    let addr = format!("{}:{}", config.web.host, config.web.port);
    info!("Starting web server at http://{}", addr);

    // Open browser if requested
    if args.open {
        let url = format!("http://{}", addr);
        if let Err(e) = open_browser(&url) {
            error!("Failed to open browser: {}", e);
        }
    }

    // Start web server
    // Import the web module's start function
    panoptes::web::start_server(config, db).await
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()?;
    }
    Ok(())
}
