<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath> -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure

## [1.0.0] - 2025-11-27

### Added
- Core file watching functionality using `notify` crate
- Integration with Ollama API for local AI inference
- Support for Moondream vision model
- Image analysis for JPG, JPEG, PNG, WebP, GIF, BMP formats
- Automatic filename generation based on image content
- Date prefix option for generated filenames
- Configurable filename length limits
- Nickel configuration file support
- CLI options for runtime configuration
- Dry-run mode for testing
- Verbose logging option
- Oil Shell launcher script
- Nix flake for reproducible builds
- Podman containerization with Chainguard Wolfi base
- Comprehensive Justfile with 20+ recipes
- RSR Gold compliance documentation suite

### Security
- Memory-safe Rust implementation
- Non-root container execution
- Input sanitization for filenames
- Local-only processing (no external API calls)

### Documentation
- README.adoc with full usage guide
- SECURITY.md with vulnerability reporting process
- CONTRIBUTING.adoc with TPCF guidelines
- GOVERNANCE.adoc with project governance model
- CODE_OF_CONDUCT.adoc (Contributor Covenant 2.1)
- CLAUDE.adoc for AI assistant integration

## [0.1.0] - 2025-11-27

### Added
- Initial proof of concept
- Basic file watching
- Ollama integration prototype

---

## Versioning Policy

This project uses [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality additions
- **PATCH**: Backwards-compatible bug fixes

## Release Process

1. Update CHANGELOG.md
2. Update version in Cargo.toml
3. Create annotated git tag
4. Build release artifacts
5. Publish release notes

[Unreleased]: https://gitlab.com/hyperpolymath/panoptes/-/compare/v1.0.0...HEAD
[1.0.0]: https://gitlab.com/hyperpolymath/panoptes/-/releases/v1.0.0
[0.1.0]: https://gitlab.com/hyperpolymath/panoptes/-/releases/v0.1.0
