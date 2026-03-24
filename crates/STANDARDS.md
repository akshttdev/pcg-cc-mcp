# Rust Implementation Standards

These are implementation-level patterns and conventions for the Rust codebase. For critical rules (error handling, UUID handling, access control, error types), see `.claude/rules/rust-standards.md`.

## Axum Route Handlers

- Handler signature order: `Extension(access_context)`, `State(deployment)`, `Path(id)`, `Query(params)`, `Json(payload)`
- Return `Result<ResponseJson<ApiResponse<T>>, ApiError>` — never raw `impl IntoResponse`
- Success: `Ok(ResponseJson(ApiResponse::success(data)))`
- Each route module exports a `router()` function that returns `Router<DeploymentImpl>`
- Use `middleware::from_fn_with_state()` for model-loading middleware, not inline guards

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
