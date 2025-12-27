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

## rust-s3 Library Usage

### Error Handling Differences

The `rust-s3` library handles errors differently depending on the operation type:

#### Write Operations (PUT, POST, DELETE)
Write operations return proper S3 errors with full HTTP status and response body:

```rust
match bucket.put_multipart_chunk(...).await {
    Err(e) => {
        // e.to_string() contains:
        // "S3 error: Got HTTP 403 with content '<?xml...><Code>InvalidAccessKeyId</Code>...'"
        eprintln!("Upload error: {}", e);
    }
}
```

**Error includes:**
- HTTP status code (403, 404, etc.)
- Full XML error response
- Error codes like `InvalidAccessKeyId`, `SignatureDoesNotMatch`

#### Read Operations (GET, LIST)
Read operations that parse XML responses fail early with parsing errors:

```rust
match bucket.list("", Some("/")).await {
    Err(e) => {
        // e.to_string() contains:
        // "serde xml: missing field `Owner`"
        // The actual S3 error (403 InvalidAccessKeyId) is hidden!
    }
}
```

**Problem:** When S3 returns an error response (e.g., 403 with error XML), rust-s3 tries to parse it as a success response and fails with XML parsing errors like:
- `serde xml: missing field 'Owner'`
- `serde xml: missing field 'Name'`

This hides the actual credential/authentication errors.

#### HEAD Operations (Recommended for Connection Testing)
HEAD requests don't parse response bodies, so errors come through properly:

```rust
match bucket.head_object("test").await {
    Ok(_) => println!("Authenticated and object exists"),
    Err(e) => {
        let err_str = e.to_string();
        if err_str.contains("404") {
            // Authenticated successfully, object just doesn't exist
            println!("Authentication OK");
        } else if err_str.contains("403") || err_str.contains("InvalidAccessKeyId") {
            // Proper credential error!
            eprintln!("Invalid credentials: {}", err_str);
        }
    }
}
```

**Why HEAD works:**
- No response body to parse
- HTTP status codes come through properly
- Perfect for connection/credential testing

### Best Practices

**For connection testing:**
- ✅ Use `bucket.head_object()` - gets proper error messages
- ❌ Avoid `bucket.list()` or `bucket.exists()` - gives XML parsing errors

**For error handling:**
- Write operations: Check error string for HTTP codes and error messages directly
- Read operations: Be aware that credential errors appear as XML parsing errors
- HEAD operations: Check for 404 (success) vs 403 (auth failure)

**Example connection test:**
```rust
async fn test_connection(&self) -> Result<(), CloudStorageError> {
    match self.bucket.head_object("__test__").await {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("404") {
                Ok(()) // Authenticated, object doesn't exist
            } else if err_str.contains("403") || err_str.contains("InvalidAccessKeyId") {
                Err(CloudStorageError::InvalidCredentials(err_str))
            } else {
                Err(CloudStorageError::S3(e))
            }
        }
    }
}
```

### Related Files
- `cloud_storage/src/lib.rs` - S3 operations implementation
- `ZSTD_COMPRESSION_BUG.md` - Bug #1 documents how download errors save XML responses as files
