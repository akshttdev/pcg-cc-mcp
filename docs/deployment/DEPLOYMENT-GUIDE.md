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
| `HOST` | `0.0.0.0` | Backend bind address |
| `RUST_LOG` | `info` | Log level (`debug` for development) |

**Build toolchain (set by Flox, or add to `.env` if not using Flox):**

| Variable | Value | Why |
|----------|-------|-----|
| `CMAKE_POLICY_VERSION_MINIMUM` | `3.5` | audiopus_sys build requires this |
| `SQLX_OFFLINE` | `true` | Use cached query metadata at compile time |
| `RUSTC_WRAPPER` | `sccache` | Shared compilation cache (optional, faster rebuilds) |

**Secrets (add to `.env`, never commit):**

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Claude AI integration |
| `OPENAI_API_KEY` | NORA AI assistant (required for NORA) |
| `GITHUB_TOKEN` | GitHub OAuth / E2E tests |

**Feature-specific env vars** are documented in each sprint's planning file under "Deployment Instructions". Check the active branch's planning doc for any new env vars introduced by that branch.

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

Migrations run automatically on server startup via SQLx. All migrations live in `crates/db/migrations/` and follow the naming convention `YYYYMMDDHHMMSS_description.sql`.

**Branch-specific migration details** (new columns, seed data, rollback plans) are documented in each sprint's planning file under "Deployment Instructions".

General rules:
- Migrations should be additive (add columns, don't drop them)
- New columns should be nullable or have defaults (safe for existing data)
- Use `IF NOT EXISTS` for index creation (idempotent)
- After adding migrations, run `cargo sqlx prepare --workspace` and commit the `.sqlx/` cache

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
