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

# Check 

cargo sqlx prepare --check
