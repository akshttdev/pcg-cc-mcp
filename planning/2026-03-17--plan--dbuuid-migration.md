# DbUuid Migration Plan — Eliminate BLOB/Uuid Boilerplate

**Date**: 2026-03-17
**Status**: Planning
**Goal**: Use String/DbUuid in all interfaces, push BLOB conversions to DB consumer layer

## Current State

| Type | Count | Location |
|------|-------|----------|
| `pub id: Uuid` in models | ~150 | 104 model files |
| `pub id: DbUuid` in models | ~14 | 9 recently-converted models |
| `Vec<u8>` for IDs | ~20 | 4 files (orchestration, entity_conversion, person_association) |
| `AccessContext.user_id: Uuid` usage | 62 | 17 route files |
| `.to_string()` on user_id | 26 | 9 route files |
| `Path<Uuid>` in handlers | ~416 | 79 route files |
| `.as_bytes().to_vec()` BLOB binding | 21 | 9 code files |
| `DbUuid::from()` bridge calls | 88 | 18 files |
| `Uuid::parse_str()` in routes | 138 | 42 files |

## Remaining BLOB Columns (root cause: `users.id` is BLOB)

| Table | Column | FK chain |
|-------|--------|----------|
| `users` | `id` | Root |
| `project_members` | `user_id`, `granted_by` | → users.id |
| `organization_members` | `user_id` | → users.id |
| `client_members` | `user_id` | → users.id |

All other UUID columns are TEXT.

## Migration Phases (Ordered by ROI)

### Phase A: `AccessContext.user_id: Uuid` → `DbUuid`
**ROI**: Highest. Single most-propagated Uuid in codebase — every authenticated request flows through this.
**Eliminates**: ~26 `.to_string()` calls, ~10 `DbUuid::from(access_context.user_id)` bridges
**Files**: `access_control.rs` + ~17 route files
**Risk**: Medium. Validate with `DbUuid::parse()` at construction.
**Approach**:
1. Change `AccessContext { user_id: Uuid }` → `AccessContext { user_id: DbUuid }`
2. Update `build_access_context` to construct DbUuid from the decoded row
3. In BLOB-column bindings, use `bind_uuid_blob(&self.user_id)` directly
4. Remove `.to_string()` at all route-handler call sites

### Phase B: `Path<Uuid>` → `Path<String>` in route handlers
**ROI**: High. Eliminates second most common `.to_string()` boilerplate.
**Eliminates**: ~416 Path extractions that immediately convert to string
**Files**: ~79 route files (mechanical find-replace)
**Risk**: Low-Medium. Move UUID validation to model layer via `DbUuid::parse()`.
**Approach**: Batch by domain (CRM routes first, then core, then topology/nora/topsi).

### Phase C: Migrate `users.id` BLOB → TEXT
**ROI**: High but highest risk. Eliminates ALL BLOB code paths.
**Eliminates**: All 21 `.as_bytes().to_vec()`, all `Vec<u8>` FromRow structs, all `bind_uuid_blob` usage, all `#[sqlx(try_from = "Vec<u8>")]` annotations
**Files**: 1 migration + ~9 code files + `db_uuid.rs` cleanup
**Risk**: Medium-High. Must preserve sessions, FK relationships, sovereign_storage refs.
**Approach**:
1. Write migration: create new TEXT columns, copy with hex conversion, swap, drop old
2. Update all 4 member table FromRow structs
3. Delete `bind_uuid_blob` / `str_to_uuid_blob` helper family
4. Test auth flow end-to-end

### Phase D: Batch convert remaining 104 models from `Uuid` → `DbUuid`
**ROI**: Medium. Eliminates remaining `DbUuid::from()` bridges and `Uuid::parse_str()` calls.
**Files**: 104 model files + corresponding route files
**Risk**: Low. Mechanical change, follow per-model checklist.
**Reference**: `planning/notes/2026-03-14--reference--dbuuid-phase3-remaining.md`

## Recommended Start

Phase A first — self-contained, moderate scope, immediate ROI. Can be a single PR.
Phase B can follow in the same sprint. Phase C requires DB migration testing.
