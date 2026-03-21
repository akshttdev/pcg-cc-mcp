---
name: qa-review
description: Pre-merge QA — static checks, conflict scan, barrel exports, and optional browser smoke tests
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Agent, mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_console_messages, mcp__playwright__browser_take_screenshot
---

# QA Review

Quick quality gate before merging. For deep security/correctness analysis, use `/regression-test` instead.

## 1. Static Checks

Run all CI checks locally:
```bash
flox activate -- cargo fmt --all -- --check
flox activate -- cargo clippy --all --all-targets -- -D warnings [with current -A flags from ci.yml]
cd frontend && npx tsc --noEmit
cd frontend && npx eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings [threshold from package.json]
```

Report: PASS/FAIL for each.

## 2. Structural Integrity

```bash
# Conflict markers
grep -rn "<<<<<<" crates/ frontend/src/ shared/ planning/ --include="*.rs" --include="*.ts" --include="*.tsx" --include="*.md"

# Dead imports (files importing from deleted/renamed modules)
git diff main --name-only --diff-filter=D | while read f; do grep -rn "$f" crates/ frontend/src/ 2>/dev/null; done

# Barrel export gaps (new .ts/.tsx files not in index.ts)
git diff main --name-only --diff-filter=A -- 'frontend/src/lib/api/*.ts' | while read f; do
  base=$(basename "$f" .ts)
  grep -q "$base" frontend/src/lib/api/index.ts || echo "MISSING EXPORT: $f"
done
```

## 3. Type Consistency

- `FlowStatus`, `TaskStatus` — do all frontend `Record<Status, ...>` maps include every variant?
- `shared/types.ts` — run `npm run generate-types:check` to verify types are current
- Query keys — any inline `['string', id]` instead of factory functions?

## 4. E2E Tests (if servers are running)

Run the main test suite against the affected areas:
```bash
# Main suite (excludes quarantine + demos)
FRONTEND_PORT=<port> npx playwright test --reporter=list

# Or target specific test files for faster feedback
FRONTEND_PORT=<port> npx playwright test e2e/<affected-area>.spec.ts --reporter=list
```

If E2E tests aren't practical (no servers, no seed data), fall back to Playwright MCP smoke tests:
- Navigate to affected pages, verify no blank screen or console errors
- Take snapshots of key UI states

Default pages: `/login`, `/`, `/settings`, `/workflows`

## 5. Summary

| Check | Status |
|-------|--------|
| cargo fmt | PASS/FAIL |
| cargo clippy | PASS/FAIL |
| tsc | PASS/FAIL |
| eslint | PASS/FAIL |
| Conflict markers | N found |
| Dead imports | N found |
| Export gaps | N found |
| Type consistency | PASS/FAIL |
| Smoke tests | N/A or N/N passed |

**Overall**: READY / NEEDS FIXES

## 6. End-to-End Integration Check

For each new backend endpoint in the PR:
- Does a frontend consumer actually CALL it? (grep for the endpoint path in `frontend/src/`)
- If no consumer exists, flag it — an unwired endpoint is incomplete work

For each new frontend feature:
- Does it call the correct backend API? (not a stale/old API)
- Are the response types correct? (match `shared/types.ts`)

This catches the #1 sprint failure pattern: backend ships but frontend never wires up.

## 7. Fix What You Find (Broken Windows)

Don't just report — fix issues as you go:
- **Critical/High**: fix immediately, verify the fix compiles
- **Low-effort** (missing aria-labels, unused imports, wrong types): fix in the same pass
- **Architectural** (needs refactoring): document in planning doc, don't fix

After fixing, re-run the checks that were affected to confirm green.
