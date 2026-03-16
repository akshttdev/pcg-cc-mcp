# Modularity Sprint 2 — Week of 2026-03-15

## Context

Following the tech debt sprint (PR #34) that split `api.ts` (6,669→21 modules) and `organization-profile.tsx` (7,086→33 files), this sprint continues modularity improvements across the full stack. Focus: large file splits, DRY helper centralization, and pattern extraction.

**Branch:** `modularity/sprint-2026-03-15` from `main` @ `e7b4689a6`
**Working directory:** `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`

### PR #36 Impact Assessment (pre-sprint review)

PR #36 (`e7b4689a6`) landed after initial planning. Impact:

- **Base commit updated** from `616ac8d0b` → `e7b4689a6`
- **Day 1a**: `formatDate` duplication now **16 files** (was 12+). `formatCurrency` added to `CrmPipelineBoard.tsx` — additional consumer to migrate.
- **Day 1c**: `CrmDealDetailPanel` raw `fetch()` calls already replaced with `intelligenceApi.triggerResearch()` in PR #36 review items. Remove from Day 1c scope.
- **New split candidate**: `CrmDealDetailPanel.tsx` grew to **1,202 lines** — add to Day 4 or stretch.
- **New split candidate**: `crm_deals.rs` grew to **964 lines** — monitor, not yet critical.
- **Days 2-5 line counts unchanged** — all targets confirmed at planned sizes.

---

## Sprint Scope (5 days)

### Day 1: Shared Helpers + EmptyState Adoption (Foundation)

**Goal:** Centralize duplicated utilities and adopt existing-but-unused EmptyState component. Foundation for Days 2-5.

**1a. Create `frontend/src/lib/formatters.ts`** — centralized formatting
- `formatDate` (duplicated in 12+ files)
- `formatCurrency` (duplicated in 4+ files)
- `formatCompactNumber` / `fmtFollowers` (duplicated in 5+ files)
- `parseJsonArray` (duplicated in 3+ files)
- Update all consumers to `import from '@/lib/formatters'`
- Org-profile `helpers.ts` re-exports from `@/lib/formatters` for backward compat
- Remove local definitions from: ArtifactCard, ArtifactPreviewPanel, TaskArtifactsPanel, ProcessesTab, MeetingHistory, ApprovalCard, SocialAccountConnect, EmailInbox, CrmPipelineBoard, CrmPipelineMetrics, crm-contact-detail, PulseSection, social sub-views

**1b. `EmptyState` component adoption** — already exists at `frontend/src/components/ui/empty-state.tsx` with zero imports
- Supports: `icon`, `title`, `description`, `action`, `secondaryAction`
- Replace 10-15 highest-traffic inline empty state patterns
- Target files being split on Days 3-5 first (workflows.tsx, project-detail.tsx, etc.)

**1c. Consolidate top 10 raw `fetch()` calls** into API client
- ~~CrmDealDetailPanel~~ (already fixed in PR #36), NoraAssistant, MeetingMode, MeshPanel, EnhancedTaskDetailsPanel
- Create API methods in appropriate domain modules for missing endpoints

**Verification:** `npx tsc --noEmit`, grep confirms no duplicate `formatDate` definitions

---

### Day 2: nora/tools.rs Split (7,347 lines → 8-10 modules)

**Goal:** Split the single largest file in the codebase into domain-specific modules.

**Target structure:**
```
crates/nora/src/tools/
├── mod.rs              # ExecutiveTools struct, new(), execute_tool(), get_tools_by_category() (~500 lines)
├── types.rs            # ToolCategory, ToolParameter, ParameterType, Permission, enums (~300 lines)
├── executive_tool.rs   # NoraExecutiveTool enum definition (50+ variants, ~430 lines)
├── schemas.rs          # get_openai_tool_schemas() JSON schema definitions (~1,300 lines)
├── parse.rs            # parse_tool_call() match arms (~500 lines)
├── definitions.rs      # initialize_tools() ToolDefinition registrations (~650 lines)
├── execute.rs          # execute_tool_implementation() dispatch (~3,800 lines)
│                       # Sub-split if >1,000: execute_project.rs, execute_media.rs, execute_comms.rs
├── user_scoped.rs      # execute_user_scoped_tool() (~300 lines)
└── tests.rs            # #[cfg(test)] block (~150 lines)
```

**Natural groupings** (from 127 `NoraExecutiveTool::` match arms):
- Project Management: ~13 variants
- Media Production: ~12 variants
- Communication: ~6 variants
- Coordination/Planning: ~9 variants
- File/Web/Code: ~10 variants
- Analytics/Finance/HR: ~9 variants

**Approach:** `mod.rs` keeps `ExecutiveTools` struct; each module exports tool functions; `pub use` re-exports from `mod.rs` for zero-change external API.

**Verification:** `cargo check -p nora`, `cargo test -p nora`, `cargo clippy -p nora`

---

### Day 3: workflows.tsx (2,418 lines) + WorkflowEditor.tsx (2,240 lines)

**Goal:** Split the 2 largest workflow frontend files.

**3a. `pages/workflows.tsx` → `pages/workflows/` directory**

Internal components (with line ranges):
- `WorkflowsPage` (130-735) → `index.tsx` shell (~200 lines)
- `WorkflowBuilderTab` (736-936) → `tabs/BuilderTab.tsx`
- `WorkflowDetailPanel` (937-1199) → `components/WorkflowDetailPanel.tsx`
- `RunWorkflowDialog` (1200-1396) → `components/RunWorkflowDialog.tsx`
- `InlineEditField` (1397-1517) → `components/InlineEditField.tsx`
- `StagingTab` (1518-2310) → `tabs/StagingTab.tsx` (may need sub-split if >500)
- `RunsTab` (2311-2418) → `tabs/RunsTab.tsx`

```
pages/workflows/
├── index.tsx                    # Shell + tab routing
├── types.ts                     # Shared interfaces
├── constants.ts                 # VALID_GLOBAL_FILTERS, etc.
├── tabs/
│   ├── BuilderTab.tsx
│   ├── StagingTab.tsx
│   └── RunsTab.tsx
└── components/
    ├── WorkflowDetailPanel.tsx
    ├── RunWorkflowDialog.tsx
    └── InlineEditField.tsx
```

**3b. `components/workflows/WorkflowEditor.tsx` → `components/workflows/editor/` directory**

Internal components:
- `WorkflowEditor` (311-1074) → `index.tsx`
- `WorkflowGraphView` (1075-1234) → `WorkflowGraphView.tsx`
- `NodePicker` (1235-1360) → `NodePicker.tsx`
- `NodeConfigPanel` (1361-2042) → `NodeConfigPanel.tsx` (~680 lines, may sub-split)
- `OutputNodeConfig` (2043-2203) → `OutputNodeConfig.tsx`
- `DataSourceNodeConfig` (2204-2240) → `DataSourceNodeConfig.tsx`
- `NODE_TYPE_DEFS` (220-310) → `node-types.ts`

Barrel re-export from `WorkflowEditor.tsx` → `editor/index.tsx` for backward compat.

**Verification:** `npx tsc --noEmit`, `/workflows` route loads, editor dialog opens

---

### Day 4: project-detail.tsx (2,293 lines) + task_server.rs (3,457 lines)

**Goal:** Split the next-largest files on each side of the stack.

**4a. `components/projects/project-detail.tsx` → `project-detail/` directory**

Component has ~20 state variables, many fetch calls, long scrollable page:
- `index.tsx` — orchestration + layout (~400 lines)
- `hooks/useProjectData.ts` — extract useState/useEffect data loading
- `sections/ProjectHeroCard.tsx` — brand hero card (~200 lines)
- `sections/ProjectStatsPanel.tsx` — stats sidebar
- `sections/IntegrationsSection.tsx` — email/social/airtable status (~200 lines)
- `sections/BoardsSection.tsx` — board listing + creation
- `helpers.ts` — `hexToRgba`, brand profile logic
- `types.ts` — `ProjectDetailProps`, `ProviderStatus`, etc.

**4b. `crates/server/src/mcp/task_server.rs` (3,457 lines) → `task_server/` directory**

MCP server with 20+ tool handler methods:
```
crates/server/src/mcp/task_server/
├── mod.rs          # TaskServer struct, new(), accessible_project_ids() (~100 lines)
├── helpers.rs      # parse_task_status, priority_to_string, TaskSummary, etc. (~250 lines)
├── task_ops.rs     # create/list/update/delete/get/assign/bulk tasks (~1,100 lines)
├── project_ops.rs  # list_projects, scaffold_project, list_members (~400 lines)
├── knowledge.rs    # add/list/get knowledge, completeness (~400 lines)
├── topology.rs     # get_topology, issues, find_path (~500 lines)
├── deps.rs         # manage_task_dependencies, check_dependencies (~350 lines)
└── policy.rs       # evaluate_policy, add_comment, unified_search (~350 lines)
```

**Verification:** `npx tsc --noEmit`, `cargo check -p server`, `cargo test -p server`

---

### Day 5: Hook Extraction + brand-guide.tsx + Final Sweep

**Goal:** Extract shared query hooks, split one more large page, final DRY cleanup.

**5a. Extract shared query hooks** → `frontend/src/hooks/queries/`
- `useOrganization(orgId)` — used in 5+ components
- `useProjectList(orgId)` — used in 3+ components
- `useCrmContacts(orgId)` — used in 3+ components
- `useOrgMembers(orgId)` — used in 3+ components

**5b. Split `pages/brand-guide.tsx` (1,703 lines) → `pages/brand-guide/` directory**
- `index.tsx` — page shell with tabs
- Extract each major section (Visual Identity, Positioning, Audience, Content, Competitive) into sub-components
- Extract local helper functions

**5c. Split oversized hooks (stretch)**
- `useConversationHistory.ts` (540 lines) → extract `flattenEntries`, `executionHelpers`, `patchWithKey` into separate files
- `useAutonomy.ts` (478 lines) → extract `useCheckpoints`, `useApprovalGates`

**5d. Final DRY sweep**
- Grep for remaining `formatDate`/`formatCurrency` local definitions
- Verify zero duplicates remain
- Audit: `find frontend/src -name "*.tsx" | xargs wc -l | sort -rn | head -20` — no split target over 500 lines

**Verification:** `npx tsc --noEmit`, `cargo check`, e2e health checks 40/40

---

## Sprint Summary

| Day | Target | Lines Before | Max File After | New Files |
|-----|--------|-------------|----------------|-----------|
| 1 | Shared helpers + EmptyState | N/A (DRY) | `formatters.ts` ~80 | 1 new + ~27 modified |
| 2 | `nora/tools.rs` | 7,347 | ~800 | 8-10 |
| 3 | `workflows.tsx` + `WorkflowEditor.tsx` | 2,418 + 2,240 | ~400 each | 12-15 |
| 4 | `project-detail.tsx` + `task_server.rs` | 2,293 + 3,457 | ~400 + ~250 | 12-16 |
| 5 | Hooks + `brand-guide.tsx` + sweep | 540 + 478 + 1,703 | ~200-300 each | 10-14 |

**Total:** ~20,500 lines reorganized, 43-55 new module files, zero files over 800 lines in split outputs.

## Key Considerations

- **Branch:** `git checkout -b modularity/sprint-2026-03-15 main`
- **Each day is independently shippable** — can PR after any day
- **Day 1 is foundational** — shared helpers consumed by Day 3-5 splits
- **Backward compatibility:** Barrel re-exports where external consumers exist
- **Follow established patterns:** Same split approach as api.ts and org-profile (PR #34)

## Verification (End of Sprint) — ALL PASSED

1. `npx tsc --noEmit` — 0 TS errors
2. `cargo check` — compiles clean (only pre-existing warnings)
3. E2E health checks — **40/40 passed**
4. Zero duplicate `formatDate`/`formatCurrency` definitions confirmed
5. `EmptyState` component used in 7 files (13 inline patterns replaced)
6. 10 raw `fetch()` consumers consolidated into API client modules
7. Split targets no longer in top 25 largest files

## QA Review (2026-03-16)

Three agents reviewed all 5 days for dropped code, lost functionality, and missing comments.

**Regression found and fixed:**
- `discord.tsx`: `formatDate` replaced a local version that included time (hour:minute). Voice session timestamps lost time display. Fixed → `formatDateTime`. Commit `50e4c5278`.
- `call-intake.tsx`: Local `fmt()` not using centralized `formatDateTime`. Fixed in same commit.

**Incomplete (not regressions):**
- `formatRelativeDate` not centralized — 3 local implementations remain (EmailInbox, CommunicationsInbox, WorkflowRunsPanel) with different behaviors. These were correctly renamed from `formatDate` to `formatRelativeDate` to avoid shadowing, but not consolidated since they have genuinely different logic.

**Clean (no issues):**
- Day 2: All 364 NoraExecutiveTool enum variants, all methods, all types preserved
- Day 3: All 7 workflow components + 7 editor components migrated, router and barrel imports verified
- Day 4: All 5 project-detail sections + 3 dialogs, all 26 task_server tool methods preserved
- Day 5: All 11 brand-guide sections + wizard, 3 shared hooks with backward compat re-exports

## Completion Status — DONE (2026-03-16)

**PR:** #37 — `modularity/sprint-2026-03-15`

| Day | Commit | Files Changed | Lines +/- |
|-----|--------|--------------|-----------|
| 1 | `b9a3fe450` | 50 | +635 / -429 |
| 2 | `b5c8ea922` | 8 new | +7,404 |
| 3 | `109c91518` | 18 | +4,824 / -9,587 |
| 4 | `b45732753` | 21 | +6,231 / -8,164 |
| 5 | `bf7550ffa` | 30 | +1,925 / -1,763 |
| QA | `50e4c5278` | 2 | +5 / -8 |
| W1 | `80ed60c11` | 2 | +31 / -10 |

**New module files created:** ~60
**Stretch items deferred:** useConversationHistory.ts split, useAutonomy.ts split (below priority threshold)

## Remaining Large Files (Future Sprints)

After this sprint, the top frontend files by size are:
- `virtual-environment.tsx` (1,282 lines) — not a high-traffic page
- `TaskFormDialog.tsx` (1,240 lines) — complex form, would benefit from split
- `company-profile.tsx` (1,218 lines) — similar to project-detail split
- `MeetingMode.tsx` (1,207 lines) — Topsi meeting component
- `CrmDealDetailPanel.tsx` (1,202 lines) — grew in PR #36, split candidate

## Critical Files

| Day | File | Lines | Action |
|-----|------|-------|--------|
| 1 | `frontend/src/lib/formatters.ts` | NEW | Centralized formatters |
| 1 | `frontend/src/components/ui/empty-state.tsx` | EXISTS (0 imports) | Adopt in 10-15 files |
| 2 | `crates/nora/src/tools.rs` | 7,347 | Split → `tools/` (8-10 modules) |
| 3 | `frontend/src/pages/workflows.tsx` | 2,418 | Split → `workflows/` directory |
| 3 | `frontend/src/components/workflows/WorkflowEditor.tsx` | 2,240 | Split → `editor/` directory |
| 4 | `frontend/src/components/projects/project-detail.tsx` | 2,293 | Split → `project-detail/` directory |
| 4 | `crates/server/src/mcp/task_server.rs` | 3,457 | Split → `task_server/` directory |
| 5 | `frontend/src/pages/brand-guide.tsx` | 1,703 | Split → `brand-guide/` directory |
| 5 | `frontend/src/hooks/queries/` | NEW | Shared query hooks |
| 5 | `frontend/src/hooks/useConversationHistory.ts` | 540 | Extract sub-modules (stretch) |
