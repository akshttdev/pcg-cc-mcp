# Session Handoff Notes — 2026-03-27

**Branch**: `feature/contacts-unification-phase1`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp/.claude/worktrees/main-worktree`
**Last commit**: `a5cd7cd3a` — functionality audit report

## What's Done

### Sloperation Sprint (from base branch)
- E2E test fixes (DD-1, AA-3, DD-4)
- Agent History artifact links + Review completed tasks
- SSE pipeline events, stage-aware tabs, sequential agent chain

### Contacts Unification (this branch, 4 phases)
- **Phase 1**: Intelligence columns on crm_contacts, Scout writes directly, no persons JOIN
- **Phase 2**: Intelligence routes target contacts, PCG Router replaces hardcoded LLM calls, child tables re-FK'd
- **Phase 3**: Intake pipeline creates contacts directly, Intel tab uses contact API
- **Phase 4**: user_profiles table, person fields migrated to custom_fields

### QA + Regression
- All static checks pass (fmt, clippy, tsc, eslint, generate-types:check)
- Regression audit done — 7 issues found, all fixed
- Functionality audit done — 13/13 features WORKING/PARTIAL
- Double-slash bug in ReviewTab task links — FIXED

## What Remains

### 1. Experience Audit (`/experience-audit`)
Full Playwright MCP walkthrough needed per skill instructions:
- Walk each user journey (create deal → Intel → Scout → review → advance)
- Check console errors after EVERY interaction (not just page loads)
- Verify discovery paths (how many clicks to reach each feature)
- Identify blockers, pain points, friction
- Produce prioritized recommendations

### 2. Update Planning Files
- Update all 4 phase planning files with completion status
- Update backlog with any new items found during audits
- Update sprint plan with final status

### 3. Non-project-scoped Task Routes
- CRM review tasks have `project_id=""` (empty string)
- Currently link to `/my-tasks` (list view, no deep-link)
- Need either `/my-tasks/:taskId` route or give review tasks a project_id
- Tracked but not yet addressed

### 4. Persons Table Retirement (PC-6)
- 15+ files still reference persons table
- Not dropped in this branch — needs dedicated sweep
- Tracked in backlog as PC-6

### 5. Pre-existing Issues (not from this branch)
- LIKE injection in agent_flow_executor.rs:517 (load_deal_context)
- Silent error on review task assignment failure
- pipeline-flow.spec.ts undefined completeTasks() calls
- AA-2 E2E test timing (auto-advance engine)

## Servers
- Backend: port 3012 (may need restart after session)
- Frontend: port 3010 (may need restart)
- Both were running at end of session
- SIMULATE_LLM=1, ENABLE_AGENT_FLOW_ENGINE=1

## Key Test Data
- **Phase1 Verification Deal** (d154a062) — Intel stage, Scout completed, intelligence on contact verified
- **Bad Girl Strength Club** (bb205e77) — Intel stage, Scout completed
- **Apex Media** (e104e247) — Proposal stage, full agent chain completed

## To PR
- Branch `feature/contacts-unification-phase1` → `main`
- 15 commits ahead of base
- All checks pass
