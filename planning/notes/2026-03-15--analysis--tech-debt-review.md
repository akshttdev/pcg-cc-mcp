# Tech Debt & Production Readiness Review

**Date:** 2026-03-15
**Scope:** Full stack — Rust backend, React frontend, infrastructure, CI/CD, security
**Branch:** `dev/2026-03-14` (main)

---

## Executive Summary

The platform is functional and feature-rich but showing signs of rapid growth without proportional architectural discipline. The codebase spans ~250K LOC across 10+ Rust crates and 550+ TypeScript files. Key concerns for production readiness:

1. **Reliability**: ~100+ `.unwrap()` calls in production Rust code — any one can crash the server
2. **Scalability**: SQLite cannot handle concurrent writes at scale
3. **Security**: Missing rate limiting on most endpoints, no input validation framework
4. **Maintainability**: Several monolithic files (api.ts: 6.6K lines, organization-profile.tsx: 7K lines)
5. **Observability**: Minimal structured logging, no query performance monitoring

The good news: SQL injection is well-protected (SQLx parameterized queries), XSS prevention via React, auth is implemented, and the type generation pipeline (Rust → TypeScript) is solid.

---

## Platform Overview

### Architecture
| Layer | Tech | Status |
|-------|------|--------|
| **Backend** | Rust, Axum, Tokio | Functional, needs error handling hardening |
| **Frontend** | React 18, TypeScript, Vite, Tailwind, shadcn/ui | Functional, needs modularization |
| **Database** | SQLite + SQLx | Works for dev, not production-ready |
| **Real-time** | SSE (Server-Sent Events) | Functional |
| **AI Integration** | MCP server, executor pattern | Functional, needs resilience |
| **Auth** | GitHub OAuth (device flow) | Functional, needs session hardening |
| **CI/CD** | GitHub Actions (Tauri release only) | Incomplete — no test/lint CI |
| **Deployment** | Flox dev env, Docker available | Dev-focused, no prod deployment guide |

### Codebase Scale
| Area | Metric |
|------|--------|
| Rust crates | 10 (server, db, services, executors, utils, nora, local-deployment, ...) |
| TypeScript files | 558 |
| Frontend pages | 74 |
| React components | 550+ |
| API endpoints | 200+ (estimated from route files) |
| DB migrations | 202 |
| Zustand stores | 15 |
| Context providers | 10 |

---

## Findings by Category

### Legend

| Field | Values |
|-------|--------|
| **Severity** | CRITICAL > HIGH > MEDIUM > LOW |
| **Effort** | S (< 1 day) · M (1-3 days) · L (1-2 weeks) · XL (2+ weeks) |
| **Impact** | How much production readiness improves |
| **ROI** | Impact / Effort — prioritize HIGH ROI first |
| **Blocker** | Does this block other work? |

---

## 1. RELIABILITY & ERROR HANDLING

### 1.1 `.unwrap()` / `.expect()` in Production Rust Code

| Field | Value |
|-------|-------|
| Severity | CRITICAL |
| Effort | L |
| Impact | HIGH |
| ROI | HIGH |
| Blocker | No, but any unwrap can crash the server |

**Problem:** 100+ `.unwrap()` calls in non-test Rust code. Any one can panic and crash the server process. Hotspots:

| File | Count | Risk |
|------|-------|------|
| `crates/server/src/mcp/task_server.rs` | 15+ | MCP tool execution crashes |
| `crates/server/src/routes/airtable.rs` | 9 | Airtable sync crashes |
| `crates/services/src/services/events.rs` | 10+ | Real-time event stream crashes |
| `crates/server/src/routes/task_attempts.rs` | 2 | Task execution crashes mid-flight |
| `crates/services/src/services/apn_bridge/connector.rs` | 2 | Mesh networking crashes |
| `crates/db/src/models/*.rs` | 50+ | JSON serialization panics |

**Common patterns:**
```rust
serde_json::to_string(&value).unwrap()       // Serialization assumed infallible
let user_id = self.user_id.unwrap();          // Optional assumed present
Uuid::parse_str(&external_id).unwrap()        // External input assumed valid
HeaderValue::from_str(&cookie).unwrap()       // Cookie chars assumed valid
```

**Fix:** Create helper `fn json_str<T: Serialize>(v: &T) -> Result<String>`, replace unwraps with `?` operator. Prioritize by traffic: MCP server > event streams > route handlers > models.

---

### 1.2 Global Mutex Without Timeout (Deadlock Risk)

| Field | Value |
|-------|-------|
| Severity | CRITICAL |
| Effort | S |
| Impact | HIGH |
| ROI | VERY HIGH |
| Blocker | Yes — can deadlock entire executor |

**File:** `crates/services/src/services/worktree_manager.rs:88,386`

```rust
let mut locks = WORKTREE_CREATION_LOCKS.lock().unwrap();
```

If lock holder panics, all future worktree operations deadlock permanently. Server requires restart.

**Fix:** Use `tokio::sync::Mutex` with `tokio::time::timeout(Duration::from_secs(30), ...)`. Or use `std::sync::Mutex` with `lock().unwrap_or_else(|e| e.into_inner())` for poisoned mutex recovery.

---

### 1.3 Missing Graceful Shutdown

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | M |
| Impact | HIGH |
| ROI | HIGH |
| Blocker | No |

**File:** `crates/server/src/main.rs`

14+ unbounded `tokio::spawn()` calls with no cancellation tokens, no task tracking, no signal handling. On SIGTERM:
- In-flight requests dropped
- Database writes may be incomplete
- SSE connections not cleanly closed
- Spawned tasks leak

**Fix:** Implement `CancellationToken` pattern. Use `tokio::task::JoinSet` to track spawned tasks. Add SIGTERM/SIGINT handler that drains connections before exit.

---

### 1.4 No Granular Error Boundaries (Frontend)

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | S |
| Impact | MEDIUM |
| ROI | HIGH |
| Blocker | No |

**Current:** Single `ErrorBoundary` at app root (Sentry-powered). If any component in a page throws, the entire app shows a blank error page.

**Fix:** Add error boundaries around:
- Each major page (`<ErrorBoundary>` wrapping route content)
- Each tab within multi-tab pages (org-profile has 8+ tabs)
- Dialog/modal content areas

---

## 2. SCALABILITY & DATABASE

### 2.1 SQLite Not Suitable for Production

| Field | Value |
|-------|-------|
| Severity | CRITICAL |
| Effort | XL |
| Impact | HIGH |
| ROI | MEDIUM (high effort) |
| Blocker | Yes — blocks horizontal scaling |

**Problems:**
- Write lock on entire database (no concurrent writes)
- No horizontal scaling (single file)
- Single point of failure (no replicas)
- No connection pooling
- 202 migrations with no rollback testing

**Mitigations already in place:** WAL mode, good pragma configuration.

**Fix:** Migrate to PostgreSQL. SQLx already supports it — queries use `?` placeholders (SQLite-compatible). Main work: migration conversion, BLOB UUID handling, JSON column types. Consider: do this BEFORE production launch, not after.

**Interim:** If staying on SQLite short-term, add:
- Automated backups (hourly)
- Migration rollback testing
- Write queue to serialize mutations

---

### 2.2 Missing Database Indexes

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | M |
| Impact | HIGH |
| ROI | HIGH |
| Blocker | No |

Migration `20260320000000_add_org_scoping_indexes.sql` added indexes retroactively, suggesting gaps. No comprehensive index audit documented.

**Fix:** Audit all foreign key columns. Enable SQLx query logging to find slow queries. Add missing indexes in a new migration.

---

### 2.3 N+1 Query Patterns

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | L |
| Impact | HIGH |
| ROI | MEDIUM |
| Blocker | No |

No evidence of query optimization strategy. 96 service files likely contain hidden N+1 patterns (e.g., loading org → loading clients → loading projects per client → loading boards per project).

**Fix:** Enable query logging. Profile top-10 endpoints by request count. Add JOINs or batch queries where N+1 found.

---

## 3. SECURITY

### 3.1 No Rate Limiting on Most Endpoints

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | M |
| Impact | HIGH |
| ROI | HIGH |
| Blocker | No |

`tower_governor` is in dependencies but only applied to Nora routes. All other API endpoints are unprotected. Vulnerable to:
- Brute force login attempts
- API abuse / scraping
- Resource exhaustion via expensive endpoints (agent chat, workflow execution)

**Fix:** Add global rate limiter middleware (100 req/min per user default). Add per-endpoint overrides for expensive operations (10 req/min for agent chat, 5 req/min for workflow execution).

---

### 3.2 No Input Validation Framework

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | L |
| Impact | HIGH |
| ROI | MEDIUM |
| Blocker | No |

Manual validation scattered throughout code. No centralized validation. Easy to miss edge cases. UUID format checks done inconsistently. No max string length enforcement.

**Fix:** Adopt `validator` crate. Add validation derives to all request structs. Create middleware that validates before handler runs.

---

### 3.3 Auth Session Hardening

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | M |
| Impact | MEDIUM |
| ROI | MEDIUM |
| Blocker | No |

- No session timeout (tokens live forever)
- No CSRF protection on state-changing endpoints
- Default credentials (admin/admin123) lack forced-change-on-first-login
- Cookie generation uses `.unwrap()` on header value construction

**Fix:** Add session expiry (24h default, configurable). Add CSRF token for non-GET requests. Force password change on first admin login.

---

### 3.4 Missing HTTP Client Timeouts

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | S |
| Impact | MEDIUM |
| ROI | HIGH |
| Blocker | No |

Multiple routes create their own `reqwest::Client` without timeouts. Research tools, webhook calls, and external API calls could hang indefinitely, blocking Tokio worker threads.

**Fix:** Create shared global HTTP client with 30s default timeout. Use `once_cell::sync::Lazy<reqwest::Client>`.

---

## 4. MAINTAINABILITY & CODE QUALITY

### 4.1 Monolithic Files

| File | Lines | Severity | Effort | Impact |
|------|-------|----------|--------|--------|
| `frontend/src/lib/api.ts` | 6,669 | HIGH | L | HIGH |
| `frontend/src/pages/organization-profile.tsx` | 7,069 | HIGH | L | HIGH |
| `frontend/src/components/workflows/WorkflowEditor.tsx` | 2,240 | MEDIUM | M | MEDIUM |
| `frontend/src/components/projects/project-detail.tsx` | 2,293 | MEDIUM | M | MEDIUM |
| `frontend/src/dialogs/tasks/TaskFormDialog.tsx` | 1,238 | MEDIUM | M | LOW |

**ROI:** HIGH — these files are the hardest to modify and most likely to cause merge conflicts.

**Fix priority:**
1. `api.ts` → Split into `api/projects.ts`, `api/tasks.ts`, `api/crm.ts`, etc. with barrel re-export
2. `organization-profile.tsx` → Extract 8 tab components into `org-tabs/` directory
3. Others → Follow sidebar modularization pattern (#30)

---

### 4.2 257 `any` Types in Frontend

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | L (incremental) |
| Impact | MEDIUM |
| ROI | MEDIUM |
| Blocker | No |

Spread across 43 files. ESLint `no-explicit-any` set to `warn` not `error`, with 110 warnings allowed.

**Fix:** Change ESLint rule to `error`. Fix incrementally per-file. Prioritize api.ts (24 instances) and hook files first.

---

### 4.3 Large Custom Hooks

| Hook | Lines | Problem |
|------|-------|---------|
| `useConversationHistory.ts` | 540 | Mixes WebSocket, deduplication, UI state |
| `useAutonomy.ts` | 478 | Mixes checkpoint gates, approval gates |
| `useExecutionEvents.ts` | 342 | Complex async generator |
| `useBowser.ts` | 335 | Mixed concerns |
| `useCollaboration.ts` | 325 | Mixed concerns |

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | L |
| Impact | HIGH |
| ROI | MEDIUM |

**Fix:** Decompose into smaller, composable hooks. Extract WebSocket logic into shared `useWebSocketStream` with retry logic.

---

### 4.4 No Unit Tests for Frontend Hooks or Components

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | XL |
| Impact | HIGH |
| ROI | MEDIUM |
| Blocker | No |

558 TypeScript files, 0 unit test files for hooks. Only Rust backend has some tests. Frontend relies entirely on TypeScript compilation + linting.

**Fix:** Add vitest + testing-library. Start with highest-risk hooks (useConversationHistory, useAutonomy). Target 20 test files covering critical paths.

---

## 5. CI/CD & OBSERVABILITY

### 5.1 No CI Pipeline for Core Application

| Field | Value |
|-------|-------|
| Severity | HIGH |
| Effort | M |
| Impact | HIGH |
| ROI | VERY HIGH |
| Blocker | Yes — untested code reaches production |

Only workflow is Tauri desktop release. Missing:
- `cargo test` / `cargo clippy` on PR
- `npx tsc --noEmit` / `npm run lint` on PR
- `npm run generate-types:check` on PR
- Security audit (`cargo audit`, `npm audit`)

**Fix:** Create `.github/workflows/ci.yml` with:
```yaml
jobs:
  backend: cargo fmt --check, cargo clippy, cargo test
  frontend: tsc --noEmit, npm run lint, generate-types:check
  security: cargo audit, npm audit
```

---

### 5.2 Missing Structured Logging

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | M |
| Impact | HIGH |
| ROI | HIGH |
| Blocker | No |

Backend uses `tracing` crate but:
- `eprintln!("[DEBUG] ...")` scattered in production code (e.g., `crates/nora/src/brain/mod.rs:629,760,1197`)
- No JSON log format for production
- No request ID propagation
- Missing `#[tracing::instrument]` on most async functions

**Fix:** Replace all `eprintln!` with `tracing::debug!`. Add `tracing-subscriber` JSON formatter for production. Add request ID middleware. Instrument top-20 hot-path functions.

---

### 5.3 No Query Performance Monitoring

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | S |
| Impact | MEDIUM |
| ROI | HIGH |
| Blocker | No |

No visibility into slow queries, query counts per request, or database lock contention.

**Fix:** Enable SQLx query logging at `DEBUG` level. Add Prometheus histogram for query duration. Alert on queries > 100ms.

---

## 6. FRONTEND PERFORMANCE

### 6.1 Missing Code Splitting for Heavy Pages

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | S |
| Impact | HIGH |
| ROI | VERY HIGH |
| Blocker | No |

`organization-profile.tsx` (7K lines) loads all 8 tabs eagerly. 3D libraries (@pixiv/three-vrm, @react-three/*) in bundle even for non-3D routes.

**Fix:** Lazy-load tab content within org-profile. Dynamic import 3D libraries only for vibeland route (may already be lazy via route splitting — verify).

---

### 6.2 93 Production Dependencies

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | M |
| Impact | MEDIUM |
| ROI | MEDIUM |
| Blocker | No |

Both `lodash` (full) and `lodash-es` imported. Heavy 3D libraries included. Bundle could be 15-20% smaller.

**Fix:** Replace lodash with native JS utilities. Verify 3D libs are code-split. Audit for unused dependencies with `depcheck`.

---

## 7. ARCHITECTURE IMPROVEMENTS

### 7.1 State Management Consolidation

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | L |
| Impact | MEDIUM |
| ROI | LOW |
| Blocker | No |

10 React context providers + 15 Zustand stores with unclear boundaries. Some stores overlap with React Query cache. No documented state taxonomy.

**Fix:** Document which stores own which state. Consolidate related contexts. Move cache-like state to React Query. This is low ROI — defer unless actively causing bugs.

---

### 7.2 Shared HTTP Client (Backend)

| Field | Value |
|-------|-------|
| Severity | MEDIUM |
| Effort | S |
| Impact | MEDIUM |
| ROI | HIGH |
| Blocker | No |

Each route creates its own `reqwest::Client`, fragmenting connection pools.

**Fix:** Create `once_cell::sync::Lazy<reqwest::Client>` with 30s timeout, connection pooling. Inject via Axum state.

---

## Prioritized Remediation Roadmap

### Phase 0: Immediate (< 1 day, before next deploy)
| # | Item | Effort | Impact | Ref |
|---|------|--------|--------|-----|
| 1 | Fix worktree manager deadlock risk | S | CRITICAL | 1.2 |
| 2 | Add shared HTTP client with timeout | S | MEDIUM | 3.4, 7.2 |
| 3 | Add error boundaries to major pages | S | MEDIUM | 1.4 |

### Phase 1: Foundation (1 week)
| # | Item | Effort | Impact | Ref |
|---|------|--------|--------|-----|
| 4 | Create CI pipeline (test + lint + clippy) | M | HIGH | 5.1 |
| 5 | Add global rate limiting | M | HIGH | 3.1 |
| 6 | Replace top-30 critical `.unwrap()` calls | M | HIGH | 1.1 |
| 7 | Enable structured JSON logging | M | HIGH | 5.2 |

### Phase 2: Hardening (2-3 weeks)
| # | Item | Effort | Impact | Ref |
|---|------|--------|--------|-----|
| 8 | Implement graceful shutdown | M | HIGH | 1.3 |
| 9 | Database index audit + migration | M | HIGH | 2.2 |
| 10 | Modularize api.ts | L | HIGH | 4.1 |
| 11 | Modularize organization-profile.tsx | L | HIGH | 4.1 |
| 12 | Add input validation framework | L | HIGH | 3.2 |
| 13 | Auth session hardening | M | MEDIUM | 3.3 |
| 14 | Query performance monitoring | S | MEDIUM | 5.3 |

### Phase 3: Scale Preparation (1-2 months)
| # | Item | Effort | Impact | Ref |
|---|------|--------|--------|-----|
| 15 | Remaining `.unwrap()` elimination | L | HIGH | 1.1 |
| 16 | PostgreSQL migration | XL | HIGH | 2.1 |
| 17 | Frontend unit test infrastructure | XL | HIGH | 4.4 |
| 18 | Decompose large hooks | L | HIGH | 4.3 |
| 19 | Fix 257 `any` types (incremental) | L | MEDIUM | 4.2 |
| 20 | N+1 query audit | L | HIGH | 2.3 |

### Phase 4: Polish (Ongoing)
| # | Item | Effort | Impact | Ref |
|---|------|--------|--------|-----|
| 21 | Reduce ESLint max-warnings to 0 | L | MEDIUM | 4.2 |
| 22 | State management consolidation | L | MEDIUM | 7.1 |
| 23 | Bundle size optimization | M | MEDIUM | 6.2 |
| 24 | Dependency cleanup | M | MEDIUM | 6.2 |
| 25 | Storybook for component library | XL | MEDIUM | — |

---

## Summary Statistics

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Reliability | 3 | 1 | 1 | 0 | 5 |
| Scalability | 1 | 2 | 0 | 0 | 3 |
| Security | 0 | 2 | 2 | 0 | 4 |
| Maintainability | 0 | 3 | 2 | 0 | 5 |
| CI/CD & Observability | 0 | 1 | 2 | 0 | 3 |
| Frontend Performance | 0 | 0 | 2 | 0 | 2 |
| Architecture | 0 | 0 | 2 | 0 | 2 |
| **Total** | **4** | **9** | **11** | **0** | **24** |

**Estimated total effort for Phase 0-2 (production-minimum):** 4-6 weeks
**Estimated total effort for full remediation:** 3-4 months

---

## What's Working Well

- SQL injection protection via SQLx parameterized queries
- XSS prevention via React 18 auto-escaping
- TypeScript strict mode enabled
- Type generation pipeline (Rust → TypeScript via ts-rs)
- Zustand stores use individual key selectors (avoids unnecessary re-renders)
- Route-level code splitting via React.lazy()
- Sentry error tracking integrated
- Flox dev environment ensures consistent toolchain
- Good separation of concerns in executor pattern
- Non-root Docker user configured
