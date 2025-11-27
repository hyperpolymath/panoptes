// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! File analyzers for different content types

pub mod archive;
pub mod audio;
pub mod code;
pub mod document;
pub mod image;
pub mod pdf;
pub mod video;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::{AppConfig, Result};

/// Result of file analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Suggested filename (without extension)
    pub suggested_name: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Suggested category
    pub category: Option<String>,
    /// Suggested tags
    pub tags: Vec<String>,
    /// File hash for deduplication
    pub file_hash: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Trait for file analyzers
#[async_trait]
pub trait FileAnalyzer: Send + Sync {
    /// Name of this analyzer
    fn name(&self) -> &'static str;

    /// File extensions this analyzer handles
    fn supported_extensions(&self) -> &[&str];

    /// Check if this analyzer can handle a file
    fn can_handle(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.supported_extensions().iter().any(|e| e.eq_ignore_ascii_case(ext))
        } else {
            false
        }
    }

    /// Analyze a file and return suggestions
    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult>;

    /// Priority (higher = preferred when multiple analyzers match)
    fn priority(&self) -> u8 {
        50
    }
}

/// Registry of all file analyzers
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn FileAnalyzer>>,
}

impl AnalyzerRegistry {
    /// Create a new registry with default analyzers
    pub fn new(config: &AppConfig) -> Self {
        let mut registry = Self {
            analyzers: Vec::new(),
        };

        // Register analyzers based on config
        if config.analyzers.image.enabled {
            registry.register(Box::new(image::ImageAnalyzer::new()));
        }
        if config.analyzers.pdf.enabled {
            registry.register(Box::new(pdf::PdfAnalyzer::new()));
        }
        if config.analyzers.audio.enabled {
            registry.register(Box::new(audio::AudioAnalyzer::new()));
        }
        if config.analyzers.video.enabled {
            registry.register(Box::new(video::VideoAnalyzer::new()));
        }
        if config.analyzers.code.enabled {
            registry.register(Box::new(code::CodeAnalyzer::new()));
        }

        // Always register these
        registry.register(Box::new(document::DocumentAnalyzer::new()));
        registry.register(Box::new(archive::ArchiveAnalyzer::new()));

        registry
    }

    /// Register a new analyzer
    pub fn register(&mut self, analyzer: Box<dyn FileAnalyzer>) {
        self.analyzers.push(analyzer);
        self.analyzers.sort_by_key(|a| std::cmp::Reverse(a.priority()));
    }

    /// Find the best analyzer for a file
    pub fn find_analyzer(&self, path: &Path) -> Option<&dyn FileAnalyzer> {
        self.analyzers.iter()
            .find(|a| a.can_handle(path))
            .map(|a| a.as_ref())
    }

    /// Get all registered analyzers
    pub fn analyzers(&self) -> &[Box<dyn FileAnalyzer>] {
        &self.analyzers
    }

    /// Get number of registered analyzers
    pub fn len(&self) -> usize {
        self.analyzers.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.analyzers.is_empty()
    }

    /// Get analyzer names
    pub fn analyzer_names(&self) -> Vec<&'static str> {
        self.analyzers.iter().map(|a| a.name()).collect()
    }
}

impl Clone for AnalyzerRegistry {
    fn clone(&self) -> Self {
        // Can't clone Box<dyn FileAnalyzer>, so recreate with defaults
        Self {
            analyzers: Vec::new(),
        }
    }
}

/// Calculate file hash for deduplication
pub fn calculate_file_hash(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let hash = blake3::hash(&data);
    Ok(hash.to_hex().to_string())
}

/// Clean and sanitize a suggested filename
pub fn clean_filename(raw: &str) -> String {
    let mut clean = raw.trim().replace(['\n', '\r'], "");

    // Remove common chat prefixes
    if let Some(idx) = clean.find(':') {
        if idx < 30 {
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

/// Infer category from filename and content
pub fn infer_category(name: &str, extension: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    let ext_lower = extension.to_lowercase();

    // Category inference based on extension and name patterns
    match ext_lower.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "bmp" | "tiff" => {
            if name_lower.contains("screenshot") { Some("Screenshots") }
            else if name_lower.contains("photo") || name_lower.contains("img") { Some("Photos") }
            else if name_lower.contains("diagram") || name_lower.contains("chart") { Some("Diagrams") }
            else { Some("Images") }
        }
        "pdf" => {
            if name_lower.contains("invoice") || name_lower.contains("receipt") { Some("Finance") }
            else if name_lower.contains("resume") || name_lower.contains("cv") { Some("Career") }
            else if name_lower.contains("manual") || name_lower.contains("guide") { Some("Manuals") }
            else { Some("Documents") }
        }
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => {
            if name_lower.contains("podcast") { Some("Podcasts") }
            else if name_lower.contains("voice") || name_lower.contains("recording") { Some("Recordings") }
            else { Some("Music") }
        }
        "mp4" | "mkv" | "webm" | "avi" | "mov" => {
            if name_lower.contains("tutorial") || name_lower.contains("lesson") { Some("Tutorials") }
            else if name_lower.contains("screen") { Some("Screen Recordings") }
            else { Some("Videos") }
        }
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" => Some("Code"),
        "zip" | "tar" | "gz" | "7z" | "rar" => Some("Archives"),
        "doc" | "docx" | "odt" | "txt" | "md" => Some("Documents"),
        "xls" | "xlsx" | "csv" | "ods" => Some("Spreadsheets"),
        "ppt" | "pptx" | "odp" => Some("Presentations"),
        _ => None
    }.map(String::from)
}

/// Extract tags from analysis metadata
pub fn extract_tags(name: &str, metadata: &serde_json::Value) -> Vec<String> {
    let mut tags = Vec::new();

    // Extract words from name as potential tags
    for word in name.split('_') {
        if word.len() >= 3 && !is_stop_word(word) {
            tags.push(word.to_string());
        }
    }

    // Extract tags from metadata if present
    if let Some(obj) = metadata.as_object() {
        if let Some(meta_tags) = obj.get("tags").and_then(|t| t.as_array()) {
            for tag in meta_tags {
                if let Some(s) = tag.as_str() {
                    tags.push(s.to_string());
                }
            }
        }
    }

    // Deduplicate
    tags.sort();
    tags.dedup();
    tags
}

fn is_stop_word(word: &str) -> bool {
    matches!(word.to_lowercase().as_str(),
        "the" | "and" | "for" | "with" | "from" | "this" | "that" | "are" | "was" | "were"
    )
}
