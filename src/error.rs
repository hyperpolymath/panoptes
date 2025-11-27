// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Error types for Panoptes

use thiserror::Error;

/// Result type alias for Panoptes operations
pub type Result<T> = std::result::Result<T, PanoptesError>;

/// Panoptes error types
#[derive(Error, Debug)]
pub enum PanoptesError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Watch error: {0}")]
    Watch(#[from] notify::Error),

    #[error("Ollama not available: {0}")]
    OllamaUnavailable(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("PDF error: {0}")]
    Pdf(String),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Audio error: {0}")]
    Audio(String),
}
