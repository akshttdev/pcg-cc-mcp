# E2E Test Quarantine

Tests in this directory are **excluded from the main test suite** via `testIgnore` in `playwright.config.ts`. They were ported from the `sloperation317` branch and need individual verification before promotion.

## Promotion Process

1. Run the test individually: `npx playwright test e2e/quarantine/<test>.spec.ts --reporter=list`
2. If it passes: move to `e2e/` (main suite) or `e2e/demos/` (demo-only tests)
3. If it fails on seed data: fix hardcoded UUIDs or add setup in `beforeAll`, then re-test
4. If it fails on LLM dependency: keep here, document in backlog with `@llm-required` note

## Test Inventory

| File | LLM Required? | Seed Data? | Promotion Target |
|------|--------------|------------|-----------------|
| pipeline-userflows.spec.ts | No | Hardcoded IDs | `e2e/` |
| dealflow-pipeline.spec.ts | No | Agents, deal | `e2e/` |
| hudson-as-sirak.spec.ts | No | Hardcoded UUIDs | `e2e/` |
| hudson-exact-flow.spec.ts | No | Hardcoded UUIDs | `e2e/` |
| company-links.spec.ts | No | Hudson deal | `e2e/` |
| trace-company-nav.spec.ts | No | Hardcoded UUIDs | `e2e/` |
| full-pipeline-walkthrough.spec.ts | **Yes** (OpenAI) | Creates own | `e2e/` (when LLM available) |
| operator-walkthrough.spec.ts | **Yes** (OpenAI) | Creates own | `e2e/` (when LLM available) |
| visual-pipeline-walkthrough.spec.ts | **Yes** (OpenAI) | Creates own | `e2e/demos/` (demo-only) |
