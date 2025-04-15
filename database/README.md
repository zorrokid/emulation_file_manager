# Creating database

Database URL is defined in `.env` file.

Creating database: `sqlx database create`

# Migrations

Add migration: `sqlx migrate add <name>`

Run migrations: `sqlx migrate run`

# Check 

cargo sqlx prepare --check


