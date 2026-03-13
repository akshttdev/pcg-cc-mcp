# Dogfood Pipeline Activation & QA Loop

**Date:** 2026-03-13
**Depends on:** PR #21 (merged), PR #22 (WIP, Topsi-Workflow Bridge)
**Companion:** `notes/2026-03-13--reference--llm-workflow-config.md`
**Status:** CODE COMPLETE — all phases implemented, pending runtime configuration

---

## Goal

Activate the full dogfooding pipeline end-to-end: feedback/webhook → LLM triage → task creation → agent execution → PR → automated QA → human approval. All tracked on the ORCHA Platform kanban board with proper status transitions and audit trail.

---

## Task Lifecycle (Status Transitions)

Using existing schema (5 statuses + `approval_status` + PR tracking):

| Pipeline Stage | `status` | `approval_status` | PR State | What Triggers Transition |
|---|---|---|---|---|
| Reported | `todo` | — | — | Triage pipeline creates task |
| In Development | `inprogress` | — | — | `auto_start_agent_execution()` |
| QA Review | `inreview` | `Pending` | `open` | Dev agent finishes → PR created → QA triggered |
| QA Iteration | `inprogress` | `ChangesRequested` | `open` | QA finds issues → dev agent re-runs |
| Awaiting Human | `inreview` | `Pending` | `open` | QA passes → summary comment |
| Deployed | `done` | `Approved` | `merged` | Human merges PR |
| Rejected | `cancelled` | `Rejected` | `closed` | Human rejects |

### Schema Improvement Recommendations (Future)

1. **Add `qa` status** — distinguish QA review from human approval on kanban
2. **Add `deployed` status** — distinguish "code merged" from "manually done"
3. **Add `task_id` FK on `merges`** — direct task→PR lookup (currently goes through task_attempts)
4. **`task_status_history` table** — full audit of `(task_id, from, to, changed_by, timestamp)`

---

## Phases — All Complete

### Phase 7: Operational Activation ✅

- **7A**: Seed ORCHA Dev Agent + QA Agent + execution configs — `20260330000000_seed_dogfood_agents.sql`
- **7B**: Document runtime config (API keys, GitHub PAT) — `notes/2026-03-13--reference--llm-workflow-config.md`

### Phase 8: Auto-Approve + QA Loop ✅

- **8A**: Wire `auto_approve` in `fire_triggers_for_data_source()` — includes contact-to-company linking + auto-start agent execution
- **8B**: QA agent review hook after PR creation — `try_auto_qa_review()` in container.rs
- **8C**: Audit trail via PR comments — `post_dev_agent_pr_comment()` + structured QA review comments

### Phase 9: PR Feedback Loop ✅

- **9A**: QA review output schema (verdict/criteria/issues) — JSON parsed from execution artifacts
- **9B**: Feedback loop controller (max 2 dev↔QA iterations) — `handle_qa_result()` in container.rs
- **9C**: Structured PR comment format — markdown with criteria checks + issues table

### Phase 10: E2E Demonstration Tests ✅

- **10A**: Feedback → triage → task — `e2e/workflow-pipeline.spec.ts`
- **10B**: Webhook → DataSource → triage
- **10C**: Task assignment → execution status
- **10D**: Full pipeline walkthrough

### Phase 11: Frontend/UX Polish ✅

- **11A**: Completion criteria in task form — already in TaskFormDialog
- **11B**: Agent execution dashboard — `frontend/src/pages/agent-executions.tsx` with summary cards, filters, agent badges
- **11C**: Project scaffolding UI — already implemented (4 templates)
- **11D**: ORCHA Task Server in MCP carousel — already in default_mcp.json

---

## Remaining: Runtime Configuration Only

See companion doc `notes/2026-03-13--reference--llm-workflow-config.md` for the full checklist:

1. Set `ANTHROPIC_API_KEY` (or other provider key)
2. Set GitHub PAT in deployment config
3. Set `GITHUB_WEBHOOK_SECRET` for production
4. Optional: pin model on Bug Triage Pipeline node
