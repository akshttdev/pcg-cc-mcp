# Tech Debt Sprint — Week of 2026-03-15

## Context

Tech debt review identified 24 findings across the platform. This 1-week sprint targets the highest-ROI items aligned with two active pain points: **server crashes/hangs** and **merge conflicts/DX friction**. Deployment target is internal/team use. PostgreSQL migration deferred.

Source: `planning/notes/2026-03-15--analysis--tech-debt-review.md`

**Open PRs at sprint start:**
- PR #31 (`fix/dev-script-port-override`): dev script respects FRONTEND_PORT/BACKEND_PORT env vars — merge before sprint branch

**Base branch:** `tech-debt/sprint-2026-03-15` from `main` @ `bc7f4c347`, merged with `e2e/dogfood-demo-replay-2026-03-14` (fast-forward to `037487ade`)

---

## Sprint Scope (5 working days)

### Day 1: Critical Crash Prevention

**1. Fix worktree manager deadlock** (ref 1.2 — S effort, CRITICAL)
- File: `crates/services/src/services/worktree_manager.rs`
- Current architecture: hybrid mutex — outer `std::sync::Mutex<HashMap>` (held briefly for map access) + inner `tokio::sync::Mutex<()>` (per-worktree async lock)
- The outer `std::sync::Mutex` at lines 88 and 386 uses `.lock().unwrap()` — if any panic poisons the mutex, all future worktree ops deadlock
- Fix: replace `.lock().unwrap()` with `.lock().unwrap_or_else(|e| e.into_inner())` for poisoned-mutex recovery
- Also add `tokio::time::timeout(Duration::from_secs(30), lock.lock())` on the inner async lock (line 96, 393)

**2. Shared HTTP client with timeouts** (ref 3.4/7.2 — S effort)
- Create `once_cell::sync::Lazy<reqwest::Client>` with 30s timeout + connection pooling
- Add `http_client: reqwest::Client` field to `LocalDeployment` (app state struct)
- **Scope:** 97+ individual `reqwest::Client` constructions found across codebase
  - Route handlers in `crates/server/src/routes/`: ~20 constructions (intelligence, meet, email, invite, topsi, pulse, twilio, etc.)
  - NORA crate: ~15 constructions (tools, integrations, executor, voice TTS/STT, crawler)
  - Services: ~10 constructions (analytics, workflow_llm, airtable_service, apn_bridge, editron, agent_tools)
  - Other crates: ~10 (topsi agent, apn-client, cli, discord-bots, task_scheduler, mcp)
- **Day 1 scope:** Create shared client + update `crates/server/src/routes/` handlers only
- **Remaining crates:** Incremental migration in follow-up (services, nora, etc.)

**3. Frontend error boundaries** (ref 1.4 — S effort)
- Existing: `ErrorBoundaryFallback` component at `frontend/src/components/error-boundary-fallback.tsx` + Sentry `ErrorBoundary` wrapping entire app in `main.tsx`
- Existing fallback uses `min-h-screen` — unsuitable for inline/per-section use. Need a new component.
- `react-error-boundary` package NOT installed — use Sentry's `ErrorBoundary` component (already available via `@sentry/react` v9.34.0)
- Create a reusable `PageErrorBoundary` component that wraps `Sentry.ErrorBoundary` with an inline fallback (no `min-h-screen`)
- Add `<PageErrorBoundary>` wrappers around each `<Suspense>` in route definitions (App.tsx)
- Add error boundary around each tab in org-profile page (8 tabs: Overview, Pipelines, Contacts, Projects, Social, Intelligence, Members, Integrations)

### Day 2-3: Top `.unwrap()` Elimination (ref 1.1 — M effort for top-45)

Prioritize by traffic and crash risk (verified counts post-merge):

**Original 5 target files (37 unwraps):**
1. `crates/server/src/mcp/task_server.rs` (15 unwraps) — MCP tool execution; mostly `serde_json::to_string_pretty().unwrap()` and `to_value().unwrap()`, plus 2x `self.user_id.unwrap()`
2. `crates/services/src/services/events.rs` (9 unwraps) — SSE event streams; `serde_json::to_value().unwrap()` and `from_value().unwrap()` in patch building
3. `crates/server/src/routes/airtable.rs` (10 unwraps) — 7x `config.airtable.token.as_ref().unwrap()` (after `is_configured()` check) + 2x `Uuid::parse_str().unwrap()`
4. `crates/server/src/routes/task_attempts.rs` (2 unwraps) — `current.as_ref().unwrap()` after conditional check
5. `crates/services/src/services/apn_bridge/connector.rs` (1 unwrap) — `serde_json::to_string(&APNMessage::Ping).unwrap()`

**Additional files from dogfood demo merge (8 unwraps):**
6. `crates/server/src/routes/tasks.rs` (2 unwraps) — `serde_json::to_string(mcps).unwrap()` (line 580), `serde_json::to_string(tags).unwrap()` (line 592)
7. `crates/services/src/services/workflow_execution.rs` (3 unwraps) — 5x `Regex::new().unwrap()` (lines 245, 259, 275, 350, 376 — compile-time-known, acceptable per standards) + `data.as_object().unwrap()` (lines 1007, 1009 — NOT safe, needs guard)
8. `crates/db/src/models/activity.rs` (3 unwraps) — `serde_json::to_string(v).unwrap()` (lines 88, 92, 96 — JSON serialization in activity logging)

**Total: 45 unwraps across 8 files** (5 Regex unwraps in workflow_execution.rs are acceptable per rust-standards.md, leaving 40 to fix)

Approach:
- **Error type is `ApiError`** (defined in `crates/server/src/error.rs`), NOT `AppError`
- `ApiError` already has `InternalError(String)` and `BadRequest(String)` variants — use these
- Create helper `fn json_str<T: Serialize>(v: &T) -> Result<String, ApiError>` in server crate (not utils — it depends on ApiError)
- Create helper `fn json_value<T: Serialize>(v: &T) -> Result<serde_json::Value, ApiError>` for the `to_value` pattern
- Replace `serde_json::to_string(&v).unwrap()` → `json_str(&v)?`
- Replace `serde_json::to_value(&v).unwrap()` → `json_value(&v)?`
- Replace `Uuid::parse_str(&s).unwrap()` → `Uuid::parse_str(&s).map_err(|e| ApiError::BadRequest(e.to_string()))?`
- Replace `self.user_id.unwrap()` → `self.user_id.ok_or(ApiError::Unauthorized("no user_id".into()))?`
- Replace `config.airtable.token.as_ref().unwrap()` → `config.airtable.token.as_ref().ok_or(ApiError::BadRequest("Airtable not configured".into()))?`
- Replace `data.as_object().unwrap()` → guard with `if let Some(obj) = data.as_object()` or `.ok_or()`
- Target: eliminate all 40 fixable unwraps across 8 files (5 Regex unwraps are acceptable)

### Day 3-4: CI Pipeline (ref 5.1 — M effort, VERY HIGH ROI)

**Existing CI:** Only `.github/workflows/release.yml` (Tauri desktop release, triggered on `v*` tags). No test/lint/clippy CI.

Create `.github/workflows/ci.yml`:
```yaml
jobs:
  backend:
    - cargo fmt --all -- --check
    - cargo clippy --all --all-targets --all-features -- -D warnings
    - cargo test --workspace
  frontend:
    - npx tsc --noEmit
    - npm run lint
    - npm run generate-types:check
  security:
    - cargo audit
    - npm audit
```
- Trigger on PR to `main` and `dev/*` branches
- **Flox note:** Existing release.yml uses `dtolnay/rust-toolchain@stable`, but project uses **Rust nightly-2025-05-18** (from `.flox/env/manifest.toml`). CI must use nightly to match dev environment.
- `cargo-audit` is NOT in the flox manifest — install via `cargo install cargo-audit` in CI or add to manifest
- Key env vars needed: `SQLX_OFFLINE=true`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`
- Cache: cargo registry + target dir + sccache (project uses `RUSTC_WRAPPER=sccache`)
- Note: `cargo test --workspace` requires `SQLX_OFFLINE=true` (no live DB in CI)

### Day 4-5: Monolithic File Splitting (ref 4.1 — highest DX impact)

**Split `frontend/src/lib/api.ts` (6,669 lines, 212 importers across codebase):**
- Create `frontend/src/lib/api/` directory
- File contains 60+ API domain modules (`const xxxApi = { ... }`) — group into ~12 files:
  - `projects.ts` — projectsApi, projectControllersApi, commitsApi
  - `tasks.ts` — tasksApi, attemptsApi, taskArtifacts, taskApproval, comments
  - `crm.ts` — crmApi, crmPipelines, crmDeals, crmActivities
  - `workflows.ts` — workflowsApi, workflowTemplates, triggersApi, schemasApi, stagingApi
  - `organizations.ts` — organizations, users, personsApi, companies
  - `agents.ts` — agents, agentFlows, agentWatchers, agentExecutionConfig, executionProcesses, executionSummary
  - `communication.ts` — emailApi, emailMessages, socialApi, communicationsApi, discordApi, meetingsApi
  - `intelligence.ts` — knowledgeApi, intelligenceApi, reportsApi, pulseApi, wideResearch, dataSourcesApi
  - `artifacts.ts` — artifactReviews, artifactContent
  - `integrations.ts` — airtableApi, vibeApi, aptosApi, ralphApi, pcgRouterApi, quickbooks, editron
  - `finance.ts` — invoices, proposals, deliverablesApi
  - `auth.ts` — authApi, config, profiles, githubAuth, github, mcpServers, systemSettingsApi, mediaApi, images, templates, intake, entityConversion, modelPricing, tokenUsageApi, activity, commandCenter, reviewApi, agentWallet, fileSystem, approvals
- Also extract shared `makeRequest()` helper and types into `client.ts`
- Barrel re-export from `api/index.ts` for backward compatibility
- Update imports across codebase

**Split `frontend/src/pages/organization-profile.tsx` (7,069 lines, no external importers — well isolated):**
- Create `frontend/src/pages/organization-profile/` directory (not `org-tabs/` — co-locate everything)
- More components than originally estimated — 17 distinct functions + many sub-components:
  - **Tabs:** `OverviewTab` (line 651), `PipelinesTab` (line 886), `ContactsTab` (line 928), `ProjectsTab` (line 1604), `IntelligenceTab` (line 3862), `SocialTab` (line 5422), `IntegrationsTab` (line 5522), `MembersTab` (line 6198)
  - **Sub-views within tabs:** `DataSourcesView` (line 2035), `ArtifactsView` (line 2391), `EditableWorkflowsView` (line 2629), `LegacyPipelinesView` (line 2895)
  - **Shared components:** `BrandIdentityCard` (line 204), `ContactDetailModal`, `ContactCard`, social sub-views (SocialOverviewView, SocialAccountsView, SocialContentView, SocialInboxView, SocialAnalyticsView)
  - **Unused:** `KnowledgeTab` (line 3400) — extract but don't wire up
- Recommended structure:
  ```
  organization-profile/
    index.tsx              — main page shell + tab routing (~500 lines)
    tabs/                  — one file per tab
    components/            — shared sub-components (ContactDetailModal, BrandIdentityCard, etc.)
    constants.ts           — PLATFORM_ICONS, STATUS_COLORS, etc.
    helpers.ts             — formatDate, formatCurrency, parseJsonArray, etc.
  ```
- State management: React Query hooks for data, URL search params for tab routing (`?tab=...`)
- Shared state stays in parent, passed via props

### Stretch Goals (if time permits)

- **Structured logging** (ref 5.2): 20 `eprintln!` calls across 8 files. Most are in CLI tools (acceptable). Replace 3 DEBUG calls in `crates/nora/src/brain/mod.rs` and 1 probe warning in `crates/services/src/services/editron/mod.rs` with `tracing::debug!()`/`tracing::warn!()`.
- **Code splitting** (ref 6.1): Lazy-load org-profile tab content
- **Query monitoring** (ref 5.3): Enable SQLx query logging at DEBUG level
- **Minor type fix:** Replace `Record<string, any>` with `Record<string, unknown>` in `frontend/src/components/notifications/utils.ts:45` (introduced in dogfood merge)

---

## What's NOT in Scope

- PostgreSQL migration (deferred)
- Input validation framework (Phase 2)
- Rate limiting (less urgent for internal use)
- Auth session hardening (less urgent for internal use)
- Frontend unit test infrastructure (Phase 3)
- Large hook decomposition (Phase 3)
- `any` type elimination (incremental, ongoing)

---

## Verification

- [ ] Server no longer panics on malformed UUID/JSON input to MCP endpoints
- [ ] Worktree manager recovers from poisoned mutex (`.unwrap_or_else(|e| e.into_inner())` pattern)
- [ ] Inner async lock has 30s timeout via `tokio::time::timeout()`
- [ ] `LocalDeployment` has `http_client` field; route handlers use it instead of `Client::new()`
- [ ] HTTP client has 30s default timeout configured
- [ ] `PageErrorBoundary` component exists and wraps route-level `<Suspense>` blocks
- [ ] Org-profile page: error in one tab doesn't crash other tabs (8 error boundaries)
- [ ] All 40 fixable `.unwrap()` calls eliminated from the 8 target files (5 Regex unwraps kept)
- [ ] `json_str()` and `json_value()` helpers exist and are used
- [ ] CI pipeline `.github/workflows/ci.yml` exists and passes on current dev branch
- [ ] `api.ts` split: `npm run generate-types:check` still passes, no import errors
- [ ] `organization-profile.tsx` split: all 8 tabs render correctly
- [ ] `cargo clippy` and `npx tsc --noEmit` pass after all changes

## Execution Order

```
Day 1: Deadlock fix → HTTP client → Error boundaries (3 independent items, can parallelize)
Day 2: unwrap elimination (MCP task_server, events, activity)
Day 3: unwrap elimination (airtable, task_attempts, tasks, connector, workflow_execution) + CI pipeline setup
Day 4: api.ts split (~60 API modules → 12 files + barrel export)
Day 5: org-profile.tsx split (8 tab components) + verification pass
```

## Implementation Notes (from codebase research)

### Key Files
- Error type: `ApiError` in `crates/server/src/error.rs` (23+ variants, implements `IntoResponse`)
- App state: `LocalDeployment` in `crates/local-deployment/src/lib.rs` (lines 41-60), aliased as `DeploymentImpl` in `crates/server/src/lib.rs:18`
- Shared HTTP client: add `http_client: reqwest::Client` field to `LocalDeployment`, initialize in `Deployment::new()` with 30s timeout
- Routes receive state via `State(deployment)` pattern, assembled in `crates/server/src/routes/mod.rs:273` with `.with_state(deployment)`
- Existing fallback: `frontend/src/components/error-boundary-fallback.tsx` (full-page only — uses `min-h-screen`)
- Route definitions: `frontend/src/App.tsx` (uses `React.lazy()` + `<Suspense>`)
- Sentry: already configured in `main.tsx` with `@sentry/react` v9.34.0 (use `Sentry.ErrorBoundary` for per-section boundaries)

### Dependency Status
- `reqwest` already in `local-deployment`, `server`, and `services` Cargo.toml (v0.12, features: json)
- `once_cell` already in `server` Cargo.toml (v1.20) — ready for shared client singleton
- `tokio` in `services` with `features = ["full"]` — includes `time` for timeout support
- `react-error-boundary` NOT installed — not needed, use `@sentry/react` ErrorBoundary instead
- `cargo-audit` NOT in flox manifest — install in CI workflow or add to manifest

### Patterns to Follow
- MCP task_server: most unwraps are `serde_json::to_string_pretty()` and `to_value()` — use `json_str()`/`json_value()` helpers
- events.rs: mix of `to_value().unwrap()` and `from_value().unwrap()` — the `from_value` calls need `map_err` since they're deserialization (can fail on bad data)
- airtable.rs: 7 of 10 unwraps are `config.airtable.token.as_ref().unwrap()` — extract to a helper method `fn get_airtable_token(config) -> Result<&str, ApiError>`
- tasks.rs: 2 unwraps are `serde_json::to_string(mcps/tags).unwrap()` — same `json_str()` helper
- activity.rs: 3 unwraps are `serde_json::to_string(v).unwrap()` inside `.map()` — need `filter_map` or `try_collect` pattern
- workflow_execution.rs: `data.as_object().unwrap()` after `is_object()` check — refactor to `if let Some(obj)` pattern; 5x `Regex::new().unwrap()` are acceptable (compile-time-known patterns, should ideally be `lazy_static` but not a crash risk)
