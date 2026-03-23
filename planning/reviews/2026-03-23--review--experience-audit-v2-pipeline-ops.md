# Experience Audit Report (v2 — with Visual & Layout Quality)

**Date**: 2026-03-23
**Scope**: CRM Pipeline features from pipeline-ops sprint (W1-W7)
**Branch**: `feature/2026-03-21--pipeline-ops`
**Viewport**: Desktop (1440x900), Laptop (1024x768), Mobile (375x812)
**Method**: Playwright MCP — screenshots + snapshots + full interaction scripts

## Executive Summary

The CRM pipeline board has strong visual design, good information density, and thoughtful features (stage checklists, AI agent badges, contextual tabs). A critical state-sync bug — kanban board not refreshing after stage moves via the detail panel — undermines the core workflow. Several polish issues (invoice formatting, tab truncation, "Closed" date on open deals) reduce quality. Console errors: **0** across all interactions (the DialogTitle fix resolved all previous errors).

**Overall Grade: B-**

## Findings by Severity

### BLOCKERS (1)

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 1 | Deal move | Kanban board does not refresh after stage move via detail panel progression bar. Deal appears to stay in old column. Only refreshes after another action (e.g., creating a deal). | Invalidate kanban query on stage-move mutation success. Add success toast. | INVESTMENT |

### PAIN POINTS (3)

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 2 | Deal move | No toast/feedback when moving a deal via stage progression bar | Add sonner toast: "Moved to {stage}. Probability updated: {old}% → {new}%" | QUICK WIN |
| 3 | Deal detail | Last 2 tabs ("Activity", "Agent History") truncated in Sheet mode — users can't see all tabs | Add horizontal scroll or overflow dropdown for tab header row | QUICK WIN |
| 4 | Pipeline board | Collapsing sidebar does not reclaim space for kanban columns — board width stays the same | Fix CSS: content area should be flex-responsive to sidebar width | INVESTMENT |

### FRICTION (5)

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 5 | Deck & Close | Invoice amount shows "$150000" not "$150,000" | Apply `Intl.NumberFormat` to invoice amount | QUICK WIN |
| 6 | Deck & Close | "Send Invoice" enabled on Lead/Business Analysis stage deals — conceptually wrong | Conditionally disable with tooltip: "Available after Proposal Meeting" | QUICK WIN |
| 7 | Deal move | Stage move silently changes probability (70%→20%) with no notification | Include probability change in toast message | QUICK WIN |
| 8 | Overview | "Closed: 3/21/2026" shown on active deals in Timeline section | Only show Closed date when deal is in Closed Won/Lost | QUICK WIN |
| 9 | Overview | "Ask Topsi" button below the fold in Sheet mode, requires scrolling | Move action buttons higher or use sticky footer | POLISH |

### POLISH (4)

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 10 | Expanded mode | Dialog content stays single-column with wasted whitespace on right | Consider 2-column layout in expanded mode | INVESTMENT |
| 11 | Deal detail | Stage label truncated ("Business Anal...") in Sheet-mode progression bar | Add tooltip on hover for full stage name | QUICK WIN |
| 12 | Mobile | "Active" badge overlaps "Powerclub Global" heading at 375px | Stack badge below heading on mobile | QUICK WIN |
| 13 | Mobile | Topsi FAB overlaps deal card content at 375px | Adjust FAB position or add bottom padding on mobile | QUICK WIN |

## What Works Well

- **Deal card design**: Clean, informative — initials avatar, company, status badge, amount, probability, recency
- **Stage progression bar**: Interactive, supports forward AND backward movement, color-coded
- **Stage Checklist (Review tab)**: Contextual checklist items per stage with clear descriptions — excellent workflow guidance
- **AI agent badges**: Each stage shows assigned agent (Scout, Astra, Cash, Lux) with bot/human icons
- **Create Deal form**: Well-structured, sensible defaults (Lead stage, USD), required indicators, disabled submit
- **Deal creation feedback**: Dual toast ("Deal created" + "Deal added to pipeline")
- **Operator Context editing**: Clean inline edit with textarea, Save/Cancel, helpful placeholder
- **Empty states**: All tabs have appropriate messages with actionable guidance
- **Sidebar tooltips**: All collapsed icon buttons show tooltips on hover
- **0 console errors**: Entire audit ran with zero JS errors

## Journey Scorecard

| Journey | Actor | Grade | Notes |
|---------|-------|-------|-------|
| Pipeline board first impressions | USER | **B+** | Clean design, rightmost columns cut off on narrow viewports |
| Deal detail — Sheet mode | USER | **B** | Good content, tabs truncated |
| Deal detail — Expanded mode | USER | **B+** | All tabs visible, content doesn't reflow |
| Move deal between stages | USER | **D** | Core workflow: no feedback, board doesn't update |
| Overview tab content | USER | **A-** | Well-organized, "Closed" date issue |
| Other tabs (Agent History, Deck, Intel) | HYBRID | **B+** | Good empty states, checklist excellent, invoice formatting |
| Add deal flow | USER | **A** | Form well-designed, validation works, instant feedback |
| Responsive/visual quality | USER | **B-** | Works at 1024, struggles at 375, sidebar collapse no effect |
| Workflow coherence | USER | **B** | Backward movement works, org nav works |

## Quick Wins (do first)

1. **#2** — Add toast on stage move + probability change notification — ~30 min
2. **#5** — Format invoice amount with Intl.NumberFormat — ~5 min
3. **#8** — Hide "Closed" date on non-closed deals — ~10 min
4. **#6** — Gate "Send Invoice" on deal stage — ~15 min
5. **#3** — Add horizontal scroll to Sheet tab headers — ~30 min
6. **#11** — Add tooltip to truncated stage labels — ~15 min
7. **#12** — Fix mobile badge overlap — ~15 min
8. **#13** — Fix mobile Topsi FAB overlap — ~10 min

## Investments (plan for)

1. **#1** — Fix kanban real-time refresh after stage move — 0.5–1 day — core workflow
2. **#4** — Sidebar collapse reclaims kanban space — 0.5 day — CSS layout
3. **#10** — Two-column layout in expanded dialog mode — 1–2 days — better space utilization

## Visual & Layout Quality

| Check | Status | Issues |
|-------|--------|--------|
| Sidebar expanded/collapsed | PARTIAL | Board doesn't resize when sidebar collapses (#4) |
| Dialog/panel sizing | PASS | Sheet fits content; Dialog has unused space (#10 — polish) |
| Interactive element reachability | PARTIAL | Tab headers truncated in Sheet (#3); "Ask Topsi" below fold (#9) |
| Text/content overflow | PARTIAL | Stage label truncated (#11); invoice not formatted (#5) |
| Whitespace consistency | PASS | Generally good spacing; expanded mode wastes space (#10) |
| Workflow coherence | PARTIAL | "Send Invoice" on Lead (#6); "Closed" on active deals (#8); no move feedback (#2) |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | All tabs have good empty states | None |
| Loading states | Spinner on initial load | None |
| Error states | ErrorBoundary catches crashes | None (after DialogTitle fix) |
| Validation feedback | Create Deal form has inline validation | Stage move has zero feedback (#1, #2) |

## Console Errors

**0 errors across all 9 steps and 66 Playwright interactions.** The DialogTitle/DialogDescription fix (commit `17bc841a6`) resolved all previous errors.
