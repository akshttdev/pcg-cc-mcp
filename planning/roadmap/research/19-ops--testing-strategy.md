# Testing Strategy Research: ORCHA Platform

**Date**: 2026-03-19
**Author**: Claude Research Sprint
**Status**: Research Document
**Scope**: Comprehensive testing tooling and strategy for Rust/Axum + React/TypeScript AI orchestration platform

---

## Table of Contents

1. [Current State Assessment](#1-current-state-assessment)
2. [Rust Testing](#2-rust-testing)
3. [Frontend Testing](#3-frontend-testing)
4. [E2E Testing](#4-e2e-testing)
5. [AI/LLM Testing](#5-aillm-testing)
6. [Performance Testing](#6-performance-testing)
7. [CI/CD Testing Pipeline](#7-cicd-testing-pipeline)
8. [Prioritized Adoption Roadmap](#8-prioritized-adoption-roadmap)
9. [Budget & Timeline Summary](#9-budget--timeline-summary)

---

## 1. Current State Assessment

### What Exists Today

| Layer | Tool | Coverage | Notes |
|-------|------|----------|-------|
| Rust unit tests | `cargo test` | Low-moderate | Tests exist in some crates, `continue-on-error: true` in CI |
| Rust linting | `cargo clippy` | Full | `-D warnings` flag but `continue-on-error` in CI |
| Rust formatting | `cargo fmt` | Full | Runs in CI |
| Frontend typechecking | `tsc --noEmit` | Full | Runs in CI |
| Frontend linting | ESLint + Prettier | Full | Runs in CI |
| E2E | Playwright | Moderate | ~6 spec files, auth setup, quarantine pattern, demo scripts |
| Type generation | ts-rs | Full | `generate-types:check` in CI |
| Security audit | cargo-audit + npm audit | Basic | `--audit-level=high` |
| Integration tests | None | None | No Axum integration tests |
| Frontend unit tests | None | None | No Vitest/Jest setup |
| Visual regression | None | None | |
| Performance tests | None | None | |
| AI output testing | None | None | |
| Code coverage | None | None | |

### Critical Gaps

1. **CI doesn't block on failures** — clippy and cargo test use `continue-on-error: true`
2. **No frontend unit testing** — zero component-level tests
3. **No API integration tests** — all Axum routes untested in isolation
4. **No AI output validation** — non-deterministic agent outputs not tested
5. **No performance baselines** — no benchmarks for regression detection
6. **No code coverage tracking** — unknown actual test coverage
7. **E2E not in CI** — Playwright tests run manually only

---

## 2. Rust Testing

### 2.1 Property-Based Testing

#### proptest
- **License**: MIT / Apache 2.0
- **What it does**: Generates random inputs to find edge cases, with shrinking to minimal failing examples
- **Pros**:
  - Excellent for testing data models (serialization round-trips, DB model invariants)
  - Finds edge cases humans miss (empty strings, Unicode, overflow)
  - Shrinking produces minimal reproducible failures
  - Integrates with `#[test]` — no new test runner needed
  - Mature, widely adopted in Rust ecosystem
- **Cons**:
  - Compile-time cost for macro expansion
  - Non-trivial to write good strategies for complex domain types
  - Random generation can be slow for complex types
- **ORCHA fit**: High value for workflow engine (state machine transitions), CRM data models (serialization/deserialization round-trips), DbUuid handling, JSON tag parsing
- **Stage**: 0 (immediate — low effort, high safety ROI)
- **Effort**: 2-3 days to add to critical crates (db, utils)
- **Budget impact**: Zero (OSS)

#### quickcheck
- **License**: MIT / Unlicense
- **What it does**: Similar to proptest but older API, requires implementing `Arbitrary` trait
- **Pros**: Lighter weight, simpler API
- **Cons**: Less powerful shrinking, less ergonomic than proptest, smaller ecosystem
- **Recommendation**: **Skip** — proptest is strictly better for ORCHA's needs

### 2.2 Snapshot Testing

#### insta
- **License**: Apache 2.0
- **What it does**: Captures output as snapshot files, fails when output changes, interactive review mode
- **Pros**:
  - Perfect for API response format testing (JSON snapshots of route responses)
  - `cargo insta review` provides interactive diff acceptance
  - Inline snapshots or file-based — flexible
  - Excellent for SQL query output validation
  - Catches unintended serialization changes (critical for ts-rs type generation)
  - `assert_json_snapshot!` for structured JSON comparison
  - redaction support for dynamic fields (timestamps, UUIDs)
- **Cons**:
  - Snapshot files add repo size
  - Can become "snapshot fatigue" — blindly accepting changes
  - Requires discipline to review snapshot diffs meaningfully
- **ORCHA fit**: Extremely high. With 130+ route modules and auto-generated TypeScript types, catching unintended API response changes is critical. Also valuable for workflow engine output validation.
- **Stage**: 0 (immediate)
- **Effort**: 1-2 days for initial setup, then incremental per-route adoption
- **Budget impact**: Zero (OSS)

### 2.3 Mocking

#### mockall
- **License**: MIT / Apache 2.0
- **What it does**: Procedural macro that auto-generates mock implementations of traits
- **Pros**:
  - Works with `#[automock]` attribute — minimal boilerplate
  - Supports expectation counts, argument matchers, return sequences
  - Enables testing services in isolation (mock DB, mock GitHub client, mock LLM provider)
- **Cons**:
  - Increases compile time significantly (proc macro overhead)
  - Generated code can be hard to debug
  - Requires trait-based architecture (ORCHA's executor pattern is already trait-based)
  - Can lead to tests that pass but don't reflect real behavior
- **ORCHA fit**: High for executor layer (mock LLM providers), service layer (mock GitHub API, mock DB). The existing executor trait pattern is ideal for mockall.
- **Stage**: 0-1 (start with executor mocks)
- **Effort**: 3-5 days for initial trait extraction and mock setup
- **Budget impact**: Zero (OSS)

### 2.4 Axum Integration Testing

#### axum-test
- **License**: MIT
- **What it does**: Provides `TestServer` that runs an Axum app in-memory, sends real HTTP requests without network overhead
- **Pros**:
  - Tests full request/response cycle including middleware, extractors, error handling
  - No port binding — runs in-process, fast and parallelizable
  - Supports cookies, headers, multipart, SSE testing
  - Type-safe response assertions
  - Tests the actual router configuration (catches routing bugs)
- **Cons**:
  - Requires app to be constructable without side effects (DB, external services)
  - Need to manage test database state
  - SSE testing support is limited but functional
- **ORCHA fit**: Critical. With 130+ route modules and no integration tests, this is the highest-impact testing gap. The Axum router, middleware (auth, rate limiting), and error handling are completely untested in isolation.
- **Stage**: 0 (immediate — most impactful addition)
- **Effort**: 1 week for test harness + 10 critical routes, then incremental
- **Budget impact**: Zero (OSS)

#### tower-test
- **License**: MIT
- **What it does**: Lower-level testing for Tower services and middleware
- **Pros**: Tests middleware in isolation, useful for auth/rate-limit middleware
- **Cons**: Lower-level than axum-test, more boilerplate
- **Recommendation**: Use alongside axum-test for middleware-specific tests only

### 2.5 Database Testing

#### SQLx Test Fixtures (built-in)
- **License**: MIT / Apache 2.0 (part of SQLx)
- **What it does**: `#[sqlx::test]` macro provides per-test database with migrations auto-applied
- **Pros**:
  - Already a dependency (SQLx is the ORM)
  - Each test gets its own database — true isolation
  - Migrations run automatically — tests always match schema
  - Works with SQLite (file-based, fast) and PostgreSQL
  - Fixtures can pre-populate test data
- **Cons**:
  - SQLite in-memory databases don't support all features
  - Per-test DB creation adds ~50-100ms overhead per test
  - Need to maintain test fixtures as schema evolves
- **ORCHA fit**: Essential. The database layer (`crates/db`) with 140+ models and 212 migrations needs automated testing. Current `SQLX_OFFLINE=true` mode means queries aren't validated against a real DB in CI.
- **Stage**: 0 (immediate)
- **Effort**: 2-3 days for test harness, then incremental per-model
- **Budget impact**: Zero (already a dependency)

#### testcontainers-rs
- **License**: MIT
- **What it does**: Spins up Docker containers (PostgreSQL, Redis, etc.) per test
- **Pros**:
  - Tests against real database engines
  - Disposable — no state leakage
  - Supports PostgreSQL, Redis, Kafka, etc.
- **Cons**:
  - Requires Docker — adds CI complexity
  - Slower than SQLite in-memory (~2-5s container startup)
  - Overkill for SQLite-based development
- **ORCHA fit**: Moderate. Only needed when migrating to PostgreSQL (Stage 1-2). SQLx test fixtures with SQLite are sufficient for Stage 0.
- **Stage**: 2 (when PostgreSQL migration happens)
- **Effort**: 1-2 days setup
- **Budget impact**: Zero (OSS), but requires Docker in CI

### 2.6 Load Testing

#### goose
- **License**: Apache 2.0
- **What it does**: Rust-based load testing framework, scriptable in Rust
- **Pros**:
  - Written in Rust — team already knows the language
  - Scriptable scenarios with weighted task sets
  - HTML report generation
  - Supports SSE/WebSocket connections
  - Can be compiled into the project's test suite
- **Cons**:
  - Smaller community than k6
  - Documentation is adequate but not extensive
  - Requires a running server instance
- **ORCHA fit**: Good for Stage 1-2 when preparing for multi-tenant pilots. Testing SSE streams, concurrent agent executions, and API throughput.
- **Stage**: 1 (before pilot onboarding)
- **Effort**: 3-5 days for initial scenarios
- **Budget impact**: Zero (OSS)

#### k6
- **License**: AGPL-3.0 (open source core), commercial cloud offering
- **What it does**: JavaScript-scriptable load testing, Grafana ecosystem
- **Pros**: Excellent reporting, large community, cloud option
- **Cons**: **AGPL license** is problematic for SaaS — must open-source modifications. JavaScript scripting adds language context-switch.
- **Recommendation**: **Avoid** due to AGPL. Use goose instead.

#### drill
- **License**: GPL-2.0
- **What it does**: YAML-configured HTTP load testing
- **Pros**: Simple YAML config, fast
- **Cons**: **GPL license** — problematic for commercial use. Limited scriptability.
- **Recommendation**: **Avoid** due to GPL.

### 2.7 Fuzzing

#### cargo-fuzz (libFuzzer)
- **License**: MIT / Apache 2.0
- **What it does**: Coverage-guided fuzzing using LLVM's libFuzzer
- **Pros**:
  - Coverage-guided — finds deep bugs
  - Excellent for parsing code (JSON workflows, MCP protocol messages)
  - Minimal setup via `cargo fuzz init`
  - Can find memory safety issues, panics, infinite loops
- **Cons**:
  - Requires nightly Rust (ORCHA already uses nightly)
  - Long-running — not suitable for CI fast path
  - Corpus management needed
- **ORCHA fit**: High value for workflow definition parsing, MCP message handling, and data source connector inputs. These are attack surfaces for a SaaS platform.
- **Stage**: 1 (before external users)
- **Effort**: 2-3 days for initial fuzz targets
- **Budget impact**: Zero (OSS), CI compute cost for periodic fuzz runs

---

## 3. Frontend Testing

### 3.1 Vitest

- **License**: MIT
- **What it does**: Vite-native test runner, drop-in Jest replacement with native ESM support
- **Pros**:
  - Native Vite integration — same config, same transforms, instant startup
  - Jest-compatible API — minimal learning curve
  - Watch mode with HMR-speed re-execution
  - Built-in coverage via v8 or istanbul
  - TypeScript support out of the box
  - Runs in Node or browser mode
  - ORCHA already uses Vite — zero config overhead
- **Cons**:
  - Younger than Jest (but very stable at this point, v3.x)
  - Some Jest plugins don't work (but most do)
- **ORCHA fit**: Essential. This is the missing frontend unit test layer. With React components for Kanban boards, CRM pipelines, workflow editors, and form dialogs, component-level testing prevents regressions that E2E alone cannot efficiently catch.
- **Stage**: 0 (immediate)
- **Effort**: 1 day setup, then incremental per-component
- **Budget impact**: Zero (OSS)

### 3.2 React Testing Library

- **License**: MIT
- **What it does**: DOM-based component testing that encourages testing user behavior, not implementation details
- **Pros**:
  - Tests what users see and do (click, type, read) — not internal state
  - Works with Vitest seamlessly
  - `screen.getByRole()`, `userEvent` API matches Playwright mental model
  - De facto standard for React testing
  - Catches accessibility issues (tests via roles/labels)
- **Cons**:
  - Requires jsdom or happy-dom for DOM simulation
  - Complex components with many providers need wrapper setup
  - Not suitable for visual testing (layout, styling)
- **ORCHA fit**: High. Priority targets: `TaskCard`, `KanbanBoard`, `WorkflowEditor`, `ProjectFormDialog`, `PipelineView`. These are high-churn components where unit tests prevent regressions.
- **Stage**: 0 (with Vitest)
- **Effort**: 1 day setup (included with Vitest), then per-component
- **Budget impact**: Zero (OSS)

### 3.3 MSW (Mock Service Worker)

- **License**: MIT
- **What it does**: Intercepts HTTP requests at the network level using Service Worker (browser) or Node interceptors (tests)
- **Pros**:
  - Mocks API responses without changing application code
  - Same mock definitions work in Vitest, Storybook, and browser dev
  - Supports REST and GraphQL
  - Request handlers are composable and overridable per test
  - Simulates error states, slow responses, SSE streams
  - Does not require a running backend
- **Cons**:
  - Initial handler setup is non-trivial for 130+ API routes
  - Mocks can drift from real API — need contract testing to prevent
  - SSE stream mocking requires custom handlers
- **ORCHA fit**: Critical for frontend unit testing. Without MSW, every component test that calls an API would need the backend running. Start with handlers for the most-used endpoints: tasks, projects, CRM, workflows.
- **Stage**: 0 (required for Vitest component tests)
- **Effort**: 2-3 days for core handlers, then incremental
- **Budget impact**: Zero (OSS)

### 3.4 Storybook

- **License**: MIT
- **What it does**: Component development environment — render components in isolation with controls
- **Pros**:
  - Visual component catalog for team communication
  - Interaction testing (play functions)
  - Addon ecosystem (a11y, viewport, dark mode)
  - Chromatic integration for visual regression
  - Documentation generation
  - Works with Vite builder
- **Cons**:
  - Significant setup overhead — each component needs a story
  - Maintenance burden: stories can go stale
  - Adds build complexity
  - Team of 3 may not benefit from visual catalog as much as a larger team
- **ORCHA fit**: Moderate. Valuable for the component library (shadcn/ui customizations, Kanban primitives), but lower priority than automated testing. More impactful when team grows or external contributors join.
- **Stage**: 2 (when team grows beyond 3)
- **Effort**: 3-5 days initial setup, ongoing per-component
- **Budget impact**: Zero (OSS core), Chromatic costs $149/mo for teams

### 3.5 Chromatic (Visual Regression)

- **License**: Commercial (SaaS) with free tier
- **What it does**: Visual regression testing via Storybook snapshots, cloud-based diff detection
- **Pros**:
  - Catches CSS/layout regressions automatically
  - Cloud-hosted — no infrastructure to manage
  - PR integration with visual diffs
  - Free tier: 5,000 snapshots/month
- **Cons**:
  - Requires Storybook (dependency chain)
  - $149/mo starter, $349/mo for teams beyond free tier
  - External service dependency
  - Overkill for team of 3
- **ORCHA fit**: Low priority now. More valuable at Stage 2-3 when UI stability matters for SaaS customers.
- **Stage**: 2-3
- **Effort**: 1 day (if Storybook already set up)
- **Budget impact**: $0-349/month depending on tier

---

## 4. E2E Testing

### 4.1 Playwright Best Practices for AI/Async UIs

ORCHA's UI has several patterns that make E2E testing challenging:

#### SSE-Driven Updates
- **Problem**: Content appears asynchronously via SSE (process logs, task diffs, agent status)
- **Solution**: Use `page.waitForResponse()` with SSE endpoint matchers, or custom `waitForSelector` on final DOM state
- **Pattern**: `await expect(page.getByText('Agent completed')).toBeVisible({ timeout: 30_000 })`

#### Non-Deterministic AI Outputs
- **Problem**: Agent responses vary between runs
- **Solution**: Assert on structural properties, not exact content:
  - Assert element count (e.g., "at least 3 workflow steps created")
  - Assert format (e.g., "response contains valid JSON")
  - Assert state transitions (e.g., "task moved to 'done' status")
  - Use regex matchers for content validation

#### Long-Running Operations
- **Problem**: Workflow executions and agent tasks can take 30-60+ seconds
- **Solution**: Separate fast smoke tests from slow integration tests. Fast tests mock the AI backend; slow tests run against real (or staged) LLM responses.

### 4.2 Test Data Management

Current approach (seed DB + API helpers) is solid. Recommendations:

- **Extend `e2e/helpers/seed.ts`** with factories for all entity types (organizations, projects, workflows, data sources)
- **Database snapshots**: Create pre-built SQLite snapshots for different test scenarios (empty org, org with CRM data, org with running workflows)
- **API-driven setup preferred**: Create data via API in `beforeAll` rather than direct DB manipulation — tests validate the creation path too
- **Cleanup strategy**: Since SQLite is file-based, copy `test-seed.sqlite` to a temp file per test suite, set `DATABASE_URL` per worker

### 4.3 Parallel Test Execution

Current config: `fullyParallel: false, workers: 1` — everything sequential.

**Recommendations**:
- **Short-term (Stage 0)**: Keep sequential for shared-state tests (auth), but use `test.describe.parallel()` within independent test files
- **Medium-term (Stage 1)**: Per-worker database isolation (each Playwright worker gets its own SQLite copy + backend instance)
- **Long-term (Stage 2)**: Full parallel with `workers: 4+`, per-worker auth, per-worker DB. Use Playwright's `--shard` for CI matrix distribution.

### 4.4 CI Integration

Current gap: Playwright tests do not run in CI at all.

**Implementation plan**:
1. **Stage 0**: Add Playwright to CI with smoke tests only (health check, login, navigation)
   - Start backend in CI using `dev_assets_seed/test-seed.sqlite`
   - Run `npx playwright test --grep @smoke` (tag critical tests)
   - Upload trace artifacts on failure
2. **Stage 1**: Full E2E suite in CI, parallel sharding
3. **Stage 2**: Visual regression via Playwright screenshots + `toHaveScreenshot()`

### 4.5 Playwright Visual Regression (Built-in)

- **License**: Apache 2.0 (part of Playwright)
- **What it does**: `expect(page).toHaveScreenshot()` captures and diffs screenshots
- **Pros**:
  - No extra tool needed — built into Playwright
  - Platform-specific baselines (Linux CI vs macOS dev)
  - Configurable threshold and mask regions
  - Free
- **Cons**:
  - Baseline images must be committed to repo
  - Sensitive to font rendering, antialiasing differences across platforms
  - Requires updating baselines when intentional UI changes happen
- **ORCHA fit**: Good for Stage 1-2 when UI stability matters. Start with critical pages (login, dashboard, Kanban board, CRM pipeline).
- **Stage**: 1
- **Effort**: 1-2 days to add to existing tests
- **Budget impact**: Zero

---

## 5. AI/LLM Testing

This is the most novel and critical testing domain for ORCHA. AI outputs are non-deterministic, expensive to test with real models, and critical to product quality.

### 5.1 Testing Non-Deterministic Outputs

**Approach: Contract-Based AI Testing**

Instead of testing exact output, test the contract:

```
Test: "Given a brief, agent produces a structured workflow"
Assert:
  - Response is valid JSON matching WorkflowDefinition schema
  - Response has >= 1 workflow step
  - Each step has required fields (id, type, config)
  - Estimated cost < $2.00
  - Execution time < 60 seconds
```

**Implementation pattern for ORCHA**:
1. **Schema validation**: Agent outputs must conform to structured response envelope (status, message, artifacts)
2. **Invariant testing**: Cost < threshold, token count < limit, no PII in output
3. **Behavioral testing**: Given known input, output should contain certain structural elements
4. **Golden test sets**: Curated input/output pairs that are human-validated, tested for regression

### 5.2 promptfoo

- **License**: MIT
- **What it does**: CLI and library for testing and evaluating LLM prompts. Supports A/B testing, model comparison, automated evaluation.
- **Pros**:
  - YAML-based test definitions — low code barrier
  - Supports OpenAI, Anthropic, local models, custom providers
  - Built-in assertions: contains, regex, JSON schema, similarity, LLM-as-judge
  - Model comparison (test same prompt across Claude, GPT-4, Gemini)
  - Cost tracking per evaluation
  - CI-friendly with `promptfoo eval --ci`
  - Web UI for result analysis
  - Custom provider support — can test ORCHA's own routing layer
- **Cons**:
  - Node.js based — adds a runtime dependency for Rust-primary team
  - Real LLM calls are expensive ($0.01-0.10 per eval)
  - YAML config can get complex for multi-step agent flows
- **ORCHA fit**: High. Critical for:
  - Validating prompt templates in `crates/executors/` across model changes
  - Regression testing when switching models in cost-tiered routing
  - Evaluating agent quality metrics (CAPO = Cost per Accepted Outcome)
  - A/B testing prompt variations before deployment
- **Stage**: 0-1 (start with system prompt validation, expand to full eval)
- **Effort**: 2-3 days for initial eval suite
- **Budget impact**: Zero (OSS), $5-50/run in LLM API costs

### 5.3 deepeval

- **License**: Apache 2.0
- **What it does**: Python-based LLM evaluation framework with built-in metrics (faithfulness, relevance, hallucination detection)
- **Pros**:
  - Rich metric library: G-Eval, faithfulness, answer relevancy, contextual recall
  - Hallucination detection (important for agent-generated code)
  - Conversational testing (multi-turn agent interactions)
  - Integration with pytest
- **Cons**:
  - Python — adds language to stack
  - Heavier than promptfoo for simple evaluations
  - Metrics use LLM-as-judge (adds cost)
- **ORCHA fit**: Moderate. More relevant at Stage 2-3 when agent quality metrics are a product differentiator. For Stage 0-1, promptfoo is sufficient.
- **Stage**: 2-3
- **Effort**: 3-5 days
- **Budget impact**: Zero (OSS), LLM eval costs

### 5.4 Mock LLM Providers

**Approach: Deterministic Test Doubles**

For fast, reliable tests, mock the LLM layer:

1. **Recorded responses**: Record real LLM responses, replay in tests (VCR pattern)
   - Store in `test_fixtures/llm_responses/` organized by prompt hash
   - Use MSW (frontend) or mockall (backend) to intercept
2. **Deterministic stub provider**: Implement executor trait with canned responses
   - Already natural given ORCHA's executor trait pattern (`crates/executors/`)
   - `MockExecutor` returns known responses for known inputs
3. **Schema-only validation**: For integration tests, return minimal valid JSON without calling LLM

**Implementation**:
```rust
// crates/executors/src/mock_executor.rs
#[cfg(test)]
pub struct MockExecutor {
    responses: HashMap<String, String>,
}
```

- **Stage**: 0 (essential for testable executor layer)
- **Effort**: 1-2 days
- **Budget impact**: Zero (eliminates LLM costs in test suite)

### 5.5 Agent Behavior Testing

**Pattern: Scenario-Based Agent Testing**

For ORCHA's multi-agent orchestration:

1. **Happy path scenarios**: Agent receives task, produces correct output, transitions state
2. **Error handling scenarios**: LLM returns malformed response, timeout, rate limit
3. **Delegation scenarios**: Agent delegates to sub-agent, receives result, continues
4. **Circuit breaker scenarios**: Multiple failures trigger escalation to human
5. **Cost scenarios**: Agent stays within CAPO budget

**Testing framework**: Combine proptest (random inputs) + mockall (controlled LLM responses) + insta (snapshot expected state transitions). No external tool needed — this is architectural.

- **Stage**: 0-1 (start with error handling scenarios)
- **Effort**: 1-2 weeks for comprehensive agent test suite
- **Budget impact**: Zero

---

## 6. Performance Testing

### 6.1 Criterion

- **License**: MIT / Apache 2.0
- **What it does**: Statistical microbenchmarking for Rust, with regression detection
- **Pros**:
  - Statistical significance testing — not just "faster/slower" but confidence intervals
  - HTML reports with plots
  - Baseline comparison across git commits
  - Integrates with cargo bench
  - Supports async benchmarks via `criterion::async_executor::Tokio`
- **Cons**:
  - Compile time overhead (links criterion library)
  - Microbenchmarks don't capture system-level performance
  - Need stable machine for reliable results (CI variability)
- **ORCHA fit**: High value for hot paths:
  - Workflow engine step execution
  - JSON serialization/deserialization (140+ models)
  - Database query patterns (org-scoped queries with JOINs)
  - DbUuid parse/format operations
  - SSE message serialization
- **Stage**: 1 (before pilot load)
- **Effort**: 2-3 days for initial benchmarks
- **Budget impact**: Zero (OSS)

### 6.2 SSE/WebSocket Load Testing

**Challenge**: ORCHA uses SSE for real-time process logs, task diffs, and agent status. Standard HTTP load testers don't handle persistent connections well.

**Approach**:
1. **goose** (see 2.6): Supports SSE via custom task functions that hold connections open
2. **Custom Rust harness**: Use `reqwest::Client` with SSE parsing to simulate N concurrent subscribers
3. **Metrics to capture**: Connection establishment time, message delivery latency, max concurrent connections before degradation, memory per connection

**Key scenarios**:
- 100 concurrent SSE connections (Stage 1 pilot scale)
- 1,000 concurrent connections (Stage 2 SaaS scale)
- SSE reconnection storm (all clients reconnect simultaneously)

- **Stage**: 1 (before pilots)
- **Effort**: 3-5 days
- **Budget impact**: Zero (custom tooling)

### 6.3 Database Query Performance

**Approach**:
1. **SQLx query logging**: Enable `RUST_LOG=sqlx=info` to capture query timings
2. **EXPLAIN ANALYZE**: Add test helpers that run `EXPLAIN QUERY PLAN` on critical queries
3. **Benchmark suites**: Use Criterion to benchmark common query patterns with realistic data volumes:
   - List tasks with org scoping (the sidebar query)
   - CRM pipeline queries with contact JOINs
   - Artifact queries with hex-to-text UUID conversion
   - Full-text search across entities

**Known concerns from codebase**:
- BLOB UUID hex-to-text conversion in artifact org-scoping JOINs
- 212 migrations may have accumulated suboptimal indexes
- Migration `20260318000000` added comprehensive indexes — need to validate they're effective

- **Stage**: 1
- **Effort**: 2-3 days
- **Budget impact**: Zero

### 6.4 Memory Leak Detection

**Tools**:

#### Valgrind/DHAT (Linux only)
- Heap profiling, leak detection
- Excellent for long-running services
- **Limitation**: Linux only, macOS dev team would need CI or Docker

#### cargo-instruments (macOS)
- Integrates with Apple Instruments
- Memory allocations, leaks, CPU profiling
- **Limitation**: macOS only, not CI-friendly

#### jemalloc + DHAT-rs
- **License**: MIT (dhat-rs)
- Drop-in allocator replacement with profiling
- Works on all platforms
- Can be enabled conditionally with feature flags

**ORCHA-specific concerns**:
- APN node process accumulation (already mitigated with `kill_existing_apn_nodes()`)
- Long-running SSE connections — verify no per-connection memory growth
- Workflow execution context — ensure cleanup after workflow completion
- Agent conversation history — bounded or unbounded?

- **Stage**: 1 (before long-running production deployment)
- **Effort**: 2-3 days
- **Budget impact**: Zero

---

## 7. CI/CD Testing Pipeline

### 7.1 Test Parallelization in GitHub Actions

**Current state**: Single CI job, sequential steps.

**Recommended architecture**:

```yaml
# Matrix strategy for parallel execution
jobs:
  rust-checks:
    strategy:
      matrix:
        check: [fmt, clippy, test-db, test-services, test-server, test-executors]

  frontend-checks:
    strategy:
      matrix:
        check: [typecheck, lint, vitest]

  e2e:
    needs: [rust-checks, frontend-checks]
    strategy:
      matrix:
        shard: [1, 2, 3, 4]  # Playwright --shard=${{ matrix.shard }}/4
```

**Optimizations**:
- **Rust compilation caching**: `sccache` (already in flox) + GitHub Actions cache for `target/`
- **Frontend caching**: `pnpm store` cache via `actions/cache`
- **Playwright browser caching**: Cache `~/.cache/ms-playwright/`
- **Estimated CI time reduction**: From ~15-20min sequential to ~5-8min parallel

- **Stage**: 0 (immediate — fixes the "CI doesn't block" problem)
- **Effort**: 2-3 days
- **Budget impact**: Moderate increase in GitHub Actions minutes (2-3x parallelism), offset by faster feedback

### 7.2 Test Result Caching

**Strategies**:
1. **Rust incremental compilation**: Already enabled via sccache in flox
2. **cargo-nextest**: Drop-in replacement for `cargo test` with:
   - Per-test timeout (catches hanging tests)
   - JUnit XML output for CI integration
   - Retry flaky tests automatically
   - Parallel test execution within a crate
   - **License**: MIT / Apache 2.0
3. **Playwright test caching**: Use `--last-failed` to only re-run failures
4. **Git-based test selection**: Only run tests for changed crates (detect via `cargo metadata` + git diff)

- **Stage**: 0-1
- **Effort**: 1-2 days for cargo-nextest, 2-3 days for smart test selection
- **Budget impact**: Reduces CI costs by 30-50%

### 7.3 Flaky Test Detection and Quarantine

**Current state**: ORCHA already has an `e2e/quarantine/` directory pattern. Extend this systematically.

**Recommended approach**:
1. **Automatic flaky detection**: Run failed tests 3x. If any pass, mark as flaky.
2. **Quarantine workflow**: Flaky tests move to quarantine, tracked in `planning/BACKLOG--remaining-work.md` (per existing convention)
3. **Weekly flaky burn-down**: Review quarantined tests, fix root causes, promote back
4. **CI integration**: Quarantined tests run in a separate job that doesn't block merge

**Tools**:
- **cargo-nextest**: Built-in retry and flaky detection (`--retries 2`)
- **Playwright**: `retries: 2` in config (already set for CI: `retries: process.env.CI ? 1 : 0`)
- **GitHub Actions**: Use `@flaky` tag + separate job for quarantined tests

- **Stage**: 0 (formalize existing pattern)
- **Effort**: 1-2 days
- **Budget impact**: Zero

### 7.4 Code Coverage

#### cargo-tarpaulin
- **License**: MIT / Apache 2.0
- **What it does**: Source-based code coverage for Rust, outputs lcov/cobertura
- **Pros**:
  - Simple: `cargo tarpaulin --out lcov`
  - GitHub Actions integration
  - Supports workspace coverage
  - Codecov/Coveralls compatible
- **Cons**:
  - Linux only (requires ptrace) — won't work on macOS dev machines
  - Can be slow on large workspaces
  - Occasional false positives/negatives with async code
  - Not compatible with all proc macros
- **ORCHA fit**: Good for CI (Linux runners), not for local dev

#### cargo-llvm-cov
- **License**: MIT / Apache 2.0
- **What it does**: LLVM-based source coverage using Rust's built-in instrumentation
- **Pros**:
  - More accurate than tarpaulin (LLVM-level instrumentation)
  - Works on macOS and Linux
  - Faster than tarpaulin
  - Branch coverage support
  - Integrates with `cargo nextest`
  - Supports merging coverage from multiple test runs (unit + integration)
- **Cons**:
  - Requires nightly Rust (ORCHA already uses nightly)
  - Slightly more setup than tarpaulin
- **ORCHA fit**: Preferred over tarpaulin. Cross-platform support is important for a macOS dev team.
- **Stage**: 1 (after test coverage is meaningful enough to measure)
- **Effort**: 1 day setup, CI integration
- **Budget impact**: Zero (OSS)

#### Frontend Coverage (Vitest built-in)
- Vitest has built-in coverage via `@vitest/coverage-v8`
- Outputs lcov, can merge with Rust coverage in a single Codecov report
- **Stage**: 0 (comes free with Vitest setup)

---

## 8. Prioritized Adoption Roadmap

### Stage 0 — Dogfood & Capture (Q2 2026, April-June)

**Theme**: Establish testing foundation, fix CI blocking, prevent regressions during rapid development.

| Priority | Tool | Effort | Impact |
|----------|------|--------|--------|
| **P0** | Fix CI to block on failures (remove `continue-on-error`) | 1 hour | Critical — tests are useless if CI ignores them |
| **P0** | axum-test (API integration tests for 10 critical routes) | 5 days | Catches routing, auth, serialization bugs |
| **P0** | SQLx test fixtures (DB model tests) | 3 days | Validates 140+ models against real schema |
| **P1** | Vitest + React Testing Library + MSW | 4 days | Frontend unit testing foundation |
| **P1** | insta (snapshot testing for API responses) | 2 days | Catches unintended API changes |
| **P1** | MockExecutor for LLM testing | 2 days | Enables testing agent code without LLM costs |
| **P1** | cargo-nextest (replace cargo test) | 1 day | Flaky detection, per-test timeouts, parallel execution |
| **P2** | proptest (workflow engine, data models) | 3 days | Edge case discovery |
| **P2** | promptfoo (initial prompt eval suite) | 3 days | Prompt regression detection |
| **P2** | mockall (service layer mocks) | 3 days | Isolated service testing |
| **P2** | CI parallelization (GitHub Actions matrix) | 3 days | 3x faster CI |
| **P2** | Playwright in CI (smoke tests only) | 2 days | E2E regression gate |

**Total Stage 0 effort**: ~30 engineer-days (~6 weeks for 1 person, ~2 weeks for team of 3 with AI assist)
**Total Stage 0 cost**: $0 (all MIT/Apache 2.0 OSS)

### Stage 1 — First Pilots & Pre-Seed (Q3-Q4 2026)

| Priority | Tool | Effort | Impact |
|----------|------|--------|--------|
| **P0** | Full E2E suite in CI with parallelization | 3 days | Required for pilot reliability |
| **P0** | cargo-fuzz (MCP protocol, workflow parser) | 3 days | Security hardening before external users |
| **P1** | goose (load testing SSE + API) | 5 days | Capacity planning for multi-tenant |
| **P1** | Criterion (benchmark hot paths) | 3 days | Performance regression detection |
| **P1** | cargo-llvm-cov (code coverage in CI) | 1 day | Coverage visibility |
| **P1** | Memory leak detection (dhat-rs) | 3 days | Long-running service stability |
| **P2** | Playwright visual regression (screenshots) | 2 days | UI stability for pilots |
| **P2** | Database query benchmarks | 3 days | Query performance baselines |
| **P2** | promptfoo full eval suite (all agent prompts) | 5 days | Agent quality assurance |

**Total Stage 1 effort**: ~28 engineer-days
**Total Stage 1 cost**: $0 (all OSS)

### Stage 2 — SaaS Launch & Seed (2027)

| Priority | Tool | Effort | Impact |
|----------|------|--------|--------|
| **P1** | testcontainers-rs (PostgreSQL testing) | 2 days | Database migration validation |
| **P1** | Storybook (component catalog) | 5 days | Team scaling, design system |
| **P2** | Chromatic (visual regression SaaS) | 1 day | Automated visual QA |
| **P2** | deepeval (advanced LLM metrics) | 5 days | Agent quality differentiation |
| **P2** | Contract testing (API compatibility) | 5 days | Multi-version API support |

**Total Stage 2 effort**: ~18 engineer-days
**Total Stage 2 cost**: $0-349/month (Chromatic)

---

## 9. Budget & Timeline Summary

### Total Investment by Stage

| Stage | Engineer-Days | Dollar Cost | Tools Added |
|-------|--------------|-------------|-------------|
| **Stage 0** (Q2 2026) | ~30 days | $0 | 11 tools/practices |
| **Stage 1** (Q3-Q4 2026) | ~28 days | $0 | 8 tools/practices |
| **Stage 2** (2027) | ~18 days | $0-349/mo | 5 tools/practices |
| **Total** | ~76 days | $0-349/mo | 24 tools/practices |

### License Summary

| License | Tools |
|---------|-------|
| **MIT** | Vitest, React Testing Library, MSW, mockall, axum-test, cargo-nextest, dhat-rs, cargo-llvm-cov |
| **MIT / Apache 2.0** | proptest, insta, cargo-tarpaulin, Criterion, cargo-fuzz |
| **Apache 2.0** | Playwright, goose, Storybook, deepeval |
| **AGPL (AVOID)** | k6 |
| **GPL (AVOID)** | drill |
| **Commercial** | Chromatic ($0-349/mo) |

### Tools Explicitly NOT Recommended

| Tool | Reason |
|------|--------|
| **k6** | AGPL license — incompatible with SaaS business model |
| **drill** | GPL license — viral copyleft risk |
| **quickcheck** | proptest is strictly superior |
| **Jest** | Vitest is better fit for Vite-based project |
| **Cypress** | Playwright already adopted, switching cost unjustified |
| **cargo-tarpaulin** | cargo-llvm-cov is more accurate and cross-platform |

### Key ROI Arguments

1. **Fixing CI blocking alone** (1 hour) prevents every category of regression from reaching main
2. **axum-test + SQLx fixtures** (8 days) covers the largest untested surface (130+ routes, 140+ models)
3. **MockExecutor** (2 days) eliminates LLM API costs from the test suite entirely
4. **Vitest + MSW** (4 days) establishes the missing frontend test layer
5. **cargo-nextest** (1 day) gives flaky detection, parallel execution, and per-test timeouts for free

### Risk: What Happens Without This Investment

- **Stage 0 without testing**: Dogfood failures erode team confidence, bugs reach client deliverables
- **Stage 1 without testing**: Pilot customers hit regressions, pre-seed investors see instability
- **Stage 2 without testing**: SaaS customers churn due to reliability issues, scaling exposes untested paths

---

## Appendix A: Quick-Start Commands

### Vitest Setup (Day 1)
```bash
cd frontend
pnpm add -D vitest @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom msw @vitest/coverage-v8
```

### cargo-nextest Setup (Day 1)
```bash
cargo install cargo-nextest
cargo nextest run --workspace  # replaces cargo test
```

### insta Setup (Day 1)
```bash
cargo install cargo-insta
# Add to Cargo.toml: insta = { version = "1", features = ["json", "redactions"] }
cargo insta review  # interactive snapshot acceptance
```

### promptfoo Setup (Day 1)
```bash
npx promptfoo@latest init
# Define prompts and assertions in promptfooconfig.yaml
npx promptfoo eval
npx promptfoo view  # web UI for results
```

### cargo-llvm-cov Setup (Day 1)
```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

---

## Appendix B: Testing Pyramid for ORCHA

```
                    /\
                   /  \
                  / E2E \          Playwright (existing + CI)
                 /________\        ~20 tests, 5-10 min
                /          \
               / Integration \     axum-test + SQLx fixtures
              /______________\     ~200 tests, 2-3 min
             /                \
            /   Component/Unit  \  Vitest + RTL (frontend)
           /____________________\  cargo test (backend)
          /                      \ ~500+ tests, 1-2 min
         /    Contract/Schema     \
        /__________________________\  insta + proptest + promptfoo
       /                            \ ~100 tests, 30 sec
      /       Mock/Stub Layer        \
     /________________________________\ MockExecutor + MSW + mockall
```

**Target test execution times** (Stage 0 exit):
- Unit tests (Rust + Frontend): < 2 minutes
- Integration tests (axum-test): < 3 minutes
- E2E smoke (Playwright CI): < 5 minutes
- Full E2E suite: < 15 minutes
- Full pipeline including coverage: < 20 minutes
