---
name: database
description: >
  Use when adding or modifying database migrations, repositories, models, or SQLx queries
  in the database crate. Triggers on "add migration", "create table", "add column",
  "new repository", "add query", "update schema", or any database layer work.
---

You are a database engineer with deep expertise in the **Emulation File Manager** project.
You implement SQLx + SQLite database changes following strict patterns and workflows.

---

## Mandatory Workflow After Any Schema or Query Change

Run these steps **in order** — do not skip or reorder them:

```bash
# 1. Apply new migrations to the live database (tbls reads this DB)
cargo sqlx migrate run --source database/migrations --database-url sqlite://database/data/db.sqlite

# 2. Regenerate .sqlx offline query metadata (CI fails without this)
cargo sqlx prepare --workspace -- --all-targets

# 3. Regenerate ER diagrams from the now-migrated live database
tbls doc --force
```

Commit migration file + `.sqlx/` metadata + `database/docs/schema/` **together** in one commit.

> **Critical:** `tbls doc` reads the live DB at `database/data/db.sqlite`. Running it before
> step 1 silently produces stale docs reflecting the old schema.

---

## Project Structure

```
database/
├── migrations/          # SQLx migration files (timestamp-prefixed)
├── src/
│   ├── lib.rs           # Pool setup, test helpers (setup_test_db, setup_test_repository_manager)
│   ├── models.rs        # DB model structs (FileInfo, FileSetFileInfo, …)
│   ├── repository/      # One file per entity
│   └── repository_manager.rs  # Aggregates all repositories
├── docs/schema/         # tbls-generated ER diagrams and schema.json
```

---

## Schema Conventions

| Convention | Rule |
|---|---|
| Table names | `snake_case` |
| Junction tables | `table1_table2` (alphabetical) |
| Primary key | `id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL` |
| Foreign keys | `{table}_id`; always specify `ON DELETE` behaviour |
| Timestamps | `TEXT` as ISO 8601 via `chrono` |
| Booleans | `INTEGER NOT NULL DEFAULT 0` (0 = false, 1 = true) |
| Nullable fields | Only when absence is semantically meaningful |

**`ON DELETE` policy:**
- `CASCADE` — dependent records are deleted with parent (e.g., `file_set_file_info` when `file_set` is deleted)
- `SET NULL` — FK becomes null (used sparingly)
- `RESTRICT` — prevent deletion if dependents exist

---

## SQLite Migration Constraints

SQLite has limited `ALTER TABLE` support:
- ✅ `ADD COLUMN` works
- ❌ `DROP COLUMN`, `RENAME COLUMN`, change `NOT NULL`, change `DEFAULT` — require recreate-table

**Recreate-table pattern:**

```sql
PRAGMA foreign_keys = OFF;

CREATE TABLE my_table_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    -- new definition here
);

INSERT INTO my_table_new SELECT ... FROM my_table;

DROP TABLE my_table;
ALTER TABLE my_table_new RENAME TO my_table;

-- Recreate any indexes that existed on the original table
-- CREATE INDEX ...

PRAGMA foreign_keys = ON;
```

---

## Repository Pattern

Each entity gets its own repository in `database/src/repository/`:

```rust
pub struct MyEntityRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl MyEntityRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self { Self { pool } }

    pub async fn add(&self, ...) -> Result<i64, Error> { ... }

    // Provide _with_tx variants when the operation participates in cross-entity transactions
    pub async fn add_with_tx(&self, tx: &mut Transaction<'_, Sqlite>, ...) -> Result<i64, Error> { ... }
}
```

After adding a new repository:
1. Add a field to `RepositoryManager` in `repository_manager.rs`
2. Initialise it in `RepositoryManager::new`
3. Add a getter method returning a reference

---

## Query Patterns

**Compile-time verified (preferred):**
```rust
// INSERT
let result = sqlx::query!(
    "INSERT INTO my_table (col1, col2) VALUES (?, ?)",
    val1, val2
).execute(&*self.pool).await?;

// SELECT with struct mapping
let rows = sqlx::query_as::<_, MyModel>(
    "SELECT id, col1, col2 FROM my_table WHERE id = ?"
).bind(id).fetch_all(&*self.pool).await?;
```

**Dynamic IN clauses (use QueryBuilder):**
```rust
let mut qb = QueryBuilder::<Sqlite>::new("SELECT * FROM my_table WHERE id IN (");
let mut sep = qb.separated(", ");
for id in ids { sep.push_bind(id); }
sep.push_unseparated(") AND active = true");
let rows = qb.build_query_as::<MyModel>().fetch_all(&*self.pool).await?;
```

**Never use string interpolation or format! to build queries — always use bind parameters.**

---

## Type Conversions

| Rust type | DB type | Conversion |
|---|---|---|
| `FileType` | `INTEGER` | `to_db_int()` / `from_db_int()` |
| `Sha1Checksum` ([u8; 20]) | `BLOB` | `.to_vec()` / `.try_into()` |
| `bool` | `INTEGER` | automatic via SQLx |
| `Option<String>` | `TEXT` (nullable) | automatic via SQLx |
| `chrono::NaiveDateTime` | `TEXT` | automatic via SQLx |

---

## Invariants to Enforce

- `is_available = true` ↔ `archive_file_name = Some(...)` on `file_info`
- When inserting into `file_info` via `add_file_info`, derive `is_available` from
  `archive_file_name.is_some()` — never rely on the column DEFAULT
- `update_is_available` must only be called when the file is genuinely becoming available

---

## Testing

**Always** use in-memory SQLite helpers — never a real file DB:

```rust
#[async_std::test]
async fn test_example() {
    let pool = database::setup_test_db().await;
    let repo = MyEntityRepository::new(Arc::new(pool));
    // ...
}
```

`setup_test_db()` returns a `Pool<Sqlite>` backed by `sqlite::memory:` with all migrations applied.
`setup_test_repository_manager()` wraps it in a full `RepositoryManager`.

---

## Checklist

After any database change:

- [ ] Migration SQL file created with descriptive name
- [ ] `cargo sqlx migrate run --source database/migrations --database-url sqlite://database/data/db.sqlite`
- [ ] `cargo sqlx prepare --workspace -- --all-targets`
- [ ] `tbls doc --force`
- [ ] `cargo test -p database` passes
- [ ] Migration file + `.sqlx/` + `database/docs/schema/` committed together
