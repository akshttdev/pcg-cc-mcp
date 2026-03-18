# Dev Velocity Sprint — 10 Days

## Context

Post-sloperation316 merge (PR #45), the codebase has ~251 files of new CRM/sovereign-stack features layered on top of 5 modularity sprints and a tech debt sprint. Current metrics show significant remaining friction:

- **404 `.unwrap()` calls** in non-test Rust code (40 eliminated in PR #34, ~364 remain)
- **188 `: any` type usages** in frontend (masks type errors, slows refactoring)
- **67 files** with inline query keys (cache invalidation is fragile)
- **50 files** with raw `useMutation` (no standardized error/success handling)
- **10 files** >900 lines in frontend (merge conflict magnets)

**Parallel branch**: `refactor/quality-sprint-dialogs-modularity` has 7 unmerged commits (query keys, mutations, MeetingMode/NoraAssistant/project-tasks splits). Will be merged separately — we work in non-conflicting areas and integrate when it lands. This means:
- **Skip**: NoraAssistant split, project-tasks split (already done there)
- **Skip**: Query key work in org-profile/sidebar (already done there)
- **Non-conflicting**: DbUuid (Rust only), unwrap elimination (Rust only), `any` types (different TS files), route authz (Rust only), sovereign hardening (Rust only), virtual-environment + data-sources splits (untouched by quality sprint)

**Goal**: Increase development speed and reduce regressions by eliminating the top sources of bugs, merge conflicts, and wasted debugging time.

---

## Sprint Workstreams (Priority Order)

### A. DbUuid Phase A — `AccessContext.user_id` migration (Days 1-2)

**Why highest ROI**: Every authenticated request flows through `AccessContext`. Currently `Uuid` type forces ~36 `.to_string()` / `.as_bytes()` conversions scattered across 18 route files. This is the #1 source of BLOB/TEXT binding bugs (caused 3 issues in PR #45 alone).

**Scope**:
1. Change `AccessContext { user_id: Uuid }` → `AccessContext { user_id: DbUuid }`
2. Update `build_access_context` in `middleware/access_control.rs`
3. Update `orcha_auth.rs` construction site
4. Remove `.to_string()` at ~26 route handler call sites
5. Use `bind_uuid_blob(&self.user_id)` for the 4 remaining BLOB columns
6. Remove `DbUuid::from(access_context.user_id)` bridge calls (~10 sites)

**Files**: `middleware/access_control.rs`, `middleware/orcha_auth.rs`, + ~17 route files
**Risk**: Medium — validate with login flow + API smoke test
**Ref**: `planning/2026-03-17--plan--dbuuid-migration.md`

### B. Unwrap Elimination — Round 2 (Days 2-3)

**Why**: 364 remaining `.unwrap()` calls. Any one crashes the server. PR #34 fixed the top 40 across 8 files and established the `json_str`/`json_value` helper pattern. This round targets the next ~80 highest-traffic unwraps.

**Scope** (top files by unwrap count from agent research):
1. `crates/db/src/models/*.rs` — ~50 `serde_json::to_string().unwrap()` in model serialization
2. `crates/nora/src/` — ~15 unwraps in tool execution, voice, crawler
3. `crates/server/src/mcp/task_server/*.rs` — ~12 unwraps in split MCP modules
4. `crates/server/src/sovereign_stack.rs` — ~5 unwraps in new code from PR #45
5. `crates/topsi/src/` — ~8 unwraps in platform data

**Approach**: Extend existing `json_str`/`json_value` helpers from PR #34. Add `json_str_or_default()` for model serialization where returning `"{}"` is acceptable.
**Target**: Reduce from 404 → ~320 (eliminate ~80 highest-risk unwraps)

### C. `any` Type Elimination — Systematic Pass (Days 3-5)

**Why**: 188 `: any` lines prevent TypeScript from catching bugs at compile time. Every `any` is a type error that will only surface at runtime. The sloperation316 merge added ~25 new `any` usages.

**Scope**:
1. **client-overview.tsx** (~20 `any` casts) — define proper `ClientProject`, `ClientMember` interfaces
2. **CRM deal/pipeline components** (~15) — type deal stage triggers, move handlers
3. **Route handler response types** (~10) — type API responses properly
4. **Remaining scattered** (~30) — across pages/components

**Approach**:
- Start with files that have 5+ `any` usages (highest concentration)
- Use `shared/types.ts` generated types where available
- Add new interfaces in component files for API shapes not in shared types
- Target: 188 → ~60 (eliminate ~128, leaving only genuinely dynamic/external data)

### D. Fine-Grained Route Authorization (Days 5-6)

**Why**: QA review of PR #45 found that `communications.rs` and new CRM deal endpoints lack org-level authorization. All routes are behind `require_auth` (authenticated) but don't verify the user has access to the specific org/project/deal they're querying. This is a security gap.

**Scope**:
1. `routes/communications.rs` — add org/project membership checks to all 10 handlers
2. `routes/crm_deals.rs` — add org membership checks to new deal action endpoints (generate_proposal, approve_proposal, generate_deck, send_invoice, mark_won, list_transcripts, link_transcript)
3. `routes/org_cloud.rs` — audit existing `require_org_cloud_access` pattern for completeness

**Approach**: Follow existing `require_org_cloud_access` pattern. Extract reusable `require_project_access` / `require_org_membership` helpers if patterns repeat.

### E. Large File Splits — 2 Remaining Files (Days 6-7)

**Why**: Files >900 lines are merge conflict magnets and slow to navigate.

**Already done** (via quality sprint branch, merged in Step 0):
- MeetingMode (1207 → 4 files)
- NoraAssistant (1027 → 5 files)
- project-tasks (997 → 4 files)

**Remaining targets for this sprint**:
1. **`virtual-environment.tsx`** (1,282 lines) — split into `pages/virtual-environment/`
2. **`data-sources.tsx`** (991 lines) — split into `pages/data-sources/`

**Approach**: Follow the pattern established in PR #34 (org-profile split) and quality sprint (MeetingMode/NoraAssistant splits): extract types, extract sub-components, keep orchestrator in index.tsx, barrel exports for backward compatibility.

### F. Sovereign Stack Hardening (Days 8-9)

**Why**: New code from PR #45 has several quality issues identified in QA.

**Scope**:
1. Add `CancellationToken` to `SovereignStackService::start()` for graceful shutdown
2. Add `tracing::warn!` to `Manifest::load()` on parse errors (currently silent)
3. Extract `resolve_volume_path` to shared utility (duplicated in `org_cloud.rs` and `data_sources.rs`)
4. Add missing `REFERENCES` constraints in `org_cloud` migration (org_id, project_id, task_id FKs)

### G. Planning & Documentation Cleanup (Day 10)

**Scope**:
1. Update `BACKLOG--remaining-work.md` with sprint results
2. Archive completed planning docs
3. Update quality sprint tracker with current metrics
4. Write sprint retrospective

---

## Daily Schedule

| Day | Workstream | Deliverable |
|-----|-----------|-------------|
| 1 | A: DbUuid Phase A | `AccessContext.user_id` → `DbUuid`, 18 files updated |
| 2 | A+B: Finish DbUuid + start unwrap elimination | Phase A complete, model unwraps started |
| 3 | B+C: Finish unwraps + start `any` elimination | ~80 unwraps fixed, `any` pass in CRM/pipeline |
| 4 | C: Continue `any` elimination | client-overview, deal components typed |
| 5 | C+D: Finish `any` + start route authz | ~128 `any` removed, communications authz |
| 6 | D+E: Finish authz + virtual-environment split | CRM deal authz, virtual-environment.tsx → directory |
| 7 | E+F: data-sources split + sovereign hardening | data-sources.tsx → directory, cancellation token |
| 8 | F: Finish sovereign hardening | Shared volume resolver, FK constraints |
| 9 | H: DbUuid Phase B — CRM routes | `Path<Uuid>` → `Path<String>` in ~30 CRM/deal route handlers |
| 10 | G: Docs + retro | Backlog updated, sprint retrospective |

---

## Success Metrics

| Metric | Before | Target | Tracking |
|--------|--------|--------|----------|
| `.unwrap()` (non-test Rust) | 404 | ~320 | `grep -rn '\.unwrap()' crates/ --include='*.rs' \| wc -l` |
| `: any` (frontend) | 188 | ~60 | `grep -rn ': any' frontend/src/ --include='*.ts' --include='*.tsx' \| wc -l` |
| Files >900 lines | 10 (7 after quality sprint merge) | 5 | `find frontend/src -name '*.tsx' -o -name '*.ts' \| xargs wc -l \| awk '$1>900'` |
| Unauthz'd route handlers | ~17 | 0 | Manual audit |
| BLOB/TEXT binding bugs | Recurring | Eliminated for AccessContext | DbUuid Phase A |

---

## Verification

After each workstream:
- `flox activate -- cargo check --workspace` (Rust)
- `cd frontend && npx tsc --noEmit` (TypeScript)
- Login flow smoke test (after DbUuid Phase A)
- API endpoint spot checks (after route authz)

---

## Out of Scope

- Quality sprint work (Days 6-8 of that sprint) — separate branch
- DbUuid Phases B-D — future sprints
- PostgreSQL migration — deferred
- Workflow trigger system — separate feature
- Agent flow orchestration engine — separate feature (P0 but architectural)

---

## Decisions

- **Day 1 priority**: DbUuid Phase A — eliminates recurring BLOB/TEXT binding bugs
- **Unwrap scope**: ~80 highest-traffic (404 → ~320) — keeps time for file splits
- **File splits**: 2 remaining after quality sprint merge — virtual-environment (1282), data-sources (991). NoraAssistant and project-tasks already split on refactor branch.
- **Day 9**: DbUuid Phase B — `Path<Uuid>` → `Path<String>` in CRM route handlers (~30 files)
- **Quality sprint**: Work in non-conflicting areas (all Rust work + virtual-environment/data-sources splits). Merge quality sprint branch when available.
