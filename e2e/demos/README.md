# E2E Demo Scripts

Feature demonstration scripts for the ORCHA platform. These walk through
complete user flows in a linear, sequential manner — designed to showcase
functionality rather than exhaustively test edge cases.

## Principle

All demo scripts drive the **actual app UI** — clicking buttons, filling forms,
navigating pages. API calls are only used for prerequisite state that is out of
scope for the demo (e.g., ensuring an organization or project exists).

| What                          | API OK? | UI Required? |
|-------------------------------|---------|-------------|
| Org/project existence         | Yes     | No          |
| Auth/login                    | Yes     | No          |
| Creating tasks                | No      | Yes         |
| Creating data sources         | No      | Yes         |
| Building workflows            | No      | Yes         |
| Status changes                | No      | Yes         |
| Adding watchers               | No      | Yes         |
| Notification interactions     | No      | Yes         |
| Running workflows             | No      | Yes         |
| Reviewing staging             | No      | Yes         |

## Scripts

| Script | Flow |
|--------|------|
| `bug-report-lifecycle.spec.ts` | Submit bug via Feedback dialog → find on kanban → add QA watcher → status progression (To Do → In Progress → In Review → Done) |
| `manual-qa-trigger.spec.ts` | Create task via UI → add QA watcher → move to In Review → verify watcher behavior (no PR path) → complete to Done |
| `notification-center.spec.ts` | Create task to generate activity → bell icon → dropdown → activity items → mark all read → create another task → click notification to navigate |
| `workflow-crm-pipeline.spec.ts` | Create conversation data source → build 7-node extraction workflow in Workflow Builder → run against source → review staging → verify CRM |

## Running

```bash
# Run all demos (always headed — browser stays open between tests)
npx playwright test --project=demos

# Run a single demo
npx playwright test --project=demos e2e/demos/bug-report-lifecycle.spec.ts

# Custom slow-motion speed (default 250ms)
E2E_SLOWMO=500 npx playwright test --project=demos

# Custom frontend port
FRONTEND_PORT=3001 npx playwright test --project=demos
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `E2E_SLOWMO` | `250` | Milliseconds to slow down each action |
| `FRONTEND_PORT` | `3000` | Port the frontend dev server listens on |
| `E2E_BROWSER` | `chromium` | Browser engine: `chromium`, `firefox`, `webkit` |
| `E2E_SCREENSHOTS` | `false` | Set to `"true"` for failure screenshots |

## Workflow CRM Pipeline Demo

The most comprehensive demo. It exercises:

1. **Data Source Creation** — creates a conversation-type data source via the
   organization profile UI, containing a simulated client meeting transcript
   with multiple participants, companies, deals, and action items.

2. **Workflow Builder** — creates a 7-node workflow entirely through the UI:
   - `Data Source` → 3× `LLM Extract` (contacts, companies, deals) → 3× `Output CRM`
   - Each extract node gets a tailored prompt template
   - Nodes are connected via the input connection dropdowns

3. **Workflow Execution** — runs the workflow against the data source from the
   data sources page, producing staged records for review.

4. **Staging Review & CRM Verification** — checks staged records and verifies
   that contacts (Marcus Webb, Lisa Park, Raj Patel), companies (Acme Corp,
   GlobalTech Industries), and deals ($450K enterprise license, $200K analytics
   platform) appear in the CRM.

**Note:** Steps involving workflow execution require a working LLM backend
(PCG Router with model access). Without it, the workflow execution will log
"no staged records" rather than failing.

## Notes

- These scripts use dedicated demo fixtures (`e2e/demos/fixtures.ts`) that
  always share a single browser window — no open/close between tests.
- The manual QA trigger demo verifies the "no PR" path since we can't create
  real GitHub PRs in a local test environment.
- All test-created data is prefixed with `[E2E]` for easy cleanup.
- Serial mode (`test.describe.configure({ mode: "serial" })`) ensures steps
  run in order within each demo.
