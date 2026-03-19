# Research: Build Performance, Developer Experience & Toolchain Optimization

**Date**: 2026-03-19
**Author**: Claude Research Sprint
**Status**: Research Complete
**Scope**: ORCHA monorepo — 21 Rust crates, 783 frontend source files, Tauri desktop targets

---

## Table of Contents

1. [Current State Assessment](#1-current-state-assessment)
2. [Rust Build Performance](#2-rust-build-performance)
3. [Frontend Build Performance](#3-frontend-build-performance)
4. [Monorepo Management](#4-monorepo-management)
5. [Developer Experience](#5-developer-experience)
6. [CI/CD Optimization](#6-cicd-optimization)
7. [Code Quality Automation](#7-code-quality-automation)
8. [Documentation](#8-documentation)
9. [Implementation Priority Matrix](#9-implementation-priority-matrix)
10. [Cost Summary](#10-cost-summary)

---

## 1. Current State Assessment

### What ORCHA Already Has

| Area | Current Tool | Status |
|------|-------------|--------|
| Compile cache | sccache | Configured in `.cargo/config.toml` and flox manifest |
| Linker | lld | Configured via rustflags for macOS (x86_64 + aarch64) |
| Cranelift | nightly-2025-05-18 + cranelift-preview component | Installed but **not activated** for debug builds |
| Frontend bundler | Vite 5.4 (root) / Vite 6.3 (workspace) | Active with React plugin, sourcemaps enabled |
| Transpiler | @vitejs/plugin-react (Babel-based) | Default Babel transform |
| Rust formatting | rustfmt | CI-enforced |
| Rust linting | clippy | CI runs but `continue-on-error: true` |
| JS formatting | prettier | Manual via `npm run format` |
| JS linting | eslint 8 | CI-enforced, max 180 warnings allowed |
| CI caching | Swatinem/rust-cache@v2 | Active for clippy, test, types-check jobs |
| Hot reload | cargo-watch | Watches `crates/` dir, rebuilds server binary |
| Type generation | ts-rs (custom fork) | Manual `npm run generate-types` |
| Dev environment | Flox | Provides full toolchain, env vars, activation hooks |
| Package manager | pnpm 10.26 | Used for frontend dependencies |

### Key Pain Points (Inferred)

1. **Target directory bloat**: Can reach 90GB+ — no automated cleanup or space-constrained profiles
2. **Full recompile on crate changes**: cargo-watch rebuilds entire server binary
3. **No cranelift activation**: Component is installed but `.cargo/config.toml` does not set `codegen-backend = "cranelift"`
4. **CI jobs install system deps redundantly**: Every Rust job runs apt-get for cmake, libopus, etc.
5. **No build caching in CI for sccache**: Disabled (`RUSTC_WRAPPER: ""`) because sccache not installed in CI runners
6. **Clippy and tests use `continue-on-error`**: Failures are informational, not blocking
7. **No pre-commit hooks**: Formatting/linting only caught in CI, not locally
8. **Babel transpiler**: Slower than SWC for large codebases
9. **No parallel CI job orchestration**: Backend jobs run independently but duplicate setup
10. **No Dockerfile / container build optimization**: No cargo-chef layer caching

---

## 2. Rust Build Performance

### 2.1 sccache Optimization (Already In Use)

**License**: Apache 2.0
**Current Status**: Configured as `RUSTC_WRAPPER` in flox and `.cargo/config.toml`

**Optimization Recommendations**:

| Action | Impact | Effort |
|--------|--------|--------|
| Enable S3/GCS backend for team-shared cache | High — cache hits across dev machines | 2-4 hours |
| Set `SCCACHE_CACHE_SIZE` to 50GB+ (default 10GB) | Medium — prevents eviction of large workspace | 5 minutes |
| Add `SCCACHE_DIR` to a fast SSD path if not default | Low-Medium | 5 minutes |
| Enable sccache in CI (install via `sccache-action`) | High — 30-50% CI speed improvement | 1 hour |
| Monitor hit rates: `sccache --show-stats` | Diagnostic | Ongoing |

**Shared Remote Cache Setup** (team benefit):
```toml
# .cargo/config.toml addition
[env]
SCCACHE_GCS_BUCKET = "orcha-sccache"
SCCACHE_GCS_RW_MODE = "READ_WRITE"
```

**Pros**: Already in place, just needs tuning; S3/GCS shared cache is the single highest-ROI improvement for multi-developer teams.
**Cons**: Remote cache adds cloud storage costs (~$5-20/month for small team); S3 latency can slow down cache misses.
**Stage fit**: Stage 0 (immediate — before April pilot work ramps up)
**Budget**: $5-20/month for cloud storage; 4 hours engineer time
**Rationale**: Team is 3 developers, shared cache means one person's build warms the cache for everyone.

---

### 2.2 Linker: mold vs lld

**mold License**: MIT
**lld License**: Apache 2.0 with LLVM exceptions

**Current Status**: lld configured for macOS (x86_64 + aarch64) and Windows (FASTLINK)

| Factor | lld (current) | mold |
|--------|---------------|------|
| macOS support | Full (via Homebrew/flox) | macOS support available since mold 2.0 (sold-linker merged) |
| Linux link speed | 2-5x faster than GNU ld | 3-10x faster than GNU ld, ~1.5-2x faster than lld |
| Memory usage | Moderate | Lower than lld |
| Stability | Very mature (LLVM project) | Mature, production-ready since v2.0 |
| Debug symbol handling | Excellent | Excellent |
| Cross-platform | macOS, Linux, Windows | macOS (since 2.0), Linux. No Windows. |
| CI (ubuntu) | Available via apt | Requires manual install or snap |

**Recommendation**: Switch to mold on Linux CI for measurable link-time improvement. Keep lld on macOS (mold's macOS support is newer and less battle-tested for Rust/Tauri linking). Evaluate mold on macOS after v2.2+.

**Configuration for CI**:
```toml
# .cargo/config.toml — linux-specific
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

**Pros**: 1.5-2x faster link times than lld on Linux; MIT licensed; lower memory.
**Cons**: macOS support is newer; Windows unsupported; one more tool to install in CI.
**Stage fit**: Stage 0-1 (quick config change for CI, keep lld on dev machines)
**Budget**: Free; 1 hour to configure + test
**Rationale**: Linking is 10-30% of incremental rebuild time; mold reduces this significantly for CI.

---

### 2.3 cargo-chef for Docker Layer Caching

**License**: MIT/Apache 2.0
**Current Status**: No Dockerfile observed in the project

**What it does**: Separates dependency compilation from source compilation in Docker builds. Dependencies are cached in a layer that only invalidates when `Cargo.toml`/`Cargo.lock` change.

**Typical Dockerfile pattern**:
```dockerfile
FROM rust:nightly AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json  # cached layer
COPY . .
RUN cargo build --release --bin server
```

**Impact for ORCHA**:
- With 21 crates and heavy deps (tokio, axum, serde, sqlx, serenity, tauri), dependency compilation is 60-80% of a clean build
- Docker layer caching means only source changes trigger recompilation — deps stay cached
- Reduces container build from ~15-25 min to ~3-8 min on subsequent builds

**Pros**: Dramatic Docker build improvement; no code changes needed; works with workspaces.
**Cons**: Only relevant when containerizing (not for local dev); adds 1 build step to CI.
**Stage fit**: Stage 1 (when deploying to cloud/Docker for pilot customers)
**Budget**: Free; 2-4 hours to write Dockerfile + test
**Rationale**: Not needed until ORCHA needs container deployments for pilots.

---

### 2.4 Incremental Compilation Best Practices

**Current Status**: Incremental compilation is ON by default for debug profile (Rust default). Release profile has it OFF (Rust default).

**Recommendations**:

| Practice | Current | Recommended |
|----------|---------|-------------|
| Debug incremental | ON (default) | Keep ON |
| `CARGO_INCREMENTAL=1` in env | Not set | Explicitly set in flox for clarity |
| Split large crates | Not assessed | Audit: `server` and `db` crates likely candidates |
| Avoid `#[derive]` on large enums | Unknown | Profile with `cargo build --timings` |
| Minimize proc-macro deps | Heavy (serde, sqlx, ts-rs, axum macros) | Accept cost; proc macros are unavoidable |
| Use `cargo build --timings` | Not in workflow | Add as periodic diagnostic |

**Crate Splitting Candidates** (reduces incremental recompile scope):
- `crates/server` — if it contains both routes and business logic, split routes into feature-grouped sub-crates
- `crates/db` — models + migrations could separate from query implementations
- `crates/executors` — each executor type could be a feature flag instead of always-compiled

**Target directory management**:
```bash
# Add to flox hook or developer scripts
# Prune target dir when it exceeds threshold
if [ $(du -sm target 2>/dev/null | cut -f1) -gt 30000 ]; then
  echo "Warning: target/ is over 30GB. Run 'cargo clean' to reclaim space."
fi
```

**Pros**: Free; reduces incremental build times; `--timings` gives data for further optimization.
**Cons**: Crate splitting requires refactoring; too many crates adds coordination overhead.
**Stage fit**: Stage 0 (timings analysis), Stage 1 (crate splitting if data supports it)
**Budget**: Free; 2 hours for timings analysis, 4-8 hours for crate splitting
**Rationale**: Data-driven — run `--timings` first, then decide what to split.

---

### 2.5 Cranelift Backend for Debug Builds

**License**: Apache 2.0 with LLVM exceptions (part of Rust nightly)
**Current Status**: Component installed (`rustc-codegen-cranelift-preview` in flox hook) but **not activated**

**What it does**: Replaces LLVM codegen with Cranelift for debug builds. Cranelift compiles 20-40% faster than LLVM but produces slower runtime code (acceptable for development).

**Activation** (missing from current config):
```toml
# .cargo/config.toml — add for debug builds only
# Note: Requires nightly, which ORCHA already uses
[unstable]
codegen-backend = true

[profile.dev]
codegen-backend = "cranelift"
```

Or via environment variable:
```bash
CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift cargo build
```

**Caveats for ORCHA**:
- Cranelift does not support all SIMD intrinsics — may fail on deps like `rustfft` (used by audio crates)
- Some proc macros may have issues on nightly cranelift
- Should test: `CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift cargo check` first

**Pros**: 20-40% faster debug compilation; already installed; zero runtime cost (debug only).
**Cons**: May fail on SIMD-heavy deps (audio/crypto); unstable feature; runtime code is slower (fine for dev); not all targets supported.
**Stage fit**: Stage 0 (test and enable if it works with ORCHA's deps)
**Budget**: Free; 1-2 hours to test compatibility
**Rationale**: Already paid the cost of installing the component — activate it.

---

### 2.6 Workspace Dependency Optimization

**Current Status**: `[workspace.dependencies]` defined for 11 core deps (tokio, axum, serde, etc.)

**Audit Recommendations**:

| Check | Purpose |
|-------|---------|
| Duplicate dep versions | `cargo tree -d` reveals deps pulled in at multiple versions |
| Feature unification | Workspace-level features prevent re-compilation with different feature sets |
| Unused deps | `cargo +nightly udeps` identifies dead dependencies |
| Heavy optional deps | Feature-gate large deps (e.g., serenity for Discord) behind workspace features |

**Quick wins**:
```toml
# Add to [workspace.dependencies] — any dep used by 3+ crates
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite"] }
```

**Feature gating heavy deps**:
```toml
# In workspace Cargo.toml
[workspace.features]
discord = []  # Only compile Discord crates when needed
audio = []    # Only compile audio/voice crates when needed
```

**Pros**: Eliminates duplicate compilations; reduces total dep count; faster CI.
**Cons**: Feature unification requires auditing all crates; over-unification can pull in unused features.
**Stage fit**: Stage 0 (quick audit with `cargo tree -d`), Stage 1 (feature gating)
**Budget**: Free; 2-4 hours for audit, 4-8 hours for feature gating
**Rationale**: Duplicate dep versions are a hidden tax on every build.

---

### 2.7 Profile Settings Optimization

**Current Status**:
```toml
[profile.release]
debug = true           # Full debug info in release
split-debuginfo = "packed"
strip = true           # Strips debug info from binary (contradicts debug = true?)

[profile.dist]
inherits = "release"
debug = false
strip = true
lto = "thin"
codegen-units = 1
opt-level = "s"        # Size optimization
```

**Recommendations**:

```toml
[profile.dev]
# codegen-backend = "cranelift"  # Enable after testing (see 2.5)
opt-level = 0          # Already default, keep for fastest compile
debug = true           # Already default

[profile.dev.package."*"]
opt-level = 2          # Optimize dependencies even in debug (faster runtime, same compile)

[profile.release]
debug = 1              # Line tables only (not full debug) — smaller, still useful for profiling
split-debuginfo = "packed"
strip = "debuginfo"    # Strip debug info but keep symbols for backtraces
lto = "thin"           # 10-20% smaller binaries, moderate compile cost
codegen-units = 16     # Default; keep for reasonable compile speed

[profile.dist]
inherits = "release"
debug = false
strip = true
lto = "fat"            # Maximum optimization for distribution
codegen-units = 1      # Maximum optimization
opt-level = "s"        # Already set
```

**Key insight**: `debug = true` + `strip = true` in release profile is contradictory. You're generating debug info then stripping it. Either:
- Set `debug = 1` (line tables only) for profiling, or
- Set `debug = false` and remove strip (nothing to strip)

**Recommended `dev.package."*"` optimization**: Compiling dependencies at opt-level 2 even in debug mode makes the resulting dev binary run faster (critical for testing workflows) while barely affecting compile time (dependencies are cached by sccache anyway).

**Pros**: Better dev-time performance; smaller release binaries; resolved contradictions.
**Cons**: `lto = "thin"` in release adds 10-30s to release builds.
**Stage fit**: Stage 0 (fix contradictions immediately; add `dev.package."*"` optimization)
**Budget**: Free; 30 minutes
**Rationale**: Immediate correctness fix plus measurable dev experience improvement.

---

### 2.8 cargo-nextest for Faster Test Execution

**License**: MIT/Apache 2.0
**Current Status**: Not in use; tests run via `cargo test --workspace`

**What it does**: Drop-in replacement for `cargo test` that runs tests in parallel processes (not threads), with better output, retries, and JUnit XML reporting.

**Performance characteristics**:
- 30-60% faster for workspaces with many test binaries (ORCHA has 21 crates)
- Each test binary runs in its own process — better isolation
- Failed test retries prevent flaky test noise
- JUnit XML output integrates with CI dashboards

**Usage**:
```bash
# Install
cargo install cargo-nextest

# Run (drop-in replacement)
cargo nextest run --workspace

# CI with JUnit output
cargo nextest run --workspace --profile ci
```

**Configuration** (`.config/nextest.toml`):
```toml
[profile.default]
retries = 0
slow-timeout = { period = "60s", terminate-after = 2 }

[profile.ci]
retries = 2
fail-fast = false
junit = { path = "results.xml" }
```

**Pros**: Faster parallel execution; better output; retry support; JUnit XML for CI; drop-in replacement.
**Cons**: One more tool to install; slight behavior differences from `cargo test` (no doc tests — run separately).
**Stage fit**: Stage 0-1 (easy to adopt, immediate CI benefit)
**Budget**: Free; 1 hour to set up
**Rationale**: With 21 crates, parallel process execution is significantly faster than `cargo test`'s thread-based approach.

---

## 3. Frontend Build Performance

### 3.1 Vite Optimization (Already In Use)

**License**: MIT
**Current Status**: Vite 5.4 (root) / 6.3 (workspace dev dep), React plugin, sourcemaps enabled

**Optimization Recommendations**:

| Action | Impact | Effort |
|--------|--------|--------|
| Enable dependency pre-bundling persistence | Medium — faster cold starts | Already default in Vite 5+ |
| Add `optimizeDeps.include` for heavy deps | Medium — pre-bundles on install | 30 min |
| Configure `build.rollupOptions.output.manualChunks` | Medium — better code splitting | 1-2 hours |
| Remove `predev` script that clears `.vite/deps` | Medium — stops destroying pre-bundle cache | 5 minutes |
| Enable CSS code splitting | Low — already default | Verify |

**Pre-bundle heavy dependencies** (add to `vite.config.ts`):
```typescript
optimizeDeps: {
  include: [
    'react', 'react-dom', 'react-router-dom',
    '@tanstack/react-query', 'zustand',
    'lucide-react', 'lodash', 'date-fns',
    '@radix-ui/react-dialog', '@radix-ui/react-popover',
    'three', '@react-three/fiber', '@react-three/drei',
  ],
},
```

**Critical fix**: The `predev` script in `frontend/package.json` runs `rm -rf node_modules/.vite/deps` before every dev start. This **destroys** Vite's dependency pre-bundle cache, forcing a full re-bundle on every `npm run dev`. Remove this unless there's a specific cache invalidation bug.

**Manual chunks for better caching**:
```typescript
build: {
  sourcemap: true,
  rollupOptions: {
    output: {
      manualChunks: {
        vendor: ['react', 'react-dom', 'react-router-dom'],
        query: ['@tanstack/react-query'],
        three: ['three', '@react-three/fiber', '@react-three/drei'],
        ui: ['@radix-ui/react-dialog', '@radix-ui/react-popover', /* ... */],
      },
    },
  },
},
```

**Pros**: No new tools; better caching; faster cold starts.
**Cons**: Manual chunks require maintenance; over-splitting hurts HTTP/2 performance.
**Stage fit**: Stage 0 (remove predev cache bust; add optimizeDeps)
**Budget**: Free; 1-2 hours
**Rationale**: The `predev` rm is actively hurting DX — fix immediately.

---

### 3.2 SWC vs Babel for Transpilation

**SWC License**: Apache 2.0
**Current Status**: Using `@vitejs/plugin-react` (Babel-based)

| Factor | Babel (current) | SWC |
|--------|-----------------|-----|
| Transform speed | 1x baseline | 20-70x faster (Rust-based) |
| React Fast Refresh | Yes | Yes (via `@vitejs/plugin-react-swc`) |
| Plugin ecosystem | Massive | Growing, covers 90% of common cases |
| Vite integration | `@vitejs/plugin-react` | `@vitejs/plugin-react-swc` |
| Custom Babel plugins | Full support | Must port to SWC or use Babel fallback |
| Bundle size impact | None (build-time only) | None |

**Migration path**:
```bash
pnpm remove @vitejs/plugin-react
pnpm add -D @vitejs/plugin-react-swc
```
```typescript
// vite.config.ts
import react from "@vitejs/plugin-react-swc";
```

**ORCHA-specific consideration**: Check if any Babel plugins are configured (e.g., in `.babelrc` or custom Vite plugin options). The current `vite.config.ts` uses `react()` with no custom Babel config — migration should be clean.

**Pros**: 20-70x faster JSX/TS transforms; drop-in replacement; no config needed.
**Cons**: If using custom Babel plugins, they won't work; edge cases in TS transform (rare).
**Stage fit**: Stage 0 (trivial migration, immediate benefit)
**Budget**: Free; 30 minutes to switch + verify
**Rationale**: With 783 source files, SWC makes HMR noticeably faster.

---

### 3.3 Bundle Size Analysis and Optimization

**Tools**:
- `rollup-plugin-visualizer` (MIT) — generates treemap of bundle
- `source-map-explorer` (Apache 2.0) — analyzes production sourcemaps
- `vite-bundle-analyzer` (MIT) — Vite-specific bundle analysis

**Suspected heavy deps in ORCHA** (from `package.json`):
| Package | Typical Size | Notes |
|---------|-------------|-------|
| `three` + `@react-three/*` | 500KB-1MB gzipped | 3D rendering — must be lazy-loaded |
| `lodash` | 70KB gzipped (full) | Should use `lodash-es` or per-function imports |
| `@uiw/react-codemirror` | 200-400KB | Code editor — lazy-load |
| `@uiw/react-md-editor` | 150-300KB | Markdown editor — lazy-load |
| `@rjsf/*` | 100-200KB | JSON Schema forms — lazy-load |

**Recommendations**:
1. Add `rollup-plugin-visualizer` to build pipeline for visibility
2. Verify `lodash` imports are tree-shakeable (use `lodash-es` or named imports from `lodash/function`)
3. Ensure Three.js and editors are behind `React.lazy()` route boundaries (already standard per frontend rules)
4. Add bundle size budget to CI (`bundlesize` or custom check)

**Pros**: Visibility into what's in the bundle; identifies low-hanging fruit.
**Cons**: Analysis is one-time effort; ongoing budget enforcement needs CI integration.
**Stage fit**: Stage 0 (add visualizer), Stage 1 (enforce budgets)
**Budget**: Free; 2-4 hours for analysis + optimization
**Rationale**: Three.js alone may be 30%+ of the bundle if not code-split.

---

### 3.4 Tree Shaking Effectiveness

**Current Status**: Vite/Rollup handles tree shaking by default for ESM imports.

**Potential issues in ORCHA**:
- `lodash` (CommonJS) — not tree-shakeable. Switch to `lodash-es` or per-function imports.
- Barrel exports (`index.ts` re-exporting everything) — can defeat tree shaking if modules have side effects
- `lucide-react` — already tree-shakeable with named imports (per frontend standards)

**Verification**: Run `npx vite build --mode development` and inspect chunk contents, or use the visualizer from 3.3.

**Pros**: Free performance; already mostly working.
**Cons**: Requires auditing imports; lodash migration touches many files.
**Stage fit**: Stage 0-1 (lodash-es migration is the main win)
**Budget**: Free; 1-2 hours for lodash migration
**Rationale**: Quick win — lodash is likely the largest tree-shaking failure.

---

### 3.5 Module Federation for Micro-Frontends

**License**: MIT (via `@module-federation/vite`)
**Current Status**: Not applicable yet

**What it does**: Allows independent frontend apps to share dependencies at runtime and load remote modules. Useful when ORCHA grows to have independently deployable frontend modules.

**When it becomes relevant**:
- Multiple teams working on different frontend domains
- Need to deploy frontend modules independently
- Plugin/extension system for customer customization

**Pros**: Independent deployability; shared dependencies; plugin architecture.
**Cons**: Significant complexity; version coordination challenges; debugging is harder; overkill for <5 developers.
**Stage fit**: Stage 3-4 (only when team grows beyond 10+ frontend developers)
**Budget**: 2-4 weeks engineering effort when needed
**Rationale**: Premature optimization for a 3-person team. Revisit when frontend grows beyond single-team ownership.

---

## 4. Monorepo Management

### 4.1 Turborepo

**License**: MIT
**Current Status**: Not in use; scripts defined in root `package.json`

**What it does**: Task runner for monorepos that understands dependency graph, caches task outputs, and runs tasks in parallel.

**ORCHA fit assessment**:
| Feature | Value for ORCHA |
|---------|----------------|
| Task caching | Medium — `cargo` already caches Rust builds; useful for frontend lint/type-check |
| Parallel execution | Medium — `concurrently` already used for dev; Turbo adds dependency awareness |
| Remote caching | High — shared cache across team/CI (like sccache for JS tasks) |
| Watch mode | Low — cargo-watch and Vite already handle this |
| Incremental adoption | High — can add without restructuring |

**Configuration example**:
```json
// turbo.json
{
  "tasks": {
    "build": { "dependsOn": ["^build"], "outputs": ["dist/**", "target/release/**"] },
    "check": { "dependsOn": ["^check"] },
    "lint": {},
    "test": {},
    "dev": { "cache": false, "persistent": true }
  }
}
```

**Pros**: Understands task graph; remote caching (Vercel or self-hosted); fast; easy to adopt incrementally.
**Cons**: Another tool in the chain; remote cache requires Vercel account or self-hosting; Rust builds are already cached by sccache.
**Stage fit**: Stage 1-2 (when CI pipeline becomes a bottleneck and team grows)
**Budget**: Free (local caching) or $50-100/month (Vercel remote cache); 4-8 hours setup
**Rationale**: Main value is remote caching for frontend tasks and CI parallelization. Not urgent for 3-person team.

---

### 4.2 Nx

**License**: MIT
**Current Status**: Not in use

**What it does**: Full monorepo tool suite with project graph, affected detection, distributed task execution, and generators.

**Comparison with Turborepo**:
| Factor | Turborepo | Nx |
|--------|-----------|-----|
| Complexity | Low | High |
| Rust support | None (JS tasks only) | Plugin-based (community Rust plugin) |
| Affected detection | Hash-based | File dependency graph |
| Generators/scaffolding | No | Yes |
| Plugin ecosystem | Small | Large |
| Learning curve | 1-2 hours | 1-2 days |
| Distributed execution | Vercel | Nx Cloud |

**Pros**: More powerful affected detection; generators for scaffolding; plugin ecosystem.
**Cons**: Heavier; steeper learning curve; overkill for current team size; Rust support is community-maintained.
**Stage fit**: Stage 3+ (only if team grows to 10+ and needs advanced monorepo tooling)
**Budget**: Free (local) or $50-200/month (Nx Cloud); 2-4 days setup
**Rationale**: Turborepo is a better fit for ORCHA's current scale. Nx becomes relevant only at larger team sizes.

---

### 4.3 Cargo Workspace Best Practices

**Current Status**: Well-structured with 21 workspace members, workspace dependencies, and profile configurations.

**Recommendations**:

1. **Add `workspace.lints`** (Rust 1.74+):
```toml
[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
all = "warn"
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"  # Common pattern in ORCHA
```

Then in each crate's `Cargo.toml`:
```toml
[lints]
workspace = true
```

2. **Consolidate more workspace dependencies**: Any dependency used in 3+ crates should be in `[workspace.dependencies]`

3. **Add workspace metadata**:
```toml
[workspace.metadata]
msrv = "nightly-2025-05-18"  # Document toolchain requirement
```

**Pros**: Consistent lints across all crates; DRY dependency management; centralized config.
**Cons**: Minor migration effort for `workspace.lints`.
**Stage fit**: Stage 0 (quick wins)
**Budget**: Free; 1-2 hours
**Rationale**: Consistency improvements that prevent drift as crate count grows.

---

### 4.4 Selective Testing Based on Changed Crates

**Tools**:
- `cargo-diff` — identifies changed crates
- GitHub Actions `paths` filter — already available
- `cargo nextest --partition` — split tests across CI runners

**CI configuration for selective testing**:
```yaml
backend-test:
  if: |
    github.event_name == 'push' ||
    contains(github.event.pull_request.changed_files, 'crates/') ||
    contains(github.event.pull_request.changed_files, 'Cargo')
```

**More granular approach** (with cargo-nextest):
```yaml
- name: Detect changed crates
  id: changes
  run: |
    CHANGED=$(git diff --name-only origin/main...HEAD | grep '^crates/' | cut -d'/' -f2 | sort -u)
    echo "crates=$CHANGED" >> $GITHUB_OUTPUT

- name: Test changed crates only
  run: |
    for crate in ${{ steps.changes.outputs.crates }}; do
      cargo nextest run -p "$crate"
    done
```

**Pros**: Faster CI for changes that only touch frontend or specific crates.
**Cons**: Risk of missing cross-crate regressions if dependency graph not considered.
**Stage fit**: Stage 1 (when CI times exceed 10 minutes)
**Budget**: Free; 2-4 hours
**Rationale**: With 21 crates, running all tests for every change is wasteful.

---

## 5. Developer Experience

### 5.1 Hot Reload Optimization

**Current Status**: `cargo-watch` watches `crates/` directory, runs `cargo run --bin server`

**Issues**:
- Full server binary rebuild on any crate change (even if change is in an unrelated crate)
- No debounce configuration visible
- Old server process may stay alive on previous port (documented gotcha)

**Recommendations**:

1. **Add watch filters** to reduce unnecessary rebuilds:
```bash
cargo watch -w crates/server -w crates/db -w crates/services -w crates/utils \
  -x 'run --bin server' \
  --delay 2  # 2-second debounce
```

2. **Consider `cargo-leptos`-style auto-reload** or `systemfd`+`listenfd` for socket handoff (zero-downtime reload):
```bash
# Install systemfd
cargo install systemfd

# Start with socket handoff
systemfd --no-pid -s http::${BACKEND_PORT} -- cargo watch -x 'run --bin server'
```
This passes the listening socket to the new process, avoiding the "old server on old port" problem.

3. **Consider `bacon`** (MIT) as a cargo-watch alternative:
- Better TUI
- Configurable jobs (check, test, build)
- Lower CPU usage during idle
- `bacon.toml` for project-specific config

**Pros**: Faster feedback loop; eliminates zombie process issue; better ergonomics with bacon.
**Cons**: `systemfd` adds complexity; `bacon` is another tool to learn.
**Stage fit**: Stage 0 (watch filters + debounce), Stage 1 (systemfd or bacon)
**Budget**: Free; 1-2 hours
**Rationale**: Developer iteration speed is the highest-leverage improvement for a small team.

---

### 5.2 IDE Performance with Large Rust Projects

**Key pain point**: rust-analyzer can consume 8-16GB RAM and saturate CPU on a 21-crate workspace.

**Recommendations**:

1. **Configure rust-analyzer in `.vscode/settings.json`** (or equivalent):
```json
{
  "rust-analyzer.cargo.buildScripts.enable": true,
  "rust-analyzer.procMacro.enable": true,
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.cargo.sysroot": "discover",
  "rust-analyzer.cachePriming.numThreads": 4,
  "rust-analyzer.cargo.targetDir": true
}
```

2. **Key setting: `cargo.targetDir`**: When set to `true`, rust-analyzer uses a separate target directory, preventing contention with `cargo build`. This is the single most impactful setting for developer experience.

3. **Limit parallel operations**: Set `rust-analyzer.numThreads` to half of CPU cores to leave room for other work.

4. **Exclude heavy crates from analysis** if not actively working on them:
```json
{
  "rust-analyzer.linkedProjects": ["./crates/server/Cargo.toml"]
}
```
(Only when focusing on a specific area — not for general use.)

**Pros**: Dramatically reduces rust-analyzer resource usage; eliminates build contention.
**Cons**: Separate target dir doubles disk usage; limiting analysis scope loses cross-crate intelligence.
**Stage fit**: Stage 0 (immediate developer quality of life)
**Budget**: Free; 30 minutes
**Rationale**: IDE responsiveness directly impacts developer productivity.

---

### 5.3 rust-analyzer Optimization Tips

**Specific to ORCHA's setup**:

| Setting | Recommendation | Why |
|---------|---------------|-----|
| `check.command` | `"clippy"` | Get clippy lints in-editor without separate step |
| `diagnostics.disabled` | `["unresolved-proc-macro"]` | Suppress false positives from proc macros |
| `cargo.buildScripts.overrideCommand` | Use if build scripts are slow | Custom command can skip expensive build scripts |
| `checkOnSave` | `true` (default) | Keep — catches errors before you push |
| `inlayHints.closureReturnTypeHints.enable` | `"with_block"` | Helpful for async closures (common in Axum handlers) |
| `completion.autoimport.enable` | `true` | Essential for 21-crate workspace |

**Memory management**:
- If rust-analyzer OOMs, add `"rust-analyzer.lru.capacity"` (e.g., 256) to limit cached analysis
- On 16GB machines, close other heavy apps when working on ORCHA

**Pros**: Better editor experience; fewer false positives; faster completions.
**Cons**: Some settings are trade-offs (e.g., limiting LRU reduces accuracy of stale files).
**Stage fit**: Stage 0 (immediate)
**Budget**: Free; 15 minutes
**Rationale**: rust-analyzer is where developers spend 80% of their time.

---

### 5.4 TypeScript Generation Pipeline Optimization

**Current Status**: Manual `npm run generate-types` after Rust struct changes; CI checks with `generate-types:check`

**Issues**:
- Manual step — easy to forget
- Full binary compilation to run type generation
- No incremental generation (regenerates all types)

**Recommendations**:

1. **Add type generation to cargo-watch** (or separate watch):
```bash
# In a parallel terminal or as part of dev script
cargo watch -w crates/db/src/models -w crates/server/src -s 'cargo run --bin generate_types' --delay 5
```

2. **Consider `ts-rs` build script approach**: Instead of a separate binary, use `build.rs` to generate types at compile time. This eliminates the separate step entirely.

3. **Gate type generation behind a feature flag** to avoid compiling it during normal development:
```toml
[features]
generate-types = ["ts-rs/export"]
```

4. **Pre-commit hook** (see section 7.1): Auto-run type generation check before commit.

**Pros**: Eliminates forgotten type generation; faster feedback.
**Cons**: Watch-based generation adds compile overhead; build.rs approach may slow all builds.
**Stage fit**: Stage 0 (add to dev workflow), Stage 1 (build.rs integration)
**Budget**: Free; 1-2 hours
**Rationale**: Type drift between Rust and TypeScript is a recurring bug source.

---

### 5.5 Development Database Management

**Current Status**: SQLite with seed database in `dev_assets_seed/`, auto-copy on flox activation.

**Recommendations**:

1. **Test database isolation**: Already documented — `DATABASE_URL=sqlite:dev_assets/test-db.sqlite`

2. **Database snapshot/restore script**:
```bash
#!/bin/bash
# scripts/db-snapshot.sh
cp dev_assets/db.sqlite "dev_assets/db-snapshot-$(date +%Y%m%d-%H%M%S).sqlite"

# scripts/db-restore.sh
cp dev_assets_seed/db.sqlite dev_assets/db.sqlite
```

3. **Migration validation in CI**: Add step to verify migrations apply cleanly to seed database.

4. **Consider `sqlx migrate run --dry-run`** for pre-flight checks (SQLx 0.8+).

**Pros**: Safer database operations; faster recovery from bad data states.
**Cons**: Snapshot files take disk space; SQLite is simple enough that most of this is already manageable.
**Stage fit**: Stage 0 (scripts), Stage 1 (CI migration validation)
**Budget**: Free; 1 hour
**Rationale**: Database corruption during development wastes 15-30 minutes per incident.

---

## 6. CI/CD Optimization

### 6.1 GitHub Actions Caching Strategies for Rust

**Current Status**: `Swatinem/rust-cache@v2` on 3 of 5 jobs; sccache disabled in CI

**Current CI job structure (6 jobs, all independent)**:
1. `backend-fmt` — no cache (fast, just rustfmt)
2. `backend-clippy` — rust-cache
3. `backend-test` — rust-cache
4. `frontend-check` — pnpm cache
5. `types-check` — rust-cache + pnpm cache
6. `security` — no cache

**Problems**:
- System deps installed 3 times (clippy, test, types-check)
- sccache disabled — losing 30-50% build speed
- No shared compilation between clippy and test (different jobs, different runners)

**Recommendations**:

**A. Enable sccache in CI** (highest impact):
```yaml
env:
  SCCACHE_GHA_ENABLED: "true"
  RUSTC_WRAPPER: "sccache"

steps:
  - uses: mozilla-actions/sccache-action@v0.0.6
```
This uses GitHub Actions cache as sccache backend — free, no external storage needed.

**B. Combine backend jobs** to share compilation:
```yaml
backend:
  name: Rust checks & tests
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: mozilla-actions/sccache-action@v0.0.6
    - uses: dtolnay/rust-toolchain@master
      with:
        toolchain: nightly-2025-05-18
        components: rustfmt, clippy
    - name: System dependencies
      run: sudo apt-get update && sudo apt-get install -y cmake pkg-config libopus-dev libglib2.0-dev libssl-dev libsodium-dev
    - run: cargo fmt --all -- --check
    - run: cargo clippy --all --all-targets -- -D warnings
    - run: cargo nextest run --workspace
```
This reduces total CI time from ~(3 * compile_time) to ~(1 * compile_time + incremental_cost).

**C. Cache system dependencies**:
```yaml
- uses: awalsh128/cache-apt-pkgs-action@latest
  with:
    packages: cmake pkg-config libopus-dev libglib2.0-dev libssl-dev libsodium-dev
    version: 1.0
```

**Estimated CI time improvement**: From ~15-20 min (3 parallel Rust jobs compiling independently) to ~8-12 min (1 Rust job, sccache-warmed, running fmt+clippy+test sequentially).

**Pros**: 40-60% CI time reduction; fewer GitHub Actions minutes consumed; simpler pipeline.
**Cons**: Combined job means one failure blocks all; less granular status checks on PRs.
**Stage fit**: Stage 0 (enable sccache + cache apt), Stage 1 (combine jobs)
**Budget**: Free; 2-4 hours
**Rationale**: CI speed directly impacts PR merge velocity. With 3 developers, every minute of CI wait time is multiplied.

---

### 6.2 Parallel Job Execution Patterns

**Current State**: All 6 jobs run in parallel (no dependencies between them). This is good for latency but wasteful for compilation.

**Optimized pattern**:
```
                    ┌─── frontend-check (2 min)
                    │
checkout ───────────┼─── security (1 min)
                    │
                    └─── backend (10 min)
                           ├── fmt
                           ├── clippy
                           ├── test
                           └── types-check (needs pnpm too)
```

**Alternative: Matrix strategy** for backend variants:
```yaml
backend:
  strategy:
    matrix:
      include:
        - name: "fmt"
          command: "cargo fmt --all -- --check"
          needs_deps: false
        - name: "clippy"
          command: "cargo clippy --all --all-targets -- -D warnings"
          needs_deps: true
        - name: "test"
          command: "cargo nextest run --workspace"
          needs_deps: true
```

**Pros**: Better resource utilization; faster total pipeline.
**Cons**: Matrix strategy still runs multiple jobs (but shares config).
**Stage fit**: Stage 1
**Budget**: Free; 2 hours
**Rationale**: Diminishing returns at current team size; more impactful after Stage 0 sccache enablement.

---

### 6.3 Build Artifact Sharing Between Jobs

**Mechanism**: `actions/upload-artifact` / `actions/download-artifact` or `actions/cache`

**Relevant for ORCHA when**:
- E2E tests need the compiled backend binary
- Deployment job needs the release binary
- Type generation check needs compiled Rust binary + frontend deps

**Pattern**:
```yaml
build:
  steps:
    - run: cargo build --release --bin server
    - uses: actions/upload-artifact@v4
      with:
        name: server-binary
        path: target/release/server

e2e:
  needs: [build, frontend-check]
  steps:
    - uses: actions/download-artifact@v4
      with: { name: server-binary }
```

**Pros**: Avoids recompilation in downstream jobs; enables E2E testing in CI.
**Cons**: Upload/download adds 1-2 minutes; artifact storage counts against GitHub Actions limits.
**Stage fit**: Stage 1 (when adding E2E to CI)
**Budget**: Free (within GitHub Actions limits); 2 hours
**Rationale**: Essential for CI E2E testing pipeline.

---

### 6.4 Self-Hosted Runner Cost/Benefit

**Current Status**: Using GitHub-hosted `ubuntu-latest` runners

| Factor | GitHub-Hosted | Self-Hosted (e.g., EC2 c6g.2xlarge) |
|--------|--------------|--------------------------------------|
| Cost | $0.008/min (Linux) | ~$0.136/hr ($100/mo always-on) or ~$0.04/hr spot |
| CPU | 4 vCPUs (shared) | 8 vCPUs (dedicated) |
| RAM | 16 GB | 16 GB |
| Disk | 14 GB | 100GB+ (persistent cache) |
| Cache persistence | Action cache (10GB limit) | Local disk (unlimited) |
| Build time (estimated) | 15-20 min | 6-10 min (with warm cache) |
| Monthly cost @ 20 PRs | ~$30-50 | ~$40-100 (spot) |
| Maintenance | Zero | Patching, security, monitoring |

**When self-hosted makes sense**:
- CI time exceeds 15 min AND team is blocked on CI regularly
- Need larger disk for Rust compilation (target dir)
- Need persistent sccache between runs (not possible with ephemeral runners)

**Alternative: GitHub-hosted larger runners** ($0.016/min for 8-core):
- 2x CPU for 2x cost
- No maintenance
- Better for burst workloads

**Pros**: Faster builds; persistent caches; no GitHub storage limits.
**Cons**: Maintenance burden; security responsibility; cost even when idle.
**Stage fit**: Stage 2 (when team grows and CI utilization justifies dedicated infra)
**Budget**: $40-150/month; 1-2 days initial setup
**Rationale**: Not justified for 3 developers. Revisit when CI minutes exceed $100/month.

---

### 6.5 Build Time Metrics and Monitoring

**Tools**:
- `cargo build --timings` — HTML report of crate compilation times
- `cargo-bloat` (MIT) — identifies large functions/dependencies in binary
- GitHub Actions job summaries — built-in timing
- DataDog/Grafana CI dashboards — for trend analysis

**Quick setup**:
```yaml
# Add to CI
- name: Build with timings
  run: cargo build --timings --release --bin server
- uses: actions/upload-artifact@v4
  with:
    name: build-timings
    path: target/cargo-timings/cargo-timing.html
```

**Pros**: Data-driven optimization; identifies regression in compile times.
**Cons**: Timings add ~10% overhead; dashboard setup requires external tool.
**Stage fit**: Stage 0 (--timings in CI), Stage 2 (dashboard)
**Budget**: Free for timings; $20-50/month for monitoring dashboard
**Rationale**: "What gets measured gets improved." Without data, build optimization is guesswork.

---

## 7. Code Quality Automation

### 7.1 Pre-commit Hooks: Husky + lint-staged

**Husky License**: MIT
**lint-staged License**: MIT
**Current Status**: No pre-commit hooks. Formatting/linting only caught in CI.

**Setup**:
```bash
pnpm add -D husky lint-staged
npx husky init
```

**`.husky/pre-commit`**:
```bash
#!/bin/sh
npx lint-staged
```

**`package.json` addition**:
```json
{
  "lint-staged": {
    "frontend/src/**/*.{ts,tsx}": [
      "eslint --fix",
      "prettier --write"
    ],
    "frontend/src/**/*.{json,css,md}": [
      "prettier --write"
    ],
    "crates/**/*.rs": [
      "rustfmt"
    ]
  }
}
```

**Additional hooks**:
- **pre-push**: Run `npm run generate-types:check` to catch type drift before it hits CI
- **commit-msg**: Validate conventional commit format (see 7.4)

**Pros**: Catches formatting/lint issues before they hit CI (saves 5-10 min per failed CI run); enforces consistency.
**Cons**: Slows down commit by 2-5 seconds; developers can bypass with `--no-verify` (but shouldn't per project rules).
**Stage fit**: Stage 0 (immediate quality improvement)
**Budget**: Free; 1 hour setup
**Rationale**: Every formatting fix caught locally saves a CI round-trip.

---

### 7.2 Automated Formatting (Already In Place)

**Current Status**: rustfmt + prettier configured and CI-enforced.

**Recommendations**:
- Add `rustfmt.toml` with explicit settings for consistency:
```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
```
- Ensure prettier config is shared (`.prettierrc` at root)
- Add `format` to pre-commit hook (see 7.1)

**Stage fit**: Stage 0
**Budget**: Free; 15 minutes

---

### 7.3 Clippy Lint Configuration Optimization

**Current Status**: `cargo clippy --all --all-targets -- -D warnings` in CI with `continue-on-error: true`

**Issues**:
- `continue-on-error` means clippy warnings don't block PRs
- No project-specific lint configuration
- 27 known warnings in nora/alpha-protocol crates (structural dead code)

**Recommendations**:

1. **Create `clippy.toml`** at workspace root:
```toml
# Increase complexity threshold (default 25 is too strict for Axum handlers)
cognitive-complexity-threshold = 40
too-many-arguments-threshold = 10
type-complexity-threshold = 500
```

2. **Use workspace lints** (see 4.3) instead of CLI flags

3. **Remove `continue-on-error`** and instead suppress known warnings with targeted allows:
```rust
// In nora/alpha-protocol crates with structural dead code
#![allow(dead_code)]  // Structural: protocol fields used by external consumers
```

4. **Phased enforcement**:
   - Stage 0: Allow known warnings explicitly, remove `continue-on-error`
   - Stage 1: Reduce allowed warning count over time
   - Stage 2: Zero warnings policy

**Pros**: Clippy catches real bugs; enforced linting prevents regression.
**Cons**: Initial effort to suppress existing warnings; overly strict lints slow development.
**Stage fit**: Stage 0 (allow known warnings + enforce), Stage 1 (reduce warning budget)
**Budget**: Free; 2-4 hours to audit and allow existing warnings
**Rationale**: `continue-on-error` means clippy is informational — it should be enforced with targeted exceptions.

---

### 7.4 Commit Message Validation (commitlint)

**License**: MIT
**Current Status**: No commit message validation. Convention is documented but not enforced.

**Setup**:
```bash
pnpm add -D @commitlint/cli @commitlint/config-conventional
```

**`commitlint.config.js`**:
```javascript
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [2, 'always', [
      'feat', 'fix', 'refactor', 'docs', 'chore', 'test', 'perf', 'ci', 'style'
    ]],
    'subject-max-length': [2, 'always', 100],
  },
};
```

**`.husky/commit-msg`**:
```bash
#!/bin/sh
npx --no -- commitlint --edit ${1}
```

**Pros**: Consistent commit history; enables automated changelogs; prevents meaningless commit messages.
**Cons**: Friction for quick commits; team must learn conventional commit format; Claude Code commits may need adjustment.
**Stage fit**: Stage 1 (after pre-commit hooks are established)
**Budget**: Free; 1 hour
**Rationale**: Useful for changelog generation and release notes in Stage 1-2. Not urgent for Stage 0.

---

## 8. Documentation

### 8.1 cargo doc Integration

**License**: Built into Rust toolchain (free)
**Current Status**: No `cargo doc` in CI or development workflow

**Recommendations**:
1. Add `#![doc = include_str!("../README.md")]` to crate lib.rs files for top-level docs
2. Run `cargo doc --workspace --no-deps` in CI to catch doc warnings
3. Deploy docs to GitHub Pages for internal reference

**Configuration in CI**:
```yaml
docs:
  if: github.ref == 'refs/heads/main'
  steps:
    - run: cargo doc --workspace --no-deps --document-private-items
    - uses: peaceiris/actions-gh-pages@v4
      with:
        github_token: ${{ secrets.GITHUB_TOKEN }}
        publish_dir: ./target/doc
```

**Pros**: Auto-generated from code; catches broken doc links; searchable API reference.
**Cons**: Only useful if team writes doc comments; private docs may expose internal details.
**Stage fit**: Stage 1 (when onboarding new developers)
**Budget**: Free; 1-2 hours
**Rationale**: Investment pays off when team grows beyond 3.

---

### 8.2 API Documentation: utoipa/Swagger for Axum

**utoipa License**: MIT/Apache 2.0
**Current Status**: No OpenAPI/Swagger documentation

**What it does**: Generates OpenAPI 3.0 spec from Rust types and route definitions. Serves Swagger UI at a `/docs` endpoint.

**Integration with Axum**:
```rust
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

#[derive(OpenApi)]
#[openapi(
    paths(list_tasks, create_task, update_task),
    components(schemas(Task, CreateTask, UpdateTask))
)]
struct ApiDoc;

// Add to router
let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
    .routes(routes!(list_tasks, create_task))
    .split_for_parts();

// Serve Swagger UI
let router = router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api));
```

**ORCHA-specific benefits**:
- Auto-documents all API endpoints for frontend developers
- Enables API testing without frontend (useful for debugging)
- Essential for MCP server documentation (external consumers)
- Can generate TypeScript client from OpenAPI spec (alternative to ts-rs)

**Pros**: Auto-generated from code; interactive testing UI; industry standard; essential for external API consumers.
**Cons**: Requires annotating all routes and types; annotation maintenance overhead; adds compile time.
**Stage fit**: Stage 1 (for pilot customers and MCP documentation)
**Budget**: Free; 2-4 days to annotate existing routes
**Rationale**: External API consumers (MCP clients, pilot customers) need documentation.

---

### 8.3 Storybook for Component Library

**License**: MIT
**Current Status**: No component documentation or visual testing

**What it does**: Isolated component development environment with visual regression testing.

**ORCHA fit assessment**:
- 783 frontend source files with shadcn/ui + custom components
- Team uses click-to-react-component for navigation
- No component documentation for new team members

**Setup**:
```bash
cd frontend
npx storybook@latest init --builder vite
```

**Pros**: Visual component catalog; isolated development; visual regression testing; onboarding aid.
**Cons**: Significant setup for existing components; maintenance overhead per component; build time for Storybook.
**Stage fit**: Stage 2-3 (when frontend team grows or component library is shared)
**Budget**: Free; 2-4 days initial setup, ongoing 15 min per new component
**Rationale**: Overkill for 3-person team. Valuable when multiple developers work on UI or when ORCHA offers a plugin/theme system.

---

### 8.4 Architecture Decision Records (ADRs)

**Current Status**: Planning docs in `planning/` directory serve a similar purpose

**Assessment**: ORCHA already has a robust planning doc convention (`YYYY-MM-DD--<type>--<topic>.md`). ADRs would overlap with existing `plan` and `reference` type documents.

**Recommendation**: Do not add a separate ADR framework. Instead:
- Use `reference` type planning docs for architectural decisions
- Add a `Decision` section template to planning doc format
- Include "Alternatives Considered" in significant technical decisions

**Pros**: Leverage existing convention; no new tools.
**Cons**: Less standardized than formal ADR tools (adr-tools, etc.).
**Stage fit**: Not needed — existing planning system is sufficient
**Budget**: Free; 0 effort
**Rationale**: Adding another documentation system creates maintenance burden with no clear benefit over current approach.

---

## 9. Implementation Priority Matrix

### Stage 0 (Immediate — April 2026)

| # | Action | Impact | Effort | Dependencies |
|---|--------|--------|--------|-------------|
| 1 | Enable cranelift for debug builds | High | 1-2 hr | Test with ORCHA deps |
| 2 | Fix `profile.release` contradiction (`debug=true` + `strip=true`) | Medium | 15 min | None |
| 3 | Add `[profile.dev.package."*"] opt-level = 2` | High | 5 min | None |
| 4 | Remove `predev` rm of `.vite/deps` in frontend | Medium | 5 min | Verify no cache bug |
| 5 | Switch to `@vitejs/plugin-react-swc` | High | 30 min | None |
| 6 | Enable sccache in CI (`sccache-action`) | High | 1 hr | None |
| 7 | Cache apt packages in CI | Medium | 30 min | None |
| 8 | Set up husky + lint-staged pre-commit hooks | High | 1 hr | None |
| 9 | Add rust-analyzer settings (targetDir, etc.) | Medium | 30 min | None |
| 10 | Add `cargo build --timings` to CI | Low | 30 min | None |
| 11 | Run `cargo tree -d` audit and unify workspace deps | Medium | 2-4 hr | None |
| 12 | Add cargo-watch filters and debounce | Medium | 30 min | None |
| 13 | Add `optimizeDeps.include` to Vite config | Medium | 30 min | None |
| 14 | Add `workspace.lints` to Cargo.toml | Medium | 1-2 hr | None |

**Total Stage 0 effort**: ~10-14 hours (1.5-2 developer-days)
**Expected impact**: 30-50% faster local dev builds, 40-60% faster CI

### Stage 1 (First Pilots — Q3 2026)

| # | Action | Impact | Effort | Dependencies |
|---|--------|--------|--------|-------------|
| 1 | Combined backend CI job | High | 2-4 hr | Stage 0 sccache |
| 2 | cargo-nextest for test execution | Medium | 1 hr | None |
| 3 | Selective testing based on changed crates | Medium | 2-4 hr | None |
| 4 | commitlint for commit message validation | Low | 1 hr | Stage 0 husky |
| 5 | Bundle size analysis + manual chunks | Medium | 2-4 hr | None |
| 6 | lodash -> lodash-es migration | Low | 1-2 hr | None |
| 7 | cargo doc in CI | Low | 1-2 hr | None |
| 8 | utoipa/Swagger API docs | High | 2-4 days | None |
| 9 | cargo-chef Docker layer caching | High | 2-4 hr | Dockerfile needed |
| 10 | sccache shared remote cache (S3/GCS) | High | 2-4 hr | Cloud account |
| 11 | Build artifact sharing in CI (for E2E) | Medium | 2 hr | E2E in CI |
| 12 | mold linker in Linux CI | Low-Med | 1 hr | None |
| 13 | Remove clippy `continue-on-error` | Medium | 2-4 hr | Suppress known warnings |

**Total Stage 1 effort**: ~5-7 developer-days

### Stage 2-3 (SaaS Launch — Q4 2026+)

| # | Action | Impact | Effort | Dependencies |
|---|--------|--------|--------|-------------|
| 1 | Turborepo for task orchestration | Medium | 4-8 hr | Team growth |
| 2 | Self-hosted CI runners (evaluate) | Medium | 1-2 days | CI cost data |
| 3 | Larger GitHub runners (8-core) | Medium | 30 min | Budget approval |
| 4 | CI metrics dashboard | Low | 1 day | Monitoring infra |
| 5 | Storybook for component library | Low | 2-4 days | Frontend team growth |
| 6 | Crate splitting (server, db) | Medium | 1-2 days | Timings data |

### Stage 4-5 (Growth — 2028+)

| # | Action | Impact | Effort |
|---|--------|--------|--------|
| 1 | Nx monorepo tools | Medium | 2-4 days |
| 2 | Module federation | Medium | 2-4 weeks |
| 3 | Distributed CI execution | High | 1-2 weeks |

---

## 10. Cost Summary

### Recurring Costs

| Item | Monthly Cost | Stage |
|------|-------------|-------|
| sccache cloud storage (S3/GCS) | $5-20 | Stage 1 |
| Turborepo remote cache (Vercel) | $0-100 | Stage 2 |
| Larger GitHub runners (8-core) | $0-50 | Stage 2 |
| Self-hosted runner (spot instance) | $40-100 | Stage 2 |
| CI metrics dashboard (DataDog/Grafana) | $20-50 | Stage 2 |

**Stage 0 total recurring cost: $0/month**
**Stage 1 total recurring cost: $5-20/month**

### One-Time Engineering Investment

| Stage | Developer-Days | Key Deliverables |
|-------|---------------|------------------|
| 0 | 1.5-2 | Cranelift, SWC, sccache CI, pre-commit hooks, Vite optimization |
| 1 | 5-7 | Combined CI, nextest, utoipa, Docker builds, shared cache |
| 2 | 5-10 | Turborepo, self-hosted runners, Storybook |

### License Summary

All recommended tools use permissive licenses compatible with ORCHA's proprietary + future source-available model:

| Tool | License | Commercial Use |
|------|---------|---------------|
| sccache | Apache 2.0 | Yes |
| mold | MIT | Yes |
| cargo-chef | MIT/Apache 2.0 | Yes |
| cargo-nextest | MIT/Apache 2.0 | Yes |
| cranelift | Apache 2.0 + LLVM | Yes |
| SWC (@vitejs/plugin-react-swc) | Apache 2.0 | Yes |
| husky | MIT | Yes |
| lint-staged | MIT | Yes |
| commitlint | MIT | Yes |
| Turborepo | MIT | Yes |
| utoipa | MIT/Apache 2.0 | Yes |
| Storybook | MIT | Yes |
| bacon | MIT | Yes |
| rollup-plugin-visualizer | MIT | Yes |

---

## Appendix: Quick Reference Commands

```bash
# Diagnose build performance
cargo build --timings --release --bin server
sccache --show-stats
cargo tree -d  # duplicate deps

# Enable cranelift (test)
CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift cargo check

# Switch to SWC
cd frontend && pnpm remove @vitejs/plugin-react && pnpm add -D @vitejs/plugin-react-swc

# Install pre-commit hooks
pnpm add -D husky lint-staged && npx husky init

# Install cargo-nextest
cargo install cargo-nextest --locked

# Build with mold (Linux)
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build

# Bundle analysis
cd frontend && pnpm add -D rollup-plugin-visualizer && pnpm run build
```
