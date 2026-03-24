# async-std → tokio Migration Guide

## Context

async-std is being discontinued. This project uses it as the primary async runtime across 7 crates, but tokio is already present in the dependency tree (pulled in by relm4 0.9.1 directly and by surf→deadpool transitively). Migrating to tokio:

- Eliminates the mixed-runtime situation documented in `MIXED_RUNTIME_ISSUE.md`
- Removes the discontinued async-std dependency
- Aligns with the dominant Rust async ecosystem

**Why tokio over smol:** relm4 already depends on tokio and provides the runtime via `RelmApp::run()`. No second runtime is needed.

**This is a refactoring** — no new behavior, existing tests define correctness.

---

## async-std vs tokio: Key Differences

Both are async runtimes — they provide the executor that polls futures to completion. Understanding the differences explains why this migration matters.

**API design:** async-std mirrors the Rust standard library (`async_std::fs::File`, `async_std::net::TcpListener`, etc.) — it was designed as a drop-in async companion to std. tokio is purpose-built for high-performance networked services with its own API conventions and a richer surrounding ecosystem.

**Runtime model:** tokio uses a work-stealing multi-threaded scheduler by default. Tasks can migrate between threads for throughput, but spawned futures must be `Send`. async-std also has a multi-threaded executor but is less tunable and generally considered simpler.

**Ecosystem:** tokio won. The majority of the async Rust ecosystem (hyper, axum, reqwest, tonic, sqlx's tokio backend, etc.) targets tokio. Libraries like `surf` and `deadpool` pull in tokio transitively regardless — which is exactly the mixed-runtime situation in `MIXED_RUNTIME_ISSUE.md`. When a tokio-aware future gets polled by the async-std executor, it can block the thread silently instead of returning a proper error.

**async-std is discontinued:** No longer actively maintained, which removes it as a viable long-term choice regardless of technical preference.

**What stays the same:** The `async/await` syntax, `Future` trait, and `Pin` machinery are part of the Rust language — not the runtime. Migrating runtimes changes which executor runs your futures and which I/O primitives you use, but async functions don't change shape.

---

## Channel Strategy: Use `flume`

Use `flume 0.11` instead of `tokio::sync::mpsc`. It is **not** currently in the dep tree and must be added explicitly — either per-crate (as shown in each step below) or once in the workspace `Cargo.toml` under `[workspace.dependencies]`.

- Same `Sender<T>` / `Receiver<T>` naming as async-std channels → minimal diff
- `flume::unbounded()` matches `async_std::channel::unbounded()` exactly
- Runtime-agnostic

**Critical API difference:** flume's async recv is `.recv_async().await` (not `.recv().await`). Every `.recv().await` on a channel receiver must become `.recv_async().await`. Missing this compiles fine but blocks the executor.

**For sends:** all channels in this project are unbounded, so `sender.send(val)` is synchronous and never blocks. Replace `sender.send(val).await` with `sender.send(val).ok()`. Do **not** use `.send_async(val).await` — it's only needed for bounded channels where you want async backpressure.

**Multiline send patterns:** `sed` only replaces single-line matches. Patterns where `.await` appears on the next line after the closing `)` must be fixed manually — just remove the `.await` line.

**Async helpers that wrap sends:** if an existing `async fn` wraps a send call (e.g. `send_progress_event`), keep it `async` even though the body is now synchronous. Making it sync would require updating every call site to remove `.await`.

---

## Implementation Order (bottom-up by architecture layer)

> **Warning:** Steps 1 and 6 are coupled. Changing `database` to `runtime-tokio` immediately breaks `service` tests — sqlx now requires a tokio context, but service tests still run on the async-std executor. Do not stop after Step 1 without completing Step 6. The practical order is: 1 → 4 → 5 → 6 (service tests depend on cloud_storage and http_downloader channel types being flume before service compiles), then 2, 3, 7 in any order.

### Step 1: `database` crate

**Files:** `database/Cargo.toml`, all `database/src/repository/*.rs` files

**Cargo.toml:**
```toml
# Remove:
async-std = { version = "1.13.1", features = ["attributes"] }
sqlx = { version = "0.8.6", features = ["runtime-async-std", "sqlite", "migrate", "chrono"] }

# Add:
tokio = { version = "1", features = ["rt", "macros"] }
sqlx = { version = "0.8.6", features = ["runtime-tokio", "sqlite", "migrate", "chrono"] }
```

**Source:** Replace all `#[async_std::test]` → `#[tokio::test]` in all 12 repository files.

**After:** Run `cargo sqlx prepare --workspace -- --all-targets` (critical — SQLx offline metadata must be regenerated). Then `cargo test -p database`.

---

### Step 2: `executable_runner` crate

**Files:** `executable_runner/Cargo.toml`, `src/lib.rs`, `src/ops.rs`

**Cargo.toml:**
```toml
# Remove:
async-process = "2.3.0"
async-std = { version = "1.13.1", features = ["attributes"] }

# Add:
tokio = { version = "1", features = ["process", "rt", "macros"] }
```

**Source:**
- `use async_process::Command` → `use tokio::process::Command`
- `#[async_std::test]` → `#[tokio::test]`

**After:** `cargo test -p executable_runner`

---

### Step 3: `dat_file_parser` crate

**Files:** `dat_file_parser/Cargo.toml` only

Remove `async-std` dependency. No source changes needed — all tests are synchronous `#[test]`.

**After:** `cargo test -p dat_file_parser`

---

### Step 4: `cloud_storage` crate

**Files:** `cloud_storage/Cargo.toml`, `src/lib.rs`, `src/ops.rs`, `src/mock.rs`

**Cargo.toml:**
```toml
# Remove:
async-std = { version = "1.13.2", features = ["attributes"] }
rust-s3 = { version = "0.37.0", default-features = false, features = ["with-async-std", "async-std-rustls-tls", "fail-on-err"] }

# Add:
tokio = { version = "1", features = ["fs", "io-util", "rt", "macros"] }
flume = "0.11"
rust-s3 = { version = "0.37.0", default-features = false, features = ["with-tokio", "tokio-rustls-tls", "fail-on-err"] }
```

**Source changes:**
- `use async_std::channel::Sender` → `use flume::Sender` (lib.rs, ops.rs, mock.rs)
- `use async_std::io::WriteExt` → `use tokio::io::AsyncWriteExt` (lib.rs)
- `use async_std::io::ReadExt` → `use tokio::io::AsyncReadExt` (lib.rs)
- `use async_std::stream::StreamExt` → `use futures::StreamExt` (lib.rs)
- `async_std::fs::File::open/create` → `tokio::fs::File::open/create` (lib.rs)
- `async_std::fs::read()` → `tokio::fs::read()` (mock.rs)
- `async_std::channel::unbounded()` → `flume::unbounded()` (mock.rs)
- All `#[async_std::test]` → `#[tokio::test]` (mock.rs, 8 tests)

**Gotcha:** Verify `futures::StreamExt` works on rust-s3's stream type with `with-tokio` — it implements `futures::Stream`, so this should be fine.

**After:** `cargo test -p cloud_storage`

---

### Step 5: `http_downloader` crate — surf → reqwest

**Files:** `http_downloader/Cargo.toml`, `src/lib.rs`

**Cargo.toml:**
```toml
# Remove:
surf = { version = "2.3", default-features = true }
async-std = { version = "1.13.2", features = ["attributes"] }

# Add:
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }
tokio = { version = "1", features = ["fs", "io-util", "rt", "macros"] }
flume = "0.11"
```

**Source — key API differences from surf:**

| surf | reqwest |
|------|---------|
| `surf::client().with(Redirect::default())` | `reqwest::Client::builder().build().unwrap()` (follows redirects by default) |
| `client.get(url).await` | `client.get(url).send().await` |
| `response.header("Content-Length")` | `response.content_length()` |
| `response.header("Content-Disposition")` | `response.headers().get("content-disposition")` |
| `response.take_body()` + `body.read(&mut buf)` | `response.bytes_stream()` + `stream.next().await` |
| `async_std::channel::{Sender, Receiver}` | `flume::{Sender, Receiver}` |
| `async_std::fs::File::create` | `tokio::fs::File::create` |
| `async_std::fs::remove_file` | `tokio::fs::remove_file` |

Use `futures::StreamExt` for `.next()` on the reqwest bytes stream.

**After:** `cargo test -p http_downloader`

---

### Step 6: `service` crate

**Files:** `service/Cargo.toml`, all files with `async_std::channel` imports or `#[async_std::test]`

**Cargo.toml:**
```toml
# Remove:
async-std = { version = "1.13.2", features = ["attributes"] }

# Add:
tokio = { version = "1", features = ["rt", "macros"] }
flume = "0.11"
```

**Source changes:**
- All `use async_std::channel::{Receiver, Sender}` / `use async_std::channel::Sender` → `use flume::{Receiver, Sender}` / `use flume::Sender`
- All `async_std::channel::unbounded()` inline calls → `flume::unbounded()`
- All `.recv().await` → `.recv_async().await`
- All `.send(val).await` / `.send(val)\n.await` → `.send(val).ok()` (unbounded, synchronous — see Channel Strategy)
- All `#[async_std::test]` → `#[tokio::test]` (~230 tests across ~37 files)

**Special case** — `service/src/file_import/add_file_set/context.rs`: has `async_std::task::block_on(setup_test_db())` inside a sync test helper. Use the current-thread runtime (no `rt-multi-thread` needed):
```rust
// Replace:
async_std::task::block_on(setup_test_db())
// With:
tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap()
    .block_on(setup_test_db())
```

**Affected files include:**
- `src/cloud_sync/context.rs`, `service.rs`, `steps.rs`
- `src/download_service.rs`
- `src/external_executable_runner/context.rs`, `service.rs`
- `src/file_set_download/context.rs`, `download_service_ops.rs`, `service.rs`
- `src/mass_import/common_steps/context.rs`, `steps.rs`, `service.rs`
- `src/mass_import/with_dat/context.rs`, `with_files_only/context.rs`
- All service `*_service.rs` test modules

**After:** `cargo test -p service`

---

### Step 7: `relm4-ui` crate

**Files:** `relm4-ui/Cargo.toml`, `src/app.rs`, `src/file_set_form.rs`, `src/import_form.rs`

**Cargo.toml:**
```toml
# Remove:
async-std = { version = "1.13.2", features = ["attributes"] }

# Add:
tokio = { version = "1", features = ["rt"] }   # needed for tokio::task::spawn
flume = "0.11"
```

Note: tokio must be an explicit dependency because `tokio::task::spawn` is called directly. relm4 provides the runtime at app startup, but the crate still needs tokio in its dep tree to resolve the import.

**`src/app.rs`:**
```rust
// Remove:
use async_std::{channel::unbounded, task};

// Add:
use flume::unbounded;
```
- `cloud_sync_cancel_tx: Option<async_std::channel::Sender<()>>` → `Option<flume::Sender<()>>`
- `task::spawn(async move { ... })` → `tokio::task::spawn(async move { ... })`
- `.recv().await` → `.recv_async().await`

**`src/file_set_form.rs` and `src/import_form.rs`:** Same pattern as app.rs.

**Important:** Do NOT add `#[tokio::main]` to `main.rs`. relm4 already provides the tokio runtime via `RelmApp::run()`. Adding a second runtime would panic.

**After:** `cargo check` + manual launch test

---

## Final Verification

```bash
cargo test --verbose          # All tests must pass
cargo clippy --all-targets    # No new warnings
cargo run --bin efm-relm4-ui  # Manual smoke test
```

Also update `docs/MIXED_RUNTIME_ISSUE.md` to reflect that tokio is now the sole runtime.

Update `CLAUDE.md`: change *"Async Runtime: async-std (not tokio — use `#[async_std::test]` in tests)"* to reflect tokio and `#[tokio::test]`.

---

## Critical Files

- `database/Cargo.toml` — SQLx runtime feature flag (must change first, triggers sqlx prepare)
- `cloud_storage/src/lib.rs` — most complex: rust-s3 + tokio::fs + tokio::io + flume + futures
- `http_downloader/src/lib.rs` — largest rewrite: surf → reqwest streaming API
- `service/src/cloud_sync/steps.rs` — representative of ~230 test annotation changes + inline channel creation
- `relm4-ui/src/app.rs` — task::spawn + recv_async, must not add second tokio runtime
