# Knowledge Graph Blockers — Org SELECT drift + Project Knowledge BLOB/TEXT mismatch

**Date**: 2026-05-16
**Branch**: `fix/knowledge-graph-blockers`
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-kg-fix`
**Base**: `main` (4114b5560)
**Ports**: FRONTEND 3020 / BACKEND 3022 (shares ports with avatar-engine worktree — only one backend runs at a time)
**DB**: symlinked to `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-avatar-engine/dev_assets/db.sqlite` so the dashboard's live DB picks up the fix without re-seeding

## Goal

Restore the **Organizational Knowledge Graph** to functional. Two unrelated pre-existing bugs on `main` are blocking the org Intelligence page and project knowledge views — they affect every project and every org, not just Inessa's new data. This PR fixes both as a tight, surgical change off `main`, intentionally scoped to NOT touch the avatar-engine PR's scope.

Context: discovered while wiring the Inessa Wellness Brand Strategy doc into PCG Org's Knowledge Graph. Same fix is needed for Sirak Studios Master Node work on Knowledge Graph data sync.

## Success criteria

- `GET /api/organizations` returns 200 with all orgs the user can see
- `GET /api/organizations/:id/knowledge` returns 200 with the brand summary + data sources + org-scoped knowledge entries (TIACA's existing seed data should now actually surface for Sirak Studios)
- `GET /api/projects/:id/knowledge` returns 200 with project-scoped knowledge entries (existing MVI artifacts should now surface for their project)
- Project Intelligence page on the org-profile renders the Organization Intelligence badges section + the per-project aggregation
- `/check` passes (cargo fmt, clippy, tsc, eslint, generate-types:check)
- No changes outside `crates/db/src/models/user.rs`, `crates/db/src/models/project_knowledge_source.rs`, and downstream generated TS types
- Existing TIACA + MVI seed data renders correctly without re-seeding

## Out of scope (explicit non-goals)

- Avatar engine paths (separate PR #68)
- Per-user Vibe caps (Phase 4.1 of the avatar-engine plan)
- Migrating `project_knowledge_sources.project_id` from BLOB column declaration to TEXT (the schema declaration stays BLOB; we align the Rust side to match what's actually written to disk by `upsert_source`)
- Adding the `created_by` field to knowledge sources
- Fixing the access control on `/api/organizations/:id/knowledge` (currently has admin bypass + is_member check — fine as-is)
- Re-seeding any data

---

## Bug 1 — Organization SELECT queries missing 3 columns

### Symptom

```
GET /api/organizations  →  500
body: {"message":"DatabaseError: no column found for name: wallet_address"}
```

Downstream effect: org Intelligence page fails to load → "Organization not found" rendered → entire Knowledge Graph view unreachable.

### Root cause

Migration `20260427000001_org_vibe_and_node_ownership.sql` added 3 columns to `organizations`:

```sql
ALTER TABLE organizations ADD COLUMN wallet_address TEXT;
ALTER TABLE organizations ADD COLUMN vibe_balance REAL NOT NULL DEFAULT 0.0;
ALTER TABLE organizations ADD COLUMN vibe_budget_total REAL;
```

The `Organization` struct in `crates/db/src/models/user.rs` was updated to include matching fields:

```rust
pub struct Organization {
    ...
    pub address: Option<String>,
    pub wallet_address: Option<String>,
    #[sqlx(default)]
    pub vibe_balance: f64,
    pub vibe_budget_total: Option<f64>,
}
```

But the explicit-column SELECT statements in the same file were never updated — they stop at `address`. SQLx tries to deserialize `wallet_address` from the result set, doesn't find the column, returns `DatabaseError: no column found for name: wallet_address`.

### Fix

Update every SELECT that hydrates `Organization` to include the 3 missing columns. Identified callsites in `user.rs`:

- `find_by_id` — `SELECT * FROM organizations WHERE id = ?` (already uses `*`, may be fine — verify)
- `find_all` — explicit columns, missing the 3
- `find_all_active` — explicit columns, missing the 3
- `find_by_user` — explicit columns, missing the 3
- `create` `RETURNING` — explicit columns, missing the 3
- `update` `RETURNING` (if explicit) — verify and fix

Pattern of the fix (apply consistently):

```sql
-- BEFORE
SELECT id, name, slug, description, avatar_url, owner_id, settings,
       is_active, created_at, updated_at, invite_token, pending_owner_email,
       created_by_org_id, address
FROM organizations ...

-- AFTER
SELECT id, name, slug, description, avatar_url, owner_id, settings,
       is_active, created_at, updated_at, invite_token, pending_owner_email,
       created_by_org_id, address, wallet_address, vibe_balance, vibe_budget_total
FROM organizations ...
```

No struct changes needed (struct is already correct). No migration needed (column exists). No frontend changes (TS types stay identical — the fields are already exported by ts-rs).

---

## Bug 2 — `project_knowledge_sources` id+project_id BLOB/TEXT mismatch

### Symptom

```
GET /api/projects/:id/knowledge  →  500
body: {"message":"Failed to fetch knowledge sources: error occurred while
decoding column \"id\": invalid length: expected 16 bytes, found 36"}
```

After id is realigned, same error on `project_id`. After both, same error from the completeness view because `v_project_knowledge_completeness` returns `p.id` (TEXT 36) but struct expects BLOB-16.

Downstream effect: Project Knowledge view dead for every project. Org Intelligence aggregation across projects renders empty.

### Root cause — actual schema vs Rust struct vs writer

| Surface | Format used |
|---|---|
| Column declaration | `id BLOB`, `project_id BLOB` |
| `upsert_source` writes | TEXT-36 (calls `.to_string()` on both) |
| Seed data on disk | TEXT-36 (MVI rows + TIACA org-scoped rows) |
| `find_by_project` reads | binds as TEXT, struct expects Uuid (BLOB-16 via SQLx decoder) |
| `v_project_knowledge_completeness` view | returns `p.id` (TEXT-36 from projects.id which is TEXT) |
| Rust struct `ProjectKnowledgeSource.id`/`.project_id` | `Uuid` → SQLx tries BLOB-16 decode → fails on TEXT-36 |

The migration comment at `20260225000000_knowledge_sheaf_complete_triggers.sql:172` says "project_id is BLOB" reflecting the *original* design intent, but the actual `upsert_source` writer always wrote TEXT. The struct was never aligned with the writer.

### Fix — align Rust struct to what's actually on disk

`upsert_source` is the canonical writer and it writes TEXT. The view returns TEXT. The seed data is TEXT. The simplest, safest, least-disruptive fix is to **change the struct to read TEXT (`String`)** instead of `Uuid` (BLOB-16 decoder).

```rust
// BEFORE
pub struct ProjectKnowledgeSource {
    pub id: Uuid,
    pub project_id: Uuid,
    ...
}

pub struct ProjectKnowledgeCompleteness {
    pub project_id: Uuid,
    ...
}

// AFTER
pub struct ProjectKnowledgeSource {
    pub id: String,
    pub project_id: String,
    ...
}

pub struct ProjectKnowledgeCompleteness {
    pub project_id: String,
    ...
}
```

Also adjust the `find_by_project` / `get_completeness` function signatures to take `&str` (or `String`) instead of `Uuid`. Path handlers that currently use `Path<Uuid>` will need to change to `Path<String>` and use `parse_db_uuid_param` for validation — this is consistent with `rust-standards.md` ("Path parameters: Use `Path<String>`, not `Path<Uuid>`").

Frontend impact: `ts-rs` exports `Uuid` as `string` anyway, so the generated TS type is unchanged. `npm run generate-types` should be a no-op (verify).

### Alternative considered — change writer to BLOB-16

Would also work but requires:
- Updating `upsert_source` to bind raw 16 bytes
- Migrating ALL existing seed data (MVI rows, TIACA rows, future Sirak Studios Master Node data) from TEXT to BLOB
- Updating the view to wrap `p.id` in a hex-decode
- Risk of breaking other code paths that read these tables

Rejected. The TEXT path matches the canonical writer and the rest of the codebase's DbUuid TEXT-everywhere direction.

---

## Phases

### Phase 1 — Bug 1 fix (org SELECT drift)

1.1 Identify all SELECT statements in `crates/db/src/models/user.rs` that hydrate `Organization`. Add the 3 missing columns.

1.2 Run `cargo sqlx prepare --workspace` to refresh offline cache if used.

1.3 Test via `curl localhost:3022/api/organizations` (with auth cookie) — expect 200 + org list.

### Phase 2 — Bug 2 fix (project knowledge type alignment)

2.1 Change `ProjectKnowledgeSource.id` and `.project_id` from `Uuid` to `String` in `crates/db/src/models/project_knowledge_source.rs`.

2.2 Change `ProjectKnowledgeCompleteness.project_id` from `Uuid` to `String`.

2.3 Update `find_by_project(project_id: Uuid)` → `find_by_project(project_id: &str)`.

2.4 Update `get_completeness(project_id: Uuid)` → `get_completeness(project_id: &str)`.

2.5 Audit all callers of these two functions — update their argument passing.

2.6 Audit handlers in `crates/server/src/routes/knowledge.rs` that take `Path<Uuid>` for project_id — convert to `Path<String>` + `parse_db_uuid_param(&project_id, "project ID")?` per `rust-standards.md`.

2.7 Run `npm run generate-types` and verify no TS type changes (or only trivial ones).

2.8 Test via `curl localhost:3022/api/projects/01010101-0101-0101-0101-1ec55a070001/knowledge` (with auth cookie) — expect 200 + the Inessa strategy artifact in the list.

### Phase 3 — Verification

3.1 `cargo fmt --all`

3.2 `flox activate -- cargo clippy --all --all-targets -- -D warnings` per `rust-standards.md`

3.3 `cd frontend && npx tsc --noEmit && npx eslint . --ext ts,tsx`

3.4 `npm run generate-types:check`

3.5 `cargo test --workspace` (especially any test touching org list or project knowledge)

3.6 Manual smoke via Playwright:
- Org Intelligence page renders for PCG Org with the Inessa strategy entry visible in the Organization Intelligence badges section
- Project Knowledge view renders for the Inessa Wellness Brand Strategy project showing the strategy doc as an artifact
- Org Intelligence page renders for Sirak Studios with the 5 TIACA seed entries visible

### Phase 4 — Deployment

4.1 Stop the running avatar-engine backend on port 3022 (PID will be different at deploy time)

4.2 Start this worktree's backend on the same port pointed at the symlinked DB (already configured in .env)

4.3 Run the verification smoke from Phase 3.6 against the live dashboard at `localhost:3020`

4.4 When user is ready to resume avatar-engine work: stop this backend, restart avatar-engine backend. Both backends point at the same DB so state is preserved.

4.5 Push branch and open PR against `main`. Title: `fix: org list SELECT drift + project knowledge BLOB/TEXT alignment`.

---

## Test plan

- [ ] `GET /api/organizations` returns 200 with org list including PCG, Sirak Studios, Media Monsters etc.
- [ ] `GET /api/organizations/01010101-0101-0101-0101-010101010101/knowledge` returns 200 with Inessa strategy entry in `knowledge_entries`
- [ ] `GET /api/organizations/02020202-0202-0202-0202-020202020202/knowledge` returns 200 with TIACA's 5 seed entries (contract, readme, company_intel, contact_intel, brand_guide)
- [ ] `GET /api/projects/01010101-0101-0101-0101-1ec55a070001/knowledge` returns 200 with the Inessa strategy artifact entry
- [ ] `GET /api/projects/2d18132a-d74e-40fd-b3da-f2966fcfb3c4/knowledge` returns 200 with MVI artifact entries
- [ ] Dashboard `/organizations/<pcg-id>/intelligence` renders the Knowledge tab with the Inessa entry shown
- [ ] Dashboard `/organizations/<sirak-id>/intelligence` renders TIACA entries
- [ ] All `/check` items pass

## Open questions

None at this point. The fix is purely additive (org SELECTs) + a type swap that matches what's already on disk (knowledge struct fields).

## Risks

1. **Frontend code depending on `id` or `project_id` being typed as a specific Uuid brand** — low risk, ts-rs exports `Uuid` → `string` already
2. **Other queries in the codebase that hydrate `Organization` with the old explicit column list** — need to grep, may exist outside user.rs
3. **Existing tests asserting on `.id` being a Uuid value** — should still compile since we're swapping the value type, not removing fields

## Why this isn't part of the avatar-engine PR

Both bugs are:
- Pre-existing on `main`
- Affect surfaces unrelated to avatar engine (org list, project knowledge)
- Block production-relevant dashboard functionality (Knowledge Graph)
- Touch files (`user.rs`, `project_knowledge_source.rs`, `knowledge.rs`) that avatar-engine PR doesn't touch

Bundling them into PR #68 would expand its scope, complicate the review, and entangle the avatar-engine merge with unrelated work. They belong on their own PR off `main`.

---

## Coordination with Sirak Studios Master Node work

The user noted Sirak Studios is actively building on the Knowledge Graph for data sync. This fix unblocks that work — without it, any Knowledge Graph entries Sirak Studios writes won't render in the dashboard.

If the Master Node work has already been making schema changes, run a quick diff between this branch and that branch before opening the PR. Coordinate merge order so neither stomps the other.
