# ORCHA — Deployment Guide

> Last updated: 2026-03-23

## Local Development

### Prerequisites

- [Flox](https://flox.dev/docs/install-flox/) (recommended) — manages Rust nightly, Node.js, pnpm, cmake, sqlx-cli, sccache
- OR manually: Node.js 18+, pnpm 8+, Rust nightly-2025-05-18, cmake, pkg-config, libopus, sqlx-cli

### First-Time Setup

```bash
git clone <repo-url> && cd pcg-cc-mcp
flox activate           # installs toolchain, seeds database, sets env vars
pnpm install            # install frontend deps (also activates git hooks via prepare script)
pnpm run dev            # starts frontend + backend with hot reload
```

The app will be available at `http://localhost:$FRONTEND_PORT` (default 3000).

### Environment Variables

Two configuration layers:

| Layer | File | When to use |
|-------|------|-------------|
| **Flox** | `.flox/env/manifest.toml` | Build toolchain vars (CMAKE, SQLX_OFFLINE, RUSTC_WRAPPER). Shared across machines. |
| **.env** | `.env` (gitignored) | Secrets (API keys), ports, per-worktree overrides. |

**Required in `.env`:**

| Variable | Default | Description |
|----------|---------|-------------|
| `FRONTEND_PORT` | `3000` | Vite dev server port |
| `BACKEND_PORT` | `3002` | Axum backend port |
| `DATABASE_URL` | `sqlite://dev_assets/db.sqlite` | SQLite path |

**Build toolchain (set by Flox, or add to `.env` if not using Flox):**

| Variable | Value | Why |
|----------|-------|-----|
| `CMAKE_POLICY_VERSION_MINIMUM` | `3.5` | audiopus_sys build requires this |
| `SQLX_OFFLINE` | `true` | Use cached query metadata at compile time |

**Optional — Agent Flow Engine:**

| Variable | Default | Description |
|----------|---------|-------------|
| `ENABLE_AGENT_FLOW_ENGINE` | `0` | Set to `1` to enable AI agent auto-execution |
| `AGENT_FLOW_POLL_INTERVAL` | `15` | Seconds between executor polls |
| `AGENT_FLOW_MAX_CONCURRENT` | `5` | Max flows per tick |

### Git Worktree Port Isolation

When using git worktrees for parallel development, each worktree needs unique ports in its own `.env`:

```bash
# Root worktree
FRONTEND_PORT=3000
BACKEND_PORT=3002

# Agent worktree
FRONTEND_PORT=3010
BACKEND_PORT=3012
```

### Pre-Commit Hooks

Auto-installed via `pnpm install` (npm `prepare` script). Handles:
- Rust formatting on staged `.rs` files
- Frontend lint-staged (ESLint + Prettier) on staged `.ts`/`.tsx` files
- Type generation reminders when `#[derive(TS)]` changes

### Database

The dev database is seeded from `dev_assets_seed/db.sqlite` on first `flox activate` or manually:

```bash
cp dev_assets_seed/db.sqlite dev_assets/db.sqlite
```

After adding new migrations:
```bash
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace
```

After modifying Rust types with `#[derive(TS)]`:
```bash
npm run generate-types
```

---

## Docker Deployment

```bash
docker compose up                                    # full stack
docker compose -f docker-compose.local.yml up        # local variant
```

See `docker-compose.yml` and `Dockerfile` for details.

---

## Production Build

```bash
# Full production build
./scripts/build-npm-package.sh

# Or manually:
cargo build --release --bin server
cd frontend && npx vite build
```

---

## Database Migrations

Migrations run automatically on server startup. Current migrations:

| Migration | What it does |
|-----------|-------------|
| `20260414000000` | Adds `stage_config` to pipeline stages, `crm_deal_id`/`cancel_deadline`/`retry_count`/`last_error` to agent_flows |
| `20260414000001` | Seeds default stage_config JSON for all pipeline stages |

All migrations are **additive** (nullable columns, `IF NOT EXISTS` indexes). Safe to deploy without downtime.

### Rollback

All columns are nullable — old code ignores them. The agent flow engine is gated behind `ENABLE_AGENT_FLOW_ENGINE=1` (default off). To disable after deploy, unset the env var.

---

## Troubleshooting

### Flox cache errors (`sed: can't read .../del.env`)

```bash
ls ~/.cache/flox/run/
rm -rf ~/.cache/flox/run/<project-hash>/
flox activate   # creates fresh activation
```

### CMake error: "Compatibility with CMake < 3.5 has been removed"

```bash
export CMAKE_POLICY_VERSION_MINIMUM=3.5
```

Flox sets this automatically. If not using Flox, add to `.env`.

### Database migration error on startup

```bash
rm -f dev_assets/db.sqlite
cp dev_assets_seed/db.sqlite dev_assets/db.sqlite
```

If the seed itself is outdated, regenerate: `./scripts/create-test-seed.sh`

### Vite module resolution error ("Failed to fetch dynamically imported module")

```bash
rm -rf frontend/node_modules/.vite
# Restart dev servers
```

---

## Architecture Reference

See `CLAUDE.md` for full architecture docs including:
- Project structure (`crates/`, `frontend/`, `shared/`)
- Key patterns (SSE streaming, executor pattern, MCP integration)
- API patterns and authentication
- Testing strategy

## Archived Deployment Docs

Previous deployment guides for APN/Pythia, Docker/Nginx, and storage provider setup are in `docs/deployment/archive/`.
