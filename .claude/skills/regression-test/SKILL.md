---
name: regression-test
description: Full regression analysis of a branch against main — security, correctness, type safety, API contracts, and migration risks
user-invocable: true
allowed-tools: Agent, Bash, Read, Grep, Glob
---

# Regression Test

Run a comprehensive regression analysis of the current branch against `main`. This goes deeper than static checks — it audits security, correctness, and contract integrity.

## Usage

```
/regression-test              # full analysis against main
/regression-test --quick      # skip Rust backend, frontend only
/regression-test --backend    # Rust only
```

## Phase 1: Scope Assessment

```bash
git log --oneline main..HEAD | wc -l
git diff main...HEAD --stat | tail -5
```

Identify: number of commits, files changed, lines added/removed. Flag if >500 files (massive PR — consider splitting).

## Phase 2: Parallel Deep Analysis (use Agent tool, 3 concurrent)

Launch three agents in parallel:

### Agent A: Backend Security & Correctness
Analyze `git diff main...HEAD -- crates/` focusing on:
1. **SQL injection** — string interpolation in queries (not bind params)
2. **Auth bypass** — new endpoints missing `AccessContext` or `require_auth` middleware
3. **Race conditions** — check-then-act patterns on shared state (look for SELECT then UPDATE without atomic guard)
4. **Panics** — `unwrap()`, `expect()`, slice indexing without bounds checks in non-test code
5. **Data loss** — DELETE/UPDATE without WHERE clauses, DROP without backup
6. **Silent error drops** — `let _ =` on critical operations (DB writes, state transitions)
7. **UUID handling** — `uuid::Uuid` in route handlers (should be `DbUuid`), BLOB/TEXT mismatches in JOINs
8. **Status string correctness** — hardcoded task/flow status strings matching enum serialization

### Agent B: Frontend & Type Safety
Analyze `git diff main...HEAD -- frontend/ shared/` focusing on:
1. **Type mismatches** — `as any` casts, wrong types on API responses
2. **Stale closures** — `useEffect` missing deps, stale refs in callbacks
3. **API contract** — frontend sending fields backend doesn't expect, or missing required fields
4. **XSS** — `dangerouslySetInnerHTML`, unsanitized user input in style/class interpolation
5. **Accessibility** — missing aria-labels on interactive elements, keyboard navigation gaps
6. **Error handling** — unhandled promise rejections, missing error states in components
7. **Generated types** — do `shared/types.ts` types match current Rust structs?

### Agent C: Migrations, Config & Infrastructure
Analyze `git diff main...HEAD -- crates/db/migrations/ .github/ .claude/ *.toml *.json *.yml` focusing on:
1. **Migration safety** — reversibility, data loss risk, transaction wrapping
2. **Migration ordering** — timestamp conflicts, dependency issues
3. **CI config** — are checks correct? Any suppressed lints that should be fixed?
4. **Pre-commit hooks** — do they work on macOS and Linux?
5. **Config changes** — breaking changes to .env, Cargo.toml, package.json

## Phase 3: Cross-cutting Analysis

After agents complete, check:
1. **Conflict markers** — `grep -r "<<<<<<" crates/ frontend/ shared/ planning/`
2. **Dead imports** — files that import from deleted/renamed modules
3. **Barrel export gaps** — new files not exported from `index.ts` or `mod.rs`
4. **Query key consistency** — inline query keys vs factory functions
5. **Status enum consistency** — `FlowStatus`, `TaskStatus` maps in frontend match backend variants

## Phase 4: Document Findings

Create/update a findings section in the branch's planning doc with:

```markdown
## Regression Test Results (YYYY-MM-DD)

### Summary
- Files analyzed: N
- Critical: N | High: N | Medium: N | Low: N

### Critical (must fix before merge)
| # | Issue | File:Line | Fix |
|---|-------|-----------|-----|

### High (should fix before merge)
| # | Issue | File:Line | Status |
|---|-------|-----------|--------|

### Medium (tracked for follow-up)
...

### Low (nits)
...

### Clean Areas (verified correct)
...
```

## Phase 5: QA Gate

Run `/qa-review` to verify static checks, structural integrity, and type consistency pass. If dev servers are available, run E2E tests:

```bash
# Full suite
FRONTEND_PORT=<port> npx playwright test --reporter=list

# Quarantine (verify individually before promoting)
FRONTEND_PORT=<port> npx playwright test e2e/quarantine/<test>.spec.ts --reporter=list
```

Note any test failures — distinguish between pre-existing failures (check quarantine list) and new regressions introduced by this branch.

## Phase 6: Fix Critical & Low-Effort Issues

After documenting, fix:
1. All Critical issues
2. All High issues that are low-effort (< 5 min each)
3. Low-effort Medium/Low issues (missing aria-labels, unused imports, etc.)

Do NOT fix issues that require architectural changes — document those for follow-up.

After fixing, re-run affected checks locally to confirm green before pushing. Batch all fixes into a single commit — avoid push-fix-push cycles.

## Output

End with a summary table and merge recommendation:

```
## Merge Recommendation

| Check | Status |
|-------|--------|
| Critical issues | N found, N fixed |
| High issues | N found, N fixed |
| Auth/security | PASS/FAIL |
| Type safety | PASS/FAIL |
| Migration safety | PASS/FAIL |

**Recommendation**: MERGE / NEEDS FIXES / BLOCK
```
