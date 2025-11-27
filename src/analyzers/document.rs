// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Document analyzer for office documents and text files

use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, info, warn};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for document files
pub struct DocumentAnalyzer;

impl DocumentAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Extract text from plain text files
    fn extract_text_file(path: &Path) -> Result<String> {
        Ok(std::fs::read_to_string(path)?)
    }

    /// Extract text from XLSX/XLS using calamine
    fn extract_spreadsheet(path: &Path) -> Result<String> {
        use calamine::{Reader, open_workbook_auto};

        let mut workbook = open_workbook_auto(path)
            .map_err(|e| PanoptesError::Analysis(format!("Failed to open spreadsheet: {}", e)))?;

        let mut text = String::new();

        // Get sheet names
        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        text.push_str(&format!("Sheets: {}\n", sheet_names.join(", ")));

        // Read first sheet
        if let Some(sheet_name) = sheet_names.first() {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                // Get first 20 rows
                for (i, row) in range.rows().enumerate() {
                    if i >= 20 {
                        text.push_str("...\n");
                        break;
                    }
                    let row_text: Vec<String> = row.iter()
                        .map(|c| c.to_string())
                        .collect();
                    text.push_str(&row_text.join("\t"));
                    text.push('\n');
                }
            }
        }

        Ok(text)
    }

    /// Extract text from DOCX (simple XML parsing)
    fn extract_docx(path: &Path) -> Result<String> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| PanoptesError::Analysis(format!("Failed to open DOCX: {}", e)))?;

        // DOCX stores content in word/document.xml
        let mut document_xml = match archive.by_name("word/document.xml") {
            Ok(file) => file,
            Err(_) => return Err(PanoptesError::Analysis("No document.xml found".to_string())),
        };

        let mut content = String::new();
        std::io::Read::read_to_string(&mut document_xml, &mut content)?;

        // Simple XML text extraction
        let mut text = String::new();
        let mut in_text = false;
        let mut current = String::new();

        for c in content.chars() {
            match c {
                '<' => {
                    if in_text && !current.is_empty() {
                        text.push_str(&current);
                        text.push(' ');
                        current.clear();
                    }
                    in_text = false;
                }
                '>' => {
                    // Check if this is a text tag
                    if current.contains("w:t") && !current.contains('/') {
                        in_text = true;
                    }
                    current.clear();
                }
                _ => {
                    if in_text {
                        text.push(c);
                    } else {
                        current.push(c);
                    }
                }
            }
        }

        Ok(text)
    }

    /// Extract content based on file type
    fn extract_content(path: &Path) -> Result<String> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "txt" | "md" | "markdown" | "rst" | "adoc" | "asciidoc" => Self::extract_text_file(path),
            "xlsx" | "xls" | "ods" | "csv" => Self::extract_spreadsheet(path),
            "docx" => Self::extract_docx(path),
            _ => Err(PanoptesError::UnsupportedFileType(ext)),
        }
    }
}

impl Default for DocumentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for DocumentAnalyzer {
    fn name(&self) -> &'static str {
        "document"
    }

    fn supported_extensions(&self) -> &[&str] {
        &[
            "txt", "md", "markdown", "rst", "adoc", "asciidoc",
            "docx", "doc", "odt", "rtf",
            "xlsx", "xls", "ods", "csv",
            "pptx", "ppt", "odp",
            "json", "yaml", "yml", "toml", "xml"
        ]
    }

    fn priority(&self) -> u8 {
        50
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing document: {:?}", path);

        let file_hash = calculate_file_hash(path)?;

        let content = match Self::extract_content(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to extract content: {}", e);
                String::new()
            }
        };

        let content_preview = if content.len() > 2000 {
            format!("{}...", &content[..2000])
        } else {
            content.clone()
        };

        let line_count = content.lines().count();
        let word_count = content.split_whitespace().count();

        let metadata = serde_json::json!({
            "line_count": line_count,
            "word_count": word_count,
            "char_count": content.len(),
        });

        // Use text model for summarization
        let client = OllamaClient::new(&config.ai_engine.url);
        let prompt = format!(
            "{}\n\nDocument content:\n{}",
            config.prompts.document,
            content_preview
        );

        let suggested_name = if !content.is_empty() {
            match client.generate(&config.ai_engine.models.text, &prompt).await {
                Ok(response) => {
                    let name = clean_filename(&response);
                    if name.is_empty() || name.len() < 3 {
                        // Fallback: use first line or file stem
                        content.lines().next()
                            .map(|l| clean_filename(l))
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| {
                                path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .map(|s| clean_filename(s))
                                    .unwrap_or_else(|| "document".to_string())
                            })
                    } else {
                        name
                    }
                }
                Err(e) => {
                    warn!("LLM failed: {}", e);
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| clean_filename(s))
                        .unwrap_or_else(|| "document".to_string())
                }
            }
        } else {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| clean_filename(s))
                .unwrap_or_else(|| "document".to_string())
        };

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");
        let category = infer_category(&suggested_name, extension);
        let tags = extract_tags(&suggested_name, &metadata);

        let confidence = if content.len() > 100 { 0.75 } else { 0.50 };

        Ok(AnalysisResult {
            suggested_name,
            confidence,
            category,
            tags,
            file_hash,
            metadata,
        })
    }
}
