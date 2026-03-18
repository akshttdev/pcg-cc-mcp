# Modularity Sprint 4 — Full-Stack Pattern Completion + Rust Route Splits

**Branch:** `modularity/sprint-4` from `main`
**Date:** 2026-03-16
**Status:** In Progress

## Context

Sprint 3 (PR #40) established infrastructure (query key factory, useMutationWithToast, 4 component splits) but adoption is incomplete: 83% of mutations and ~85% of query keys are still inline. Meanwhile, 3 Rust route files exceed 2,500 lines with duplicated billing logic. This sprint completes frontend pattern adoption for the highest-impact hooks and splits the 3 largest Rust routes.

## Measured Starting State

- 122 of 147 mutations still raw (83%)
- 414 inline query key strings remaining
- 75+ raw fetch() calls without makeRequest
- `enhanced/index.tsx` at 749 lines (Sprint 3 debt)
- 3 Rust route files: topsi.rs (2,800), nora.rs (2,475), organizations.rs (2,665)

---

## Phase 1: Sprint 3 Debt + Monster Hook API Extraction

### 1a. Extract `useTaskPanelData` from `enhanced/index.tsx` (749 -> ~250 lines)

**File:** `frontend/src/components/tasks/TaskDetails/enhanced/index.tsx`

Create:
- `enhanced/useTaskPanelData.ts` — all useState declarations (artifacts, workflow, vibe, chat, etc.), all useEffect data-fetching blocks (agent lookup, chat messages, artifacts, workflow events, vibe balance), handleRefresh, handleArtifactDownload, handleSendMessage (~166 lines), mode detection, initialPrompt memo
- `enhanced/utils.ts` — `detectCardMode`, `normalizeFlowEventType`, `FLOW_EVENT_TYPES` constant

The component `index.tsx` becomes a thin shell: imports hook + utils, destructures returned state, renders JSX (tabs + tab content).

### 1b. Create 3 API modules for monster hooks (~44 raw fetch eliminated)

| Module | Source hook | Endpoints | Fetches |
|--------|-----------|-----------|---------|
| `lib/api/autonomy.ts` | useAutonomy.ts | autonomy-mode, checkpoints, gates, reviews | 18 |
| `lib/api/bowser.ts` | useBowser.ts | sessions, screenshots, actions, allowlist | 14 |
| `lib/api/collaboration.ts` | useCollaboration.ts | collaboration state, pause/resume, takeover, inject | 12 |

All use `makeRequest`/`handleApiResponse` from `lib/api/client.ts`. Export from `lib/api/index.ts`.

### 1c. Fix `useAgents.ts` factory migration

Replace `[...AGENTS_QUERY_KEY, 'active']` spreads with `agentKeys.active()`, `agentKeys.detail(id)`, `agentKeys.byName(name)`. Migrate 6 mutations to `useMutationWithToast`.

---

## Phase 2: Monster Hook Rewrites + Query Key Adoption

### 2a. Add query key factories for 3 domains

**File:** `lib/query-keys.ts` — add `autonomyKeys`, `bowserKeys`, `collaborationKeys` (~60 lines total)

### 2b. Rewrite 3 monster hooks

| Hook | Before | After | Keys | Mutations |
|------|--------|-------|------|-----------|
| `useAutonomy.ts` | 478 lines, 18 fetch, 26 keys | ~200 lines | 26 -> factory | 10 -> useMutationWithToast |
| `useBowser.ts` | 335 lines, 14 fetch, 13 keys | ~150 lines | 13 -> factory | 6 -> useMutationWithToast |
| `useCollaboration.ts` | 325 lines, 12 fetch, 12 keys | ~130 lines | 12 -> factory | 7 -> useMutationWithToast |

Each rewrite: replace `fetch()` -> `apiModule.*`, inline keys -> factory, `useMutation` -> `useMutationWithToast`.

---

## Phase 3: Rust Route Splits

### 3a. `topsi.rs` (2,800 lines) -> `topsi/` directory

```
crates/server/src/routes/topsi/
  mod.rs        (~500) — init, get_status, get_instance, shared types, topsi_routes()
  chat.rs       (~150) — chat_with_topsi, execute_command
  voice.rs      (~400) — synthesize, transcribe, voice_interaction, config
  meetings.rs   (~700) — 10 meeting handlers
  admin.rs      (~200) — admin prompt, user settings, tool risk map
  topology.rs   (~350) — topology overview, issues, recommendations
```

### 3b. `nora.rs` (2,475 lines) -> `nora/` directory

```
crates/server/src/routes/nora/
  mod.rs           (~500) — init, get_status, get_instance, shared types, nora_routes()
  chat.rs          (~300) — chat, chat_stream
  voice.rs         (~400) — synthesize, transcribe, voice_interaction, analytics
  coordination.rs  (~250) — stats, agents, directives, SSE
  modes.rs         (~200) — list/apply modes, rapid playbook, graph plans
  project_ops.rs   (~200) — create project/board/task via nora
```

### 3c. `organizations.rs` (2,665 lines) -> `organizations/` directory

```
crates/server/src/routes/organizations/
  mod.rs        (~500) — CRUD, router composition
  members.rs    (~500) — list/add/change/remove members, assignments
  brand.rs      (~400) — brand profile, research, seeding
  knowledge.rs  (~200) — knowledge entries
  intake.rs     (~150) — intake tokens, context, submission
```

### 3d. Extract billing helpers

**Create:** `crates/server/src/helpers/billing.rs`

Extract from topsi.rs + nora.rs:
- `resolve_billing_project(pool, explicit_project_id, user_id) -> Option<Uuid>`
- `check_vibe_balance(pool, project_id) -> Result<(), ApiError>`

Replace 5 duplicated blocks across topsi.rs and nora.rs.

---

## Phase 4: Frontend Mutation/Key Sweep + Final Verification

### 4a. Top mutation consumer migration

| File | Mutations | Pattern |
|------|-----------|---------|
| `CrmPipelineSettings.tsx` (724 lines) | 6 | -> useMutationWithToast |
| `AgentSettings.tsx` (918 lines) | 5 | -> useMutationWithToast |
| `ApprovalPanel.tsx` | 4 | -> useMutationWithToast |
| `MembersTab.tsx` | 5 | -> useMutationWithToast |
| `OrganizationsSettings.tsx` | 7 | -> useMutationWithToast |

### 4b. Top query key consumer migration

| File | Keys | Domain |
|------|------|--------|
| `pulse.tsx` | 9 | pulseKeys.* |
| `oss-library-listener.tsx` | 9 | ossKeys.* |
| `organization-profile/index.tsx` | 7 | organizationKeys.* |
| `data-source-detail.tsx` | 6 | dataSourceKeys.* |
| `project-controller.tsx` | 7 | controllerKeys.* |

### 4c. Final sweep & verification

- `npx tsc --noEmit` — zero errors
- `flox activate -- cargo check --workspace` — zero errors
- Grep remaining inline keys/fetch/mutations for progress metrics

---

## Expected Outcomes

| Metric | Sprint 3 End | Sprint 4 Target |
|--------|-------------|-----------------|
| Raw fetch() calls | 75+ | ~20 |
| useMutationWithToast adoption | 25/147 (17%) | ~85/147 (58%) |
| Inline query keys | 414 | ~250 |
| `enhanced/index.tsx` | 749 lines | ~250 lines |
| Rust files >2000 lines | 3 | 0 |
| Duplicated billing code | 5 blocks | 0 |

## Actual Results (PR #42)

**Status:** Complete — `npx tsc --noEmit` (0 errors), `cargo check --workspace` (0 errors)

| Metric | Sprint 3 End | Target | Actual |
|--------|-------------|--------|--------|
| Raw fetch() in monster hooks | 44 | 0 | **0** |
| useMutationWithToast adopters | 8 files | - | **17 files** |
| Inline query keys | 414 | ~250 | **311** |
| `enhanced/index.tsx` | 749 lines | ~250 | **246 lines** |
| Rust files >2000 lines (target routes) | 3 | 0 | **0** |
| Duplicated billing code | 5 blocks | 0 | **Deferred** (kept in topsi/chat.rs + nora/chat.rs) |

### Deferred to Sprint 5
- **Billing helper extraction** (Phase 3d) — billing logic kept in topsi/chat.rs and nora/chat.rs; extract to shared `helpers/billing.rs` in next sprint
- **Over-invalidation** (M1 from QA review) — autonomy/bowser/collaboration mutations use broad `autonomyKeys.all` invalidation instead of targeted keys; refine in next sprint
- **oss-library-listener mutations** — query keys migrated but 4 mutations still use raw `useMutation`; migrate in next sprint
- **MembersTab raw fetch** — `allUsers` query still uses `fetch()` instead of `makeRequest`

## Verification Commands

```bash
# Frontend type check
cd frontend && npx tsc --noEmit

# Rust compile check
flox activate -- cargo check --workspace

# Progress metrics
grep -rn "fetch(" frontend/src/hooks/useAutonomy.ts frontend/src/hooks/useBowser.ts frontend/src/hooks/useCollaboration.ts | wc -l
grep -rn "useMutationWithToast" frontend/src/ | wc -l
grep -rn "queryKey: \['" frontend/src/ | wc -l
```
