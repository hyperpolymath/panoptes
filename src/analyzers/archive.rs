// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Archive file analyzer

use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, info, warn};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for archive files
pub struct ArchiveAnalyzer;

impl ArchiveAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// List contents of a ZIP file
    fn list_zip(path: &Path) -> Result<ArchiveContents> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| PanoptesError::Archive(format!("Failed to open ZIP: {}", e)))?;

        let mut contents = ArchiveContents::default();
        contents.file_count = archive.len();

        for i in 0..archive.len().min(50) {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                contents.total_size += file.size();

                // Categorize file
                if let Some(ext) = Path::new(&name).extension().and_then(|e| e.to_str()) {
                    *contents.extensions.entry(ext.to_lowercase()).or_insert(0) += 1;
                }

                if contents.sample_files.len() < 10 {
                    contents.sample_files.push(name);
                }
            }
        }

        Ok(contents)
    }

    /// List contents of a TAR file
    fn list_tar(path: &Path) -> Result<ArchiveContents> {
        let file = std::fs::File::open(path)?;

        // Check if gzipped
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let reader: Box<dyn std::io::Read> = if ext == "gz" || ext == "tgz" {
            Box::new(flate2::read::GzDecoder::new(file))
        } else {
            Box::new(file)
        };

        let mut archive = tar::Archive::new(reader);
        let mut contents = ArchiveContents::default();

        for entry in archive.entries()
            .map_err(|e| PanoptesError::Archive(format!("Failed to read TAR: {}", e)))?
        {
            if let Ok(entry) = entry {
                contents.file_count += 1;
                contents.total_size += entry.size();

                if let Ok(path) = entry.path() {
                    let name = path.to_string_lossy().to_string();

                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        *contents.extensions.entry(ext.to_lowercase()).or_insert(0) += 1;
                    }

                    if contents.sample_files.len() < 10 {
                        contents.sample_files.push(name);
                    }
                }
            }

            if contents.file_count >= 100 {
                break; // Limit for large archives
            }
        }

        Ok(contents)
    }

    /// Get archive contents based on type
    fn get_contents(path: &Path) -> Result<ArchiveContents> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "zip" | "jar" | "war" | "ear" => Self::list_zip(path),
            "tar" | "tgz" | "gz" => Self::list_tar(path),
            _ => Err(PanoptesError::UnsupportedFileType(ext)),
        }
    }

    /// Detect archive type from contents
    fn detect_archive_type(contents: &ArchiveContents) -> Option<&'static str> {
        let exts = &contents.extensions;

        // Check for specific project types
        if exts.contains_key("rs") || contents.sample_files.iter().any(|f| f.contains("Cargo.toml")) {
            return Some("rust_project");
        }
        if exts.contains_key("py") || contents.sample_files.iter().any(|f| f.contains("setup.py") || f.contains("pyproject.toml")) {
            return Some("python_project");
        }
        if exts.contains_key("js") || exts.contains_key("ts") || contents.sample_files.iter().any(|f| f.contains("package.json")) {
            return Some("node_project");
        }
        if exts.contains_key("java") || contents.sample_files.iter().any(|f| f.contains("pom.xml") || f.contains("build.gradle")) {
            return Some("java_project");
        }

        // Check for media archives
        let image_exts = ["jpg", "jpeg", "png", "gif", "webp"];
        let audio_exts = ["mp3", "wav", "flac", "ogg"];
        let video_exts = ["mp4", "mkv", "avi", "mov"];

        let image_count: usize = image_exts.iter().filter_map(|e| exts.get(*e)).sum();
        let audio_count: usize = audio_exts.iter().filter_map(|e| exts.get(*e)).sum();
        let video_count: usize = video_exts.iter().filter_map(|e| exts.get(*e)).sum();

        if image_count > contents.file_count / 2 {
            return Some("image_collection");
        }
        if audio_count > contents.file_count / 2 {
            return Some("audio_collection");
        }
        if video_count > contents.file_count / 2 {
            return Some("video_collection");
        }

        // Check for document archives
        let doc_exts = ["pdf", "doc", "docx", "txt", "md"];
        let doc_count: usize = doc_exts.iter().filter_map(|e| exts.get(*e)).sum();
        if doc_count > contents.file_count / 2 {
            return Some("document_collection");
        }

        None
    }
}

#[derive(Default, Debug)]
struct ArchiveContents {
    file_count: usize,
    total_size: u64,
    extensions: std::collections::HashMap<String, usize>,
    sample_files: Vec<String>,
}

impl Default for ArchiveAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for ArchiveAnalyzer {
    fn name(&self) -> &'static str {
        "archive"
    }

    fn supported_extensions(&self) -> &[&str] {
        &["zip", "tar", "gz", "tgz", "7z", "rar", "jar", "war", "ear"]
    }

    fn priority(&self) -> u8 {
        40
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing archive: {:?}", path);

        let file_hash = calculate_file_hash(path)?;

        let contents = match Self::get_contents(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read archive: {}", e);
                ArchiveContents::default()
            }
        };

        let archive_type = Self::detect_archive_type(&contents);

        let metadata = serde_json::json!({
            "file_count": contents.file_count,
            "total_size_bytes": contents.total_size,
            "extensions": contents.extensions,
            "archive_type": archive_type,
            "sample_files": contents.sample_files,
        });

        // Use LLM to suggest name based on contents
        let client = OllamaClient::new(&config.ai_engine.url);
        let prompt = format!(
            "{}\n\nArchive contains {} files.\nFile types: {:?}\nSample files: {:?}\nDetected type: {:?}",
            config.prompts.archive,
            contents.file_count,
            contents.extensions,
            contents.sample_files.iter().take(5).collect::<Vec<_>>(),
            archive_type
        );

        let suggested_name = match client.generate(&config.ai_engine.models.text, &prompt).await {
            Ok(response) => {
                let name = clean_filename(&response);
                if name.is_empty() {
                    // Fallback based on detected type
                    match archive_type {
                        Some(t) => t.to_string(),
                        None => format!("archive_{}files", contents.file_count),
                    }
                } else {
                    name
                }
            }
            Err(e) => {
                warn!("LLM failed: {}", e);
                match archive_type {
                    Some(t) => t.to_string(),
                    None => format!("archive_{}files", contents.file_count),
                }
            }
        };

        let category = Some("Archives".to_string());
        let mut tags = extract_tags(&suggested_name, &metadata);

        // Add archive type as tag
        if let Some(t) = archive_type {
            tags.push(t.replace('_', " "));
        }

        Ok(AnalysisResult {
            suggested_name,
            confidence: 0.65,
            category,
            tags,
            file_hash,
            metadata,
        })
    }
}
