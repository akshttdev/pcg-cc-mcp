---
paths:
  - "e2e/**/*.ts"
  - "e2e/**/*.spec.ts"
---

# E2E Testing Standards

Core principles for all E2E test work. Area-specific setup and rules are in:
- `e2e/TESTING.md` — shared environment, running tests, test data conventions
- `e2e/pipeline/TESTING.md` — pipeline stage ownership, agent simulation, testid helpers
- `e2e/demos/TESTING.md` — demo-specific setup (GITHUB_TOKEN, timing, headed mode)

## Red-Green-Refactor

### Red (Write the failing test first)
- Describe expected behavior from the USER's perspective
- The test MUST exercise the UI — never substitute API calls for user interactions
- Run the test — it should FAIL. If it passes immediately, make it more specific
- Tag known-unimplemented features with `test.fixme(true, "reason")` **inside** a test function body
- When restructuring or adding tests: write ALL new tests first, confirm they fail for the right reasons
- Commit failing tests before making them green

### Green (Make it pass with minimum changes)
- Fix the app, test infrastructure, or selectors — one test at a time
- If a test fails because of missing `data-testid`, fix the component (see frontend-standards.md)

### Refactor (Clean up without changing behavior)
- Extract repeated setup into helpers, verify tests still pass

## Test Design Rules

### UI-first
- Every test step goes through the UI — click, fill, verify visible text
- API calls ONLY for: **setup**, **cleanup** (`afterAll`), **verification** (backend state the UI doesn't expose)

### Selectors (priority order)
1. `getByTestId()` — interactive elements with testids
2. `getByRole()` — standard elements with accessible names
3. `getByTitle()` — elements with title attributes
4. `getByText()` — content verification
5. `locator()` with CSS — last resort; add a `data-testid` instead

**Never use:** `.nth(N)`, `div > div > button`, `[class*="..."]`, `xpath=ancestor::`, parent-then-child chains

### Assertions — test behavior, not existence
- Every assertion must verify a **user-visible outcome**, not just that an element exists
- `t()` timeout helper for all waits — never hardcode timeouts

### Acceptance specs as comments
- Every test block includes the acceptance scenario (Given/When/Then format)
- Every "Then" line MUST be a real assertion

## Writing Tests: Inspect First, Then Write

**Before writing ANY test**, use Playwright MCP to walk through the user flow interactively. Never guess selectors — the snapshot shows exact button names, roles, and text content.

## When Tests Fail

- **Selector failure**: Reproduce via MCP snapshot first
- **Red (expected)**: `test.fixme(true, "reason")`
- **Flaky**: Fix missing waits or race conditions
- **Regression**: Fix the app
- **Environment**: Fix setup (see `e2e/TESTING.md`)
