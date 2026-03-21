# UX Quick Wins Mini Sprint

**Date**: 2026-03-21
**Branch**: `feature/2026-03-19--tier1-phase0`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `main` (after PR #56 merge)
**Goal**: Fix 7 low-effort UX issues identified across 5 audit reports
**Estimated effort**: ~5-6h total

---

## Items

| # | What | Source | Est | Status |
|---|------|--------|-----|--------|
| 1 | AI Usage: auto-select org from ViewContext | gap-fixes audit P1 | 1h | ✅ DONE — falls back to first org, org selector for multi-org users |
| 2 | FeedbackDialog: consume `defaultType` prop | gap-fixes audit F1 | 1h | ✅ DONE — useEffect syncs type on re-open |
| 3 | Org-not-found: error state with icon + CTA | v2 audit P4 | 1-2h | ✅ DONE — Building2 icon + helpful message + back link |
| 4 | "Organisation" → "Organization" spelling | v2 audit F4 | 15min | ✅ DONE — 3 files fixed |
| 5 | Agent Flows empty state guidance | v2 audit F3 | 30min | ✅ DONE — explains how flows are created |
| 6 | Collapse empty sidebar sections | v1 audit F6 | 30min | ✅ DONE — default collapsed when no items |
| 7 | Aria-labels on workflow card buttons | v1 audit F8 | 30min | ✅ DONE — 4 buttons labeled |

## Files to Modify

| File | Items | Action |
|------|-------|--------|
| `frontend/src/pages/ai-usage/AIUsagePage.tsx` | #1 | Use org from ViewContext/OrganizationProvider as default |
| `frontend/src/components/dialogs/feedback/FeedbackDialog.tsx` | #2 | Read `defaultType` from NiceModal props, set initial combobox value |
| `frontend/src/pages/ai-usage/` or org layout | #3, #4 | Add error state component for org-not-found, fix spelling |
| `frontend/src/pages/workflows/index.tsx` or AgentFlowBoard | #5 | Update empty state message with guidance |
| `frontend/src/components/layout/sidebar/Sidebar.tsx` | #6 | Collapse sections when they have no children |
| `frontend/src/pages/workflows/index.tsx` or WorkflowCard | #7 | Add aria-label to icon-only buttons |

## Verification

After implementation, run:
1. `/playwright-smoke /,/ai-usage,/workflows`
2. `/experience-audit` on friction button + AI Usage journeys
3. `npm run check` for lint/type safety
