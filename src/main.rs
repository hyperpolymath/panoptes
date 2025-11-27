// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Panoptes: Local AI File Scanner & Renamer
//!
//! A Rust-based file system watcher that uses local AI (Moondream via Ollama)
//! to automatically rename images and documents based on their visual content.

use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use clap::Parser;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{fs, thread};
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Panoptes CLI arguments
#[derive(Parser, Debug)]
#[command(name = "panoptes")]
#[command(author = "Jonathan D. A. Jewell <hyperpolymath>")]
#[command(version = "1.0.0")]
#[command(about = "Local AI-powered file scanner and renamer", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.ncl")]
    config: PathBuf,

    /// Directory to watch (overrides config)
    #[arg(short, long)]
    watch: Option<PathBuf>,

    /// Ollama API URL (overrides config)
    #[arg(long)]
    api_url: Option<String>,

    /// AI model to use (overrides config)
    #[arg(short, long)]
    model: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Dry run mode (don't actually rename files)
    #[arg(long)]
    dry_run: bool,
}

/// Application configuration (mirrors Nickel schema)
#[derive(Debug, Deserialize, Clone)]
struct AppConfig {
    watch_path: String,
    ai_engine: EngineConfig,
    rules: RuleConfig,
    prompts: PromptConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct EngineConfig {
    url: String,
    model: String,
}

#[derive(Debug, Deserialize, Clone)]
struct RuleConfig {
    sanitize: bool,
    date_prefix: bool,
    max_length: usize,
}

#[derive(Debug, Deserialize, Clone)]
struct PromptConfig {
    image: String,
    document: String,
}

/// Ollama API request payload
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    images: Option<Vec<String>>,
}

/// Ollama API response
#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Panoptes error types
#[derive(Error, Debug)]
enum PanoptesError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Watch error: {0}")]
    Watch(#[from] notify::Error),
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watch_path: "/var/home/core/Downloads/scan_input".to_string(),
            ai_engine: EngineConfig {
                url: "http://localhost:11434/api/generate".to_string(),
                model: "moondream".to_string(),
            },
            rules: RuleConfig {
                sanitize: true,
                date_prefix: true,
                max_length: 50,
            },
            prompts: PromptConfig {
                image: "Analyze this image and generate a concise, descriptive filename \
                        (max 5 words). Use snake_case. Do not include the file extension. \
                        Return ONLY the filename."
                    .to_string(),
                document: "Summarize the header or title of this document text into a \
                           concise filename (max 5 words). Use snake_case. Return ONLY \
                           the filename."
                    .to_string(),
            },
        }
    }
}

fn load_config(path: &Path) -> Result<AppConfig, PanoptesError> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        // Parse Nickel-style JSON config (Nickel exports to JSON)
        // In production, use nickel-lang crate for native parsing
        serde_json::from_str(&content)
            .map_err(|e| PanoptesError::Config(format!("Failed to parse config: {}", e)))
    } else {
        info!("Config file not found, using defaults");
        Ok(AppConfig::default())
    }
}

fn apply_cli_overrides(mut config: AppConfig, args: &Args) -> AppConfig {
    if let Some(ref watch) = args.watch {
        config.watch_path = watch.to_string_lossy().to_string();
    }
    if let Some(ref url) = args.api_url {
        config.ai_engine.url = url.clone();
    }
    if let Some(ref model) = args.model {
        config.ai_engine.model = model.clone();
    }
    config
}

#[tokio::main]
async fn main() -> Result<(), PanoptesError> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    info!("Panoptes v1.0.0 - Local AI File Scanner");
    info!("Loading configuration from: {:?}", args.config);

    let config = load_config(&args.config)?;
    let config = apply_cli_overrides(config, &args);

    info!("Watching directory: {}", config.watch_path);
    info!("Using model: {}", config.ai_engine.model);

    if args.dry_run {
        warn!("DRY RUN MODE - files will not be renamed");
    }

    // Create watch directory if it doesn't exist
    let watch_path = Path::new(&config.watch_path);
    if !watch_path.exists() {
        fs::create_dir_all(watch_path)?;
        info!("Created watch directory: {:?}", watch_path);
    }

    let (tx, rx) = channel();
    let notify_config = Config::default().with_poll_interval(Duration::from_secs(2));
    let mut watcher: RecommendedWatcher = Watcher::new(tx, notify_config)?;

    watcher.watch(watch_path, RecursiveMode::NonRecursive)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    info!("Scanner active. Waiting for files...");

    for res in rx {
        match res {
            Ok(event) => {
                if let EventKind::Create(_) = event.kind {
                    for path in event.paths {
                        let config_clone = config.clone();
                        let client_clone = client.clone();
                        let dry_run = args.dry_run;
                        tokio::spawn(async move {
                            if let Err(e) =
                                process_file(path.clone(), config_clone, client_clone, dry_run)
                                    .await
                            {
                                error!("Failed to process {:?}: {}", path, e);
                            }
                        });
                    }
                }
            }
            Err(e) => warn!("Watch error: {:?}", e),
        }
    }

    Ok(())
}

async fn process_file(
    path: PathBuf,
    config: AppConfig,
    client: Client,
    dry_run: bool,
) -> Result<(), PanoptesError> {
    // Skip hidden files and temp files
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    if filename.starts_with('.') || filename.ends_with(".tmp") || filename.ends_with(".part") {
        debug!("Skipping temporary/hidden file: {:?}", path);
        return Ok(());
    }

    // Wait for file write completion (simple debounce)
    thread::sleep(Duration::from_secs(1));

    // Verify file still exists and is readable
    if !path.exists() {
        debug!("File no longer exists: {:?}", path);
        return Ok(());
    }

    info!("Analyzing: {:?}", path);

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let new_name = match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" => {
            analyze_image(&path, &config, &client).await?
        }
        "pdf" => {
            warn!("PDF analysis requires rasterization - skipping vision analysis");
            None
        }
        _ => {
            debug!("Unsupported file type: {}", extension);
            None
        }
    };

    if let Some(name) = new_name {
        if dry_run {
            info!("DRY RUN: Would rename {:?} to {}.{}", path, name, extension);
        } else {
            rename_file(&path, name, &config)?;
        }
    } else {
        debug!("No rename suggestion for: {:?}", path);
    }

    Ok(())
}

async fn analyze_image(
    path: &PathBuf,
    config: &AppConfig,
    client: &Client,
) -> Result<Option<String>, PanoptesError> {
    let image_data = fs::read(path)?;
    let encoded = general_purpose::STANDARD.encode(&image_data);

    let payload = OllamaRequest {
        model: config.ai_engine.model.clone(),
        prompt: config.prompts.image.clone(),
        stream: false,
        images: Some(vec![encoded]),
    };

    debug!("Sending request to Ollama API");

    let response = client
        .post(&config.ai_engine.url)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        warn!("API returned error status: {}", response.status());
        return Ok(None);
    }

    let json: OllamaResponse = response.json().await?;
    let cleaned = clean_filename(&json.response);

    if cleaned.is_empty() {
        warn!("AI returned empty filename suggestion");
        return Ok(None);
    }

    info!("AI suggested filename: {}", cleaned);
    Ok(Some(cleaned))
}

fn clean_filename(raw: &str) -> String {
    let mut clean = raw.trim().replace(['\n', '\r'], "");

    // Remove common chat prefixes
    if let Some(idx) = clean.find(':') {
        if idx < 20 {
            clean = clean[idx + 1..].trim().to_string();
        }
    }

    // Remove quotes
    clean = clean.trim_matches('"').trim_matches('\'').to_string();

    // Sanitize: keep only alphanumeric, underscore, hyphen
    clean = clean
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
        .collect::<String>();

    // Convert spaces to underscores and lowercase
    clean = clean.replace(' ', "_").to_lowercase();

    // Remove consecutive underscores
    while clean.contains("__") {
        clean = clean.replace("__", "_");
    }

    clean.trim_matches('_').to_string()
}

fn rename_file(original: &PathBuf, new_name: String, config: &AppConfig) -> Result<(), PanoptesError> {
    let parent = original.parent().ok_or_else(|| {
        PanoptesError::Config("Cannot determine parent directory".to_string())
    })?;

    let ext = original
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut final_name = new_name;

    if config.rules.date_prefix {
        let date = Local::now().format("%Y-%m-%d").to_string();
        final_name = format!("{}_{}", date, final_name);
    }

    // Truncate to max length
    if final_name.len() > config.rules.max_length {
        final_name.truncate(config.rules.max_length);
        // Clean up trailing underscore from truncation
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

    fs::rename(original, &new_path)?;
    info!("Renamed to: {:?}", new_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_filename_basic() {
        assert_eq!(clean_filename("hello world"), "hello_world");
        assert_eq!(clean_filename("  spaced  "), "spaced");
    }

    #[test]
    fn test_clean_filename_with_prefix() {
        assert_eq!(
            clean_filename("Here is your filename: sunset_beach"),
            "sunset_beach"
        );
    }

    #[test]
    fn test_clean_filename_special_chars() {
        assert_eq!(
            clean_filename("file@name#with$special!chars"),
            "filenamewithspecialchars"
        );
    }

    #[test]
    fn test_clean_filename_quotes() {
        assert_eq!(clean_filename("\"quoted_name\""), "quoted_name");
        assert_eq!(clean_filename("'single_quoted'"), "single_quoted");
    }

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.ai_engine.model, "moondream");
        assert!(config.rules.date_prefix);
        assert_eq!(config.rules.max_length, 50);
    }
}
