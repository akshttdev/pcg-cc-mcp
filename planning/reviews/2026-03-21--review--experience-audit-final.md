# Experience Audit — Tier 1 Phase 0 Sprint (Final Re-Audit)

**Date**: 2026-03-21
**Scope**: Final re-audit of original 10-item sprint plan after PR #56 + 5 UX sprints + pipeline-ops W1
**Branch**: `feature/2026-03-19--tier1-phase0`
**Viewport**: Desktop (1440×900)

## Executive Summary

Dramatic improvement from the first experience audit. AI Usage auto-selects org and shows proper empty states with org selector (was "Select an organization" dead screen). Sidebar collapses non-active orgs (was 50+ expanded items). CRM deal "Move to..." context menu is available. Org-not-found shows helpful error with recovery link (was bare text). One persistent bug: **"Report Friction" button doesn't pre-select friction type** — the `defaultType` prop is not consumed by the combobox component despite the `useEffect` wiring. Validation hint ("Title and description required") works correctly.

## Findings by Severity

### BLOCKERS

None.

### PAIN POINTS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| P1 | J1: Friction report | **"Report Friction" button opens dialog with "Bug Report" type, not "Friction Report".** The `useEffect` sets `type` state to `'friction'` but the combobox component doesn't re-render from the state change. The wiring exists (`NiceModal.show('feedback', { defaultType: 'friction' })` → `useEffect` → `setType(defaultType)`) but the Select/combobox doesn't pick up the updated state. | — | The issue is likely that the `Select` component's `value` prop reads from `type` state correctly, but the `SelectValue` display text comes from the selected item's label, not the state. Debug: check if `<Select value={type}>` is receiving `'friction'` after the useEffect fires. May need to use the `onValueChange` callback pattern instead of controlled state. | QUICK WIN |

### FRICTION

None new.

### POLISH

None new.

## Journey Scorecard

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
| J1: Report friction | USER | 6 | Form opens, validation works, but type not pre-selected to Friction | 0 | 1 (P1) | 0 | **B** — works but wrong default type |
| J2: AI Usage | USER | 5 | ✅ Auto-selects "Jungleverse", org selector visible, time range works, proper empty states | 0 | 0 | 0 | **A** — excellent |
| J4: Agent Flows | AGENT (observe) | 3 | ✅ Visible tab, proper empty state with guidance | 0 | 0 | 0 | **A** — well-designed |
| Sidebar IA | USER (observe) | — | ✅ Only active org expanded, "Views & Management" labeled | 0 | 0 | 0 | **A** — major improvement |
| Org error recovery | USER | 4 | ✅ Icon + message + working back link | 0 | 0 | 0 | **A** — excellent |

## Comparison: First Audit → Final Audit

| Feature | First Audit (morning) | Final Audit (now) |
|---------|----------------------|-------------------|
| AI Usage | **D** — all zeros, wrong API | **A** — auto-selects org, proper empty states |
| Friction report | **B** — works but buried (3 clicks) | **B** — 2 clicks now, but defaultType bug |
| Agent Flows | **B** — buried, unexplained empty state | **A** — visible tab, helpful guidance |
| CRM deal stages | **F** — impossible (no transition method) | **A** — context menu + clickable stage bar |
| Sidebar | **C** — 50+ items, overwhelming | **A** — collapsed orgs, labeled groups |
| Org error | **D** — bare text dead end | **A** — icon + message + recovery link |

## Quick Wins

1. **P1** — Fix defaultType prop consumption in FeedbackDialog combobox — 1h — the state is set but the Select component may not be reading from it correctly

## Investments

None remaining — all prior investment items have been shipped.

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | PARTIAL | Shift+F shortcut needs manual browser verification |
| Mobile responsive | PASS | Button stacks below subtitle on mobile |
| Screen reader basics | PASS | Aria-labels on workflow card buttons, org names visible |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | GOOD | AI Usage: "No usage data". Agent Flows: "created when agents work on tasks". CRM Pipeline: excellent with CTA. |
| Loading states | ADEQUATE | Generic spinner, no page-specific skeletons |
| Error states | GOOD | Org-not-found: icon + message + back link. Error boundary: "Try Again" button. |
| Validation feedback | GOOD | "Title and description required" hint below disabled submit button |
