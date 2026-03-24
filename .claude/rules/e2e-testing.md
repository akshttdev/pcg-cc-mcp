# E2E Testing Standards

## Red-Green-Refactor

### Red (Write the failing test first)
- Describe expected behavior from the USER's perspective
- The test MUST exercise the UI — never substitute API calls for user interactions
- Run the test — it should FAIL. If it passes immediately, make it more specific
- Tag known-unimplemented features with `test.fixme(true, "reason")` **inside** a test function body
- When restructuring or adding tests: write ALL new tests first, run them, confirm they fail for the right reasons. Do NOT write implementation alongside the test.
- If you have an implementation in mind, add it **commented out** — the test must fail when run
- Commit failing tests before making them green. This proves the tests actually test something.

### Green (Make it pass with minimum changes)
- Fix the app, test infrastructure, or selectors — one test at a time
- If a test fails because of missing `data-testid`, fix the component (see frontend-standards.md)
- Don't refactor yet — get green first

### Refactor (Clean up without changing behavior)
- Extract repeated setup into helpers
- Consolidate selectors, remove duplication
- Verify tests still pass

## Test Organization

- Group specs by **user journey**, not by spec ID or component
- Prefer one end-to-end flow over many isolated slices — a single entity flowing through its lifecycle tests more realistically than separate files each creating their own data
- Separate specs only when setup is fundamentally different (e.g., testing validation requires data that conflicts with the happy-path flow)
- Reference acceptance specs from the backlog in comments
- **Testid helpers live in `shared/testids.ts`** — the single source of truth for both frontend components and E2E tests
- Feature-area E2E files re-export from shared: `export { pipeline, deck } from '../../shared/testids'`
- **NEVER hardcode testid strings** in spec files — always import from the helpers
- When a test needs a new testid: add the helper to `shared/testids.ts`, add `data-testid={tid.foo}` in the component, import in the spec
- Feature-area helpers go in `helpers.ts` next to specs; reuse `e2e/helpers/` for auth, timing, cleanup, seed

## Test Design Rules

### UI-first
- Every test step goes through the UI — click, fill, verify visible text
- API calls ONLY for: **setup** (prerequisite data), **cleanup** (`afterAll`), **verification** (backend state the UI doesn't expose)
- If a feature has no UI path, that's a finding — document it, don't work around it

### Selectors (priority order)
1. `getByTestId()` — interactive elements with testids (buttons, menus, cards, form fields)
2. `getByRole()` — standard elements with accessible names (links, headings, tabs, textboxes)
3. `getByTitle()` — elements with title attributes
4. `getByText()` — content verification
5. `locator()` with CSS — last resort; if needed, add a `data-testid` to the component instead

**Never use:** `.nth(N)`, `div > div > button`, `[class*="..."]`, `xpath=ancestor::`, parent-then-child chains

### Assertions — test behavior, not existence
- Every assertion must verify a **user-visible outcome**, not just that an element exists
- `toBeVisible()` alone is an existence check — pair it with interaction and a behavioral outcome (toast appears, data persists, item moves)
- After filling a form and clicking Save: verify toast, then verify persistence (reload or API check)
- After moving/creating/deleting: verify the change is reflected in the correct location
- `t()` timeout helper for all waits — never hardcode timeouts
- Check console errors after key actions

### Test data
- Prefix with `TEST_DATA_PREFIX` ("[E2E]") + `Date.now()` for uniqueness
- Clean up in `afterAll` — never leave test data behind

### Serial execution
- `test.describe.configure({ mode: "serial" })` for multi-step flows
- Share state via module-level variables, not fixtures
- `test.fixme()` MUST be inside a test function body — at describe level it skips the entire block

### Acceptance specs as comments
- Every test block must include the acceptance scenario it implements as a comment (Given/When/Then format)
- Source the scenario from: (1) Gherkin specs in the backlog if they exist, (2) planning files or feature instructions, (3) generate your own from the feature description — ask the user if unclear
- Every "Then" line MUST be a real assertion in the test
- If a "Then" line can't be tested yet, add a `test.fixme()` test for it

## Writing Tests: Inspect First, Then Write

**Before writing ANY test**, use Playwright MCP to walk through the user flow interactively:
1. Navigate to the page, take a snapshot
2. Perform each action (click, fill, hover), snapshot after each
3. Note the actual element names, roles, and structure from the snapshot
4. Write the test to match what you observed — not what you assume

**Never guess selectors.** The snapshot shows exact button names, exact roles, exact text content. Use those.

**Verify with the app, not with theories.** If a test fails, reproduce the steps via MCP before debugging code. Most "bugs" are wrong selectors or missing waits.

**Button text changes are state assertions.** `"Expand"` → `"Minimize"` proves the transition happened. Checking for a container role does not.

**Wait for transitions.** Panel open/close, tab switches, modal transitions need `demoPause` waits.

## When Tests Fail

- **Selector failure**: First reproduce via Playwright MCP snapshot. Then fix the component (add `data-testid`) or fix the selector.
- **Red (expected)**: Feature not implemented — `test.fixme(true, "reason")`
- **Flaky**: Fix the test (missing waits, race conditions)
- **Regression**: Fix the app
- **Environment**: Fix setup (servers, DB seed)

## Context Management

- **Save progress to the planning doc** periodically so context compaction doesn't lose state
- **Be targeted with MCP** — don't take full page snapshots when you only need one element's selector
- **Use subagents for test runs** when feasible — keeps verbose test output out of main context
- **Compact proactively** before context gets forced — not every cycle, but don't wait until it's required either

## Running Tests

```bash
# Specific folder
FRONTEND_PORT=3000 npx playwright test e2e/<folder>/ --reporter=list

# Specific file
FRONTEND_PORT=3000 npx playwright test e2e/<folder>/<file>.spec.ts

# Headed + slow for debugging
E2E_HEADED=true FRONTEND_PORT=3000 npx playwright test e2e/<folder>/

# List tests without running
npx playwright test e2e/<folder>/ --list
```
