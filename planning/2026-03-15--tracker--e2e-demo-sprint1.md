# Sprint 1: Fix Demo 1 + Demo 2 E2E Scripts (Manual Flow)

**Branch:** `e2e/dogfood-demo-replay-2026-03-14`
**Started:** 2026-03-15
**Status:** Complete — 15/15 tests passing
**Next:** Sprint 2 — agent-driven demos + visual indicators (see main plan)

## MCP Investigation Findings

### Demo 1: Bug Report Lifecycle
| Item | Finding |
|------|---------|
| Feedback button | Sidebar collapsed = icon-only, no accessible name. **Fix:** Added `data-testid="feedback-button"` to sidebar.tsx |
| Feedback Type | Default is "Bug Report" — no need to select it |
| Severity combobox | Filter text: "Medium - Affects some functionality" — `/Medium/` works |
| Severity options | `option "High - Significant impact"` — `/high/i` works |
| Title field | `textbox "Title *"` — confirmed correct |
| Description field | `textbox "Description *"` — confirmed correct |
| Submit success | Toast: "Thank you! Your feedback has been submitted." then dialog closes. **Bug:** test checked `"Thank You!"` which never appears as DOM text |
| Bug title format | Feedback creates tasks with `[Bug]` prefix: `[Bug] <title>` |
| Kanban columns | Headers are `paragraph` elements: "To Do", "In Progress", "In Review", "Done", "Cancelled" |
| Column counts | Sibling `generic` element with count text |
| Task cards | `button` role with accessible name = title + priority + description |

### Demo 2: Manual QA Trigger
| Item | Finding |
|------|---------|
| createDemoProject | Works correctly, returns project ID |
| createTaskViaUI | Works correctly, waits for "Agent Reviewers" to appear |
| addQaWatcher | **Bug found:** `agentsApi.listActive()` returned `{data:[...]}` wrapper, not array. Caused `availableAgents.filter is not a function` crash |
| changeTaskStatus | Uses combobox filter + option click + toast confirmation |
| ORCHA QA agent | Exists in seed migration `20260330000000_seed_dogfood_agents.sql` |

## Bugs Found & Fixed

1. **`agentsApi.listActive()` crash** — Backend returns `ApiResponse::success(data)` = `{data:[...], success:true}`, but frontend `listActive()` returned `response.json()` directly without unwrapping `.data`. Caused `availableAgents.filter is not a function` TypeError when opening the agent watcher picker. Fixed in `frontend/src/lib/api.ts`.

2. **`agentsApi.search()` same issue** — Same unwrapping bug. Fixed alongside `listActive`.

3. **Feedback dialog "Thank You!" assertion** — Old test checked for `page.getByText("Thank You!")` but the actual UI shows a toast with "Thank you! Your feedback has been submitted." and then the dialog closes. Fixed to check `/Thank you.*feedback/i`.

4. **Feedback button selector** — Sidebar collapsed mode renders the button as icon-only (no accessible name). Added `data-testid="feedback-button"` and created `openFeedbackDialog()` helper.

## Implementation Progress

- [x] Merge main into branch
- [x] MCP walkthrough: Demo 1 feedback dialog
- [x] MCP walkthrough: Demo 1 kanban board
- [x] Add `data-testid="feedback-button"` to sidebar.tsx
- [x] Add `ensureAgentsSeeded()` helper
- [x] Add `openFeedbackDialog()` helper
- [x] Fix `agentsApi.listActive()` and `agentsApi.search()` crash
- [x] Fix `bug-report-lifecycle.spec.ts`
- [x] Fix `manual-qa-trigger.spec.ts`
- [x] Run Demo 1 automated — 8/8 passed
- [x] Run Demo 2 automated — 7/7 passed
- [x] Run both demos together — 15/15 passed (53.9s)
- [ ] PR

## Files Modified

| File | Changes |
|------|---------|
| `frontend/src/components/layout/sidebar.tsx` | Added `data-testid` to feedback button (collapsed + expanded) |
| `frontend/src/lib/api.ts` | Fixed `listActive()` and `search()` to unwrap `body.data` |
| `e2e/helpers.ts` | Added `ensureAgentsSeeded()`, `openFeedbackDialog()` |
| `e2e/demos/bug-report-lifecycle.spec.ts` | Self-contained setup, correct selectors, stronger assertions |
| `e2e/demos/manual-qa-trigger.spec.ts` | Self-contained setup, watcher state verification |

## Sprint 2 Scope (Next)

Sprint 1 proved the UI works but **doesn't prove the agent system works**. Key gaps:

1. **Status transitions are manual** — test script clicks through To Do → In Progress → In Review → Done. In production, dev agents and QA watchers drive these transitions.
2. **No visual indicators** — when an agent triggers a status change, the user sees nothing. Need toast notifications.
3. **QA watcher never actually fires** — Demo 2 has no PR linked to the task, so `spawn_watcher_reviews()` finds no PR and silently skips.

Sprint 2 will:
- Add **toast notifications** for agent-driven transitions (agent started, PR created, QA triggered, QA verdict)
- Rewrite Demo 1 to use **real agent execution** (dev agent → branch → PR → QA watcher → verdict)
- Update Demo 2 to **link a PR** so the QA watcher actually triggers on In Review
- Assert that **toasts appear** as proof the features are working

See `planning/2026-03-14--plan--e2e-demo-fixes.md` for full Sprint 2 plan.
