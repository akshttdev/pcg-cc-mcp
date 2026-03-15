---
name: review-rust
description: Review Rust code changes for unwrap() calls, missing error handling, and platform standard violations
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Agent
---

# Rust Code Review

Review staged or recent Rust changes for platform standard violations.

## What to check

1. **unwrap/expect audit**: Search changed `.rs` files for `.unwrap()` and `.expect()` outside of `#[cfg(test)]` blocks. Flag every instance.

2. **Error handling**: Check that all `?` operators have appropriate error context. Look for bare `Err(format!(...))` without structured error types.

3. **Concurrency**: Check for `std::sync::Mutex` in async code, missing timeouts on lock acquisitions, unbounded `tokio::spawn()`.

4. **Database**: Check for `SELECT *`, missing parameterization, new tables without indexes on foreign keys.

5. **Input validation**: Check that route handlers validate input (UUID format, string length, enum variants) before processing.

## Scope

If `$ARGUMENTS` is provided, review that specific file or crate.
Otherwise, review files changed since the last commit: `git diff HEAD~1 --name-only -- '*.rs'`

## Output

Report findings as:
```
## Rust Review: [scope]

### Violations Found
- [ ] `file:line` — description (severity)

### Clean
- All error handling checks passed
```
