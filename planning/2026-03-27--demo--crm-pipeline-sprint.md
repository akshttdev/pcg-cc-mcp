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

### W1: Verify contacts-unification ✅ CI PASSES (~1h)
- [x] Run `/check` — all 5 CI checks pass (2 clippy fixes applied: dead fn in intelligence.rs, map_or in stage_transition.rs)
- [ ] Test Scout writing intelligence to `crm_contacts` (Phase 1)
- [ ] Test Intel tab reading from contacts directly (Phase 3) — `IntelTab.tsx:81` prefers `crm_contact_id`
- [ ] Test PCG Router model routing (Phase 2) — `pcg_router.rs:236` route_completion()
- [ ] Verify E2E pipeline tests pass (headed mode)

### W2: Add workflow templates (~2h)
System workflows already persist via `seed_defaults()` (INSERT OR REPLACE in `data_source_workflows.rs:486-509`). 5 templates exist. Add 3 new ones:
- [ ] Add `sales_conversation_workflow()` function — extract prospects from call transcripts
- [ ] Add `prospect_list_workflow()` function — parse CSV/text list of potential clients
- [ ] Add `company_research_workflow()` function — extract company profile from article
- [ ] Add all 3 to `seed_defaults()` array
- [ ] Restart server → verify templates appear in data-source-detail workflow dropdown
- **Files**: `crates/server/src/routes/data_source_workflows.rs` (add functions near line 398, update seed_defaults at line 486)
- **No migration needed** — seed_defaults() runs at startup

### W3: Enable real LLM execution (~4-6h, RISKIEST)
- [ ] Set `SIMULATE_LLM=0` in `.env` (currently `=1` at line 58)
- [ ] **CRITICAL FIX**: Add tool usage instructions to `build_agent_prompt()` (`agent_flow_executor.rs:1258`)
  - Each agent prompt must describe available tools: `get_deal_context`, `update_deal_field`, `save_artifact`
  - Without this, real LLM will produce text-only responses (no tool calls)
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
