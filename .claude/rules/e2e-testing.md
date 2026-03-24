# E2E Testing Standards

## Red-Green-Refactor for E2E Tests

Follow the **Red → Green → Refactor** cycle for every test:

### Red (Write the failing test first)
- Write the test spec describing expected behavior from the USER's perspective
- The test MUST exercise the UI — never substitute API calls for user interactions
- Run the test. It should FAIL — either because the feature isn't implemented, the selector is wrong, or the test infrastructure is missing
- If it passes immediately, the test isn't testing anything new — make it more specific
- Tag known-failing tests with `test.fixme(true, "reason")` — not `test.skip`

### Green (Make it pass with minimum changes)
- Fix the application code, test infrastructure, or selectors to make the test pass
- Don't refactor yet — get the green checkmark first
- If a test fails because of missing UI elements (no button, no link), that's a real finding — fix the app, not the test

### Refactor (Clean up without changing behavior)
- Extract repeated setup into helpers
- Consolidate selectors into constants
- Remove duplication between test files
- Verify tests still pass after refactoring

## Test Organization

### File structure
- Group specs by **feature area**, not by page or component
- Each spec file maps to one or more **acceptance specs** from the backlog
- Spec IDs in test names match backlog IDs (e.g., `"DL-1: create a deal via the pipeline board"`)
- Test names are human-readable user actions, not technical descriptions

### Shared infrastructure
- `fixtures.ts` — shared page state for serial multi-step flows (same pattern as `e2e/demos/fixtures.ts`)
- `helpers.ts` — feature-area-specific helpers (navigate, create entity, interact with UI)
- Reuse helpers from `e2e/helpers/` (auth, timing, cleanup, seed) — don't reinvent them

## Test Design Rules

### UI-first, always
- Every test step must go through the UI — click buttons, fill forms, verify visible text
- API calls are ONLY allowed for:
  - **Setup**: creating prerequisite data that isn't part of the test flow (e.g., creating a contact before testing deal creation)
  - **Cleanup**: deleting test data in `afterAll`
  - **Verification**: confirming backend state when the UI doesn't expose it (e.g., checking DB after a save)
- If a feature has no UI path, that's a **finding** — document it, don't work around it with API calls

### Selectors
- Prefer `getByRole()` over `getByTestId()` — tests should match what users see
- Use `getByText()` for content verification
- Use `getByRole("button", { name: "Add Deal" })` for interactions
- Only fall back to `locator()` with CSS selectors when role/text selectors can't work (e.g., icon-only buttons)
- Never use fragile selectors like `.nth(3)` or `div > div > button` — if you need these, the component needs better accessibility

### Assertions
- Assert visible state, not DOM state — `toBeVisible()` over `toBeInTheDocument()`
- Assert user-meaningful outcomes — "deal card appears in column" not "DOM has child element"
- Use `t()` timeout helper for all waits — never hardcode timeouts
- Check console errors after key actions — silent JS errors are test failures

### Test data
- Prefix all test data with `TEST_DATA_PREFIX` ("[E2E]")
- Include `Date.now()` in names for uniqueness across parallel runs
- Always clean up in `afterAll` — never leave test data behind
- Use seed helpers for prerequisite data that the test itself doesn't need to exercise

### Serial execution
- Use `test.describe.configure({ mode: "serial" })` for multi-step flows where state carries across tests
- Share state between steps via module-level variables (not test fixtures)
- Each test in a serial block should be independently understandable but depend on prior steps completing

## Acceptance Spec Mapping

Tests should trace back to acceptance specs in the backlog. Each test name should include the spec ID:

```typescript
// Good — traceable to backlog spec
test("DL-1: create a deal via the pipeline board", async ({ page }) => { ... });

// Bad — no traceability
test("should create deal", async ({ page }) => { ... });
```

When a backlog spec has multiple scenarios, each scenario becomes a separate test:

```typescript
test("DL-3: won deal creates delivery pipeline entry", async ({ page }) => { ... });
test("DL-3: won deduplicates by contact", async ({ page }) => { ... });
```

## When Tests Fail

- **Red (expected)**: The test was just written and the feature isn't implemented yet. Tag with `test.fixme(true, "DL-3: Won chain doesn't create Client/Project yet")`
- **Flaky**: If a test passes sometimes and fails sometimes, it's a test bug — fix the test (usually missing waits or race conditions)
- **Regression**: If a previously-passing test fails, it's an app bug — fix the app
- **Environment**: If the test fails because servers aren't running or DB is stale, that's setup — not a test or app issue

## Running Tests

```bash
# Run a specific spec folder
FRONTEND_PORT=3000 npx playwright test e2e/<folder>/ --reporter=list

# Run a specific spec file
FRONTEND_PORT=3000 npx playwright test e2e/<folder>/<file>.spec.ts

# Run headed (visible browser) for debugging
E2E_HEADED=true FRONTEND_PORT=3000 npx playwright test e2e/<folder>/

# Run with demo pacing (slow, for watching)
E2E_HEADED=true E2E_DEMO_PACE=long FRONTEND_PORT=3000 npx playwright test e2e/<folder>/
```

## Environment
- Tests expect servers already running (no auto-start)
- Auth credentials come from `e2e/helpers/auth.ts` (`TEST_USER`)
- Test seed DB should be compatible with current migrations
- Port configured via `FRONTEND_PORT` env var (default 3000)
