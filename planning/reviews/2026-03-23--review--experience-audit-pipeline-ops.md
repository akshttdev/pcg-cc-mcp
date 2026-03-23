# Experience Audit Report

**Date**: 2026-03-23
**Scope**: CRM Pipeline features from pipeline-ops sprint (W1-W7)
**Branch**: `feature/2026-03-21--pipeline-ops`
**Viewport**: Desktop (1440x900) + Mobile (375x812)

## Executive Summary

The CRM pipeline board is well-designed with clear stage columns, owner badges, and intuitive deal progression. The biggest issue — DialogTitle/DialogDescription context errors causing 8 console errors per panel open and crashing the expand button — was **fixed during this audit**. Remaining items are UX polish (invoice formatting, sidebar tooltips, command palette scope).

## Findings by Severity

### BLOCKERS (0 remaining — 3 found, all fixed)

All 3 blockers traced to the same root cause (SheetTitle/SheetDescription used outside Sheet context). Fixed in commit `17bc841a6`.

| # | Finding | Status |
|---|---------|--------|
| 1.1 | Error toasts on page load | **FIXED** |
| 2.1 | Expand button crashes pipeline tab | **FIXED** |
| 2.2 | Delete dialog trapped user (downstream of same crash) | **FIXED** |

### PAIN POINTS

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 2.3 | Deal card | "..." menu only offers Delete — no Edit/Duplicate/Archive | Add common CRM actions to dropdown | INVESTMENT |
| 5.1 | Sidebar | 20+ nav items across 4+ groups is overwhelming | Progressive disclosure, role-based filtering | STRUCTURAL |
| 5.2 | Sidebar | Collapsed sidebar icons lack tooltips/accessible labels | Add `title` or `aria-label` to icon buttons | QUICK WIN |
| 6.2 | Mobile | Kanban board barely usable (only ~1.5 columns visible) | Add mobile kanban view (list or swipe) | INVESTMENT |

### FRICTION

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 2.4 | Deal move | No toast/feedback when deal stage changes via progression bar | Add sonner toast on successful move | QUICK WIN |
| 3.2 | Overview | Call Scheduling section not visible (code exists in OverviewTab but may be conditional or below fold) | Verify rendering conditions, ensure visibility | QUICK WIN |
| 4.5 | Deck & Close | "Send Invoice" active on Lead-stage deal — should be gated | Disable when deal.stage is early-stage | QUICK WIN |
| 5.4 | Cmd+K | Command palette only has 3 actions and 1 project — can't search deals/contacts | Expand search scope to CRM entities | INVESTMENT |
| 6.6 | Mobile | No dedicated mobile kanban — just squished desktop | Consider list view for mobile | STRUCTURAL |

### POLISH

| # | Journey | Finding | Recommendation | Category |
|---|---------|---------|----------------|----------|
| 3.3 | Overview | Contact section missing phone number field | Add if available in data model | QUICK WIN |
| 4.3 | Deck & Close | Invoice amount shows "$150000" not "$150,000" | Use Intl.NumberFormat | QUICK WIN |
| 6.5 | Mobile | Sidebar doesn't auto-expand returning to desktop | Check responsive breakpoint listener | QUICK WIN |

## Journey Scorecard

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
| 1. Pipeline Board | USER | 6 | Full | 0* | 0 | 0 | **A** |
| 2. Deal Interaction | USER | 7 | Full | 0* | 1 | 1 | **B** |
| 3. Overview Tab | USER | 5 | Full | 0 | 0 | 1 | **A** |
| 4. Agent History + Deck | HYBRID | 7 | Full | 0 | 0 | 1 | **A** |
| 5. Sidebar & Nav | USER | 6 | Full | 0 | 2 | 1 | **B** |
| 6. Mobile | USER | 6 | Full | 0 | 1 | 1 | **B** |

*Blockers found and fixed during audit

## Quick Wins (do first)

1. Add tooltips to collapsed sidebar icon buttons — ~30 min
2. Add sonner toast on deal stage move — ~15 min
3. Format invoice amount with `Intl.NumberFormat` — ~5 min
4. Gate "Send Invoice" button on deal stage — ~15 min
5. Verify CallSchedulingSection rendering conditions — ~15 min

## Investments (plan for)

1. Expand command palette to search CRM entities (deals, contacts, companies) — 2-3 days
2. Add Edit/Duplicate/Archive to deal card menu — 1 day
3. Mobile-optimized kanban view (list or swipe) — 3-5 days

## Structural Recommendations (roadmap)

1. Sidebar navigation simplification — role-based item filtering, reduce top-level groups from 4 to 2-3
2. Mobile-first kanban — single-column swipeable stage view for phone users

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | PARTIAL | Cmd+K works; collapsed sidebar lacks labels |
| Mobile responsive | PARTIAL | Layout adapts but kanban is cramped |
| Screen reader basics | PASS | sr-only DialogTitle/SheetTitle added for deal panel |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | Good | "No deals", "No agent activity yet" with helpful CTAs |
| Loading states | Good | Spinner on page load |
| Error states | Improved | ErrorBoundary catches crashes (no longer triggered) |
| Validation feedback | Not tested | No form submissions in these journeys |
