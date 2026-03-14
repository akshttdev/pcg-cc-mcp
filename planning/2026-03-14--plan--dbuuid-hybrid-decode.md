# DbUuid Hybrid Decode Helper — BLOB/TEXT UUID Unification

**Date:** 2026-03-14
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13`
**Blocks:** QA bugs #1, #2, #6, #7, #8, #10, #11 (7 of 15 bugs)

---

## Problem

The database is in a hybrid state: legacy tables store UUIDs as 16-byte BLOBs, newer tables store them as 36-char TEXT strings. The Rust `Agent` model uses `sqlx::types::Uuid` which only decodes BLOBs, but the seed DB has TEXT UUIDs in the agents table. This breaks every agent-related API endpoint.

## Solution

Create a `DbUuid` newtype that transparently decodes **both** BLOB and TEXT UUIDs from SQLite, always encodes as TEXT. Roll out in phases: Phase 1 (type + helpers), Phase 2 (agent + FK references), Phase 3 (remaining models, future PR).

---

## Phase 1: Create `DbUuid` Type + Helpers

### Step 1: Create `crates/db/src/db_uuid.rs` (~120 lines)

```rust
/// A UUID that transparently reads both BLOB (16-byte) and TEXT (36-char)
/// formats from SQLite, and always writes as TEXT.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DbUuid(String);
```

**Core trait impls:**
- `sqlx::Decode<Sqlite>` — inspect `type_info().name()`, decode TEXT directly or BLOB via `Uuid::from_slice()` → hyphenated string
- `sqlx::Encode<Sqlite>` — always encode as TEXT string
- `sqlx::Type<Sqlite>` — report as TEXT

**Helper functions (on `DbUuid`):**

| Helper | Signature | Purpose |
|--------|-----------|---------|
| `new()` | `fn new() -> Self` | Generate a new v4 UUID as TEXT |
| `from_string(s: impl Into<String>)` | `fn from_string(s: impl Into<String>) -> Self` | Wrap an existing UUID string |
| `as_str()` | `fn as_str(&self) -> &str` | Borrow inner string |
| `into_string()` | `fn into_string(self) -> String` | Consume and return inner string |
| `parse(s: &str)` | `fn parse(s: &str) -> Result<Self>` | Validate and wrap a UUID string |

**Conversion trait impls:**

| Trait | Purpose |
|-------|---------|
| `Deref<Target=str>` | Use `&db_uuid` anywhere `&str` is expected |
| `Display` | `format!("{}", db_uuid)` returns hyphenated UUID |
| `From<String>` | `DbUuid::from(string)` |
| `From<uuid::Uuid>` | `DbUuid::from(uuid)` — for migrating code that creates `Uuid::new_v4()` |
| `From<DbUuid> for String` | `String::from(db_uuid)` |
| `AsRef<str>` | Borrow as `&str` |
| `PartialEq<str>` | Compare `db_uuid == "some-uuid-string"` |
| `PartialEq<String>` | Compare `db_uuid == some_string` |
| `ts_rs::TS` | TypeScript type generation (exports as `string`) |

**Migration helpers (free functions):**

| Helper | Signature | Purpose |
|--------|-----------|---------|
| `bind_uuid(uuid: &DbUuid)` | Returns `&str` | For raw `sqlx::query()` `.bind()` calls — replaces `.as_bytes().as_slice()` |
| `bind_optional_uuid(uuid: &Option<DbUuid>)` | Returns `Option<&str>` | For optional UUID bindings — replaces `.as_ref().map(\|u\| u.as_bytes().to_vec())` |

These two free functions aren't strictly needed (since `Deref` handles most cases) but make the migration mechanical: search for `.as_bytes()` → replace with `bind_uuid()` or just `&id`.

### Step 2: Register module in `crates/db/src/lib.rs`

Add `pub mod db_uuid;` and `pub use db_uuid::DbUuid;` to the crate root.

### Step 3: Unit tests in `crates/db/src/db_uuid.rs`

- Decode TEXT UUID → correct string
- Decode BLOB UUID → correct hyphenated string
- Encode always produces TEXT
- `DbUuid::new()` produces valid v4 UUID
- `DbUuid::parse()` rejects invalid strings
- `From<uuid::Uuid>` conversion
- `Deref` / `AsRef` / `Display` work correctly

---

## Phase 2: Convert Agent + FK References

### Step 4: Convert `agent.rs` (~8 query functions)

**File:** `crates/db/src/models/agent.rs`

For each of the 8 query functions:
1. Change struct field `pub id: Uuid` → `pub id: DbUuid` (line 91)
2. Change `pub parent_agent_id: Option<Uuid>` → `Option<DbUuid>` (line ~95)
3. Change `pub owner_id: Option<Uuid>` → `Option<DbUuid>` (line ~97)
4. In `query_as!()` macros: change type casts from `"id!: Uuid"` → `"id!: DbUuid"`, etc.
5. In `create()`: change `Uuid::new_v4()` → `DbUuid::new()`
6. In function signatures: `id: Uuid` → `id: &DbUuid` or `id: &str`
7. Remove `use uuid::Uuid` if no longer needed

**Functions to update:**
- `find_all()` (line 267) — return type casting
- `find_active()` (line 309) — return type casting
- `find_by_id()` (line 352) — parameter + return type
- `find_by_short_name()` (line 395) — return type casting
- `find_by_wallet()` (line 438) — return type casting
- `create()` (line 491) — UUID generation + return type
- `update()` (line 578) — parameter + return type
- `find_visible_for_user()` (line 706) — parameter + return type
- `record_task_completion()` (line 663) — parameter type
- `update_status()` (line 749) — parameter type

### Step 5: Convert `agent_execution_config.rs` (FK join)

**File:** `crates/db/src/models/agent_execution_config.rs`

- Line 317: `JOIN agents a ON a.id = aec.agent_id` — runtime `query_as::<_>`
- Change `agent_id` field from `Uuid` to `DbUuid`
- Update `.bind()` calls for agent_id parameters

### Step 6: Convert `token_usage.rs` (FK join)

**File:** `crates/db/src/models/token_usage.rs`

- Line 287: `LEFT JOIN agents a ON tu.agent_id = a.id` — runtime `query_as::<_>`
- Change agent-related fields to `DbUuid`

### Step 7: Convert `crm_contact.rs` (assigned_agent_id)

**File:** `crates/db/src/models/crm_contact.rs`

- Line 127, 195: `pub assigned_agent_id: Option<Uuid>` → `Option<DbUuid>`
- Line 494, 531: `.bind(data.assigned_agent_id)` — now encodes as TEXT automatically

### Step 8: Fix server-side agent queries

**Files with manual BLOB workarounds to clean up:**

| File | Line | Current Pattern | New Pattern |
|------|------|-----------------|-------------|
| `routes/agent_chat.rs` | 824 | `.bind(agent_id)` (Uuid) | `.bind(agent_id.as_str())` or `.bind(&agent_id)` |
| `routes/twilio.rs` | 918 | `query_as("SELECT id...")` → `Uuid::from_slice()` | `query_scalar::<_, String>()` — no manual conversion |
| `routes/orcha.rs` | 102 | `query_as::<_, (Vec<u8>, ...)>()` → `Uuid::from_slice()` | `query_as::<_, (String, ...)>()` — direct string |
| `sovereign_storage.rs` | 521 | BLOB IDs → hex in JSON | TEXT IDs → clean UUIDs in JSON |

### Step 9: Update `sqlx-data.json` offline cache

```bash
DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace
```

Commit the updated `.sqlx/` directory.

---

## Phase 3: Remaining Models (Future PR)

Convert the remaining ~107 models from `Uuid` → `DbUuid`. This is mechanical:

**Per-model checklist:**
1. `pub id: Uuid` → `pub id: DbUuid`
2. All `Option<Uuid>` FK fields → `Option<DbUuid>`
3. Replace `.as_bytes().as_slice()` → `&field` (Deref handles it)
4. Replace `.as_ref().map(|u| u.as_bytes().to_vec())` → `field.as_ref().map(|u| u.as_str())`
5. Replace `Uuid::new_v4()` → `DbUuid::new()`
6. Update `query_as!()` type casts
7. Run `cargo sqlx prepare --workspace`

**Known `.as_bytes()` sites to update:**
- `user.rs` (lines 212, 226-227)
- `person.rs` (lines 187, 272, 286, 330, 344, 361, 376, 407-408, 424)
- `project_knowledge_source.rs` (lines 124, 150-151, 184, 199, 214)
- `org_brand_profile.rs` (lines 109, 136-137)
- `activity.rs` (line 156)

This phase is low-risk per-file but high-volume (~107 files). Can be batched by domain (CRM models, workflow models, core models).

---

## Files Modified Summary

| # | File | Change |
|---|------|--------|
| 1 | `crates/db/src/db_uuid.rs` | **NEW** — DbUuid type + helpers + tests (~120 lines) |
| 2 | `crates/db/src/lib.rs` | Add `pub mod db_uuid; pub use db_uuid::DbUuid;` |
| 3 | `crates/db/src/models/agent.rs` | `Uuid` → `DbUuid` in struct + 10 functions |
| 4 | `crates/db/src/models/agent_execution_config.rs` | `agent_id: Uuid` → `DbUuid` |
| 5 | `crates/db/src/models/token_usage.rs` | Agent FK fields → `DbUuid` |
| 6 | `crates/db/src/models/crm_contact.rs` | `assigned_agent_id: Uuid` → `DbUuid` |
| 7 | `crates/server/src/routes/agent_chat.rs` | Remove manual BLOB binding |
| 8 | `crates/server/src/routes/twilio.rs` | Remove `Uuid::from_slice()` workaround |
| 9 | `crates/server/src/routes/orcha.rs` | Remove `Vec<u8>` → `Uuid::from_slice()` workaround |
| 10 | `crates/server/src/sovereign_storage.rs` | Clean up BLOB→JSON serialization |
| 11 | `.sqlx/` | Updated offline query cache |

## Verification

1. `cargo check --workspace` — no compile errors
2. `cargo test --workspace` — existing tests pass
3. `DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace` — cache updated
4. Start dev server → Settings > Agents → agents load (no 500)
5. Create Task → Agent picker → agents listed (no perpetual loading)
6. Task detail → Agent Watcher panel → no 500 errors
7. `cd frontend && npx tsc --noEmit` — no TS errors (after `npm run generate-types` if DbUuid affects TS types)
