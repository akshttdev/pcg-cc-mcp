---
name: check
description: Run all project checks (typecheck, lint, clippy, format) and report results
user-invocable: true
allowed-tools: Bash, Read, Grep
---

# Full Project Check

Run all checks and report a summary. Execute these in parallel where possible:

## Backend (run inside `flox activate`)
```bash
flox activate -- cargo fmt --all -- --check
flox activate -- cargo clippy --all --all-targets --all-features -- -D warnings
flox activate -- cargo test --workspace --no-fail-fast 2>&1 | tail -20
```

## Frontend
```bash
cd frontend && npx tsc --noEmit
cd frontend && npm run lint
cd frontend && npm run format:check
```

## Type Sync
```bash
npm run generate-types:check
```

Report results as a summary table:
| Check | Status | Issues |
|-------|--------|--------|

If any check fails, show the first 10 lines of errors and suggest fixes.
