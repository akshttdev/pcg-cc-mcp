---
name: demo-walkthrough
description: Walk through an E2E demo test step-by-step via Playwright MCP, verifying every interaction and fixing errors
user-invocable: true
---

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
- **Verify OBJECTIVES, not just visibility.** For each step, understand:
  - What is the test CHECKING? (e.g., "task moved to Done column" not just "Done is visible")
  - What state change should have occurred? (e.g., kanban column counts changed)
  - What user-facing result confirms success? (e.g., toast message, card in correct column, drawer content)
- **Verify state transitions.** When data changes (status, column, count, badge), navigate to the relevant view and confirm the change is reflected in the UI — not just that the page loaded.
- **Check console errors** after each page navigation and after form submissions.
- **If an error occurs**, investigate immediately:
  - Read the error message
  - Check the backend logs if it's a 500 error
  - Check the frontend code if it's a rendering error
  - Fix the issue before continuing — nothing is out of scope
- **If a step requires a helper function** (e.g., `createDemoProject`, `apiLogin`), execute the equivalent:
  - For API helpers: use `curl` via Bash to call the API directly
  - For UI helpers: execute the same clicks/fills the helper does via Playwright MCP
  - For GitHub helpers: use the GitHub API via curl with GITHUB_TOKEN
- **UI interactions must go through Playwright MCP** — never substitute API calls for steps the test does via UI (creating tasks, clicking buttons, filling forms, changing status via combobox)
- **Log each step result**: PASS (with what was verified) or FAIL (with error details)

### Before starting

1. Reset the DB from clean seed: `cp dev_assets_seed/test-seed.sqlite dev_assets/db.sqlite`
2. Restart the server to pick up the clean DB
3. This ensures each walkthrough starts from a known state

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

These helpers use the GitHub API. You MUST replicate them via curl — never skip.
Sandbox repo: `KingBodhi/e2e-demo-sandbox` (configurable via E2E_DEMO_REPO_* env vars)

**`simulateDevAgentWork(request, taskId)`** — Execute these 5 steps:
1. Get SHA of `main` branch: `GET /repos/KingBodhi/e2e-demo-sandbox/git/refs/heads/main`
2. Create branch `e2e-demo/{taskId8}-{timestamp}`: `POST /repos/.../git/refs` with `{ref, sha}`
3. Create a file on the branch: `PUT /repos/.../contents/src/agent-work-{ts}.ts` with `{message, content(base64), branch}`
4. Create PR: `POST /repos/.../pulls` with `{title, body, head: branch, base: main}`
5. Create task attempt: `POST /api/task-attempts/create-record` with `{task_id, executor, base_branch}`
6. Link PR to attempt: `POST /api/task-attempts/{attemptId}/link-pr` with `{pr_number, pr_url, target_branch}`
7. Update task status to inreview: `PUT /api/tasks/{taskId}` with `{status: "inreview"}`

**`postDevAgentSummaryComment(request, prNumber, taskId, title)`** — Post a markdown comment:
- `POST /repos/.../issues/{prNumber}/comments` with formatted dev summary body

**`simulateQaVerdict(request, taskId, agentId, verdict)`** — Set QA result:
- `PATCH /api/tasks/{taskId}/collaborators` with `{actor_id, actor_type: "agent_watcher", action: verdict}`

**`postQaReviewComment(request, prNumber, taskId, verdict, opts)`** — Post QA review:
- `POST /repos/.../issues/{prNumber}/comments` with structured review markdown

**`fetchPrComments(request, prNumber)`** — Read comments:
- `GET /repos/.../issues/{prNumber}/comments`

**`getPrUrl(prNumber)`** — `https://github.com/KingBodhi/e2e-demo-sandbox/pull/{prNumber}`

**`cleanupDemoBranches(request, branches)`** — Delete each branch:
- `DELETE /repos/.../git/refs/heads/{branch}`

**`cleanupDemoPr(request, prNumber)`** — Close PR:
- `PATCH /repos/.../pulls/{prNumber}` with `{state: "closed"}`
