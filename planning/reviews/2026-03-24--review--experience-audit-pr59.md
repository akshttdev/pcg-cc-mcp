# Experience Audit: PR #59 Pipeline Features

**Date**: 2026-03-24
**Branch**: `pr/59-pipeline-e2e-agent-autoadvance`
**Auditor**: Claude (automated via Playwright MCP)
**Servers**: Frontend :3000, Backend :3002 (ENABLE_AGENT_FLOW_ENGINE=1 SIMULATE_LLM=1)
**Console errors**: 0 throughout entire audit

---

## Journey 1: Open and resize deal drawer

| Step | Action | Result | Status |
|------|--------|--------|--------|
| 1 | Navigate to CRM > Pipeline via sidebar | Pipeline view loads with Acquisition pipeline, 2 deals, $30k total | PASS |
| 2 | Click "Auto-Advance Test" deal card | Drawer opens on right side | PASS |
| 3 | Verify `deal-detail-panel` testid | Present on the panel element | PASS |
| 4 | Drag left edge resize handle to the left | Panel width changed from 559px to 706px (delta 147px) | PASS |
| 5 | Verify width persists after release | Width stayed at 706px | PASS |
| 6 | Close drawer, reopen deal | Width persisted at 706px across close/reopen | PASS |
| 7 | Click Close (X) button | Drawer closes, panel removed from DOM | PASS |
| 8 | Resize constraints: drag to narrow extreme | Min width enforced at ~404px | PASS |
| 9 | Resize constraints: drag to wide extreme | Max width enforced at ~1138px | PASS |

**Observations**:
- Resize handle element lacks a `data-testid` attribute (uses CSS class `cursor-col-resize`). Recommend adding `data-testid="deal-panel-resize-handle"` for E2E test stability.
- Resize is smooth with proper min/max constraints preventing degenerate panel sizes.

---

## Journey 2: View agent activity on deal

| Step | Action | Result | Status |
|------|--------|--------|--------|
| 1 | Observe pipeline stage headers | Each stage shows agent assignment: "Scout -- AI agent", "Astra -- AI agent", "Cash -- AI agent", "Lux -- AI agent", "Sales Rep -- Human", etc. | PASS |
| 2 | Check deal cards for agent status badges | No active agent badges visible on current deal cards (correct: no deals have active/failed/cancelled agent flows) | PASS (N/A) |
| 3 | Open "Auto-Advance Test" deal, click Agent History tab | Tab opens, shows "No agent activity yet" empty state | PASS |
| 4 | Open "Expand Test Deal", click Agent History tab | Shows "Agent Execution History" heading with 6 completed flows | PASS |
| 5 | Verify agent flows listed with name and status | Lux (Completed, 2s), Cash (Completed, 2s), Astra (Completed, 2s), Scout (Completed, 2s), Astra (Completed, 2s), Scout (Completed, 2s) | PASS |
| 6 | Expand a flow card (Lux) | Shows timeline: "phase started", "artifact created", "flow completed" with timestamps | PASS |

**Observations**:
- Agent status badges on deal cards (pending, failed, cancelled) are implemented in code (`CrmDealCard.tsx` lines 225-228) but cannot be visually verified because no deals currently have those statuses. The code correctly checks `deal.active_agent_flow_status` for `executing`, `planning`, `failed`, `cancelled`.
- The "Retry Agent" button in Agent History tab is also implemented (line 112-124 of `AgentHistoryTab.tsx`) but only appears when `hasFailedOrCancelled && !hasActiveFlow`. Since all flows are "Completed", it correctly does not show.
- Empty state message is clear and helpful: "Agent flows will appear here when triggered by stage transitions."

---

## Journey 3: Expand deal to fullscreen

| Step | Action | Result | Status |
|------|--------|--------|--------|
| 1 | Open "Expand Test Deal" drawer | Drawer opens with deal detail | PASS |
| 2 | Click Expand button in header | Full-screen dialog opens | PASS |
| 3 | Verify `deal-detail-expanded` testid | Present on the expanded dialog container | PASS |
| 4 | Verify content renders in expanded mode | Full deal content with all tabs (Overview, Intel, Review, Transcripts, Proposal, Deck & Close, Projects, Activity, Agent History) and pipeline stepper | PASS |
| 5 | Verify "Expand" button becomes "Minimize" | Button text changed to "Minimize" in expanded view | PASS |
| 6 | Click Close to dismiss | Dialog closes cleanly, returns to pipeline view | PASS |

---

## Feature Summary

| Feature | Implemented | Visually Verified | Notes |
|---------|------------|-------------------|-------|
| Resizable deal drawer | YES | YES | Drag left edge, min ~404px, max ~1138px, persists across close/reopen |
| Deal detail Close button | YES | YES | X button in header, removes panel from DOM |
| Agent status badges on deal cards | YES (code) | NO (no test data) | Code handles executing/planning/failed/cancelled states; needs deals with those statuses to visually verify |
| Agent History tab | YES | YES | Shows "Agent Execution History" heading, flow cards with agent name/status/duration, expandable event timeline |
| Retry Agent button | YES (code) | NO (no test data) | Shows only when failed/cancelled flows exist and no active flow running |
| Auto-advance | YES (backend) | PARTIAL | "Expand Test Deal" shows it traversed scout->astra->cash->lux (all completed), confirming auto-advance worked; real-time observation not possible in current session |

---

## Findings

### Positive
1. Zero console errors across all interactions
2. Resize behavior is smooth with proper min/max constraints
3. Panel width persists across close/reopen (stored in state/localStorage)
4. Agent History tab has good empty state and populated state UX
5. Flow cards are expandable with event timeline drill-down
6. Expanded dialog renders full deal content correctly
7. testids are properly set: `deal-detail-panel`, `deal-detail-expanded`, `deal-detail-expand`, `deal-detail-close`, `deal-card-{id}`, `deal-menu-{id}`

### Issues / Recommendations
1. **Missing testid on resize handle** -- The resize handle div uses CSS class `cursor-col-resize` but has no `data-testid`. Add `data-testid="deal-panel-resize-handle"` for E2E test reliability. (Low priority)
2. **Agent status badges untestable with current seed data** -- No deals have `active_agent_flow_status` set to `pending`, `failed`, or `cancelled`. Consider adding a seed deal with a failed agent flow to enable visual verification. (Medium priority)
3. **Retry Agent button untestable** -- Same root cause as #2. Need a deal with a failed/cancelled flow to verify the retry button renders and functions. (Medium priority)
4. **Agent History heading inconsistency** -- Empty state says "No agent activity yet" while populated state says "Agent Execution History". This is fine UX but note for E2E tests: the heading text depends on data presence.

---

## Screenshots

- `planning/reviews/screenshots/pr59-pipeline-overview.png` -- Pipeline kanban view
- `planning/reviews/screenshots/pr59-deal-drawer-open.png` -- Deal drawer at default width
- `planning/reviews/screenshots/pr59-deal-drawer-resized.png` -- Deal drawer after resize (wider)
- `planning/reviews/screenshots/pr59-agent-history-empty.png` -- Agent History tab empty state
- `planning/reviews/screenshots/pr59-agent-history-populated.png` -- Agent History tab with completed flows
- `planning/reviews/screenshots/pr59-deal-expanded.png` -- Expanded deal dialog
