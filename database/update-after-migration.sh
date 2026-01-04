#!/bin/bash

# Script to run after adding/modifying database migrations
# This ensures all documentation and offline query data stays in sync

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔄 Updating database schema documentation and SQLx offline data..."
echo

# Step 1: Run migrations
echo "📋 Step 1/3: Running database migrations..."
cd "$SCRIPT_DIR"
sqlx migrate run
echo "✅ Migrations applied"
echo

# Step 2: Regenerate schema documentation
echo "📚 Step 2/3: Regenerating schema documentation with tbls..."
cd "$WORKSPACE_ROOT"
tbls doc --force
echo "✅ Schema documentation updated"
echo

# Step 3: Regenerate SQLx offline query data
echo "💾 Step 3/3: Regenerating SQLx offline query data..."
cargo sqlx prepare --workspace -- --all-targets
echo "✅ SQLx offline data updated"
echo

echo "🎉 Done! Don't forget to:"
echo "   1. Review the changes: git status"
echo "   2. Commit the updated files: git add database/docs/schema/ .sqlx/ && git commit"
