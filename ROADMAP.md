<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath> -->

# Panoptes Roadmap

This document outlines the planned development direction for Panoptes.

## Current Status: v1.0.0 (Stable)

The initial stable release provides:
- Image file scanning and renaming
- Moondream vision model integration
- Local-only processing
- RSR Gold compliance

## Short-term Goals (v1.x)

### v1.1.0 - PDF Support
- [ ] PDF first-page rasterization
- [ ] Text extraction fallback
- [ ] Document type detection
- [ ] Configurable page selection

### v1.2.0 - Extended Format Support
- [ ] HEIC/HEIF image support
- [ ] RAW image format support (CR2, NEF, ARW)
- [ ] WebM/MP4 thumbnail extraction
- [ ] Office document preview generation

### v1.3.0 - Enhanced AI Features
- [ ] Multiple model support (LLaVA, BakLLaVA)
- [ ] Custom prompt templates
- [ ] Batch processing mode
- [ ] Confidence scoring

## Medium-term Goals (v2.x)

### v2.0.0 - Architecture Evolution
- [ ] Plugin system for custom processors
- [ ] Multi-directory watching
- [ ] Remote Ollama support (secure)
- [ ] Web UI dashboard

### v2.1.0 - CRDT Integration
- [ ] Distributed file tracking
- [ ] Multi-device synchronization
- [ ] Offline-first rename queue
- [ ] Conflict resolution

### v2.2.0 - Advanced Classification
- [ ] Category-based organization
- [ ] Tagging system
- [ ] Semantic search
- [ ] Duplicate detection

## Long-term Vision (v3.x)

### v3.0.0 - Platform Expansion
- [ ] macOS native support (FSEvents)
- [ ] Windows support
- [ ] Mobile companion app
- [ ] Cloud integration (optional)

### v3.1.0 - Enterprise Features
- [ ] Multi-user support
- [ ] Audit logging
- [ ] Policy-based rules
- [ ] LDAP/SAML integration

## Non-Goals

The following are explicitly out of scope:

- **Cloud-required features**: Core functionality must work offline
- **Proprietary AI APIs**: No dependency on external AI services
- **Telemetry**: No usage data collection
- **Ads/Monetization**: Project remains free and open source

## End-of-Life Policy

- Major versions supported for 2 years
- Security patches for 1 year after EOL
- Clear migration guides provided
- Data export always available

## Contributing to the Roadmap

Have ideas for Panoptes? We welcome input:

1. Open a feature request issue
2. Discuss in existing roadmap issues
3. Submit implementation proposals

See [CONTRIBUTING.adoc](CONTRIBUTING.adoc) for details.

## Timeline Disclaimer

This roadmap represents current intentions, not commitments. Priorities may shift based on:

- Community feedback
- Resource availability
- Technical discoveries
- Security requirements

---

*Last updated: 2025-11-27*
