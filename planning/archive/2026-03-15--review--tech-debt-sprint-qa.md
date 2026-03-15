# QA Review — Tech Debt Sprint PR #34

**Date:** 2026-03-15
**PR:** #34 (`tech-debt/sprint-2026-03-15` → `main`)
**Merge:** Squash merged as `db9c018`
**Reviewers:** 3 parallel QA agents (Rust backend, org-profile split, api.ts split)

---

## Review Scope

| Area | Files | Lines Changed |
|------|-------|---------------|
| Rust backend (unwrap elimination, shared HTTP client, CI) | 15 files | ~600 |
| api.ts split | 22 files | ~6,700 |
| organization-profile.tsx split | 34 files | ~7,700 |
| Stretch goals (logging, type fix, e2e commands) | 5 files | ~200 |
| Planning docs | 3 files | ~300 |
| **Total** | **83 files** | **+15,148 / -13,837** |

---

## Findings Fixed Before Merge

### Org-Profile Split (4 issues → all resolved)

| # | Severity | Finding | Fix |
|---|----------|---------|-----|
| 1 | HIGH | Pipeline types/constants (`PipelineNode`, `PipelineBlueprint`, `CONFERENCE_PIPELINE`, `EDITRON_PIPELINE`, `STATIC_PIPELINES`, `AGENT_COLORS`) duplicated in both `constants.ts` and `WorkflowsView.tsx` — constants.ts copies were dead code | Removed dead code from `constants.ts` |
| 2 | MEDIUM | `SOURCE_TYPE_META` duplicated in `constants.ts` and `KnowledgeTab.tsx` — constants.ts copy unused | Removed from `constants.ts` |
| 3 | MEDIUM | `formatDate` duplicated in `PulseSection.tsx` (local copy) instead of importing from `helpers.ts` | Changed to `import { formatDate } from '../../helpers'` |
| 4 | MEDIUM | `BrandIdentityCard` not re-exported from `index.tsx` — external consumers would lose access | Added `export { BrandIdentityCard } from './components/BrandIdentityCard'` to `index.tsx` |

**Fix commit:** `5948c05df` (pushed to PR before merge)

---

## Findings Accepted (Pre-existing, Not Introduced by PR)

### Rust Backend

| # | Severity | File | Finding | Recommendation |
|---|----------|------|---------|----------------|
| 1 | CRITICAL | `crates/services/src/services/apn_bridge/connector.rs:154` | `.expect("Failed to create HTTP client")` in non-test constructor — will panic if TLS backend unavailable | Return `Result` from `APNConnector::new()` or use `unwrap_or_else` with fallback client |
| 2 | WARNING | `crates/services/src/services/events.rs` (lines 54-125) | 10 `.expect()` calls on `try_into()` and `serde_json::to_value()` — violates project Rust standards | Replace with `.map_err()` + `?` |
| 3 | WARNING | `crates/server/src/routes/task_attempts.rs:636` | `.expect("current checked via queued above")` — logically safe but violates standards | Destructure with guard or use `unreachable!()` |
| 4 | WARNING | `crates/services/src/services/workflow_execution.rs` (lines 246-498) | 6 `Regex::new().unwrap()` on hardcoded patterns — should be `lazy_static!` for both correctness and performance | Convert to `lazy_static!` or `once_cell::sync::Lazy` |
| 5 | NIT | `crates/local-deployment/src/lib.rs:155-159` | HTTP client fallback via `unwrap_or_else(|_| reqwest::Client::new())` creates a client with no timeout, silently defeating the purpose | Log warning in fallback branch |
| 6 | NIT | `crates/services/src/services/apn_bridge/connector.rs:400` | `.unwrap_or_default()` on ping serialization sends empty string over WebSocket | Consider skipping ping instead of sending empty string |

### CI Pipeline

| # | Severity | Finding | Recommendation |
|---|----------|---------|----------------|
| 7 | NIT | `cargo clippy` and `cargo test` missing `--all-features` — postgres feature code paths not linted | Add `--all-features` flag |
| 8 | NIT | `cargo audit` has `continue-on-error: true` (soft audit) | Intentional; revisit when moving to production |

---

## Clean Areas (No Issues Found)

### api.ts Split — All Checks Pass

| Check | Status |
|-------|--------|
| Barrel re-exports complete (21 modules via `index.ts`) | PASS |
| Backward compatibility (`@/lib/api` resolves to `api/index.ts`) | PASS |
| Shared client (`makeRequest`, `resolveApiUrl`) properly used | PASS |
| All type exports accessible from domain modules | PASS |
| 80+ consumer imports verified resolving | PASS |
| No circular dependencies | PASS |
| TypeScript compiles (0 new errors) | PASS |

**Note:** 5 cross-module `import type` statements exist between domain modules (e.g., `tasks.ts` imports `AssignedTask` from `projects.ts`). These are compile-time only with no runtime risk. Consider moving shared types to a `types.ts` module for full decoupling in the future.

### Org-Profile Split — Structure Verified

| Check | Status |
|-------|--------|
| All 33 components/tabs present with correct exports | PASS |
| Lazy loading (React.lazy + Suspense + ErrorBoundary per tab) | PASS |
| Social/Intelligence/Integrations index.tsx default exports | PASS |
| All relative import paths correct | PASS |
| No missing functions from original file | PASS |

### Rust Unwrap Elimination — Target Files Clean

| File | Original Unwraps | Remaining | Status |
|------|-----------------|-----------|--------|
| `mcp/task_server.rs` | 15 | 0 | CLEAN |
| `routes/airtable.rs` | 10 | 0 | CLEAN |
| `services/events.rs` | 9 | 0 new (10 pre-existing `.expect()`) | CLEAN (PR scope) |
| `db/models/activity.rs` | 3 | 0 | CLEAN |
| `routes/tasks.rs` | 2 | 0 | CLEAN |
| `routes/task_attempts.rs` | 2 | 0 new (1 pre-existing `.expect()`) | CLEAN (PR scope) |
| `services/workflow_execution.rs` | 2 fixable + 5 regex | 0 fixable + 5 regex (acceptable) | CLEAN |
| `services/apn_bridge/connector.rs` | 1 | 0 new (1 pre-existing `.expect()`) | CLEAN (PR scope) |

### Structured Logging

| File | Change | Status |
|------|--------|--------|
| `crates/nora/src/brain/mod.rs` | 3x `eprintln!` → `tracing::debug!()` | CLEAN |
| `crates/services/src/services/editron/mod.rs` | 1x `eprintln!` → `tracing::warn!()` | CLEAN |

---

## E2E Test Results

| Suite | Passed | Failed | Notes |
|-------|--------|--------|-------|
| Health checks | 40/40 | 0 | All core flows verified |
| Demo suite | 28/31 | 1 | Pre-existing flaky locator in `bug-report-lifecycle.spec.ts:168` — `getByText(/Passed|qa_pass/i)` matches 2 elements. 2 tests skipped downstream. |

---

## Action Items for Future Sprints

1. **[P2]** Fix `connector.rs:154` `.expect()` — return `Result` from constructor
2. **[P2]** Convert 6 regex `.unwrap()` in `workflow_execution.rs` to `lazy_static!`
3. **[P3]** Replace 10 `.expect()` calls in `events.rs` with proper error propagation
4. **[P3]** Add `--all-features` to CI clippy/test commands
5. **[P3]** Fix flaky `bug-report-lifecycle.spec.ts:168` locator — use more specific selector
6. **[P3]** Move cross-module `import type` statements in api.ts modules to shared `types.ts`
