# Manual Demo Walkthrough

Walk through an E2E demo test step-by-step using Playwright MCP, verifying every interaction succeeds and fixing any errors found.

ARGUMENTS: <demo-name> [additional-instructions]

## How to use

```
/demo-walkthrough dealflow-pipeline-demo
/demo-walkthrough bug-report-lifecycle "focus on feedback dialog validation"
/demo-walkthrough workflow-crm-pipeline "verify data source creation works"
```

## Process

### Phase 1: Read the demo test file

1. Find the demo spec file matching the argument:
   - Search `e2e/demos/<demo-name>.spec.ts`
   - If not found, search `e2e/demos/*<demo-name>*`
2. Read the ENTIRE file — every test step, every assertion, every helper call
3. Read all referenced helpers (`e2e/helpers/*.ts`) to understand what each does
4. List all steps in order with their exact actions and assertions

### Phase 2: Verify prerequisites

1. Check the server is running: `lsof -i :$BACKEND_PORT -sTCP:LISTEN`
2. Check the frontend is running: `lsof -i :$FRONTEND_PORT -sTCP:LISTEN`
3. Read `.env` for `FRONTEND_PORT` and `BACKEND_PORT`
4. Check if the demo requires special env vars (GITHUB_TOKEN, ANTHROPIC_API_KEY, etc.)
5. Note which steps will be skipped due to missing env vars

### Phase 3: Execute each step via Playwright MCP

For EVERY step in the demo, execute it exactly as the test code does:

1. **Navigate** — use `mcp__playwright__browser_navigate` to go to the URL
2. **Wait** — use `mcp__playwright__browser_wait_for` for elements/text the test expects
3. **Fill forms** — use `mcp__playwright__browser_fill_form` with exact field values from the test
4. **Click** — use `mcp__playwright__browser_click` on the exact element the test targets
5. **Verify** — after each action, check:
   - The page snapshot shows the expected state
   - Console errors are checked via `mcp__playwright__browser_console_messages`
   - The assertion the test makes is satisfied (element visible, text present, URL changed, etc.)

### Rules

- **NEVER skip steps.** Execute every single step the test code does.
- **NEVER assume a step works** — verify the snapshot after every action.
- **Check console errors** after each page navigation and after form submissions.
- **If an error occurs**, investigate immediately:
  - Read the error message
  - Check the backend logs if it's a 500 error
  - Check the frontend code if it's a rendering error
  - Fix the issue before continuing
- **If a step requires a helper function** (e.g., `createDemoProject`, `apiLogin`), execute the equivalent actions:
  - For API helpers: use `curl` via Bash to call the API directly
  - For UI helpers: execute the same clicks/fills the helper does
- **Log each step result**: PASS (with brief proof) or FAIL (with error details)

### Phase 4: Report

After completing all steps, produce a summary:

```
## Demo Walkthrough: <demo-name>

### Steps Completed
1. Step 1: <name> — PASS
2. Step 2: <name> — PASS
3. Step 3: <name> — FAIL (reason)
...

### Errors Found & Fixed
- <error description> → <fix applied>

### Steps Skipped (env-dependent)
- Step N: <name> — requires GITHUB_TOKEN

### Console Errors
- <list of any console errors seen, with page context>
```

### Environment Variables

Env vars are in the ROOT worktree `.env` (not the current worktree):
- `GITHUB_TOKEN` — at `/Users/mediamonsters/topos/pcg-cc-mcp/.env`
- `ANTHROPIC_API_KEY` — at `/Users/mediamonsters/topos/pcg-cc-mcp/.env`
- Playwright config auto-loads from root `.env` via `dotenv`
- For API calls during walkthrough, source the root `.env`:
  `source /Users/mediamonsters/topos/pcg-cc-mcp/.env`
- NEVER skip steps that require GITHUB_TOKEN or ANTHROPIC_API_KEY — these keys ARE available
- For Bash API calls, export the token: `export GITHUB_TOKEN=$(grep GITHUB_TOKEN /Users/mediamonsters/topos/pcg-cc-mcp/.env | cut -d= -f2)`

### Helper Functions

- The login helper uses credentials from `e2e/helpers/auth.ts` (typically admin/admin123)
- The `openFeedbackDialog` helper clicks "More" → "Feedback & Support" in the sidebar
- The `createDemoProject` helper creates a project via POST /api/projects under Powerclub Global org
- The `navigateToProjectTasks` helper navigates to `/projects/{id}/tasks` and waits for "Create new task"
- The `createTaskViaUI` helper clicks "Create new task", fills title, clicks "Create Task", waits for "Agent Reviewers"
- The `ensureAgentsSeeded` helper calls POST /api/agents/seed — returns array directly (not wrapped)
- The `addQaWatcher` helper clicks "Add" button then "ORCHA QA" button, waits for "Watching"
- The `changeTaskStatus` helper clicks the status combobox, selects new status, waits for toast
- The `findTaskCard` helper finds a button matching the task title regex
- Demo pause values: short=500ms, medium=1000ms, long=2000ms — you can skip these
- Tests use `t()` timeout scaler — default 1x, can be set via TIMEOUT_SCALE env var

### GitHub Demo Helpers (e2e/helpers/demo/)

- `simulateDevAgentWork(request, taskId)` — creates branch, commit, PR, links to task, changes status
- `simulateQaVerdict(request, taskId, agentId, verdict)` — posts QA verdict via API
- `postDevAgentSummaryComment(request, prNumber, taskId, title)` — posts dev summary on PR
- `postQaReviewComment(request, prNumber, taskId, verdict, details)` — posts QA review on PR
- `fetchPrComments(request, prNumber)` — fetches PR comments from GitHub API
- `getPrUrl(prNumber)` — returns GitHub PR URL for the sandbox repo
- `cleanupDemoBranches(request, branches)` — deletes demo branches
- `cleanupDemoPr(request, prNumber)` — closes demo PR
