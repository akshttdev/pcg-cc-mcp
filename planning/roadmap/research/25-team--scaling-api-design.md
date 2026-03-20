# ORCHA Platform — Team Scaling, API Design & Developer Ecosystem Research

**Date**: 2026-03-19
**Type**: Research Report
**Author**: Roadmap Research Sprint
**Scope**: API design, developer experience, team scaling, Rust ecosystem, frontend architecture, i18n, accessibility
**Context**: ORCHA is a 23-crate Rust/Axum + React/Vite AI orchestration platform, currently 3-person founding team scaling through Series A (~2029). See `5-year-product-roadmap.md` for stage definitions.

---

## Table of Contents

1. [API Design for Public Platform](#1-api-design-for-public-platform)
2. [Developer Experience / Platform](#2-developer-experience--platform)
3. [Team Scaling Engineering Practices](#3-team-scaling-engineering-practices)
4. [Rust Ecosystem Team Patterns](#4-rust-ecosystem-team-patterns)
5. [Frontend Architecture Scaling](#5-frontend-architecture-scaling)
6. [Internationalization](#6-internationalization)
7. [Accessibility](#7-accessibility)
8. [Summary Decision Matrix](#8-summary-decision-matrix)

---

## 1. API Design for Public Platform

### 1.1 OpenAPI/Swagger Generation from Axum — utoipa

**Library**: [utoipa](https://github.com/juhaku/utoipa)
**License**: MIT / Apache 2.0
**Maturity**: Production-ready, 3.5K+ GitHub stars, active maintenance

**How it works**: Procedural macros (`#[utoipa::path]`, `#[derive(ToSchema)]`) annotate Axum handlers and structs. At build time, generates OpenAPI 3.1 spec. Companion crate `utoipa-swagger-ui` serves interactive docs at `/swagger-ui`.

**Pros**:
- Zero runtime overhead — spec generated at compile time
- Tight Axum integration via `utoipa-axum` (auto-discovers routes from Router)
- Derives work on existing serde structs — incremental adoption
- Generates spec that feeds SDK generation (see 1.6)
- Supports response schemas, security schemes, examples
- Active ecosystem: utoipa-scalar, utoipa-redoc for alternative UIs

**Cons**:
- Macro annotations add visual noise to handler functions
- Complex types (enums with data, generics) sometimes need manual `ToSchema` impls
- Custom extractors (like `DbUuid`) need `ToSchema` wrapper implementations
- Spec drift risk if annotations fall behind code changes (mitigated by CI validation)

**ORCHA-specific considerations**:
- Current codebase has ~80+ route handlers across `crates/server/src/routes/` — incremental annotation is feasible
- `DbUuid` type needs a `ToSchema` impl mapping to `string(uuid)` in OpenAPI
- SSE endpoints need manual documentation (not natively supported by OpenAPI)
- MCP server endpoints are separate from REST — document separately

**Roadmap stage fit**: **Stage 1** (pilot readiness). Pilots need API documentation. Start annotating public-facing endpoints in Stage 0 during stability fixes.

**Budget/timeline impact**: 1-2 weeks for initial annotation of core endpoints (tasks, projects, CRM, workflows). Ongoing: ~30 min per new endpoint.

**Recommendation**: **Adopt in Stage 0-1**. Add `utoipa` dependency, annotate endpoints incrementally starting with the public API surface. Add CI step: `cargo test` that validates OpenAPI spec generation succeeds.

---

### 1.2 API Versioning Strategies

Three dominant strategies exist. Evaluation for ORCHA:

#### URL-based versioning (`/api/v1/tasks`, `/api/v2/tasks`)

**Pros**:
- Simplest to implement and understand
- Easy to route in Axum (nested routers per version)
- Clear in logs, debugging, documentation
- Caching works naturally (different URL = different cache key)

**Cons**:
- URL proliferation over time
- Clients must update URLs on version bump
- Encourages "big bang" version changes rather than incremental evolution

#### Header-based versioning (`Accept: application/vnd.orcha.v2+json`)

**Pros**:
- Clean URLs stay stable
- Fine-grained: can version individual resources differently
- HTTP-correct (content negotiation)

**Cons**:
- Harder to test (can't just change URL in browser)
- Easy to forget in client code
- Load balancers/CDNs may not respect custom headers for routing

#### Content negotiation / evolution (no explicit version, additive changes only)

**Pros**:
- Least client friction
- Forces disciplined API design (never remove fields, only add)
- Works well with GraphQL's approach

**Cons**:
- Eventually accumulates cruft
- Breaking changes become impossible without escape hatch
- Requires strict API review discipline

**Recommendation for ORCHA**: **URL-based versioning, adopt at Stage 1**.

Rationale: ORCHA is pre-public-API. Start with `/api/v1/` prefix when opening the API to external consumers (Stage 1 pilots). URL-based is the simplest to implement, easiest for pilot clients to understand, and most compatible with OpenAPI tooling. At 3-5 pilot clients, the overhead of header-based versioning is not justified.

**Implementation pattern in Axum**:
```rust
// In router setup
let v1_routes = Router::new()
    .nest("/tasks", task_routes())
    .nest("/projects", project_routes());

let app = Router::new()
    .nest("/api/v1", v1_routes)
    // Keep /api/ as alias for current version during transition
    .nest("/api", v1_routes.clone());
```

**Budget/timeline**: 1-2 days to restructure existing routes under `/api/v1/`. No functionality change.

---

### 1.3 GraphQL vs REST vs tRPC

#### Current state: REST (Axum)

ORCHA currently uses REST with Axum. 80+ endpoints, well-structured by domain (projects, tasks, CRM, workflows, agents).

#### GraphQL — async-graphql

**Library**: [async-graphql](https://github.com/async-graphql/async-graphql)
**License**: MIT / Apache 2.0
**Maturity**: Production-ready, 3.4K+ stars

**Pros**:
- Client-driven queries reduce over-fetching (mobile clients, third-party integrations)
- Single endpoint simplifies client code
- Subscriptions replace SSE for real-time updates
- Self-documenting schema (introspection)
- Works alongside REST (not either/or)

**Cons**:
- N+1 query problem requires DataLoader pattern (additional complexity)
- Caching is harder than REST (no URL-based cache keys)
- Authorization per field is more complex than per endpoint
- Learning curve for team; debugging is harder
- ORCHA's current SSE-based real-time system would need rearchitecting for subscriptions
- Adds significant surface area to maintain alongside REST

#### tRPC patterns (type-safe RPC)

Not directly applicable to Rust backend / React frontend (tRPC is TypeScript-to-TypeScript). However, ORCHA already achieves a similar effect via `ts-rs` type generation from Rust structs. The current `npm run generate-types` workflow provides end-to-end type safety without tRPC.

**Recommendation**: **Stay REST, do not adopt GraphQL until Stage 3+**.

Rationale:
- ORCHA's 3-person team cannot maintain two API paradigms
- `ts-rs` already provides type safety (tRPC's primary value)
- REST + OpenAPI + SDK generation (see 1.6) covers external developer needs
- GraphQL's value (client-driven queries) matters most when you have many diverse clients — not applicable until Stage 3+ with marketplace/third-party apps
- Re-evaluate at Stage 3 when third-party integrators need flexible queries

**Budget/timeline**: $0 now. If adopted at Stage 3, estimate 4-6 weeks to stand up GraphQL layer alongside REST.

---

### 1.4 Rate Limiting — governor

**Library**: [governor](https://github.com/antifuchs/governor)
**License**: MIT
**Maturity**: Production-ready, well-maintained, based on GCRA algorithm

**How it works**: Token bucket / GCRA rate limiter. Integrates with Tower middleware via `tower-governor` for Axum.

**Companion**: [tower-governor](https://github.com/benwis/tower-governor) — Tower middleware adapter.
**License**: MIT

**Pros**:
- Zero-allocation fast path (GCRA algorithm)
- Composable: per-IP, per-API-key, per-endpoint granularity
- In-memory (no Redis needed at current scale)
- Axum-native via Tower layer
- Battle-tested algorithm (generic cell rate algorithm)

**Cons**:
- In-memory only — does not work across multiple server instances (need Redis-backed limiter at scale)
- Configuration requires understanding burst vs sustained rates
- Per-IP limiting can be fooled by proxies (need X-Forwarded-For awareness)

**ORCHA-specific considerations**:
- Already identified in roadmap as Stage 0 Priority 3 ("rate limiting to public-facing endpoints")
- Current `tower_governor` is mentioned in roadmap but not yet implemented
- For multi-tenant: rate limits should be per-organization, not just per-IP
- API key-based limiting needed for Stage 1+ (external API consumers)

**Implementation pattern**:
```rust
// Per-IP rate limiting (Stage 0)
use tower_governor::{GovernorLayer, GovernorConfigBuilder};

let governor_conf = GovernorConfigBuilder::default()
    .per_second(10)
    .burst_size(30)
    .finish()
    .unwrap();

let app = Router::new()
    .nest("/api", api_routes)
    .layer(GovernorLayer { config: governor_conf });

// Per-API-key limiting (Stage 1+)
// Custom key extractor that reads X-API-Key header
```

**Roadmap stage fit**: **Stage 0** (basic per-IP), **Stage 1** (per-API-key, per-org).

**Budget/timeline**: 1-2 days for basic per-IP limiting. 1 week for per-API-key with org-level quotas.

**Recommendation**: **Adopt immediately in Stage 0**. This is a security baseline.

---

### 1.5 API Key Management Patterns

No single library — this is an architectural pattern. Components:

#### Key generation and storage
- Generate API keys as cryptographically random tokens (256-bit, base62 encoded)
- Store only the hash (SHA-256 or Argon2) in the database — never store plaintext
- Prefix keys for identification: `orcha_live_` / `orcha_test_` (like Stripe's `sk_live_` / `sk_test_`)
- Display key only once at creation time

#### Key scoping
- Per-organization keys (not per-user) for B2B SaaS
- Optional per-key permission scopes (read-only, read-write, admin)
- Key metadata: created_by, created_at, last_used_at, expires_at

#### Key lifecycle
- Rotation: generate new key, grace period where both work, revoke old
- Expiration: optional TTL, renewal flow
- Revocation: immediate invalidation (requires cache invalidation)

#### Middleware pattern for Axum
```rust
// API key extraction + validation as Axum middleware
async fn api_key_auth(
    State(db): State<DbPool>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key = headers.get("X-API-Key")
        .or_else(|| headers.get("Authorization")) // Bearer token fallback
        .ok_or(ApiError::Unauthorized("Missing API key"))?;

    let key_hash = sha256(key);
    let api_key_record = db.find_api_key_by_hash(&key_hash).await?;
    // Check expiry, scopes, rate limits, org membership
    // Inject org_id into request extensions
    request.extensions_mut().insert(api_key_record.organization_id);
    next.run(request).await
}
```

**Roadmap stage fit**: **Stage 1** (pilot clients need programmatic access).

**Budget/timeline**: 1 week for full implementation (DB model, key generation, middleware, management UI, key rotation).

**Recommendation**: **Implement at Stage 1**. Build the DB model and generation logic early. The management UI can be minimal (admin-only page). Key scoping (permissions per key) can wait until Stage 2.

---

### 1.6 SDK Generation — OpenAPI Generator

**Tool**: [OpenAPI Generator](https://github.com/OpenAPITools/openapi-generator)
**License**: Apache 2.0
**Maturity**: Industry standard, 22K+ stars, 40+ language targets

**How it works**: Takes an OpenAPI spec (generated by utoipa, see 1.1) and generates typed client SDKs for any target language.

**Key targets for ORCHA**:
- TypeScript/JavaScript (most developer integrators)
- Python (data science / ML pipeline users)
- Rust (Rust ecosystem users)
- Go (infrastructure/DevOps users)

**Pros**:
- Single source of truth (OpenAPI spec) generates all SDKs
- Community-maintained templates for 40+ languages
- Customizable templates (Mustache-based)
- CI-automatable: spec change triggers SDK rebuild + publish
- Includes request/response types, error handling, authentication

**Cons**:
- Generated code quality varies by language target (TypeScript is good, some others less so)
- Customization requires learning Mustache template system
- Generated code may not match target language idioms perfectly
- Breaking API changes require SDK version bumps and deprecation management

**Alternatives considered**:
- **Speakeasy** (commercial, $500+/mo): higher quality output, managed publishing. Overkill for Stage 1-2.
- **Fern** (MIT core, commercial features): good TypeScript output, growing ecosystem. Worth evaluating at Stage 2.
- **Manual SDKs**: highest quality, highest maintenance cost. Only for primary language (TypeScript).

**Recommended approach (progressive)**:

| Stage | Approach |
|-------|----------|
| 0-1 | No SDK. Pilots use raw REST + OpenAPI docs |
| 2 | TypeScript SDK via OpenAPI Generator. Publish to npm as `@orcha/sdk` |
| 3 | Python SDK via OpenAPI Generator. Publish to PyPI |
| 4 | Evaluate Speakeasy/Fern for higher-quality multi-language SDKs |

**Budget/timeline**:
- Stage 2: 1 week to set up generation pipeline + CI + npm publishing
- Per additional language: 2-3 days setup + testing

**Recommendation**: **Defer SDK generation to Stage 2**. The prerequisite is a stable, versioned, OpenAPI-documented API (built in Stages 0-1). Premature SDK generation on an unstable API creates maintenance burden.

---

## 2. Developer Experience / Platform

### 2.1 Developer Portal Patterns

A developer portal consists of: API reference (auto-generated from OpenAPI), guides/tutorials (hand-written), API playground (interactive), changelog, and status page.

#### Documentation site options

| Tool | License | Pros | Cons | Stage fit |
|------|---------|------|------|-----------|
| **Docusaurus** (Meta) | MIT | React-based (team knows React), versioned docs, search, i18n | Heavy for small docs | Stage 2 |
| **Mintlify** | Commercial ($150/mo+) | Beautiful out-of-box, OpenAPI auto-import, AI search | Cost, vendor lock-in | Stage 3+ |
| **Starlight** (Astro) | MIT | Lightweight, fast, good DX | Smaller ecosystem | Stage 2 |
| **Swagger UI / Scalar** | Apache 2.0 / MIT | Free, auto-generated from OpenAPI | Reference only, no guides | Stage 1 |

**Recommendation**:
- **Stage 1**: Serve Swagger UI / Scalar from the Axum server itself (utoipa-swagger-ui, utoipa-scalar). Zero additional infrastructure.
- **Stage 2**: Docusaurus or Starlight for full developer portal with guides. Docusaurus preferred because the team already knows React.
- **Stage 3+**: Evaluate Mintlify if developer acquisition becomes a growth lever.

#### API Playground

- **Stage 1**: Swagger UI's "Try it out" feature is sufficient
- **Stage 2**: Consider embedding a Hoppscotch-like playground (MIT) in the developer portal
- **Stage 3**: Custom playground with pre-populated examples and auth token management

**Budget/timeline**: Stage 1: included in utoipa setup (0 additional). Stage 2: 1-2 weeks for Docusaurus site.

---

### 2.2 Webhook Management for Integrations

**Current state**: Webhook triggers partially implemented (PR #49, HMAC validation exists, cooldown race condition unresolved).

**Pattern for outbound webhooks (ORCHA notifying external systems)**:

Components needed:
1. **Webhook registration**: UI + API for users to register URLs per event type
2. **Event catalog**: defined event types (task.created, deal.stage_changed, workflow.completed, etc.)
3. **Delivery engine**: async delivery with retry (exponential backoff, 3-5 attempts)
4. **Signing**: HMAC-SHA256 signature in header for recipient verification
5. **Delivery log**: audit trail of all webhook deliveries (status, response code, latency)
6. **Replay**: ability to re-send failed webhooks from the delivery log

**Implementation approach for Rust/Axum**:
- Tokio background task for webhook delivery queue
- `reqwest` for HTTP delivery
- Database table: `webhook_subscriptions` (org_id, url, events[], secret, active)
- Database table: `webhook_deliveries` (subscription_id, event_type, payload, status, attempts, last_attempt_at)

**Libraries**:
- `reqwest` (MIT/Apache 2.0): HTTP client for delivery
- `hmac` + `sha2` (MIT/Apache 2.0): HMAC signing
- No dedicated "webhook framework" needed — the pattern is simple enough to implement directly

**Roadmap stage fit**: **Stage 1** (outbound webhooks for pilot integrations), **Stage 2** (full management UI + delivery logs).

**Budget/timeline**: 1 week for core delivery engine + registration API. 1 week for management UI + delivery logs.

---

### 2.3 OAuth 2.0 / OIDC for Third-Party App Authorization

**Current state**: GitHub OAuth for user authentication. No OAuth provider capability (ORCHA as authorization server).

For a platform/marketplace (Stage 2+), ORCHA needs to *be* an OAuth provider — allowing third-party apps to request permission to act on behalf of ORCHA users.

#### Library options

| Library | License | Language | Approach |
|---------|---------|----------|----------|
| **oxide-auth** | MIT | Rust | OAuth 2.0 authorization server library for Rust/Axum |
| **openidconnect-rs** | MIT | Rust | OIDC client library (for consuming, not providing) |
| **Custom implementation** | N/A | Rust | Build on `jsonwebtoken` (MIT) + database |

**oxide-auth** is the only mature Rust OAuth provider library. It supports authorization code flow, client credentials, refresh tokens.

**Pros** (oxide-auth):
- Handles the complex OAuth state machine
- Integrates with Axum via `oxide-auth-axum`
- Supports PKCE (required for public clients)

**Cons** (oxide-auth):
- Somewhat complex API surface
- Documentation is adequate but not extensive
- May not support all OIDC features (if OIDC provider needed, may need extension)

**Alternative**: Use an external identity provider (Auth0, Keycloak). This offloads complexity but adds a dependency and cost.

| Approach | Cost | Complexity | Stage fit |
|----------|------|-----------|-----------|
| oxide-auth (self-hosted) | Free | High initial, low ongoing | Stage 2 |
| Keycloak (self-hosted, MIT) | Free (hosting cost) | Medium initial, medium ongoing | Stage 2-3 |
| Auth0 (managed) | Free to $240/mo+ | Low initial, low ongoing | Stage 1-2 |

**Recommendation**: **Stage 1**: Continue GitHub OAuth for user auth. **Stage 2**: Implement OAuth 2.0 provider via `oxide-auth` for third-party app authorization (marketplace prerequisite). If team capacity is constrained, Keycloak is a viable alternative that also provides SSO/SAML for enterprise features.

**Budget/timeline**: oxide-auth: 2-3 weeks. Keycloak: 1 week setup + ongoing ops.

---

### 2.4 Marketplace / Plugin Architecture Patterns

**Current state**: Workflow node types are defined in code. No external plugin system.

**Progressive plugin architecture**:

| Stage | Capability | Mechanism |
|-------|-----------|-----------|
| 1 | Internal workflow templates shared across orgs | Database-stored workflow definitions (already exists) |
| 2 | Workflow marketplace (user-published templates) | Template publishing + discovery + install flow |
| 2-3 | Custom workflow nodes (user-defined) | WASM sandboxed execution or Docker container nodes |
| 3-4 | Third-party apps (OAuth-authorized) | OAuth provider + REST API + webhooks |
| 4-5 | Agent marketplace | MCP-based agent publishing (agents as MCP servers) |

**Key architectural decisions**:

1. **WASM for custom nodes** (Stage 2-3):
   - Users write custom workflow node logic
   - Compiled to WASM, sandboxed execution via `wasmtime` (Apache 2.0)
   - Pros: security isolation, language-agnostic (Rust, Go, C, AssemblyScript)
   - Cons: limited I/O (no network access from WASM without explicit host grants), learning curve
   - Alternative: Docker containers for custom nodes (more flexible I/O, heavier isolation overhead)

2. **MCP as plugin protocol** (Stage 3+):
   - ORCHA already has 4 MCP servers with 26+ tools
   - Third-party agents could be MCP servers that plug into ORCHA's orchestration
   - MCP provides tool discovery, typed schemas, and execution protocol
   - This aligns with the existing architecture and industry direction

**Recommendation**:
- **Stage 2**: Workflow template marketplace (publishing, discovery, install) — no new runtime needed
- **Stage 3**: WASM-based custom workflow nodes via `wasmtime`
- **Stage 3-4**: OAuth + REST + webhooks for third-party app integrations
- **Stage 4-5**: MCP-based agent marketplace

**Budget/timeline**: Template marketplace: 2-3 weeks. WASM nodes: 4-6 weeks. Full third-party app platform: 8-12 weeks.

---

### 2.5 MCP Server as Developer API

**Current state**: Already implemented — 4 MCP servers, 26+ tools.

**This is ORCHA's strongest developer experience asset**. MCP is becoming the standard for AI agent tool integration. ORCHA's MCP servers already allow AI agents (Claude, Cursor, etc.) to manage tasks, projects, CRM, and workflows programmatically.

**Enhancements for developer experience**:

| Enhancement | Stage | Effort |
|-------------|-------|--------|
| MCP server documentation (tool catalog, examples) | 1 | 1 week |
| MCP authentication (API key-based, not just local) | 1-2 | 1 week |
| Remote MCP server hosting (expose to cloud AI agents) | 2 | 2 weeks |
| MCP tool versioning | 2-3 | 1 week |
| Third-party MCP server registration | 3-4 | 2-3 weeks |

**Recommendation**: **Leverage MCP as the primary developer API for AI-native integrations**. REST API + SDK for traditional integrations. MCP for AI agent integrations. This dual-track approach differentiates ORCHA from competitors.

---

## 3. Team Scaling Engineering Practices

### 3.1 Monorepo vs Polyrepo Decision Framework

**Current state**: Monorepo — 23 Rust crates in one repo + React frontend. This is correct for the current team size.

**Decision framework**:

| Factor | Monorepo wins when... | Polyrepo wins when... |
|--------|----------------------|----------------------|
| Team size | < 30 engineers | > 50 engineers |
| Deploy cadence | Components deploy together | Independent deploy cycles |
| Code sharing | Heavy cross-crate dependencies | Minimal shared code |
| CI/CD | Build caching handles scale | CI time > 30 min even cached |
| Ownership | Shared ownership culture | Strong team boundaries |

**ORCHA's situation**: 23 tightly coupled Rust crates with shared `db`, `utils`, `services` dependencies. Frontend proxies to backend. Deploy as a unit. **Monorepo is correct through Stage 4 (25 people)**.

**When to re-evaluate**:
- When CI times exceed 30 minutes with caching (currently ~5-10 min with sccache)
- When teams want independent deploy cadences (e.g., frontend team ships daily, backend ships weekly)
- When a service genuinely has zero shared code with the rest (candidate: APN node, VIBELAND)

**Monorepo tooling to add as team grows**:

| Tool | Purpose | When | License |
|------|---------|------|---------|
| **Turborepo** or **Nx** | Frontend build orchestration | Stage 2 (5+ people) | MIT |
| **cargo-hakari** | Workspace-hack for faster Rust builds | Stage 1 (when builds slow) | MIT/Apache 2.0 |
| **CODEOWNERS** | See 3.2 below | Stage 1 (4+ people) | N/A (GitHub feature) |

**Recommendation**: **Stay monorepo through Stage 4**. Add build optimization tooling (cargo-hakari, Turborepo) as compile times grow. First polyrepo candidate: APN node (Stage 5, genuinely independent service).

---

### 3.2 Code Ownership (CODEOWNERS)

**When to add**: Stage 1 (4+ people, when the first hire joins).

**Pattern**: GitHub `CODEOWNERS` file maps paths to responsible team members. PRs touching those paths require review from owners.

**Recommended initial structure for ORCHA**:

```
# .github/CODEOWNERS

# Default — founding team reviews everything
* @founding-team

# Domain ownership (add as team grows)
/crates/db/           @backend-lead
/crates/server/       @backend-lead
/frontend/            @frontend-lead
/crates/nora/         @voice-lead
/crates/apn-client/   @infra-lead
/crates/alpha-protocol-core/  @infra-lead

# Critical paths — require senior review
/crates/db/migrations/  @senior-dev
/.github/workflows/     @senior-dev
/Cargo.toml            @senior-dev
```

**Scaling pattern**:
- Stage 1 (4 people): CODEOWNERS with broad ownership (1-2 owners per area)
- Stage 2 (5-8 people): Refine to domain teams (backend, frontend, infra)
- Stage 3+ (8+ people): Per-crate ownership, require 2 reviewers for critical paths

**Budget/timeline**: 30 minutes to set up. Update as team grows.

**Recommendation**: **Implement at Stage 1**. Zero cost, high value for PR review quality.

---

### 3.3 RFC/ADR Process for Architectural Decisions

**RFC (Request for Comments)**: Propose significant changes before implementation.
**ADR (Architecture Decision Record)**: Document decisions after they're made.

**Current state**: ORCHA uses planning docs (`planning/YYYY-MM-DD--plan--*.md`) which serve a similar purpose. Formalize this into an ADR process as team grows.

**Recommended ADR format** (lightweight, fits existing conventions):

```markdown
# ADR-NNNN: <Title>

**Date**: YYYY-MM-DD
**Status**: proposed | accepted | deprecated | superseded by ADR-XXXX
**Deciders**: <names>

## Context
What is the issue that we're seeing that is motivating this decision?

## Decision
What is the change that we're proposing?

## Consequences
What becomes easier or more difficult because of this change?
```

**When to require an ADR**:
- New external dependency added to `Cargo.toml` or `package.json`
- Database schema changes affecting 3+ tables
- New crate added to workspace
- API breaking changes
- Authentication/authorization changes
- New infrastructure component

**Tooling**: Store in `docs/adrs/` directory. No special tooling needed — markdown files in the repo. Optional: [adr-tools](https://github.com/npryce/adr-tools) (MIT) for templating.

**Roadmap stage fit**: **Stage 1** (formalize when team grows beyond founders).

**Budget/timeline**: 1 hour to set up directory + template. Ongoing: 1-2 hours per ADR.

**Recommendation**: **Adopt at Stage 1**. Existing planning doc convention is the seed — formalize into ADRs when decisions become architectural (not just task-level plans).

---

### 3.4 On-Call Rotation and Incident Management

**Current state**: No formal on-call. 3-person team handles issues ad-hoc.

**Progressive on-call strategy**:

| Stage | Team size | Approach |
|-------|-----------|----------|
| 0 | 3 | Informal. Sentry alerts to shared channel. Whoever sees it handles it. |
| 1 | 4-5 | Weekly primary on-call rotation. PagerDuty free tier (5 users). Runbooks for common issues. |
| 2 | 5-8 | Primary + secondary rotation. Incident severity levels (P1-P4). Post-incident reviews for P1/P2. |
| 3+ | 8+ | Full incident management. Dedicated SRE hire. SLA tracking. Status page. |

**Tools**:

| Tool | License/Cost | When |
|------|-------------|------|
| **Sentry** | Free tier (5K events/mo) | Stage 0 (already in use) |
| **PagerDuty** | Free (5 users) → $21/user/mo | Stage 1 |
| **Grafana OnCall** | Apache 2.0 (self-hosted) | Stage 2 alternative to PagerDuty |
| **Statuspage** (Atlassian) | $29/mo | Stage 2 (external status page for pilots) |
| **Betteruptime** | Free tier | Stage 1-2 alternative |

**Runbook starter set (create at Stage 1)**:
1. Backend won't start (port conflict, DB locked, migration failure)
2. Frontend build failure (type generation out of sync)
3. Agent execution stuck (executor timeout, MCP connection failure)
4. Database performance degradation (SQLite lock contention)
5. SSE connection drops (proxy timeout, server restart)

**Recommendation**: **Stage 0**: Continue informal + Sentry. **Stage 1**: PagerDuty free tier + weekly rotation + 5 runbooks. **Stage 2**: Severity levels + post-incident reviews.

---

### 3.5 Knowledge Base and Documentation Culture

**Current state**: Good internal docs (CLAUDE.md, MEMORY.md, architecture docs). No external docs. No structured onboarding.

**Progressive documentation strategy**:

| Stage | Focus | Tooling |
|-------|-------|---------|
| 0 | CLAUDE.md + MEMORY.md (already excellent) | Markdown in repo |
| 1 | Onboarding guide for hire #1. API docs (utoipa). ADRs. | Markdown in repo + Swagger UI |
| 2 | External developer docs. Internal knowledge base. | Docusaurus (external) + Notion/Outline (internal) |
| 3+ | Developer relations content. Video tutorials. | Docusaurus + YouTube/Loom |

**Internal knowledge base options**:

| Tool | License/Cost | Pros | Cons |
|------|-------------|------|------|
| **Outline** | BSL → MIT (self-hosted) | Beautiful, Markdown, API, self-hosted | Requires hosting |
| **Notion** | Free (small team) → $10/user/mo | Universal familiarity | Vendor lock-in, not self-hosted |
| **Markdown in repo** | Free | Version controlled, searchable | No rich collaboration |
| **Confluence** | $6/user/mo | Enterprise standard | Heavy, slow, not developer-friendly |

**Recommendation**: **Stage 0-1**: Markdown in repo (current approach is good). **Stage 2**: Add Outline (self-hosted, aligns with sovereignty values) or Notion (pragmatic, familiar) for internal wiki. Add Docusaurus for external developer docs.

**Key documentation to write at each stage**:
- Stage 0: Update CLAUDE.md as ground truth (ongoing)
- Stage 1: New hire onboarding guide (1 day), API reference (auto-generated), first 5 runbooks
- Stage 2: Architecture overview for external developers, getting-started guide, integration tutorials
- Stage 3: Marketplace developer guide, agent development guide, video content

---

## 4. Rust Ecosystem Team Patterns

### 4.1 Trait-Based Plugin Systems

**Current state**: Executor pattern exists (`crates/executors/`) with pluggable AI agent backends. This is already a trait-based plugin system.

**Formalization pattern**:

```rust
// Define a plugin trait with async methods
#[async_trait]
pub trait WorkflowNode: Send + Sync {
    fn node_type(&self) -> &str;
    fn schema(&self) -> schemars::Schema; // Input/output schema
    async fn execute(&self, input: Value, ctx: &ExecutionContext) -> Result<Value, NodeError>;
}

// Plugin registry
pub struct NodeRegistry {
    nodes: HashMap<String, Box<dyn WorkflowNode>>,
}

impl NodeRegistry {
    pub fn register<N: WorkflowNode + 'static>(&mut self, node: N) {
        self.nodes.insert(node.node_type().to_string(), Box::new(node));
    }
}

// Feature-gated registration
#[cfg(feature = "editron")]
registry.register(EditronNode::new());
```

**Key patterns for team scaling**:
- **Trait objects (`Box<dyn Trait>`)** for runtime polymorphism (plugin registration)
- **Generics (`impl Trait`)** for compile-time polymorphism (internal hot paths)
- **Inventory crate** (MIT) for automatic plugin discovery at link time (no manual registration)
- **linkme** (MIT/Apache 2.0) for distributed slice pattern (register plugins across crates)

**Recommendation**: **Formalize the existing executor pattern into a documented plugin trait system in Stage 1**. This becomes the foundation for third-party workflow nodes (Stage 2-3) and agent marketplace (Stage 4-5).

**Budget/timeline**: 1 week to formalize + document the trait pattern. Existing code already follows this pattern implicitly.

---

### 4.2 Feature Flags in Cargo for Optional Modules

**Current state**: Some feature flags exist (e.g., PostgreSQL feature gate), but not systematically used.

**Pattern for team scaling**:

```toml
# crates/server/Cargo.toml
[features]
default = ["crm", "tasks", "workflows", "agents"]
crm = ["db/crm"]
tasks = ["db/tasks"]
workflows = ["db/workflows"]
agents = ["executors"]
voice = ["nora"]           # Optional: Nora voice interface
apn = ["apn-client"]       # Optional: APN mesh networking
editron = ["editron-runner"] # Optional: Editron integration
analytics = ["dep:posthog"] # Optional: analytics
```

**Benefits**:
- Faster compile times during development (disable unused features)
- Smaller binaries for deployment (only include needed features)
- Clear dependency graph between subsystems
- Enables "edition" packaging (free tier excludes `voice`, `apn`, `analytics`)

**When features become valuable**:
- Stage 1: When compile times start to matter (currently ~5-10 min)
- Stage 2: When packaging for different deployment targets (cloud vs self-hosted vs edge)
- Stage 3: When pricing tiers map to feature sets

**Cons**:
- Feature flag combinations create testing matrix (2^N configurations)
- Conditional compilation can hide bugs
- `#[cfg(feature = "...")]` annotations add noise
- CI must test multiple feature combinations

**Recommendation**: **Stage 1** — add feature flags for clearly separable subsystems (voice, APN, Editron). Do not over-segment. **Stage 2** — map features to pricing tiers. Keep the default feature set comprehensive to avoid breaking existing workflows.

**Budget/timeline**: 1-2 days for initial feature flag setup. Ongoing maintenance cost is low.

---

### 4.3 Crate Extraction Strategies

**Current state**: 23 crates, well-structured but tightly coupled through `db` and `utils`.

**When to extract a new crate**:
1. **Reusability**: Code is needed by multiple parent crates AND has no upward dependency
2. **Compile time**: A large crate's compile time exceeds 30 seconds
3. **Ownership**: A distinct team will own the code
4. **Deployment**: The code deploys independently (e.g., CLI tool, APN node)

**When NOT to extract**:
- "Just for organization" — use modules within the existing crate instead
- When it would create circular dependencies
- When it would require duplicating types or traits

**Current extraction candidates**:

| Candidate | Reason | Stage | From |
|-----------|--------|-------|------|
| `crates/auth` | Auth logic currently spread across `server` + `services` | Stage 1 | `server/auth.rs` + `services/auth` |
| `crates/webhooks` | Webhook delivery engine (see 2.2) | Stage 1 | New code |
| `crates/billing` | Stripe integration (see roadmap Priority 19) | Stage 2 | New code |
| `crates/wasm-runtime` | WASM plugin execution (see 2.4) | Stage 3 | New code |

**Anti-pattern to avoid**: Don't create a crate for every "concept." The current structure (db, server, services, executors, utils) is good. Crate boundaries should map to compilation units and ownership, not domain concepts.

**Recommendation**: **Extract cautiously**. Current 23-crate structure is healthy. Add new crates for genuinely new subsystems (billing, webhooks). Don't split existing crates unless compile time or ownership demands it.

---

### 4.4 Internal Crate Documentation Standards

**Current state**: Code comments exist but are inconsistent. No enforced documentation standard.

**Recommended standard (adopt incrementally)**:

```rust
//! # Crate-level documentation
//!
//! Brief description of what this crate does and who depends on it.
//!
//! ## Usage
//!
//! ```rust
//! use orcha_db::models::Task;
//! ```
//!
//! ## Architecture
//!
//! Describe key patterns, entry points, and gotchas.

/// Public function documentation.
///
/// # Arguments
/// * `org_id` - The organization to scope the query to
///
/// # Errors
/// Returns `DbError::NotFound` if no matching record exists.
///
/// # Examples
/// ```
/// let tasks = list_tasks(&pool, org_id).await?;
/// ```
pub async fn list_tasks(pool: &SqlitePool, org_id: DbUuid) -> Result<Vec<Task>, DbError> {
```

**Enforcement**:
- `#![warn(missing_docs)]` on public crates (db, services, utils)
- `#![deny(missing_docs)]` on API boundary crates (server — public endpoints)
- CI step: `cargo doc --no-deps --workspace` must succeed without warnings

**Roadmap stage fit**: **Stage 1** (when onboarding first hire). Start with crate-level `//!` docs for all 23 crates. Add function-level docs to public APIs incrementally.

**Budget/timeline**: 2-3 days for initial crate-level docs. Ongoing: part of PR review.

---

### 4.5 Error Handling Patterns at Scale

**Current state**: Uses both `thiserror` (typed errors) and `anyhow` (context-rich errors). This is the correct dual approach.

**Libraries**:

| Library | License | Purpose | Current usage |
|---------|---------|---------|---------------|
| **thiserror** | MIT/Apache 2.0 | Typed error enums for library crates | Used in `crates/db`, `crates/services` |
| **anyhow** | MIT/Apache 2.0 | Contextual errors for application code | Used in `crates/server` route handlers |

**Scaling patterns**:

1. **Library crates** (db, services, utils): Use `thiserror` for typed error enums. Callers can match on variants.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Record not found: {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("Duplicate key: {field}")]
    DuplicateKey { field: String },
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
```

2. **Application crate** (server): Use `anyhow` or convert typed errors into HTTP responses.

```rust
// Axum error response type
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Internal(String),
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound { .. } => ApiError::NotFound(err.to_string()),
            DbError::DuplicateKey { .. } => ApiError::BadRequest(err.to_string()),
            DbError::Sqlx(_) => ApiError::Internal("Internal database error".into()),
        }
    }
}
```

3. **Error context enrichment** (Stage 2+): Use `error-stack` (MIT/Apache 2.0, by Hash) for rich error context chains. Better than `anyhow` for production debugging because it preserves typed error variants while adding context.

**Important pattern from MEMORY.md**: "Error messages must be actionable and role-scoped (no platform details to end users)." This means:
- User-facing errors: human-readable, actionable ("Task not found. Check the task ID and try again.")
- Internal errors: full context chain (DB query, parameters, stack trace) logged to tracing
- Never expose SQL errors, file paths, or internal state to API responses

**Recommendation**: **Current approach is correct**. Continue `thiserror` for library crates, `anyhow`/`ApiError` for server. Evaluate `error-stack` at Stage 2 when production debugging becomes critical. Add the role-scoped error pattern to API error responses.

---

## 5. Frontend Architecture Scaling

### 5.1 Micro-Frontends Evaluation

**Verdict: Do not adopt.**

Micro-frontends solve a team coordination problem at 50+ frontend developers with independent deploy cadences. ORCHA has 1-2 frontend developers through Stage 3.

**Problems micro-frontends create**:
- Shared state management becomes complex (how do independently deployed apps share auth, notifications?)
- Bundle size increases (each micro-frontend bundles its own React)
- CSS isolation challenges
- Development experience degrades (can't just `pnpm run dev`)
- Testing across micro-frontend boundaries is painful

**When it would make sense for ORCHA**: Stage 5 (25+ engineers), if the platform genuinely has independent sub-products with separate teams. Even then, a well-structured monolith is often better.

**Recommendation**: **Do not adopt**. The monorepo React app with good module boundaries (feature-based directory structure) scales to 15+ engineers without micro-frontends.

---

### 5.2 Module Federation with Vite

**Library**: [@module-federation/vite](https://github.com/module-federation/vite) or native Vite federation (experimental)
**License**: MIT

Module federation allows separately built frontend modules to share dependencies at runtime. Primary use case: plugin system where third-party UI components load into the host app.

**Pros**:
- Enables third-party UI plugins (marketplace widgets, custom dashboard components)
- Shared dependencies (React, design system) loaded once
- Independent build/deploy of plugin modules

**Cons**:
- Vite support is less mature than webpack's (webpack originated module federation)
- Runtime dependency resolution adds complexity and failure modes
- Version mismatches between host and remote modules cause subtle bugs
- Significant DevX overhead for a small team

**Recommendation**: **Defer to Stage 3-4**. Only relevant when building a marketplace/plugin system with third-party UI components. Until then, the added complexity is not justified.

---

### 5.3 Shared Component Library Extraction

**Current state**: 311+ React components in `frontend/src/components/`. Using shadcn/ui as the base design system.

**Extraction strategy**:

| Stage | Approach |
|-------|----------|
| 0-2 | Components stay in-app. shadcn/ui primitives + custom components in `components/ui/`. |
| 2-3 | Extract stable, generic components into `packages/ui` within the monorepo. |
| 3-4 | If multiple frontends exist (main app, developer portal, admin dashboard), publish as `@orcha/ui` npm package. |

**What to extract first** (Stage 2-3):
- Design tokens (colors, spacing, typography) — already in Tailwind config
- Primitives (Button, Dialog, Input, Select) — shadcn/ui handles this
- Domain components (TaskCard, KanbanBoard, PipelineView) — extract when reused across apps

**Tooling**: Monorepo package (Turborepo or pnpm workspaces) before npm publishing. Only publish externally when there are external consumers.

**Recommendation**: **Stage 0-2: Keep components in-app** (current approach is fine). **Stage 2-3**: Extract into monorepo package if building developer portal as separate app. Do not prematurely extract.

---

### 5.4 Design System Maintenance — Storybook

**Library**: [Storybook](https://storybook.js.org/)
**License**: MIT
**Maturity**: Industry standard, massive ecosystem

**Pros**:
- Visual component catalog for design review
- Isolated component development (test components without full app context)
- Accessibility testing addon (a11y)
- Visual regression testing (Chromatic integration)
- Onboarding tool (new developers browse component library)

**Cons**:
- Significant setup and maintenance overhead
- Story files add code surface area (~1 story per component)
- Can fall out of sync with actual usage if not enforced
- Build time increases (Storybook builds separately)

**When Storybook becomes valuable**:
- When a designer joins the team (Stage 3, hire #5)
- When multiple developers work on UI simultaneously
- When visual regression testing is needed

**Recommendation**: **Defer to Stage 2-3**. For a 3-person team with no dedicated designer, Storybook is overhead. Invest in it when the product designer joins and component consistency matters across multiple developers.

**Budget/timeline**: 1 week initial setup + 2-3 days to write stories for core components. Ongoing: ~30 min per new component.

---

### 5.5 State Management at Scale

**Current state**: React Query (TanStack Query) for server state + React Context for client state. This is a good foundation.

**Evaluation of alternatives**:

| Tool | License | What it does | When to consider |
|------|---------|-------------|-----------------|
| **React Query** (current) | MIT | Server state, caching, sync | Already adopted. Excellent choice. |
| **React Context** (current) | MIT | Lightweight client state | Already adopted. Fine for auth, theme, sidebar state. |
| **Zustand** | MIT | Lightweight client state store | If Context prop drilling becomes painful (5+ nested contexts) |
| **Jotai** | MIT | Atomic state (bottom-up) | If complex interdependent UI state emerges (e.g., workflow editor canvas state) |
| **Redux Toolkit** | MIT | Heavy client state management | **Do not adopt.** Overkill for ORCHA's needs. |

**Current state management is correct for ORCHA's scale.** React Query handles 90% of state (server data), Context handles the rest (auth, UI preferences).

**When to add Zustand** (Stage 2-3):
- Workflow editor canvas state (zoom, pan, selection, drag state) is complex enough to warrant a dedicated store
- VIBELAND 3D environment state (camera, avatar position, room state)
- When you have 5+ React Contexts and prop drilling becomes painful

**Recommendation**: **Stay with React Query + Context through Stage 2**. Add Zustand for specific complex UI state (workflow editor, 3D environment) if needed. Never adopt Redux.

---

## 6. Internationalization (i18n)

### 6.1 React i18n Frameworks

#### react-i18next

**Library**: [react-i18next](https://react.i18next.com/)
**License**: MIT
**Maturity**: Industry standard, 9K+ stars, massive ecosystem

**Pros**:
- Most adopted React i18n library
- Namespace support (split translations by feature/page for lazy loading)
- Interpolation, pluralization, context-based translations
- ICU message format support (via plugin)
- Backend plugins for loading translations from APIs, files, or services
- Excellent TypeScript support
- Runtime language switching

**Cons**:
- Bundle size (~12KB gzipped for i18next + react-i18next)
- Learning curve for advanced features (namespaces, backends, interpolation)
- Translation key management requires discipline (easy to have orphaned keys)

#### FormatJS (react-intl)

**Library**: [FormatJS](https://formatjs.io/) / react-intl
**License**: MIT
**Maturity**: Production-ready, 14K+ stars, backed by ex-Yahoo engineers

**Pros**:
- ICU message format native (industry standard for complex translations)
- Compile-time message extraction (ensures no missing translations)
- Strong TypeScript types for messages
- Smaller runtime than i18next for basic use cases
- Better handling of pluralization, gender, ordinals (ICU strength)

**Cons**:
- Less flexible than i18next for dynamic backends
- Steeper learning curve for ICU message syntax
- Smaller plugin ecosystem than i18next
- Build-time extraction step adds CI complexity

**Recommendation**: **react-i18next** (Stage 2-3).

Rationale: Larger ecosystem, more flexible backend options (important for loading translations dynamically), simpler initial setup. FormatJS is technically superior for complex pluralization, but i18next handles ORCHA's use cases adequately.

**Implementation approach**:
1. Wrap the app in `I18nextProvider`
2. Extract all user-facing strings into `public/locales/{lang}/{namespace}.json`
3. Use `useTranslation` hook in components: `const { t } = useTranslation('tasks'); t('createTask')`
4. Start with `en` as base language. Add languages when market demands it.

**Budget/timeline**: 2-3 weeks for initial extraction of existing strings (60+ pages, 311+ components). Ongoing: part of development workflow.

---

### 6.2 Rust-Side Localization — fluent-rs

**Library**: [fluent-rs](https://github.com/projectfluent/fluent-rs)
**License**: Apache 2.0
**Maturity**: Production-ready, Mozilla project, used in Firefox

**How it works**: Fluent is a localization system designed for natural-sounding translations. Unlike key-value systems, Fluent uses a custom syntax that handles pluralization, gender, and grammatical cases natively.

```ftl
# messages.ftl
task-created = Task "{$name}" was created.
tasks-count = { $count ->
    [one] {$count} task
   *[other] {$count} tasks
}
```

**Pros**:
- Designed for asymmetric localization (translations can be structurally different from source)
- Natural pluralization and gender handling
- Isolating model: translator errors can't break the app (graceful fallback)
- Mozilla-backed, used in Firefox (billions of users)

**Cons**:
- Custom syntax (not ICU MessageFormat) — translators need to learn it
- Smaller ecosystem than ICU-based tools
- Not as many translation management tools support Fluent natively

**When ORCHA needs Rust-side localization**:
- Error messages returned from API (currently English-only)
- Email notifications and templates
- PDF/document generation with localized content
- CLI tool output (crates/cli, crates/setup-wizard)

**Recommendation**: **Stage 3+**. Most localization happens on the frontend (React). Rust-side localization only matters for server-generated content (emails, notifications, error messages). Until ORCHA has non-English markets, server-side localization is premature.

**Budget/timeline**: 1-2 weeks when needed. Lower priority than frontend i18n.

---

### 6.3 RTL Support Considerations

RTL (right-to-left) support is required for Arabic, Hebrew, Persian, and Urdu markets.

**What RTL requires**:
1. **CSS logical properties**: Replace `margin-left` with `margin-inline-start`, `text-align: left` with `text-align: start`, etc.
2. **Tailwind CSS RTL plugin**: `tailwindcss-rtl` (MIT) adds `ms-*` (margin-start), `me-*` (margin-end) utilities
3. **HTML `dir` attribute**: `<html dir="rtl" lang="ar">`
4. **Icon flipping**: Directional icons (arrows, chevrons) need mirroring
5. **Layout mirroring**: Sidebar moves to right, navigation flows right-to-left

**ORCHA-specific challenges**:
- Kanban board: column order reverses in RTL
- Workflow editor: node flow direction reverses
- Timeline/Gantt views: time axis reverses
- These complex UIs need explicit RTL testing

**Recommendation**: **Stage 3-4**. RTL is a significant engineering effort. Only invest when entering Arabic, Hebrew, or Persian markets. The prerequisite is i18n infrastructure (Stage 2-3).

**Budget/timeline**: 2-4 weeks for full RTL support across 60+ pages. Higher cost if CSS is not already using logical properties.

**Mitigation**: Start using Tailwind logical property classes (`ms-*`, `me-*`, `ps-*`, `pe-*`) in new code starting Stage 1. This is free and makes future RTL adoption cheaper.

---

### 6.4 Localization Workflow and Translation Management

**Tools**:

| Tool | Cost | Pros | Cons | Stage fit |
|------|------|------|------|-----------|
| **Crowdin** | Free (OSS) → $40/mo | Industry standard, i18next integration, in-context editing | Cost at scale | Stage 2-3 |
| **Lokalise** | $120/mo+ | Good DX, GitHub integration, QA checks | Expensive for startups | Stage 3+ |
| **Weblate** | Free (self-hosted, GPLv3) | Self-hosted, sovereign data | Ops overhead | Stage 2 (sovereign-aligned) |
| **Manual JSON files** | Free | Simple, no vendor | Doesn't scale past 2-3 languages | Stage 2 |

**Workflow**:
1. Developer writes UI with translation keys
2. CI extracts new/changed keys
3. Translation management tool shows untranslated keys to translators
4. Translators submit translations (with context screenshots)
5. CI pulls translations back into repo
6. Automated checks: missing translations, placeholder mismatches, length limits

**Recommendation**: **Stage 2**: Manual JSON files for first additional language. **Stage 3**: Crowdin or Weblate for professional translation management.

---

## 7. Accessibility

### 7.1 WCAG 2.1 AA Compliance Patterns

WCAG 2.1 AA is the standard for web accessibility. Key requirements:

| Principle | Requirement | ORCHA impact |
|-----------|------------|--------------|
| **Perceivable** | Color contrast 4.5:1, alt text for images, captions for video | Audit current Tailwind theme for contrast ratios |
| **Operable** | Keyboard navigable, no timing traps, skip navigation | Kanban board and workflow editor need keyboard support |
| **Understandable** | Consistent navigation, error identification, labels | Form validation messages, consistent layout |
| **Robust** | Valid HTML, ARIA roles, assistive tech compatible | Semantic HTML, proper ARIA for custom components |

**Current state assessment**:
- shadcn/ui components are built on Radix UI primitives, which have **excellent** built-in accessibility (ARIA roles, keyboard navigation, focus management)
- Custom components (KanbanBoard, WorkflowEditor, TaskCard) likely need accessibility work
- Tailwind's default theme has good contrast ratios, but custom theme colors should be audited

**Compliance timeline**:

| Stage | Level | Effort |
|-------|-------|--------|
| 0-1 | Foundational: semantic HTML, ARIA on custom components, keyboard navigation for core flows | 1-2 weeks |
| 2 | WCAG 2.1 AA audit + remediation for primary user flows | 2-4 weeks |
| 3+ | Full WCAG 2.1 AA compliance, automated testing in CI | Ongoing |

**Recommendation**: **Stage 1**: Fix low-hanging fruit (semantic HTML, ARIA labels on custom components). **Stage 2**: Formal WCAG audit + remediation. Accessibility becomes a competitive differentiator for enterprise sales (Stage 2-3).

---

### 7.2 React Accessibility Testing

#### axe-core

**Library**: [axe-core](https://github.com/dequelabs/axe-core) / [@axe-core/react](https://github.com/dequelabs/axe-core-npm)
**License**: MPL 2.0
**Maturity**: Industry standard, used by Microsoft, Google, etc.

**Usage modes**:
1. **Development overlay** (`@axe-core/react`): Shows accessibility violations in browser console during development
2. **Playwright integration** (`@axe-core/playwright`): Automated accessibility testing in CI
3. **jest-axe**: Unit test accessibility of rendered components

**Pros**:
- Catches 30-50% of WCAG violations automatically
- Zero false positives (high confidence results)
- Integrates with existing Playwright test infrastructure
- Component-level and page-level scanning

**Cons**:
- Cannot catch all accessibility issues (keyboard flow, cognitive load, screen reader experience require manual testing)
- MPL 2.0 license requires sharing modifications to axe-core itself (but using it as a dependency is fine)

#### jest-axe

**Library**: [jest-axe](https://github.com/nickcolley/jest-axe)
**License**: MIT

**Pros**: Simple assertion in component tests: `expect(await axe(container)).toHaveNoViolations()`
**Cons**: Requires rendering components in JSDOM (less accurate than browser-based testing)

**Recommended testing strategy**:

| Layer | Tool | When |
|-------|------|------|
| Development | `@axe-core/react` overlay | Stage 1 (add to dev setup) |
| E2E tests | `@axe-core/playwright` | Stage 2 (add to existing Playwright suite) |
| CI gate | axe violations = test failure | Stage 2 (block PRs with new violations) |
| Manual | Screen reader testing (VoiceOver, NVDA) | Stage 2-3 (quarterly audits) |

**Budget/timeline**: 1 day to add `@axe-core/react` to dev overlay. 1 week to integrate with Playwright tests.

---

### 7.3 Keyboard Navigation Patterns

**Core requirements**:
- All interactive elements focusable via Tab
- Focus visible indicator (outline) on all focusable elements
- Escape closes modals/dialogs/dropdowns
- Arrow keys navigate within composite widgets (lists, menus, tabs)
- Enter/Space activates buttons and links

**ORCHA-specific keyboard patterns needed**:

#### Kanban Board
- Tab between columns
- Arrow keys between cards within a column
- Enter to open card detail
- Keyboard shortcut to move card between columns (e.g., Ctrl+Arrow)
- Screen reader: announce column name and card count

#### Workflow Editor (Canvas)
- Tab between nodes
- Arrow keys to navigate between connected nodes
- Enter to open node config
- Delete to remove selected node
- Keyboard shortcut to add new node (e.g., Ctrl+N)
- Screen reader: describe node type, connections, position in flow

#### Dialog/Modal
- Focus trap (Tab stays within dialog)
- Escape closes dialog
- Focus returns to trigger element on close
- Radix UI Dialog (used by shadcn/ui) handles this automatically

**Implementation**:
- shadcn/ui (Radix-based) handles Dialog, Dropdown, Select, Tabs keyboard patterns
- Custom components (Kanban, Workflow Editor) need explicit keyboard event handlers
- Use `react-aria` (Apache 2.0) utilities for complex custom widgets if needed

**Recommendation**: **Stage 1**: Verify shadcn/ui keyboard patterns work correctly. **Stage 2**: Add keyboard navigation to Kanban board and Workflow Editor. These are the highest-impact custom components.

---

### 7.4 Screen Reader Compatibility for Complex UIs

**Challenge areas**:
- Kanban boards: visual layout is meaningless to screen readers
- Workflow editors: graph/canvas UIs are inherently visual
- Drag-and-drop: no native screen reader support

**Patterns**:

#### Kanban Board
- Use ARIA live regions to announce card movements
- Provide a table/list alternative view (switchable)
- Each column: `role="region"` with `aria-label="In Progress (5 tasks)"`
- Each card: `role="article"` with meaningful `aria-label`
- Drag-and-drop: provide keyboard move alternative (select card → Ctrl+Arrow to move)

#### Workflow Editor
- Provide a list/table view of nodes and connections as alternative
- Node graph as `role="tree"` with `aria-label` for each node
- Use `aria-describedby` to announce node connections
- This is one of the hardest accessibility challenges — invest in Stage 3+

#### Data Tables
- Use semantic `<table>`, `<th>`, `<td>` elements (not divs styled as tables)
- `scope` attributes on headers
- sortable columns announced via `aria-sort`

**Recommendation**: **Stage 2**: Kanban board accessibility (table alternative view + keyboard move). **Stage 3**: Workflow editor accessibility (list alternative + keyboard navigation). These are the two hardest components to make accessible.

**Budget/timeline**: Kanban accessibility: 1-2 weeks. Workflow editor accessibility: 2-3 weeks (most complex).

---

## 8. Summary Decision Matrix

### Adopt Now (Stage 0)

| Item | Why now | Effort | License |
|------|---------|--------|---------|
| **governor / tower-governor** rate limiting | Security baseline | 1-2 days | MIT |
| **Tailwind logical properties** in new code | Free RTL preparation | 0 (habit change) | N/A |

### Adopt at Stage 1 (Pilots)

| Item | Why Stage 1 | Effort | License |
|------|-------------|--------|---------|
| **utoipa** OpenAPI generation | Pilot clients need API docs | 1-2 weeks | MIT/Apache 2.0 |
| **URL-based API versioning** (`/api/v1/`) | API stability for pilots | 1-2 days | N/A |
| **CODEOWNERS** | PR review quality with first hire | 30 min | N/A |
| **ADR process** | Document architectural decisions | 1 hour setup | N/A |
| **API key management** | Programmatic access for pilots | 1 week | N/A |
| **Trait-based plugin formalization** | Foundation for extensibility | 1 week | N/A |
| **Cargo feature flags** | Build optimization | 1-2 days | N/A |
| **@axe-core/react** dev overlay | Low-cost accessibility baseline | 1 day | MPL 2.0 |
| **PagerDuty** free tier + runbooks | On-call for reliability | 1 day + 2 days runbooks | Free tier |
| **Crate-level documentation** | Onboarding first hire | 2-3 days | N/A |

### Adopt at Stage 2 (SaaS Launch)

| Item | Why Stage 2 | Effort | License |
|------|-------------|--------|---------|
| **TypeScript SDK** via OpenAPI Generator | Developer adoption | 1 week | Apache 2.0 |
| **Docusaurus** developer portal | External developer docs | 1-2 weeks | MIT |
| **Outbound webhooks** delivery engine | Integration capability | 2 weeks | N/A |
| **OAuth 2.0 provider** (oxide-auth) | Third-party app authorization | 2-3 weeks | MIT |
| **Workflow template marketplace** | Network effects | 2-3 weeks | N/A |
| **react-i18next** | First non-English market | 2-3 weeks | MIT |
| **axe-core/playwright** CI integration | Accessibility in CI | 1 week | MPL 2.0 |
| **Kanban board accessibility** | Enterprise sales requirement | 1-2 weeks | N/A |
| **Storybook** | Designer joining team | 1 week setup | MIT |
| **Zustand** (if needed) | Complex canvas state | 2-3 days | MIT |
| **Outline** or **Notion** internal wiki | Team knowledge sharing | 1-2 days | BSL/MIT or SaaS |

### Adopt at Stage 3+ (PMF / Growth)

| Item | Why deferred | Effort | License |
|------|-------------|--------|---------|
| **GraphQL** (async-graphql) | Third-party flexible queries | 4-6 weeks | MIT/Apache 2.0 |
| **WASM plugin runtime** (wasmtime) | Custom workflow nodes | 4-6 weeks | Apache 2.0 |
| **Module federation** | Third-party UI plugins | 3-4 weeks | MIT |
| **fluent-rs** server localization | Non-English server content | 1-2 weeks | Apache 2.0 |
| **RTL support** | Arabic/Hebrew markets | 2-4 weeks | N/A |
| **Workflow editor accessibility** | Full WCAG compliance | 2-3 weeks | N/A |
| **Crowdin/Weblate** translation management | Professional translation workflow | 1 week setup | SaaS / GPLv3 |
| **error-stack** | Production debugging | 1-2 weeks migration | MIT/Apache 2.0 |

### Do Not Adopt

| Item | Why not | Alternative |
|------|---------|-------------|
| **Micro-frontends** | Team too small, adds complexity | Well-structured monolith |
| **Redux** | Overkill for ORCHA's state needs | React Query + Context + Zustand |
| **tRPC** | Requires TypeScript backend | ts-rs type generation achieves same goal |
| **Polyrepo** (before Stage 5) | Tight coupling, shared deploys | Monorepo with CODEOWNERS |
| **Component library npm package** (before Stage 3) | No external consumers yet | In-app components |

---

## Appendix: Total Effort Estimates by Stage

| Stage | Effort from this report | Calendar time (parallel) |
|-------|------------------------|--------------------------|
| 0 | 2-3 days | 1 week (alongside other Stage 0 work) |
| 1 | ~5-6 weeks | 2-3 months (spread across Stage 1 priorities) |
| 2 | ~10-14 weeks | 4-6 months (spread across Stage 2 priorities) |
| 3+ | ~16-24 weeks | Absorbed into Stage 3-4 engineering capacity |

These estimates assume the current team velocity of 3 developers + Claude Code at 90% code generation rate.

---

*This research report informs the `5-year-product-roadmap.md` and should be referenced when planning implementation sprints for each stage.*
