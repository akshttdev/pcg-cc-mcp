# DbUuid Hybrid Decode Helper — BLOB/TEXT UUID Unification

**Date:** 2026-03-14
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13`
**Status:** COMPLETE (Phases 1 & 2). Phase 3 future work → `notes/2026-03-14--reference--dbuuid-phase3-remaining.md`
**Blocks:** QA bugs #1, #2, #6, #7, #8, #10, #11 (7 of 15 bugs)

---

## Problem

The database is in a hybrid state: legacy tables store UUIDs as 16-byte BLOBs, newer tables store them as 36-char TEXT strings. The Rust `Agent` model uses `sqlx::types::Uuid` which only decodes BLOBs, but the seed DB has TEXT UUIDs in the agents table. This breaks every agent-related API endpoint.

## Solution

Create a `DbUuid` newtype that transparently decodes **both** BLOB and TEXT UUIDs from SQLite, always encodes as TEXT. Roll out in phases: Phase 1 (type + helpers), Phase 2 (agent + FK references), Phase 3 (remaining models, future PR).

---

## Phase 1: Create `DbUuid` Type + Helpers — DONE

**Commit:** `2cf77c82b`

- **NEW:** `crates/db/src/db_uuid.rs` (~300 lines) — hybrid decode type with 12 unit tests
- `sqlx::Decode<Sqlite>` inspects `type_info().name()`, branches TEXT vs BLOB
- Always encodes as TEXT via `sqlx::Encode<Sqlite>`
- Helper functions: `new()`, `from_string()`, `as_str()`, `into_string()`, `parse()`
- Migration helpers: `bind_uuid()`, `bind_optional_uuid()`
- Conversion traits: `Deref<Target=str>`, `Display`, `From<String>`, `From<uuid::Uuid>`, `AsRef<str>`, `PartialEq<str>`, `PartialEq<String>`, `ts_rs::TS`

---

## Phase 2: Convert Agent + FK References — DONE

**Commit:** `2cf77c82b`

### Files Modified

| # | File | Change |
|---|------|--------|
| 1 | `crates/db/src/db_uuid.rs` | **NEW** — DbUuid type + helpers + 12 tests (~300 lines) |
| 2 | `crates/db/src/lib.rs` | Added `pub mod db_uuid; pub use db_uuid::DbUuid;` |
| 3 | `crates/db/src/models/agent.rs` | `Uuid` → `DbUuid` in 3 structs + 10 functions |
| 4 | `crates/db/src/models/agent_execution_config.rs` | `agent_id: Uuid` → `DbUuid`, `find_by_agent_id` → `&str` |
| 5 | `crates/server/src/routes/agents.rs` | `Path<Uuid>` → `Path<String>`, removed `uuid::Uuid` import |
| 6 | `crates/server/src/routes/agent_chat.rs` | `Path<Uuid>` → `Path<String>`, Uuid bridge for conversations |
| 7 | `crates/server/src/routes/orcha.rs` | Removed `Uuid::from_slice()` workaround, `String` types |
| 8 | `crates/server/src/routes/tasks.rs` | Simplified agent watcher registration, `String` agent_id |
| 9 | `crates/server/src/routes/workflow_staging.rs` | Removed `Uuid::parse_str` intermediary |
| 10 | `crates/services/src/services/agent_registry.rs` | `assign_wallet` → `&str`, `owner_id` conversion |
| 11 | `crates/services/src/services/qa_review.rs` | Direct `&str` agent lookup |
| 12 | `crates/services/src/services/user_onboarding.rs` | `orcha_agent_id` → `String` |
| 13 | `crates/local-deployment/src/container.rs` | Simplified agent config lookup |
| 14 | `.sqlx/` | 8 query cache files updated |

## Verification

- [x] `cargo check --workspace` — no compile errors
- [x] `cargo test -p db db_uuid` — 12 tests passing
- [x] `DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace` — cache updated
