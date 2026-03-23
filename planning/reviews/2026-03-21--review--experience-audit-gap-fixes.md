# Experience Audit — Sprint Gap Fixes (PR #56)

**Date**: 2026-03-21
**Scope**: PR #56 gap fixes — AI Usage wiring, sidebar friction button, clarification form
**Branch**: `feature/2026-03-19--tier1-phase0` (after merge of PR #56)
**Viewport**: Desktop (1440×900)

## Executive Summary

PR #56 fixed the major gap items from the sprint audit. The AI Usage page is now wired to `costsApi` (no more all-zeros) but requires org selection — it doesn't auto-select the current org context, showing "Select an organization to view costs" instead. The sidebar "Report Friction" button exists and opens the feedback dialog, but **doesn't pre-select the Friction Report type** — it opens as Bug Report. The clarification form in AgentFlowCard is code-complete but untestable without agent flow seed data.

## Findings by Severity

### BLOCKERS

None.

### PAIN POINTS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| P1 | J1: AI Usage | **Page shows "Select an organization to view costs" instead of auto-selecting current org.** Breadcrumb shows "Jungleverse" context but page ignores it. User must manually pick an org before seeing any data. Previous version showed zeros — this is better but still requires an extra step. | `ux-audit-ai-usage-select-org.png` | Auto-select `effectiveOrgId` from the current org context (ViewContext/OrganizationProvider). If the user navigates from an org, use that org. Only show the selector if no context is available. | QUICK WIN |

### FRICTION

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| F1 | J2: Friction button | **"Report Friction" button doesn't pre-select friction type.** Clicking "Report Friction" in sidebar More menu opens FeedbackDialog defaulting to "Bug Report" instead of "Friction Report". The `defaultType='friction'` prop is passed but the combobox doesn't respond to it. User must manually switch the type — defeats the purpose of a dedicated button. | — | Fix FeedbackDialog to read `defaultType` prop and set initial combobox value. The wiring exists (`NiceModal.show('feedback', { defaultType: 'friction' })`) but the dialog component may not consume the prop. | QUICK WIN |
| F2 | J1: AI Usage | **No org selector visible on the page.** The page says "Select an organization" but provides no dropdown or picker to do so. The user has no way to select an org from this page — they'd need to navigate to an org first, then come back. | — | Add an org selector dropdown at the top of the AI Usage page, or auto-populate from current context. | QUICK WIN |

### POLISH

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| PO1 | J2: Friction button | **"Report Friction" is in the More popover (2 clicks from sidebar).** Previous audit found it in More → Feedback & Support → select type (3 clicks). Now it's More → Report Friction (2 clicks). Improved but not 1-click. | — | Consider adding a small icon button directly in the sidebar bottom bar (next to Settings) for 1-click access during dogfooding. | POLISH |

## Journey Scorecard

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
| J1: AI Usage (now wired) | USER | 9 | Page loads, wiring confirmed, but requires org selection before showing data | 0 | 1 (P1) | 1 (F2) | **C** — wired but not auto-contextual |
| J2: Friction via sidebar button | USER | 6 | Button exists, dialog opens, but type not pre-selected | 0 | 0 | 1 (F1) | **B** — works but defaultType broken |
| J3: Clarification form | HYBRID | 6 | Cannot test — no agent flows with NeedsClarification in seed data. Code trace confirms form exists. | 0 | 0 | 0 | **N/A** — verified via code trace only |

## Quick Wins

1. **P1** — Auto-select org from context in AI Usage page — 1-2 hours — eliminates the "select an organization" dead screen
2. **F1** — Fix FeedbackDialog to consume `defaultType` prop — 1 hour — the wiring exists but the component ignores the initial value
3. **F2** — Add org selector dropdown to AI Usage page header — 1 hour — fallback for when no org context exists

## Comparison: Before vs After PR #56

| Feature | Before PR #56 | After PR #56 | Remaining |
|---------|---------------|--------------|-----------|
| AI Usage | Shows $0.00 (old tokenUsageApi) | Shows "Select an organization" (new costsApi wired but no auto-context) | Auto-select org from context |
| Friction button | Hidden in More → Feedback → select type (3 clicks) | More → Report Friction (2 clicks) | Fix defaultType pre-selection |
| Clarification form | No form — status display only | Full form with text input, radio options, submit button | Needs seed data to test live |
| CostSummaryCards | Inline in 518-line ai-usage.tsx | Extracted to 89-line component | Done |
| CostTrendChart | Inline | Extracted to 56-line component | Done |
| ClarificationRequest type | Not in shared/types.ts | Properly exported, imported by AgentFlowCard | Done |
| TokenUsageWidget | Used old tokenUsageApi | Migrated to costsApi | Done |
