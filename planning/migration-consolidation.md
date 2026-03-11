# SQLx Migration Consolidation Plan

## Current State
- **178 migration files** in `crates/db/migrations/`
- Spanning June 2025 to March 2026
- Many are small incremental changes (add column, rename, fix constraint)
- Production database has run all 178 migrations

## Goal
Consolidate into ~10 logical baseline files for new environments while keeping production safe.

## Strategy: Baseline Migration Pattern

### How it works
1. **Archive** all 178 migrations into `migrations/_archive/`
2. **Create logical baseline files** that reproduce the full current schema
3. **Bootstrap script** handles both existing and new databases:
   - **Existing DB (production)**: Insert `_sqlx_migrations` rows for baseline files (marking them as already applied). Future migrations run normally.
   - **New DB**: Run baselines only. Insert rows for the 178 archived migrations so SQLx skips them.
4. Future migrations continue as normal after the baseline timestamp.

### Important: dump the schema from production
Reconstruct baselines from the **actual production schema** (not by replaying migration files) to avoid drift from manual changes or migration bugs.

## Proposed Baseline Groups

### 1. Core (projects, tasks, boards, templates)
Tables: `projects`, `tasks`, `task_attempts`, `task_templates`, `project_boards`, `project_pods`, `project_assets`, FTS indexes

### 2. Execution (processes, sessions, logs, worktrees)
Tables: `execution_processes`, `execution_process_logs`, `executor_sessions`, `execution_summaries`, `merges`, `follow_up_drafts`

### 3. Users & Auth (users, permissions, roles, tiers)
Tables: `users`, `project_permissions`, `orcha_tiers`, `user_tiers`

### 4. Organizations & Clients (orgs, clients, folders, hierarchy)
Tables: `organizations`, `clients`, `project_folders`, `org_invites`, `org_addresses`

### 5. CRM (contacts, deals, pipelines, companies, meetings)
Tables: `crm_contacts`, `crm_deals`, `crm_pipelines`, `crm_pipeline_stages`, `companies`, `persons`, `person_associations`, `scheduled_meetings`, `meeting_person_links`

### 6. Workflows & Data Sources (definitions, runs, staging, triggers)
Tables: `data_sources`, `workflow_definitions`, `workflow_runs`, `workflow_output_staging`, `workflow_triggers`, `workflow_executions`

### 7. Agents & AI (agents, models, PCG router, token usage)
Tables: `agents`, `agent_conversations`, `agent_wallets`, `nora_config`, `pcg_router_*`, `token_usage`, `model_pricing`

### 8. Integrations & Media (artifacts, media, social, email, DAM)
Tables: `execution_artifacts`, `media_assets`, `social_platform_*`, `email_accounts`, `cms_*`, `cinematic_briefs`, `media_pipeline_*`

### 9. Knowledge & Entities (entity system, knowledge graph, topology)
Tables: `entities`, `entity_relations`, `knowledge_sources`, `knowledge_sheaves`, `topsi_*`, `orchestration_contexts`

### 10. Seed Data (default users, orgs, templates)
All INSERT statements for default records (admin user, default org, default templates, etc.)

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Production schema drift from migration files | Dump actual schema from production DB |
| Bootstrap script must run before next `sqlx migrate run` | Document clearly; add pre-flight check |
| Checksum mismatches if baseline files change | Generate baselines once, then freeze |
| Seed data may differ between environments | Separate seed baselines from schema baselines |

## Prerequisites Before Starting
- [ ] Dump current production schema (`sqlite3 prod.db .schema`)
- [ ] Verify no pending migrations
- [ ] Snapshot/backup production database
- [ ] Test bootstrap script against a copy of production DB
- [ ] Test fresh DB setup with baselines only
