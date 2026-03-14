# DbUuid Phase 3: Remaining Model Conversions

**Source:** Extracted from `2026-03-14--plan--dbuuid-hybrid-decode.md` (archived)
**Prerequisite:** Phase 1 (DbUuid type) and Phase 2 (Agent + FK references) — both complete
**Priority:** Low-risk, high-volume mechanical work. Can be batched by domain.

---

## Scope

Convert the remaining ~107 models from `Uuid` → `DbUuid`. All use the same pattern.

## Per-Model Checklist

1. `pub id: Uuid` → `pub id: DbUuid`
2. All `Option<Uuid>` FK fields → `Option<DbUuid>`
3. Replace `.as_bytes().as_slice()` → `&field` (Deref handles it)
4. Replace `.as_ref().map(|u| u.as_bytes().to_vec())` → `field.as_ref().map(|u| u.as_str())`
5. Replace `Uuid::new_v4()` → `DbUuid::new()`
6. Update `query_as!()` type casts
7. Run `cargo sqlx prepare --workspace`

## Known `.as_bytes()` Sites

- `user.rs` (lines 212, 226-227)
- `person.rs` (lines 187, 272, 286, 330, 344, 361, 376, 407-408, 424)
- `project_knowledge_source.rs` (lines 124, 150-151, 184, 199, 214)
- `org_brand_profile.rs` (lines 109, 136-137)
- `activity.rs` (line 156)

## Skipped from Phase 2 (Priority Targets)

- `token_usage.rs` — `agent_id` FK join field
- `crm_contact.rs` — `assigned_agent_id` field
- `routes/twilio.rs` — `Uuid::from_slice()` workaround for agent lookup
- `sovereign_storage.rs` — BLOB ID → JSON serialization
- `agent_conversation.rs` — `agent_id: Uuid` (currently bridged with `Uuid::parse_str` in agent_chat.rs)

## Suggested Batching

1. **Agent-adjacent** (5 files above) — removes all Phase 2 bridge code
2. **CRM models** — contacts, deals, pipelines, pipeline stages
3. **Workflow models** — definitions, executions, staging
4. **Core models** — users, projects, orgs, tasks, activities
