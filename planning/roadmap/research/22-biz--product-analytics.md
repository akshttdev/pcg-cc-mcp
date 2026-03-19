# Research: Product Analytics, User Behavior Tracking & Business Intelligence

**Date**: 2026-03-19
**Scope**: Tool evaluation for ORCHA AI orchestration SaaS platform
**Context**: Rust/Axum backend, React/Vite frontend, existing PostHog placeholder integration

---

## Executive Summary

ORCHA already has a PostHog integration skeleton in `crates/services/src/services/analytics.rs` (server-side event capture via HTTP API with `POSTHOG_API_KEY`/`POSTHOG_API_ENDPOINT` env vars). This research evaluates the full landscape of product analytics, session recording, feature flags, BI, customer success metrics, AI-specific analytics, and data pipelines — prioritized by roadmap stage fit and license compatibility.

**Top-level recommendation**: PostHog as the unified product analytics + session recording + feature flags platform (MIT, already partially integrated), supplemented by DuckDB for embedded analytics queries, Evidence.dev for internal BI dashboards, and custom Rust instrumentation for AI-specific metrics (agent success rates, CAPO tracking, token costs). Avoid AGPL tools (Grafana, Metabase, Plausible) in the core product to preserve licensing flexibility for source-available distribution (Stage 3+).

---

## Table of Contents

1. [Product Analytics](#1-product-analytics)
2. [Session Recording & Heatmaps](#2-session-recording--heatmaps)
3. [Feature Flags & A/B Testing](#3-feature-flags--ab-testing)
4. [Business Intelligence](#4-business-intelligence)
5. [Customer Success Metrics](#5-customer-success-metrics)
6. [AI-Specific Analytics](#6-ai-specific-analytics)
7. [Data Pipeline for Analytics](#7-data-pipeline-for-analytics)
8. [Consolidated Recommendations by Stage](#8-consolidated-recommendations-by-stage)
9. [License Summary Matrix](#9-license-summary-matrix)
10. [Sources](#10-sources)

---

## 1. Product Analytics

### PostHog (MIT) — RECOMMENDED

**Current state in ORCHA**: Server-side Rust integration exists (`analytics.rs`) using HTTP capture API. Build-time `POSTHOG_API_KEY` env var in `crates/server/build.rs`. No frontend JS SDK integration yet.

**Capabilities (2026)**:
- Product analytics: funnels, trends, retention, paths, stickiness, lifecycle
- Web analytics: pageviews, referrers, UTM tracking
- Session replay (see Section 2)
- Feature flags + experiments (see Section 3)
- Surveys (NPS/CSAT — see Section 5)
- Data warehouse connectors
- AI product assistant for debugging
- Official Rust SDK (`posthog-rs`) with async client, feature flag evaluation, group analytics

**Free tier**: 1M product analytics events/month, 5K session recordings, 1M feature flag requests, 250 survey responses — no credit card required.

**Self-hosted**: Full self-hosting available (Docker Compose). Suitable for data sovereignty requirements. ClickHouse-backed for analytics queries at scale.

**Pros**:
- MIT license — no copyleft restrictions for ORCHA's source-available plans
- All-in-one platform reduces vendor sprawl
- Official Rust SDK with async support — matches Axum/Tokio stack
- Already partially integrated (server-side capture)
- Free tier generous enough for Stage 0-1
- Self-hostable for data sovereignty (Stage 3+)

**Cons**:
- Self-hosted deployment requires ClickHouse + Kafka + PostgreSQL (operational overhead)
- Rust SDK less mature than JS/Python SDKs (feature flags require polling)
- Enterprise features (SAML, advanced permissions) require paid plan

**Roadmap stage**: Stage 0 (complete integration), expand through Stage 5
**Budget impact**: $0 through Stage 1 (free tier). Cloud pricing scales with usage — estimated $150-500/mo at Stage 2.
**License**: MIT

---

### Umami (MIT) — Web Analytics Only

**What it is**: Privacy-focused web analytics. Tracks pageviews, visitors, bounce rate, sessions, referrers, devices, countries. REST API for programmatic access.

**Pros**:
- MIT license — fully permissive
- Lightweight — Docker deploy in minutes, PostgreSQL or MySQL backend
- Clean dashboard, multi-site support, RBAC
- Cloud free tier: 1M events/month
- GDPR-friendly by default (no cookies, no personal data)

**Cons**:
- Web analytics only — no product analytics (no funnels, retention, user paths)
- No session recording, no feature flags, no A/B testing
- No Rust SDK (HTTP API only)
- Would be a parallel system alongside PostHog with significant overlap

**Roadmap stage**: Not recommended as standalone. PostHog covers this and more.
**Budget impact**: $0 self-hosted
**License**: MIT

---

### Plausible (AGPL-3.0) — LICENSE RISK

**What it is**: Privacy-focused web analytics, similar to Umami but with a stronger brand and polish.

**Pros**:
- Simple, clean UI
- GDPR/CCPA compliant without cookie consent banners
- Good for public-facing marketing sites

**Cons**:
- **AGPL-3.0 license** — if ORCHA embeds or modifies Plausible and serves it over a network, all connected server-side code may need to be released under AGPL. This directly conflicts with ORCHA's planned source-available licensing model.
- Web analytics only (same limitations as Umami)
- No Rust SDK
- Community Edition has reduced features vs cloud

**Roadmap stage**: AVOID for core product. Could use for marketing site only (Stage 1+) if completely isolated.
**Budget impact**: $9/mo cloud (10K pageviews)
**License**: AGPL-3.0 — **NOT COMPATIBLE with source-available distribution plans**

---

### Mixpanel / Amplitude — Proprietary Comparison Points

**Mixpanel**:
- Event-based pricing: 1M events/month free
- Strong funnel/retention analysis, recently added session replay and feature flags (late 2025)
- AI-powered natural language querying
- No self-hosted option
- Best for: marketing/growth teams wanting polished UI

**Amplitude**:
- MTU-based pricing: 10K MTUs free
- Deeper behavioral analysis, predictive analytics, multi-touch attribution
- Warehouse-native queries (sits on your data warehouse)
- Session replay and heatmaps
- Best for: larger organizations with dedicated analytics teams

**Why PostHog over both**: PostHog's developer-first approach matches ORCHA's engineering-led team. MIT license enables self-hosting and source-available distribution. Combined platform (analytics + flags + replay) reduces integration complexity. Free tier is more generous. The trade-off is less polish in specific areas (Amplitude's predictive analytics is deeper, Mixpanel's UI is more polished for non-technical users) — but these matter more at Stage 3+ when non-technical users are a priority.

---

## 2. Session Recording & Heatmaps

### PostHog Session Replay — RECOMMENDED

**Capabilities**: Records user sessions including mouse movements, clicks, scrolls, and DOM changes. Jump from any analytics graph directly to relevant session recordings. Network tab, console logs, and error tracking included in replay.

**Free tier**: 5,000 recordings/month — sufficient through Stage 1.

**Pros**:
- Integrated with product analytics (click on a funnel drop-off → see the sessions)
- No additional SDK — same PostHog JS snippet handles replay
- Privacy controls: mask sensitive DOM elements, exclude forms
- Self-hostable

**Cons**:
- Recording quality depends on DOM complexity (heavy canvas/WebGL won't capture well)
- Storage-intensive for self-hosted deployments
- Mobile replay is newer and less mature

**Roadmap stage**: Stage 0-1 (enable alongside analytics SDK)
**License**: MIT

---

### Microsoft Clarity — FREE, Check Terms

**What it is**: Free session recording and heatmap tool from Microsoft. No traffic limits, no paid tier. Includes click heatmaps, scroll heatmaps, rage click detection.

**Capabilities**: Session recordings, heatmaps, scroll maps, rage click detection, dead click detection, excessive scrolling detection. GDPR/CCPA/HIPAA compliant with proper consent.

**Pros**:
- Completely free, forever, no traffic limits
- Automatic PII masking (passwords, credit cards)
- ML-powered insights (identifies problematic sessions automatically)
- Very lightweight script (~10KB)

**Cons**:
- **Data shared with Microsoft** — terms require marketing consent category, not just analytics consent. This is a significant concern for B2B SaaS customers who may not want their usage data flowing to Microsoft.
- Proprietary, closed-source
- No self-hosted option
- No integration with product analytics (separate silo)
- 30-day data retention limit
- No API for programmatic access to session data

**Roadmap stage**: Stage 0 only (quick wins for UX debugging during dogfooding). Replace with PostHog replay before pilot customers.
**Budget impact**: $0
**License**: Proprietary (free) — **data sharing with Microsoft is a deal-breaker for enterprise customers**

---

### OpenReplay (Elastic License 2.0) — LICENSE NOTE

**What it is**: Self-hosted session replay with co-browsing, performance monitoring, and product analytics.

**Pros**:
- Self-hosted with full data control
- Co-browsing feature (watch user sessions live) — useful for customer support
- DevTools integration (network tab, console, performance)

**Cons**:
- **Elastic License 2.0** — not truly open source. Cannot offer OpenReplay as a managed service. Acceptable for internal use but limits future SaaS packaging.
- Separate platform from analytics (integration overhead)
- Heavier infrastructure requirements than PostHog replay

**Roadmap stage**: Not recommended. PostHog replay covers the use case with better license and integration.
**License**: Elastic License 2.0 — source-available, not OSS

---

### Privacy-Preserving Recording Patterns

Regardless of tool choice, ORCHA should implement:
1. **DOM masking**: CSS class `ph-no-capture` on sensitive elements (PostHog convention)
2. **Consent gating**: Only enable replay after explicit user opt-in (required for GDPR)
3. **Org-level toggle**: Let tenant admins disable session recording for their org
4. **Retention limits**: Auto-delete recordings after 30-90 days
5. **Network sanitization**: Strip auth tokens and API keys from recorded network requests

---

## 3. Feature Flags & A/B Testing

### PostHog Feature Flags — RECOMMENDED (Stage 0-2)

**Capabilities**: Boolean, multivariate, and JSON payload flags. Percentage-based rollouts, user/group targeting, flag integration with analytics events. Local evaluation available via Rust SDK.

**Pros**:
- Same platform as analytics — flag exposure auto-tracked in analytics
- Rust SDK supports feature flag evaluation (local + remote)
- Experiments (A/B tests) built on top of flags with statistical engine
- Free: 1M flag requests/month

**Cons**:
- Flag evaluation in Rust SDK requires periodic polling (not real-time push)
- No OpenFeature provider yet (vendor lock-in at SDK level)
- Statistical engine less sophisticated than GrowthBook's (no CUPED, no sequential testing)

**Roadmap stage**: Stage 0-2
**License**: MIT

---

### GrowthBook (MIT) — RECOMMENDED (Stage 2+ for A/B Testing)

**What it is**: Open source feature flagging and A/B testing platform. MIT-licensed core (enterprise features under separate commercial license).

**Capabilities**: Boolean/number/string/JSON feature flags. World-class experiment statistics engine with CUPED, sequential testing, Bayesian analysis, bandit algorithms, SRM checks. SDKs for React, JS, Python, Go, and more.

**Pros**:
- MIT license (core)
- Best-in-class experiment statistics engine — superior to PostHog for rigorous A/B testing
- Warehouse-native: queries your data warehouse directly (no data duplication)
- SDK for React (frontend) and REST API for Rust backend
- Self-hostable

**Cons**:
- No Rust SDK (HTTP API integration needed)
- Separate platform from analytics (requires data warehouse as bridge)
- Enterprise features (SSO, audit log, visual editor) under commercial license
- Adds infrastructure complexity

**Roadmap stage**: Stage 2+ (when A/B testing rigor matters for PMF experiments)
**Budget impact**: $0 self-hosted (MIT core). Cloud: $99/mo starter.
**License**: MIT (core), commercial (enterprise)

---

### Unleash (Apache 2.0) — DEPRECATED, AVOID

**What it is**: Feature flag management platform, formerly Apache 2.0 open source.

**Critical issue**: Open source version deprecated December 2025, end-of-life December 31, 2026. Migrated to enterprise-only model.

**Recommendation**: **AVOID** — dead-end investment. Use PostHog flags (Stage 0-1) and GrowthBook (Stage 2+).

**License**: Apache 2.0 (but EOL)

---

### OpenFeature (Apache 2.0) — ADOPT as Abstraction Layer (Stage 2+)

**What it is**: CNCF incubating project. Vendor-agnostic API specification for feature flags. Not a flag service — it's a standard interface that decouples your code from any specific flag provider.

**How it works**: Your application code calls the OpenFeature SDK. A "provider" plugin connects to your actual flag service (PostHog, GrowthBook, LaunchDarkly, etc.). Switching providers requires changing one line, not refactoring all flag evaluations.

**Pros**:
- Apache 2.0
- CNCF backing — industry standard trajectory
- Prevents vendor lock-in at the code level
- SDKs for JS/TS, Go, Java, Python, .NET, C++ (no Rust SDK yet)

**Cons**:
- No Rust SDK (would need custom provider or HTTP bridge)
- Additional abstraction layer adds complexity for small team
- Provider ecosystem still maturing

**Roadmap stage**: Stage 2+ (when multi-provider flexibility matters)
**Budget impact**: $0 (SDK only, no service)
**License**: Apache 2.0

---

## 4. Business Intelligence

### Apache Superset (Apache 2.0) — RECOMMENDED for Advanced Analytics

**What it is**: Enterprise-grade BI platform. SQL-based dashboarding, rich visualization library, granular RBAC, row-level security.

**Pros**:
- Apache 2.0 — fully compatible with ORCHA's licensing plans
- 70K+ GitHub stars, massive community
- Advanced analytics: parameterized queries, cohort analysis, data slicing
- Granular RBAC and RLS in open source (not paywalled like Metabase)
- Connects to 40+ data sources including ClickHouse, DuckDB, PostgreSQL, SQLite
- Plugin architecture for custom visualizations

**Cons**:
- Steep learning curve — requires data engineering expertise
- Heavy infrastructure (Python/Flask backend, Redis, PostgreSQL metadata store)
- Not suitable for embedding in customer-facing dashboards without significant work
- Overkill for Stage 0-1

**Roadmap stage**: Stage 2-3 (internal analytics dashboards, operational BI)
**Budget impact**: $0 self-hosted. Preset.io cloud from $500/mo.
**License**: Apache 2.0

---

### Metabase (AGPL-3.0) — LICENSE RISK

**What it is**: User-friendly BI tool. Visual query builder, clean dashboards, minimal training required.

**Pros**:
- Easiest BI tool for non-technical users
- Beautiful default visualizations
- Embedded analytics support (iframe or SDK)
- Strong community (45K+ GitHub stars)

**Cons**:
- **AGPL-3.0** — same license risk as Plausible/Grafana. If ORCHA embeds Metabase dashboards in the product and serves them to customers, AGPL obligations may apply to ORCHA's server code.
- Advanced governance (audit log, SAML) only in paid tier
- Less powerful than Superset for complex queries

**Roadmap stage**: AVOID for product embedding. Could use for internal-only dashboards if isolated.
**License**: AGPL-3.0 — **NOT COMPATIBLE with source-available distribution**

---

### Evidence.dev (MIT) — RECOMMENDED for Internal BI

**What it is**: Code-first BI framework. Write reports in SQL + Markdown, render to polished pages. Git-versioned, CI/CD deployable. "Jupyter Notebooks meets production dashboards."

**Pros**:
- MIT license
- Code-first approach matches engineering team's workflow
- Git-versioned reports — review, branch, merge analytics changes
- Connects to DuckDB, PostgreSQL, SQLite, ClickHouse, Snowflake
- Lightweight — static site output, no heavy backend
- Markdown + SQL is a natural fit for developer-written dashboards

**Cons**:
- No visual query builder (code-only)
- Not suitable for non-technical self-service BI
- Smaller community than Superset/Metabase
- No embedded analytics SDK for customer-facing use

**Roadmap stage**: Stage 0-1 (internal dogfooding metrics, CAPO dashboards)
**Budget impact**: $0 (MIT, self-hosted)
**License**: MIT

---

### Redash (BSD 2-Clause) — DECLINING, AVOID

**What it is**: SQL-based dashboarding tool. Write queries, pick visualizations, add to dashboards.

**Pros**:
- BSD license — fully permissive
- Simple and focused on SQL workflow
- 35+ data source connectors

**Cons**:
- Development has slowed significantly since Databricks acquisition
- Community fork exists but uncertain long-term viability
- No modern features (no AI, no advanced visualization types)
- No embedded analytics support

**Roadmap stage**: AVOID — declining project
**License**: BSD 2-Clause

---

### Grafana (AGPL-3.0) — LICENSE RISK, Use for Ops Only

**What it is**: Industry-standard operational dashboarding. Prometheus/Loki/Tempo integration.

**AGPL concern**: If ORCHA modifies Grafana and serves it to customers over a network, AGPL obligations trigger. Using unmodified Grafana for internal operational monitoring is generally safe, but embedding modified Grafana in the product is risky.

**Recommendation**: Use Grafana for **internal operational monitoring only** (server metrics, error rates, uptime). Do NOT embed in customer-facing product. For customer-facing dashboards, use Evidence.dev or Apache Superset.

**Roadmap stage**: Stage 1 (operational monitoring, alongside Prometheus)
**Budget impact**: $0 self-hosted
**License**: AGPL-3.0 — **safe for internal use, NOT for product embedding**

---

## 5. Customer Success Metrics

### NPS/CSAT Collection Patterns

**Recommended approach**: PostHog Surveys (included in free tier — 250 responses/month).

**In-app survey patterns for SaaS (2026 best practices)**:
- **Continuous NPS**: Survey each customer quarterly, triggered by login count or time-based
- **Event-triggered CSAT**: After key interactions — onboarding complete, first workflow run, support ticket closed
- **Milestone CSAT**: After completing first client delivery cycle, after 30/60/90 days
- **Response rate benchmarks**: In-app surveys achieve 20-35% response rates vs 10-15% for email
- **Segmentation**: By role, plan tier, lifecycle stage, org size
- **SaaS NPS benchmark**: Median 30, high-growth SaaS target 35+

**Implementation plan**:
- Stage 0: Manual NPS collection (Slack/email to PCG team)
- Stage 1: PostHog Surveys for pilot customers (in-app NPS quarterly + CSAT after onboarding)
- Stage 2: Automated survey flows triggered by usage milestones

**Budget impact**: $0 (PostHog free tier includes surveys)

---

### Churn Prediction Approaches

**Build vs Buy**: At ORCHA's stage (pre-revenue), building a lightweight custom health score is more appropriate than buying a platform like Gainsight ($50K+/year) or ChurnZero.

**Usage-based health scoring model for ORCHA**:

| Signal | Weight | Leading/Lagging | Source |
|--------|--------|-----------------|--------|
| Login frequency (7-day trend) | High | Leading | Auth events |
| Workflow executions per week | High | Leading | Execution logs |
| Agent task completion rate | Medium | Leading | Task status |
| Feature breadth (unique features used) | Medium | Leading | PostHog events |
| Team member invites | Medium | Leading | User management |
| Support ticket frequency | Low | Lagging | Support system |
| NPS score | Low | Lagging | Surveys |
| Days since last login | High | Leading | Auth events |

**Implementation plan**:
- Stage 1: Simple health score (login frequency + workflow executions + feature breadth) calculated in a daily Rust background job, stored in DB
- Stage 2: Expose health scores in admin dashboard, add automated alerts for declining scores
- Stage 3: ML-based churn prediction using historical data (DuckDB for feature engineering)

---

### Cohort Analysis Patterns

PostHog provides built-in cohort analysis (retention by signup week, feature adoption cohorts, behavioral cohorts). No additional tooling needed.

Key cohorts for ORCHA:
- By onboarding completion status
- By first workflow execution date
- By plan tier
- By primary use case (CRM vs project management vs AI agents)
- By organization size

---

## 6. AI-Specific Analytics

This is ORCHA's most differentiated analytics need. No off-the-shelf tool covers all requirements. The recommendation is a hybrid approach: custom Rust instrumentation feeding into PostHog (product-level) and DuckDB/Evidence.dev (operational-level).

### Agent Execution Success Rates

**Current state**: `token_usage.rs` model exists, task status tracking exists, but no aggregated success rate calculation or dashboard.

**Metrics to track**:
| Metric | Definition | Target (Stage 0) |
|--------|-----------|-------------------|
| First-pass success rate | Tasks completed without retry / total tasks | > 70% |
| Overall success rate | Tasks eventually completed / total tasks | > 85% |
| Mean time to completion | Wall-clock from dispatch to accepted output | < 15 min |
| Escalation rate | Tasks requiring human intervention / total | < 20% |
| Error rate by model | Failures per model tier | Varies |

**Implementation**: Emit PostHog events on agent task state transitions (`agent_task_started`, `agent_task_completed`, `agent_task_failed`, `agent_task_escalated`). Build funnels in PostHog for conversion rates. Detailed operational data in SQLite → DuckDB for deeper analysis.

---

### Token Usage and Cost Tracking (CAPO)

**Current state**: `token_usage.rs` model captures per-call token counts. No cost calculation, no aggregation dashboard.

**CAPO (Cost per Accepted Outcome)** is the primary metric:
```
CAPO = Total LLM spend on task / (1 if accepted, 0 if rejected)
Effective CAPO = Total LLM spend / Number of accepted outcomes
```

**Required instrumentation**:
1. **Per-call cost**: `tokens_in × model_input_price + tokens_out × model_output_price + cached_tokens × cache_price`
2. **Per-task cost**: Sum of all LLM calls within a task execution
3. **Per-workflow cost**: Sum of all tasks within a workflow run
4. **Per-org cost**: Sum of all executions attributed to an organization

**Model pricing table** (maintain in Rust config, update monthly):
| Model | Input $/MTok | Output $/MTok | Cache Read $/MTok |
|-------|-------------|---------------|-------------------|
| Claude Haiku | $0.25 | $1.25 | $0.03 |
| Claude Sonnet | $3.00 | $15.00 | $0.30 |
| Claude Opus | $15.00 | $75.00 | $1.50 |
| GPT-4o | $2.50 | $10.00 | N/A |
| GPT-4o-mini | $0.15 | $0.60 | N/A |

**Implementation**: Custom Rust module — `crates/services/src/services/cost_tracking.rs`. Pricing table as a TOML config. Background job aggregates daily/weekly/monthly costs per org. Expose via API for frontend dashboard (`ai-usage.tsx` page already exists).

---

### Model Performance Comparison Dashboards

**Metrics per model**:
- Success rate (task completion)
- Average tokens per task
- Average cost per task
- Average latency (time to first token, total time)
- Retry rate
- Quality score (if human review data available)

**Implementation**: Evidence.dev dashboard pulling from DuckDB analytics queries over the token_usage and task tables. Internal-only dashboard for Stage 0-1. Expose as customer-facing "AI Usage" page at Stage 2.

---

### Prompt Effectiveness Metrics

**Key metrics (2026 industry standard)**:
| Metric | What It Measures |
|--------|------------------|
| Tool Selection Quality | Did the agent pick the right tools for the task? |
| Action Completion Rate | Did the agent finish what it started? |
| Context Adherence | Did the agent stay on-topic / avoid hallucination? |
| First-pass Accept Rate | Was the output accepted without revision? |
| Prompt Injection Resistance | Did the agent resist adversarial inputs? |

**Implementation approach**: Tag each prompt template with a version ID. Track success rates per prompt version. A/B test prompt variants using PostHog feature flags (serve different prompt templates to different users/tasks, measure completion rates).

---

### Workflow Completion Funnel Analysis

**Funnel stages**:
1. Workflow created
2. Workflow triggered (manual or automated)
3. All nodes started
4. All nodes completed
5. Output accepted by user

**Drop-off analysis**: Track which node types have the highest failure rates. Identify bottleneck nodes. Measure average time per node type.

**Implementation**: PostHog custom events at each funnel stage. Rust emits events: `workflow_created`, `workflow_triggered`, `workflow_node_started`, `workflow_node_completed`, `workflow_node_failed`, `workflow_completed`, `workflow_output_accepted`.

---

## 7. Data Pipeline for Analytics

### Event Ingestion Architecture

**Stage 0-1 (Low volume, < 100K events/day)**:
- PostHog cloud for product analytics events (JS SDK frontend + Rust SDK backend)
- SQLite operational DB as source of truth for AI metrics
- No separate data pipeline needed

**Stage 2 (Medium volume, 100K-10M events/day)**:
- PostHog cloud or self-hosted for product events
- DuckDB for analytical queries over operational data (direct SQLite → DuckDB ETL)
- Evidence.dev for internal dashboards

**Stage 3+ (High volume, 10M+ events/day)**:
- ClickHouse for dedicated analytics warehouse
- Kafka or Redpanda for event streaming
- PostHog self-hosted (uses ClickHouse internally)
- Apache Superset for advanced BI

---

### Data Warehouse Options

#### DuckDB (MIT) — RECOMMENDED (Stage 0-2)

**What it is**: Embedded columnar analytical database. Runs in-process, no server required. Reads Parquet, CSV, JSON, and can attach to SQLite directly.

**Pros**:
- MIT license
- Zero infrastructure — runs embedded in Rust process or as CLI tool
- Can query SQLite databases directly (`ATTACH 'dev_assets/db.sqlite' AS ops`)
- Excellent for datasets < 10GB (ORCHA's likely range through Stage 2)
- Perfect for ETL scripts, Evidence.dev data source, ad-hoc analysis
- Rust crate available (`duckdb-rs`)

**Cons**:
- Single-machine only — no distributed queries
- Not suitable for high-concurrency real-time dashboards (no server mode for web apps)
- Performance degrades beyond single-machine memory capacity

**Roadmap stage**: Stage 0-2 (embedded analytics, Evidence.dev data source)
**Budget impact**: $0
**License**: MIT

---

#### ClickHouse (Apache 2.0) — Stage 3+

**What it is**: Distributed columnar database for real-time analytics. Handles billions of rows with sub-second queries.

**Pros**:
- Apache 2.0 license
- Massive scale (petabyte-class)
- Real-time ingestion + query
- PostHog uses ClickHouse internally — natural pairing for self-hosted PostHog
- MergeTree engine excellent for time-series analytics data

**Cons**:
- Significant operational complexity (clusters, replication, sharding)
- Overkill for < 10M rows
- Requires dedicated DevOps attention
- Memory-hungry

**Roadmap stage**: Stage 3+ (when data volume exceeds DuckDB capacity)
**Budget impact**: $0 self-hosted. ClickHouse Cloud from $197/mo.
**License**: Apache 2.0

---

### ETL from Operational DB to Analytics

**Pattern for ORCHA**:

```
SQLite (operational) → Daily Export → Parquet files → DuckDB (analytics)
                                                    → Evidence.dev (dashboards)
                                                    → PostHog (product events)
```

**Implementation**:
1. Rust background job runs daily (or on-demand)
2. Queries SQLite for new events since last export
3. Writes Parquet files to `analytics/` directory
4. DuckDB reads Parquet files for analytical queries
5. Evidence.dev connects to DuckDB for dashboard rendering

This avoids loading the operational SQLite with analytical query patterns.

---

### Real-Time vs Batch Analytics

| Use Case | Pattern | Tool |
|----------|---------|------|
| User-facing product analytics | Real-time | PostHog (cloud) |
| Internal CAPO dashboard | Near real-time (5-min) | Custom Rust → SQLite |
| Cost reports | Daily batch | DuckDB + Evidence.dev |
| Cohort analysis | Daily batch | PostHog |
| Model performance comparison | Daily batch | DuckDB + Evidence.dev |
| Churn health scores | Daily batch | Custom Rust background job |
| Workflow completion funnels | Real-time | PostHog |

---

## 8. Consolidated Recommendations by Stage

### Stage 0: Dogfood & Capture (Q2 2026)

| Tool | Action | Effort | Priority |
|------|--------|--------|----------|
| PostHog JS SDK | Add to frontend, enable autocapture | 2 hours | P1 |
| PostHog Rust SDK | Replace custom HTTP integration with official `posthog-rs` | 4 hours | P1 |
| Custom CAPO tracking | Build `cost_tracking.rs` service module | 1 week | P1 |
| Evidence.dev | Set up internal CAPO + success rate dashboard | 2 days | P2 |
| DuckDB | Add `duckdb-rs` for analytical queries | 1 day | P2 |
| Microsoft Clarity | Quick UX debugging during dogfooding | 30 min | P3 |

**Total effort**: ~2 weeks
**Total cost**: $0

### Stage 1: First Pilots & Pre-Seed (Q3-Q4 2026)

| Tool | Action | Effort | Priority |
|------|--------|--------|----------|
| PostHog Surveys | Enable NPS/CSAT for pilot customers | 1 day | P1 |
| PostHog Session Replay | Enable for pilot orgs (with consent) | 1 day | P1 |
| PostHog Feature Flags | Implement for gradual rollout to pilots | 3 days | P1 |
| Health score service | Build usage-based health scoring in Rust | 1 week | P2 |
| Grafana + Prometheus | Internal operational monitoring | 3 days | P2 |
| Remove Clarity | Replace with PostHog replay before pilots | 1 hour | P2 |
| AI usage dashboard | Customer-facing cost/usage page | 1 week | P2 |

**Total effort**: ~3 weeks
**Total cost**: $0 (PostHog free tier)

### Stage 2: SaaS Launch & Seed (Q1-Q3 2027)

| Tool | Action | Effort | Priority |
|------|--------|--------|----------|
| PostHog Cloud (paid) | Scale beyond free tier | — | P1 |
| GrowthBook | Add for rigorous A/B testing (pricing experiments) | 1 week | P2 |
| OpenFeature | Abstraction layer for flags (prevent vendor lock-in) | 3 days | P3 |
| Apache Superset | Internal advanced BI (if Evidence.dev insufficient) | 1 week | P3 |
| Customer health alerts | Automated churn risk notifications | 3 days | P2 |

**Total effort**: ~3 weeks
**Total cost**: $150-500/mo (PostHog), $0-99/mo (GrowthBook)

### Stage 3+: PMF & Growth (2028+)

| Tool | Action | Effort | Priority |
|------|--------|--------|----------|
| PostHog self-hosted | Migrate to self-hosted for data sovereignty | 1-2 weeks | P1 |
| ClickHouse | Dedicated analytics warehouse | 1 week | P2 |
| ML churn prediction | Build on historical health score data | 2-4 weeks | P3 |
| Embedded analytics | Customer-facing dashboards (Superset or custom) | 4-8 weeks | P2 |

---

## 9. License Summary Matrix

| Tool | License | Safe for Product? | Self-Hostable | Notes |
|------|---------|-------------------|---------------|-------|
| PostHog | MIT | Yes | Yes | Recommended primary platform |
| Umami | MIT | Yes | Yes | Redundant with PostHog |
| Plausible | AGPL-3.0 | **NO** | Yes | Copyleft risk |
| GrowthBook | MIT (core) | Yes | Yes | Enterprise features commercial |
| Unleash | Apache 2.0 | N/A | Yes | **EOL Dec 2026** |
| OpenFeature | Apache 2.0 | Yes | N/A (SDK) | No Rust SDK yet |
| Apache Superset | Apache 2.0 | Yes | Yes | Heavy infrastructure |
| Metabase | AGPL-3.0 | **NO** | Yes | Copyleft risk |
| Grafana | AGPL-3.0 | **Internal only** | Yes | Safe unmodified for ops |
| Redash | BSD 2-Clause | Yes | Yes | **Declining project** |
| Evidence.dev | MIT | Yes | Yes | Lightweight, code-first |
| DuckDB | MIT | Yes | Yes (embedded) | Recommended analytics DB |
| ClickHouse | Apache 2.0 | Yes | Yes | Stage 3+ scale |
| Microsoft Clarity | Proprietary | Caution | No | Data shared with Microsoft |
| OpenReplay | Elastic 2.0 | Limited | Yes | Not truly OSS |
| Mixpanel | Proprietary | N/A | No | Comparison reference |
| Amplitude | Proprietary | N/A | No | Comparison reference |

---

## 10. Sources

### Product Analytics
- [PostHog Product Analytics](https://posthog.com/product-analytics)
- [PostHog Analytics 2026 Review (Userpilot)](https://userpilot.com/blog/posthog-analytics/)
- [PostHog GitHub Repository](https://github.com/PostHog/posthog)
- [PostHog Rust SDK](https://posthog.com/docs/libraries/rust)
- [PostHog Rust SDK GitHub](https://github.com/PostHog/posthog-rs)
- [Using PostHog with Rust (Shuttle)](https://www.shuttle.dev/blog/2024/03/14/using-posthog-rust)
- [Umami GitHub Repository](https://github.com/umami-software/umami)
- [Self-Hosted Analytics 2026: Umami vs Plausible (SelfHostWise)](https://selfhostwise.com/posts/self-hosted-website-analytics-in-2026-umami-vs-plausible-complete-guide/)
- [Plausible vs Umami 2026 (Canadian Web Hosting)](https://blog.canadianwebhosting.com/plausible-vs-umami-self-hosted-analytics-2026/)
- [Plausible AGPL License Explanation](https://plausible.io/blog/open-source-licenses)
- [PostHog vs Mixpanel (PostHog)](https://posthog.com/blog/posthog-vs-mixpanel)
- [Amplitude vs Mixpanel vs PostHog (Brainforge)](https://www.brainforge.ai/resources/amplitude-vs-mixpanel-vs-posthog)
- [Best Product Analytics Tools 2026 (Vision Labs)](https://visionlabs.com/blog/best-product-analytics-tools/)

### Session Recording & Heatmaps
- [PostHog Open Source Session Replay Tools](https://posthog.com/blog/best-open-source-session-replay-tools)
- [Microsoft Clarity Terms](https://clarity.microsoft.com/terms)
- [Microsoft Clarity Comprehensive Guide 2026](https://modifyed.in/microsoft-clarity/)
- [Microsoft Clarity and GDPR (Cookie Script)](https://cookie-script.com/guides/microsoft-clarity-session-replay-gdpr)
- [Microsoft Clarity Privacy Concerns (Captain Compliance)](https://captaincompliance.com/education/microsoft-clarity-heat-mapping-user-data-and-data-privacy-concerns/)
- [OpenReplay GitHub](https://github.com/openreplay/openreplay)
- [OpenReplay Alternatives 2026 (Better Stack)](https://betterstack.com/community/comparisons/openreplay-alternatives/)

### Feature Flags & A/B Testing
- [GrowthBook Official](https://www.growthbook.io/)
- [GrowthBook GitHub](https://github.com/growthbook/growthbook)
- [GrowthBook Feature Flag Experiments Docs](https://docs.growthbook.io/feature-flag-experiments)
- [Unleash GitHub](https://github.com/Unleash/unleash)
- [Unleash Open Source EOL Notice](https://www.getunleash.io/open-source)
- [Open Source Feature Flag Tools Compared 2026 (FlagShark)](https://flagshark.com/blog/open-source-feature-flag-tools-compared-2026/)
- [OpenFeature Official](https://openfeature.dev/)
- [OpenFeature Introduction](https://openfeature.dev/docs/reference/intro/)
- [OpenFeature Guide (SigNoz)](https://signoz.io/blog/openfeature/)

### Business Intelligence
- [Apache Superset vs Metabase 2026 (Bix Tech)](https://bix-tech.com/apache-superset-vs-metabase-the-nononsense-guide-to-choosing-the-right-opensource-bi-platform-in-2026/)
- [Metabase vs Superset 2026 (Insia)](https://www.insia.ai/blog-posts/metabase-vs-superset)
- [Evidence.dev Official](https://evidence.dev/)
- [Evidence.dev GitHub](https://github.com/evidence-dev/evidence)
- [Evidence.dev Docs](https://docs.evidence.dev/)
- [Redash GitHub](https://github.com/getredash/redash)
- [Grafana AGPL Licensing](https://grafana.com/licensing/)
- [Grafana AGPL Implications (Community Forum)](https://community.grafana.com/t/agpl-implications/47776)
- [Grafana License Change (OpenLogic)](https://www.openlogic.com/blog/grafana-license-change)

### Data Pipeline & Warehousing
- [DuckDB vs ClickHouse 2026 (Tasrie IT)](https://tasrieit.com/blog/clickhouse-vs-duckdb-2026)
- [DuckDB vs ClickHouse: Embedded vs Distributed (DB Pro)](https://www.dbpro.app/blog/duckdb-vs-clickhouse)
- [ClickHouse vs DuckDB vs Snowflake 2026 (Sfotex)](https://sfotex.com/blog/clickhouse-vs-duckdb-vs-snowflake/)
- [ClickHouse vs DuckDB Nodes (Tinybird)](https://www.tinybird.co/blog/clickhouse-vs-duckdb-nodes)

### Customer Success
- [NPS Tools for SaaS 2026 (Zonka)](https://www.zonkafeedback.com/blog/nps-tools-for-saas)
- [SaaS NPS Benchmarks 2026 (SurveySparrow)](https://surveysparrow.com/blog/saas-nps-benchmarks-2026/)
- [Churn Prediction Software 2026 (Pecan)](https://www.pecan.ai/blog/customer-churn-prediction-software/)
- [Customer Health Scoring (Cerebral Ops)](https://blog.cerebralops.in/customer-health-scoring-predicting-churn-before-it-happens/)
- [CSM Tools 2026 (Pylon)](https://www.usepylon.com/blog/csm-tools-2026)

### AI-Specific Analytics
- [Evaluating Agentic AI Pipelines 2026 (Stack AI)](https://www.stackai.com/blog/how-to-evaluate-agentic-ai-pipelines-metrics-frameworks-and-real-world-examples)
- [AI Agent Evaluation Metrics 2026 (Master of Code)](https://masterofcode.com/blog/ai-agent-evaluation)
- [Top 5 Agent Evaluation Tools 2026 (Maxim)](https://www.getmaxim.ai/articles/top-5-tools-for-agent-evaluation-in-2026/)
- [AI Agent Success Metrics (MindStudio)](https://www.mindstudio.ai/blog/ai-agent-success-metrics)
- [AI Agent Performance Measurement (Microsoft)](https://www.microsoft.com/en-us/dynamics-365/blog/it-professional/2026/02/04/ai-agent-performance-measurement/)
- [AI Agent Observability Tools 2026 (AI Multiple)](https://research.aimultiple.com/agentic-monitoring/)
