#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEV_ASSETS="$REPO_ROOT/dev_assets"
SEED_DIR="$REPO_ROOT/dev_assets_seed"

ensure_dev_assets() {
  if [ ! -d "$DEV_ASSETS" ]; then
    if [ -d "$SEED_DIR" ]; then
      echo "[pcg-cc-mcp] Seeding dev assets from dev_assets_seed" >&2
      cp -R "$SEED_DIR" "$DEV_ASSETS"
    else
      echo "[pcg-cc-mcp] dev_assets_seed missing; creating empty dev_assets" >&2
      mkdir -p "$DEV_ASSETS"
    fi
  fi

  if [ -d "$SEED_DIR" ]; then
    if [ ! -f "$DEV_ASSETS/db.sqlite" ] && [ -f "$SEED_DIR/db.sqlite" ]; then
      cp "$SEED_DIR/db.sqlite" "$DEV_ASSETS/db.sqlite"
    fi
    if [ ! -f "$DEV_ASSETS/config.json" ] && [ -f "$SEED_DIR/config.json" ]; then
      cp "$SEED_DIR/config.json" "$DEV_ASSETS/config.json"
    fi
  fi
}

ensure_dev_assets

export RUST_LOG="${RUST_LOG:-info}"
cd "$REPO_ROOT"
exec cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p server --bin mcp_task_server "$@"
