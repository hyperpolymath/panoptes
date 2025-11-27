# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

# Panoptes v3.0 - Local AI File Scanner
# RSR Gold Compliant Justfile with Comprehensive Recipes

set shell := ["/usr/bin/env", "bash", "-c"]
set dotenv-load

# === Variables ===
app_name := "panoptes"
version := "3.0.0"
container_name := "panoptes-llm"
vision_model := "moondream"
text_model := "llama3.2:3b"
code_model := "deepseek-coder:1.3b"
watch_dir := env_var_or_default("PANOPTES_WATCH_DIR", "./watch")
ollama_image := "docker.io/ollama/ollama:latest"
web_port := "8080"
db_path := "panoptes.db"

# Default: show available recipes
default:
    @just --list --unsorted

# === Quick Start ===

# Full setup: environment, engine, build, and run
quickstart: setup start-engine build-release
    @echo "Panoptes ready! Run: just watch"

# === Setup & Install ===

# Initial setup: create directories and validate environment
setup:
    @echo "Setting up Panoptes v{{version}} environment..."
    mkdir -p {{watch_dir}}
    mkdir -p static
    @echo "Watch directory: {{watch_dir}}"
    @echo "Verifying dependencies..."
    @command -v cargo >/dev/null 2>&1 || (echo "ERROR: Rust/Cargo not found" && exit 1)
    @command -v podman >/dev/null 2>&1 || (echo "WARNING: Podman not found - needed for AI engine")
    @echo "Setup complete."

# Install development dependencies
dev-deps:
    cargo install cargo-audit cargo-deny cargo-outdated cargo-watch cargo-tarpaulin

# Install all models
install-models: pull-vision-model pull-text-model pull-code-model
    @echo "All models installed!"

# === Build ===

# Build the scanner binary (debug)
build:
    cargo build

# Build optimized release binary
build-release:
    cargo build --release
    @echo "Binaries available at: target/release/"
    @ls -la target/release/panoptes* 2>/dev/null || true

# Build all binaries
build-all: build-release
    @echo "Built: panoptes, panoptes-undo, panoptes-web"

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and rebuild (development)
watch-build:
    cargo watch -x "build"

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
    @just pull-vision-model

# Pull the Moondream vision model
pull-vision-model:
    @echo "Pulling {{vision_model}} model..."
    podman exec -it {{container_name}} ollama pull {{vision_model}}

# Pull the text model
pull-text-model:
    @echo "Pulling {{text_model}} model..."
    podman exec -it {{container_name}} ollama pull {{text_model}}

# Pull the code model
pull-code-model:
    @echo "Pulling {{code_model}} model..."
    podman exec -it {{container_name}} ollama pull {{code_model}}

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
    @echo ""
    @curl -s http://localhost:11434/api/tags 2>/dev/null | jq -r '.models[].name' || echo "Engine not responding"

# List available models
list-models:
    @curl -s http://localhost:11434/api/tags | jq -r '.models[] | "\(.name) (\(.size | . / 1024 / 1024 / 1024 | floor)GB)"'

# === Runtime ===

# Run the scanner in watch mode
watch:
    ./target/release/{{app_name}} watch

# Run scanner with verbose logging
watch-verbose:
    ./target/release/{{app_name}} --verbose watch

# Run scanner in dry-run mode (no actual renames)
watch-dry:
    ./target/release/{{app_name}} --verbose watch --dry-run

# Run scanner processing existing files
watch-existing:
    ./target/release/{{app_name}} watch --process-existing

# Analyze a single file
analyze file:
    ./target/release/{{app_name}} analyze {{file}}

# Analyze a directory
analyze-dir dir="./watch":
    ./target/release/{{app_name}} analyze {{dir}} --recursive

# Analyze with JSON output
analyze-json file:
    ./target/release/{{app_name}} --format json analyze {{file}} --dry-run

# Show scanner status
status:
    ./target/release/{{app_name}} status

# === Web UI ===

# Start the web dashboard
web:
    ./target/release/panoptes-web

# Start web dashboard on custom port
web-port port="8080":
    ./target/release/panoptes-web --port {{port}}

# Start web and open browser
web-open:
    ./target/release/panoptes-web --open

# === Database Operations ===

# Show database statistics
db-stats:
    ./target/release/{{app_name}} db stats

# List all tags
db-tags:
    ./target/release/{{app_name}} db tags

# List all categories
db-categories:
    ./target/release/{{app_name}} db categories

# Search files in database
db-search query:
    ./target/release/{{app_name}} db search "{{query}}"

# Export database to JSON
db-export file="export.json":
    ./target/release/{{app_name}} db export {{file}}

# Vacuum database (reclaim space)
db-vacuum:
    ./target/release/{{app_name}} db vacuum

# === History & Undo ===

# List recent history
history:
    ./target/release/{{app_name}} history list

# Undo the last rename
undo:
    ./target/release/{{app_name}} history undo

# Undo multiple renames
undo-count count="5":
    ./target/release/{{app_name}} history undo --count {{count}}

# Preview what would be undone
undo-dry:
    ./target/release/{{app_name}} history undo --dry-run

# Clear all history
history-clear:
    ./target/release/{{app_name}} history clear --force

# === Configuration ===

# Show current configuration
config-show:
    ./target/release/{{app_name}} config show

# Generate default configuration
config-generate:
    ./target/release/{{app_name}} config generate --output config.json

# Validate configuration
config-validate:
    ./target/release/{{app_name}} config validate

# Edit configuration
config-edit:
    ./target/release/{{app_name}} config edit

# Initialize new project
init dir=".":
    ./target/release/{{app_name}} init --dir {{dir}}

# === Testing ===

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run specific test
test-one name:
    cargo test {{name}} -- --nocapture

# Run tests with coverage
test-coverage:
    cargo tarpaulin --out Html --output-dir coverage/

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

# Run clippy with all features
lint-all:
    cargo clippy --all-features -- -D warnings

# Fix clippy warnings automatically
lint-fix:
    cargo clippy --fix --allow-dirty

# Run all checks (format + lint + test)
check: fmt-check lint test

# Run pre-commit checks
pre-commit: fmt lint test
    @echo "Pre-commit checks passed!"

# === Security & Compliance ===

# Audit dependencies for vulnerabilities
audit:
    cargo audit

# Check dependency licenses
audit-license:
    cargo deny check

# Audit SPDX license headers
audit-spdx:
    @echo "Checking SPDX headers..."
    @find . -name "*.rs" -o -name "Justfile" -o -name "*.nix" | \
        head -30 | \
        xargs -I {} sh -c 'head -5 {} | grep -q "SPDX-License-Identifier" || echo "Missing SPDX: {}"'
    @echo "License audit complete."

# Check for outdated dependencies
outdated:
    cargo outdated

# Generate Software Bill of Materials
sbom:
    @echo "Generating SBOM..."
    cargo metadata --format-version 1 > sbom.json
    @echo "SBOM written to sbom.json"

# === Documentation ===

# Generate documentation
docs:
    cargo doc --no-deps --open

# Generate docs without opening browser
docs-build:
    cargo doc --no-deps

# Validate links in documentation
check-links:
    @command -v lychee >/dev/null 2>&1 && lychee --verbose docs/ *.md *.adoc || echo "lychee not installed"

# === RSR Gold Compliance ===

# Run full RSR compliance validation
validate: check audit audit-spdx
    @echo ""
    @echo "=== RSR Gold Compliance Validation ==="
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
    @! test -f package.json && echo "  [OK] No package.json" || echo "  [WARN] package.json found"
    @test -f Cargo.toml && echo "  [OK] Rust (Cargo.toml)" || echo "  [FAIL] No Cargo.toml"
    @test -f flake.nix && echo "  [OK] Nix flake" || echo "  [WARN] No flake.nix"
    @echo ""
    @echo "=== Validation Complete ==="

# === Utility ===

# Show project statistics
stats:
    @echo "=== Project Statistics ==="
    @echo "Lines of Rust code:"
    @find . -name "*.rs" -exec cat {} \; 2>/dev/null | wc -l
    @echo "Number of source files:"
    @find . -name "*.rs" 2>/dev/null | wc -l
    @echo "Total project size:"
    @du -sh . 2>/dev/null || echo "Unknown"

# Show version information
version:
    @echo "Panoptes v{{version}}"
    @echo "Rust: $(rustc --version)"
    @echo "Cargo: $(cargo --version)"

# Full development cycle: setup, build, test
dev: setup build test
    @echo "Development environment ready."

# Production build and validation
release: build-release validate
    @echo "Release build complete and validated."

# Clean everything (build artifacts, database, history)
clean-all: clean
    rm -f {{db_path}}
    rm -f panoptes_history.jsonl
    rm -rf coverage/
    @echo "All artifacts cleaned."

# === Container Build ===

# Build container image
container-build:
    podman build -t panoptes:{{version}} -t panoptes:latest .

# Run scanner in container
container-run:
    podman run --rm -it \
        -v {{watch_dir}}:/watch:Z \
        --network host \
        panoptes:latest

# Push container to registry
container-push registry="ghcr.io/hyperpolymath":
    podman tag panoptes:latest {{registry}}/panoptes:{{version}}
    podman push {{registry}}/panoptes:{{version}}

# === Development Workflow ===

# Start development session (build + watch)
dev-session:
    @echo "Starting development session..."
    @just build
    @just watch-build

# Run continuous integration checks locally
ci: clean build-release check validate
    @echo "CI checks passed!"

# === Help ===

# Show detailed help
help:
    @echo "Panoptes v{{version}} - Local AI File Scanner"
    @echo ""
    @echo "Quick Start:"
    @echo "  1. just quickstart       # Full setup and build"
    @echo "  2. just watch            # Start watching files"
    @echo ""
    @echo "Web Dashboard:"
    @echo "  just web                 # Start web UI at http://localhost:{{web_port}}"
    @echo "  just web-open            # Start and open browser"
    @echo ""
    @echo "Common Operations:"
    @echo "  just analyze FILE        # Analyze a single file"
    @echo "  just undo                # Undo last rename"
    @echo "  just history             # View rename history"
    @echo "  just status              # Check system status"
    @echo ""
    @echo "For RSR compliance: just validate"
    @echo ""
    @just --list
