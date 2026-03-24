---
paths:
  - "crates/**/*.rs"
---

# Rust Code Standards

> Implementation patterns (Axum handlers, concurrency, HTTP clients, service layer, ts-rs, modularity, validation) are in `crates/STANDARDS.md`.

## Error Handling — NEVER use .unwrap() or .expect() in non-test code

- Use `?` operator for error propagation
- Use `.map_err()` to convert between error types
- Use `.unwrap_or_default()` or `.unwrap_or_else()` for safe defaults
- The ONLY acceptable places for `.unwrap()` are:
  - `#[cfg(test)]` blocks
  - `lazy_static!` / `once_cell` with compile-time-known values (e.g., hardcoded regex)

## UUID Handling — Use DbUuid, not uuid::Uuid

- **Path parameters**: Use `Path<String>`, not `Path<Uuid>`. Validate with `parse_db_uuid_param(&id, "deal ID")?` from `crate::helpers::uuid_params`
- **NEVER** use `Uuid::parse_str()` — use `DbUuid::parse()` instead. DbUuid is the project's canonical UUID type.
- **NEVER** use `uuid::Uuid::new_v4()` — use `DbUuid::new()` instead
- **Interfaces**: Use `String` or `DbUuid` in function signatures. Push BLOB conversions to the DB consumer via `bind_uuid_blob()`.
- **Import**: `use db::db_uuid::DbUuid;` — avoid `use uuid::Uuid;` in route handlers
- **Conversion helpers** (in `crates/db/src/db_uuid.rs`):
  - `DbUuid::new()` — generate v4 UUID
  - `DbUuid::parse(&str)` — validate and parse (returns `Result<DbUuid, uuid::Error>`)
  - `DbUuid::from_string(s)` — wrap without validation (trusted input only)
  - `bind_uuid(&db_uuid)` — bind to TEXT column
  - `bind_uuid_blob(&db_uuid)` — bind to legacy BLOB column
- **Route handler helper**: `parse_db_uuid_param(&str, &str)` in `crates/server/src/helpers/uuid_params.rs` — wraps `DbUuid::parse` with `ApiError::BadRequest`. Always use this in route handlers instead of raw `.map_err()`.

## Access Control

- Always check permissions via `AccessContext` methods: `.require_viewer()`, `.require_editor()`, `.require_admin()`, `.require_org_membership()`
- **Org-scoped endpoints**: use `access_context.require_org_membership(pool, org_id).await?` — defined in `crates/server/src/middleware/access_control.rs`
- **Resource-scoped endpoints**: load the resource, extract its `organization_id`, then call `require_org_membership`. See `require_deal_org_access()` in `crm_deals.rs` and `require_pipeline_org_access()` in `crm_pipelines.rs` for patterns.
- Use model-loading middleware to verify access before the handler runs
- Never trust client-supplied user IDs — use `access_context.user_id` from the auth token

## Error Types

- Domain errors go in their own enum (e.g., `ProjectError`) with `#[derive(Debug, Error)]`
- Implement `From<DomainError> for ApiError` so handlers can use `?`
- Never return raw `StatusCode` — always use `ApiError` variants for structured error responses
- Add `#[derive(TS)]` with `#[ts(type = "string")]` on error enums for TypeScript export
