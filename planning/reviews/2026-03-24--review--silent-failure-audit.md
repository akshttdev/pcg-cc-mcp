# Silent Failure Audit — Pipeline/CRM Backend

**Date**: 2026-03-24
**Branch**: `feature/2026-03-19--tier1-phase0`
**Scope**: stage_transition.rs, agent_flow_executor.rs, crm_deal_transitions.rs, crm_deal_automations.rs, agent_flow.rs

## Summary

**Total: 39 silent failures** — 6 critical, 19 high, 14 medium

Root causes:
1. `let _ = ...` drops all errors without logging
2. `.ok().flatten()` swallows DB errors silently
3. Multi-step transactions don't validate each step
4. Inconsistent error levels (WARN vs ERROR for state changes)

## Critical (6) — deal state integrity

| # | File:Line | Issue |
|---|-----------|-------|
| 1 | stage_transition.rs:209 | `won_at` update logged as WARN, should block or retry |
| 2 | stage_transition.rs:223 | `lost_at` update same pattern |
| 3 | crm_deal_automations.rs:1160 | `let _ = CrmDeal::move_to_stage` — deal doesn't move to Won |
| 4 | crm_deal_automations.rs:1222 | `let _ =` client creation — orphaned FK downstream |
| 5 | crm_deal_automations.rs:1238 | `let _ =` project creation — tasks have orphaned FK |
| 6 | crm_deal_automations.rs:1248 | `let _ =` deal-to-project link — deal never linked |

## High (19) — pipeline progression

| # | File:Line | Issue |
|---|-----------|-------|
| 7 | crm_deal_automations.rs:99 | Person intel status → 'queued' fails silently |
| 8-9 | crm_deal_automations.rs:163,178 | Phase 1 research task creation dropped |
| 10 | crm_deal_automations.rs:446 | Mark Phase 1 tasks done dropped |
| 11 | crm_deal_automations.rs:678 | Business report update (Astra) dropped |
| 12 | crm_deal_automations.rs:696 | Business report creation dropped |
| 13-14 | crm_deal_automations.rs:858,937 | Deliverable INSERT dropped (2x) |
| 15 | crm_deal_automations.rs:1027 | Knowledge source creation dropped |
| 16 | crm_deal_automations.rs:1038 | Deck URL save to deal dropped |
| 17 | crm_deal_automations.rs:1257 | Deliverable-to-project move dropped |
| 18-19 | crm_deal_automations.rs:1284,1295 | Task creation from deliverables dropped (2x) |
| 20 | crm_deal_automations.rs:1309 | VIBE transaction creation dropped |
| 21 | crm_deal_automations.rs:1336 | Won activity log dropped |
| 22 | crm_deal_transitions.rs:346 | Previous review task cancellation dropped |
| 23 | crm_deal_transitions.rs:390 | Review task INSERT dropped |
| 24 | crm_deal_transitions.rs:443 | BA task assignment dropped |

## Medium (14) — observability

Agent flow executor metadata updates, artifact content storage, auto-advance incomplete recovery.

## Fix Strategy

1. **Critical**: Convert `let _ =` to `if let Err(e) = ... { tracing::error!(...); return Err(...); }` — these should propagate errors
2. **High**: Add `tracing::error!` logging at minimum; ideally propagate errors
3. **Medium**: Add `tracing::warn!` logging
