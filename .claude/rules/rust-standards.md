---
paths:
  - "crates/**/*.rs"
---

# Rust Code Standards

## Error Handling — NEVER use .unwrap() or .expect() in non-test code

- Use `?` operator for error propagation
- Use `.map_err()` to convert between error types
- Use `.unwrap_or_default()` or `.unwrap_or_else()` for safe defaults
- The ONLY acceptable places for `.unwrap()` are:
  - `#[cfg(test)]` blocks
  - `lazy_static!` / `once_cell` with compile-time-known values (e.g., hardcoded regex)

## Axum Route Handlers

- Handler signature order: `Extension(access_context)`, `State(deployment)`, `Path(id)`, `Query(params)`, `Json(payload)`
- Return `Result<ResponseJson<ApiResponse<T>>, ApiError>` — never raw `impl IntoResponse`
- Success: `Ok(ResponseJson(ApiResponse::success(data)))`
- Each route module exports a `router()` function that returns `Router<DeploymentImpl>`
- Use `middleware::from_fn_with_state()` for model-loading middleware, not inline guards

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

## Error Types

- Domain errors go in their own enum (e.g., `ProjectError`) with `#[derive(Debug, Error)]`
- Implement `From<DomainError> for ApiError` so handlers can use `?`
- Never return raw `StatusCode` — always use `ApiError` variants for structured error responses
- Add `#[derive(TS)]` with `#[ts(type = "string")]` on error enums for TypeScript export

## Access Control

- Always check permissions via `AccessContext` methods: `.require_viewer()`, `.require_editor()`, `.require_admin()`, `.require_org_membership()`
- **Org-scoped endpoints**: use `access_context.require_org_membership(pool, org_id).await?` — defined in `crates/server/src/middleware/access_control.rs`
- **Resource-scoped endpoints**: load the resource, extract its `organization_id`, then call `require_org_membership`. See `require_deal_org_access()` in `crm_deals.rs` and `require_pipeline_org_access()` in `crm_pipelines.rs` for patterns.
- Use model-loading middleware to verify access before the handler runs
- Never trust client-supplied user IDs — use `access_context.user_id` from the auth token

## Concurrency

- Never use `std::sync::Mutex` in async code — use `tokio::sync::Mutex`
- Always add timeouts to lock acquisitions: `tokio::time::timeout(Duration::from_secs(30), mutex.lock())`
- Every `tokio::spawn()` must have error handling (`.await` the JoinHandle or log errors)
- Use `CancellationToken` for long-running spawned tasks and graceful shutdown

## HTTP Clients

- Never create ad-hoc `reqwest::Client` instances — use the shared client from app state
- All HTTP requests must have a timeout (30s default)
- Validate external input before using in URLs or headers

## Service Layer

- Services are stateless and cheap to clone — store no mutable state
- Accept `&SqlitePool` as parameter, not the full deployment/state
- Define domain-specific error enums with `#[from] sqlx::Error` variant
- Use `#[async_trait]` for trait-based services (e.g., `GitServiceTrait`, `ExecutorService`)

## Type Sharing with ts-rs

- Add `#[derive(TS)]` and `#[ts(export)]` on all API-facing structs and enums
- Use `#[ts(type = "Date")]` for `DateTime<Utc>` fields
- Use `#[ts(optional)]` for `Option<T>` fields (renders as `field?: T` in TS)
- Use `#[ts(type = "Record<string, unknown> | null")]` for `Option<Json<Value>>`
- After modifying any `#[derive(TS)]` struct: `npm run generate-types`

## Modularity & Functional Design

- Prefer pure functions that take inputs and return outputs — minimize side effects
- Route handlers should be thin: validate input → call service → format response
- Extract business logic into service methods — handlers should not contain domain logic
- Keep crate boundaries clean: `db` knows nothing about HTTP, `server` knows nothing about SQL details
- One responsibility per function — if a function does setup + work + cleanup, split it
- Prefer composing small functions over writing long procedural blocks
- Use iterators and combinators (`.map()`, `.filter()`, `.collect()`) over imperative loops where readable
- Extract repeated patterns into shared utility functions in `crates/server/src/helpers/` or `crates/utils/`
- **Large route files**: split by concern — e.g., `crm_deals.rs` (CRUD), `crm_deal_transitions.rs` (stage movement/gating), `crm_deal_automations.rs` (AI triggers, LLM calls, report generation)

## Input Validation

- Validate all user input at the API boundary (route handler level)
- Check string lengths, UUID formats, and enum variants
- Return structured error responses, never panic on bad input
