// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Panoptes: Local AI File Scanner & Renamer
//!
//! A comprehensive file analysis and organization system using local AI models.
//! Version 3.0 - Full plugin architecture with web UI and database support.

use chrono::Local;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use panoptes::analyzers::{AnalyzerRegistry, AnalysisResult};
use panoptes::config::AppConfig;
use panoptes::db::Database;
use panoptes::history::{History, create_entry};
use panoptes::ollama::OllamaClient;
use panoptes::watcher::{FileWatcher, WatchEvent, should_process, wait_for_stable};
use panoptes::{PanoptesError, Result};

/// Panoptes CLI - Local AI File Scanner & Renamer
#[derive(Parser, Debug)]
#[command(name = "panoptes")]
#[command(author = "Jonathan D. A. Jewell <hyperpolymath>")]
#[command(version = "3.0.0")]
#[command(about = "Local AI-powered file scanner and renamer", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to configuration file (JSON format)
    #[arg(short, long, default_value = "config.json", global = true)]
    config: PathBuf,

    /// Enable verbose logging (debug level)
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable trace logging (most verbose)
    #[arg(long, global = true)]
    trace: bool,

    /// Output format for results
    #[arg(long, global = true, default_value = "text", value_parser = ["text", "json", "jsonl"])]
    format: String,

    /// Suppress non-essential output (quiet mode)
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Watch directories for new files and process them
    Watch {
        /// Directories to watch (overrides config)
        #[arg(short, long)]
        dir: Vec<PathBuf>,

        /// Dry run mode (don't actually rename files)
        #[arg(long)]
        dry_run: bool,

        /// Skip Ollama health check on startup
        #[arg(long)]
        skip_health_check: bool,

        /// Process existing files in directories on startup
        #[arg(long)]
        process_existing: bool,

        /// Enable recursive directory watching
        #[arg(short, long)]
        recursive: bool,
    },

    /// Analyze a single file or directory
    Analyze {
        /// File or directory to analyze
        path: PathBuf,

        /// Dry run mode (show suggestions without renaming)
        #[arg(long)]
        dry_run: bool,

        /// Recursive analysis for directories
        #[arg(short, long)]
        recursive: bool,

        /// Minimum confidence threshold (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        min_confidence: f64,
    },

    /// Database operations
    Db {
        #[command(subcommand)]
        action: DbCommands,
    },

    /// History and undo operations
    History {
        #[command(subcommand)]
        action: HistoryCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Show AI engine status
    Status {
        /// Check specific model availability
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Initialize a new Panoptes project
    Init {
        /// Directory to initialize (default: current)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DbCommands {
    /// Show database statistics
    Stats,

    /// List all tags
    Tags {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Maximum number to show
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// List all categories
    Categories,

    /// Search files in database
    Search {
        /// Search query
        query: String,

        /// Search in tags only
        #[arg(long)]
        tags_only: bool,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Export database to JSON
    Export {
        /// Output file
        output: PathBuf,
    },

    /// Vacuum database (reclaim space)
    Vacuum,
}

#[derive(Subcommand, Debug)]
enum HistoryCommands {
    /// List recent history entries
    List {
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// Undo recent renames
    Undo {
        /// Number of renames to undo
        #[arg(short, long, default_value = "1")]
        count: usize,

        /// Dry run (show what would be undone)
        #[arg(long)]
        dry_run: bool,
    },

    /// Clear all history
    Clear {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Generate default configuration file
    Generate {
        /// Output file path
        #[arg(short, long, default_value = "config.json")]
        output: PathBuf,

        /// Include all options with defaults
        #[arg(long)]
        full: bool,
    },

    /// Validate configuration file
    Validate,

    /// Edit configuration interactively
    Edit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.trace {
        "trace"
    } else if cli.verbose {
        "debug"
    } else if cli.quiet {
        "warn"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    if !cli.quiet {
        info!("Panoptes v3.0.0 - Local AI File Scanner");
    }

    // Load configuration
    let config = AppConfig::load(&cli.config)?;

    match cli.command {
        Some(Commands::Watch { dir, dry_run, skip_health_check, process_existing, recursive: _ }) => {
            run_watch(config, dir, dry_run, skip_health_check, process_existing).await
        }
        Some(Commands::Analyze { path, dry_run, recursive, min_confidence }) => {
            run_analyze(config, path, dry_run, recursive, min_confidence, &cli.format).await
        }
        Some(Commands::Db { action }) => {
            run_db_command(config, action).await
        }
        Some(Commands::History { action }) => {
            run_history_command(config, action).await
        }
        Some(Commands::Config { action }) => {
            run_config_command(config, action, &cli.config).await
        }
        Some(Commands::Status { model }) => {
            run_status(config, model).await
        }
        Some(Commands::Init { dir, force }) => {
            run_init(dir, force).await
        }
        None => {
            // Default: run watch mode
            run_watch(config, vec![], false, false, false).await
        }
    }
}

/// Run the watch mode (main scanner loop)
async fn run_watch(
    config: AppConfig,
    dir_overrides: Vec<PathBuf>,
    dry_run: bool,
    skip_health_check: bool,
    process_existing: bool,
) -> Result<()> {
    let watch_paths: Vec<PathBuf> = if dir_overrides.is_empty() {
        config.watch_paths.iter().map(PathBuf::from).collect()
    } else {
        dir_overrides
    };

    info!("Watch directories: {:?}", watch_paths);

    if dry_run {
        warn!("DRY RUN MODE - files will not be renamed");
    }

    // Initialize components
    let client = OllamaClient::new(&config.ai_engine.url);

    // Health check
    if !skip_health_check {
        info!("Checking Ollama availability...");
        match client.health_check().await {
            Ok(()) => info!("Ollama is running"),
            Err(e) => {
                return Err(PanoptesError::OllamaUnavailable(format!(
                    "Failed to connect to Ollama: {}. Try: just start-engine", e
                )))
            }
        }

        // Check vision model
        let models = client.list_models().await?;
        let vision_model = &config.ai_engine.models.vision;
        if !models.iter().any(|m| m.starts_with(vision_model)) {
            warn!("Vision model '{}' not found. Available: {:?}", vision_model, models);
            warn!("Try: just pull-model");
        } else {
            info!("Vision model '{}' available", vision_model);
        }
    } else {
        warn!("Skipping Ollama health check");
    }

    // Initialize database
    let db = Database::open(&config.database.path)?;
    info!("Database initialized: {}", config.database.path);

    // Initialize history
    let history_path = PathBuf::from("panoptes_history.jsonl");
    let history = History::new(history_path.clone());

    // Initialize analyzer registry
    let registry = AnalyzerRegistry::new(&config);
    info!("Loaded {} analyzers: {:?}", registry.len(), registry.analyzer_names());

    // Setup file watcher
    let mut watcher = FileWatcher::new()?;
    for path in &watch_paths {
        watcher.watch(path)?;
    }

    // Process existing files if requested
    if process_existing {
        info!("Processing existing files...");
        for dir in &watch_paths {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && should_process(&path) {
                        if let Err(e) = process_file(
                            path.clone(),
                            &config,
                            &registry,
                            &db,
                            &history,
                            dry_run,
                        ).await {
                            error!("Failed to process {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    // Setup graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => info!("Received Ctrl+C, shutting down..."),
            _ = terminate => info!("Received SIGTERM, shutting down..."),
        }

        let _ = shutdown_tx.send(true);
    });

    info!("Scanner active. Press Ctrl+C to stop.");
    info!("Waiting for files...");

    // Main event loop
    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        if let Some(event) = watcher.next_event(Duration::from_millis(100)) {
            match event {
                WatchEvent::FileCreated(path) => {
                    if should_process(&path) {
                        let config_clone = config.clone();
                        let db_clone = db.clone();
                        let history_clone = History::new(history_path.clone());
                        let registry_clone = registry.clone();

                        tokio::spawn(async move {
                            // Wait for file stability
                            if !wait_for_stable(&path, Duration::from_secs(10)).await {
                                debug!("File disappeared during stability check: {:?}", path);
                                return;
                            }

                            if let Err(e) = process_file(
                                path.clone(),
                                &config_clone,
                                &registry_clone,
                                &db_clone,
                                &history_clone,
                                dry_run,
                            ).await {
                                error!("Failed to process {:?}: {}", path, e);
                            }
                        });
                    }
                }
                WatchEvent::Error(e) => {
                    warn!("Watch error: {}", e);
                }
                _ => {}
            }
        }
    }

    info!("Panoptes stopped.");
    Ok(())
}

/// Process a single file
async fn process_file(
    path: PathBuf,
    config: &AppConfig,
    registry: &AnalyzerRegistry,
    db: &Database,
    history: &History,
    dry_run: bool,
) -> Result<()> {
    info!("Analyzing: {:?}", path);

    // Find appropriate analyzer
    let analyzer = match registry.find_analyzer(&path) {
        Some(a) => a,
        None => {
            debug!("No analyzer for: {:?}", path);
            return Ok(());
        }
    };

    info!("Using analyzer: {}", analyzer.name());

    // Run analysis
    let result = analyzer.analyze(&path, config).await?;

    info!("Suggestion: {} (confidence: {:.0}%)", result.suggested_name, result.confidence * 100.0);

    if let Some(ref cat) = result.category {
        info!("Category: {}", cat);
    }
    if !result.tags.is_empty() {
        info!("Tags: {:?}", result.tags);
    }

    // Store in database
    let file_id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = db.insert_file(
        &file_id,
        path.to_str().unwrap_or(""),
        &result.suggested_name,
        &result.file_hash,
        result.category.as_deref(),
        result.confidence,
        &result.metadata,
    ) {
        warn!("Failed to store in database: {}", e);
    }

    // Add tags
    for tag in &result.tags {
        if let Err(e) = db.add_tag(&file_id, tag, result.category.as_deref()) {
            debug!("Failed to add tag '{}': {}", tag, e);
        }
    }

    // Rename file
    if result.confidence >= 0.5 {
        if dry_run {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            info!("DRY RUN: Would rename {:?} to {}.{}", path, result.suggested_name, ext);
        } else {
            rename_file(&path, &result, config, history)?;
        }
    } else {
        info!("Confidence too low ({:.0}%), skipping rename", result.confidence * 100.0);
    }

    Ok(())
}

/// Rename a file with the analysis result
fn rename_file(
    original: &Path,
    result: &AnalysisResult,
    config: &AppConfig,
    history: &History,
) -> Result<()> {
    let parent = original.parent()
        .ok_or_else(|| PanoptesError::Config("Cannot determine parent directory".to_string()))?;

    let ext = original.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut final_name = result.suggested_name.clone();

    if config.rules.date_prefix {
        let date = Local::now().format("%Y-%m-%d").to_string();
        final_name = format!("{}_{}", date, final_name);
    }

    // Truncate to max length
    if final_name.len() > config.rules.max_length {
        final_name.truncate(config.rules.max_length);
        final_name = final_name.trim_end_matches('_').to_string();
    }

    let new_path = parent.join(format!("{}.{}", final_name, ext));

    // Handle filename collision
    let new_path = if new_path.exists() {
        let timestamp = Local::now().format("%H%M%S").to_string();
        parent.join(format!("{}_{}.{}", final_name, timestamp, ext))
    } else {
        new_path
    };

    // Write history entry
    let entry = create_entry(
        uuid::Uuid::new_v4().to_string(),
        original.to_path_buf(),
        new_path.clone(),
        result.suggested_name.clone(),
        result.category.clone(),
        result.tags.clone(),
        result.file_hash.clone(),
    );
    history.append(&entry)?;

    // Perform rename
    std::fs::rename(original, &new_path)?;
    info!("Renamed to: {:?}", new_path);

    Ok(())
}

/// Run single file/directory analysis
async fn run_analyze(
    config: AppConfig,
    path: PathBuf,
    dry_run: bool,
    recursive: bool,
    min_confidence: f64,
    format: &str,
) -> Result<()> {
    let registry = AnalyzerRegistry::new(&config);
    let history = History::new(PathBuf::from("panoptes_history.jsonl"));

    let files: Vec<PathBuf> = if path.is_dir() {
        if recursive {
            walkdir(&path)
        } else {
            std::fs::read_dir(&path)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        }
    } else {
        vec![path]
    };

    let mut results = Vec::new();

    for file in files {
        if !should_process(&file) {
            continue;
        }

        if let Some(analyzer) = registry.find_analyzer(&file) {
            match analyzer.analyze(&file, &config).await {
                Ok(result) => {
                    if result.confidence >= min_confidence {
                        if format == "text" {
                            println!("{}: {} ({:.0}%)",
                                file.display(),
                                result.suggested_name,
                                result.confidence * 100.0
                            );
                        }

                        if !dry_run && result.confidence >= 0.5 {
                            rename_file(&file, &result, &config, &history)?;
                        }

                        results.push((file, result));
                    }
                }
                Err(e) => {
                    if format == "text" {
                        eprintln!("Error analyzing {}: {}", file.display(), e);
                    }
                }
            }
        }
    }

    // Output results in requested format
    match format {
        "json" => {
            let output: Vec<serde_json::Value> = results.iter().map(|(p, r)| {
                serde_json::json!({
                    "path": p.to_string_lossy(),
                    "suggested_name": r.suggested_name,
                    "confidence": r.confidence,
                    "category": r.category,
                    "tags": r.tags,
                })
            }).collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "jsonl" => {
            for (p, r) in &results {
                let line = serde_json::json!({
                    "path": p.to_string_lossy(),
                    "suggested_name": r.suggested_name,
                    "confidence": r.confidence,
                    "category": r.category,
                    "tags": r.tags,
                });
                println!("{}", serde_json::to_string(&line)?);
            }
        }
        _ => {}
    }

    if !results.is_empty() && format == "text" {
        println!("\nAnalyzed {} files", results.len());
    }

    Ok(())
}

/// Walk directory recursively
fn walkdir(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                files.extend(walkdir(&p));
            } else if p.is_file() {
                files.push(p);
            }
        }
    }

    files
}

/// Run database commands
async fn run_db_command(config: AppConfig, action: DbCommands) -> Result<()> {
    let db = Database::open(&config.database.path)?;

    match action {
        DbCommands::Stats => {
            let stats = db.get_stats()?;
            println!("Database Statistics:");
            println!("  Files: {}", stats.file_count);
            println!("  Tags: {}", stats.tag_count);
            println!("  Categories: {}", stats.category_count);
        }
        DbCommands::Tags { category, limit } => {
            let tags = db.get_all_tags()?;
            println!("Tags:");
            for (i, tag) in tags.iter().enumerate() {
                if i >= limit { break; }
                if let Some(ref cat) = category {
                    if tag.category.as_ref() == Some(cat) {
                        println!("  {} ({})", tag.name, tag.category.as_deref().unwrap_or("-"));
                    }
                } else {
                    println!("  {} ({})", tag.name, tag.category.as_deref().unwrap_or("-"));
                }
            }
        }
        DbCommands::Categories => {
            let categories = db.get_all_categories()?;
            println!("Categories:");
            for cat in categories {
                println!("  {} - {} ({} files)", cat.name, cat.description.unwrap_or_default(), cat.file_count);
            }
        }
        DbCommands::Search { query, tags_only: _, limit } => {
            let results = db.search_files(&query, limit)?;
            println!("Search results for '{}':", query);
            for file in results {
                println!("  {}: {}", file.id, file.suggested_name);
            }
        }
        DbCommands::Export { output } => {
            let files = db.get_all_files()?;
            let json = serde_json::to_string_pretty(&files)?;
            std::fs::write(&output, json)?;
            println!("Exported {} files to {:?}", files.len(), output);
        }
        DbCommands::Vacuum => {
            db.vacuum()?;
            println!("Database vacuumed successfully");
        }
    }

    Ok(())
}

/// Run history commands
async fn run_history_command(config: AppConfig, action: HistoryCommands) -> Result<()> {
    let history = History::new(PathBuf::from("panoptes_history.jsonl"));

    match action {
        HistoryCommands::List { count } => {
            let entries = history.get_recent(count)?;
            println!("Recent history ({} entries):", entries.len());
            for entry in entries {
                let status = if entry.undone { "[UNDONE]" } else { "" };
                println!("  {} {} -> {} {}",
                    entry.timestamp.format("%Y-%m-%d %H:%M"),
                    entry.original_path.display(),
                    entry.new_path.display(),
                    status
                );
            }
        }
        HistoryCommands::Undo { count, dry_run } => {
            let entries = history.get_undoable()?;
            let to_undo: Vec<_> = entries.into_iter().rev().take(count).collect();

            if to_undo.is_empty() {
                println!("No renames to undo");
                return Ok(());
            }

            for entry in to_undo {
                if entry.new_path.exists() {
                    if dry_run {
                        println!("Would undo: {} -> {}",
                            entry.new_path.display(),
                            entry.original_path.display()
                        );
                    } else {
                        std::fs::rename(&entry.new_path, &entry.original_path)?;
                        history.mark_undone(&entry.id)?;
                        println!("Undone: {} -> {}",
                            entry.new_path.display(),
                            entry.original_path.display()
                        );
                    }
                } else {
                    warn!("File not found (may have been moved/deleted): {:?}", entry.new_path);
                }
            }
        }
        HistoryCommands::Clear { force } => {
            if !force {
                eprintln!("Use --force to confirm clearing history");
                return Ok(());
            }
            history.clear()?;
            println!("History cleared");
        }
    }

    Ok(())
}

/// Run config commands
async fn run_config_command(config: AppConfig, action: ConfigCommands, config_path: &Path) -> Result<()> {
    match action {
        ConfigCommands::Show => {
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
        }
        ConfigCommands::Generate { output, full: _ } => {
            let default_config = AppConfig::default();
            default_config.save(&output)?;
            println!("Generated config at {:?}", output);
        }
        ConfigCommands::Validate => {
            println!("Configuration at {:?} is valid", config_path);
            println!("  Watch paths: {:?}", config.watch_paths);
            println!("  Vision model: {}", config.ai_engine.models.vision);
            println!("  Database: {}", config.database.path);
        }
        ConfigCommands::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            std::process::Command::new(editor)
                .arg(config_path)
                .status()?;
        }
    }

    Ok(())
}

/// Run status check
async fn run_status(config: AppConfig, model: Option<String>) -> Result<()> {
    let client = OllamaClient::new(&config.ai_engine.url);

    println!("Panoptes v3.0.0 Status");
    println!("======================");

    // Check Ollama
    match client.health_check().await {
        Ok(()) => println!("Ollama: Running"),
        Err(e) => println!("Ollama: Error - {}", e),
    }

    // List models
    match client.list_models().await {
        Ok(models) => {
            println!("\nAvailable models:");
            for m in &models {
                let marker = if Some(m.clone()) == model || m.starts_with(config.ai_engine.models.vision.as_str()) {
                    "→"
                } else {
                    " "
                };
                println!("  {} {}", marker, m);
            }
        }
        Err(e) => println!("  Error listing models: {}", e),
    }

    // Check database
    match Database::open(&config.database.path) {
        Ok(db) => {
            let stats = db.get_stats()?;
            println!("\nDatabase ({}):", config.database.path);
            println!("  Files: {}", stats.file_count);
            println!("  Tags: {}", stats.tag_count);
        }
        Err(e) => println!("\nDatabase: ✗ Error - {}", e),
    }

    println!("\nConfiguration:");
    println!("  Watch paths: {:?}", config.watch_paths);
    println!("  Vision model: {}", config.ai_engine.models.vision);
    println!("  Text model: {}", config.ai_engine.models.text);
    println!("  Code model: {}", config.ai_engine.models.code);

    Ok(())
}

/// Initialize a new Panoptes project
async fn run_init(dir: Option<PathBuf>, force: bool) -> Result<()> {
    let target = dir.unwrap_or_else(|| PathBuf::from("."));
    let config_path = target.join("config.json");

    if config_path.exists() && !force {
        return Err(PanoptesError::Config(
            "config.json already exists. Use --force to overwrite".to_string()
        ));
    }

    // Create directories
    let watch_dir = target.join("watch");
    std::fs::create_dir_all(&watch_dir)?;

    // Create default config
    let mut config = AppConfig::default();
    config.watch_paths = vec![watch_dir.to_string_lossy().to_string()];
    config.save(&config_path)?;

    println!("Panoptes initialized in {:?}", target);
    println!("\nCreated:");
    println!("  - config.json");
    println!("  - watch/");
    println!("\nNext steps:");
    println!("  1. Start Ollama: just start-engine");
    println!("  2. Start scanner: panoptes watch");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(["panoptes"]).unwrap();
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_watch_command() {
        let cli = Cli::try_parse_from([
            "panoptes", "watch", "--dry-run", "--dir", "/tmp/test"
        ]).unwrap();

        match cli.command {
            Some(Commands::Watch { dry_run, dir, .. }) => {
                assert!(dry_run);
                assert_eq!(dir, vec![PathBuf::from("/tmp/test")]);
            }
            _ => panic!("Expected Watch command"),
        }
    }

    #[test]
    fn test_cli_analyze_command() {
        let cli = Cli::try_parse_from([
            "panoptes", "analyze", "/tmp/file.jpg", "--dry-run"
        ]).unwrap();

        match cli.command {
            Some(Commands::Analyze { path, dry_run, .. }) => {
                assert!(dry_run);
                assert_eq!(path, PathBuf::from("/tmp/file.jpg"));
            }
            _ => panic!("Expected Analyze command"),
        }
    }
}
