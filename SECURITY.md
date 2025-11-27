<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath> -->

# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please follow responsible disclosure practices.

### How to Report

1. **Do NOT** open a public issue for security vulnerabilities
2. Send a detailed report to: **security@panoptes.example.com** (or open a confidential issue on GitLab)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### Response Timeline

| Action | Timeline |
|--------|----------|
| Acknowledgement | Within 24 hours |
| Initial assessment | Within 72 hours |
| Status update | Within 7 days |
| Fix release | Within 30 days (critical) / 90 days (moderate) |

### What to Expect

- Acknowledgement of your report within 24 hours
- Regular updates on the status of your report
- Credit in the security advisory (if desired)
- No legal action for responsible disclosure

## Security Architecture

### Design Principles

Panoptes follows defense-in-depth security principles:

1. **Memory Safety**: Written in Rust with no `unsafe` blocks
2. **Minimal Privileges**: Runs as non-root user in containers
3. **Local Processing**: No data leaves your machine
4. **Input Validation**: All file inputs are sanitized
5. **Container Isolation**: Podman rootless containers

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Malicious file names | Input sanitization, length limits |
| API injection | No external API calls (local only) |
| Container escape | Chainguard Wolfi minimal base, rootless |
| Supply chain | SPDX headers, dependency auditing |
| Denial of service | File debouncing, rate limiting |

### Security Boundaries

```
+------------------+     +------------------+     +------------------+
|   User Files     | --> |    Panoptes      | --> |    Ollama        |
| (untrusted input)|     |   (sandboxed)    |     | (containerized)  |
+------------------+     +------------------+     +------------------+
                              |
                              v
                         +----------+
                         | Renamed  |
                         |  Files   |
                         +----------+
```

### Dependency Security

- All dependencies are audited via `cargo audit`
- No floating version ranges (pinned versions)
- Regular dependency updates via Dependabot/Renovate
- SBOM generation available: `just sbom-generate`

## Security Checklist for Contributors

Before submitting code:

- [ ] No `unsafe` Rust code without justification
- [ ] All inputs validated and sanitized
- [ ] No hardcoded credentials or secrets
- [ ] SPDX headers present
- [ ] `cargo audit` passes
- [ ] `cargo clippy` passes with no warnings

## Known Limitations

1. **File System Permissions**: Panoptes operates with user permissions; ensure watched directories have appropriate access controls
2. **Network Exposure**: Ollama API (port 11434) is bound to localhost only
3. **Model Trust**: We use Moondream from official Ollama registry; verify model integrity

## Security Updates

Security advisories will be published via:

- GitLab Security Advisories
- CHANGELOG.md entries tagged `[SECURITY]`
- Direct notification to known affected users

## Compliance

This project maintains:

- RSR Gold compliance
- SPDX license headers on all files
- Dependency vulnerability scanning
- Secure container base images (Chainguard Wolfi)
