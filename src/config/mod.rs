// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Configuration management for Panoptes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main application configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    /// Directories to watch
    pub watch_paths: Vec<String>,

    /// AI engine configuration
    pub ai_engine: EngineConfig,

    /// Naming rules
    pub rules: RuleConfig,

    /// Prompt templates
    pub prompts: PromptConfig,

    /// Analyzer-specific settings
    #[serde(default)]
    pub analyzers: AnalyzerConfig,

    /// Web UI settings
    #[serde(default)]
    pub web: WebConfig,

    /// Database settings
    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EngineConfig {
    pub url: String,
    pub models: ModelConfig,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_retries")]
    pub retries: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelConfig {
    pub vision: String,
    #[serde(default = "default_text_model")]
    pub text: String,
    #[serde(default = "default_code_model")]
    pub code: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RuleConfig {
    pub sanitize: bool,
    pub date_prefix: bool,
    pub max_length: usize,
    #[serde(default)]
    pub auto_categorize: bool,
    #[serde(default)]
    pub duplicate_detection: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PromptConfig {
    pub image: String,
    pub document: String,
    #[serde(default = "default_audio_prompt")]
    pub audio: String,
    #[serde(default = "default_video_prompt")]
    pub video: String,
    #[serde(default = "default_code_prompt")]
    pub code: String,
    #[serde(default = "default_archive_prompt")]
    pub archive: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AnalyzerConfig {
    #[serde(default)]
    pub image: ImageAnalyzerConfig,
    #[serde(default)]
    pub pdf: PdfAnalyzerConfig,
    #[serde(default)]
    pub audio: AudioAnalyzerConfig,
    #[serde(default)]
    pub video: VideoAnalyzerConfig,
    #[serde(default)]
    pub code: CodeAnalyzerConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageAnalyzerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub formats: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PdfAnalyzerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub extract_text: bool,
    #[serde(default)]
    pub rasterize_pages: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioAnalyzerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub use_metadata: bool,
    #[serde(default)]
    pub transcribe: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoAnalyzerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_keyframes")]
    pub keyframes: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CodeAnalyzerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub languages: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WebConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_web_host")]
    pub host: String,
    #[serde(default = "default_web_port")]
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

// Default value functions
fn default_timeout() -> u64 { 120 }
fn default_retries() -> u32 { 3 }
fn default_text_model() -> String { "llama3.2:3b".to_string() }
fn default_code_model() -> String { "deepseek-coder:1.3b".to_string() }
fn default_true() -> bool { true }
fn default_keyframes() -> u32 { 5 }
fn default_web_host() -> String { "127.0.0.1".to_string() }
fn default_web_port() -> u16 { 8080 }
fn default_db_path() -> String { "panoptes.db".to_string() }

fn default_audio_prompt() -> String {
    "Based on this audio metadata, suggest a descriptive filename (max 5 words). \
     Use snake_case. Return ONLY the filename.".to_string()
}

fn default_video_prompt() -> String {
    "Analyze these video keyframes and suggest a descriptive filename (max 5 words). \
     Use snake_case. Return ONLY the filename.".to_string()
}

fn default_code_prompt() -> String {
    "Analyze this code structure and suggest a descriptive filename (max 5 words). \
     Use snake_case. Return ONLY the filename.".to_string()
}

fn default_archive_prompt() -> String {
    "Based on these archive contents, suggest a descriptive filename (max 5 words). \
     Use snake_case. Return ONLY the filename.".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec!["./watch".to_string()],
            ai_engine: EngineConfig {
                url: "http://localhost:11434/api/generate".to_string(),
                models: ModelConfig {
                    vision: "moondream".to_string(),
                    text: default_text_model(),
                    code: default_code_model(),
                },
                timeout_secs: default_timeout(),
                retries: default_retries(),
            },
            rules: RuleConfig {
                sanitize: true,
                date_prefix: true,
                max_length: 50,
                auto_categorize: true,
                duplicate_detection: true,
            },
            prompts: PromptConfig {
                image: "Analyze this image and generate a concise, descriptive filename \
                        (max 5 words). Use snake_case. Do not include the file extension. \
                        Return ONLY the filename.".to_string(),
                document: "Summarize this document into a concise filename (max 5 words). \
                           Use snake_case. Return ONLY the filename.".to_string(),
                audio: default_audio_prompt(),
                video: default_video_prompt(),
                code: default_code_prompt(),
                archive: default_archive_prompt(),
            },
            analyzers: AnalyzerConfig::default(),
            web: WebConfig::default(),
            database: DatabaseConfig::default(),
        }
    }
}

impl Default for ImageAnalyzerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            formats: vec![
                "jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif",
                "heic", "heif", "avif", "svg"
            ].into_iter().map(String::from).collect(),
        }
    }
}

impl Default for PdfAnalyzerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            extract_text: true,
            rasterize_pages: 1,
        }
    }
}

impl Default for AudioAnalyzerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_metadata: true,
            transcribe: false,
        }
    }
}

impl Default for VideoAnalyzerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            keyframes: 5,
        }
    }
}

impl Default for CodeAnalyzerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            languages: vec!["rust", "python", "javascript", "typescript", "go", "java"]
                .into_iter().map(String::from).collect(),
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_web_host(),
            port: default_web_port(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

impl AppConfig {
    /// Load configuration from a JSON file
    pub fn load(path: &Path) -> crate::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Self = serde_json::from_str(&content)
                .map_err(|e| crate::PanoptesError::Config(format!("Failed to parse config: {}", e)))?;
            Ok(config)
        } else {
            tracing::info!("Config file not found at {:?}, using defaults", path);
            Ok(Self::default())
        }
    }

    /// Save configuration to a JSON file
    pub fn save(&self, path: &Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
