---
paths:
  - "crates/**/*.rs"
---

# Rust Code Standards

## Error Handling — NEVER use .unwrap() or .expect() in non-test code

- Use `?` operator for error propagation
- Use `.map_err()` to convert between error types
- Use `.unwrap_or_default()` or `.unwrap_or_else()` for safe defaults
- For JSON serialization, prefer `serde_json::to_string(&v).map_err(|e| ...)?` over `.unwrap()`
- For UUID parsing from external input, always validate: `Uuid::parse_str(&s).map_err(|_| ApiError::BadRequest("invalid UUID"))?`
- The ONLY acceptable places for `.unwrap()` are:
  - `#[cfg(test)]` blocks
  - `lazy_static!` / `once_cell` with compile-time-known values (e.g., hardcoded regex)

## Concurrency

- Never use `std::sync::Mutex` in async code — use `tokio::sync::Mutex`
- Always add timeouts to lock acquisitions: `tokio::time::timeout(Duration::from_secs(30), mutex.lock())`
- Every `tokio::spawn()` must have error handling (`.await` the JoinHandle or log errors)
- Use `CancellationToken` for long-running spawned tasks

## HTTP Clients

- Never create ad-hoc `reqwest::Client` instances — use the shared client from app state
- All HTTP requests must have a timeout (30s default)
- Validate external input before using in URLs or headers

## Database Queries

- Never use `SELECT *` — list columns explicitly
- Always parameterize queries (SQLx does this, but double-check raw SQL)
- Add indexes for any new foreign key columns
- After modifying queries: `DATABASE_URL="sqlite:dev_assets/db.sqlite" cargo sqlx prepare --workspace`

## Input Validation

- Validate all user input at the API boundary (route handler level)
- Check string lengths, UUID formats, and enum variants
- Return structured error responses, never panic on bad input
