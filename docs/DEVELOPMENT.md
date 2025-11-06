# Development Manual

This document contains guidelines and best practices for developing the Software Collection Manager.

## Table of Contents

- [Logging and Tracing](#logging-and-tracing)
- [Project Structure](#project-structure)
- [Building and Testing](#building-and-testing)

## Logging and Tracing

### Configuration

Logging is configured in `relm4-ui/src/logging.rs`:
- **Console output**: Human-readable format for development
- **File output**: JSON format in `~/.local/share/efm/logs/` (rotated daily)
- **Default level**: `info` for most modules, `debug` for service
- **Override**: Set `RUST_LOG` environment variable (e.g., `RUST_LOG=debug cargo run`)

### When to Use Tracing

**Use `#[tracing::instrument]` for:**
- Service layer functions (business logic entry points)
- Database operations
- File I/O operations
- Network/external API calls
- Any function where you want to track execution time and context

**Use `tracing::info!` for:**
- Important state changes ("File set downloaded", "Export completed")
- User-triggered actions
- Key milestones in long operations

**Use `tracing::debug!` for:**
- Detailed flow information
- Loop iterations with data
- Intermediate calculations

**Use `tracing::error!` / `tracing::warn!` for:**
- Errors and error conditions
- Unexpected conditions that aren't errors

**Skip logging for:**
- Simple getters/setters
- Pure data transformations
- Internal helper functions
- UI event handlers (unless they do significant work)

### Best Practices

**Skip large objects in instrument:**
```rust
#[tracing::instrument(skip(self, progress_tx), fields(file_set_id))]
```
Skip `self`, channels, large objects. Use `fields()` to declare custom fields.

**Use formatting prefixes:**
- `%` - Use Display trait (cleaner for strings: `file_name = %name`)
- `?` - Use Debug trait (default)
- No prefix - Auto-detects, uses Debug by default

**Example:**
```rust
tracing::info!(
    file_set_id = context.file_set_id,
    file_set_name = %file_set.name,  // % for clean Display formatting
    "File set found"
);
```

### Availability

The `tracing` crate is currently available in:
- `relm4-ui`
- `service`

Other crates would need to add `tracing = "0.1"` to their `Cargo.toml`.

## Project Structure

(To be documented)

## Building and Testing

(To be documented)
