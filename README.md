# PCG Dashboard MCP

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
cd pcg-dashboard-mcp

# 2. Activate the flox environment (installs all tooling automatically)
flox activate

# 3. Install frontend dependencies
pnpm install

# 4. Start development servers (frontend + backend with hot reload)
npm run dev
```

On first `flox activate`, the Rust nightly toolchain is installed automatically via rustup (this may take a minute). Subsequent activations are instant.

### What Happens on `npm run dev`

- Copies the seed database from `dev_assets_seed/` to `dev_assets/` (first run only)
- Starts the Vite frontend dev server (default port 3000)
- Starts the Rust backend server via `cargo-watch` (auto-assigned port)
- Enables hot reload for both frontend and backend
- Frontend proxies API requests to the backend automatically

The app will be available at **http://localhost:3000**

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
5. **sccache** and **lld** (optional, for faster builds)

### Docker Deployment (Production)

```bash
# 1. Copy environment file and configure
cp .env.example .env

# 2. Deploy
./scripts/deploy.sh
# or manually: docker-compose build && docker-compose up -d
```

See **[docs/deployment/docker/DOCKER_DEPLOYMENT.md](docs/deployment/docker/DOCKER_DEPLOYMENT.md)** for the complete deployment guide.

### Building for Production

```bash
# Build the NPX CLI package
npm run build:npx

# Test the built package
npm run test:npm
```

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

The following variables are set automatically by the flox environment (`.flox/env/manifest.toml`):
- `SQLX_OFFLINE=true` — use cached query metadata instead of a live database connection
- `RUSTC_WRAPPER=sccache` — enable compilation caching

### Build-time
- `GITHUB_CLIENT_ID`: GitHub OAuth app ID (optional, defaults to Bloop AI's app)
- `POSTHOG_API_KEY`: Analytics key (optional)

### Runtime
- `BACKEND_PORT`: Backend server port (default: auto-assign)
- `FRONTEND_PORT`: Frontend dev port (default: 3000)
- `HOST`: Backend host (default: 127.0.0.1)
- `DATABASE_URL`: SQLite database path (default: `sqlite://dev_assets/db.sqlite`)
- `RUST_LOG`: Log level (default: `debug` in dev)
- `DISABLE_WORKTREE_ORPHAN_CLEANUP`: Debug flag for worktrees

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
