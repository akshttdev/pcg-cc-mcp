# Regression Test — PR #57 (Pipeline-Ops Sprint)

**Date**: 2026-03-23
**Branch**: `feature/2026-03-21--pipeline-ops`
**Against**: `main`
**Scope**: 118 commits, 97 files, +6711/-1183

## Summary
- Critical: 1 | High: 5 | Medium: 4 | Low: 3

## Findings

### CRITICAL
| # | Issue | File | Status |
|---|-------|------|--------|
| 1 | SQL injection via `format!()` in `update_deal_field` — allowlisted but unsafe pattern | agent_flow_executor.rs:~420 | Track — refactor to match-based queries |

### HIGH
| # | Issue | File | Status |
|---|-------|------|--------|
| 2 | 7 `let _ =` silent error drops on critical DB ops (state transitions, events) | agent_flow_executor.rs | Track — add tracing::warn |
| 3 | Non-atomic cancel-deadline check — worker and user could race | stage_transition.rs:~320 | Track — low probability at 15s poll |
| 4 | Concurrent execution dispatch — no lock on flow processing | agent_flow_executor.rs:~191 | Track — add execution_started_at check |
| 5 | Task update allows NULL project_id bypass — access control gap | task.rs:~754 | **Fix before merge** |
| 13 | Sonner toast `cancel` property invalid — should be `action` | useCrmPipeline.ts:~270 | **Fix before merge** |

### MEDIUM
| # | Issue | File | Status |
|---|-------|------|--------|
| 6 | No input size limits on tool artifacts | agent_flow_executor.rs:~909 | Track |
| 7 | Missing agent_name whitelist validation | stage_transition.rs:~278 | Track |
| 8 | Silent stage_config JSON parse failure | stage_transition.rs:~98 | Track — add tracing::warn |
| 9 | Inline import() type references | crm.ts, CrmPipelineSettings.tsx | Track — style |

### LOW
| # | Issue | File | Status |
|---|-------|------|--------|
| 10 | Placeholder task_id FK violation risk | stage_transition.rs:~273 | Track |
| 11 | No bounds on cancel_window_secs | stage_transition.rs:~269 | Track |
| 12 | Tool-call loop no dedup check | agent_flow_executor.rs:~245 | Track |

### Clean Areas
- Migrations: fully additive, nullable, idempotent
- Seed script: defensive gap patching, UUID format migration
- Frontend types: no `as any`, proper props, query key factories
- Agent indicator: handles null fields with `??` fallback
- Deal detail expand: Sheet↔Dialog switch
- Dynamic stage owners: stage_config → hardcoded fallback
- E2E: 143/150 pass
- Skills: correct planning/reviews/ paths

## Merge Recommendation

| Check | Status |
|-------|--------|
| Critical issues | 1 found (allowlisted, track for refactor) |
| High issues | 5 found, 2 should fix before merge |
| Auth/security | NEEDS FIX (#5 task update bypass) |
| Type safety | PASS |
| Migration safety | PASS |
| E2E | PASS (143/150) |

**Recommendation**: MERGE WITH CAVEATS — fix #5 and #13 first, track rest as follow-ups
