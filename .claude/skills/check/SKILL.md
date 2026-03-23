---
name: check
description: Run all project checks (typecheck, lint, clippy, format) and report results
user-invocable: true
allowed-tools: Bash, Read, Grep
---

# Full Project Check

Run all CI checks locally and report a summary. **All commands run from the project root** unless noted otherwise.

## 1. Rust Format

```bash
# IMPORTANT: Run fmt first, then check. The --check flag caches results and can miss files.
cargo fmt --all
cargo fmt --all -- --check
```

If `cargo fmt --all` changes files, stage and commit them before proceeding.

## 2. Rust Clippy

Use the **exact** `-A` flags from `.github/workflows/ci.yml`. Read them from the file:

```bash
# Extract clippy flags from CI config
grep -A20 "cargo clippy" .github/workflows/ci.yml
# Then run with those flags:
flox activate -- cargo clippy --all --all-targets -- -D warnings [paste -A flags from ci.yml]
```

Do NOT use `--all-features` — CI doesn't use it.

## 3. TypeScript (from `frontend/`)

```bash
cd frontend && npx tsc --noEmit
```

## 4. ESLint (from `frontend/`)

```bash
cd frontend && npx eslint . --ext ts,tsx
```

Check the CI max-warnings threshold if needed.

## 5. Type Generation (from **project root**)

```bash
cd /path/to/project/root
npm run generate-types:check
```

This must run from the project root, NOT from `frontend/`.

## Report

| Check | Status | Issues |
|-------|--------|--------|
| cargo fmt | PASS/FAIL | N diffs |
| cargo clippy | PASS/FAIL | N warnings/errors |
| tsc | PASS/FAIL | N errors |
| eslint | PASS/FAIL | N errors, N warnings |
| generate-types | PASS/FAIL | types up to date? |

If any check fails, show the first 10 lines of errors and suggest fixes.

If all pass, report: "All 5 checks pass. Ready to push."
