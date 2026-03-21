# Experience Audit Report v2 — Tier 1 Phase 0 Sprint

**Date**: 2026-03-21
**Scope**: 6 journeys from sprint planning file — AI Usage, friction report, agent flow observation, CRM deal stages, access control error UX
**Branch**: `feature/2026-03-19--tier1-phase0`
**Viewport**: Desktop (1440×900)

## Executive Summary

Interactive testing revealed a stark quality gap: **CRM deal creation is excellent** (instant feedback, toast confirmation, data persists) but **deal stage transitions are completely broken** (drag-and-drop fires but doesn't move deals, no alternative UI mechanism exists). The friction report flow works end-to-end with good validation. AI Usage shows all zeros (old API wired). Access control errors show a bare "Organisation not found" message with no recovery path. The new actor classification (USER/AGENT/HYBRID) proved valuable — agent flow observation couldn't be tested due to missing seed data, but the empty state is adequate.

## Findings by Severity

### BLOCKERS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| B1 | J5: CRM Pipeline | **Users cannot move deals between pipeline stages.** Drag-and-drop fires (dnd-kit status shows drop event) but deal stays in original column. Stage bar in deal detail is display-only. Context menu has only Edit/Delete — no "Move to stage". Zero of three possible transition methods work. | `ux-audit-deal-stuck-in-lead.png` | Wire `onDragEnd` handler to backend `move_to_stage` API. Add "Move to..." in context menu as quick alternative. Make stage bar clickable in deal detail. | INVESTMENT |

### PAIN POINTS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| P1 | J5: Deal detail | **Stage progress bar is display-only.** Shows "Lead" with circle indicators for future stages, but clicking does nothing. Standard CRM pattern (HubSpot, Pipedrive) makes this interactive. | — | Make stage indicators clickable with confirmation: "Move to Business Analysis?" | INVESTMENT |
| P2 | J5: Deal context menu | **Context menu missing "Move to stage".** Only Edit and Delete. This is the most discoverable place for stage transitions. | `ux-audit-deal-stuck-in-lead.png` | Add "Move to..." submenu with all pipeline stages listed. Highlight next stage as recommended. | QUICK WIN |
| P3 | J1: AI Usage | **Dashboard shows all zeros.** Time range dropdown works (switches between 7/30/90 days) but all data is $0.00 because page uses old `tokenUsageApi` instead of new `costsApi`. Plan marked this DONE. | `ux-audit-ai-usage.png` | Wire `costsApi` into AI Usage page components. | QUICK WIN |
| P4 | J6: Access control | **"Organisation not found" error is a dead end.** Plain text, no icon, no CTA, no link back to valid orgs. Console shows 6 repeated API errors (client polling failed endpoint). Breadcrumb shows wrong org context. | `ux-audit-org-not-found.png` | Show error with icon + "Go to your organizations" CTA. Stop API retry on 404. Fix breadcrumb to reflect actual org (or hide it). | QUICK WIN |

### FRICTION

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| F1 | J5: Drag-and-drop | **No visual feedback during drag.** Columns don't highlight as valid drop targets. No ghost card preview. Drop silently fails with no error message or explanation. | — | Add column highlight on dragover, ghost card at drop position, error toast if FSM blocks transition. | INVESTMENT |
| F2 | J5: Stage agent labels | **"Scout", "Astra", "Cash", "Lux" shown without explanation.** User doesn't know these are AI agents assigned to each stage. | — | Add tooltips: "Scout — AI research agent for this stage." | QUICK WIN |
| F3 | J4: Agent Flows empty state | **"Historical agent execution flows appear here" — doesn't explain how flows get created.** User has no way to create a flow from this page and no guidance on what triggers one. | — | Add: "Agent flows are created when tasks are assigned to AI agents. Assign a task to get started." with link to task creation. | QUICK WIN |
| F4 | J6: Spelling inconsistency | **"Organisation not found" uses British spelling** but rest of app uses "Organization" (American). Minor but noticeable. | — | Standardize to "Organization" throughout. | QUICK WIN |
| F5 | J6: Console error spam | **6 repeated API errors for non-existent org.** Client retries the failed endpoint multiple times, spamming console. | — | Add retry backoff or stop retrying on 404 responses. | QUICK WIN |
| F6 | J2: Feedback dialog | **Submit button disabled without explanation.** When required fields are empty, the button is disabled but there's no inline message saying "Title is required" — user must figure out which fields to fill. | — | Add inline validation hints on required fields or show a tooltip on the disabled button: "Fill in title and description to submit." | QUICK WIN |

### POLISH

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| PO1 | J1: AI Usage | **Time range dropdown works smoothly.** Switching between Today/7/30/90 days updates the UI instantly with "No usage data for this period." Good empty state per-period. | — | — (positive finding) | — |
| PO2 | J2: Friction report | **Frustration level buttons with contextual labels.** Selecting level 4 shows "High", level 3 shows "Moderate". Excellent micro-interaction. Tips update contextually for friction type. | — | — (positive finding) | — |
| PO3 | J5: CRM Pipeline | **Deal creation is best-in-class.** Instant kanban card, dual toast ("Deal created" + "Deal added to pipeline"), pipeline totals update, "Research needed" contextual status badge, data persists after navigation. | — | Use this as the gold standard template for all create flows. | — |

## Journey Scorecard

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
| J1: View AI spending | USER | 9 | Page loads, time range works, but all data is zeros | 0 | 1 (P3) | 0 | **D** — interactive controls work but data is wrong |
| J2: Report friction | USER | 9 | **FULLY COMPLETE** — filled form, submitted, toast confirmed, dialog closed | 0 | 0 | 1 (F6) | **A** — excellent end-to-end |
| J3: Respond to clarification | HYBRID | 7 | Cannot test — no flows in seed data | 0 | 0 | 1 (F3) | **N/A** — no test data |
| J4: Agent flow progression | AGENT (observe) | 5 | Cannot test — no flows in seed data. Empty state adequate. | 0 | 0 | 1 (F3) | **B** — empty state OK but unexplained |
| J5: Move deal between stages | HYBRID | 9 | **FAILED** — deal creation works perfectly but stage transition impossible via any method | 1 (B1) | 2 (P1, P2) | 2 (F1, F2) | **F** — creation A+, transition impossible |
| J6: Access control error UX | USER (negative) | 4 | Error shown but unhelpful — no recovery path | 0 | 1 (P4) | 2 (F4, F5) | **D** — error is a dead end |

## Quick Wins (do first)

1. **P2** — Add "Move to..." submenu in deal context menu — 2-4 hours — **unblocks the #1 BLOCKER**
2. **P4** — Add icon + "Go to your organizations" CTA on org-not-found error — 1-2 hours
3. **F2** — Add tooltips to agent names (Scout, Astra, etc.) on pipeline stages — 1 hour
4. **F3** — Improve Agent Flows empty state with guidance on how flows get created — 30 min
5. **F4** — Fix "Organisation" → "Organization" spelling — 15 min
6. **F5** — Stop API retry on 404 responses — 1 hour
7. **F6** — Add inline validation hints on feedback dialog required fields — 1-2 hours

## Investments (plan for)

1. **B1** — Wire dnd-kit `onDragEnd` to backend `move_to_stage` API + visual DnD feedback — 2-3 days — **Core CRM workflow is non-functional without this**
2. **P1** — Make stage progress bar clickable in deal detail — 1 day — Standard CRM expectation
3. **P3** — Wire `costsApi` into AI Usage page — 1-2 days — Sprint deliverable marked DONE but frontend never updated

## Structural Recommendations

1. **Three complementary stage transition methods** — Drag on kanban + click stage in detail + context menu "Move to" — all calling the same `move_to_stage` API. Currently zero work. Competitors offer all three. — Scope: 2-3 days
2. **Error state pattern library** — "Organisation not found" is a symptom of no standard error component. Create a reusable `<ErrorState icon={} title={} description={} action={} />` component and apply it to 404, 403, and 500 responses. — Scope: 1-2 days to build + apply

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | Not tested this run | — |
| Mobile responsive | PASS (tested in prior audit) | Minor button overlap on Projects page |
| Color/contrast | Not tested this run | — |
| Screen reader basics | PARTIAL | Icon-only buttons on workflow cards lack aria-labels |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | Mixed | CRM Pipeline: excellent. Agent Flows: adequate but unexplained. AI Usage: misleading (shows zeros, not "no data"). Org not found: bare text, no guidance. |
| Loading states | Generic spinner on all pages | No page-specific skeletons |
| Error states | Poor | "Organisation not found" is the only error tested — dead end with no recovery. Console error spam. |
| Validation feedback | Disabled-button pattern | Works but no inline hints explaining WHY the button is disabled |
