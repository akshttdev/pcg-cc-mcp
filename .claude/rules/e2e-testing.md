# E2E Testing Standards

## Red-Green-Refactor

### Red (Write the failing test first)
- Describe expected behavior from the USER's perspective
- The test MUST exercise the UI — never substitute API calls for user interactions
- Run the test — it should FAIL. If it passes immediately, make it more specific
- Tag known-unimplemented features with `test.fixme(true, "reason")` **inside** a test function body

### Green (Make it pass with minimum changes)
- Fix the app, test infrastructure, or selectors to make the test pass
- If a test fails because of missing UI or missing `data-testid`, fix the component (see frontend-standards.md)
- Don't refactor yet — get green first

### Refactor (Clean up without changing behavior)
- Extract repeated setup into helpers
- Consolidate selectors, remove duplication
- Verify tests still pass

## Test Organization

- Group specs by **feature area**, not by page or component
- Each spec file maps to acceptance specs from the backlog
- Spec IDs in test names match backlog IDs: `"DL-1: create a deal via the pipeline board"`
- Feature-area helpers go in `helpers.ts` next to specs; reuse `e2e/helpers/` for auth, timing, cleanup, seed

## Test Design Rules

### UI-first
- Every test step goes through the UI — click, fill, verify visible text
- API calls ONLY for: **setup** (prerequisite data), **cleanup** (`afterAll`), **verification** (backend state the UI doesn't expose)
- If a feature has no UI path, that's a finding — document it, don't work around it

### Selectors (priority order)
1. `getByTestId()` — interactive elements with testids (buttons, menus, cards)
2. `getByRole()` — standard elements with accessible names (links, headings, tabs, textboxes)
3. `getByText()` — content verification
4. `locator()` with CSS — last resort; if needed, add a `data-testid` to the component instead

**Never use:** `.nth(N)`, `div > div > button`, `[class*="..."]`, `xpath=ancestor::`, parent-then-child chains

### Assertions
- `toBeVisible()` over `toBeInTheDocument()` — assert what the user sees
- `t()` timeout helper for all waits — never hardcode timeouts
- Check console errors after key actions

### Test data
- Prefix with `TEST_DATA_PREFIX` ("[E2E]") + `Date.now()` for uniqueness
- Clean up in `afterAll` — never leave test data behind

### Serial execution
- `test.describe.configure({ mode: "serial" })` for multi-step flows
- `test.beforeEach` with `login()` + `navigate()` — each test gets a fresh page
- Share state via module-level variables, not fixtures
- `test.fixme()` MUST be inside a test function body — at describe level it skips the entire block

### Imports
- Use `import { test, expect } from "@playwright/test"` — not custom fixtures
- The `chromium` project provides auth via `storageState`

## When Tests Fail

- **Selector failure**: Fix the component (add `data-testid`), not the test
- **Red (expected)**: Feature not implemented — `test.fixme(true, "reason")`
- **Flaky**: Fix the test (missing waits, race conditions)
- **Regression**: Fix the app
- **Environment**: Fix setup (servers, DB seed)

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
