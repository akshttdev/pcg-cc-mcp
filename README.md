# ORCHA — AI-Powered CRM & Task Platform

A comprehensive project management dashboard with built-in Model Context Protocol (MCP) server for AI agent integration.

## Features

- **Project Management**: Full-featured kanban board for task tracking
- **AI Agent Integration**: Built-in MCP server for seamless AI workflows
- **Real-time Updates**: Server-sent events for live collaboration
- **Git Integration**: Automated worktree management for isolated task execution
- **On-chain Identity**: Every account ships with an Aptos wallet; manage balances and activity under Settings → Wallet.
- **Brand Pods & Asset Vault**: Break projects into goal-focused pods and catalogue brand assets (logos, guides, transcripts) per client engagement.
- **Multi-platform Support**: Cross-platform desktop and web application

## Tech Stack

- **Backend**: Rust with Axum web framework, Tokio async runtime, SQLx
- **Frontend**: React 18 + TypeScript + Vite, Tailwind CSS, shadcn/ui components
- **Database**: SQLite with SQLx migrations
- **Type Sharing**: ts-rs generates TypeScript types from Rust structs
- **MCP Server**: Built-in Model Context Protocol server for AI agent integration

## Quick Start

### Prerequisites

Install [Flox](https://flox.dev/docs/install-flox/) — it manages all project dependencies (Node.js, Rust, pnpm, cargo-watch, sqlx-cli, sccache, lld, etc.) automatically.

```bash
# macOS
brew install flox
```

### First-Time Setup

```bash
# 1. Clone the repository
git clone <repository-url>
cd pcg-cc-mcp

# 2. Activate the flox environment (installs all tooling automatically)
flox activate

# 3. Install frontend dependencies
pnpm install

# 4. Activate pre-commit hooks
git config core.hooksPath .githooks

# 5. Start development servers (frontend + backend with hot reload)
pnpm run dev
```

On first `flox activate`, the Rust nightly toolchain is installed automatically via rustup (this may take a minute). Subsequent activations are instant.

### What Happens on `pnpm run dev`

- Copies the seed database from `dev_assets_seed/` to `dev_assets/` (first run only)
- Starts the Vite frontend dev server (port from `FRONTEND_PORT` in `.env`, default 3000)
- Starts the Rust backend server via `cargo-watch` (port from `BACKEND_PORT` in `.env`, default 3002)
- Enables hot reload for both frontend and backend
- Frontend proxies API requests to the backend automatically

The app will be available at `http://localhost:$FRONTEND_PORT`

Default login: `admin` / `admin123`

### Build Optimizations

The dev environment includes build speed tools configured in `.cargo/config.toml`:
- **sccache** — caches compiled crates across `cargo clean` and branch switches
- **lld** — faster linker for macOS (replaces default ld)

These are installed automatically via the flox environment.

### Manual Setup (without Flox)

If you prefer not to use flox, install these manually:

1. **Node.js 18+** and **pnpm 8+**
2. **Rust nightly** (version pinned in `rust-toolchain.toml`)
3. **cargo-watch** (`cargo install cargo-watch`)
4. **sqlx-cli** (`cargo install sqlx-cli --no-default-features --features sqlite`)
5. **cmake**, **pkg-config**, **libopus** (system packages)
6. **sccache** and **lld** (optional, for faster builds)

You must also set these environment variables (flox sets them automatically):

```bash
export SQLX_OFFLINE=true
export CMAKE_POLICY_VERSION_MINIMUM=3.5   # required — audiopus_sys build fails without this
export RUSTC_WRAPPER=sccache              # optional — faster rebuilds
```

Add them to your shell profile or `.env` file.

### Docker Deployment (Production)

```bash
docker compose up                                   # full stack
docker compose -f docker-compose.local.yml up       # local variant
```

### Building for Production

```bash
# Full production build
./scripts/build-npm-package.sh

# Or manually:
cargo build --release --bin server
cd frontend && npx vite build
```

---

## Environment Configuration

This project has two layers of environment configuration. Understanding when to use each is important.

### `.env` — per-machine, per-worktree settings

Each developer (and each git worktree) has its own `.env` file. This file is **gitignored** and holds secrets + local overrides.

| Variable | Default | Description |
|----------|---------|-------------|
| `FRONTEND_PORT` | `3000` | Vite dev server port |
| `BACKEND_PORT` | `3002` | Axum backend port |
| `DATABASE_URL` | `sqlite://dev_assets/db.sqlite` | SQLite path |
| `HOST` | `0.0.0.0` | Backend bind address |
| `RUST_LOG` | `info` | Log level (`debug` for development) |
| `CMAKE_POLICY_VERSION_MINIMUM` | — | Set to `3.5` (flox sets this; add to `.env` if not using flox) |
| `SQLX_OFFLINE` | — | Set to `true` (flox sets this; add to `.env` if not using flox) |

**Secrets** (add to `.env`, never commit):

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Claude AI integration |
| `OPENAI_API_KEY` | NORA AI assistant (required for NORA) |
| `GITHUB_TOKEN` | GitHub OAuth / E2E tests |
| `CLOUDFLARE_TUNNEL_TOKEN` | Production tunnel |

### `.flox/env/manifest.toml` — reproducible toolchain

Flox provides deterministic versions of all build tools and sets env vars like `SQLX_OFFLINE`, `CMAKE_POLICY_VERSION_MINIMUM`, and `RUSTC_WRAPPER` automatically.

**When to edit `manifest.toml`:**
- Adding a new system dependency (C library, CLI tool)
- Changing the Rust toolchain version
- Adding env vars that should be consistent across all machines

**When to edit `.env`:**
- Secrets (API keys, tokens)
- Per-machine overrides (ports, paths, log level)
- Per-worktree isolation (different ports per worktree)

**Rule of thumb:** If flox is active, it provides the build toolchain vars. `.env` provides secrets and port overrides. If you're not using flox, `.env` must provide both.

### Worktree port isolation

When using git worktrees for parallel development, each worktree needs unique ports in its own `.env`:

```bash
# Root worktree (.env)
FRONTEND_PORT=3000
BACKEND_PORT=3002

# Agent worktree (.env)
FRONTEND_PORT=3010
BACKEND_PORT=3012
```

Before starting servers, check for port conflicts:
```bash
source .env
lsof -i :$FRONTEND_PORT -P | grep LISTEN
lsof -i :$BACKEND_PORT -P | grep LISTEN
```

---

## Troubleshooting

### Flox cache errors (`sed: can't read .../del.env`)

The flox activation cache is stale. Fix:

```bash
ls ~/.cache/flox/run/
rm -rf ~/.cache/flox/run/<project-hash>/
flox activate   # creates fresh activation
```

This only removes cached runtime state — your environment definition (`.flox/env/manifest.toml`) is untouched.

### Database migration error on startup

If the backend crashes with `SqliteError: no such column: ...` or `UNIQUE constraint failed: _sqlx_migrations.version`, the seed database is out of sync with the current branch's migrations.

**Quick fix** — reseed from `dev_assets_seed/`:
```bash
rm -f dev_assets/db.sqlite
cp dev_assets_seed/db.sqlite dev_assets/db.sqlite
# Then restart the backend
```

**If the seed itself is outdated** (migration still fails after reseeding), regenerate it:
```bash
./scripts/create-test-seed.sh
# Or manually: delete dev_assets/db.sqlite, let the server create a fresh one,
# then copy it back to dev_assets_seed/ for future use
```

**Prevention**: After adding new migrations, always update `dev_assets_seed/db.sqlite` so other developers (and future checkouts) start with a compatible seed. The seed should be committed when migrations change.

### CMake error: "Compatibility with CMake < 3.5 has been removed"

The `audiopus_sys` crate requires `cmake_minimum_required(VERSION 2.x)` which CMake 4.x rejects. Set:

```bash
export CMAKE_POLICY_VERSION_MINIMUM=3.5
```

Flox sets this automatically. If not using flox, add it to `.env` or your shell profile. The `backend:dev:watch` npm script also sets it inline.

## Development Commands

### Core Development
```bash
# Start both servers with hot reload
pnpm run dev

# Individual servers
npm run frontend:dev    # Frontend only
npm run backend:dev     # Backend only

# Run all checks (linting, type checking)
npm run check

# Lint (proxies to the frontend ESLint config)
pnpm run lint
```

> **Note:** `pnpm run lint` mirrors the frontend ESLint configuration. It currently surfaces the outstanding warnings/errors in the dashboard so the team can address them incrementally.

### Frontend Commands
```bash
cd frontend
npm run lint           # ESLint with TypeScript
npm run lint:fix       # Auto-fix ESLint issues
npm run format         # Prettier formatting
npm run build          # Production build
```

### Backend Commands
```bash
# Run tests
cargo test --workspace
cargo test -p <crate_name>     # Test specific crate

# Code quality
cargo fmt --all                # Format code
cargo clippy --all --all-targets --all-features -- -D warnings  # Linting
cargo check                    # Quick compilation check

# Type generation (after modifying Rust types)
npm run generate-types
npm run generate-types:check   # Verify types are up to date
```

## Additional Documentation

- [Project Drawer Visualization Concepts](docs/project_drawer_visualizations.md) — 31 experimental ways to visualize the new project → board → pod → task/asset structure, complete with ASCII sketches.

## Project Structure

```
crates/
├── server/              # Axum HTTP server, API routes, MCP server
├── db/                  # Database models, migrations, SQLx queries
├── executors/           # AI coding agent integrations (Claude, Gemini, etc.)
├── services/            # Business logic, GitHub auth, git operations
├── local-deployment/    # Local deployment logic
└── utils/              # Shared utilities

frontend/               # React application
├── src/
│   ├── components/     # React components (TaskCard, ProjectCard, etc.)
│   ├── pages/         # Route pages
│   ├── hooks/         # Custom React hooks (useEventSourceManager, etc.)
│   └── lib/           # API client, utilities

shared/types.ts        # Auto-generated TypeScript types from Rust
```

## Key Features

### Event Streaming
Real-time updates via Server-Sent Events:
- Process logs: `/api/events/processes/:id/logs`
- Task diffs: `/api/events/task-attempts/:id/diff`

### Git Worktree Management
- Isolated execution environment for each task
- Automatic cleanup of orphaned worktrees
- Managed by `WorktreeManager` service

### MCP Integration
Duck Kanban acts as MCP server providing tools:
- `list_projects`, `list_tasks`, `create_task`, `update_task`
- AI agents can manage tasks via MCP protocol

### Executor Pattern
Pluggable AI agent executors with actions:
- `coding_agent_initial`, `coding_agent_follow_up`, `script`
- Support for Claude, Gemini, and other AI providers

### Specialized Agents
- **Editron** – Post-Production Architect that ingests Dropbox batches, analyzes footage, synthesizes story blueprints, and drives iMovie/Compressor automations for recaps, highlight reels, and social hooks. See [`docs/editron-pipeline.md`](docs/editron-pipeline.md).
- **Master Cinematographer (Spectra)** – AI cinematic specialist that generates Stable Diffusion / Runway motion plates, LUT kits, and typography packs Editron can pull into the motion systems tier.

## Database Operations

```bash
# SQLx migrations
sqlx migrate run        # Apply migrations
sqlx database create    # Create database

# Database is auto-copied from dev_assets_seed/ on dev server start
```

> **Schema update (2025-10-02):** Run `sqlx migrate run` followed by `pnpm run generate-types` to pick up the new `project_pods` and `project_assets` tables.

## Environment Variables

See [Environment Configuration](#environment-configuration) above for the full guide on `.env` vs `.flox/env/manifest.toml`.

### Pre-commit hooks

Activate once after cloning:

```bash
git config core.hooksPath .githooks
```

This enables:
- Rust formatting on staged `.rs` files
- Frontend lint-staged (ESLint + Prettier) on staged `.ts`/`.tsx` files
- Type generation reminders when `#[derive(TS)]` changes

### After modifying Rust types or queries

```bash
# If you changed SQL queries:
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace

# If you added/changed #[derive(TS)] structs:
npm run generate-types

# Commit the updated .sqlx/ directory and shared/types.ts
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make changes following existing patterns
4. Run tests and checks: `npm run check`
5. Commit changes: `git commit -m 'Add amazing feature'`
6. Push to branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

## Architecture

The application follows a modular architecture:

- **REST API**: All endpoints under `/api/*`
- **Authentication**: GitHub OAuth (device flow)
- **Database Layer**: All queries in `crates/db/src/models/`
- **Frontend Proxy**: Vite dev server proxies to backend
- **Component Patterns**: Consistent patterns in `frontend/src/components/`

## License

See [LICENSE](LICENSE) file for details.
