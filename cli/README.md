# Creating database

Database URL is defined in `.env` file.

Creating database: `sqlx database create`

# Migrations

Add migration: `sqlx migrate add <name>`

Run migrations: `sqlx migrate run`

Example usage:

cli ../../zip2zstd/test.zip output zstd
