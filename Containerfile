# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

# Panoptes Container Build
# Uses Chainguard Wolfi base for minimal attack surface (RSR compliant)

# === Build Stage ===
FROM cgr.dev/chainguard/wolfi-base:latest AS builder

# Install Rust toolchain
RUN apk add --no-cache \
    rust \
    cargo \
    openssl-dev \
    pkgconf \
    musl-dev

WORKDIR /build

# Copy source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release binary
RUN cargo build --release

# === Runtime Stage ===
FROM cgr.dev/chainguard/wolfi-base:latest

# Install minimal runtime dependencies
RUN apk add --no-cache \
    openssl \
    ca-certificates

# Create non-root user for security
RUN adduser -D -u 1000 panoptes

# Copy binary from builder
COPY --from=builder /build/target/release/panoptes /usr/local/bin/panoptes

# Copy configuration
COPY config.ncl /etc/panoptes/config.ncl

# Create watch directory
RUN mkdir -p /watch && chown panoptes:panoptes /watch

# Switch to non-root user
USER panoptes

# Set working directory
WORKDIR /home/panoptes

# Volume for watched files
VOLUME ["/watch"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep panoptes || exit 1

# Default command
ENTRYPOINT ["/usr/local/bin/panoptes"]
CMD ["--config", "/etc/panoptes/config.ncl", "--watch", "/watch"]

# Labels for OCI compliance
LABEL org.opencontainers.image.title="Panoptes"
LABEL org.opencontainers.image.description="Local AI-powered file scanner and renamer"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.vendor="Jonathan D. A. Jewell"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://gitlab.com/hyperpolymath/panoptes"
