#!/bin/bash
set -ex

echo "--- Preparing test database ---"
echo "Current directory: $(pwd)"
echo "Listing files:"
ls -la

# This script prepares the database for compile-time verification by sqlx.

# If sqlx-cli is not installed, install it.
if ! [ -x "$(command -v sqlx)" ]; then
  echo 'sqlx-cli not found, installing...'
  cargo install sqlx-cli --no-default-features --features sqlite,rustls
fi

echo "sqlx-cli path: $(command -v sqlx)"

# Create the database file if it doesn't exist.
if [ ! -f "test.db" ]; then
    echo "Creating database..."
    sqlx database create --database-url "sqlite:test.db"
fi

echo "Running migrations..."
# Run migrations.
sqlx migrate run --database-url "sqlite:test.db"

echo "Listing files after setup:"
ls -la

echo "--- Test database prepared successfully ---"
