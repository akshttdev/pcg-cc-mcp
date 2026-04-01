# Experience Audit Report

**Date**: 2026-03-27
**Scope**: Contacts Unification Phase 1 — CRM pipeline, deal creation, Intel tab, contact intelligence, review tasks, navigation
**Branch**: `feature/contacts-unification-phase1`
**Viewport**: Desktop (1280x720) + Sidebar expanded/collapsed

## Executive Summary

The CRM pipeline experience is generally strong — deal creation, kanban board, stage progression, and Intel tab all work. The contacts unification (Phase 1) successfully shows intelligence data from Scout on the Intel tab. Two critical issues: review task links from My Tasks lead to blank pages (BLOCKER), and raw JSON from simulated Scout research leaks into user-facing text across multiple views (PAIN POINT). Contact detail panels don't surface intelligence data despite it being stored on the contact.

## Findings by Severity

### BLOCKERS

| # | Journey/Page | Finding | Recommendation | Category |
|---|-------------|---------|----------------|----------|
| B1 | My Tasks → Review Task Link | Clicking arrow on CRM review tasks (project_id="") navigates to `/projects/tasks/:id` which matches no route — shows blank white page. Console: "No routes matched location". User hits a dead end with no way to view or complete the review task from My Tasks. | Add a `/my-tasks/:taskId` route that renders the task detail, OR ensure CRM review tasks get a valid `project_id` so the existing project task route works. | QUICK WIN |

### PAIN POINTS

| # | Journey/Page | Finding | Recommendation | Category |
|---|-------------|---------|----------------|----------|
| P1 | Pipeline cards, Intel tab, Review tab, Operator Context | Raw JSON leaks into intelligence summary text. Shows `Original context: {"name":"Phase1 Verification Deal","description":"[Scout Research — Simulated]\n\n...` — visible on deal cards, Intel tab "Person Intelligence" section, Review tab "Intel Snapshot", and Operator Context field. | Strip the `Original context: {...}` suffix from `intelligence_summary` before display. Either sanitize on write (in `agent_flow_executor.rs`) or truncate at render time in the frontend component. | QUICK WIN |
| P2 | Contact Detail Panel | Contact detail (from Contacts page) does not display intelligence data. Phase 1 added `intelligence_summary`, `intelligence_status`, `intelligence_confidence` etc. to `crm_contacts`, but the contact detail panel only shows: name, stage badge, email, company, linked deals, created date. User must navigate to the deal's Intel tab to see intelligence. | Add an "Intelligence" section to the contact detail panel showing: status badge, summary text, confidence %, last run date. Consider linking to the deal Intel tab for full details. | INVESTMENT |

### FRICTION

| # | Journey/Page | Finding | Recommendation | Category |
|---|-------------|---------|----------------|----------|
| F1 | Create Deal dialog | Contact dropdown has no search/filter. With 94 contacts, user must scroll through the full list to find the right contact. | Add a search input at the top of the contact dropdown that filters by name or company as the user types. | QUICK WIN |
| F2 | Create Deal → Pipeline | Two redundant toast notifications on deal creation: "Deal created" AND "Deal added to pipeline." Same action, two toasts. | Consolidate into a single toast: "Deal created and added to pipeline." | QUICK WIN |
| F3 | Org header → Pipeline total | Header shows "Pipeline $382k" while pipeline body shows "$406,000" after adding a deal. The header total is stale (doesn't update without full page reload). Also note inconsistent formatting: header uses "$382k" shorthand, body uses "$406,000" full format. | Either live-update the header stat on deal mutations, or remove it to avoid stale data. Standardize number formatting. | QUICK WIN |
| F4 | Deal card layout | New deal card (without contact/assignee) shows less information density than cards with contacts — no timestamp, no assignee. The visual inconsistency makes the card look incomplete even though the data simply doesn't exist yet. | Show "Unassigned" placeholder for missing assignee and "Just now" for timestamp on newly created deals. | POLISH |

### POLISH

| # | Journey/Page | Finding | Recommendation | Category |
|---|-------------|---------|----------------|----------|
| PO1 | Pipeline kanban | Empty stage columns show "No deals" text and "+ Add deal" — good empty state. Stage headers show agent/role assignment and dollar total — useful context. | No change needed — good pattern. | — |
| PO2 | Dark mode | Dark mode toggle works instantly, pipeline kanban renders correctly, deal cards have appropriate contrast, status badges remain readable. No visual issues found. | No change needed. | — |
| PO3 | Sidebar collapsed | Content area reflows correctly when sidebar collapses. More pipeline stages visible. Tab bar reveals hidden tabs (Integrations, Cloud). No clipping or overlap. | No change needed. | — |

## Journey Scorecard

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
| Create CRM Deal | USER | 5 | YES | 0 | 0 | 2 (F1, F2) | B |
| View Pipeline Kanban | USER | 2 | YES | 0 | 1 (P1) | 1 (F3) | B |
| Open Deal → Intel Tab | USER | 3 | YES | 0 | 1 (P1) | 0 | B |
| Open Deal → Review Tab | USER | 3 | YES | 0 | 1 (P1) | 0 | B |
| Scout Intelligence (observe) | AGENT | 3 | YES | 0 | 0 | 0 | A |
| View Contact Intelligence | USER | 3 | PARTIAL | 0 | 1 (P2) | 0 | C |
| Complete Review Task from My Tasks | USER | 2 | NO | 1 (B1) | 0 | 0 | F |
| Dark mode toggle | USER | 1 | YES | 0 | 0 | 0 | A |
| Navigate CRM sections | USER | 2 | YES | 0 | 0 | 0 | A |

## Quick Wins (do first)

Ranked by impact-to-effort ratio:

1. **B1** — Add `/my-tasks/:taskId` route or fix review task `project_id` — ~2 hours — unblocks review task workflow
2. **P1** — Strip raw JSON from intelligence summaries — ~30 min — removes most visually jarring issue across 4 views
3. **F1** — Add search to contact dropdown in Create Deal — ~1 hour — prevents frustration at scale
4. **F2** — Consolidate deal creation toasts — ~15 min — removes redundancy
5. **F3** — Sync header pipeline total on mutations — ~1 hour — removes stale data confusion

## Investments (plan for)

1. **P2** — Surface intelligence data on contact detail panel — ~4 hours — completes the contacts unification story; currently intelligence is stored but invisible on the contact. Enables the use case: "I want to see what Scout found about this person."

## Structural Recommendations (roadmap discussion)

1. **Contact search across the app** — The contact dropdown in Create Deal is just a `<select>`. As contacts scale, every contact picker needs search/autocomplete. Consider a shared `<ContactPicker>` component with debounced search that can be used in deal creation, task assignment, and anywhere contacts are referenced.
2. **Intelligence display consistency** — Intelligence data appears differently in: deal card preview (truncated), Intel tab (full), Review tab (snapshot), Operator Context (raw). Consider a shared `<IntelligenceSummary>` component with consistent formatting and a "show more" pattern.

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | PARTIAL | Sidebar and top nav are keyboard-accessible. Deal cards are buttons (good). Contact dropdown is a native select (accessible). Form fields have labels. Not tested: drag-and-drop stage transitions have no keyboard equivalent. |
| Mobile responsive | NOT TESTED | Desktop-only audit. |
| Color/contrast | PASS | Dark mode has good contrast. Status badges use color + text (not color-only). |
| Screen reader basics | PARTIAL | Buttons have labels, form fields associated with labels. Icon-only buttons in collapsed sidebar have no visible labels (rely on tooltips). Deal cards have verbose accessible names that include full intelligence text (noisy for screen readers). |

## Visual & Layout Quality

| Check | Status | Issues |
|-------|--------|--------|
| Sidebar expanded/collapsed | PASS | Content reflows correctly, no clipping or overlap |
| Dialog/panel sizing | PASS | Create Deal dialog fits content well, action buttons visible without scrolling |
| Interactive element reachability | PASS | All buttons clickable, dropdowns open correctly, no z-index issues observed |
| Text/content overflow | FAIL | Intelligence summary text includes raw JSON that overflows visually on deal cards; truncation with ellipsis works but the truncated content is still garbage (JSON) |
| Whitespace consistency | PASS | Consistent spacing in pipeline columns, deal cards, contact grid |
| Workflow coherence | PARTIAL | Deal → Intel → Scout → Review flow is clear. But Review tasks in My Tasks lead to dead end (B1). Contact → Intelligence path is missing (P2). |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | 8/10 pages have good empty states | Pipeline empty columns show "No deals" + "+ Add deal" (good). Recent Activity shows "No recent activity" with explanation (good). |
| Loading states | PARTIAL | My Tasks showed "Loading..." text during data fetch. Pipeline loads without visible spinner (fast enough). No skeleton screens observed. |
| Error states | NOT TESTED | Did not simulate API errors. Console showed 0 errors throughout all journeys. |
| Validation feedback | PASS | Create Deal: "Create Deal" button disabled until required field filled. Required field marked with asterisk. |
