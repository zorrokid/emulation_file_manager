# Spec 007: async-std -> tokio Migration

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
Planning

## Affected Crates
- `database` - switch SQLx and repository tests to tokio
- `cloud_storage` - switch rust-s3 integration, file I/O, and tests to tokio
- `http_downloader` - replace surf with reqwest and remove async-std runtime usage
- `service` - migrate remaining channel usage and async tests
- `relm4-ui` - finish the partial tokio/flume migration in the GUI
- `executable_runner` - replace async-process usage with tokio process support
- `dat_file_parser` - remove the unused async-std dependency
- `docs` - align runtime documentation with the migration plan and current repo naming

## Problem

The workspace still treats `async-std` as the primary runtime even though the wider Rust ecosystem and this application's GTK stack are already centered around tokio. That leaves the project with three related problems:

1. `async-std` is discontinued, so keeping it as the long-term default is not viable.
2. The codebase has a mixed-runtime setup where tokio is already present transitively while several crates still use async-std directly.
3. The existing `specs/007-async-std-to-tokio/` folder predates the current spec structure and now contains stale implementation notes, outdated file references, and no `spec.md`.

## Proposed Solution

Treat the migration as a bottom-up refactor that removes direct `async-std` usage crate by crate while keeping the architecture boundaries intact.

The implementation should follow two practical stages:

1. **Channel normalization** - use `flume` for cross-crate progress and cancellation channels so those code paths become runtime-agnostic before the runtime switch is finished everywhere.
2. **Runtime migration** - move the remaining crates, tests, and async I/O integrations from async-std to tokio, starting at the lower layers (`database`) and moving upward through `service` to `relm4-ui`.

The spec documentation should also be brought into the repo's current format so the future implementation work has a stable source of truth.

## Key Decisions

| Decision | Rationale |
|---|---|
| Use tokio as the target runtime | relm4 already provides a tokio runtime, SQLx has first-class tokio support, and async-std is discontinued. |
| Use `flume` for channels instead of tokio mpsc | The codebase already uses sender/receiver patterns that map cleanly to flume, and flume keeps these paths runtime-agnostic. |
| Migrate bottom-up by architecture layer | Changing `database` first exposes runtime coupling in `service`, so the migration must respect the repository's layer boundaries. |
| Do not add a second runtime to the GUI entry point | `RelmApp::run()` already provides the GUI runtime; adding `#[tokio::main]` on top would create the wrong ownership model and risk runtime conflicts. |
| Update the spec folder before code work resumes | The current `tasks.md` is stale and incomplete, so the implementation plan needs to be normalized before it is used for execution. |

## Acceptance Criteria

- The spec folder contains both `spec.md` and `tasks.md` in the repo's current format.
- The task list accurately reflects the current codebase layout, including the partially migrated relm4-ui modules and the repo's current documentation layout.
- Direct `async-std` dependencies are removed from the migration target crates once implementation is complete, unless a deferral is explicitly documented.
- Async tests in migrated crates use `#[tokio::test]`, and async runtime helpers are updated to tokio-compatible equivalents.
- GUI code continues to rely on relm4's runtime entry point instead of introducing a second top-level runtime.
- Workspace validation succeeds with the existing commands: `cargo test --workspace`, `cargo check --all-targets`, `cargo clippy --all-targets`, and `cargo sqlx prepare --workspace -- --all-targets`.
- Runtime documentation reflects tokio as the target end state of the migration.

## As Implemented

_(Pending)_
