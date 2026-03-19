# OSS Infrastructure & Billing Tools Research

**Type:** reference
**Date:** 2026-03-19
**Context:** Evaluate permissively-licensed open source tools for ORCHA's infrastructure stack across billing, job queues, caching, search, file storage, API gateway, and messaging.
**License Policy:** Only MIT, Apache 2.0, BSD. NO copyleft (GPL/AGPL), NO source-available (SSPL/BSL). Exceptions noted explicitly.

---

## Table of Contents

1. [Billing & Payments](#1-billing--payments)
2. [Job Queues & Background Workers](#2-job-queues--background-workers)
3. [Caching](#3-caching)
4. [Search](#4-search)
5. [File Storage](#5-file-storage)
6. [API Gateway / Reverse Proxy](#6-api-gateway--reverse-proxy)
7. [Message Queue / Event Bus](#7-message-queue--event-bus)
8. [Consolidated Recommendations](#8-consolidated-recommendations)
9. [Sources](#9-sources)

---

## 1. Billing & Payments

### 1.1 async-stripe (Rust Stripe SDK)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT OR Apache 2.0 |
| **Language** | Rust |
| **Maturity** | Stable (v0.31+), v1.0.0-alpha.8 modularized |
| **Repo** | [arlyon/async-stripe](https://github.com/arlyon/async-stripe) |

**What it does:** Async (and blocking) Rust bindings for the full Stripe API. Tracks latest Stripe API version. Supports feature-gated modules to reduce compile time (e.g., enable only `billing`, `checkout`, `connect`).

**Pros:**
- Native Rust, async/await with tokio-hyper runtime — fits ORCHA's Axum stack perfectly
- Feature flags reduce binary size (only compile the Stripe APIs you use)
- MIT/Apache 2.0 — no license concerns
- Actively maintained, tracks Stripe API updates
- Auto-generated from Stripe's OpenAPI spec — comprehensive coverage

**Cons:**
- v1.0 still in alpha (modular crate split); v0.31 is stable but monolithic
- Some niche Stripe APIs may lag behind the official SDKs
- No built-in retry/backoff — must implement via tower middleware

**Roadmap Stage Fit:** Stage 1 (First Pilots) — needed as soon as billing goes live

**Budget/Timeline Impact:** Zero cost (Stripe's API fees are per-transaction). Integration effort: ~1 week for subscriptions + metered billing. Recommend starting with v0.31 stable, migrate to v1.0 when it stabilizes.

**Rationale:** This is the only real option for Stripe integration in Rust. The alternative would be raw HTTP calls to the Stripe API, which is significantly more work for no benefit.

---

### 1.2 Usage-Based Billing Patterns

**Credit-Based Abstraction Layer (Recommended for ORCHA):**

The pattern is: internal credit ledger that maps to Stripe metered billing.

```
User Action → Credit Deduction (internal DB) → Periodic Sync → Stripe Meter Event
```

**Why credits instead of raw usage:**
- Abstracts away model cost volatility (GPT-4 vs Claude vs local model)
- Simple mental model for users: "1 credit = 1 simple action, 5 credits = complex workflow"
- Pre-paid credit packs generate upfront revenue (deferred revenue accounting)
- Allows promotional credits, referral bonuses without Stripe complexity

**Implementation pattern:**
1. `credit_ledger` table: `org_id`, `balance`, `reserved`, `lifetime_used`
2. `credit_transactions` table: `org_id`, `amount`, `type` (purchase/consumption/refund/grant), `reference_id`, `created_at`
3. On action start: reserve credits (optimistic lock)
4. On action complete: deduct reserved credits, record actual cost
5. Periodic batch: aggregate consumption → send to Stripe Meter Events API

**Roadmap Stage Fit:** Stage 1-2 (credit system is the billing abstraction layer)

---

### 1.3 Billing Platforms Comparison

#### Lago — DISQUALIFIED (AGPL-3.0)

| Attribute | Detail |
|-----------|--------|
| **License** | **AGPL-3.0** |
| **Status** | **DISQUALIFIED — copyleft license** |

Lago is a strong open-source billing engine with excellent usage-based billing support, but AGPL-3.0 requires that any service using Lago must also be open-sourced under AGPL if users interact with it over a network. This is incompatible with ORCHA's proprietary/source-available licensing strategy.

**Alternative path:** Lago offers a commercial license, but this adds vendor dependency and cost.

---

#### Kill Bill (Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | Apache 2.0 |
| **Language** | Java |
| **Maturity** | 10+ years in production |
| **Repo** | [killbill/killbill](https://github.com/killbill/killbill) |

**What it does:** Full subscription billing and payments platform. Handles subscriptions, invoicing, payments, dunning, taxation, multi-tenancy, catalog management, entitlements.

**Pros:**
- Apache 2.0 — fully permissive, includes patent grant
- Battle-tested at scale for 10+ years
- Plugin architecture (payment gateways, tax engines, notifications)
- Multi-tenant by design
- Handles complex billing scenarios (proration, add-ons, trials, overages)
- Self-hostable with full control

**Cons:**
- Java/JVM stack — adds operational complexity to a Rust/Node stack (separate JVM process, memory overhead)
- Heavyweight for an early-stage product (Kill Bill is enterprise-grade complexity)
- Learning curve is steep — extensive configuration required
- REST API integration means network hop overhead from Rust backend
- Overkill until ORCHA has 100+ paying customers

**Roadmap Stage Fit:** Stage 3+ (PMF & Growth) — only if billing complexity exceeds what Stripe + credit ledger can handle

**Budget/Timeline Impact:** Free to use. Deployment: 1-2 weeks to stand up and configure. Ongoing: JVM ops overhead. Recommend deferring until Stripe's native capabilities are genuinely insufficient.

**Rationale:** Keep in reserve. Stripe handles 95% of what ORCHA needs through Stage 2. Kill Bill becomes relevant when you need custom billing logic that Stripe can't express (complex enterprise contracts, custom dunning workflows, multi-currency with custom FX rules).

---

#### OpenMeter (Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | Apache 2.0 |
| **Language** | Go |
| **Maturity** | YC W23, acquired by Kong (2026) |
| **Repo** | [openmeterio/openmeter](https://github.com/openmeterio/openmeter) |

**What it does:** Real-time usage metering and billing engine. Ingests millions of usage events, aggregates them (SUM, COUNT, AVG, MIN, MAX), applies pricing rules, generates invoices. Built on Kafka + ClickHouse internally.

**Pros:**
- Apache 2.0 — fully permissive
- Purpose-built for AI/API usage metering (their explicit use case)
- Real-time aggregation with flexible dimensions
- Usage limits and entitlements with real-time balance tracking
- Self-hostable or SaaS (OpenMeter Cloud)
- Integrates with Stripe for payment collection
- Kong acquisition means continued investment and API gateway synergies

**Cons:**
- Requires Kafka + ClickHouse for self-hosted deployment — heavy infrastructure for early stage
- Go codebase — no Rust SDK (HTTP API integration)
- Younger project (2023) — less battle-tested than Stripe Meters or Kill Bill
- Kong acquisition introduces strategic uncertainty (roadmap may shift toward Kong's priorities)
- SaaS version has usage-based pricing that adds to costs

**Roadmap Stage Fit:** Stage 2-3 (SaaS Launch & Scale) — when Stripe Meters hit rate limits or you need sub-second aggregation

**Budget/Timeline Impact:** Self-hosted: free but requires Kafka + ClickHouse ops. Cloud: usage-based pricing (contact for rates). Integration: ~1 week via REST API. Recommend Stripe Meters first, evaluate OpenMeter when hitting >100M events/month.

**Rationale:** Best open-source option for high-volume usage metering. The Kafka + ClickHouse backbone handles the scale that Stripe Meters can't (Stripe caps at 1,000 events/sec standard). But it's premature infrastructure for Stage 0-1.

---

### 1.4 Billing Recommendation Summary

| Stage | Approach | Tools |
|-------|----------|-------|
| **Stage 0-1** | Stripe-native | async-stripe + Stripe Meters + internal credit ledger table |
| **Stage 2** | Add credit system | Credit abstraction layer + Stripe Billing + usage dashboard |
| **Stage 3+** | Evaluate scale tools | OpenMeter (if >100M events/month) or Kill Bill (if enterprise billing complexity) |

---

## 2. Job Queues & Background Workers

**This is the #1 architectural gap for ORCHA.** No background worker exists. The Agent Flow Orchestration Engine, workflow triggers, cron scheduling, and billing event processing all depend on this.

### 2.1 apalis (MIT)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT |
| **Language** | Rust |
| **Maturity** | v0.7.4, actively maintained |
| **Repo** | [apalis-dev/apalis](https://github.com/geofmureithi/apalis) |

**What it does:** Type-safe, extensible background job processing library for Rust. Tower middleware compatible. Multiple storage backends: Redis, PostgreSQL, SQLite, MySQL, in-memory.

**Pros:**
- **MIT license** — no concerns
- **Rust-native** — no cross-language overhead, compiles into your binary
- **SQLite backend** (`apalis-sqlite`) — uses ORCHA's existing database, zero new infrastructure
- **Tower middleware integration** — same middleware stack as Axum (tracing, rate limiting, timeouts)
- **Axum integration** — official example shows Extension-based DI, same patterns ORCHA already uses
- **Cron support** — built-in cron scheduling via `apalis-cron`
- **Heartbeat + orphan recovery** — re-enqueues jobs from dead workers automatically
- **Async-first** — built on tokio, familiar dependency injection (similar to axum handlers)
- **Event-driven storage** — SQLite update hooks for near-instant job pickup (no polling delay)
- **Concurrency control** — configurable per-worker concurrency and parallelism

**Cons:**
- Smaller community than Redis-based alternatives (sidekiq, bull)
- v0.x — API may change (though core concepts are stable)
- SQLite backend limits horizontal scaling (single-writer constraint)
- No built-in web dashboard (apalis-board exists but is basic)
- Limited documentation compared to mature solutions

**Roadmap Stage Fit:** **Stage 0 (immediate)** — this is the P0 blocker. Agent Flow Orchestration Engine needs a background worker NOW.

**Budget/Timeline Impact:** Zero cost. Integration: ~3-5 days for basic setup with SQLite backend. The SQLite backend means zero new infrastructure — jobs are stored in the same database ORCHA already uses. Migrate to Redis/Postgres backend later if horizontal scaling is needed.

**Rationale:** **This is the recommended choice.** apalis is the best fit because:
1. Rust-native (no JVM/Python sidecar)
2. SQLite backend matches ORCHA's current database
3. Tower middleware means tracing/metrics come free
4. Cron support covers workflow trigger scheduling
5. Axum integration is first-class

---

### 2.2 SQLx-Based Job Queue (DIY)

**What it is:** Roll your own job queue using SQLx + a `jobs` table in SQLite.

**Pattern:**
```sql
CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    queue TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, dead
    attempts INTEGER DEFAULT 0,
    max_attempts INTEGER DEFAULT 3,
    scheduled_at TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

Worker loop: `SELECT ... WHERE status = 'pending' AND scheduled_at <= now() ORDER BY created_at LIMIT 1 FOR UPDATE`.

**Pros:**
- Full control, no dependencies beyond SQLx (already in stack)
- Perfectly tailored to ORCHA's needs
- No learning curve for new library APIs

**Cons:**
- Must implement: retry logic, dead letter queue, heartbeat, orphan recovery, concurrency control, graceful shutdown
- 2-4 weeks of engineering to reach production quality
- Every edge case (poison messages, at-least-once delivery, backpressure) must be handled manually
- apalis already solved these problems

**Roadmap Stage Fit:** Not recommended — apalis provides this with better quality in less time

**Rationale:** Only consider DIY if apalis has a showstopper limitation. The effort/quality tradeoff strongly favors apalis.

---

### 2.3 Redis-Backed Queues

**Tools:** `deadpool-redis` + custom queue logic, or `apalis-redis`

**Pros:**
- Redis is the industry standard for job queues (sidekiq, bull, celery all use it)
- Atomic operations, pub/sub for instant job notification
- Horizontal scaling — multiple workers across machines
- `apalis-redis` gives you apalis's type-safe API with Redis backend

**Cons:**
- **New infrastructure dependency** — ORCHA currently has no Redis
- Adds ops burden (Redis memory management, persistence config, failover)
- Overkill for current scale (single-machine deployment)

**Roadmap Stage Fit:** Stage 2-3 (when horizontal scaling is needed)

**Budget/Timeline Impact:** Redis hosting: $15-50/month (managed) or self-hosted. Migration from apalis-sqlite to apalis-redis is a backend swap, ~1 day.

**Rationale:** Defer until multi-machine deployment. apalis's backend abstraction makes this a trivial migration later.

---

### 2.4 Tokio Task Spawning (Lightweight Alternative)

**What it is:** Use `tokio::spawn` for fire-and-forget background tasks, `tokio::select!` for cancellation.

**Pros:**
- Zero dependencies, already in the stack
- Lowest latency (in-process)
- Simple for short-lived, non-critical tasks

**Cons:**
- **No persistence** — tasks lost on server restart
- No retry logic, no dead letter queue
- No scheduling (cron)
- No visibility (which tasks are running? which failed?)
- Not suitable for the orchestration engine (must survive restarts)

**Roadmap Stage Fit:** Stage 0 — useful for non-critical background tasks (cache warming, SSE notifications) alongside apalis for durable jobs

**Rationale:** Use tokio::spawn for ephemeral tasks (push SSE events, send webhooks). Use apalis for durable jobs (agent execution, workflow progression, billing events).

---

### 2.5 tokio-cron-scheduler (MIT/Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT OR Apache 2.0 |
| **Language** | Rust |
| **Maturity** | v0.15.1, actively maintained |
| **Repo** | [mvniekerk/tokio-cron-scheduler](https://github.com/mvniekerk/tokio-cron-scheduler) |

**What it does:** Schedule tasks on tokio using cron expressions, at an instant, or on a fixed interval. Optional persistence via PostgreSQL or NATS.

**Pros:**
- MIT/Apache 2.0 — no license concerns
- Natural English cron expressions (via `english-to-cron`: "every 15 seconds")
- Job lifecycle notifications (started, stopped, removed)
- Lightweight — just scheduling, no queue semantics
- Persistence options for surviving restarts

**Cons:**
- Scheduling only — no job queue, retry, or dead letter semantics
- Overlaps with `apalis-cron` which provides scheduling + queue in one package
- Notification ordering not guaranteed (tokio::spawn based)

**Roadmap Stage Fit:** Stage 0 — but only if NOT using apalis (which has cron built in)

**Budget/Timeline Impact:** Zero cost. Integration: ~1 day.

**Rationale:** If using apalis, its built-in cron support (`apalis-cron`) is preferred to avoid two scheduling systems. Use tokio-cron-scheduler only if you need standalone cron without a job queue.

---

### 2.6 Job Queue Recommendation Summary

| Component | Tool | Stage |
|-----------|------|-------|
| **Durable job queue** | apalis + apalis-sqlite | Stage 0 (immediate) |
| **Cron scheduling** | apalis-cron (built into apalis) | Stage 0 |
| **Ephemeral background tasks** | tokio::spawn | Stage 0 |
| **Horizontal scaling** | apalis-redis (backend swap) | Stage 2-3 |

---

## 3. Caching

### 3.1 Valkey / Redis (BSD-3-Clause)

| Attribute | Detail |
|-----------|--------|
| **License** | BSD-3-Clause (Valkey); Redis Ltd proprietary (Redis 7.4+) |
| **Language** | C |
| **Maturity** | Valkey forked from Redis 7.2.4 (last BSD version); now at 8.x |
| **Repo** | [valkey-io/valkey](https://github.com/valkey-io/valkey) |

**Important license note:** Redis changed to a dual SSPL/RSALv2 license in March 2024. **Redis 7.4+ is NOT permissively licensed.** Valkey is the Linux Foundation fork that continues under BSD-3-Clause. For ORCHA's license policy, **use Valkey, not Redis**.

**What it does:** In-memory data store for caching, pub/sub, streams, sorted sets. Drop-in replacement for Redis (90%+ command compatibility in 2026).

**Pros:**
- BSD-3-Clause — fully permissive
- Industry standard — massive ecosystem, client libraries, tooling
- Valkey GLIDE client (Rust core) for high-performance access
- Streams for lightweight event sourcing
- Pub/sub for real-time notifications
- Sorted sets for rate limiting, leaderboards
- Linux Foundation governance — no single-company control

**Cons:**
- **New infrastructure dependency** — ORCHA has no Redis/Valkey today
- Memory-based — costs scale with data size
- Operational overhead (persistence config, memory limits, eviction policies)
- Valkey has diverged ~10% from Redis — some Redis-specific features may not exist
- Premature for single-machine deployment

**Roadmap Stage Fit:** Stage 2 (SaaS Launch) — when multi-tenant caching, rate limiting, and session management are needed

**Budget/Timeline Impact:** Managed: $15-50/month. Self-hosted: free but ops time. Integration: ~1 week for caching layer + rate limiting.

---

### 3.2 moka (MIT/Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT OR Apache 2.0 |
| **Language** | Rust |
| **Maturity** | v0.12.10, actively maintained (March 2026) |
| **Repo** | [moka-rs/moka](https://github.com/moka-rs/moka) |

**What it does:** High-performance concurrent in-memory cache for Rust. Inspired by Java's Caffeine. Thread-safe, async-aware (tokio/async-std), with sophisticated eviction policies.

**Pros:**
- MIT/Apache 2.0 — no license concerns
- **Zero infrastructure** — in-process cache, no external services
- Async-aware (`moka::future::Cache`) — works with tokio
- Size-aware eviction (weighted entries — cache by memory, not just count)
- Variable per-entry expiration using hierarchical timer wheels
- Eviction notifications (useful for write-back caching)
- Upsert/compute methods for atomic read-modify-write
- Cache statistics (hit rate, eviction count)
- Battle-tested (used in production by multiple Rust projects)

**Cons:**
- In-process only — no distributed caching (each server instance has its own cache)
- Cache is lost on restart
- Not suitable for session storage or shared state across instances
- No persistence option

**Roadmap Stage Fit:** **Stage 0 (immediate)** — can be added today for hot-path caching

**Budget/Timeline Impact:** Zero cost. Integration: ~2-4 hours per cache site. Typical targets:
- User/org lookups (avoid DB hit on every authenticated request)
- CRM pipeline stage definitions (rarely change)
- Workflow definitions (cache after first load)
- LLM model pricing data (for credit calculations)

**Rationale:** Add moka now for in-process caching. Add Valkey later when multi-instance deployment requires distributed cache. The two are complementary — moka for L1 (process-local), Valkey for L2 (shared across instances).

---

### 3.3 Cache Invalidation Patterns for Multi-Tenant

**Pattern 1: TTL-based (simplest)**
- Set per-entry TTL (e.g., 60s for user lookups, 300s for config)
- Acceptable staleness window per data type
- No coordination needed between instances

**Pattern 2: Event-driven invalidation**
- On write to DB, publish invalidation event (via NATS/Valkey pub/sub)
- All instances evict the stale entry
- Requires event bus infrastructure

**Pattern 3: Scoped cache keys**
- Key format: `{org_id}:{entity_type}:{entity_id}`
- Bulk invalidation: clear all keys with `{org_id}:` prefix on org-level changes
- Prevents cross-tenant cache pollution

**Recommendation:** Start with TTL-based (moka) at Stage 0. Move to event-driven (moka + Valkey pub/sub) at Stage 2 when multi-instance.

---

### 3.4 Caching Recommendation Summary

| Stage | Tool | Use Case |
|-------|------|----------|
| **Stage 0** | moka | In-process L1 cache for hot paths |
| **Stage 2** | Valkey | Distributed L2 cache, rate limiting, sessions |
| **Stage 2+** | moka + Valkey | L1/L2 layered caching with pub/sub invalidation |

---

## 4. Search

### 4.1 Meilisearch (MIT)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT (Community Edition) |
| **Language** | Rust |
| **Maturity** | v1.x stable, widely adopted |
| **Repo** | [meilisearch/meilisearch](https://github.com/meilisearch/meilisearch) |

**Important license note:** Meilisearch Community Edition is MIT. The Enterprise Edition has a commercial license. For ORCHA, the Community Edition is sufficient through Stage 3.

**What it does:** Lightning-fast full-text search engine with typo tolerance, faceting, filtering, and hybrid search (semantic + keyword). REST API. Sub-50ms search-as-you-type.

**Pros:**
- MIT — fully permissive
- Written in Rust — excellent performance, low memory footprint
- Rust SDK (`meilisearch-sdk`) for native integration
- Typo tolerance, synonyms, stop words out of the box
- Hybrid search: full-text + vector embeddings (semantic search)
- RESTful HTTP API — easy to integrate
- Auto-indexing with minimal configuration
- Multi-tenant filtering via `tenant_tokens` (scoped API keys per org)
- Sub-50ms responses — instant search experience

**Cons:**
- Separate process — another service to deploy and monitor
- Not a database — requires syncing data from SQLite to Meilisearch index
- RAM-heavy for large indexes (entire index loaded in memory)
- No SQL — custom query language
- Sync latency — index updates are async (seconds, not real-time)

**Roadmap Stage Fit:** Stage 1-2 — when search becomes a user-facing feature (task search, CRM contact search, artifact discovery)

**Budget/Timeline Impact:** Self-hosted: free. Cloud: usage-based pricing. Integration: ~1 week for indexing pipeline + search API. Sync strategy: webhook on DB writes → push to Meilisearch index.

---

### 4.2 Tantivy (MIT)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT |
| **Language** | Rust |
| **Maturity** | Stable, maintained by Quickwit |
| **Repo** | [quickwit-oss/tantivy](https://github.com/quickwit-oss/tantivy) |

**What it does:** Full-text search engine library (not a server). Rust-native Lucene equivalent. You embed it in your application.

**Pros:**
- MIT — fully permissive
- **Embedded in process** — no external service, no network hop
- Rust-native — compiles into ORCHA's binary
- Extremely fast: 0.5ms P99 latency, 15-20x faster indexing than Python alternatives
- Full Lucene-class features: BM25 scoring, phrase queries, faceting, range queries
- Configurable tokenizers with stemming for 17 languages
- Disk-based indexes — doesn't require full index in RAM

**Cons:**
- Library, not server — must build your own indexing pipeline, query API, and index management
- No built-in multi-tenancy (must implement index-per-org or filtered queries)
- No REST API — must wrap with Axum handlers
- No hybrid/semantic search (pure keyword/BM25)
- More engineering effort than Meilisearch for equivalent functionality
- No auto-sync with database — manual index management

**Roadmap Stage Fit:** Stage 0-1 — for embedded search within the Rust binary (e.g., task search, workflow search)

**Budget/Timeline Impact:** Zero cost. Integration: ~2 weeks for production-quality search with indexing pipeline. More upfront work than Meilisearch but zero operational overhead.

---

### 4.3 SQLite FTS5 (Built-in)

**What it is:** SQLite's built-in full-text search extension. Already available — no new dependencies.

**Pros:**
- Zero cost, zero new infrastructure, zero new dependencies
- Already in ORCHA's stack (SQLite)
- Good enough for basic keyword search
- BM25 ranking support
- Prefix queries, phrase queries, boolean operators

**Cons:**
- No typo tolerance
- No synonym support
- No faceting
- No semantic/vector search
- Limited tokenization (no CJK, no stemming beyond basic)
- Performance degrades with large datasets (>1M rows)

**Roadmap Stage Fit:** **Stage 0 (immediate)** — add FTS5 virtual tables for task/contact search today

**Budget/Timeline Impact:** Zero cost. Integration: ~1-2 days. Add `CREATE VIRTUAL TABLE tasks_fts USING fts5(title, description, content=tasks)` and trigger-based sync.

---

### 4.4 pgvector (PostgreSQL Extension)

| Attribute | Detail |
|-----------|--------|
| **License** | PostgreSQL License (permissive, BSD-like) |
| **Maturity** | Production-ready, widely adopted in 2026 |
| **Repo** | [pgvector/pgvector](https://github.com/pgvector/pgvector) |

**What it does:** Vector similarity search for PostgreSQL. Stores embeddings as native column type, supports cosine distance, Euclidean distance, inner product. HNSW and IVFFlat indexes for fast approximate nearest neighbor search.

**Pros:**
- Permissive license (PostgreSQL License ≈ BSD)
- Hybrid search: combine vector similarity with SQL filters and full-text search
- HNSW indexes: excellent recall-performance balance
- No new infrastructure if you're already on PostgreSQL
- Mature in 2026 — powers production RAG systems at scale

**Cons:**
- **Requires PostgreSQL** — ORCHA currently uses SQLite
- Would only become relevant after database migration to PostgreSQL
- Embedding generation requires LLM API calls (adds cost and latency)
- Vector indexes consume significant memory

**Roadmap Stage Fit:** Stage 2-3 — after PostgreSQL migration (if it happens), for semantic search over artifacts, documents, CRM data

**Budget/Timeline Impact:** PostgreSQL migration is a prerequisite (see `research--database-migration.md`). pgvector itself is free. Embedding costs: ~$0.0001 per 1K tokens (OpenAI ada-002).

**Rationale:** Excellent long-term choice for semantic search. But blocked by the SQLite → PostgreSQL migration. If staying on SQLite, use Tantivy for keyword search and a separate vector DB (Qdrant — Apache 2.0) for embeddings.

---

### 4.5 Search Recommendation Summary

| Stage | Tool | Use Case |
|-------|------|----------|
| **Stage 0** | SQLite FTS5 | Basic keyword search for tasks, contacts |
| **Stage 1** | Tantivy (embedded) | Fast full-text search within Rust binary |
| **Stage 2** | Meilisearch | User-facing search-as-you-type, multi-tenant |
| **Stage 3+** | pgvector (if on PostgreSQL) | Semantic/hybrid search over documents and artifacts |

---

## 5. File Storage

### 5.1 object_store (MIT/Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT OR Apache 2.0 |
| **Language** | Rust |
| **Maturity** | Production (used by crates.io, InfluxDB IOx) |
| **Repo** | [apache/arrow-rs-object-store](https://github.com/apache/arrow-rs-object-store) |

**What it does:** Generic object store interface for uniformly interacting with AWS S3, Google Cloud Storage, Azure Blob Storage, and local filesystem. Originally from InfluxData, donated to Apache Arrow.

**Pros:**
- MIT/Apache 2.0 — fully permissive
- **Unified API** — same code works with local files, S3, GCS, Azure, and in-memory (swap backend via config)
- Production-proven at scale (crates.io, InfluxDB)
- Small dependency footprint
- Streaming uploads/downloads
- Async-first (tokio)
- Apache Arrow ecosystem — well-maintained, well-funded

**Cons:**
- Object store semantics (not filesystem) — no seek, no partial reads by default
- Higher-level operations (multipart upload management) may need custom code
- Not as feature-rich as aws-sdk-s3 for S3-specific features (lifecycle policies, bucket notifications)

**Roadmap Stage Fit:** **Stage 0-1** — adopt now for local filesystem storage, seamlessly migrate to S3 later

**Budget/Timeline Impact:** Zero cost for library. S3 costs start when deploying to cloud (~$0.023/GB/month for S3 Standard). Integration: ~3-5 days for file upload/download API.

**Rationale:** **This is the recommended choice over rust-s3.** The unified interface means ORCHA can use local filesystem in development/Stage 0 and swap to S3 in production with zero code changes — just a config change. This is the exact abstraction needed for a product evolving from local deployment to cloud SaaS.

---

### 5.2 rust-s3 (MIT/Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT OR Apache 2.0 |
| **Repo** | [durch/rust-s3](https://crates.io/crates/rust-s3) |

**What it does:** Modest interface to AWS S3 and compatible APIs (Backblaze B2, Wasabi, Minio, GCS).

**Pros:**
- Simple API for basic S3 operations (put, get, list, delete)
- Supports S3-compatible backends
- Both async and sync modes

**Cons:**
- S3-only — no local filesystem abstraction (must mock or use MinIO locally)
- Less maintained than object_store
- No streaming — loads entire objects into memory

**Roadmap Stage Fit:** Not recommended — object_store is superior

---

### 5.3 aws-sdk-s3

| Attribute | Detail |
|-----------|--------|
| **License** | Apache 2.0 |
| **Repo** | [awslabs/aws-sdk-rust](https://crates.io/crates/aws-sdk-s3) |

**What it does:** Official AWS SDK for Rust. Full S3 API coverage.

**Pros:**
- Official AWS SDK — complete S3 API coverage
- Apache 2.0
- Multipart uploads, presigned URLs, bucket notifications, lifecycle policies
- Well-maintained by AWS

**Cons:**
- AWS-specific — no local filesystem abstraction
- Heavy dependency tree
- Verbose API compared to object_store

**Roadmap Stage Fit:** Stage 2-3 — only if you need S3-specific features not in object_store (presigned URLs, bucket notifications)

---

### 5.4 File Upload in Axum

**Pattern:** Axum's `Multipart` extractor handles file uploads. Default body limit: 2MB (configurable via `DefaultBodyLimit`).

```
Client --multipart/form-data--> Axum Multipart extractor --> Stream to object_store
```

**Key decisions:**
- **Stream directly to storage** (don't buffer entire file in memory)
- **Generate UUID filenames** (prevent path traversal, collisions)
- **Store metadata in DB** (filename, content-type, size, org_id, uploaded_by)
- **Virus scanning** — defer to Stage 2 (ClamAV integration)

**Roadmap Stage Fit:** Stage 0-1

---

### 5.5 Document Versioning

**Pattern:** Object store keys include version: `{org_id}/{entity_type}/{entity_id}/v{version}/{filename}`

**Implementation:**
- `file_versions` table: `id`, `file_id`, `version`, `storage_key`, `size`, `hash`, `created_by`, `created_at`
- On upload: increment version, store new object, insert version record
- On access: serve latest version by default, allow explicit version access
- Retention policy: keep last N versions, configurable per org

**Roadmap Stage Fit:** Stage 1-2

---

### 5.6 File Storage Recommendation Summary

| Stage | Tool | Use Case |
|-------|------|----------|
| **Stage 0** | object_store (local backend) | File uploads stored on local filesystem |
| **Stage 1** | object_store (S3 backend) | Migrate to S3/R2 for cloud deployment |
| **Stage 2** | aws-sdk-s3 (if needed) | Presigned URLs, advanced S3 features |

---

## 6. API Gateway / Reverse Proxy

### 6.1 Pingora (Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | Apache 2.0 |
| **Language** | Rust |
| **Maturity** | v0.7.0, used in production at Cloudflare |
| **Repo** | [cloudflare/pingora](https://github.com/cloudflare/pingora) |

**What it does:** Rust async multithreaded framework for building HTTP proxy services. Replaced nginx at Cloudflare. HTTP/1, HTTP/2, gRPC, WebSocket proxying. Programmable routing, load balancing, TLS.

**Pros:**
- Apache 2.0 — fully permissive
- **Rust-native** — could compile into ORCHA's binary or run as sidecar
- 70% less CPU, 67% less memory than nginx in production
- Programmable in Rust — custom routing logic, not just config files
- Connection pooling, TLS, gRPC, WebSocket support
- Customizable load balancing and failover strategies
- OpenSSL and BoringSSL support (FIPS compliance)

**Cons:**
- Framework, not turnkey solution — must write Rust code for routing logic
- Smaller community than nginx/Traefik
- No built-in dashboard or management UI
- No Docker label-based auto-discovery (unlike Traefik)
- Overkill for current deployment (single-machine, nginx already configured)
- Learning curve for proxy-specific concepts

**Roadmap Stage Fit:** Stage 3-4 — when building custom API gateway with Rust-native rate limiting, auth, and routing

**Budget/Timeline Impact:** Zero cost. Integration: 2-4 weeks for custom proxy. Significant engineering investment.

**Rationale:** Exceptional technology, but premature. ORCHA already has nginx in docker-compose. Pingora becomes interesting when you need a programmable gateway with custom auth, rate limiting, and routing logic compiled into a Rust binary — likely at Stage 3+ when building the platform's API gateway layer.

---

### 6.2 Traefik (MIT)

| Attribute | Detail |
|-----------|--------|
| **License** | MIT |
| **Language** | Go |
| **Maturity** | Industry standard, widely adopted |
| **Repo** | [traefik/traefik](https://github.com/traefik/traefik) |

**What it does:** Cloud-native reverse proxy and load balancer with automatic service discovery via Docker labels, Kubernetes, Consul, etc. Built-in TLS automation (Let's Encrypt), middleware (rate limiting, auth, compression), web dashboard.

**Pros:**
- MIT — fully permissive
- **Auto-discovery** — add Docker labels, Traefik configures itself
- Built-in Let's Encrypt TLS automation
- Web dashboard for monitoring routes and services
- Rich middleware ecosystem (rate limiting, auth headers, circuit breaker, retry)
- Kubernetes-native (IngressRoute CRD)
- Canary deployments and traffic mirroring
- Metrics (Prometheus), tracing (Jaeger/Zipkin)

**Cons:**
- Go-based — separate binary, not embeddable in Rust
- More complex than nginx for simple setups
- Memory usage higher than nginx for equivalent workloads
- Configuration can be confusing (file + Docker labels + Kubernetes CRDs)
- Enterprise features (WAF, API gateway) require paid Traefik Enterprise

**Roadmap Stage Fit:** Stage 1-2 — replace nginx when deploying to Kubernetes or when needing auto-discovery, TLS automation, and richer middleware

**Budget/Timeline Impact:** Zero cost (open source). Integration: ~2-3 days to replace nginx config with Traefik labels. Significant improvement in operational ergonomics.

---

### 6.3 Kong (Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | Apache 2.0 (OSS Gateway) |
| **Language** | Lua (OpenResty/nginx) |
| **Maturity** | Industry standard API gateway |
| **Repo** | [Kong/kong](https://github.com/Kong/kong) |

**What it does:** API gateway and microservice management layer. Plugin architecture for auth, rate limiting, transformations, logging. REST, gRPC, GraphQL support. DB-less and DB modes.

**Pros:**
- Apache 2.0 — fully permissive
- Rich plugin ecosystem (auth, rate limiting, transformations, logging)
- DB-less mode for simple deployments (declarative config)
- Kubernetes-native (Kong Ingress Controller)
- gRPC and GraphQL support
- Recently acquired OpenMeter — built-in API monetization path

**Cons:**
- Lua/OpenResty stack — unfamiliar to Rust/Node teams
- Heavyweight for a single-app deployment
- Enterprise features (RBAC, workspaces, OIDC) require Kong Enterprise (paid)
- Plugin development in Lua (though Go/Python/JS PDKs exist)
- Overkill until ORCHA exposes a public API for third parties

**Roadmap Stage Fit:** Stage 3-4 — when ORCHA has a public API that needs gateway-level auth, rate limiting, and monetization

**Budget/Timeline Impact:** Zero cost (OSS). Enterprise: custom pricing. Integration: 1-2 weeks.

---

### 6.4 API Gateway Recommendation Summary

| Stage | Tool | Use Case |
|-------|------|----------|
| **Stage 0-1** | nginx (current) | Simple reverse proxy, already configured |
| **Stage 1-2** | Traefik | Replace nginx for auto-discovery, TLS, middleware |
| **Stage 3+** | Kong or Pingora | Public API gateway with rate limiting, auth, monetization |

---

## 7. Message Queue / Event Bus

### 7.1 NATS (Apache 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | Apache 2.0 |
| **Language** | Go (server), Rust client |
| **Maturity** | Production-grade, widely adopted |
| **Repo** | [nats-io/nats.rs](https://github.com/nats-io/nats.rs) |

**What it does:** Simple, secure, performant messaging system. Pub/sub, request/reply, queue groups, JetStream (persistent messaging with exactly-once delivery), key-value store, object store.

**Pros:**
- Apache 2.0 — fully permissive
- **Sub-millisecond latency** for small payloads
- Rust client (`async-nats`) actively maintained (March 2026)
- JetStream: persistent messaging with at-least-once/exactly-once delivery
- Built-in key-value store and object store (reduces need for Redis for some use cases)
- Tiny binary (~20MB), minimal resource footprint
- Simple deployment — single binary, no ZooKeeper/Raft dependencies
- Edge-friendly — runs on constrained devices
- Queue groups for load balancing across consumers

**Cons:**
- Smaller ecosystem than Kafka/RabbitMQ
- JetStream is newer — less battle-tested than Kafka for event sourcing
- Limited message transformation capabilities (no routing logic like RabbitMQ exchanges)
- Large message handling (>1MB) less efficient than Kafka/RabbitMQ
- Fewer monitoring/management tools than Kafka ecosystem

**Roadmap Stage Fit:** Stage 1-2 — when inter-service communication is needed (e.g., separate worker processes, webhook delivery, event-driven workflows)

**Budget/Timeline Impact:** Zero cost. Self-hosted: single binary, ~50MB RAM. Integration: ~3-5 days for pub/sub + JetStream. NATS is the simplest messaging system to operate.

**Rationale:** **Recommended for ORCHA.** The simplicity/performance ratio is unmatched. For an early-stage product, NATS provides pub/sub, persistent messaging, and KV store in a single 20MB binary. Kafka is overkill until processing millions of events/second.

---

### 7.2 RabbitMQ (MPL 2.0)

| Attribute | Detail |
|-----------|--------|
| **License** | MPL 2.0 |
| **Language** | Erlang |
| **Maturity** | 15+ years, industry standard |
| **Repo** | [rabbitmq/rabbitmq-server](https://github.com/rabbitmq/rabbitmq-server) |

**License compatibility note:** MPL 2.0 is a weak copyleft license — it requires MPL-licensed source files to remain available, but allows combining with proprietary code in a "larger work." It is explicitly compatible with Apache 2.0, BSD, and GPL. **For ORCHA's purposes, MPL 2.0 is acceptable** — you can use RabbitMQ as a service without open-sourcing your own code.

**Pros:**
- Mature, battle-tested, extensive documentation
- Complex routing (exchanges, bindings, routing keys)
- Dead letter queues, priority queues, delayed messages
- Management UI built-in
- Erlang's fault tolerance (let-it-crash supervision)
- Plugins for MQTT, STOMP, WebSocket

**Cons:**
- Erlang runtime — operational unfamiliarity for Rust/Node team
- Heavier resource footprint than NATS
- Complex to tune for high throughput
- No native stream processing (unlike Kafka)
- Overkill for ORCHA's current needs

**Roadmap Stage Fit:** Stage 3+ — only if complex routing patterns are needed (unlikely for ORCHA)

---

### 7.3 Redis/Valkey Streams

| Attribute | Detail |
|-----------|--------|
| **License** | BSD-3-Clause (Valkey) |

**What it does:** Append-only log structure within Redis/Valkey. Consumer groups, acknowledgements, pending entry tracking, dead letter patterns.

**Pros:**
- BSD-3-Clause (Valkey) — fully permissive
- If already running Valkey for caching, Streams adds zero infrastructure
- Consumer groups with acknowledgement
- Simpler than Kafka for basic event streaming
- Good for lightweight event sourcing patterns

**Cons:**
- Memory-bound — events consume RAM (unlike Kafka's disk-based log)
- Limited retention (must trim streams or accept memory growth)
- No built-in exactly-once delivery
- Weaker ordering guarantees than Kafka
- Requires Valkey (not in ORCHA's stack yet)

**Roadmap Stage Fit:** Stage 2 — use if Valkey is already deployed for caching; avoids adding another service

**Rationale:** Pragmatic choice if you're already running Valkey. Don't deploy Valkey just for Streams — use NATS instead.

---

### 7.4 Kafka / Redpanda

#### Apache Kafka (Apache 2.0)

Industry standard for event streaming. Apache 2.0 license. But:
- **JVM-based** — heavy operational overhead (ZooKeeper historically, KRaft now)
- **Overkill** for ORCHA's scale through Stage 3
- Recommendation: defer until processing >1M events/day consistently

#### Redpanda — DISQUALIFIED (BSL)

| Attribute | Detail |
|-----------|--------|
| **License** | **BSL (Business Source License)** |
| **Status** | **DISQUALIFIED — not permissive** |

Redpanda is Kafka API-compatible, written in C++, with impressive performance. But BSL prohibits offering it as a commercial streaming service, and the "source available" nature doesn't meet ORCHA's permissive license requirement.

BSL code converts to Apache 2.0 after 4 years, but current versions are restricted.

---

### 7.5 Message Queue Recommendation Summary

| Stage | Tool | Use Case |
|-------|------|----------|
| **Stage 0** | SSE (existing) | Real-time frontend updates |
| **Stage 1-2** | NATS | Inter-service pub/sub, event-driven workflows, webhook delivery |
| **Stage 2** | Valkey Streams | Lightweight event sourcing (if Valkey already deployed for caching) |
| **Stage 3+** | Kafka | High-volume event streaming (>1M events/day) |

---

## 8. Consolidated Recommendations

### Immediate (Stage 0 — April 2026)

| Need | Tool | License | Effort |
|------|------|---------|--------|
| Background worker (P0 BLOCKER) | **apalis + apalis-sqlite** | MIT | 3-5 days |
| Cron scheduling | **apalis-cron** | MIT | Included with apalis |
| In-process cache | **moka** | MIT/Apache 2.0 | 2-4 hours per cache site |
| Basic search | **SQLite FTS5** | Public domain | 1-2 days |
| File storage abstraction | **object_store (local)** | MIT/Apache 2.0 | 3-5 days |

**Total Stage 0 infrastructure work: ~2-3 weeks**

### Stage 1 (First Pilots — Q3 2026)

| Need | Tool | License | Effort |
|------|------|---------|--------|
| Stripe billing | **async-stripe** | MIT/Apache 2.0 | 1 week |
| Credit ledger | Custom (SQLite tables) | N/A | 1 week |
| Full-text search | **Tantivy** (embedded) | MIT | 2 weeks |
| Pub/sub messaging | **NATS** | Apache 2.0 | 3-5 days |

### Stage 2 (SaaS Launch — Q4 2026)

| Need | Tool | License | Effort |
|------|------|---------|--------|
| Distributed cache | **Valkey** | BSD-3-Clause | 1 week |
| Reverse proxy upgrade | **Traefik** | MIT | 2-3 days |
| Search-as-you-type | **Meilisearch** | MIT | 1 week |
| File storage (cloud) | **object_store (S3)** | MIT/Apache 2.0 | Config change |
| Usage metering (if needed) | **OpenMeter** | Apache 2.0 | 1 week |

### Stage 3+ (Growth)

| Need | Tool | License | Effort |
|------|------|---------|--------|
| API gateway | **Kong** or **Pingora** | Apache 2.0 | 2-4 weeks |
| Enterprise billing | **Kill Bill** | Apache 2.0 | 2 weeks |
| Semantic search | **pgvector** | PostgreSQL License | After PG migration |
| High-volume streaming | **Kafka** | Apache 2.0 | 2-4 weeks |

### License Disqualifications

| Tool | License | Issue |
|------|---------|-------|
| **Lago** | AGPL-3.0 | Copyleft — requires open-sourcing network-accessible modifications |
| **Redpanda** | BSL | Source-available, not open source — commercial streaming service restriction |
| **Redis 7.4+** | SSPL/RSALv2 | Not permissive — use Valkey (BSD-3) instead |

### Architecture Principle

**Start embedded, migrate to distributed.** Every recommendation follows this principle:
- apalis-sqlite → apalis-redis
- moka → moka + Valkey
- object_store (local) → object_store (S3)
- SQLite FTS5 → Tantivy → Meilisearch
- tokio::spawn → NATS
- nginx → Traefik → Kong/Pingora

This minimizes infrastructure complexity at each stage while providing clear migration paths as scale demands.

---

## 9. Sources

### Billing & Payments
- [async-stripe on crates.io](https://crates.io/crates/async-stripe)
- [async-stripe on GitHub](https://github.com/arlyon/async-stripe)
- [Kill Bill — Open Source Billing](https://killbill.io/)
- [Kill Bill on GitHub](https://github.com/killbill/killbill)
- [OpenMeter — Usage Metering](https://openmeter.io/)
- [OpenMeter on GitHub](https://github.com/openmeterio/openmeter)
- [Kong Acquires OpenMeter](https://konghq.com/blog/news/kong-acquires-openmeter)
- [Best Open Source Billing for AI Startups — Flexprice](https://flexprice.io/blog/best-open-source-usage-based-billing-platform-for-an-ai-startup-(2025-guide))

### Job Queues & Background Workers
- [apalis on GitHub](https://github.com/geofmureithi/apalis)
- [apalis-sqlite on GitHub](https://github.com/apalis-dev/apalis-sqlite)
- [apalis Axum Example](https://github.com/geofmureithi/apalis/blob/main/examples/axum/src/main.rs)
- [tokio-cron-scheduler on GitHub](https://github.com/mvniekerk/tokio-cron-scheduler)
- [tokio-cron-scheduler on crates.io](https://crates.io/crates/tokio-cron-scheduler)

### Caching
- [moka on GitHub](https://github.com/moka-rs/moka)
- [moka on crates.io](https://crates.io/crates/moka)
- [Valkey — Open Source In-Memory Data Store](https://valkey.io/)
- [Redis vs Valkey in 2026](https://dev.to/synsun/redis-vs-valkey-in-2026-what-the-license-fork-actually-changed-1kni)
- [Valkey vs Redis Comparison — Better Stack](https://betterstack.com/community/comparisons/redis-vs-valkey/)
- [Valkey GLIDE Client](https://aws.amazon.com/blogs/database/introducing-valkey-glide-an-open-source-client-library-for-valkey-and-redis-open-source/)

### Search
- [Meilisearch on GitHub](https://github.com/meilisearch/meilisearch)
- [Meilisearch Rust SDK](https://github.com/meilisearch/meilisearch-rust)
- [Tantivy on GitHub](https://github.com/quickwit-oss/tantivy)
- [pgvector on GitHub](https://github.com/pgvector/pgvector)
- [PostgreSQL Vector Search Guide 2026](https://calmops.com/database/postgresql/postgresql-vector-search-pgvector-complete-guide-2026/)

### File Storage
- [object_store on GitHub](https://github.com/apache/arrow-rs-object-store)
- [object_store on crates.io](https://crates.io/crates/object_store)
- [rust-s3 on crates.io](https://crates.io/crates/rust-s3)
- [aws-sdk-s3 on crates.io](https://crates.io/crates/aws-sdk-s3)
- [Axum Multipart File Upload to S3](https://medium.com/intelliconnect-engineering/uploading-files-to-aws-s3-using-axum-a-rust-framework-c96b1c774dfc)

### API Gateway / Reverse Proxy
- [Pingora on GitHub](https://github.com/cloudflare/pingora)
- [Cloudflare Open Sources Pingora](https://blog.cloudflare.com/pingora-open-source/)
- [Pingora Deep Dive — DEV Community](https://dev.to/kanywst/pingora-the-rust-proxy-that-retired-nginx-2hd1)
- [Traefik on GitHub](https://github.com/traefik/traefik)
- [Traefik vs Caddy vs nginx 2026](https://selfhostwise.com/posts/traefik-vs-caddy-vs-nginx-proxy-manager-which-reverse-proxy-should-you-choose-in-2026/)
- [Kong on GitHub](https://github.com/Kong/kong)

### Message Queue / Event Bus
- [NATS on GitHub](https://github.com/nats-io/nats.rs)
- [NATS About](https://nats.io/about/)
- [NATS vs Kafka vs Redis Streams 2026](https://www.javacodegeeks.com/2026/03/nats-vs-kafka-vs-redis-streams-for-java-microservices-when-simpler-actually-wins.html)
- [RabbitMQ on GitHub](https://github.com/rabbitmq/rabbitmq-server)
- [RabbitMQ MPL 2.0 License](https://github.com/rabbitmq/rabbitmq-server/blob/main/LICENSE-MPL-RabbitMQ)
- [Redis Streams vs Kafka vs RabbitMQ 2026](https://www.index.dev/skill-vs-skill/redis-pubsub-vs-kafka-vs-rabbitmq)
- [Event Bus with Redis Streams](https://anykeyh.hashnode.dev/designing-event-bus-using-redis-stream)
- [Redpanda Licensing](https://docs.redpanda.com/current/get-started/licensing/overview/)
