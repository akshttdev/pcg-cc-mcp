# Deployment Guide: Sloperation Intent Sprint

**Branch**: `feature/2026-03-24--sloperation-sprint`
**Base**: `feature/2026-03-19--tier1-phase0` + `pr/59-pipeline-e2e-agent-autoadvance`
**Date**: 2026-03-24

---

## Pre-Deployment Checklist

- [ ] All commits pushed to remote
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy` passes (with CI `-A` flags)
- [ ] `npx tsc --noEmit` passes
- [ ] `npm run generate-types:check` passes
- [ ] E2E pipeline tests pass (`npx playwright test e2e/pipeline/ --headed`)

---

## Database Migrations (5 new)

Applied in order by `sqlx migrate run`:

| Migration | What It Does |
|-----------|-------------|
| `20260417000000` | Adds Discovery stage (position 3) to Sales pipelines missing it. Shifts subsequent stages. Updates Discovery stage_config with `require_transcript_or_source` exit validation. |
| `20260417000001` | Updates Proposal stage_config: sequential agent queue (Astra deep_research first, Cash proposal second). |
| `20260417000002` | Renames Invoice/Proposal Meeting to "Present & Invoice". Updates stage_config with deck_url exit validation. Handles both `stage_type = 'invoice'` and `stage_type = 'present'`. |
| `20260417000003` | Creates `deal_data_sources` join table with dual scoping (`relevant_stages`, `relevant_agents`). |
| `20260417000004` | Sets `auto_skip: true` on Lead stages. Deals entering Lead immediately advance to Intel. |

**To apply manually:**
```bash
DATABASE_URL="sqlite:dev_assets/db.sqlite" sqlx migrate run --source crates/db/migrations
```

**On dev server restart:** Migrations auto-apply via the flox activation hook.

---

## Environment Variables

No new env vars. Existing required:
```bash
ENABLE_AGENT_FLOW_ENGINE=1   # Background agent worker
SIMULATE_LLM=1               # Simulated responses (no API credits)
```

---

## Breaking Changes

### Behavioral Changes

1. **Auto-advance now blocked by pending tasks**: Agent completion no longer auto-advances if deal-linked tasks (review tasks) are pending. Operators must complete/cancel review tasks in the UI before the deal advances. This was a bug fix — previously review tasks were created but ignored.

2. **Lead stage auto-skips**: Deals entering Lead immediately advance to Intel. Disable via Pipeline Settings → Edit Lead stage → uncheck auto_skip.

3. **Proposal stage triggers two agents**: Astra Pass 2 (deep_research) fires first, then Cash (proposal) chains automatically. Previously only Cash fired.

4. **Won stage triggers full provisioning**: DnD/context menu to Won now creates client + project + tasks + VIBE transaction (same as mark-won API). Previously only created delivery deal.

5. **Stage-aware tab visibility**: Deal detail panel shows only stage-relevant tabs by default. "All tabs" toggle available.

### Stage Changes

```
BEFORE (9 stages)           AFTER (10 stages)
Lead                        Lead (auto_skip → Intel)
Intel                       Intel
Business Analysis           Business Analysis
Proposal                    Discovery (NEW — human stage)
Polish                      Proposal (Astra P2 → Cash chain)
Invoice                     Polish
Negotiation                 Present & Invoice (RENAMED)
Won                         Negotiation
Lost                        Won (full provisioning)
                            Lost
```

### API Changes

New endpoints:
- `POST /api/crm/deals/:id/data-sources` — link data source to deal
- `GET /api/crm/deals/:id/data-sources` — list linked data sources

Modified responses:
- `POST /api/crm/deals/:id/mark-won` now returns `client_name` and `vibe_amount` in response
- `PATCH /api/crm/deals/:id/stage` to Won triggers full provisioning (not just delivery deal)

### Type Changes

New types (auto-generated via `npm run generate-types`):
- `DealDataSource` — join table record
- `LinkDataSourceRequest` — with `relevant_stages` and `relevant_agents`
- `StageConfig.auto_skip` — boolean field
- `StageConfig.review_assignee` — optional string field
- `StageValidation.RequireTranscriptOrSource` — new validation variant

---

## Rollback Plan

1. **Revert to tier1**: `git checkout feature/2026-03-19--tier1-phase0`
2. **DB rollback**: Not easily reversible (new table + inserted stages). Use backup:
   ```bash
   cp dev_assets_seed/test-seed.sqlite dev_assets/db.sqlite
   ```
3. **No data loss**: All changes are additive (new stages, new table, new columns on existing tables).

---

## Post-Deployment Verification

1. Navigate to CRM Pipeline → verify Discovery column appears between BA and Proposal
2. Create a deal → should auto-skip Lead → land at Intel → Scout triggers
3. Let Scout complete + complete review task → deal advances to BA
4. Let Astra complete + complete review task → deal advances to Discovery
5. Manually advance Discovery → Proposal → verify Astra P2 fires first, then Cash chains
6. Verify stage-aware tabs: Lead shows 2 tabs, Intel shows 5, Discovery shows 6
7. Move deal to Won → verify rich toast with client + project + tasks info
8. Open Pipeline Settings → verify Lead has auto_skip enabled

---

## Known Issues (Post-Sprint)

- 5 pipeline-flow E2E tests fail due to Discovery stage in the chain (tests expect old stage order)
- Person invite (F3) still a stub — logs only, no token generation
- Cost bridge (W8) deferred — agent flows don't record token usage yet
- Data source picker is a basic select dropdown — no search/filter
