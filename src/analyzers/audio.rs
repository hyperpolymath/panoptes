// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Audio file analyzer using metadata and optional transcription

use async_trait::async_trait;
use id3::TagLike;
use std::path::Path;
use tracing::{debug, info, warn};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for audio files
pub struct AudioAnalyzer;

impl AudioAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Extract metadata from MP3 using ID3 tags
    fn extract_mp3_metadata(path: &Path) -> Option<AudioMetadata> {
        let tag = id3::Tag::read_from_path(path).ok()?;

        Some(AudioMetadata {
            title: tag.title().map(String::from),
            artist: tag.artist().map(String::from),
            album: tag.album().map(String::from),
            year: tag.year(),
            genre: tag.genre().map(String::from),
            duration_secs: None, // ID3 doesn't store duration directly
        })
    }

    /// Extract metadata using symphonia (supports many formats)
    fn extract_generic_metadata(path: &Path) -> Option<AudioMetadata> {
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let file = std::fs::File::open(path).ok()?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let mut probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .ok()?;

        let mut metadata = AudioMetadata::default();

        // Get duration from codec params
        if let Some(track) = probed.format.default_track() {
            if let Some(n_frames) = track.codec_params.n_frames {
                if let Some(sample_rate) = track.codec_params.sample_rate {
                    metadata.duration_secs = Some(n_frames as f64 / sample_rate as f64);
                }
            }
        }

        // Get metadata tags
        if let Some(meta) = probed.metadata.get() {
            if let Some(rev) = meta.current() {
                for tag in rev.tags() {
                    match tag.std_key {
                        Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                            metadata.title = Some(tag.value.to_string());
                        }
                        Some(symphonia::core::meta::StandardTagKey::Artist) => {
                            metadata.artist = Some(tag.value.to_string());
                        }
                        Some(symphonia::core::meta::StandardTagKey::Album) => {
                            metadata.album = Some(tag.value.to_string());
                        }
                        Some(symphonia::core::meta::StandardTagKey::Genre) => {
                            metadata.genre = Some(tag.value.to_string());
                        }
                        Some(symphonia::core::meta::StandardTagKey::Date) => {
                            if let Ok(year) = tag.value.to_string().parse::<i32>() {
                                metadata.year = Some(year);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Some(metadata)
    }
}

#[derive(Default, Debug)]
struct AudioMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    duration_secs: Option<f64>,
}

impl Default for AudioAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for AudioAnalyzer {
    fn name(&self) -> &'static str {
        "audio"
    }

    fn supported_extensions(&self) -> &[&str] {
        &["mp3", "wav", "flac", "ogg", "m4a", "aac", "wma", "opus", "aiff"]
    }

    fn priority(&self) -> u8 {
        80
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing audio: {:?}", path);

        let file_hash = calculate_file_hash(path)?;

        // Try MP3-specific first, then generic
        let audio_meta = if path.extension().and_then(|e| e.to_str()) == Some("mp3") {
            Self::extract_mp3_metadata(path).or_else(|| Self::extract_generic_metadata(path))
        } else {
            Self::extract_generic_metadata(path)
        };

        let metadata = match &audio_meta {
            Some(meta) => serde_json::json!({
                "title": meta.title,
                "artist": meta.artist,
                "album": meta.album,
                "year": meta.year,
                "genre": meta.genre,
                "duration_secs": meta.duration_secs,
            }),
            None => serde_json::json!({}),
        };

        // Build suggested name from metadata
        let suggested_name = if let Some(ref meta) = audio_meta {
            // Prefer artist - title format
            match (&meta.artist, &meta.title) {
                (Some(artist), Some(title)) => {
                    clean_filename(&format!("{} - {}", artist, title))
                }
                (None, Some(title)) => clean_filename(title),
                (Some(artist), None) => {
                    if let Some(album) = &meta.album {
                        clean_filename(&format!("{} - {}", artist, album))
                    } else {
                        clean_filename(artist)
                    }
                }
                (None, None) => {
                    // No metadata, use LLM on filename
                    let filename = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("audio");

                    let client = OllamaClient::new(&config.ai_engine.url);
                    let prompt = format!(
                        "This audio file is named '{}'. Suggest a cleaner filename. {}",
                        filename, config.prompts.audio
                    );

                    match client.generate(&config.ai_engine.models.text, &prompt).await {
                        Ok(response) => clean_filename(&response),
                        Err(_) => clean_filename(filename),
                    }
                }
            }
        } else {
            // No metadata extraction possible
            let filename = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("audio");
            clean_filename(filename)
        };

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp3");
        let category = infer_category(&suggested_name, extension);

        // Build tags from metadata
        let mut tags = Vec::new();
        if let Some(ref meta) = audio_meta {
            if let Some(ref genre) = meta.genre {
                tags.push(genre.clone());
            }
            if let Some(ref artist) = meta.artist {
                tags.push(artist.clone());
            }
        }
        tags.extend(extract_tags(&suggested_name, &metadata));
        tags.sort();
        tags.dedup();

        let confidence = if audio_meta.as_ref().and_then(|m| m.title.as_ref()).is_some() {
            0.95 // High confidence from metadata
        } else {
            0.60 // Lower confidence from filename
        };

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
