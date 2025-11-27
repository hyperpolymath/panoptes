// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Image file analyzer using vision models

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use image::GenericImageView;
use std::path::Path;
use tracing::{debug, info, warn};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for image files
pub struct ImageAnalyzer;

impl ImageAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Convert image to base64 for vision model
    fn encode_image(path: &Path) -> Result<String> {
        let data = std::fs::read(path)?;
        Ok(general_purpose::STANDARD.encode(&data))
    }

    /// Resize large images for faster processing
    fn prepare_image(path: &Path) -> Result<Vec<u8>> {
        let img = image::open(path)?;

        // Resize if too large (max 1024px on longest side)
        let img = if img.width() > 1024 || img.height() > 1024 {
            img.resize(1024, 1024, image::imageops::FilterType::Triangle)
        } else {
            img
        };

        // Convert to JPEG for consistent encoding
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        img.write_to(&mut cursor, image::ImageFormat::Jpeg)?;

        Ok(buffer)
    }
}

impl Default for ImageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for ImageAnalyzer {
    fn name(&self) -> &'static str {
        "image"
    }

    fn supported_extensions(&self) -> &[&str] {
        &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "heic", "heif", "avif"]
    }

    fn priority(&self) -> u8 {
        100 // High priority for images
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing image: {:?}", path);

        // Calculate file hash for deduplication
        let file_hash = calculate_file_hash(path)?;

        // Get image metadata
        let img = image::open(path)?;
        let (width, height) = img.dimensions();
        let format = image::ImageFormat::from_path(path)
            .map(|f| format!("{:?}", f))
            .unwrap_or_else(|_| "unknown".to_string());

        // Prepare image for API (resize if needed)
        let image_data = match Self::prepare_image(path) {
            Ok(data) => general_purpose::STANDARD.encode(&data),
            Err(_) => Self::encode_image(path)?, // Fallback to raw
        };

        // Call vision model
        let client = OllamaClient::new(&config.ai_engine.url);
        let response = client
            .generate_with_image(
                &config.ai_engine.models.vision,
                &config.prompts.image,
                &image_data,
            )
            .await;

        let suggested_name = match response {
            Ok(text) => clean_filename(&text),
            Err(e) => {
                warn!("Vision model failed: {}, using fallback", e);
                // Fallback: use dimensions as name
                format!("image_{}x{}", width, height)
            }
        };

        // Build metadata
        let metadata = serde_json::json!({
            "width": width,
            "height": height,
            "format": format,
            "aspect_ratio": format!("{:.2}", width as f64 / height as f64),
        });

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        let category = infer_category(&suggested_name, extension);
        let tags = extract_tags(&suggested_name, &metadata);

        Ok(AnalysisResult {
            suggested_name,
            confidence: 0.85,
            category,
            tags,
            file_hash,
            metadata,
        })
    }
}
