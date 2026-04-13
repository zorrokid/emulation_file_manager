# Task Breakdown: async-std -> tokio Migration

## Phase 3 - Implementation

### Migration Audit
- [x] T1 [workspace] - Audit current partial migration state before touching runtime-sensitive crates
  **File:** `database/Cargo.toml`, `cloud_storage/Cargo.toml`, `http_downloader/Cargo.toml`, `service/Cargo.toml`, `relm4-ui/Cargo.toml`, `executable_runner/Cargo.toml`, `dat_file_parser/Cargo.toml`, `relm4-ui/src/app.rs`, `relm4-ui/src/file_set_form.rs`, `relm4-ui/src/import_form.rs`, `relm4-ui/src/import/import_form.rs`
  Audit complete. Findings:
  - `database`, `executable_runner`, and `dat_file_parser` still use direct `async-std` dependencies.
  - `cloud_storage` already uses `flume` channels, but runtime APIs and tests are still on async-std.
  - `http_downloader` already depends on `flume`, but source still uses `surf` and async-std I/O.
  - `service` already depends on `flume`, but `service/src/download_service.rs` still uses `async_std::channel`, tests are still `#[async_std::test]`, and the known `block_on` / doc-example holdouts remain.
  - `relm4-ui` is partially migrated: `src/app.rs` and `src/import/import_form.rs` use `flume`/`tokio`, while `src/file_set_form.rs` and `src/import_form.rs` still use async-std.

### Core Runtime Changes
- [ ] T2 [database] - Switch SQLx from async-std to tokio
  **File:** `database/Cargo.toml`, `database/src/repository/*.rs`
  Replace `runtime-async-std` with `runtime-tokio`, add the required `tokio` dependency, and migrate all repository tests from `#[async_std::test]` to `#[tokio::test]`.

- [ ] T3 [database] - Regenerate SQLx offline metadata after the workspace compiles again
  **File:** `.sqlx/**/*`
  Run `cargo sqlx prepare --workspace -- --all-targets` after the runtime-sensitive crates compile cleanly and commit the resulting metadata with the migration.

- [ ] T4 [cloud_storage] - Complete the channel and runtime migration
  **File:** `cloud_storage/Cargo.toml`, `cloud_storage/src/lib.rs`, `cloud_storage/src/ops.rs`, `cloud_storage/src/mock.rs`
  Keep the existing `flume` channel migration, switch rust-s3 from async-std features to tokio features, update file and stream I/O to tokio-compatible APIs, and migrate the remaining async-std tests to `#[tokio::test]`.

- [ ] T5 [http_downloader] - Replace surf with reqwest and finish the runtime transition
  **File:** `http_downloader/Cargo.toml`, `http_downloader/src/lib.rs`
  Preserve the existing download behavior while replacing `surf` with `reqwest`, removing the remaining async-std I/O/channel usage, and keeping the already-added `flume` dependency aligned with the implementation.

- [ ] T6 [service] - Finish the service-layer channel cleanup and switch service tests to tokio
  **File:** `service/Cargo.toml`, `service/src/**/*.rs`
  Keep the existing `flume` migration, replace the remaining direct `async_std::channel` usage in `service/src/download_service.rs`, convert the remaining test/runtime holdouts to tokio, and update the synchronous helper in `service/src/file_import/add_file_set/context.rs` plus the `#[async_std::main]` doc example in `service/src/file_set_download/download_service_ops.rs`.

- [ ] T7 [relm4-ui] - Normalize the GUI runtime migration without adding a second runtime
  **File:** `relm4-ui/Cargo.toml`, `relm4-ui/src/app.rs`, `relm4-ui/src/file_set_form.rs`, `relm4-ui/src/import_form.rs`, `relm4-ui/src/import/import_form.rs`
  Keep the existing tokio/flume migration in `src/app.rs` and `src/import/import_form.rs`, migrate the legacy `src/file_set_form.rs` and `src/import_form.rs` modules off async-std, and remove the redundant `async-std` dependency from `relm4-ui/Cargo.toml` without adding a second runtime entry point.

- [ ] T8 [executable_runner] - Replace async-process and async-std test usage
  **File:** `executable_runner/Cargo.toml`, `executable_runner/src/lib.rs`, `executable_runner/src/ops.rs`
  Move process execution to `tokio::process::Command`, migrate tests to `#[tokio::test]`, and update doc examples accordingly.

- [ ] T9 [dat_file_parser] - Remove the unused async-std dependency
  **File:** `dat_file_parser/Cargo.toml`
  Drop the dependency once the workspace no longer relies on async-std through this crate.

### Documentation Alignment
- [ ] T10 [docs] - Update runtime documentation to reflect the migration plan and current repo naming
  **File:** `docs/ASYNC_STD_TO_TOKIO_MIGRATION.md`, `docs/MIXED_RUNTIME_ISSUE.md`
  Update the docs to reflect the audited partial migration state, document tokio as the target runtime, and keep the migration notes aligned with the final implementation plan.

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
