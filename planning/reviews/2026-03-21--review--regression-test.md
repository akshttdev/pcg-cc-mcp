# Regression Test Results (2026-03-21)

**Branch**: `feature/2026-03-19--tier1-phase0`
**Against**: `main`
**Scope**: 60 commits, 52 files, +3464/-776 lines

### Summary
- Files analyzed: 52
- Critical: 0 | High: 2 (fixed) | Medium: 1 | Low: 2

### High (fixed before merge)

| # | Issue | File:Line | Fix |
|---|-------|-----------|-----|
| 1 | `uuid::Uuid::new_v4()` used instead of `DbUuid::new()` in stage transition processor | `stage_transition.rs:422,427` | Replaced with `DbUuid::new()`, removed `use uuid::Uuid` |
| 2 | Delivery deal creation error silently dropped (`let _ = ... .ok()?`) | `stage_transition.rs:388` | Added `match` with `tracing::error!` on failure |

### Medium (tracked for follow-up)

| # | Issue | File:Line | Status |
|---|-------|-----------|--------|
| 3 | DB query errors silently swallowed via `.ok()` in intel status checks | `stage_transition.rs:~160` | Low risk — internal checks. Add warning logs in follow-up. |

### Low (nits)

| # | Issue | File:Line | Status |
|---|-------|-----------|--------|
| 4 | FeedbackDialog `useEffect` for `defaultType` sync could be fragile if misused | `FeedbackDialog.tsx:87` | Safe for current usage. Add comment. |
| 5 | `AgentFlowExecutorConfig::max_concurrent` field defined but not yet enforced in tick() | `agent_flow_executor.rs:199` | Config reads from env but tick() processes all flows. Tracked for pipeline-ops sprint. |

### Clean Areas (verified correct)

| Area | Status |
|------|--------|
| SQL injection | ✅ All queries use bind params |
| Auth on new endpoints | ✅ `cancel_deal_agent` and `approve_deal_agent` have `require_deal_org_access` |
| Webhook cooldown race fix | ✅ Atomic `try_claim_trigger()` replaces racy `is_past_cooldown()` |
| BackgroundWorker shutdown | ✅ All 4 new workers use `tokio::select!` with `shutdown.cancelled()` |
| Type safety | ✅ No `as any` casts, all props properly typed |
| API contracts | ✅ costs.ts, agent-flows.ts, crm.ts all match backend endpoints |
| Dead code cleanup | ✅ tokenUsageApi fully removed, no orphaned imports |
| Keyboard shortcut (Shift+F) | ✅ No browser conflicts, respects form focus |
| Sidebar restructure | ✅ Backward compatible via `MANAGEMENT_NAV_ITEMS = [...BUSINESS, ...PLATFORM]` |
| Migration safety | ✅ All additive, nullable columns, idempotent index |
| Conflict markers | ✅ None found |
| README security | ✅ No hardcoded production credentials |

## Merge Recommendation

| Check | Status |
|-------|--------|
| Critical issues | 0 found |
| High issues | 2 found, 2 fixed |
| Auth/security | PASS |
| Type safety | PASS |
| Migration safety | PASS |
| Frontend contracts | PASS |
| Dead code | PASS |
| Graceful shutdown | PASS |

**Recommendation**: MERGE
