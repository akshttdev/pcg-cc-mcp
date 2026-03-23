# UX Quick Wins — 5 Mini Sprints

**Date**: 2026-03-21
**Branch**: `feature/2026-03-19--tier1-phase0`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `main` (after PR #56 merge)
**Status**: COMPLETE — 31 items shipped across 5 sprints + regression fixes
**Source**: 5 audit reports (functionality audit, experience audit v1/v2, pipeline stages audit, gap fixes audit)

---

## Sprint 1 — UX Polish (7 items) ✅

| # | What | Source | Commit |
|---|------|--------|--------|
| 1 | AI Usage: auto-select org + org selector for multi-org | gap audit P1 | `47ead8a` |
| 2 | FeedbackDialog: useEffect syncs defaultType on re-open | gap audit F1 | `47ead8a` |
| 3 | Org-not-found: Building2 icon + helpful message + back link | v2 audit P4 | `47ead8a` |
| 4 | "Organisation" → "Organization" spelling (3 files) | v2 audit F4 | `47ead8a` |
| 5 | Agent Flows empty state: explains how flows are created | v2 audit F3 | `47ead8a` |
| 6 | Sidebar: collapse empty Internal Projects/Clients by default | v1 audit F6 | `47ead8a` |
| 7 | Workflow card aria-labels on all icon-only buttons | v1 audit F8 | `47ead8a` |

## Sprint 2 — Navigation & Accessibility (7 items) ✅

| # | What | Source | Commit |
|---|------|--------|--------|
| 8 | Notifications: retry:1 + retryDelay to stop 500 spam | v1 audit P4 | `b7f6f87` |
| 9 | Agent Flows promoted to visible workflow tab | v1 audit F1 | `b7f6f87` |
| 10 | Shift+F keyboard shortcut for friction report | v1 audit F2 | `b7f6f87` |
| 11 | Mobile: "Create Project" stacks below subtitle | v1 audit F5 | `b7f6f87` |
| 12 | VIBELAND/VIBE sidebar tooltips | v1 audit PO1 | `b7f6f87` |
| 13 | Workflow "Never run" → "click ▶ to execute" | v1 audit PO2 | `b7f6f87` |
| 14 | Stage agent badges show type on hover | pipeline F2 | `b7f6f87` |

## Sprint 3 — CRM Pipeline UX (3 items) ✅

| # | What | Source | Commit |
|---|------|--------|--------|
| 15 | CRM org tab bar overflow-x-auto for responsive scroll | v1 audit F7 | `c364cdc` |
| 16 | Feedback dialog inline validation hint below disabled submit | v2 audit F6 | `0e9e77a` |
| 17 | CRM deal "Move to..." stage submenu in context menu | pipeline P2 | `c5f66c9` |

## Sprint 4 — Backend Hardening + CRM UX (6 items) ✅

| # | What | Source | Commit |
|---|------|--------|--------|
| 18 | Webhook cooldown: atomic try_claim_trigger() | func audit F7 | `33a8a20` |
| 19 | DbUuid::nil() + remove uuid::Uuid from transitions | func audit F1 | `f009bd6` |
| 20 | Auto-install git hooks via npm prepare script | func audit F6 | `15f1c9b` |
| 21 | AgentFlowExecutorConfig from env (poll interval + max concurrent) | func audit F13 | `6deb59f` |
| 22 | Clickable stage progress bar in deal detail dialog | pipeline P1 | `9dcbc3d` |
| 23 | Sidebar: only expand active org by default | v1 audit P1 | `dfc77db` |

## Sprint 5 — Code Quality & Architecture (4 items + cleanup) ✅

| # | What | Source | Commit |
|---|------|--------|--------|
| 24 | Remove dead tokenUsageApi (80 lines) | gap audit | `243d1b9` |
| 25 | SubmitFeedbackRequest validation (lengths, enums, ranges) | func audit F9 | `3aceca0` |
| 26 | Sidebar restructure: 3 labeled groups (Global Views / Business / Platform) | v1 audit P2 | `66ec594` |
| 27 | Migrate 4 background tasks to BackgroundWorker trait | func audit F8 | `c27b51a` |
| 28 | Remove old bare tokio::spawn loops from main.rs (-226 lines) | cleanup | `664253a` |

## Regression Fixes ✅

| # | What | Source | Commit |
|---|------|--------|--------|
| 29 | uuid::Uuid → DbUuid in stage_transition.rs | regression | `36f6b78` |
| 30 | Delivery deal error logging (was silently dropped) | regression | `36f6b78` |
| 31 | Warning logs on 4 swallowed .ok() DB queries | regression | `b4acafb` |

---

## Post-Sprint Fix ✅

| # | What | Commit |
|---|------|--------|
| 32 | FeedbackDialog defaultType bug — reset effect hardcoded `setType('bug')`, overriding defaultType prop | `61b6654` |
| 33 | More popover closes on action click (Report Friction, Feedback & Support) | `61b6654` |

**Root cause**: Two competing `useEffect` hooks — the `modal.visible` reset effect fired after the `defaultType` sync effect and overwrote the type back to `'bug'`. Fix: use `defaultType ?? 'bug'` in the reset effect.

---

## Remaining (INVESTMENT-level — not mini sprint candidates)

| Item | Effort | Owner |
|------|--------|-------|
| Dashboard/home page with activity feed | 3-5 days | Future sprint |
| DnD visual feedback (column highlights, ghost cards) | 2-3h | Pipeline-ops W3 |
| Agent flow LLM dispatch | 3 days | Pipeline-ops W2 |
| Deal detail expand mode + agent history tab | 1.5 days | Pipeline-ops W5 |
