// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Video file analyzer using keyframe extraction

use async_trait::async_trait;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info, warn};
use base64::{engine::general_purpose, Engine as _};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for video files
pub struct VideoAnalyzer;

impl VideoAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if FFmpeg is available
    fn ffmpeg_available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Extract video metadata using FFprobe
    fn get_video_metadata(path: &Path) -> Option<VideoMetadata> {
        let output = Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

        let format = json.get("format")?;
        let duration = format.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok());

        // Find video stream
        let streams = json.get("streams")?.as_array()?;
        let video_stream = streams.iter()
            .find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("video"))?;

        let width = video_stream.get("width").and_then(|w| w.as_u64()).map(|w| w as u32);
        let height = video_stream.get("height").and_then(|h| h.as_u64()).map(|h| h as u32);
        let codec = video_stream.get("codec_name").and_then(|c| c.as_str()).map(String::from);
        let fps = video_stream.get("r_frame_rate")
            .and_then(|f| f.as_str())
            .and_then(|f| {
                let parts: Vec<&str> = f.split('/').collect();
                if parts.len() == 2 {
                    let num: f64 = parts[0].parse().ok()?;
                    let den: f64 = parts[1].parse().ok()?;
                    Some(num / den)
                } else {
                    f.parse().ok()
                }
            });

        // Get title from format tags
        let title = format.get("tags")
            .and_then(|t| t.get("title"))
            .and_then(|t| t.as_str())
            .map(String::from);

        Some(VideoMetadata {
            duration_secs: duration,
            width,
            height,
            codec,
            fps,
            title,
        })
    }

    /// Extract keyframes from video
    fn extract_keyframes(path: &Path, count: u32, temp_dir: &Path) -> Vec<std::path::PathBuf> {
        let mut frames = Vec::new();

        // Get video duration first
        let metadata = Self::get_video_metadata(path);
        let duration = metadata.as_ref()
            .and_then(|m| m.duration_secs)
            .unwrap_or(60.0);

        // Calculate timestamps for evenly spaced keyframes
        let interval = duration / (count + 1) as f64;

        for i in 1..=count {
            let timestamp = interval * i as f64;
            let output_path = temp_dir.join(format!("frame_{}.jpg", i));

            let result = Command::new("ffmpeg")
                .args([
                    "-ss", &format!("{:.2}", timestamp),
                    "-i",
                ])
                .arg(path)
                .args([
                    "-vframes", "1",
                    "-q:v", "2",
                    "-y",
                ])
                .arg(&output_path)
                .output();

            if result.map(|o| o.status.success()).unwrap_or(false) {
                if output_path.exists() {
                    frames.push(output_path);
                }
            }
        }

        frames
    }
}

#[derive(Debug)]
struct VideoMetadata {
    duration_secs: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    codec: Option<String>,
    fps: Option<f64>,
    title: Option<String>,
}

impl Default for VideoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for VideoAnalyzer {
    fn name(&self) -> &'static str {
        "video"
    }

    fn supported_extensions(&self) -> &[&str] {
        &["mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v"]
    }

    fn priority(&self) -> u8 {
        75
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing video: {:?}", path);

        let file_hash = calculate_file_hash(path)?;

        // Get video metadata
        let video_meta = Self::get_video_metadata(path);

        let metadata = match &video_meta {
            Some(meta) => serde_json::json!({
                "duration_secs": meta.duration_secs,
                "width": meta.width,
                "height": meta.height,
                "codec": meta.codec,
                "fps": meta.fps,
                "title": meta.title,
            }),
            None => serde_json::json!({}),
        };

        // Try to use title from metadata first
        if let Some(ref meta) = video_meta {
            if let Some(ref title) = meta.title {
                let suggested_name = clean_filename(title);
                if !suggested_name.is_empty() && suggested_name.len() > 3 {
                    let extension = path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("mp4");
                    let category = infer_category(&suggested_name, extension);
                    let tags = extract_tags(&suggested_name, &metadata);

                    return Ok(AnalysisResult {
                        suggested_name,
                        confidence: 0.95,
                        category,
                        tags,
                        file_hash,
                        metadata,
                    });
                }
            }
        }

        // If FFmpeg is available, extract keyframes and analyze
        let suggested_name = if Self::ffmpeg_available() {
            let temp_dir = std::env::temp_dir().join("panoptes_frames");
            std::fs::create_dir_all(&temp_dir)?;

            let keyframe_count = config.analyzers.video.keyframes;
            let frames = Self::extract_keyframes(path, keyframe_count, &temp_dir);

            if !frames.is_empty() {
                // Encode first frame for vision model
                let frame_data = std::fs::read(&frames[0])?;
                let encoded = general_purpose::STANDARD.encode(&frame_data);

                let client = OllamaClient::new(&config.ai_engine.url);
                let result = client
                    .generate_with_image(
                        &config.ai_engine.models.vision,
                        &config.prompts.video,
                        &encoded,
                    )
                    .await;

                // Clean up temp frames
                for frame in &frames {
                    let _ = std::fs::remove_file(frame);
                }

                match result {
                    Ok(response) => clean_filename(&response),
                    Err(e) => {
                        warn!("Vision model failed for video: {}", e);
                        // Fallback
                        let duration = video_meta.as_ref()
                            .and_then(|m| m.duration_secs)
                            .map(|d| format!("{}min", (d / 60.0) as u32))
                            .unwrap_or_default();
                        format!("video{}", if duration.is_empty() { "".to_string() } else { format!("_{}", duration) })
                    }
                }
            } else {
                // No frames extracted
                "video".to_string()
            }
        } else {
            warn!("FFmpeg not available, using basic video naming");
            let duration = video_meta.as_ref()
                .and_then(|m| m.duration_secs)
                .map(|d| format!("{}min", (d / 60.0) as u32))
                .unwrap_or_default();
            format!("video{}", if duration.is_empty() { "".to_string() } else { format!("_{}", duration) })
        };

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
        let category = infer_category(&suggested_name, extension);
        let tags = extract_tags(&suggested_name, &metadata);

        Ok(AnalysisResult {
            suggested_name,
            confidence: 0.70,
            category,
            tags,
            file_hash,
            metadata,
        })
    }
}
