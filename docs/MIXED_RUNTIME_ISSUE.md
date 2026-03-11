# Mixed Async Runtime Issue

## Overview

This project uses **async-std** as the primary async runtime, but inadvertently includes **tokio** through transitive dependencies. This creates a mixed runtime environment where both runtimes coexist in the same process.

## Current State

### Direct Runtime Usage
- All application code uses `async-std`
- Database layer (`sqlx`) configured with `runtime-async-std` feature
- All `Cargo.toml` files explicitly depend on `async-std`

### Tokio Introduction Path

Tokio is pulled in through the following dependency chain:

```
service
├── http_downloader
│   └── surf v2.3.2
│       └── http-client v6.5.3
│           └── deadpool v0.7.0
│               └── tokio v1.46.1
└── cloud_storage
    └── rust-s3 v0.37.1
        └── surf v2.3.2
            └── (same chain as above)
```

**Key culprit**: `deadpool` (connection pooling library used by surf's http-client) depends on tokio.

### Configuration Attempts

Both `surf` and `rust-s3` are properly configured to use async-std:
- `http_downloader/Cargo.toml`: Uses `surf` with default features
- `cloud_storage/Cargo.toml`: Uses `rust-s3` with `with-async-std` and `async-std-rustls-tls` features

Despite these configurations, tokio still gets pulled in as a transitive dependency.

## Impact

### Observed Effects

1. **Thread naming confusion**: Panic messages show `thread 'tokio-runtime-worker'` even though the code uses async-std
2. **Memory overhead**: Two separate thread pools running (async-std executor + tokio runtime)
3. **Resource usage**: Additional threads and memory for the unused tokio runtime

### Not a Problem

- Does not cause functional bugs (the panic is due to application logic, not runtime mixing)
- Both runtimes can coexist safely in most cases
- No runtime conflicts observed in normal operation

### Potential Risks

1. **Deadlocks**: If code accidentally blocks one runtime while waiting for the other
2. **Confusion**: Debugging becomes harder when stack traces show tokio threads
3. **Resource waste**: Running two runtimes when only one is needed

## Solutions

### Option 1: Accept the Status Quo (Current)
**Pros**: No changes needed, works fine
**Cons**: Wastes some resources, confusing thread names

### Option 2: Switch to Tokio
**Pros**: 
- Most popular runtime in Rust ecosystem
- Better ecosystem support
- Single runtime

**Cons**:
- Requires changing all code to use tokio
- Need to update sqlx to `runtime-tokio-rustls`
- More work to implement

### Option 3: Find Alternative HTTP Libraries
**Pros**: Stay with async-std
**Cons**: 
- Limited options (most HTTP clients use tokio)
- May have other limitations
- Not worth the effort

### Option 4: Wait for Ecosystem Changes
**Pros**: surf/deadpool might eventually drop tokio dependency
**Cons**: No guarantee this will happen, could take years

## Decision

**Migrate to tokio.** async-std has been discontinued, making the status quo untenable. See the full migration guide at [`docs/ASYNC_STD_TO_TOKIO_MIGRATION.md`](ASYNC_STD_TO_TOKIO_MIGRATION.md).

Key factors:
- async-std is discontinued — the "wait and see" option is off the table
- relm4 0.9.1 already provides a tokio runtime via `RelmApp::run()`
- `flume` (already in the dep tree) provides a runtime-agnostic channel replacement with minimal API diff

## Detection

You can verify the mixed runtime situation with:

```bash
# Check for tokio in dependency tree
cargo tree -p service -i tokio

# Check what depends on tokio
cargo tree -p service 2>&1 | grep -B 10 "tokio"
```

## Last Updated

2026-03-09
