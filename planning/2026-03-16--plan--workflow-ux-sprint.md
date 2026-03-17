# 10-Day Sprint: Workflow UX Improvements

**Date**: 2026-03-16
**Status**: **COMPLETE** (implemented 2026-03-17)
**Type**: plan
**Source**: `planning/notes/2026-03-16--review--demo-workflow-ux-audit.md`
**Constraints**: No full wizard (incremental improvements only), automations out of scope, frontend-focused with backend changes where needed, demo script updates as validation criteria.

---

## Context

The workflow system (data extraction pipelines: Data Source → LLM Extract → Output → CRM) is powerful but fragmented. The core problem: **completing one workflow execution requires 5+ page navigations across disconnected URL trees** (Intelligence → Data Source → My Workflows → Staging → CRM). The audit identified 27 UX issues. This sprint targets the highest-ROI subset, focusing on run flow unification, shared components for user/org workflow levels, and editor polish.

---

## Sprint Overview

| Day | Theme | Audit Findings | Shippable Increment |
|-----|-------|----------------|---------------------|
| 1-2 | Run flow unification | P0-4, P0-5, P1-7 | Run + review staging without leaving the page |
| 3 | Node picker + workflow ID | P0-2, P0-3 | Search/filter in node picker, auto-generated workflow IDs |
| 4 | Graph view fix + polish | P0-1, P2-14 | Reliable connection rendering, interactive graph |
| 5-6 | Shared workflow components | P1-10 (user/org split) | Unified workflow list, ownership badges, copy between levels |
| 7 | Editor layout + runs display | P1-9, P1-6 | Better config panel sizing, human-readable run history |
| 8 | Real-time updates | Cross-cutting | Agent watcher polling, staging auto-refresh |
| 9 | UX polish pass | P2-12, P2-13 | Prompt templates, output schema validation, breadcrumbs |
| 10 | Demo script updates + validation | Validation | Updated e2e demos exercising new UX, manual QA |

---

## Day 1: Inline Staging Review in RunWorkflowDialog

**Findings**: P0-4 (run flow fragmentation), P0-5 (staging empty state)

The `RunWorkflowDialog` currently navigates away on success (line 78: `navigate('/workflows?tab=staging&run=...')`). Instead, show staging results inline.

### Implementation

1. **Add `onRunComplete` callback prop to `RunWorkflowDialog`**
   - New prop: `onRunComplete?: (runId: string, stagedRecords: number) => void`
   - When provided, call it instead of navigating away
   - When not provided, keep current navigate behavior (backward compat)

2. **Create `RunAndReviewPanel` composite component**
   - New file: `frontend/src/components/workflows/RunAndReviewPanel.tsx`
   - Composes: run status summary + `StagingReviewContent` (already extracted and reusable)
   - Shows after run completes with staged_records > 0
   - Includes "View in Staging Tab" secondary link for full-page view
   - Includes "Run Another" button to reset and run again

3. **Wire into `BuilderTab`**
   - After successful run, replace the dialog with an expanded `RunAndReviewPanel` below the workflow card grid
   - User stays on `/workflows` tab, reviews and commits inline

4. **Fix `StagingTab` empty state** (`frontend/src/pages/workflows/tabs/StagingTab.tsx`)
   - Replace dead-end empty state with:
     - "Run a Workflow" CTA button (opens `RunWorkflowDialog`)
     - Brief explanation: "Records extracted by workflow runs appear here for review"
     - Link to Builder tab

### Files
- `frontend/src/pages/workflows/components/RunWorkflowDialog.tsx` — add callback prop
- New: `frontend/src/components/workflows/RunAndReviewPanel.tsx`
- `frontend/src/pages/workflows/tabs/BuilderTab.tsx` — wire up inline review
- `frontend/src/pages/workflows/tabs/StagingTab.tsx` — fix empty state

---

## Day 2: Inline Staging on Data Source Detail Page

**Findings**: P0-4 (continued), P1-7 (data source detail too dense)

### Implementation

1. **Extract `DataSourceWorkflowRunner` component** from `data-source-detail.tsx` (730 lines)
   - Extract the workflow execution section into a focused component
   - Props: `dataSourceId`, `organizationId`
   - Manages its own workflow selection, model selection, run state

2. **Simplify the workflow panel header**
   - Current: workflow selector + model selector + Builder link + Run History + Run button all in one row
   - New: workflow selector + Run button as primary row
   - Model selector in collapsible "Options" section
   - Builder and Run History as icon buttons or dropdown menu

3. **Show staging inline after run completes**
   - Reuse `StagingReviewContent` (same as Day 1)
   - Show below the workflow panel, expanding the page
   - Add step indicator: `Select Workflow → Run → Review → Commit`

4. **Add cross-links after commit**
   - "View in CRM" button linking to `/organizations/:orgId/crm/contacts`
   - "Run Again" button to reset

### Files
- `frontend/src/pages/data-source-detail.tsx` — extract workflow section
- New: `frontend/src/components/workflows/DataSourceWorkflowRunner.tsx`

---

## Day 3: Node Picker Search + Workflow ID Auto-Generation

**Findings**: P0-2 (node picker search), P0-3 (workflow ID confusion)

### Implementation

1. **Node picker search/filter** (`NodePicker.tsx`, currently 126 lines)
   - Add `Input` with search icon at top (sticky)
   - Filter `NODE_TYPES` by matching label, description, or type
   - Add proper category headers: "Sources", "Processing", "Actions", "Outputs" with item counts
   - Show "No matches" state when filter returns empty
   - Auto-focus search on picker open

2. **Workflow ID auto-generation** (`editor/index.tsx`)
   - New state: `const [idLocked, setIdLocked] = useState(true)` (default: locked for new workflows)
   - When locked: derive ID from name via `name.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '')`
   - Add lock/unlock toggle icon next to ID field
   - Show ID as muted helper text below name when locked: "ID: my_workflow_name"
   - When unlocked: user can manually edit (current behavior)
   - For existing workflows: ID field stays disabled (already the case, line 457)

### Files
- `frontend/src/components/workflows/editor/NodePicker.tsx`
- `frontend/src/components/workflows/editor/index.tsx`

---

## Day 4: Graph View Connection Fix + Interactive Polish

**Findings**: P0-1 (graph connections not rendering), P2-14 (graph view not editable)

### Investigation First
The SVG connection code in `WorkflowGraphView.tsx` is correct — Bezier curves with arrow markers. The `connections` array data likely has mismatched node IDs or is empty. Steps:
1. Add `console.warn` when `connections` references non-existent node IDs (line 87)
2. Trace data from API response → `WorkflowDefinition` type → graph props
3. Check if `WorkflowConnection.source`/`.target` field names match between API and frontend types

### Implementation

1. **Fix connection data flow** — likely a serialization/deserialization mismatch
   - Check `parse_workflow_from_row()` in `data_source_workflows.rs` (line 478) — it parses `steps_json` which contains `{nodes, connections}`
   - Verify frontend `WorkflowConnection` type matches backend struct

2. **Add interactive features to graph view**
   - Click node in graph → open its config panel (currently switches back to list view via `onSelectNode`)
   - Change `onSelectNode` handler in `editor/index.tsx` to select node without toggling view
   - Add selected node highlight (thicker border or glow effect)
   - Add hover state showing full node name + connection count

3. **Layout improvements**
   - Add zoom +/- buttons (SVG transform scale)
   - Handle single-node workflows (currently may have odd spacing)
   - Add "fit to view" button

### Files
- `frontend/src/components/workflows/editor/WorkflowGraphView.tsx`
- `frontend/src/components/workflows/editor/index.tsx` — fix `onSelectNode` handler
- `frontend/src/lib/api.ts` — verify `WorkflowConnection` type
- Possibly: `crates/server/src/routes/data_source_workflows.rs` — if backend serialization issue

---

## Day 5: Shared Workflow Components + Ownership Model

**Findings**: P1-10 (navigation jumps between org Intelligence and My Workflows)

Backend already supports `owner_type` ("organization", "user", "system") and `owner_id` on workflow definitions. The `POST /api/workflows/definitions` endpoint accepts both fields.

### Implementation

1. **Extract shared `WorkflowCardGrid` component**
   - Currently duplicated between `BuilderTab` (lines 116-206) and Intelligence `WorkflowsView`
   - New file: `frontend/src/components/workflows/WorkflowCardGrid.tsx`
   - Props: `workflows`, `onEdit`, `onRun`, `onDelete`, `showOwnerBadge?`
   - Each card shows: name, description, node badges, last run status, owner badge

2. **Add ownership filter to `BuilderTab`**
   - Filter toggle: "My Workflows" / "Organization" / "All"
   - Default: "All" (current behavior) — but show owner badge on each card
   - Use `owner_type` field from API response

3. **Add ownership badge component**
   - "Personal" (blue) for `owner_type: "user"`
   - Org name (green) for `owner_type: "organization"`
   - "System" (gray) for `owner_type: "system"` (already exists)

4. **Wire Intelligence WorkflowsView to use shared component**
   - Replace its custom workflow card rendering with `WorkflowCardGrid`
   - Add "Run" button (currently missing — only Edit and Delete)

### Files
- New: `frontend/src/components/workflows/WorkflowCardGrid.tsx`
- `frontend/src/pages/workflows/tabs/BuilderTab.tsx` — use shared component
- `frontend/src/pages/organization-profile/tabs/intelligence/WorkflowsView.tsx` — use shared component + add Run
- `frontend/src/lib/api.ts` — verify `WorkflowDefinition` includes `owner_type`/`owner_id`

---

## Day 6: Copy/Promote Workflows Between Levels

**Findings**: User requirement for tools to move workflows between user and org levels

### Implementation

1. **Add actions dropdown to workflow cards** (in `WorkflowCardGrid`)
   - "Copy to Organization..." — opens org selector dialog, creates duplicate with `owner_type: "organization"`
   - "Copy to My Workflows" — creates duplicate with `owner_type: "user"`
   - Only show relevant action (don't show "Copy to Org" on org-owned workflows)
   - New copy gets name suffix: " (Copy)"

2. **Create `CopyWorkflowDialog`**
   - For user→org: show org selector (user's organizations list)
   - For org→user: just confirm
   - Calls `workflowsApi.createDefinition()` with same nodes/connections, new owner

3. **Handle duplicate IDs**
   - Backend requires unique workflow IDs
   - Auto-append `_copy` or `_copy_2` suffix to ID
   - Show the new ID in the confirmation dialog

### Files
- New: `frontend/src/components/workflows/CopyWorkflowDialog.tsx`
- `frontend/src/components/workflows/WorkflowCardGrid.tsx` — add actions dropdown

---

## Day 7: Editor Layout + Runs Display

**Findings**: P1-9 (node config panel too wide), P1-6 (runs show raw UUIDs)

### Implementation

1. **Node config panel width constraints** (`editor/index.tsx`)
   - Add `max-w-[480px]` to right panel (currently unbounded flex-1)
   - Default left/right split closer to 50/50 instead of 35/65
   - Left panel `min-w-[320px]` (already `leftPanelWidth` state with default 340)

2. **Runs tab deleted data source display** (`WorkflowRunsPanel.tsx`)
   - Current fallback: `id.slice(0, 8) + '...'` (line 107)
   - Change to: `"(Deleted source)"` with muted styling
   - Add title attribute with full UUID for debugging

3. **RunsTab same fix** (`frontend/src/pages/workflows/tabs/RunsTab.tsx`)
   - Same batch-resolution pattern — apply same "(Deleted source)" treatment

### Files
- `frontend/src/components/workflows/editor/index.tsx` — panel width constraints
- `frontend/src/components/workflows/WorkflowRunsPanel.tsx` — deleted source display
- `frontend/src/pages/workflows/tabs/RunsTab.tsx` — same fix

---

## Day 8: Real-Time Updates

**Findings**: Cross-cutting (agent watcher static, staging no auto-refresh)

### Implementation

1. **Agent watcher polling** (`AgentWatcherPanel.tsx`)
   - Currently: single `loadWatchers()` call on mount, no refresh
   - Fix: convert to React Query with `refetchInterval: 10_000` (10s polling)
   - Add "Last updated: Xs ago" indicator
   - Add pulse animation on status change (compare previous vs new data)

2. **Staging auto-refresh** (`StagingReviewPanel.tsx`)
   - `StagingReviewContent` has no auto-refresh currently
   - Add `refetchInterval: 10_000` to the staging records query
   - Show "N new records" banner when count changes between fetches
   - Don't auto-refresh if user is actively editing a record (check edit state)

3. **Workflow run completion notification**
   - After `runMutation` succeeds in `RunWorkflowDialog`, the staging data should be immediately available
   - Add `queryClient.invalidateQueries({ queryKey: ['staging', runId] })` on run completion
   - This ensures the inline staging review (Days 1-2) shows fresh data

### Files
- `frontend/src/components/AgentWatcherPanel.tsx` — convert to polling
- `frontend/src/components/workflows/StagingReviewPanel.tsx` — add refetchInterval
- `frontend/src/pages/workflows/components/RunWorkflowDialog.tsx` — invalidate staging queries

---

## Day 9: UX Polish Pass

**Findings**: P2-12 (prompt template), P2-13 (output schema), navigation breadcrumbs

### Implementation

1. **Prompt template improvements** (`NodeConfigPanel.tsx`)
   - Replace long placeholder (7-line example) with short one: "Describe what to extract (e.g., 'Extract all contacts with name, email, and company')"
   - Add "Templates" dropdown with 3 starters: Contact Extraction, Company Research, Deal Pipeline
   - Selecting a template fills the textarea

2. **Output schema validation** (same file)
   - Replace freeform `Input` with a `Select` dropdown of known target schemas: `contacts[]`, `companies[]`, `deals[]`, `tasks[]`
   - Allow custom entry via "Custom..." option that reveals the text input
   - Show inline validation error for invalid format

3. **Add workflow breadcrumbs**
   - New: `frontend/src/components/workflows/WorkflowBreadcrumb.tsx`
   - Shows: `Org > Intelligence > Data Sources > [Source] > Run > Review`
   - Contextual: only shows steps relevant to current location
   - Clickable segments for navigation

4. **Sidebar workflow sub-links**
   - Under "My Workflows" in sidebar, add sub-items: Builder, Runs, Staging (with pending count badge)
   - Uses `?tab=builder`, `?tab=runs`, `?tab=staging` query params

### Files
- `frontend/src/components/workflows/editor/NodeConfigPanel.tsx` — prompt + schema UX
- New: `frontend/src/components/workflows/WorkflowBreadcrumb.tsx`
- `frontend/src/components/layout/sidebar/OrgWorkspaceLinks.tsx` — workflow sub-links

---

## Day 10: Demo Script Updates + Validation

**Findings**: Validation criteria

### Implementation

1. **Update existing demo specs** in `e2e/demos/`
   - `workflow-crm-pipeline.spec.ts` — update for inline staging review (no page navigation after run)
   - `workflow-spanish-pipeline.spec.ts` — same flow updates
   - `pipeline-intelligence-workflow.spec.ts` — update for shared components, ownership badges
   - `bug-report-lifecycle.spec.ts` — verify agent watcher updates without reload
   - `notification-center.spec.ts` — no major changes expected
   - `manual-qa-trigger.spec.ts` — verify watcher polling

2. **Manual QA checklist**
   - [ ] Create workflow in Builder: ID auto-generates from name
   - [ ] Node picker: search filters nodes correctly
   - [ ] Graph view: connections render between linked nodes
   - [ ] Run workflow from Builder: staging review shows inline (no navigation)
   - [ ] Run workflow from Data Source detail: staging review shows inline
   - [ ] Copy workflow from Personal to Org: appears in Intelligence
   - [ ] Runs tab: deleted data sources show "(Deleted source)" not UUIDs
   - [ ] Agent watcher: status updates without page reload
   - [ ] Staging: auto-refreshes when new records arrive
   - [ ] Full flow: Select source → Run → Review → Commit in ≤ 3 page contexts

3. **Bug fix buffer** — afternoon reserved for issues found during validation

### Files
- `e2e/demos/workflow-crm-pipeline.spec.ts`
- `e2e/demos/workflow-spanish-pipeline.spec.ts`
- `e2e/demos/pipeline-intelligence-workflow.spec.ts`
- `e2e/demos/bug-report-lifecycle.spec.ts`
- `e2e/demos/manual-qa-trigger.spec.ts`

---

## Dependency Graph

```
Day 1 (Inline staging in Builder) ──> Day 2 (Inline staging in DS detail)
                                              │
Day 3 (Node picker + ID) ─────────────────────┤
                                              │
Day 4 (Graph view fix) ───────────────────────┤
                                              │
Day 5 (Shared components) ──> Day 6 (Copy/promote) ──> Day 9 (Polish)
                                              │              │
Day 7 (Layout + runs) ──> Day 8 (Real-time) ──┘              │
                                                              v
                                                        Day 10 (Validation)
```

- Days 1-2 are sequential (Day 2 reuses `RunAndReviewPanel` from Day 1)
- Days 3, 4, 7 are independent of each other — can be reordered
- Days 5-6 are sequential (Day 6 uses `WorkflowCardGrid` from Day 5)
- Day 8 depends on Days 1-2 (needs inline staging to add auto-refresh)
- Day 10 depends on all prior days

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Graph connections bug is backend serialization | Day 4 expands to include Rust changes | Check API response first thing Day 4; `parse_workflow_from_row()` already handles connections |
| `DataSourceDetailPage` is 730 lines, extraction risk | Day 2 regression | Extract incrementally, keep existing behavior for unchanged paths |
| `StagingReviewContent` props don't support all inline use cases | Days 1-2 blocked | It's already used in both dialog and tab modes — low risk |
| Demo specs rely on navigation patterns that change | Day 10 scope grows | Reserve full afternoon for demo fixes |
| Node picker search adds height to constrained popup | Day 3 layout issues | Make search bar sticky, picker body scrollable |

---

## Success Criteria

1. **Navigation reduction**: Workflow execution completes in ≤ 3 page contexts (down from 5+)
2. **Graph view**: Connection lines visible between connected nodes
3. **Node picker**: Can find any node type by typing 2-3 characters
4. **Workflow ID**: Users never manually type a workflow ID
5. **Shared components**: Same workflow card grid used in both My Workflows and Intelligence
6. **Real-time**: Agent watcher updates within 10 seconds, staging shows new records without manual refresh
7. **All 6 demo specs pass** with updated selectors and flows

---

## Implementation Notes (2026-03-17)

### Deviations from Plan

| Day | Planned | Actual | Reason |
|-----|---------|--------|--------|
| 2 | Extract `DataSourceWorkflowRunner` component | Wired `RunAndReviewPanel` inline without full extraction | Lower risk — 730-line file extraction deferred to avoid regressions |
| 2 | Step indicator + cross-links | Not added | `RunAndReviewPanel` already provides Run Another + Full View + Close — sufficient |
| 5 | Ownership filter toggle (My/Org/All) | Ownership badges shown, no filter toggle | Cards show badges — filtering adds state complexity without clear UX value yet |
| 8 | "Last updated: Xs ago" indicator, pulse animation | Not added | Polling works silently; indicator adds noise for 10s intervals |
| 8 | "N new records" banner | Not added | Auto-refresh with `refetchInterval` is sufficient; banner adds complexity |
| 9 | `WorkflowBreadcrumb.tsx` component | Not created | Sidebar sub-links (Builder/Runs/Staging) solve the same navigation problem more naturally |
| 9 | Sidebar sub-links in `OrgWorkspaceLinks.tsx` | Added to `Sidebar.tsx` instead | "My Workflows" lives in the primary nav section in `Sidebar.tsx`, not in org workspace links |

### New Files Created
- `frontend/src/components/workflows/RunAndReviewPanel.tsx` — inline staging review composite
- `frontend/src/components/workflows/WorkflowCardGrid.tsx` — shared card grid with ownership badges
- `frontend/src/components/workflows/CopyWorkflowDialog.tsx` — copy/promote workflows between levels

### Files Modified (20 files total)
- `frontend/src/pages/workflows/components/RunWorkflowDialog.tsx` — `onRunComplete` callback, staging cache invalidation
- `frontend/src/pages/workflows/tabs/BuilderTab.tsx` — uses `WorkflowCardGrid`, inline review, copy dialog
- `frontend/src/pages/workflows/tabs/StagingTab.tsx` — actionable empty state with "Go to Builder" CTA
- `frontend/src/pages/workflows/tabs/RunsTab.tsx` — "(Deleted source)" display
- `frontend/src/pages/data-source-detail.tsx` — inline `RunAndReviewPanel`, removed unused staging query
- `frontend/src/pages/organization-profile/tabs/intelligence/WorkflowsView.tsx` — uses `WorkflowCardGrid`, Run button added
- `frontend/src/components/workflows/editor/NodePicker.tsx` — search/filter, categories, counts
- `frontend/src/components/workflows/editor/index.tsx` — ID auto-generation, graph view selection fix, panel width cap
- `frontend/src/components/workflows/editor/WorkflowGraphView.tsx` — zoom controls, node selection, connection highlighting
- `frontend/src/components/workflows/editor/NodeConfigPanel.tsx` — prompt templates, output schema Select
- `frontend/src/components/workflows/WorkflowRunsPanel.tsx` — "(Deleted source)" display
- `frontend/src/components/workflows/StagingReviewPanel.tsx` — `refetchInterval` (paused during editing)
- `frontend/src/components/tasks/AgentWatcherPanel.tsx` — converted to React Query with 10s polling
- `frontend/src/components/layout/sidebar/Sidebar.tsx` — workflow sub-links (Builder/Runs/Staging)
- `e2e/demos/workflow-crm-pipeline.spec.ts` — inline staging flow
- `e2e/demos/workflow-spanish-pipeline.spec.ts` — inline staging flow
- `e2e/demos/pipeline-intelligence-workflow.spec.ts` — shared component docs

### Conflict Resolution Plan with PR #43 (Modularity Sprint 5)

**Analysis date**: 2026-03-17
**PR #43 branch**: `modularity/sprint-5` — infrastructure extraction + route splits
**Method**: `git merge-tree --write-tree` between both branches

| File | Conflict Type | Resolution Strategy |
|------|--------------|---------------------|
| `frontend/src/pages/data-source-detail.tsx` | Content conflict (imports + query keys) | **Take both**: PR #43 migrates `['dataSource', id]` → `dataSourceKeys.detail(id)`. Our branch removes `AlertTriangle`, `stagingApi`, `useMemo` and adds `RunAndReviewPanel`. Changes are adjacent lines — merge both sets. |
| `frontend/src/pages/organization-profile/tabs/intelligence/WorkflowsView.tsx` | Auto-merges cleanly | No action needed — our `WorkflowCardGrid` rewrite already uses `workflowKeys.definitions()` which PR #43 introduces. |

**All other files**: No conflicts. PR #43's Rust route splits (twilio, task_attempts, nora, helpers) and frontend API modules (topsi.ts, topiclips.ts) are in completely different files.

**Merge order recommendation**: Either order works. If PR #43 merges first, rebase our branch and resolve the single `data-source-detail.tsx` conflict (~2 min).
