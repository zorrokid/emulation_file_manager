# Task Breakdown: async-std -> tokio Migration

## Phase 3 - Implementation

### Migration Audit
- [ ] T1 [workspace] - Audit current partial migration state before touching runtime-sensitive crates
  **File:** `database/Cargo.toml`, `cloud_storage/Cargo.toml`, `http_downloader/Cargo.toml`, `service/Cargo.toml`, `relm4-ui/Cargo.toml`, `executable_runner/Cargo.toml`, `dat_file_parser/Cargo.toml`, `relm4-ui/src/app.rs`, `relm4-ui/src/file_set_form.rs`, `relm4-ui/src/import_form.rs`, `relm4-ui/src/import/import_form.rs`
  Confirm which `flume` and `tokio` changes are already present, capture any partial relm4-ui migration, and update implementation steps before changing coupled crates.

### Core Runtime Changes
- [ ] T2 [database] - Switch SQLx from async-std to tokio
  **File:** `database/Cargo.toml`, `database/src/repository/*.rs`
  Replace `runtime-async-std` with `runtime-tokio`, add the required `tokio` dependency, and migrate all repository tests from `#[async_std::test]` to `#[tokio::test]`.

- [ ] T3 [database] - Regenerate SQLx offline metadata after the workspace compiles again
  **File:** `.sqlx/**/*`
  Run `cargo sqlx prepare --workspace -- --all-targets` after the runtime-sensitive crates compile cleanly and commit the resulting metadata with the migration.

- [ ] T4 [cloud_storage] - Complete the channel and runtime migration
  **File:** `cloud_storage/Cargo.toml`, `cloud_storage/src/lib.rs`, `cloud_storage/src/ops.rs`, `cloud_storage/src/mock.rs`
  Keep `flume` for progress channels, switch rust-s3 from async-std features to tokio features, update file and stream I/O to tokio-compatible APIs, and migrate tests to `#[tokio::test]`.

- [ ] T5 [http_downloader] - Replace surf with reqwest and finish the runtime transition
  **File:** `http_downloader/Cargo.toml`, `http_downloader/src/lib.rs`
  Preserve the existing download behavior while moving response streaming, file I/O, cancellation, and progress reporting off async-std.

- [ ] T6 [service] - Finish the service-layer channel cleanup and switch service tests to tokio
  **File:** `service/Cargo.toml`, `service/src/**/*.rs`
  Replace all remaining `async_std::channel` usage with `flume`, convert `recv().await` calls to `recv_async().await`, convert test attributes to `#[tokio::test]`, and update the synchronous helper in `service/src/file_import/add_file_set/context.rs` to use a current-thread tokio runtime.

- [ ] T7 [relm4-ui] - Normalize the GUI runtime migration without adding a second runtime
  **File:** `relm4-ui/Cargo.toml`, `relm4-ui/src/app.rs`, `relm4-ui/src/file_set_form.rs`, `relm4-ui/src/import_form.rs`, `relm4-ui/src/import/import_form.rs`
  Finish the partial tokio/flume migration already present in the GUI, update any legacy import form module still using async-std, and keep `RelmApp::run()` as the only runtime entry point.

- [ ] T8 [executable_runner] - Replace async-process and async-std test usage
  **File:** `executable_runner/Cargo.toml`, `executable_runner/src/lib.rs`, `executable_runner/src/ops.rs`
  Move process execution to `tokio::process::Command`, migrate tests to `#[tokio::test]`, and update doc examples accordingly.

- [ ] T9 [dat_file_parser] - Remove the unused async-std dependency
  **File:** `dat_file_parser/Cargo.toml`
  Drop the dependency once the workspace no longer relies on async-std through this crate.

### Documentation Alignment
- [ ] T10 [docs] - Update runtime documentation to reflect the migration plan and current repo naming
  **File:** `docs/ASYNC_STD_TO_TOKIO_MIGRATION.md`, `docs/MIXED_RUNTIME_ISSUE.md`
  Replace outdated references, document tokio as the target runtime, and keep the migration notes aligned with the repo's current instruction files.

## Phase 4 - QA Review

- [ ] T11 [workspace] - Review runtime-sensitive test coverage after implementation
  **File:** `database/src/repository/*.rs`, `cloud_storage/src/mock.rs`, `http_downloader/src/lib.rs`, `service/src/**/*.rs`, `executable_runner/src/**/*.rs`
  Confirm that each migrated crate still has coverage for its async entry points, cancellation paths, and progress-event behavior.

## Phase 5 - Tests and Validation

- [ ] T12 [workspace] - Run crate-level verification for each migrated crate
  **File:** `database`, `cloud_storage`, `http_downloader`, `service`, `relm4-ui`, `executable_runner`, `dat_file_parser`
  Run the existing crate-level `cargo test -p <crate>` or `cargo check -p <crate>` commands as each crate is migrated to catch coupling problems early.

- [ ] T13 [workspace] - Run final workspace validation
  **File:** workspace root
  Run `cargo test --workspace`, `cargo check --all-targets`, `cargo clippy --all-targets`, and `cargo sqlx prepare --workspace -- --all-targets` once the migration is complete.

## Manual Verification Checklist

- [ ] Application launches via `cargo run --bin efm-relm4-ui`
- [ ] Cloud sync starts, reports progress, and can be cancelled
- [ ] File set download reports progress and respects cancellation
- [ ] HTTP download completes and writes the expected file
- [ ] Mass import completes without runtime-related panics
- [ ] No crate in the workspace still declares a direct `async-std` dependency unless intentionally deferred and documented
