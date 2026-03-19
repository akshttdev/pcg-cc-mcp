# Modularity Sprint 5 — Infrastructure Extraction + Pattern Completion

**Date:** 2026-03-16
**Branch:** `modularity/sprint-5` from `main`
**Status:** Complete (PR #43)

## Context

Sprint 4 (PR #42) completed frontend pattern adoption for 3 monster hooks, split 3 Rust route files into directory modules, and created 3 API modules. However, significant debt remains: 111 raw `fetch()` calls across 53 frontend files, 2 Rust route files >1900 lines, billing logic duplicated in 6+ files, and `nora/mod.rs` still at 1172 lines.

## Measured Starting State (post-Sprint 4)

| Metric | Value |
|--------|-------|
| Raw fetch() calls | 111 across 53 files |
| Inline query key strings | 311 |
| Raw useMutation calls | ~65 across ~31 files |
| nora/mod.rs | 1172 lines |
| twilio.rs | 2285 lines |
| task_attempts.rs | 1909 lines |
| VIBE billing duplication | 5 blocks in 4 files |
| Conversation persistence duplication | 7 files |

## Tier 1: Highest-ROI Infrastructure

### 1a. Create `topsi` API Module + `topsiKeys` Query Key Factory
- NEW: `frontend/src/lib/api/topsi.ts` — wraps all topsi endpoints via makeRequest/handleApiResponse
- EDIT: `frontend/src/lib/query-keys.ts` — add topsiKeys factory
- EDIT: `frontend/src/lib/api/index.ts` — add export
- EDIT: `frontend/src/pages/topsi.tsx` — replace 7 fetch → API module calls
- EDIT: `frontend/src/hooks/api/useTopsiRecommendations.ts` — use API module + topsiKeys

### 1b. Extract VIBE Billing Helpers into `helpers/billing.rs`
- NEW: `crates/server/src/helpers/billing.rs` — `ensure_vibe_balance()` + `record_llm_vibe_usage()`
- EDIT: `crates/server/src/helpers/mod.rs` — add module
- EDIT: `crates/server/src/routes/topsi/chat.rs` — replace ~15 lines with 2 calls
- EDIT: `crates/server/src/routes/nora/chat.rs` — replace ~30 lines with 3 calls (chat + stream)
- EDIT: `crates/server/src/routes/agent_chat.rs` — replace ~25 lines with 2 calls

### 1c. Top-8 Inline Query Key Migration
Replace ~50 inline `queryKey: [...]` with factory calls across 8 files:
1. `components/email/EmailInbox.tsx` → commsKeys
2. `components/communications/CommunicationsInbox.tsx` → commsKeys
3. `pages/workflows/tabs/WorkflowsView.tsx` → workflowKeys
4. `pages/discord.tsx` → discordKeys
5. `pages/social.tsx` → socialKeys (new factory)
6. `components/pulse/PulseWidget.tsx` → pulseKeys
7. `pages/settings/NetworkSettings.tsx` → networkKeys (new factory)
8. `pages/person-profile.tsx` → entityKeys

## Tier 2: Route Splits + Hook Migrations

### 2a. Split `twilio.rs` (2285 lines) → `twilio/` Directory
```
crates/server/src/routes/twilio/
├── mod.rs        (~300) — router, shared types, statics
├── calls.rs      (~700) — inbound/outbound call handlers
├── sms.rs        (~500) — SMS handling, thread buffer
├── webhooks.rs   (~400) — status callbacks
├── state.rs      (~300) — call state management
```

### 2b. Split `task_attempts.rs` (1909 lines) → `task_attempts/` Directory
```
crates/server/src/routes/task_attempts/
├── mod.rs        (~300) — router, shared types
├── creation.rs   (~500) — attempt creation/execution
├── git_ops.rs    (~500) — worktree, branch, rebase, merge
├── artifacts.rs  (~300) — artifact preview/management
├── streaming.rs  (~300) — SSE diff streaming
```

### 2c. Further Decompose `nora/mod.rs` (1172 → ~500 lines)
- `nora/config.rs` (~200) — mode presets, default capabilities, LLM override logic
- `nora/conversation.rs` (~250) — conversation setup/persistence, session management
- `nora/rate_limiter.rs` (~100) — TokenBucket, rate limit state/helpers

### 2d. Create `topiclips` API Module
- NEW: `frontend/src/lib/api/topiclips.ts` (~60 lines)
- EDIT: `frontend/src/lib/query-keys.ts` — add topiclipsKeys
- EDIT: `frontend/src/pages/topiclips.tsx` — replace 7 fetch calls

### 2e. Migrate Remaining Raw `useMutation` in 6 High-Value Hooks
1. `hooks/useAgentFlows.ts` (4 mutations)
2. `hooks/useOrgOnboarding.ts` (4 mutations)
3. `hooks/useWorkflowTemplates.ts` (1 mutation)
4. `hooks/useCrmActivities.ts` (2 mutations)
5. `hooks/useTaskMutations.ts` (3 mutations)
6. `pages/oss-library-listener.tsx` (4 mutations)

### 2f. Extract Conversation Persistence Helper
- NEW: `crates/server/src/helpers/conversations.rs` — `persist_chat_exchange()`
- Replace boilerplate in: topsi/chat.rs, nora/chat.rs, nora/voice.rs, agent_chat.rs, twilio/

## Expected Outcomes

| Metric | Sprint 4 End | Sprint 5 Target |
|--------|-------------|-----------------|
| Raw fetch() calls | 111 | ~75 (-36) |
| Inline query keys | 311 | ~260 (-50) |
| useMutationWithToast files | 17 | ~23 (+6) |
| Rust files >1500 lines | 4 | 2 (brand, data_source_workflows) |
| nora/mod.rs | 1172 lines | ~500 lines |
| Billing duplication | 5 blocks | 0 (centralized) |
| Conversation duplication | 7 files | 0 (centralized) |

## Execution Order

1. **1b** Billing helpers (unblocks twilio split) ✅
2. **1a** Topsi API module + keys ✅
3. **1c** Query key migration (8 files) ✅
4. **2c** nora/mod.rs decomposition ✅
5. **2a** twilio.rs split (depends on 1b) ✅
6. **2b** task_attempts.rs split ✅
7. **2d** topiclips API module ✅
8. **2e** Hook mutation migrations (6 hooks) — deferred to Sprint 6
9. **2f** Conversation persistence helper ✅

## Actual Outcomes

| Metric | Sprint 4 End | Sprint 5 Actual | Target |
|--------|-------------|-----------------|--------|
| Raw fetch() in topsi+topiclips | 14 | **0** | 0 |
| Inline query keys | 311 | **268** (-43) | ~260 |
| Rust files >1500 lines | 4 | **2** (brand, data_source_workflows) | 2 |
| nora/mod.rs | 1172 lines | **718 lines** | ~500 |
| Billing duplication | 5 blocks | **0 (centralized)** | 0 |
| Conversation duplication | 7 files | **Helper created, 2 callers migrated** | 0 |
| twilio.rs | 2285 lines (monolithic) | **8-file directory module** | Split |
| task_attempts.rs | 1909 lines (monolithic) | **6-file directory module** | Split |

### Deferred to Sprint 6
- **2e** Hook mutation migrations (6 hooks with raw `useMutation`) — lower priority
- Remaining 3 conversation helper call sites (nora/voice, agent_chat, twilio) — depends on 2a completion
- nora/mod.rs further reduction to ~500 lines (types account for remaining ~200 lines)

## Merge Conflicts

### PR #44 (`feature/ux-sprint-3`) — Workflow UX Sprint
**Overlap:** `frontend/src/pages/organization-profile/tabs/intelligence/WorkflowsView.tsx`
- Our change: import `workflowKeys`, replace 3 inline `queryKey: ['workflowDefinitions']` with factory
- PR #44 change: major refactor replacing card list with `WorkflowCardGrid`, adding state, removing ~80 lines
- **Resolution:** After PR #44 merges, re-apply our `workflowKeys` import + update PR #44's remaining raw `['workflowDefinitions']` strings to use factory. ~5 min effort.
- **Reviewed:** QA comment posted on PR #44 (2026-03-17). Approve with nits — no blocking issues.

## PR #43 Review Feedback (2026-03-17)
Addressed all items from @KingBodhi's review in commit `eac4b1f`:
- Removed unused `Project` import from topsi/mod.rs
- Updated stale billing TODO comment
- Removed unused `NoraModePreset` re-export
- Fixed `NoraModeSummary` pub(crate)→pub visibility (leaked through handler)
- Server warnings: 25 → 22 (-3 from our PR)

## Verification

```bash
# Frontend
cd frontend && npx tsc --noEmit

# Rust
flox activate -- cargo check --workspace

# Metrics
grep -rn "fetch(" frontend/src/pages/topsi.tsx frontend/src/pages/topiclips.tsx | wc -l  # should be 0
grep -rn "queryKey: \['" frontend/src/ | wc -l  # target ~260
wc -l crates/server/src/routes/twilio.rs  # should not exist (now twilio/)
wc -l crates/server/src/routes/nora/mod.rs  # target ~500
```
