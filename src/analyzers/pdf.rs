// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! PDF document analyzer

use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, info, warn};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for PDF files
pub struct PdfAnalyzer;

impl PdfAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Extract text from PDF
    fn extract_text(path: &Path) -> Result<String> {
        let bytes = std::fs::read(path)?;
        pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| PanoptesError::Pdf(format!("Text extraction failed: {}", e)))
    }

    /// Get PDF metadata
    fn get_metadata(path: &Path) -> Result<serde_json::Value> {
        let bytes = std::fs::read(path)?;
        let doc = lopdf::Document::load_mem(&bytes)
            .map_err(|e| PanoptesError::Pdf(format!("Failed to load PDF: {}", e)))?;

        let page_count = doc.get_pages().len();

        // Try to extract metadata
        let mut metadata = serde_json::json!({
            "page_count": page_count,
        });

        // Extract document info if available
        if let Ok(info) = doc.trailer.get(b"Info") {
            if let Ok(info_ref) = info.as_reference() {
                if let Ok(info_dict) = doc.get_dictionary(info_ref) {
                    if let Ok(title) = info_dict.get(b"Title") {
                        if let Ok(title_bytes) = title.as_str() {
                            metadata["title"] = serde_json::Value::String(
                                String::from_utf8_lossy(title_bytes).to_string()
                            );
                        }
                    }
                    if let Ok(author) = info_dict.get(b"Author") {
                        if let Ok(author_bytes) = author.as_str() {
                            metadata["author"] = serde_json::Value::String(
                                String::from_utf8_lossy(author_bytes).to_string()
                            );
                        }
                    }
                    if let Ok(subject) = info_dict.get(b"Subject") {
                        if let Ok(subject_bytes) = subject.as_str() {
                            metadata["subject"] = serde_json::Value::String(
                                String::from_utf8_lossy(subject_bytes).to_string()
                            );
                        }
                    }
                }
            }
        }

        Ok(metadata)
    }
}

impl Default for PdfAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for PdfAnalyzer {
    fn name(&self) -> &'static str {
        "pdf"
    }

    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn priority(&self) -> u8 {
        90
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing PDF: {:?}", path);

        let file_hash = calculate_file_hash(path)?;
        let metadata = Self::get_metadata(path).unwrap_or_else(|_| serde_json::json!({}));

        // Try to use document title first
        if let Some(title) = metadata.get("title").and_then(|t| t.as_str()) {
            if !title.is_empty() && title.len() < 100 {
                let suggested_name = clean_filename(title);
                if !suggested_name.is_empty() {
                    let category = infer_category(&suggested_name, "pdf");
                    let tags = extract_tags(&suggested_name, &metadata);

                    return Ok(AnalysisResult {
                        suggested_name,
                        confidence: 0.95, // High confidence from metadata
                        category,
                        tags,
                        file_hash,
                        metadata,
                    });
                }
            }
        }

        // Extract text and use LLM for summarization
        let text = Self::extract_text(path)?;
        let text_preview = if text.len() > 2000 {
            format!("{}...", &text[..2000])
        } else {
            text.clone()
        };

        // Use text model for summarization
        let client = OllamaClient::new(&config.ai_engine.url);
        let prompt = format!(
            "{}\n\nDocument text:\n{}",
            config.prompts.document,
            text_preview
        );

        let suggested_name = match client.generate(&config.ai_engine.models.text, &prompt).await {
            Ok(response) => clean_filename(&response),
            Err(e) => {
                warn!("LLM failed for PDF: {}", e);
                // Fallback: use page count
                let page_count = metadata.get("page_count")
                    .and_then(|p| p.as_u64())
                    .unwrap_or(1);
                format!("document_{}pages", page_count)
            }
        };

        let category = infer_category(&suggested_name, "pdf");
        let tags = extract_tags(&suggested_name, &metadata);

        Ok(AnalysisResult {
            suggested_name,
            confidence: 0.75,
            category,
            tags,
            file_hash,
            metadata,
        })
    }
}
