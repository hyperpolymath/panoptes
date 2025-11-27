# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

# Justfile for Panoptes AI Scanner
# RSR Gold Compliant Task Runner

set shell := ["/usr/bin/env", "bash", "-c"]
set dotenv-load

# Variables
app_name := "panoptes"
container_name := "panoptes-llm"
model := "moondream"
watch_dir := env_var_or_default("PANOPTES_WATCH_DIR", "/var/home/core/Downloads/scan_input")
ollama_image := "docker.io/ollama/ollama:latest"

# Default: show available recipes
default:
    @just --list

# === Setup & Install ===

# Initial setup: create directories and validate environment
setup:
    @echo "Setting up Panoptes environment..."
    mkdir -p {{watch_dir}}
    @echo "Watch directory created: {{watch_dir}}"
    @echo "Verifying dependencies..."
    @command -v cargo >/dev/null 2>&1 || (echo "ERROR: Rust/Cargo not found" && exit 1)
    @command -v podman >/dev/null 2>&1 || (echo "ERROR: Podman not found" && exit 1)
    @echo "Setup complete."

# Install development dependencies
dev-deps:
    cargo install cargo-audit cargo-deny cargo-outdated

# === Build ===

# Build the scanner binary (debug)
build:
    cargo build

# Build optimized release binary
build-release:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean

# === AI Engine Management ===

# Start the Ollama AI Engine in Podman
start-engine:
    @echo "Starting Ollama AI Engine..."
    podman run -d \
        --name {{container_name}} \
        --replace \
        -p 11434:11434 \
        -v ollama_data:/root/.ollama \
        --stop-signal SIGKILL \
        {{ollama_image}}
    @echo "Waiting for Ollama to initialize..."
    @sleep 5
    @just pull-model

# Pull the Moondream vision model
pull-model:
    @echo "Pulling {{model}} model (this may take a while)..."
    podman exec -it {{container_name}} ollama pull {{model}}

# Stop the AI Engine
stop-engine:
    @echo "Stopping Ollama AI Engine..."
    podman stop {{container_name}} || true

# Remove the AI Engine container
remove-engine:
    podman rm -f {{container_name}} || true

# Restart the AI Engine
restart-engine: stop-engine start-engine

# Check AI Engine status
engine-status:
    @podman ps --filter name={{container_name}} --format "table {{{{.Names}}}}\t{{{{.Status}}}}\t{{{{.Ports}}}}"

# === Runtime ===

# Run the scanner in foreground
watch:
    @echo "Starting Panoptes Scanner..."
    ./target/release/{{app_name}} --config config.json

# Run scanner in debug mode with verbose logging
watch-debug:
    @echo "Starting Panoptes Scanner (debug mode)..."
    ./target/debug/{{app_name}} --config config.json --verbose

# Run scanner in dry-run mode (no actual renames)
watch-dry:
    @echo "Starting Panoptes Scanner (dry-run mode)..."
    ./target/release/{{app_name}} --config config.json --dry-run --verbose

# === Undo & History ===

# Undo the last rename
undo:
    ./target/release/panoptes-undo --count 1

# Undo multiple renames
undo-all:
    ./target/release/panoptes-undo --count 0

# Preview what would be undone
undo-dry:
    ./target/release/panoptes-undo --dry-run --count 1

# List all rename history
history:
    ./target/release/panoptes-undo --list

# === Testing ===

# Run all tests
test:
    cargo test

# Run tests with coverage
test-coverage:
    cargo tarpaulin --out Html

# === Code Quality ===

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Run all checks (format + lint + test)
check: fmt-check lint test

# === Security & Compliance ===

# Audit dependencies for vulnerabilities
audit:
    cargo audit

# Audit SPDX license headers
audit-licence:
    @echo "Checking SPDX headers..."
    @find . -name "*.rs" -o -name "*.ncl" -o -name "Justfile" -o -name "*.nix" | \
        head -20 | \
        xargs -I {} sh -c 'head -5 {} | grep -q "SPDX-License-Identifier" || echo "Missing SPDX: {}"'
    @echo "License audit complete."

# Check for outdated dependencies
outdated:
    cargo outdated

# Generate Software Bill of Materials
sbom-generate:
    @echo "Generating SBOM..."
    cargo metadata --format-version 1 > sbom.json
    @echo "SBOM written to sbom.json"

# === Documentation ===

# Generate documentation
docs:
    cargo doc --no-deps --open

# Validate links in documentation
check-links:
    @echo "Checking documentation links..."
    @command -v lychee >/dev/null 2>&1 && lychee --verbose docs/ *.md *.adoc || echo "lychee not installed, skipping link check"

# === RSR Compliance ===

# Run full RSR compliance validation
validate: check audit audit-licence check-links
    @echo ""
    @echo "=== RSR Compliance Validation ==="
    @echo "Checking required files..."
    @test -f README.adoc && echo "  [OK] README.adoc" || echo "  [FAIL] README.adoc"
    @test -f LICENSE.txt && echo "  [OK] LICENSE.txt" || echo "  [FAIL] LICENSE.txt"
    @test -f SECURITY.md && echo "  [OK] SECURITY.md" || echo "  [FAIL] SECURITY.md"
    @test -f CODE_OF_CONDUCT.adoc && echo "  [OK] CODE_OF_CONDUCT.adoc" || echo "  [FAIL] CODE_OF_CONDUCT.adoc"
    @test -f CONTRIBUTING.adoc && echo "  [OK] CONTRIBUTING.adoc" || echo "  [FAIL] CONTRIBUTING.adoc"
    @test -f FUNDING.yml && echo "  [OK] FUNDING.yml" || echo "  [FAIL] FUNDING.yml"
    @test -f GOVERNANCE.adoc && echo "  [OK] GOVERNANCE.adoc" || echo "  [FAIL] GOVERNANCE.adoc"
    @test -f MAINTAINERS.md && echo "  [OK] MAINTAINERS.md" || echo "  [FAIL] MAINTAINERS.md"
    @test -f .gitignore && echo "  [OK] .gitignore" || echo "  [FAIL] .gitignore"
    @test -f .gitattributes && echo "  [OK] .gitattributes" || echo "  [FAIL] .gitattributes"
    @test -f REVERSIBILITY.md && echo "  [OK] REVERSIBILITY.md" || echo "  [FAIL] REVERSIBILITY.md"
    @test -d .well-known && echo "  [OK] .well-known/" || echo "  [FAIL] .well-known/"
    @test -f .well-known/security.txt && echo "  [OK] .well-known/security.txt" || echo "  [FAIL] .well-known/security.txt"
    @test -f .well-known/ai.txt && echo "  [OK] .well-known/ai.txt" || echo "  [FAIL] .well-known/ai.txt"
    @test -f .well-known/provenance.json && echo "  [OK] .well-known/provenance.json" || echo "  [FAIL] .well-known/provenance.json"
    @test -f .well-known/humans.txt && echo "  [OK] .well-known/humans.txt" || echo "  [FAIL] .well-known/humans.txt"
    @echo ""
    @echo "Checking language compliance..."
    @! test -f package.json && echo "  [OK] No package.json (JavaScript eliminated)" || echo "  [WARN] package.json found"
    @test -f Cargo.toml && echo "  [OK] Rust (Cargo.toml)" || echo "  [FAIL] No Cargo.toml"
    @test -f flake.nix && echo "  [OK] Nix flake present" || echo "  [WARN] No flake.nix"
    @echo ""
    @echo "=== Validation Complete ==="

# === Utility ===

# Show project statistics
stats:
    @echo "=== Project Statistics ==="
    @echo "Lines of Rust code:"
    @find . -name "*.rs" -exec cat {} \; | wc -l
    @echo "Number of source files:"
    @find . -name "*.rs" | wc -l

# Full development cycle: setup, build, test
dev: setup build test
    @echo "Development environment ready."

# Production build and validation
release: build-release validate
    @echo "Release build complete and validated."

# === Container Build ===

# Build container image
container-build:
    podman build -t panoptes:latest .

# Run scanner in container
container-run:
    podman run --rm -it \
        -v {{watch_dir}}:/watch:Z \
        --network host \
        panoptes:latest

# === Help ===

# Show detailed help
help:
    @echo "Panoptes - Local AI File Scanner"
    @echo ""
    @echo "Quick Start:"
    @echo "  1. just setup          # Prepare environment"
    @echo "  2. just start-engine   # Start Ollama in Podman"
    @echo "  3. just build-release  # Build scanner"
    @echo "  4. just watch          # Start watching files"
    @echo ""
    @echo "For RSR compliance: just validate"
    @echo ""
    @just --list
