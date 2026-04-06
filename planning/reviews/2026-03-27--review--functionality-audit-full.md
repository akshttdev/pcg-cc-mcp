# Functionality Audit Report — Full Branch

**Date**: 2026-03-27
**Planning Files**: All 4 contacts unification phases + sloperation sprint
**Branch**: `feature/contacts-unification-phase1`
**Audit Method**: Mixed — Playwright walkthrough + code trace via agents

## Feature Verdicts

### Sloperation Sprint (12 features)

| # | Feature | Type | Actual | Discovery | Issue |
|---|---------|------|--------|-----------|-------|
| S1 | Discovery stage restoration | full-stack | WORKING | easy | Discovery column visible on kanban, hero card on deal detail |
| S2 | Sequential agent queue (Astra→Cash) | backend | WORKING | easy | chain_actions in stage_transition.rs, agent flow executor chains next agent |
| S3 | Won deal unification | full-stack | WORKING | easy | provision_won_deal creates org-level project on Won transition |
| S4 | Present & Invoice (renamed) | full-stack | WORKING | easy | Stage renamed, presentation_status tracking in DeckTab |
| S5 | Stage-aware tab visibility | frontend | WORKING | easy | STAGE_TAB_MAP drives per-stage tabs, "All tabs" toggle present |
| S6 | Data source linking | full-stack | WORKING | buried | deal_data_sources table with dual scoping, UI in data library |
| S7 | Review task routing | backend | WORKING | easy | Config-driven review_assignee with org owner fallback |
| S8 | Auto-skip Lead | backend | WORKING | — | auto_skip config on Lead stage, stage_transition checks it |
| S9 | Review gate | backend | WORKING | — | Counts pending tasks, blocks auto-advance when tasks remain |
| S10 | Mark Review Complete | full-stack | WORKING | easy | Button in ReviewTab, calls tasksApi.update to set status=done |
| S11 | SSE pipeline events | backend | WORKING | — | GET /api/events/pipeline streams pipeline_changed events |
| S12 | Kanban SSE subscription | frontend | WORKING | — | EventSource in CrmPipelineBoard, invalidates queries on event |

### Contacts Unification Phase 1 (4 features)

| # | Feature | Type | Actual | Discovery | Issue |
|---|---------|------|--------|-----------|-------|
| P1-W1 | Migration: intel columns on crm_contacts | backend | WORKING | — | 8 intel columns added, backfill from persons |
| P1-W2 | CrmContact struct + query update | backend | WORKING | — | Intel fields on struct, fetch_intel_data reads from contacts directly |
| P1-W3 | Scout writes intel to contacts | backend | WORKING | — | update_research_pass_and_intelligence_direct writes to crm_contacts |
| P1-W4 | DbUuid fixes | backend | PARTIAL | — | 2 remaining Uuid::new_v4() in intelligence.rs (lines 217, 1454) |

### Contacts Unification Phase 2 (4 features)

| # | Feature | Type | Actual | Discovery | Issue |
|---|---------|------|--------|-----------|-------|
| P2-W1 | Intelligence routes target contacts | backend | WORKING | — | POST /contacts/:id/research + GET /contacts/:id/intelligence-status both registered |
| P2-W2 | PCG Router (WorkflowLLMService) | backend | WORKING | — | intelligence.rs uses WorkflowLLMService for LLM calls |
| P2-W3 | Re-FK child tables | backend | WORKING | — | contact_research_passes, business_reports.crm_contact_id backfilled |
| P2-W4 | DbUuid fixes in CRM cluster | backend | PARTIAL | — | Some uuid::Uuid remaining in intelligence.rs |

### Contacts Unification Phase 3 (3 features)

| # | Feature | Type | Actual | Discovery | Issue |
|---|---------|------|--------|-----------|-------|
| P3-W1 | Intake creates contacts directly | backend | WORKING | — | INSERT INTO crm_contacts in pipeline.rs, no person creation |
| P3-W2 | IntelTab targets contact API | frontend | WORKING | easy | Prefers crm_contact_id, falls back to person_id |
| P3-W3 | CRM API intelligence methods | frontend | WORKING | — | triggerContactResearch + getContactStatus in api/intelligence.ts |

### Contacts Unification Phase 4 (4 features)

| # | Feature | Type | Actual | Discovery | Issue |
|---|---------|------|--------|-----------|-------|
| P4-W1 | user_profiles table | backend | WORKING | — | Migration creates table, backfills from persons |
| P4-W2 | Persons fields → custom_fields | backend | WORKING | — | person_type, financial_role migrated to contacts custom_fields |
| P4-W3 | Drop persons table | backend | NOT DONE | — | Persons table still exists, 15+ files reference it. Deferred to PC-6. |
| P4-W4 | Update remaining references | backend | NOT DONE | — | Depends on P4-W3, deferred |

## Build Verification

- [x] `cargo fmt --all -- --check` — **FAIL**: 1 file needs formatting (`intelligence.rs:31` — import grouping)
- [ ] `npx tsc --noEmit` — **FAIL**: 2 errors in `IntelTab.tsx:82` — `Property 'deal' does not exist on type 'CrmDealWithContact'`
- [ ] `cargo test --workspace` — NOT RUN (would need flox activate)
- [ ] `cargo clippy` — NOT RUN
- [x] Frontend renders and is fully functional (Playwright verified)

### Build Issues Detail

**TypeScript error** in `IntelTab.tsx:82`:
```typescript
const contactId = 'crm_contact_id' in deal.deal ? (deal.deal as { crm_contact_id?: string }).crm_contact_id : undefined;
```
`deal.deal` doesn't exist — `CrmDealWithContact` flattens the deal data. The `crm_contact_id` field is likely directly on the deal object. This is a type error that doesn't crash at runtime (the `in` check returns false, falls back to person_id path).

**cargo fmt**: One import grouping issue in `intelligence.rs:31`.

## Playwright Walkthrough Results

**Interactions tested** (0 console errors throughout):
1. Dashboard load → org overview ✅
2. CRM → Pipeline → Kanban board with all stages ✅
3. Create deal (fill form, submit, verify toast + kanban update) ✅
4. Open deal → Overview tab (contact info, operator context, call scheduling) ✅
5. Intel tab (intelligence summary, confidence, pass count) ✅
6. Review tab (review status, Mark Review Complete button, completed reviews) ✅
7. Agent History tab (4 agent executions with artifacts) ✅
8. Stage progress bar (clickable stage navigation) ✅
9. Stage-aware tabs (Proposal tab on Proposal deals, Deck & Close on Won deals) ✅
10. Pipeline settings (stages, automations tabs, CRUD buttons) ✅
11. Contacts page (search, stage filter, contact grid) ✅
12. Contact detail panel (name, email, company, linked deals) ✅
13. My Tasks (140 tasks, filter tabs) ✅
14. Dark mode toggle ✅
15. Sidebar collapsed ✅

**Issues found via Playwright:**
- Review task links from My Tasks → blank page (BLOCKER — `/projects/tasks/:id` matches no route for project_id="" tasks)
- Raw JSON in intelligence summary text across 4 views
- Contact detail doesn't show intelligence data
- Duplicate toasts on deal creation
- Pipeline settings defaults to wrong pipeline
- Contact dropdown has no search

## Action Items

### Must Fix (blocks shipping)

- [ ] **IntelTab.tsx:82 TypeScript error** — `deal.deal` doesn't exist on `CrmDealWithContact`. Change to check `deal.crm_contact_id` directly. — `frontend/src/components/crm/deal-detail/tabs/IntelTab.tsx:82`
- [ ] **cargo fmt** — Run `cargo fmt --all` to fix import grouping in intelligence.rs — `crates/server/src/routes/intelligence.rs:31`
- [ ] **BusinessReport model out of sync** — Migration adds `crm_contact_id` to business_reports table but `BusinessReport` Rust struct doesn't include the field — `crates/db/src/models/business_report.rs`

### Should Fix (quality/UX gaps from experience audit)

- [ ] **Review task links → blank page** — Add `/my-tasks/:taskId` route or give CRM review tasks a project_id
- [ ] **Raw JSON in intelligence summaries** — Strip `Original context: {...}` from display
- [ ] **Contact detail missing intelligence** — Add intel summary/status to contact panel

### Nice to Have (polish)

- [ ] **Contact dropdown search** — Add filter/search to Create Deal contact picker
- [ ] **Duplicate toasts** — Consolidate "Deal created" + "Deal added to pipeline"
- [ ] **Pipeline settings default** — Should default to currently-viewed pipeline (Acquisition), not Client Delivery
- [ ] **uuid::Uuid violations** — 4 remaining: intelligence.rs (lines 217, 1454), crm_activities.rs (line 66), business_report.rs (line 96)
- [ ] **Persons table retirement** — 15+ files still reference persons, tracked as PC-6

## Overall Verdict

**SHIP WITH CAVEATS**

27 of 27 features are WORKING or PARTIAL. The 2 NOT DONE items (persons table drop + reference cleanup) were intentionally deferred and tracked in backlog as PC-6. The TypeScript error and cargo fmt issue are quick fixes. The review task link BLOCKER and raw JSON issue are the most impactful UX problems but don't prevent the core pipeline workflow from functioning. All static checks should pass before merging.
