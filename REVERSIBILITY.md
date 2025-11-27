<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath> -->

# Reversibility

This document describes the reversibility guarantees of Panoptes operations, following RSR (Rhodium Standard Repository) principles.

## Core Principle

> Every operation can be undone. No destructive defaults.

Panoptes is designed with reversibility as a first-class concern. Users should feel confident experimenting with the system, knowing they can recover from any state.

## Operation Reversibility Matrix

| Operation | Reversible | Method | Data Loss Risk |
|-----------|------------|--------|----------------|
| File rename | Yes | Manual rename back | None |
| Configuration change | Yes | Edit config file | None |
| Scanner start/stop | Yes | Process control | None |
| Model change | Yes | Config/CLI flag | None |
| Container operations | Yes | Podman commands | None |

## Detailed Reversibility

### File Renaming

**What happens**: Files are renamed based on AI suggestions.

**How to reverse**:
1. Check the original filename in logs
2. Manually rename back using standard file operations
3. Use dry-run mode first to preview changes

**Safeguards**:
- Dry-run mode (`--dry-run`) previews without changes
- Verbose logging records all renames
- Original filenames logged before modification
- No file content modification (metadata only)

**Future improvement**: Undo log with one-command reversal

### Configuration Changes

**What happens**: Settings in `config.ncl` affect scanner behavior.

**How to reverse**:
- Git tracks all config changes
- Default configuration documented
- CLI flags can override config without modifying file

**Safeguards**:
- Configuration is declarative (no imperative side effects)
- Nickel provides type validation (catches errors early)
- Invalid configs fail fast with clear messages

### AI Engine State

**What happens**: Ollama container manages model state.

**How to reverse**:
- `just stop-engine` stops the container
- `just remove-engine` removes container state
- `podman volume rm ollama_data` removes model cache

**Safeguards**:
- Container isolation (no host system modification)
- Named volumes for explicit state management
- Stateless design (can rebuild from scratch)

### Process Operations

**What happens**: Scanner daemon starts/stops watching files.

**How to reverse**:
- `./start_scanner.oil stop` stops the daemon
- PID file allows clean shutdown
- Systemd integration (if configured) handles lifecycle

**Safeguards**:
- No persistent state between runs
- In-flight operations complete or timeout
- Signal handling for graceful shutdown

## Non-Reversible Operations

The following operations have external effects:

| Operation | Why Non-Reversible | Mitigation |
|-----------|-------------------|------------|
| Git push | Requires force push | Use branches, review before push |
| External API calls | N/A (none exist) | Local-only design |
| File deletion | N/A (never deletes) | Design principle |

## Safe Experimentation

### Recommended Workflow

```bash
# 1. Test with dry-run
just watch-dry

# 2. Review proposed changes in logs

# 3. Run on a test directory first
panoptes --watch ~/test-images --dry-run

# 4. Only then run on real data
just watch
```

### Recovery Scenarios

#### Scenario: Unwanted renames

```bash
# Check logs for original names
grep "Renamed" panoptes.log

# Manually revert specific files
mv "2025-11-27_new_name.jpg" "original_name.jpg"
```

#### Scenario: Bad configuration

```bash
# Reset to defaults
git checkout config.ncl

# Or use CLI overrides
panoptes --model moondream --watch /path
```

#### Scenario: Container issues

```bash
# Full reset
just stop-engine
just remove-engine
podman volume rm ollama_data
just start-engine
```

## Design Decisions for Reversibility

1. **No file deletion**: Panoptes never deletes files
2. **Rename only**: Only metadata (filename) is modified
3. **Logging**: All operations are logged with before/after state
4. **Dry-run mode**: Preview any operation before execution
5. **Stateless daemon**: No accumulated state that could corrupt
6. **Configuration as code**: All settings in version control
7. **Container isolation**: AI engine has no direct filesystem access

## Future Enhancements

- [ ] Undo log with timestamp-based reversal
- [ ] Automatic backup of original filenames
- [ ] Transaction log for batch operations
- [ ] Integration with btrfs/zfs snapshots
- [ ] Web UI with visual undo

## Related Documents

- [README.adoc](README.adoc) - Usage documentation
- [SECURITY.md](SECURITY.md) - Security considerations
- [CONTRIBUTING.adoc](CONTRIBUTING.adoc) - Development guidelines

---

*"The best way to predict the future is to be able to undo it."*
