# Demo Sprint: Data Source → Full CRM Pipeline (5 days)

**Date**: 2026-03-27
**Branch**: `feature/2026-03-27--demo-sprint`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `feature/contacts-unification-phase1` (all 4 phases included)

## Context

Demo the full CRM workflow: data source upload → contact/company/deal extraction → pipeline automation → Won with real LLM execution. Target: internal team (Bodhi/Sirak) for daily dogfood use.

**Merge distance**: 223 commits ahead of main, 1 behind. Large delta — factor merge conflict risk.

---

## Day 1: Real LLM Foundation (~8h)

### W1: Verify contacts-unification ✅ DONE
- [x] Run `/check` — all 5 CI checks pass (2 clippy fixes applied: dead fn in intelligence.rs, map_or in stage_transition.rs)
- [x] Eliminate persons-table queries from deal enrichment (list_enriched_deals, get_deal_rich, kanban)
- [x] Fix advance_deal() intel gate — reads from crm_contacts directly
- [x] Fix get_advance_requirements() — same pattern
- [x] Add report_id_for_deal() — queries business_reports by crm_deal_id
- [x] MCP verification: pipeline board renders, deal detail loads, Intel tab works
- [ ] Verify E2E pipeline tests pass (headed mode) — deferred, needs full seed DB

### W1b: Simulation-first pipeline ✅ DONE
- [x] SIMULATE_LLM guards on run_research_direct() and run_company_research_direct()
- [x] Thread flow_id through call_llm chain → save_artifact in all 4 agent simulation branches
- [x] Deal creation runs full process_transition() (not just Intel-specific hook)
- [x] Review tasks start as "waiting" when agent is active, promoted to "todo" on completion
- [x] MCP verified: deal created in Intel → Scout agent scheduled → card shows "scout pending" (not "Needs review")

### W2: Add workflow templates ✅ DONE (prior commit adccb389f)
- [x] sales_conversation_workflow, prospect_list_workflow, company_research_workflow
- [x] All 3 added to seed_defaults()

### W3: Enable real LLM execution (~4-6h, RISKIEST)
- [ ] Set `SIMULATE_LLM=0` in `.env` (currently `=1` at line 58)
- [x] **DONE**: Tool usage instructions added to `build_agent_prompt()` (commit 4ac25286d)
- [ ] Test each agent individually: Scout, Astra, Cash, Lux
  - Done criteria: agent produces >100 chars coherent output + calls ≥1 tool correctly
- [ ] Verify tool-call loop works (`agent_flow_executor.rs:348-398`, max 5 turns)
- [ ] Review hardcoded parameters: `max_tokens=4096`, `temperature=0.7` (`agent_flow_executor.rs:357-358`)
  - Consider: Scout (research) may need higher max_tokens; Cash (proposal) may need lower temperature
- [ ] Fix hardcoded model fallback at line 269: `Some("claude-sonnet-4-6-20250514")` — verify model ID matches DB
- **Files**: `crates/server/src/agent_flow_executor.rs` (lines 269, 336-403, 1258-1303)
- **Risk**: Real LLM may produce responses that break tool-call parsing. Mitigation: keep simulation as comparison baseline.

**Available LLM models** (from pcg_router_models):
| Model | Provider | Priority | Enabled |
|-------|----------|----------|---------|
| Claude Sonnet 4.6 | anthropic | default | ✓ |
| Claude Opus 4.6 | anthropic | — | ✓ |
| Claude Haiku 4.5 | anthropic | — | ✓ |
| GPT-4o | openai | — | ✓ (needs OPENAI_API_KEY) |

---

## Day 2: Data Source → CRM Extraction Demo (~6h)

### W5a: End-to-end extraction with real LLM (~3h)
**NOTE**: Workflow engine (`workflow_engine.rs`) does NOT check SIMULATE_LLM — it always calls real LLM via WorkflowLLMService. Data extraction already uses real LLM when API key is set.
- [ ] Upload test document (conversation transcript from `workflow-crm-pipeline.spec.ts` test data)
- [ ] Run "Data Source Analysis" workflow → LLM extracts contacts/companies/deals
- [ ] Review staging records → approve/edit → commit to CRM (`workflow_staging.rs` batch_commit at line 309)
- [ ] Verify data source linked to created deals (`deal_data_sources` table)
- [ ] Verify extracted contacts have intelligence fields populated
- **Files**: `crates/server/src/routes/data_source_workflows.rs`, `workflow_engine.rs`, `workflow_staging.rs`

### W5b: Extraction quality tuning (~3h)
- [ ] Tune extraction prompts for accurate field extraction (default prompt at `data_source_workflows.rs:104-111`)
  - Ensure deals have: name, amount, description, pipeline assignment
  - Ensure contacts have: first_name, last_name, email, company_name
- [ ] Add JSON schema enforcement in extraction prompts
- [ ] Test with 3 document types: conversation transcript, prospect list, company article
- **Files**: `crates/server/src/routes/data_source_workflows.rs` (node prompt_template fields)

---

## Day 3: Full Pipeline Demo + Won Chain (~7h)

### W4: Fix Won chain test (~2h)
- [ ] Fix DL-3 delivery deal lookup in `e2e/pipeline/pipeline-flow.spec.ts:406-408`
  - Current: imprecise filter by name + pipeline_id
  - Fix: use `crm_contact_id` + delivery pipeline_id, add wait loop for async creation
- [ ] Fix test cleanup — cascade delete delivery deals, projects, clients
  - Current cleanup (`dealflow-pipeline-demo.spec.ts:492-512`) doesn't cascade
- **Files**: `e2e/pipeline/pipeline-flow.spec.ts`, `e2e/demos/dealflow-pipeline-demo.spec.ts`

### W6: Pipeline walkthrough with real agents + review tasks (~4h)
- [ ] Create deal from extraction → enters pipeline at Lead
- [ ] Move to Intel → Scout runs (real LLM) → writes intelligence to `crm_contacts`
- [ ] Review task appears in My Tasks (assigned via `resolve_review_assignee()` at `crm_deal_transitions.rs:702`)
- [ ] User reviews Scout output in Intel tab → marks review complete
- [ ] Deal auto-advances: Intel → BA (Astra) → Proposal (Cash) → Polish (Lux)
- [ ] At each stage: review task → user reviews → marks complete → advances
- [ ] Continue through Invoice → Negotiation → Won
- [ ] Won triggers: delivery deal + client + project + tasks (via `provision_won_deal()` at `crm_deal_automations.rs:1182`)
- [ ] SSE events update kanban board in real-time (`CrmPipelineBoard.tsx:146-151`)

### W7: My Tasks integration (~1h)
- [ ] Verify review tasks appear on `/my-tasks` for logged-in user
- [ ] Verify clicking a review task navigates to deal detail
- [ ] Verify completing a review task from My Tasks unblocks pipeline advance
- **Files**: `frontend/src/pages/my-tasks.tsx` (454 lines, queries `tasksApi.getAssignedToMe()`)

---

## Day 4: UX Polish + E2E Cleanup (~6h)

### W9: Pipeline UX polish — top 3 items only (~4h)
- [ ] Agent status badge visibility — ensure running/complete/failed states are clearly distinguishable
- [ ] Stage transition feedback — toasts with agent names + progress indicators
- [ ] Deal card information density — amount, probability, badges, agent status visible at glance
- **Files**: `frontend/src/components/crm/CrmDealCard.tsx` (360 lines), `frontend/src/components/crm/deal-detail/`
- **Verify via Playwright MCP**: snapshot pipeline board, verify card layout changes

### W10: E2E cleanup (~2h)
- [ ] Consolidate redundant tests — keep unique tests from deal-lifecycle, deal-detail, agent-automations
- [ ] Remove stale/broken demo specs
- [ ] Update working demo specs for contacts-unification changes
- [ ] Target: fewer total tests, higher coverage, zero fixmes on core path
- **Files**: `e2e/pipeline/*.spec.ts`, `e2e/demos/*.spec.ts`

---

## Day 5: CI + Final Demo + Documentation (~6h)

### W11: E2E tests in CI (~3h)
- [ ] Add Playwright E2E job to `.github/workflows/ci.yml`
- [ ] Run on `main` push only (not feature branches)
- [ ] Strategy: start backend as background process, wait for health check, run tests
- [ ] Use test seed DB (`scripts/create-test-seed.sh`), headless Chromium
- [ ] Pipeline tests only (exclude demos which require headed mode)
- **Files**: `.github/workflows/ci.yml`, `playwright.config.ts`

### W12: End-to-end demo run-through (~2h)
- [ ] Full demo flow via browser:
  1. Upload conversation transcript to Data Library
  2. Run extraction workflow → verify staged contacts/companies/deals
  3. Commit to CRM → verify deals in pipeline
  4. Watch agents process deals through pipeline (real LLM)
  5. Review and approve at each stage via My Tasks
  6. Deal reaches Won → delivery deal + client + project created
- [ ] Record issues, fix immediately

### W13: Documentation (~1h)
- [ ] Update `docs/PIPELINE.md` with real LLM config + demo setup
- [ ] Write demo script for repeatable walkthroughs

---

## Key Files

| Area | File | Lines | Key Functions |
|------|------|-------|---------------|
| Agent executor | `crates/server/src/agent_flow_executor.rs` | 1,411 | `build_agent_prompt():1258`, `call_llm_once():336`, `execute_tool_call():406` |
| Stage transitions | `crates/server/src/stage_transition.rs` | 914 | `process_transition():140`, `handle_won_transition():627`, auto_skip:360 |
| Won provisioning | `crates/server/src/routes/crm_deal_automations.rs` | 1,685 | `provision_won_deal():1182` |
| Review routing | `crates/server/src/routes/crm_deal_transitions.rs` | 747 | `resolve_review_assignee():702`, review task creation:355 |
| Workflow defs | `crates/server/src/routes/data_source_workflows.rs` | 1,702 | `seed_defaults():486`, 5 system workflows |
| Workflow engine | `crates/server/src/routes/workflow_engine.rs` | 705 | `execute_workflow_nodes():95` |
| Staging + commit | `crates/server/src/routes/workflow_staging.rs` | 1,400 | `batch_commit():309`, `commit_contact():450` |
| PCG Router | `crates/server/src/routes/pcg_router.rs` | 622 | `route_completion():236`, 10 providers |
| LLM Service | `crates/services/src/services/workflow_llm.rs` | 700+ | `completion_with_tools():167` |
| Intel tab | `frontend/src/components/crm/deal-detail/tabs/IntelTab.tsx` | 388 | Research trigger, report review |
| Pipeline board | `frontend/src/components/crm/CrmPipelineBoard.tsx` | 502 | SSE subscription:146, DnD |
| Deal cards | `frontend/src/components/crm/CrmDealCard.tsx` | 360 | 9-row card layout |
| Data source UI | `frontend/src/pages/data-source-detail.tsx` | 690 | Workflow dropdown, staging review |
| My Tasks | `frontend/src/pages/my-tasks.tsx` | 454 | Assigned/Created/Watching tabs |
| Deal detail | `frontend/src/components/crm/deal-detail/` | 4,263 (18 files) | Tabs, actions, overview |

## Decisions Needed

1. **Default model for agents**: Sonnet 4.6 (fast/cheap, ~$3/M input) or Opus 4.6 (quality, ~$15/M input)?
2. **W9 UX polish**: Which 3 items are highest priority? Each could take 2-4h.
3. **W11 CI strategy**: Background process + health check, or Docker service container?
4. **Test documents for W5**: Reuse spec test data, or create new realistic transcripts?

## Verification Checklist

1. **Contacts-unification**: Scout writes to crm_contacts, Intel tab shows results
2. **Real LLM**: Each agent produces quality output, tool calls work, output persists
3. **Data extraction**: Upload doc → workflow → staging → CRM with linked data sources
4. **Pipeline flow**: Deal through Lead→Won with review tasks at each agent stage
5. **My Tasks**: Logged-in user sees review tasks, can complete from My Tasks page
6. **Won chain**: Delivery deal + client + project + tasks created
7. **SSE**: Kanban board updates in real-time as agents complete
8. **E2E suite**: Pipeline tests pass with 0 failures
9. **Demos**: Demo specs pass end-to-end with real LLM
10. **CI**: E2E tests run on main push, all green

## Cut Line

**Must-have** (Days 1-3): W1, W2, W3, W4, W5, W6, W7
**Nice-to-have** (Days 4-5): W9, W10, W11, W12, W13 — cut if time short

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Real LLM prompt quality — agents don't call tools | HIGH | Add tool instructions to prompts (W3 critical fix) |
| Extraction output format — LLM doesn't produce expected JSON | MEDIUM | Add JSON schema enforcement in prompts (W5b) |
| Won chain test failure (DL-3) | LOW | Known fix: better lookup logic (W4) |
| CI E2E setup complexity | LOW | Well-documented patterns exist |
| Merge conflicts with main (223 commits ahead) | MEDIUM | Merge main before PR |

---

## Regression Test & QA Results (2026-03-31)

### Summary
- Branch: `feature/2026-03-27--demo-sprint` vs `main`
- Commits ahead: 249 | Files changed: 512 | +32,806 / -14,069
- Critical: 2 | High: 6 | Medium: 6 | Low: 4

### Static Checks

| Check | Status | Notes |
|-------|--------|-------|
| cargo fmt | **PASS** | Clean |
| cargo clippy | **ENV BLOCKED** | `glib-sys`/`openssl-sys` build failure — needs flox activation fix; must verify in CI |
| tsc | **PASS** (branch) | 3 errors in untracked `KnowledgeGraphViz.tsx` — pre-existing, not in branch |
| eslint | **PASS** (branch) | 2 errors in `error-boundary-fallback.tsx` — pre-existing, not in branch |
| generate-types | **PASS** | `shared/types.ts` is up to date |
| Conflict markers | **PASS** | None found |
| Dead imports | **PASS** | None found |

### Critical — Must Fix Before Merge

| # | Issue | Location | Fix |
|---|-------|----------|-----|
| C1 | **Auth bypass on SSE pipeline events** — `stream_pipeline_events()` accepts any `org_id` query param with no `AccessContext` validation; any user can spy on any org's deal events | `crates/server/src/routes/pipeline_events.rs:27–79` | Add `Extension(access_context)` + `require_org_membership()` check |
| C2 | **Status string typo** — code checks `s == "complete"` but enum serializes as `"completed"`; flow completion silently misdetected | `crates/server/src/stage_transition.rs:868,887` | Use `FlowStatus` enum, fix typo |

### High — Should Fix Before Merge

| # | Issue | Location | Fix |
|---|-------|----------|-----|
| H1 | **Race condition on flow completion** — UPDATE to `completed` has no `AND status = 'executing'` guard; concurrent executors can double-complete | `crates/server/src/agent_flow_executor.rs:~805` | Add expected-status WHERE clause, check `rows_affected()` |
| H2 | **Race condition on flow failure** — same pattern; failure UPDATE has no status guard | `crates/server/src/stage_transition.rs:920` | Add `AND status IN ('planning', 'executing')` |
| H3 | **Auth audit needed on data_source_workflows.rs** — new route file, unclear if all routes check `require_org_membership()` | `crates/server/src/routes/data_source_workflows.rs` | Audit all routes for `AccessContext` + org membership check |
| H4 | **AgentFlowSummary.current_phase undefined** — frontend interface requires field, but backend returns `serde_json::Value` with no type enforcement | `frontend/src/components/crm/deal-detail/tabs/AgentHistoryTab.tsx:33–48` | Define Rust struct + ts-rs derive, or add runtime validation |
| H5 | **`as string` cast on `customFields.discovery_call_status`** — typed as `unknown`, cast without guard | `frontend/src/components/crm/deal-detail/tabs/OverviewTab.tsx:~52` | Add `typeof === 'string'` guard |
| H6 | **CallSchedulingSection form closes on mutation failure** — user loses data silently | `frontend/src/components/crm/deal-detail/tabs/OverviewCallSchedulingSection.tsx:32–43` | Re-open form in `onError` handler |

### Medium — Track for Follow-up

| # | Issue | Location |
|---|-------|----------|
| M1 | Silent error drops (`let _ =`) on task/review creation | `stage_transition.rs`, `crm_deal_automations.rs` |
| M2 | Empty string defaults (`unwrap_or("")`) masking missing flow config | `agent_flow_executor.rs:227,514` |
| M3 | JSON stage_config has no schema validation — breakage is silent | `agent_flow_executor.rs` |
| M4 | `text-[10px]` → `text-xs` (10px→12px) may cause badge/pill reflow | Multiple frontend files |
| M5 | Stage-tab visibility falls back to ALL_TABS for unknown stages | `frontend/src/components/crm/deal-detail/stage-tab-config.ts:38–44` |
| M6 | Missing accessibility labels on call scheduling inputs | `OverviewCallSchedulingSection.tsx:75–88` |

### Migration Assessment

| Migration | Risk | Notes |
|-----------|------|-------|
| `20260416000000_normalize_blob_uuids_to_text` | **HIGH** | Irreversible; 13 tables; requires DB backup before deploy |
| `20260418000001_child_tables_to_contacts` | **HIGH** | Drops `person_research_passes` + `person_social_profiles`; orphaned rows (NULL `crm_contact_id`) silently discarded |
| Stage config UPDATEs (×5) | MEDIUM | JSON format overwritten without backup; no rollback path |
| `20260417000000_discovery_stage` | MEDIUM | Position reshift assumes no stage at position >100 |
| All others | LOW | Additive (ADD COLUMN, CREATE TABLE, CREATE INDEX) |

**Pre-deployment checklist:**
- [ ] Full DB backup before running `20260416000000`
- [ ] Audit `persons.crm_contact_id IS NULL` rows before `20260418000001`
- [ ] Validate `max(position) < 100` in `crm_pipeline_stages`
- [ ] Verify clippy passes in CI (local env blocked by openssl-sys)

### API Wiring — All Connected ✅

All 7 new backend routes have frontend consumers:
`cancelDealAgent`, `approveDealAgent`, `retriggerDealAgent`, `generateInvite`, `listDataSources`, `linkDataSource`, `fetchDealAgentFlows`

### Clean Areas ✅

- UUID handling excellent — consistent `DbUuid`, no `uuid::Uuid` in route handlers
- No SQL injection — all queries parameterized
- Error propagation consistent with `?` + `ApiError`
- Migration ordering correct, no timestamp conflicts
- No conflict markers, no dead imports
- CI config unchanged, no suppressed lints

## Merge Recommendation

| Gate | Status |
|------|--------|
| Critical issues (C1, C2) | ❌ Must fix — auth bypass + status typo |
| High issues (H1–H6) | ⚠️ Should fix |
| Auth / security | ❌ C1 blocks merge |
| Type safety | ⚠️ H4, H5 should be fixed |
| Migration safety | ⚠️ Requires backup plan + orphan audit |
| Clippy | ⏳ Must verify in CI (env blocked locally) |

**Recommendation: NEEDS FIXES** — fix C1 (auth bypass) and C2 (status typo) before merge. H1–H3 (race conditions + auth audit) strongly recommended. All fixable in a single commit.
