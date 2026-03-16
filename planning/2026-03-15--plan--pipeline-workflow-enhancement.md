# Pipeline Workflow Enhancement — Stages, Intelligence & Conversation Persistence

**Date**: 2026-03-15
**Branch**: `feature/pipeline-workflow-enhancement`
**Type**: plan
**Status**: COMPLETE — merged from `sloperation314`, QA'd, E2E demo written

## Original Steps (all implemented on sloperation314)

1. **Update pipeline stages** — `crm_pipeline.rs` + 3 migrations (clients, sales, deal-person linking)
2. **Conversation persistence** — Wire `AgentConversation` into `nora.rs` and `topsi.rs` chat handlers
3. **Auto-trigger research** — Stage-aware hooks in `move_deal_stage()`
4. **Advance endpoint** — `POST /api/crm/deals/:id/advance`
5. **Deal card enrichment** — Stage-aware badges + company intel
6. **Detail panel improvements** — Stage stepper, review tab, enhanced intel tab

## Merge & Integration (2026-03-15)

Merged `sloperation314` into `feature/pipeline-workflow-enhancement`. Post-merge fixes:

- **DbUuid type mismatches**: 12 Rust compile errors in `crm_deals.rs` — sloperation314 used `Uuid` but feature branch had migrated to `DbUuid`. Fixed all function signatures (`trigger_who_is_research`, `trigger_company_research_if_idle`, `create_review_task_if_needed`, `advance_deal`).
- **TS `collaborators` field**: 4 TypeScript errors — `CreateTask` type gained a required `collaborators` field. Added `collaborators: null` to `ProjectFormDialog`, `TaskFormDialog` (2 locations), `ImportDialog`.
- **Dead code cleanup**: Commented out unused `research_prompt` format string in `intelligence.rs` (was built but never used — `run_research_direct` constructs its own prompt).

## E2E Demo

New file: `e2e/demos/pipeline-intelligence-workflow.spec.ts`
- 5-part serial test covering full 8-stage deal progression
- API-driven deal creation and stage advancement
- UI verification on pipeline board
- Validates review task creation, advance blocking, delivery deal auto-creation

## Backlog Items Resolved

- **Rename "Bug Triage Pipeline"** → "Feedback Triage Pipeline" (code + E2E tests)
- **ACP Agents MCP Servers** — already implemented (load_platform_mcp_servers + default_mcp.json)
- **Worktree Manager Deadlock Fix** — already implemented (poison recovery + 30s timeout)
- **Frontend Error Boundaries** — already implemented (PageErrorBoundary in AppShell + org-profile tabs)

## Files Changed

### From sloperation314 merge (17 files)
- `crates/db/migrations/20260402000000_update_clients_pipeline_stages.sql` — 8-stage Clients pipeline
- `crates/db/migrations/20260403000000_update_sales_pipeline_stages.sql` — Sales pipeline variant
- `crates/db/migrations/20260404000000_link_intake_deals_to_persons.sql` — Deal-person linking
- `crates/db/src/models/crm_deal.rs` — company_intelligence_status on CrmDealWithContact
- `crates/db/src/models/crm_pipeline.rs` — Updated create_clients_pipeline()
- `crates/server/src/routes/crm_deals.rs` — Stage-aware hooks + advance endpoint
- `crates/server/src/routes/intelligence.rs` — Auto-link research to CRM deals
- `crates/server/src/routes/nora.rs` — Conversation persistence
- `crates/server/src/routes/topsi.rs` — Conversation persistence
- `frontend/src/components/crm/CrmDealCard.tsx` — Stage-aware badges
- `frontend/src/components/crm/CrmDealDetailPanel.tsx` — Stage stepper + ReviewTab
- `frontend/src/components/crm/CrmPipelineBoard.tsx` — Reworked board layout
- `frontend/src/hooks/useDealRich.ts` — React Query hook for rich deal data
- `frontend/src/lib/api/crm.ts` — advanceDeal() endpoint
- `frontend/src/types/crm.ts` — company_intelligence_status field

### Post-merge fixes (9 files + 1 new)
- `crates/server/src/routes/crm_deals.rs` — DbUuid type fixes
- `crates/server/src/routes/intelligence.rs` — Dead code cleanup
- `crates/server/src/routes/data_source_workflows.rs` — Rename to Feedback Triage
- `crates/server/src/routes/feedback.rs` — Comment update
- `frontend/src/components/dialogs/projects/ProjectFormDialog.tsx` — collaborators field
- `frontend/src/components/dialogs/tasks/TaskFormDialog.tsx` — collaborators field
- `frontend/src/components/export/ImportDialog.tsx` — collaborators field
- `e2e/dogfood-workflows.spec.ts` — Feedback Triage rename
- `e2e/workflow-pipeline.spec.ts` — Feedback Triage rename
- `e2e/demos/pipeline-intelligence-workflow.spec.ts` — NEW: 8-stage pipeline E2E demo
