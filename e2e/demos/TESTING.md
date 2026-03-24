# Demo E2E — Setup & Rules

See also: `e2e/TESTING.md` for shared conventions.

## Environment

Demo tests simulate full workflows including GitHub integration:

```bash
# Required
GITHUB_TOKEN=<token>        # For simulateDevAgentWork() and createPrForTask()
FRONTEND_PORT=3000

# Recommended
E2E_HEADED=true             # Demos are visual — run headed for review
```

Without `GITHUB_TOKEN`, Steps 3-4 in bug-report and manual-qa specs will skip.

## Timing & Pace

Demo tests use slower timing for visual demonstration:
- `demoPause.short` = 500ms (tab switches, menu transitions)
- `demoPause.medium` = 1500ms (form submissions, API calls)
- `demoPause.long` = 3000ms (page transitions, data loading)

The pace is controlled by `e2e/helpers/timing.ts` — detects `--project=demos` for slower pace.

## Running

```bash
# All demos
FRONTEND_PORT=3000 npx playwright test e2e/demos/ --reporter=list

# Headed (recommended for demos)
E2E_HEADED=true FRONTEND_PORT=3000 npx playwright test e2e/demos/

# Specific demo
FRONTEND_PORT=3000 npx playwright test e2e/demos/dealflow-pipeline-demo.spec.ts
```

## Demo Files

| File | What it demonstrates |
|------|---------------------|
| `dealflow-pipeline-demo.spec.ts` | Full pipeline walkthrough with agent triggers |
| `bug-report-lifecycle.spec.ts` | Bug report → task → agent work → PR (needs GITHUB_TOKEN) |
| `manual-qa-trigger.spec.ts` | Manual QA workflow trigger → agent execution |
| `workflow-crm-pipeline.spec.ts` | Workflow integration with CRM pipeline |
| `pipeline-intelligence-workflow.spec.ts` | Intelligence data flowing into pipeline decisions |
