# Functionality Audit Report

**Date**: 2026-03-23
**Planning File**: `planning/2026-03-21--plan--pipeline-ops-sprint.md`
**Branch**: `feature/2026-03-21--pipeline-ops`
**Audit Method**: Mixed — Playwright walkthrough + code trace + backend wiring analysis

## Feature Verdicts

| # | Feature | Type | Plan Says | Actual | Accurate? | Issue |
|---|---------|------|-----------|--------|-----------|-------|
| 1 | StageTransitionProcessor types | backend | DONE | WORKING | Yes | — |
| 2 | process_transition() | backend | DONE | WORKING | Yes | — |
| 3 | handle_won_transition() | backend | DONE | WORKING | Yes | — |
| 4 | schedule/cancel agent flow | backend | DONE | WORKING | Yes | — |
| 5 | move_deal_stage → processor | backend | DONE | WORKING | Yes | — |
| 6 | advance_deal → processor | backend | DONE | WORKING | Yes | — |
| 7 | POST cancel-agent | full-stack | DONE | WORKING (easy) | Yes | — |
| 8 | POST approve-agent | full-stack | DONE | WORKING (easy) | Yes | — |
| 9 | stage_config migration | backend | DONE | WORKING | Yes | — |
| 10 | agent_flows new fields | backend | DONE | WORKING | Yes | — |
| 11 | LLM dispatch via WorkflowLLMService | backend | DONE | WORKING* | Yes | *Requires ENABLE_AGENT_FLOW_ENGINE=1 |
| 12 | Agent tools (get/update/save) | backend | DONE | WORKING | Yes | Real DB queries, not stubs |
| 13 | Retry with model fallback | backend | DONE | WORKING | Yes | 3 attempts, Sonnet fallback |
| 14 | Pulsing status badge on deal card | frontend | DONE | WORKING (easy) | Yes | Correct empty state without agent engine |
| 15 | Cancel/approve toast on move | full-stack | DONE | WORKING (easy) | Yes | Toast with Run Now + Cancel actions |
| 16 | TransitionResult response | full-stack | DONE | WORKING | Yes | — |
| 17 | "Move to..." submenu | frontend | DONE | WORKING (easy) | Yes | Stage progression bar in deal detail |
| 18 | DnD working | frontend | DONE | WORKING (easy) | Yes | handleDragEnd wired |
| 19 | CallSchedulingSection | frontend | DONE | WORKING (easy) | Yes | Date picker, method, status in OverviewTab |
| 20 | Sheet↔Dialog expand | frontend | DONE | WORKING (easy) | Yes | Maximize2/Minimize2 toggle |
| 21 | Agent History tab | frontend | DONE | WORKING (easy) | Yes | Correct empty state, fetches /agent-flows |
| 22 | GET /crm/deals/:id/agent-flows | backend | DONE | WORKING | Yes | Real SELECT from agent_flows + events |
| 23 | Generate invite link | full-stack | DONE | WORKING (easy) | Yes | Token generation, DB persist, clipboard copy |
| 24 | StageConfigEditor component | frontend | DONE | WORKING | Yes | Agent dropdown, auto-trigger, cancel window |
| 25 | StageConfigEditor in settings | frontend | DONE | **NOT WIRED (undiscoverable)** | **NO** | Component exists but no route/page mounts it |
| 26 | Dynamic stage owners | frontend | DONE | WORKING (easy) | Yes | Reads stage_config, falls back to hardcoded |
| 27 | Seed stage configs | backend | DONE | WORKING | Yes | 9 stage types + delivery stages |
| 28 | GET /api/tasks?crm_deal_id=X | backend | DONE | WORKING | Yes | TaskQuery option, handler branches |
| 29 | Task middleware empty project_id | backend | DONE | WORKING | Yes | Admin fallback for project-less tasks |
| 30 | Auth BLOB→TEXT cleanup | backend | DONE | WORKING | Yes | All binds use .as_str() |
| 31 | Types in shared/types.ts | infra | DONE | **PARTIAL** | **NO** | Types manually defined in frontend/src/types/crm.ts, not auto-generated into shared/types.ts |

## Detailed Findings

### Feature 25: StageConfigEditor in CrmPipelineSettings — NOT WIRED

- **Plan status**: DONE
- **Audit verdict**: NOT WIRED (undiscoverable)
- **Plan accurate?**: **NO — component built but never mounted in any route**
- **Component**: `CrmPipelineSettings` at `frontend/src/components/crm/CrmPipelineSettings.tsx`
- **Export**: Re-exported from `frontend/src/components/crm/index.ts`
- **Import sites**: Zero — no page, route, or parent component imports it
- **StageConfigEditor**: Integrated into `CrmPipelineSettings` at line 164-167
- **Issue**: The settings component with StageConfigEditor is fully built but has no UI entry point. No gear icon on pipeline board, no settings page section, no route. Users cannot discover or use it.
- **Impact**: Low — stage configs work via seed migration defaults. The editor would allow runtime customization but isn't needed for pipeline to function.

### Feature 31: Type generation pipeline — PARTIAL

- **Plan status**: DONE
- **Audit verdict**: PARTIAL
- **Plan accurate?**: **NO — types not auto-generated**
- **Rust structs**: Have `#[derive(TS)]` and `#[ts(export)]` in `stage_transition.rs`
- **shared/types.ts**: Does NOT contain TransitionResult, StageConfig, StageAction, etc.
- **Frontend types**: Manually defined in `frontend/src/types/crm.ts` (lines 623-662)
- **Issue**: The Rust→TS type generation pipeline doesn't pick up types from `crates/server/src/` (only `crates/db/`). Frontend types are manually maintained duplicates.
- **Impact**: Low — types work correctly, just not auto-generated. Risk of drift if Rust types change.

### Notable Correct Empty States

- **Agent History tab**: Shows "No agent activity yet" with helpful message. Correct — no flows triggered.
- **Deal card agent badges**: No pulsing badges visible. Correct — `ENABLE_AGENT_FLOW_ENGINE=1` not set.
- **Invite link section**: Not visible on test deals. Correct — renders only when `deal.won_at && deal.crm_contact_id` are both set.

## Action Items

### Should Fix (quality/UX gaps)
- [ ] **Wire CrmPipelineSettings to a route** — Add gear icon on pipeline board or settings page section. Component is complete, just needs a mount point. — `frontend/src/components/crm/CrmPipelineBoard.tsx`
- [ ] **Fix type generation for server crate types** — Either add `crates/server/src/` to the ts-rs export path, or document that frontend types in `types/crm.ts` are manually maintained. — `npm run generate-types` config

### Nice to Have (polish)
- [ ] **CallSchedulingSection visibility** — Section may not be visible below the fold on shorter deal detail panels. Consider prominence.

## Build Verification

- [x] Database seed compatible with migrations — YES (20260414000001 matches latest)
- [x] Servers healthy — frontend :3010, backend :3012 both running
- [x] E2E demos — 143 pass, 2 fail (LLM credit-dependent), 5 skip
- [ ] `cargo test --workspace` — not run in this audit
- [ ] `npm run generate-types:check` — not run (known PARTIAL for server types)

## Overall Verdict

**SHIP WITH CAVEATS**

29/31 features are WORKING with correct behavior verified via Playwright + code trace. The 2 issues are:

1. **CrmPipelineSettings NOT WIRED** — component built but no UI entry point. Low impact since stage configs work via seed defaults. Should wire before next sprint.
2. **Type pipeline PARTIAL** — server crate types manually duplicated in frontend. Works but risks drift. Low urgency.

No false DONE claims on critical features. All backend endpoints do real work (no stubs). All frontend components render correctly with proper empty states. Console errors: zero on all pipeline pages.
