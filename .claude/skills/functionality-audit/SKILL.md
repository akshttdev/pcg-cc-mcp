---
name: functionality-audit
description: Verify planned features actually WORK — detect stubs, unwired routes, false DONE claims, and broken UX paths via code trace + Playwright walkthrough
user-invocable: true
allowed-tools: Agent, Bash, Read, Write, Edit, Glob, Grep, mcp__sequential-thinking__sequentialthinking, mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_click, mcp__playwright__browser_fill_form, mcp__playwright__browser_wait_for, mcp__playwright__browser_console_messages, mcp__playwright__browser_take_screenshot, mcp__playwright__browser_hover, mcp__playwright__browser_press_key
---

# Functionality Audit

Verify that planned features actually WORK — not just compile. This skill bridges the gap between static QA (`/qa-review`) and regression analysis (`/regression-test`) by testing whether handlers do real work, routes are wired, and users can discover and complete features end-to-end.

## Arguments

`$ARGUMENTS` — path to a planning file (e.g., `planning/2026-03-19--plan--tier1-phase0-sprint.md`).

If no argument provided, use the most recent `planning/*.md` on the current branch.

## Verdict Taxonomy

Each feature gets one verdict:

- **WORKING**: handler does real work, API matches frontend, user can discover + complete happy path, result persists
- **PARTIAL**: mostly works but missing error handling, loading states, access control, or discoverability issues (feature works but is buried 3+ clicks deep with no obvious entry point)
- **STUB**: compiles, route exists, but handler returns dummy data or logs without acting
- **NOT WIRED**: route not registered, no navigation path, migration exists but model ignores columns

**Discoverability qualifier** (append to any verdict):
- **(easy)**: feature is 1-2 clicks from main navigation
- **(buried)**: feature requires 3+ clicks, hidden in "More" menus, admin-only tabs, or undocumented URL paths
- **(undiscoverable)**: no UI path exists — only reachable via direct URL or API call

---

## Phase 0: Parse Plan & Extract Features

Use Sequential Thinking (`mcp__sequential-thinking__sequentialthinking`, scale thoughts with feature count: `min(features, 8)`) to:

1. Read the planning file completely
2. Extract every discrete feature with its claimed behavior **and its self-reported status** (DONE, PARTIAL, STUB, etc.)
3. Classify each feature:
   - **frontend-only**: UI component, page, navigation change
   - **backend-only**: API endpoint, migration, background job
   - **full-stack**: backend endpoint + frontend consumer
   - **infrastructure**: CI, config, build tooling
4. Determine audit strategy per feature:
   - **full-stack / frontend-only**: Playwright walkthrough (if servers running) + code trace
   - **backend-only**: code trace (route → handler → DB)
   - **infrastructure**: build/config verification

Output a numbered feature list with classification, plan-reported status, and strategy.

## Phase 1: Environment & Build Verification

**Run this BEFORE any code trace or Playwright work.** A broken environment wastes all downstream effort.

### 1a. Database health

```bash
# Check seed DB exists and server can start
ls dev_assets/db.sqlite 2>/dev/null || echo "MISSING: reseed from dev_assets_seed/"

# Quick migration check — does the latest migration in code match what's applied?
ls crates/db/migrations/*.sql | tail -1  # latest migration file
sqlite3 dev_assets/db.sqlite "SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1;" 2>/dev/null
# If they don't match, the server will crash on startup. Reseed:
# rm -f dev_assets/db.sqlite && cp dev_assets_seed/db.sqlite dev_assets/db.sqlite
```

### 1b. Server health

```bash
source .env 2>/dev/null
FPORT=${FRONTEND_PORT:-3000}
BPORT=${BACKEND_PORT:-3002}
lsof -i :$FPORT -P 2>/dev/null | grep LISTEN  # frontend running?
lsof -i :$BPORT -P 2>/dev/null | grep LISTEN  # backend running?
```

If servers aren't running, ask the user to start them (`pnpm run dev`). If the backend crashes on startup (migration error, missing column), fix the seed DB first — see README.md Troubleshooting.

### 1c. Build checks

```bash
# TypeScript (fast)
cd frontend && npx tsc --noEmit

# Type generation
npm run generate-types:check

# Production build (catches Vite resolution errors)
cd frontend && npx vite build

# Rust tests (slow — run in background)
flox activate -- cargo test --workspace
```

If any check fails, determine whether the failure is:
- **Sprint-caused**: a regression from this sprint's changes → Must Fix
- **Pre-existing**: an environment or dependency issue → note as **Build Environment Blocker**

### 1d. Type pipeline check

```bash
# Check for types in bindings/ not collected into shared/types.ts
# (ts-rs generates into crates/db/bindings/, but shared/types.ts is the canonical frontend source)
for f in crates/db/bindings/*.ts; do
  name=$(basename "$f" .ts)
  grep -q "$name" shared/types.ts 2>/dev/null || echo "MISSING FROM shared/types.ts: $name"
done
```

## Phase 2: Backend Wiring Audit

Launch parallel agents (max 5) to audit each backend feature. **Group features by PR or subsystem** — features in the same PR often share routes, models, and access patterns, making them efficient to audit together. If the sprint has >10 features, group 2-3 per agent.

Per feature, check:

1. **Route registered?** — Is the route in a `.route()` call in the router? Grep for the path pattern.
2. **Handler does real work?** — Read the handler function. Does it:
   - Query/mutate the database? (look for `sqlx::query`, model calls)
   - Or just return `Ok(Json(...))` with hardcoded/dummy data?
   - Or just log and return `Ok(())`?
3. **Access control?** — Does the handler check `AccessContext`, org membership, or permissions? Or is it wide open?
4. **Database wired?** — If the feature involves new columns/tables:
   - Does the migration exist and apply the schema change?
   - Does the model's query actually SELECT/INSERT those columns?
   - Or does the migration add columns that no query ever touches?
5. **API contract matches frontend?** — Compare the Rust response struct (with `#[derive(TS)]`) to what the frontend expects. Check `shared/types.ts`.

### False DONE Detection (highest-priority finding)

**For every feature the plan marks as DONE, verify the claim end-to-end.** A false DONE is more dangerous than an honest PARTIAL — it means the team thinks something works when it doesn't. Check:

- If the plan says "frontend calls new API" — grep the frontend for the actual import and call site
- If the plan says "endpoint returns real data" — read the handler and confirm it queries the DB
- If the plan says "migration adds column" — verify a model query actually SELECTs/INSERTs that column
- If the plan says "types generated" — verify they appear in `shared/types.ts`, not just `crates/db/bindings/`

### Stub Detection Heuristics

A handler is likely a **STUB** if:
- It contains `todo!()`, `unimplemented!()`, or `// TODO`
- It returns a hardcoded value without touching the database
- It logs a message and returns `Ok(())`
- It accepts parameters but never uses them
- The response struct has fields but the handler populates them with defaults/zeros

A handler is likely **NOT WIRED** if:
- The function exists but no `.route()` or `.method_router()` references it
- The route path exists but points to a different handler
- The migration creates a table/column but no model query references it

A type is likely a **STUB** if:
- It has `#[derive(TS)]` and generates bindings in `crates/db/bindings/` but does NOT appear in `shared/types.ts`
- It is defined but never consumed (no handler returns it, no executor parses it, no frontend imports it)

## Phase 3: Frontend UX Walkthrough

**Only run this phase if Phase 1 confirmed servers are healthy and app renders.**

### Authentication

If redirected to `/login`, authenticate first:
1. Read credentials from `.env` (`E2E_USERNAME`, `E2E_PASSWORD`) or fall back to defaults from the test seed
2. Fill username and password fields, click Sign in
3. Wait for redirect to dashboard
4. Take snapshot to confirm logged in

### Per-feature walkthrough

For each full-stack or frontend-only feature:

#### Discovery (most common audit failure — be thorough)

Navigate to where the feature should live. Features are often buried — check ALL of these:
- **Primary nav**: sidebar links, top-level tabs
- **"More" menus**: sidebar "More" popover, tab bar "More" dropdown — hidden features live here
- **Scope toggles**: Settings page has User/Admin/Org/Client tabs — features may only appear under specific scopes
- **Nested sub-tabs**: Some pages have secondary tab bars with a "More" dropdown hiding additional tabs
- **Admin-only sections**: Some features only appear for admin users
- **Direct URL**: Try navigating to the expected URL path directly — the page may exist but have no nav link

Record the full click path (e.g., "Sidebar → More → [menu item] → [sub-option]"). If the path is 3+ clicks, flag as **(buried)** in the verdict.

#### Happy Path
- Click/interact to trigger the feature
- Fill any required forms
- Submit and wait for response
- Take snapshot of result — did the action complete?
- Check persistence: navigate away and back — is the result still there?

#### Empty State vs Broken
- If the feature shows an empty state ("No items", "No data"), this is likely **correct behavior** with no seed data — not a bug
- Distinguish: "empty because no test data exists" vs "empty because the API call failed" (check console errors)
- If no seed data exists to test the feature, note it and fall back to code trace for that feature's rendering behavior

#### Error & Edge Cases
- Check `browser_console_messages` for JavaScript errors after each navigation
- Look for: loading spinners that never resolve, error toasts, blank content areas
- Try submitting empty/invalid forms — does error feedback appear?

#### Visual Quick-Check
- Snapshot key states — look for layout shifts, overlapping text, missing icons
- Check responsive behavior if relevant (resize to mobile width)

### If dev servers are NOT running (code trace fallback):

For each frontend feature:
1. Find the React component that renders the feature
2. Trace the data flow: component → hook → API call → endpoint
3. Check: does the API call URL match a registered backend route?
4. Check: does the component handle loading, error, and empty states?
5. Check: is there a navigation path to reach this component? (router config, sidebar, links)

## Phase 4: Cross-Cutting Verification

Use Sequential Thinking (scale with feature count: `max(8, min(features, 15))` thoughts) to analyze:

1. **Race conditions**: Are there check-then-act patterns? (e.g., "check if exists, then create" without a transaction)
2. **Observability**: Do new features have any logging/metrics, or are they silent?
3. **Integration gaps**: Features that touch the same data — do they coordinate? (e.g., two endpoints writing to the same table, API client exists but page imports old client)
4. **Missing config**: Environment variables referenced but not documented, feature flags without defaults
5. **Type pipeline gaps**: Types generated by ts-rs into `crates/db/bindings/` but not collected into `shared/types.ts` — frontend can't import them from the canonical location
6. **Plan accuracy synthesis**: Compare each feature's plan-reported status against audit findings. Flag any false DONE claims.

## Phase 5: Generate Report

Create `planning/reviews/YYYY-MM-DD--review--functionality-audit.md` with:

```markdown
# Functionality Audit Report

**Date**: YYYY-MM-DD
**Planning File**: `path/to/planning-file.md`
**Branch**: `current-branch`
**Audit Method**: Playwright walkthrough / Code trace only / Mixed

## Feature Verdicts

| # | Feature | Type | Plan Says | Actual | Accurate? | Issue |
|---|---------|------|-----------|--------|-----------|-------|
<!-- One row per feature extracted in Phase 0. Append discoverability qualifier: (easy), (buried), (undiscoverable) -->

## Detailed Findings

### Feature 1: [name]
- **Plan status**: DONE / PARTIAL / STUB
- **Audit verdict**: WORKING / PARTIAL / STUB / NOT WIRED
- **Plan accurate?**: Yes / **NO — [explanation]**
- **Route**: `POST /api/tasks` → `create_task` handler
- **Handler**: Does real DB insert via `Task::create()`
- **Frontend**: TaskCard renders, form submits, result persists
- **Playwright evidence** (if tested):
  - Discovery path: [click path to reach feature]
  - Renders: Yes/No — [what was visible]
  - Console errors: N errors — [details if any]
  - Empty state: [correct empty state / broken / N/A]
- **Issues**: None

### Feature 2: [name]
...

## Action Items

### Must Fix (blocks shipping)
- [ ] [description] — [file:line]

### Should Fix (quality/UX gaps)
- [ ] [description] — [file:line]

### Nice to Have (polish)
- [ ] [description] — [file:line]

## Build Verification

- [ ] `npx tsc --noEmit` — PASS/FAIL
- [ ] `npx vite build` — PASS/FAIL
- [ ] `npm run generate-types:check` — PASS/FAIL
- [ ] `cargo test --workspace` — PASS/FAIL
- [ ] Type pipeline (bindings/ → shared/types.ts) — N types missing
- [ ] Database seed compatible with migrations — YES/NO

### Build Environment Blockers (pre-existing, not sprint-caused)
- [description of any pre-existing build/infra issues that blocked verification]

## Overall Verdict

**SHIP** / **SHIP WITH CAVEATS** / **HOLD**

[Rationale — 1-2 sentences]
```

### Verdict Criteria

- **SHIP**: All features WORKING, no Must Fix items
- **SHIP WITH CAVEATS**: Most features WORKING, remaining are PARTIAL with tracked action items, no STUB/NOT WIRED that were supposed to be complete. False DONE claims documented.
- **HOLD**: Any STUB or NOT WIRED features that were supposed to be complete, or Must Fix items unresolved

## Phase 6: Update Planning File

Add a summary section to the original planning file:

```markdown
## Functionality Audit (YYYY-MM-DD)

**Overall**: SHIP / SHIP WITH CAVEATS / HOLD
**Report**: `planning/reviews/YYYY-MM-DD--review--functionality-audit.md`
**Features**: N WORKING, N PARTIAL, N STUB, N NOT WIRED
**Plan Accuracy**: N/N features matched plan status (N false DONE claims)
**Action Items**: N Must Fix, N Should Fix, N Nice to Have
```

---

## Important Rules

- Do NOT fix issues — this is an audit, not a repair job. Document everything for follow-up.
- Do NOT modify source code (only planning/report files).
- Do NOT run destructive git operations.
- Do NOT skip features — every discrete feature in the plan gets a verdict.
- Always confirm worktree before making changes.
- When Playwright walkthrough is used, always check console messages — JS errors are a signal.
- If a feature has no frontend surface, still verify the backend wiring fully.
- Be honest about STUB verdicts — a handler that compiles but does nothing useful is a STUB, not PARTIAL.
- **False DONE is the highest-priority finding** — a feature the team thinks works but doesn't is worse than an honest STUB.
- Distinguish sprint-caused failures from pre-existing environment issues in build verification.
- **Environment first** — always verify database + servers before launching agents or Playwright. A stale seed DB or crashed backend wastes all downstream work.
- **Discovery requires depth** — features hide in More menus, scope tabs, and nested sub-tabs. Check all of these before concluding a feature is undiscoverable.
- **Empty state ≠ broken** — if a page shows "No items" with no console errors, the feature likely works but lacks seed data. Note it and verify rendering logic via code trace.
