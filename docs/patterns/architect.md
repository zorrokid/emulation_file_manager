# Architecture Patterns

Reference guide for architectural decisions in the **Emulation File Manager** — a multi-layered Rust workspace with GTK4 UI (relm4) and SQLite database.

## Project Context

### Domain Model
- **System**: Platforms like Commodore 64, NES
- **Software Title/Release**: Games or applications for systems
- **File Set**: Collections of files with same FileType (ROMs, documents, images)
- **File Info**: Individual files with SHA1 checksums, zstd compression
- **Release Items**: Physical items tracking (Disk 1, Manual, Box)
- **Emulators/Viewers**: Configured launchers for files

### Architecture Principles

**4-Layer Architecture:**
1. **Core crates** (`core_types`, `utils`, etc.) - No dependencies on other project crates
2. **Database crate** - May depend on core crates only
3. **Service crate** - May depend on core and database crates
4. **GUI crate** (`relm4-ui`) - May depend on all layers below

**Key Rules:**
- **Maintain layer boundaries**: Upper layers can depend on lower layers, never reverse
- **Core crates are pure**: No dependencies on application-specific logic
- **Service layer contains business logic**: Not in database or GUI
- **Database crate owns data access**: SQLx queries, schema migrations

### Technologies
- **Rust**: Primary language with async/await
- **relm4**: GTK4 reactive UI framework
- **SQLx**: Async SQLite with compile-time query checking
- **rust-s3**: S3-compatible cloud storage
- **zstd**: File compression

### File Management Design
- Files stored with unique names, zstd compressed
- Files can belong to multiple file sets (many-to-many)
- File sets have single FileType (ROM, Document, Image, etc.)
- File exports use file-set-specific names
- Cloud storage organized by file type
- Two-level linking: release→file_set (primary), file_set→item (metadata)

## Decision Framework

For new features, consider:
- **Which layer?** Core types, database access, business logic, or UI?
- **Which crate?** Does it fit existing crates or need a new one?
- **Dependencies?** Will it break layer boundaries?
- **Database impact?** New tables, migrations, queries?
- **File system impact?** Storage, compression, export, cloud sync?

## Examples

- "Where should I implement multi-file export logic?"
  → Service layer (`file_export` crate), uses database crate for queries, file_system for storage
  
- "How do I add a new FileType?"
  → Add to `core_types::FileType` enum, update database schema, add storage paths, consider UI filtering

- "Should I add S3 logic to database crate?"
  → No, violates layer boundaries. Keep in `cloud_storage` or service layer, database crate only handles local data

- "How to handle emulator configuration?"
  → Core type for config structure, database for persistence, service layer for launch logic, separate `executable_runner` crate for execution

Always reason about decisions using the project's established principles.
