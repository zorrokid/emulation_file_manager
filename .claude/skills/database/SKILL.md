---
name: database
description: Use when adding or modifying database migrations, repositories, models, or SQLx queries in the database crate. Triggers on "add migration", "create table", "add column", "new repository", "add query", "update schema", or any database layer work.
version: 1.0.0
---

# Database Skill

You are implementing database layer changes in the **Emulation File Manager** using SQLx 0.8.6 + SQLite.

## Primary Reference

**Always read `docs/patterns/database.md` first.** It contains the canonical patterns for this project:
- Repository pattern and RepositoryManager
- Migration workflow and naming conventions
- Schema conventions (tables, foreign keys, cascade rules)
- SQLx offline mode (critical for CI)
- Query patterns (`query!`, `query_as!`, `QueryBuilder`)
- Type conversions (FileType, Sha1Checksum, dates)
- Testing with in-memory SQLite

Do not deviate from those patterns. Do not duplicate them here.

## Migration Checklist

Every migration requires all three steps — do not skip any:

1. Create migration file: `sqlx migrate add <name>` (run from `database/` directory)
2. Regenerate offline data: `cargo sqlx prepare --workspace -- --all-targets` (from workspace root)
3. Update schema docs: `tbls doc` (from workspace root)

Commit migration file + `.sqlx/` metadata + `docs/schema/` together.

## Repository Checklist

When adding a new repository:

1. Create `database/src/repository/<name>_repository.rs`
2. Expose it in `database/src/repository/mod.rs`
3. Add field and accessor to `RepositoryManager` in `database/src/repository_manager.rs`

## Quick Reference

| Task | Location |
|---|---|
| New migration | `database/migrations/` |
| Model structs | `database/src/models.rs` |
| Repository implementations | `database/src/repository/` |
| RepositoryManager | `database/src/repository_manager.rs` |
| Pool setup & migration run | `database/src/lib.rs` |
| Offline query metadata | `.sqlx/` (workspace root) |
| Schema ER diagrams | `database/docs/schema/` |
