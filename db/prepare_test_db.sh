#!/bin/bash
set -e

# This script prepares the database for compile-time verification by sqlx.

# If sqlx-cli is not installed, install it.
if ! [ -x "$(command -v sqlx)" ]; then
  echo 'sqlx-cli not found, installing...'
  cargo install sqlx-cli --no-default-features --features sqlite,rustls
fi

# Create the database file if it doesn't exist.
if [ ! -f "test.db" ]; then
    sqlx database create --database-url "sqlite:test.db"
fi

# Run migrations.
sqlx migrate run --database-url "sqlite:test.db"
