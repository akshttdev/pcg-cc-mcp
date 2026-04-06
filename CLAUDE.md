# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Session Handoff

If `planning/SESSION-HANDOFF.md` exists, **read it first** — it contains the previous session's work state and remaining tasks. After reading, delete the file and continue with the remaining work. Use `/handoff` to create one before clearing context.

## Code Style — Modular & Composable

Write **small, pure, composable functions** that do one thing well:
- **Pure by default**: Functions should take inputs and return outputs. Minimize side effects — isolate I/O (database, network, filesystem) at the boundaries, keep business logic pure.
- **Compose, don't nest**: Build complex behavior by composing small functions, not by nesting control flow. A 50-line function with 4 levels of nesting should become 3-4 named functions piped together.
- **Immutability first**: Prefer transforming data over mutating it. Create new values instead of modifying existing ones. Mutation is acceptable only when performance requires it.
- **Data pipelines**: Express multi-step transformations as chains/pipelines — each step is a small function that receives data and returns transformed data.
- **Declarative over imperative**: Describe *what* the result should be, not *how* to compute it step-by-step. Prefer `filter`/`map`/`reduce` over manual loops with accumulators.
- **Single responsibility**: Each function does exactly one thing. If you're naming it `do_x_and_y`, split it.

## Worktree Awareness

This repo may use **git worktrees** for parallel development. Before making changes:
1. Run `git worktree list` and `pwd` to confirm which worktree you're in
2. Each worktree has its own `.env` with distinct `FRONTEND_PORT` / `BACKEND_PORT`
3. **Never switch worktrees without user approval** — staged changes are per-worktree
4. If a planning doc exists for the current branch, check its `Worktree` field matches your `pwd`

Not all developers use worktrees — single-repo workflows work normally. The checks above only matter when multiple worktrees exist.

## Essential Commands

### Local Development
```bash
# Start development servers with hot reload (frontend + backend)
pnpm run dev

# Individual dev servers
npm run frontend:dev    # Frontend only (uses FRONTEND_PORT from .env, default 3000)
npm run backend:dev     # Backend only (uses BACKEND_PORT from .env, default auto-assign)

# Build production version
./scripts/build-npm-package.sh
```

**Worktree port isolation**: Each worktree's `.env` defines unique ports to avoid conflicts:
- Root worktree: `FRONTEND_PORT=3000`, `BACKEND_PORT=3002`
- Agent worktrees: e.g., `FRONTEND_PORT=3010`, `BACKEND_PORT=3012`

Before starting servers, check `lsof -i :<port>` — kill only processes on **your** ports, never another worktree's servers.

### Testing & Validation
```bash
# Run all checks (frontend + backend)
npm run check

# Frontend specific
cd frontend && npm run lint          # Lint TypeScript/React code
cd frontend && npm run format:check  # Check formatting
cd frontend && npx tsc --noEmit     # TypeScript type checking

# Backend specific  
cargo test --workspace               # Run all Rust tests
cargo test -p <crate_name>          # Test specific crate
cargo test test_name                # Run specific test
cargo fmt --all -- --check          # Check Rust formatting
cargo clippy --all --all-targets --all-features -- -D warnings  # Linting

# Type generation (after modifying Rust types)
npm run generate-types               # Regenerate TypeScript types from Rust
npm run generate-types:check        # Verify types are up to date
```

### Database Operations
```bash
# SQLx migrations
sqlx migrate run                     # Apply migrations
sqlx database create                 # Create database

# Database is auto-copied from dev_assets_seed/ on dev server start
```

## Architecture Overview

### Tech Stack
- **Backend**: Rust with Axum web framework, Tokio async runtime, SQLx for database
- **Frontend**: React 18 + TypeScript + Vite, Tailwind CSS, shadcn/ui components  
- **Database**: SQLite with SQLx migrations
- **Type Sharing**: ts-rs generates TypeScript types from Rust structs
- **MCP Server**: Built-in Model Context Protocol server for AI agent integration

### Project Structure
```
crates/
├── server/         # Axum HTTP server, API routes, MCP server
├── db/            # Database models, migrations, SQLx queries
├── executors/     # AI coding agent integrations (Claude, Gemini, etc.)
├── services/      # Business logic, GitHub, auth, git operations
├── local-deployment/  # Local deployment logic
└── utils/         # Shared utilities

frontend/          # React application
├── src/
│   ├── components/  # React components (TaskCard, ProjectCard, etc.)
│   ├── pages/      # Route pages
│   ├── hooks/      # Custom React hooks (useEventSourceManager, etc.)
│   └── lib/        # API client, utilities

shared/types.ts    # Auto-generated TypeScript types from Rust
```

### Key Architectural Patterns

1. **Event Streaming**: Server-Sent Events (SSE) for real-time updates
   - Process logs stream to frontend via `/api/events/processes/:id/logs`
   - Task diffs stream via `/api/events/task-attempts/:id/diff`

2. **Git Worktree Management**: Each task execution gets isolated git worktree
   - Managed by `WorktreeManager` service
   - Automatic cleanup of orphaned worktrees

3. **Executor Pattern**: Pluggable AI agent executors
   - Each executor (Claude, Gemini, etc.) implements common interface
   - Actions: `coding_agent_initial`, `coding_agent_follow_up`, `script`

4. **MCP Integration**: Duck Kanban acts as MCP server
   - Tools: `list_projects`, `list_tasks`, `create_task`, `update_task`, etc.
   - AI agents can manage tasks via MCP protocol

### API Patterns

- REST endpoints under `/api/*`
- Frontend dev server proxies to backend (configured in vite.config.ts)
- Authentication via GitHub OAuth (device flow)
- All database queries in `crates/db/src/models/`

### Development Workflow

1. **Confirm worktree**: `pwd` + `git worktree list` — verify you're in the right place
2. **Check .env**: Each worktree has its own `.env` with `FRONTEND_PORT`, `BACKEND_PORT`, `DATABASE_URL`
3. **Backend changes first**: When modifying both frontend and backend, start with backend
4. **Type generation**: Run `npm run generate-types` after modifying Rust types
5. **Database migrations**: Create in `crates/db/migrations/`, apply with `sqlx migrate run`
6. **Component patterns**: Follow existing patterns in `frontend/src/components/`
7. **Before starting servers**: `lsof -i :<your-port>` — only kill processes on YOUR ports

### Testing Strategy

- **Unit tests**: Colocated with code in each crate
- **Integration tests**: In `tests/` directory of relevant crates  
- **Frontend tests**: TypeScript compilation and linting only
- **CI/CD**: GitHub Actions workflow in `.github/workflows/ci.yml`

### Flox Development Environment

This project uses [Flox](https://flox.dev) to manage the development environment. The flox manifest (`.flox/env/manifest.toml`) defines all toolchain dependencies, environment variables, and activation hooks.

**Always prefix build/run commands with `flox activate` or run inside a flox shell:**
```bash
flox activate                          # Enter the flox environment
# OR run a one-off command:
flox activate -- cargo check           # Run cargo check inside flox
flox activate -- cargo sqlx prepare --workspace  # Regenerate SQLx cache
```

**What flox provides:**
- Rust toolchain (nightly, via rustup), Node.js, pnpm, cargo-watch, sqlx-cli, cmake, libopus, pkg-config, sccache, lld
- Environment variables: `SQLX_OFFLINE=true`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`, `RUSTC_WRAPPER=sccache`
- Auto-seeds `dev_assets/db.sqlite` from `dev_assets_seed/` on first activation
- Adds `target/release` and cargo bin to PATH

**When to update `.flox/env/manifest.toml`:**
- Adding a new system dependency (e.g., a C library)
- Changing environment variables that should persist across machines
- Modifying the Rust toolchain version

**After changing Rust DB models/queries:**
```bash
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace
```
Then commit the updated `.sqlx/` directory.

### Environment Variables

Build-time (set when building):
- `GITHUB_CLIENT_ID`: GitHub OAuth app ID (default: Bloop AI's app)
- `POSTHOG_API_KEY`: Analytics key (optional)

Runtime:
- `BACKEND_PORT`: Backend server port (default: auto-assign)
- `FRONTEND_PORT`: Frontend dev port (default: 3000)
- `HOST`: Backend host (default: 127.0.0.1)
- `DATABASE_URL`: SQLite database path (default: `sqlite://dev_assets/db.sqlite`). Override for test isolation: `DATABASE_URL=sqlite:dev_assets/test-db.sqlite`
- `DISABLE_WORKTREE_ORPHAN_CLEANUP`: Debug flag for worktrees
- `VITE_SKIP_ONBOARDING`: Set to `1` to bypass all onboarding dialogs (for E2E testing)

### E2E Testing
```bash
# Run main test suite (excludes quarantine/ and demos/)
npx playwright test --reporter=list

# Run quarantined tests individually (verification before promotion)
npx playwright test e2e/quarantine/<test>.spec.ts --reporter=list

# Run demo tests
FRONTEND_PORT=<port> npx playwright test e2e/demos/ --reporter=list

# Create fresh test seed database
./scripts/create-test-seed.sh
```

Test seed: `dev_assets_seed/test-seed.sqlite` — minimal fixtures (admin user, org, pipeline stages, no entity data). Tests create their own data via API using helpers in `e2e/helpers/seed.ts`.