# Pipeline Workflow Enhancement — Stages, Intelligence & Conversation Persistence

**Date**: 2026-03-15
**Branch**: `tech-debt/sprint-2026-03-15`
**Type**: plan

## Steps

1. **Update pipeline stages** — `crm_pipeline.rs` + migration
2. **Conversation persistence** — Wire `AgentConversation` into `nora.rs` and `topsi.rs` chat handlers
3. **Auto-trigger research** — Stage-aware hooks in `move_deal_stage()`
4. **Advance endpoint** — `POST /api/crm/deals/:id/advance`
5. **Deal card enrichment** — Stage-aware badges + company intel
6. **Detail panel improvements** — Stage stepper, review tab, enhanced intel tab

## Files

- `crates/db/src/models/crm_pipeline.rs` — 8-stage clients pipeline
- `crates/db/migrations/20260402000000_update_clients_pipeline_stages.sql`
- `crates/server/src/routes/nora.rs` — conversation persistence
- `crates/server/src/routes/topsi.rs` — conversation persistence
- `crates/server/src/routes/crm_deals.rs` — stage hooks + advance endpoint
- `crates/server/src/routes/intelligence.rs` — link research to CRM deals
- `crates/db/src/models/crm_deal.rs` — add company_intelligence_status
- `frontend/src/components/crm/CrmDealCard.tsx` — stage-aware badges
- `frontend/src/components/crm/CrmDealDetailPanel.tsx` — stepper + review tab
- `frontend/src/hooks/useDealRich.ts` — new React Query hook
- `frontend/src/lib/api.ts` — advanceDeal + getDealRich
- `frontend/src/types/crm.ts` — company_intelligence_status
