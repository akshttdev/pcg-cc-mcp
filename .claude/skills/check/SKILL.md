---
name: check
description: Run all project checks (typecheck, lint, clippy, format) and report results
user-invocable: true
allowed-tools: Bash, Read, Grep
---

# Full Project Check

Run all 5 CI checks and report results.

**CRITICAL**: Each Bash tool call starts with a fresh working directory. You MUST `cd` to the correct directory at the START of every single command. Do NOT rely on `cd` from a previous Bash call.

## Run all checks

Run steps 1, 3, 4, 5 in parallel (they're independent). Step 2 (clippy) is slow — run in background or last.

### Step 1: Rust Format

```bash
cd $(git rev-parse --show-toplevel) && cargo fmt --all && cargo fmt --all -- --check
```

If this changes files, `git diff --stat` will show them. **Stop and commit before continuing.**

### Step 2: Rust Clippy (slow — run in background)

```bash
cd $(git rev-parse --show-toplevel) && CLIPPY_FLAGS=$(grep -A20 "cargo clippy" .github/workflows/ci.yml | grep -oP '\-A clippy::\S+' | tr '\n' ' ') && flox activate -- cargo clippy --all --all-targets -- -D warnings $CLIPPY_FLAGS
```

### Step 3: TypeScript

```bash
cd $(git rev-parse --show-toplevel)/frontend && npx tsc --noEmit
```

### Step 4: ESLint

```bash
cd $(git rev-parse --show-toplevel)/frontend && npx eslint . --ext ts,tsx
```

0 errors required. Warnings are pre-existing and acceptable.

### Step 5: Type Generation

```bash
cd $(git rev-parse --show-toplevel) && npm run generate-types:check
```

## Report

| Check | Status | Issues |
|-------|--------|--------|
| cargo fmt | PASS/FAIL | N diffs |
| cargo clippy | PASS/FAIL | N errors |
| tsc | PASS/FAIL | N errors |
| eslint | PASS/FAIL | N errors, N warnings |
| generate-types | PASS/FAIL | up to date? |

If any fail, show first 10 lines of errors and suggest fixes.

If all pass: **"All 5 checks pass. Ready to push."**
