# E2E Testing — Shared Setup & Conventions

## Environment

```bash
# Frontend must be running
npm run frontend:dev  # Uses FRONTEND_PORT from .env (default 3000)

# Backend must be running — see area-specific TESTING.md for required env vars
npm run backend:dev   # Uses BACKEND_PORT from .env (default 3002)
```

### Test Seed Database

Tests use `dev_assets_seed/test-seed.sqlite` — minimal fixtures (admin user, org, pipeline stages, no entity data). Tests create their own data via API.

```bash
# Create fresh test seed
./scripts/create-test-seed.sh
```

### UUID Format

All database UUIDs must be TEXT format (36-char hyphenated). Migration `20260416000000_normalize_blob_uuids_to_text.sql` converts legacy BLOB UUIDs. If tests hit "not found" errors for entities that exist, check `typeof(id)` in the DB — BLOB UUIDs won't match TEXT bindings.

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

# Exclude quarantine and demos
npx playwright test --reporter=list
```

## Test Data Conventions

- Prefix all test entities with `TEST_DATA_PREFIX` (`"[E2E]"`) + `Date.now()` for uniqueness
- Clean up in `afterAll` — never leave test data behind
- Use `apiLogin(request)` for API setup calls, `login(page)` for browser auth

## Serial Execution

- `test.describe.configure({ mode: "serial" })` for multi-step flows
- Share state via module-level variables, not fixtures
- `test.fixme()` MUST be inside a test function body — at describe level it skips the entire block

## Testid Helpers

Centralized in `shared/testids.ts` — single source of truth for both frontend components and E2E tests.

```typescript
// In spec files — import from area re-export
import { pipeline, deck } from "./testids";

// Area testids.ts re-exports from shared
export { pipeline, deck } from "../../shared/testids";
```

**NEVER hardcode testid strings.** When adding a new testid:
1. Add the helper to `shared/testids.ts`
2. Add `data-testid={tid.foo}` in the component (import from `shared/testids`)
3. Import in the spec via the area's `testids.ts`

## Context Management

- **Save progress to planning docs** so context compaction doesn't lose state
- **Be targeted with MCP** — don't take full page snapshots when you only need one element
- **Use subagents for test runs** — keeps verbose output out of main context
