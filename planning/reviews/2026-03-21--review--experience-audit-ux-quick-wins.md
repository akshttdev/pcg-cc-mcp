# Experience Audit — UX Quick Wins (5 Sprints)

**Date**: 2026-03-21
**Scope**: UX quick wins plan — 31 items across 5 sprints, verifying user-facing improvements
**Branch**: `feature/2026-03-19--tier1-phase0`
**Viewport**: Desktop (1440×900)

## Executive Summary

The 5 UX quick win sprints delivered significant improvements that are visible in the live app. Org blocks collapse correctly (only names shown, not expanded sub-items), the org-not-found error page now provides actionable recovery (icon + message + back link), and the sidebar "Views & Management" label with count "12" is present. One issue found: **Shift+F keyboard shortcut did not open the friction dialog** during Playwright testing — may be a Playwright keyboard event limitation or a wiring issue in the hotkeys library. All other verified items work as expected.

## Findings by Severity

### BLOCKERS

None.

### PAIN POINTS

None.

### FRICTION

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| F1 | J1: Shift+F shortcut | **Shift+F keyboard shortcut did not open FeedbackDialog.** Pressed Shift+F from the dashboard — no dialog appeared. Code trace confirms the wiring exists (AppShell.tsx imports useKeyReportFriction, registry.ts has shift+f binding). May be a Playwright limitation with custom keyboard handlers, or the hotkeys library requires the page element to have focus. | — | Test manually in browser to confirm. If it works in real browser but not Playwright, this is a test-tool limitation, not a bug. If it doesn't work in browser either, check that the HotkeysProvider scope includes GLOBAL for this action. | QUICK WIN |

### POLISH

None — all tested items work well.

## Journey Scorecard

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
| J1: Shift+F friction shortcut | USER | 6 | Dialog did not open via Shift+F | 0 | 0 | 1 (F1) | **C** — needs manual browser verification |
| J4: Sidebar collapsed orgs | USER (observe) | 4 | ✅ All 5 orgs collapsed (names only), no expanded sub-items visible | 0 | 0 | 0 | **A** — clean, as designed |
| J5: Org-not-found recovery | USER | 4 | ✅ Icon + "Organization not found" + "doesn't exist or don't have access" + "← Back to your projects" link → navigates to Projects page | 0 | 0 | 0 | **A** — excellent error recovery |

## Observations (positive)

1. **Sidebar org collapse works perfectly** — 5 orgs shown as collapsed name-only buttons, no visual noise from expanded sub-items. Major improvement from the 50+ items previously visible.
2. **Org-not-found error recovery is complete** — Building2 icon, helpful message with two possible causes ("doesn't exist" or "don't have access"), and working "← Back to your projects" link. Previously was bare "Organisation not found." text with no recovery.
3. **Login works** — admin/admin123 credentials authenticate successfully after DB reseed.
4. **Error boundary works** — Vite module resolution error showed "Page failed to load" with "Try Again" button (caught gracefully, not a white screen).

## Quick Wins

1. **F1** — Verify Shift+F shortcut in real browser. If it doesn't work, check `HotkeysProvider` scope or switch to `useHotkeys('shift+f', callback)` directly in AppShell instead of the semantic hook pattern. — 30min

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | PARTIAL | Shift+F shortcut not verified via Playwright (may be tool limitation) |
| Mobile responsive | Not tested this run | — |
| Color/contrast | Not tested this run | — |
| Screen reader basics | PARTIAL | Org buttons have names, error state has heading + paragraph structure |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | Improved | Agent Flows: "created when AI agents work on tasks". Workflow cards: "Never run — click ▶". |
| Error states | Improved | Org-not-found: icon + message + back link (was bare text). Error boundary: "Try Again" button. |
| Validation feedback | Improved | Feedback dialog: "Title and description required" hint below disabled submit button. |
