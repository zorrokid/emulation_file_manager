# Development Manual

This document contains guidelines and best practices for developing the Software Collection Manager.

## Table of Contents

- [Logging and Tracing](#logging-and-tracing)
- [Project Structure](#project-structure)
- [Building and Testing](#building-and-testing)
- [Tokio Async Runtime](#tokio-async-runtime)

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

## Relm4 UI Development

### Using gtk::Entry Widget Correctly

When using `gtk::Entry` widgets in Relm4's `view!` macro, be careful about how you handle text updates to avoid common pitfalls:

#### Problem: Infinite Update Loop

**DON'T do this:**
```rust
gtk::Entry {
    #[watch]
    set_text: &model.field,  // Sets text when model changes
    connect_changed[sender] => move |entry| {  // Triggers on text change
        sender.input(Msg::UpdateField(entry.buffer().text().into()));
    },
}
```

This creates an infinite loop:
1. User types → `connect_changed` fires → sends `UpdateField` message
2. Model updates → `#[watch]` triggers → `set_text` called
3. Text change → `connect_changed` fires again → repeat from step 1

#### Solution: Let Entry Manage Its Own State

**DO this instead:**
```rust
#[name = "field_entry"]
gtk::Entry {
    set_placeholder_text: Some("Enter value"),
    connect_changed[sender] => move |entry| {
        sender.input(Msg::UpdateField(entry.buffer().text().into()));
    },
}
```

Then manually set the text only when needed (e.g., loading data):
```rust
fn update_with_view(&mut self, widgets: &mut Self::Widgets, msg: Self::Input, ...) {
    match msg {
        Msg::LoadData(data) => {
            self.field = data.clone();
            widgets.field_entry.set_text(&data);  // Set text explicitly
        }
        Msg::UpdateField(value) => {
            self.field = value;  // Don't set_text here - Entry already has it
        }
    }
}
```

#### When to Use Each Signal

**`connect_changed`** - Fires on every keystroke
- Use for fields that need live updates (search, filters, notes)
- Model updates immediately as user types
- Example: Search boxes, notes fields

**`connect_activate`** - Fires only when Enter is pressed
- Use for fields that should commit on Enter (login, single-line inputs)
- Model updates only when user explicitly submits
- Example: Username, password, single-value inputs

**Key Points:**
- Entry widgets maintain their own text state internally
- Only use `#[watch]` with `set_text` if the text is controlled externally (rare)
- Use `connect_changed` for immediate updates, `connect_activate` for Enter-to-submit
- Always clear entry widgets explicitly when resetting forms

### Loading Forms with Async Data

When opening a form/dialog that needs to load data asynchronously (e.g., editing an existing item), don't show the window until the data is ready.

#### Problem: Showing Window Before Data Loads

**DON'T do this:**
```rust
ItemFormMsg::Show { edit_item_id, .. } => {
    if let Some(edit_item_id) = edit_item_id {
        sender.oneshot_command(async move {
            let result = fetch_item(edit_item_id).await;
            CommandMsg::ItemLoaded(result)
        });
    }
    root.show();  // ❌ Shows window before data loads
}
```

**Problem:** The window appears with default/empty values, then visibly updates when data loads. This creates a jarring user experience and can cause issues with widgets like dropdowns showing wrong initial selections.

#### Solution: Show Window After Data Loads

**DO this instead:**
```rust
ItemFormMsg::Show { edit_item_id, .. } => {
    if let Some(edit_item_id) = edit_item_id {
        // Fetch data first
        sender.oneshot_command(async move {
            let result = fetch_item(edit_item_id).await;
            CommandMsg::ItemLoaded(result)
        });
        // Don't show yet - wait for data
    } else {
        // For new items, show immediately
        root.show();
    }
}

// In update_cmd:
CommandMsg::ItemLoaded(Ok(item)) => {
    self.field = item.field;
    sender.input(Msg::UpdateFields);
    root.show();  // ✅ Show after data is loaded and fields are set
}
```

**Benefits:**
- Window appears with correct data already populated
- No visual "flicker" as fields update
- Dropdown widgets show correct selection immediately
- Better user experience

**Key Points:**
- Only show the window after async data is loaded and processed
- For "new item" mode, you can show immediately since no data needs loading
- Apply field updates before showing the window
- This pattern works for any form that loads data: edit dialogs, detail views, etc.

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
- `ZSTD_COMPRESSION_BUG.md` - documents the active cloud download validation bug and its resolved historical context

## Tokio Async Runtime

This project uses tokio as its async runtime. relm4 provides the runtime automatically via `RelmApp::run()` — **do not add `#[tokio::main]` to `main.rs`**, as that would start a second runtime and panic.

### How tokio schedules work

Tokio is a **cooperative** scheduler. Threads are shared between all running tasks, and a task holds its thread until it reaches an `.await` point, at which point it yields control back to the scheduler. This lets thousands of tasks share a small thread pool efficiently.

The consequence: **any blocking call inside an async task starves other tasks** on the same thread. If a task never yields, no other task on that thread can run.

relm4 runs tokio with a single-threaded runtime. There is exactly one thread for all async work:

```
Thread 1 (the only thread):
  → Task A runs until .await → yields
  → Task B runs until .await → yields
  → Task A resumes, etc.
```

If any task blocks that one thread, everything else freezes.

### The flume channel trap

This project uses `flume` for channels. Flume provides both a synchronous and an async API:

| Call | Behaviour |
|---|---|
| `receiver.recv()` | **Blocks the thread** until a message arrives |
| `receiver.recv_async().await` | Yields to the scheduler until a message arrives |

Inside `tokio::task::spawn`, always use **`recv_async().await`**. Using `recv()` compiles fine but blocks the tokio thread, starving every other task — including the one that would produce the messages you are waiting for. This is a classic deadlock:

```rust
// ❌ WRONG — blocks the only tokio thread
task::spawn(async move {
    while let Ok(event) = progress_rx.recv() { // ← never yields
        handle(event);
    }
});
// The task above holds the thread, so sync_to_cloud below never gets polled:
sender.oneshot_command(async move {
    sync_service.sync_to_cloud(progress_tx, cancel_rx).await
});
```

```rust
// ✅ CORRECT — yields between messages
task::spawn(async move {
    while let Ok(event) = progress_rx.recv_async().await {
        handle(event);
    }
});
```

The same rule applies to sends on **bounded** channels (a full bounded channel blocks on `send()`). All channels in this project are unbounded, so `sender.send(val)` is synchronous and non-blocking — use it without `.await`.

### Spawning blocking work

If you genuinely need to call a blocking API (e.g., a synchronous file operation, a CPU-heavy computation), move it off the async thread pool with `tokio::task::spawn_blocking`:

```rust
let result = tokio::task::spawn_blocking(|| {
    some_blocking_operation()
}).await?;
```

This runs the closure on a dedicated blocking thread pool that tokio keeps separate from the async worker threads, so the async scheduler is never stalled.

### Test attributes

Use `#[tokio::test]` for all async tests:

```rust
#[tokio::test]
async fn test_something() { ... }
```
