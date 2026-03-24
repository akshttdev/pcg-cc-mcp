---
name: run-pipeline-tests
description: Run pipeline E2E tests with agent simulation enabled and report results
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob
---

# Run Pipeline E2E Tests

Run the pipeline E2E test suite with agent simulation and report results.

**CRITICAL**: Each Bash tool call starts with a fresh working directory. You MUST `cd` to the correct directory at the START of every single command. Do NOT rely on `cd` from a previous Bash call.

## Step 1: Read environment config

Read the `.env` file at the project root to get `FRONTEND_PORT` and `BACKEND_PORT`.

```bash
cd $(git rev-parse --show-toplevel) && cat .env
```

Set `FPORT` to the `FRONTEND_PORT` value (default 3000) and `BPORT` to the `BACKEND_PORT` value (default 3002).

## Step 2: Verify servers are running

Check that both frontend and backend are listening on their expected ports:

```bash
lsof -i :$FPORT -sTCP:LISTEN | head -5
lsof -i :$BPORT -sTCP:LISTEN | head -5
```

If either server is not running, **stop and tell the user** which server(s) need to be started.

## Step 3: Check backend env vars

Verify the backend process has the required environment variables for agent simulation:

```bash
# Get the backend PID from lsof, then check its environment
BPID=$(lsof -i :$BPORT -sTCP:LISTEN -t | head -1)
ps eww $BPID 2>/dev/null | tr ' ' '\n' | grep -E '^(ENABLE_AGENT_FLOW_ENGINE|SIMULATE_LLM)='
```

Both `ENABLE_AGENT_FLOW_ENGINE=1` and `SIMULATE_LLM=1` must be present.

If either is missing, **warn the user**:

> The backend is missing required env vars for pipeline tests. Please restart with:
> ```
> ENABLE_AGENT_FLOW_ENGINE=1 SIMULATE_LLM=1 BACKEND_PORT=$BPORT cargo run
> ```
> See `e2e/pipeline/TESTING.md` for full setup details.

Ask the user whether to proceed anyway or wait for restart.

## Step 4: Run the tests

Run pipeline E2E tests in headed mode:

```bash
cd $(git rev-parse --show-toplevel) && E2E_HEADED=true FRONTEND_PORT=$FPORT npx playwright test e2e/pipeline/ --reporter=list
```

**Always use `E2E_HEADED=true`** per project convention (headed mode for pipeline tests).

## Step 5: Report results

Parse the test output and present a summary table:

| Test | Status | Duration |
|------|--------|----------|
| test name from output | PASS / FAIL / SKIP | Xs |

### For failures

For each failed test, report:
1. **Test name**: the full test title
2. **Error message**: the assertion or timeout error
3. **Suggested fix**: based on the error type:
   - **Timeout waiting for element**: likely a missing testid or selector mismatch — use Playwright MCP to inspect the real DOM
   - **Assertion failed**: check if the expected value matches current app behavior
   - **Navigation error**: verify the URL and that servers are running
   - **Agent flow timeout**: check that `ENABLE_AGENT_FLOW_ENGINE=1` and `SIMULATE_LLM=1` are set

### Reference

For detailed setup instructions, stage ownership model, and test file inventory, see `e2e/pipeline/TESTING.md`.
