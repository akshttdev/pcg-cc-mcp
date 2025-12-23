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

Before running this project, you need to install:

1. **Node.js 18+** and **npm** (or **pnpm 8+**)
   - Check: `node --version` and `npm --version`
   - Install from: https://nodejs.org/

2. **Rust toolchain** (nightly version specified in rust-toolchain.toml)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

3. **cargo-watch** (for backend hot reload)
   ```bash
   cargo install cargo-watch
   ```

4. **sqlx-cli** (for database migrations)
   ```bash
   cargo install sqlx-cli --no-default-features --features sqlite
   ```

5. **(Optional) pnpm** for faster package management
   ```bash
   npm install -g pnpm
   ```

### 🐳 Docker Deployment (Recommended for Production)

The easiest way to deploy is using Docker with Cloudflare Tunnel for secure port forwarding:

```bash
# 1. Copy environment file
cp .env.example .env

# 2. Get Cloudflare Tunnel token from https://one.dash.cloudflare.com/
#    and add it to .env

# 3. Run the deployment script
./deploy.sh
```

**Or manually:**
```bash
docker-compose build
docker-compose up -d
```

See **[DOCKER_DEPLOYMENT.md](DOCKER_DEPLOYMENT.md)** for complete deployment guide.

**Pull from Docker Hub (when available):**
```bash
docker pull kingbodhi/pcg-cc-mcp:latest
```

### Development Setup

```bash
# Clone the repository
git clone <repository-url>
cd pcg-dashboard-mcp

# Install dependencies
pnpm install
# or if you don't have pnpm: npm install

# Start development servers (frontend + backend)
pnpm run dev
# or: npm run dev
```

This will:
- Auto-copy the development database from `dev_assets_seed/` (first run only)
- Start frontend dev server on port 3000 (or auto-assigned)
- Start backend server on auto-assigned port
- Enable hot reload for both frontend and backend

The frontend will be available at http://localhost:3000

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

### Build-time
- `GITHUB_CLIENT_ID`: GitHub OAuth app ID (optional, defaults to Bloop AI's app)
- `POSTHOG_API_KEY`: Analytics key (optional)

### Runtime
- `BACKEND_PORT`: Backend server port (default: auto-assign)
- `FRONTEND_PORT`: Frontend dev port (default: 3000)
- `HOST`: Backend host (default: 127.0.0.1)
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
