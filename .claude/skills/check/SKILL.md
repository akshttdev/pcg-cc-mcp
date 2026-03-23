---
name: check
description: Run all project checks (typecheck, lint, clippy, format) and report results
user-invocable: true
allowed-tools: Bash, Read, Grep
---

# Full Project Check

Run all 5 CI checks and report results. Use absolute paths to avoid directory confusion.

## Step 1: Get project root

```bash
# All paths relative to this
PROJECT_ROOT=$(git rev-parse --show-toplevel)
```

## Step 2: Rust Format

```bash
# Format first (--check has caching that misses files), then verify
cd "$PROJECT_ROOT" && cargo fmt --all
cd "$PROJECT_ROOT" && cargo fmt --all -- --check
```

If formatting changed files, **stop and commit them** before continuing. Check with `git diff --stat`.

## Step 3: Rust Clippy

Extract the exact `-A` flags from CI config and run clippy:

```bash
# Read flags from ci.yml
cd "$PROJECT_ROOT"
CLIPPY_FLAGS=$(grep -A20 "cargo clippy" .github/workflows/ci.yml | grep -oP '\-A clippy::\S+' | tr '\n' ' ')
# Run clippy
flox activate -- cargo clippy --all --all-targets -- -D warnings $CLIPPY_FLAGS
```

If `flox activate` fails, run without it but ensure `CMAKE_POLICY_VERSION_MINIMUM=3.5` is set.

## Step 4: TypeScript

```bash
cd "$PROJECT_ROOT/frontend" && npx tsc --noEmit
```

## Step 5: ESLint

```bash
cd "$PROJECT_ROOT/frontend" && npx eslint . --ext ts,tsx
```

0 errors required. Warnings are pre-existing and acceptable.

## Step 6: Type Generation

```bash
cd "$PROJECT_ROOT" && npm run generate-types:check
```

**Must run from project root** — this script is defined in root `package.json`, not `frontend/package.json`.

## Report

After running all 5, output:

| Check | Status | Issues |
|-------|--------|--------|
| cargo fmt | PASS/FAIL | N diffs |
| cargo clippy | PASS/FAIL | N errors |
| tsc | PASS/FAIL | N errors |
| eslint | PASS/FAIL | N errors, N warnings |
| generate-types | PASS/FAIL | up to date? |

If any check fails, show the first 10 lines of errors and suggest fixes.

If all pass: **"All 5 checks pass. Ready to push."**
