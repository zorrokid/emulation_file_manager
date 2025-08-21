# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an **Emulation File Manager and Launcher** built in Rust using GTK4 and relm4 for the GUI, with SQLite for database operations via SQLx. The application manages emulation files, software titles, releases, and provides a GUI for launching emulators.

## Development Commands

### Building and Running
- `cargo build` - Build all workspace crates
- `cargo run -p relm4-ui-test-simle` - Run the main GUI application
- `cargo test` - Run tests across all crates
- `cargo check` - Quick compilation check

### Database Operations
- `sqlx database create` - Create development database (requires `.env` file in database/ directory)
- `sqlx migrate run` - Apply database migrations
- `sqlx migrate add <name>` - Add new migration
- `cargo sqlx prepare --check` - Verify SQLx compile-time checks

The development database is created at `database/data/db.sqlite`, while the runtime database is located at `~/.local/share/efm/db.sqlite` on Linux.

## Architecture

### Layered Architecture
The codebase follows a strict 4-layer architecture:

1. **Core Crates** (`core_types`, `utils`, `emulator_runner`, `file_import`, `file_export`, `file_system`)
   - No dependencies on other project crates
   - Provide fundamental types and utilities

2. **Database Layer** (`database`)
   - May depend on core crates only
   - Contains models, repositories, and migration management
   - Uses SQLx with async-std runtime

3. **Service Layer** (`service`)
   - May depend on core crates and database layer
   - Provides view models and business logic through `ViewModelService`

4. **GUI Layer** (`relm4-ui`)
   - Main application entry point
   - Uses relm4 for reactive GTK4 components
   - May depend on all lower layers

### Key Patterns

**Component Architecture**: The GUI uses relm4's component system with:
- `AppModel` as the root component managing application state
- `ReleasesModel` and `ReleaseModel` as child components
- Message passing between components via `AppMsg`, `ReleasesMsg`, `ReleaseMsg`
- Async initialization using `oneshot_command`

**Repository Pattern**: Database access is abstracted through repositories in `database/src/repository/`:
- Each entity has its own repository (e.g., `software_title_repository.rs`)
- Managed centrally by `RepositoryManager`
- All repositories use async SQLx operations

**Type Conversion**: The codebase maintains separation between database models and core types:
- `FileType` exists in both `database/src/models.rs` and `core_types/src/lib.rs`
- Conversion traits are implemented between these types
- Comment in models.rs indicates intention to consolidate to core_types only

### File Type Management
The application handles various emulation file types:
- Rom, DiskImage, TapeImage (executable files)
- Screenshot, TitleScreen, LoadingScreen (images)
- Manual, ManualScan, CoverScan, MediaScan, PackageScan, InlayScan (documentation/media)
- MemorySnapshot (emulator save states)

### State Management
- Uses `Arc` for shared ownership of services between components
- `OnceCell` for lazy initialization of services in components
- SQLite database provides persistent state
- Settings are loaded and shared as `Arc<Settings>`

## Workspace Structure

The project uses Cargo workspace with the following crates:
- `relm4-ui` - Main GTK4 application (note: package name is `relm4-ui-test-simle`)
- `database` - Database operations and models
- `service` - Business logic and view models
- `core_types` - Shared type definitions
- `utils` - Common utilities
- `file_import`/`file_export` - File management operations
- `file_system` - Path resolution utilities
- `emulator_runner` - Emulator execution
- `thumbnails` - Image thumbnail generation

## Development Notes

- The main application window title is "EFCM" (Emulation File Collection Manager)
- Uses `TypedListView` from relm4 for efficient list rendering
- GUI layout uses `gtk::Paned` for split-panel interface
- Database URL is configured via `.env` file in the database directory
- Runtime database auto-creates on application start if missing