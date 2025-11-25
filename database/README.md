# Creating database

## development database

Database URL is defined in `.env` file.

Creating database: `sqlx database create`

Dev db is created to data/db.sqlite 

## runtime database

When running application, runtime db is created to ~/.local/share/efm/db.sqlite in Linux 

To reset db it can be simply deleted, new db will be created when starting application again.

# Migrations

Add migration: `sqlx migrate add <name>`

Run migrations: `sqlx migrate run`

Migrations are automatically run at application startup via `sqlx::migrate!().run(&pool)` in `lib.rs`.

## Updating Schema Documentation

After creating or running migrations, regenerate the schema documentation:

```bash
# From the workspace root
tbls doc
```

This updates the ER diagrams and table documentation in `docs/schema/`. Commit the updated documentation along with your migrations.

# SQLx Offline Mode

This project uses SQLx's offline mode for CI/CD. This allows the CI build to succeed without needing a database connection during compilation.

## Why Offline Mode?

SQLx verifies SQL queries at compile time by connecting to a database. In CI, we don't have a database set up, so we use offline mode which relies on pre-generated query metadata instead of a live database connection.

## Generating Offline Query Data

Whenever you modify SQL queries (add/change/remove `sqlx::query!` or `sqlx::query_as!` calls), you must regenerate the offline query data:

```bash
# From the workspace root (includes test queries)
cargo sqlx prepare --workspace -- --all-targets

# Or without test queries (not recommended for CI)
cargo sqlx prepare --workspace
```

This will update the `.sqlx/` directory in the workspace root with JSON files containing query metadata.

**Important:** Commit the updated `.sqlx/` files to git! CI needs these files to build successfully.

## Verifying Offline Mode

To verify your queries work in offline mode (same as CI):

```bash
SQLX_OFFLINE=true cargo check
```

## Check Offline Data is Up-to-Date

To verify that the offline data matches your current queries without regenerating:

```bash
cargo sqlx prepare --check
```

This is useful in CI or pre-commit hooks to ensure developers haven't forgotten to update the offline data.




