# Task List: async-std → tokio Migration

Reference: `docs/ASYNC_STD_TO_TOKIO_MIGRATION.md`

## Strategy

Split into two phases so Phase 1 never breaks the test suite:

- **Phase 1** — channel migration only (`async_std::channel` → `flume`). Flume is runtime-agnostic, so all tests keep passing under async-std.
- **Phase 2** — runtime migration (sqlx, rust-s3, surf → reqwest, test annotations). Must be done in one continuous session because changing `database` immediately breaks `service` tests (they won't be fixed until Task 2.5).

---

## Phase 1: Channel migration (non-breaking)

### Task 1.1 — Add flume to each crate that uses channels

Add `flume = "0.11"` to the `[dependencies]` section of:
- `cloud_storage/Cargo.toml`
- `http_downloader/Cargo.toml`
- `service/Cargo.toml`
- `relm4-ui/Cargo.toml`

Also add `tokio = { version = "1", features = ["rt"] }` to `relm4-ui/Cargo.toml` — needed because `tokio::task::spawn` is used directly (relm4 provides the runtime at startup, but the crate still needs tokio in its dep tree to resolve the import).

Verify: `cargo check` passes.

---

### Task 1.2 — Migrate `cloud_storage` channels

**`cloud_storage/src/ops.rs`**
- Replace `use async_std::channel::Sender;` with `use flume::Sender;`

**`cloud_storage/src/lib.rs`**
- Replace `use async_std::channel::Sender;` with `use flume::Sender;`
- In `download_file`: the progress send is multiline:
  ```rust
  tx.send(DownloadEvent::FileDownloadProgress { .. })
      .await   // ← remove this line
      .ok();
  ```
- In `multipart_upload`: same pattern appears twice for `SyncEvent::PartUploaded` and `SyncEvent::PartUploadFailed` — remove the `.await` line from each

**`cloud_storage/src/mock.rs`**
- Replace `use async_std::channel::Sender;` with `use flume::Sender;`
- Replace `async_std::channel::unbounded()` with `flume::unbounded()` (in the test)
- In `upload_file`: two multiline send patterns — remove the `.await` line from each

> **Why `.ok()` not `.send_async().await`:** all channels in this project are unbounded. `flume::Sender::send()` on an unbounded channel is synchronous and never blocks, so there is no need for the async variant.

Verify: `cargo test -p cloud_storage`

---

### Task 1.3 — Migrate `http_downloader` channels

**`http_downloader/src/lib.rs`**
- Replace `use async_std::{channel::{Receiver, Sender}, ..}` with `use flume::{Receiver, Sender};` (keep the `fs::File` import from async_std for now)
- In `send_status_message`: change `progress_tx.send(event).await` to `progress_tx.send(event).ok()`

  Keep `send_status_message` as `async fn` even though its body is now synchronous — changing it to `fn` would require removing `.await` from every call site.

Verify: `cargo test -p http_downloader`

---

### Task 1.4 — Migrate `service` channels

Do a **workspace-wide search** for each of the following and replace in every file found:

| Search | Replace |
|--------|---------|
| `use async_std::channel::{Receiver, Sender};` | `use flume::{Receiver, Sender};` |
| `use async_std::channel::Sender;` | `use flume::Sender;` |
| `use async_std::channel::Receiver;` | `use flume::Receiver;` |
| `use async_std::channel;` | `use flume as channel;` |
| `async_std::channel::unbounded::<` | `flume::unbounded::<` |
| `async_std::channel::unbounded()` | `flume::unbounded()` |
| `.recv().await` | `.recv_async().await` |

For `.send(val).await` on a single line, replace with `.send(val).ok()`.

**Multiline send patterns** — some `.send(...)` calls span multiple lines with `.await` on its own line. When you encounter this pattern, delete the `.await` (or `.await;`) line entirely:
```rust
// Before:
tx.send(SomeEvent { .. })
    .await
    .ok();

// After:
tx.send(SomeEvent { .. })
    .ok();
```

**`service/src/cloud_sync/steps.rs`** — the `send_progress_event` helper is `async`. Keep it `async` even though its body is now just a synchronous `send`. If you make it a regular `fn`, you'd need to remove `.await` from every call site throughout the file.

Verify: `cargo check -p service`

---

### Task 1.5 — Migrate `relm4-ui` channels

**`relm4-ui/src/app.rs`**
- Replace `use async_std::{channel::unbounded, task};` with:
  ```rust
  use flume::unbounded;
  use tokio::task;
  ```
- Replace `Option<async_std::channel::Sender<()>>` with `Option<flume::Sender<()>>`
- Replace `progress_rx.recv().await` with `progress_rx.recv_async().await`

**`relm4-ui/src/file_set_form.rs`** — same three changes as `app.rs`

**`relm4-ui/src/import_form.rs`** — same import and `.recv().await` changes (no `Sender` field here)

Verify: `cargo check -p efm-relm4-ui`

---

### Phase 1 verification

```bash
cargo test --workspace
```

All tests must pass before moving to Phase 2.

---

## Phase 2: Runtime migration

> Do not stop partway through Phase 2. Task 2.1 immediately breaks `service` tests — they won't be fixed until Task 2.5.

---

### Task 2.1 — `database`: switch sqlx runtime

**`database/Cargo.toml`**
- Remove `async-std = { version = "1.13.1", features = ["attributes"] }`
- Change `sqlx` features from `runtime-async-std` to `runtime-tokio`
- Add `tokio = { version = "1", features = ["rt", "macros"] }`

**All files in `database/src/repository/`** (12 files)
- Replace all `#[async_std::test]` with `#[tokio::test]`

Verify: `cargo test -p database` — all 63 tests must pass.

---

### Task 2.2 — Regenerate SQLx offline metadata

```bash
cargo sqlx prepare --workspace -- --all-targets
```

Commit the updated `.sqlx/` directory together with the Cargo.toml changes from Task 2.1.

> This command will fail if any other crate has compile errors. At this point only `service` is broken (from Task 2.1). You can defer this until after Task 2.5 when the workspace compiles cleanly.

---

### Task 2.3 — `cloud_storage`: switch rust-s3 runtime

**`cloud_storage/Cargo.toml`**
- Remove `async-std = { version = "1.13.2", features = ["attributes"] }`
- Change rust-s3 features from `with-async-std`, `async-std-rustls-tls` to `with-tokio`, `tokio-rustls-tls`
- Add `tokio = { version = "1", features = ["fs", "io-util", "rt", "macros"] }`
- Add `futures = "0.3"` (needed for `StreamExt` on the rust-s3 stream type)

**`cloud_storage/src/lib.rs`**
- Replace `use async_std::io::WriteExt;` with `use tokio::io::AsyncWriteExt;`
- Replace `use async_std::io::ReadExt;` with `use tokio::io::AsyncReadExt;` (inside `multipart_upload`)
- Replace `use async_std::stream::StreamExt;` with `use futures::StreamExt;`
- Replace `async_std::fs::File::open(...)` with `tokio::fs::File::open(...)`
- Replace `async_std::fs::File::create(...)` with `tokio::fs::File::create(...)`

**`cloud_storage/src/mock.rs`**
- Replace `async_std::fs::read(file_path).await` with `tokio::fs::read(file_path).await`
- Replace all `#[async_std::test]` with `#[tokio::test]` (8 tests)

Verify: `cargo test -p cloud_storage`

---

### Task 2.4 — `http_downloader`: surf → reqwest

**`http_downloader/Cargo.toml`**
- Remove `surf = { version = "2.3", default-features = true }`
- Remove `async-std = { version = "1.13.2", features = ["attributes"] }`
- Add `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }`
- Add `tokio = { version = "1", features = ["fs", "io-util", "rt", "macros"] }`

**`http_downloader/src/lib.rs`** — this is a full rewrite of the async logic:

Replace the async-std file and I/O imports:
- `use async_std::io::{ReadExt, WriteExt};` → `use tokio::io::AsyncWriteExt;`
- `use async_std::{..., fs::File, ..}` → `use tokio::fs::File;`
- Add `use futures::StreamExt;` for streaming the response body

Replace the HTTP client setup:
```rust
// Before:
let client = surf::client().with(surf::middleware::Redirect::default());
let mut response = client.get(url).await.map_err(..)?;

// After (reqwest follows redirects by default):
let client = reqwest::Client::new();
let response = client.get(url).send().await.map_err(..)?;
```

Replace `Content-Length` header access:
```rust
// Before:
response.header("Content-Length").and_then(|h| h.as_str().parse::<u64>().ok())

// After:
response.content_length()
```

Replace the body streaming loop:
```rust
// Before: fixed buffer read loop
let mut body = response.take_body();
let mut buffer = vec![0u8; buffer_size];
loop {
    let bytes_read = body.read(&mut buffer).await?;
    if bytes_read == 0 { break; }
    file.write_all(&buffer[..bytes_read]).await?;
}

// After: stream chunks
let mut stream = response.bytes_stream();
while let Some(chunk_res) = stream.next().await {
    // check cancellation here
    let chunk = chunk_res.map_err(|e| DownloadError::RequestFailed(...))?;
    file.write_all(&chunk).await?;
}
```

Replace `async_std::fs::remove_file` with `tokio::fs::remove_file`.

Update `extract_filename_from_headers` — it takes `&surf::Response`, change to `&reqwest::Response`:
```rust
// Before:
response.header("Content-Disposition")?.as_str().split(...)

// After:
response.headers()
    .get("content-disposition")
    .and_then(|v| v.to_str().ok())?
    .split(...)
```

Verify: `cargo test -p http_downloader`

---

### Task 2.5 — `service`: switch test runtime

**`service/Cargo.toml`**
- Remove `async-std = { version = "1.13.2", features = ["attributes"] }`
- Add `tokio = { version = "1", features = ["rt", "macros"] }`

**All files in `service/src/`** — workspace-wide replace:
- `#[async_std::test]` → `#[tokio::test]`

**`service/src/file_import/add_file_set/context.rs`** — sync test helper using `block_on`. Replace:
```rust
// Before:
async_std::task::block_on(setup_test_db())

// After (current-thread runtime, no rt-multi-thread needed):
tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap()
    .block_on(setup_test_db())
```

**`service/src/file_set_download/download_service_ops.rs`** — has a doc example with `#[async_std::main]`. Change to `#[tokio::main]` and mark the code block `ignore` (`#[tokio::main]` requires `rt-multi-thread` which we don't add):
````
```ignore
#[tokio::main]
async fn main() { ...
````

Verify: `cargo test -p service` — all 156 tests must pass.

---

### Task 2.6 — `executable_runner`: switch process and runtime

**`executable_runner/Cargo.toml`**
- Remove `async-process = "2.3.0"`
- Remove `async-std = { version = "1.13.1", features = ["attributes"] }`
- Add `tokio = { version = "1", features = ["process", "rt", "macros"] }`

**`executable_runner/src/lib.rs`**
- Replace `use async_process::Command;` with `use tokio::process::Command;`
- Replace `#[async_std::test]` with `#[tokio::test]`

**`executable_runner/src/ops.rs`**
- Replace `#[async_std::test]` with `#[tokio::test]` (3 tests)
- Update the doc comment `/// #[async_std::main]` to `/// #[tokio::main]`
- Mark that doc example as `ignore` (same reason as service — `#[tokio::main]` requires `rt-multi-thread`)

Verify: `cargo test -p executable_runner`

---

### Task 2.7 — `dat_file_parser`: remove async-std

**`dat_file_parser/Cargo.toml`**
- Remove `async-std = { version = "1.13.2", features = ["attributes"] }`

No source changes needed — all tests are synchronous `#[test]`.

Verify: `cargo test -p dat_file_parser`

---

### Task 2.8 — Update `CLAUDE.md`

Change the async runtime line:
```
# Before:
- **Async Runtime:** async-std (not tokio — use `#[async_std::test]` in tests)

# After:
- **Async Runtime:** tokio (use `#[tokio::test]` in tests)
```

---

### Task 2.9 — Update `docs/MIXED_RUNTIME_ISSUE.md`

Add a note that the migration is complete and tokio is now the sole runtime.

---

### Phase 2 verification

```bash
cargo test --workspace
cargo check --all-targets
cargo clippy --all-targets
cargo sqlx prepare --workspace -- --all-targets
cargo run --bin efm-relm4-ui
```

Commit the updated `.sqlx/` directory.

---

## Manual smoke test checklist

- [ ] Application launches without panic
- [ ] Cloud sync starts, shows progress events, can be cancelled
- [ ] File set download from cloud completes with progress shown
- [ ] HTTP download (from download dialog) completes, progress shown, can be cancelled
- [ ] Mass import runs to completion
