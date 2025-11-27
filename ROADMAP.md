<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath> -->

# Panoptes Roadmap

This document outlines the planned development direction for Panoptes.

## Current Status: v1.0.0 (Stable)

The initial stable release provides:

- **Image analysis & renaming** - JPG, PNG, WebP, GIF, BMP, TIFF
- **Moondream vision model** - Local AI via Ollama (~1.6GB)
- **Graceful operation** - Signal handling, health checks
- **Undo support** - History log with `panoptes-undo` tool
- **Dry-run mode** - Preview changes before committing
- **RSR Gold compliance** - Full documentation suite

## Immediate Fixes (v1.0.x)

### v1.0.1 - Polish
- [ ] Improve file stability detection (check file handle locks)
- [ ] Add retry logic for transient Ollama failures
- [ ] Systemd service file for daemon operation
- [ ] Man page generation

### v1.0.2 - Robustness
- [ ] Better handling of very large files
- [ ] Configurable debounce timing
- [ ] Log rotation support
- [ ] Prometheus metrics endpoint (optional)

## Short-term Goals (v1.x)

### v1.1.0 - PDF Support
- [ ] PDF first-page rasterization via `pdfium`
- [ ] Text extraction fallback via `pdf-extract`
- [ ] Document type detection
- [ ] Configurable page selection for multi-page PDFs

### v1.2.0 - Extended Image Formats
- [ ] HEIC/HEIF support (Apple photos)
- [ ] RAW format support (CR2, NEF, ARW, DNG)
- [ ] SVG thumbnail generation
- [ ] AVIF support

### v1.3.0 - Enhanced AI
- [ ] Multiple model support (LLaVA, BakLLaVA, Phi-3)
- [ ] Model auto-selection based on file size/type
- [ ] Custom prompt templates via config
- [ ] Confidence scoring (skip low-confidence renames)

### v1.4.0 - Audio Files
- [ ] Whisper integration for speech-to-text
- [ ] Audio file naming from transcription
- [ ] Music file metadata extraction
- [ ] Podcast episode detection

## Medium-term Goals (v2.x)

### v2.0.0 - Plugin Architecture
- [ ] Trait-based analyzer plugins
- [ ] Dynamic plugin loading
- [ ] Plugin configuration schema
- [ ] Community plugin registry

### v2.1.0 - Multi-Directory & Remote
- [ ] Multiple watch directories
- [ ] Remote Ollama support (TLS)
- [ ] SSH tunnel support
- [ ] Configuration hot-reload

### v2.2.0 - Video Support
- [ ] FFmpeg integration for keyframe extraction
- [ ] Video thumbnail analysis
- [ ] Audio track transcription
- [ ] Scene detection

### v2.3.0 - Documents & Code
- [ ] Office document text extraction
- [ ] Code file analysis via DeepSeek Coder
- [ ] Archive content inspection
- [ ] Email file parsing

## Long-term Vision (v3.x)

### v3.0.0 - Advanced Features
- [ ] Web UI dashboard
- [ ] Category-based organization (auto-folders)
- [ ] Tagging system
- [ ] Duplicate detection
- [ ] Semantic search over renamed files

### v3.1.0 - Sync & Distribution
- [ ] CRDT-based distributed state
- [ ] Multi-device synchronization
- [ ] Offline-first rename queue
- [ ] Conflict resolution UI

### v3.2.0 - Platform Expansion
- [ ] Native macOS app (FSEvents optimization)
- [ ] Windows service
- [ ] Mobile companion app
- [ ] NAS integration (Synology, QNAP)

## Non-Goals

The following are explicitly **out of scope**:

- **Cloud-required features** - Core functionality must work offline
- **Proprietary AI APIs** - No dependency on OpenAI, Anthropic, etc.
- **Telemetry** - No usage data collection, ever
- **Ads/Monetization** - Project remains free and open source
- **Breaking local-first** - Network features are always optional

## Extension Points

Panoptes is designed for extensibility:

| Extension | Method | Status |
|-----------|--------|--------|
| New image formats | Add to `process_file` match | Easy |
| New AI models | Ollama model swap | Easy |
| Custom prompts | Edit `config.json` | Easy |
| New file types | Implement `FileAnalyzer` trait | v2.0 |
| Custom post-processing | Plugin system | v2.0 |

## Contributing to the Roadmap

Have ideas? We welcome input:

1. Open a feature request issue
2. Discuss in existing roadmap issues
3. Submit implementation proposals
4. Vote on priorities with reactions

See [CONTRIBUTING.adoc](CONTRIBUTING.adoc) for guidelines.

## Version Support

| Version | Status | Support Until |
|---------|--------|---------------|
| 1.0.x | **Current** | Active development |
| 0.x | Legacy | Unsupported |

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed version history.

---

*Last updated: 2025-11-27*
