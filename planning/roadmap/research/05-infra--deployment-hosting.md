# Deployment, Hosting, and Infrastructure Research

**Date:** 2026-03-19
**Author:** Claude Research Sprint
**Status:** Reference Document
**Scope:** Infrastructure patterns for ORCHA SaaS platform (Rust/Axum + React/Vite, SQLite to PostgreSQL)

---

## Executive Summary

This document evaluates deployment, hosting, and infrastructure options across 9 categories for ORCHA's evolution from a bootstrapped dogfood tool (Stage 0, Q2 2026) through full sovereign deployment (Stage 5, 2030-2031). Recommendations are staged to match the 5-year roadmap's revenue-first philosophy: minimize spend in early stages, invest in scalability only when revenue justifies it.

**Key recommendations:**
- **Stage 0-1:** Fly.io (existing Dockerfile.fly) + Neon (serverless Postgres) + Cloudflare Pages + GitHub Actions
- **Stage 2-3:** Kubernetes (managed) + Pulumi IaC + Infisical secrets + blue/green deploys
- **Stage 4-5:** Multi-region K8s + GPU node pools + APN mesh integration + sovereign self-hosted option

**License alert:** MinIO (AGPL), HashiCorp Vault (BSL), Terraform (BSL) have non-permissive licenses. Alternatives identified for each.

---

## Table of Contents

1. [Container Orchestration](#1-container-orchestration)
2. [Edge Deployment](#2-edge-deployment)
3. [Database Hosting](#3-database-hosting)
4. [Object Storage](#4-object-storage)
5. [CDN and Static Hosting](#5-cdn-and-static-hosting)
6. [CI/CD](#6-cicd)
7. [Infrastructure as Code](#7-infrastructure-as-code)
8. [Secrets Management](#8-secrets-management)
9. [Multi-Tenant Isolation](#9-multi-tenant-isolation)
10. [Deployment Patterns](#10-deployment-patterns)
11. [Database Migration Strategies](#11-database-migration-strategies)
12. [Backup and Disaster Recovery](#12-backup-and-disaster-recovery)
13. [GPU Cost Optimization](#13-gpu-cost-optimization)
14. [Staged Recommendation Summary](#14-staged-recommendation-summary)

---

## 1. Container Orchestration

### Fly.io

**What it is:** Application platform that runs Docker containers on Firecracker micro-VMs globally. Already has `Dockerfile.fly` in the codebase.

**Pros:**
- Existing integration (Dockerfile.fly already built and optimized for Alpine/no-GPU)
- Zero-ops deployment: `fly deploy` from CI
- Built-in TLS, load balancing, health checks
- Global edge network with automatic placement
- Competitive pricing for small workloads ($1.94/mo for shared-1x-256MB)
- Supports persistent volumes (SQLite-friendly), Postgres (managed Fly Postgres)
- Auto-scale to zero possible (saves cost when idle)
- Excellent Rust support (native binary deployment)

**Cons:**
- No GPU support (rules out AI inference hosting)
- Vendor lock-in on platform primitives (Machines API, Volumes)
- Limited observability built-in (need external monitoring)
- Persistent volume limited to single region (no multi-region SQLite)
- Price increases at scale vs. self-managed
- No service mesh or advanced networking primitives

**License:** Proprietary (platform service)

**Roadmap fit:**
- **Stage 0-1: PRIMARY.** Cheapest, fastest path to production. Dockerfile.fly already exists.
- **Stage 2: SUPPLEMENT.** Run lightweight services (frontend, API gateway) while GPU workloads go elsewhere.
- **Stage 3+: PHASE OUT** in favor of Kubernetes for full control.

**Budget impact:** $10-50/mo (Stage 0-1), scaling to $100-300/mo with traffic.

---

### Kubernetes (Managed: GKE, EKS, AKS, DOKS)

**What it is:** Industry-standard container orchestration. Managed offerings remove control plane overhead.

**Pros:**
- Industry standard -- largest ecosystem, most tooling, easiest to hire for
- GPU node pools available (GKE, EKS) -- critical for AI inference
- Horizontal pod autoscaling, KEDA for event-driven scaling
- Service mesh options (Istio, Linkerd) for agent-to-agent communication
- Namespace-per-tenant isolation pattern well-documented
- Helm charts for reproducible deployments
- Multi-region/multi-cloud possible
- Perfect fit for "Docker Service-Per-Agent" architecture (Priority 36)
- DOKS (DigitalOcean) cheapest managed option at $12/mo/node

**Cons:**
- Complexity overhead for a 3-person team (Stage 0-1)
- Minimum viable cluster ~$50-100/mo even idle
- Learning curve for YAML manifests, networking, RBAC
- Debugging distributed systems harder than monolith
- Control plane costs (GKE: $73/mo standard, EKS: $73/mo, DOKS: free)

**License:** Apache 2.0 (Kubernetes itself)

**Roadmap fit:**
- **Stage 0-1: SKIP.** Overkill for 3-person team with 0-5 clients.
- **Stage 2: ADOPT.** When self-service onboarding starts, need proper orchestration.
- **Stage 3-5: PRIMARY.** Required for GPU workloads, agent isolation, multi-region.

**Budget impact:** $100-300/mo (Stage 2), $500-2000/mo (Stage 3), $5000+/mo (Stage 4-5).

**Recommended provider progression:**
1. DOKS (cheapest managed, good for Stage 2)
2. GKE (best GPU support, for Stage 3+)
3. Multi-cloud (GKE + AKS for sovereignty, Stage 5)

---

### Nomad (HashiCorp)

**What it is:** Simpler orchestrator than K8s. Supports containers, raw binaries, Java, and more.

**Pros:**
- Simpler operational model than Kubernetes
- Can run raw Rust binaries directly (no container overhead)
- Integrates with Vault and Consul (HashiCorp ecosystem)
- Lower resource overhead than K8s
- Better for mixed workloads (containers + bare-metal binaries)
- Single binary deployment

**Cons:**
- **License: BSL (Business Source License)** -- non-permissive since August 2023
- Smaller ecosystem than K8s (fewer integrations, smaller community)
- No managed offerings (must self-host control plane)
- Limited GPU scheduling compared to K8s
- Fewer CI/CD integrations
- Hiring pool smaller than K8s
- HashiCorp acquired by IBM -- future direction uncertain

**License:** BSL 1.1 (NON-PERMISSIVE)

**Roadmap fit:**
- **ALL STAGES: AVOID.** BSL license conflicts with source-available strategy. K8s ecosystem is larger. Self-hosting control plane adds ops burden for small team.

---

### Docker Swarm

**What it is:** Docker's built-in orchestration mode.

**Pros:**
- Simplest learning curve (just Docker commands)
- Built into Docker Engine (no additional install)
- docker-compose.yml files work with minor changes
- Good enough for < 10 services
- Rolling updates built-in

**Cons:**
- Effectively deprecated (Docker Inc. focuses on Docker Desktop, not Swarm)
- No auto-scaling
- Limited networking features
- No GPU scheduling
- No managed offering
- Dying community and ecosystem
- No RBAC or fine-grained access control

**License:** Apache 2.0

**Roadmap fit:**
- **Stage 0: POSSIBLE** as stepping stone from docker-compose, but Fly.io is easier.
- **Stage 1+: AVOID.** Dead-end technology, no growth path.

---

### Container Orchestration Recommendation

| Stage | Primary | Rationale |
|-------|---------|-----------|
| 0 (Dogfood) | Fly.io | Existing Dockerfile, zero-ops, cheapest |
| 1 (Pilots) | Fly.io + Docker Compose (self-hosted GPU) | Add GPU box for inference |
| 2 (SaaS Launch) | Managed K8s (DOKS) | Self-service needs proper orchestration |
| 3 (PMF) | GKE or EKS | GPU node pools, multi-region |
| 4-5 (Growth/Sovereignty) | Multi-cloud K8s + APN mesh | Full sovereignty stack |

---

## 2. Edge Deployment

### Cloudflare Workers

**What it is:** V8 isolate-based serverless platform at Cloudflare's edge (330+ PoPs).

**Pros:**
- **Rust/WASM supported** via `wasm-bindgen` and `worker-rs` crate
- Sub-millisecond cold starts (V8 isolates, not containers)
- 330+ global PoPs -- lowest latency possible
- Free tier: 100K requests/day
- Workers KV, D1 (SQLite at edge), R2 (S3-compatible storage) integrated
- Excellent for API gateway, auth validation, rate limiting, static asset serving
- Durable Objects for stateful edge compute

**Cons:**
- 128MB memory limit per worker (rules out heavy Rust services)
- 30s CPU time limit (free) / 15min (paid) -- not for long-running tasks
- Limited Rust ecosystem compatibility (no Tokio, no std::net, no filesystem)
- Axum cannot run in Workers (different runtime model)
- Cannot run the full ORCHA backend -- only lightweight proxy/middleware
- Debugging WASM harder than native

**License:** Proprietary (platform service)

**Roadmap fit:**
- **Stage 1-2: ADOPT for edge middleware.** Auth token validation, rate limiting, geo-routing, static asset caching.
- **Not suitable for:** Running the Axum backend, database operations, AI inference.

**Budget impact:** Free tier covers Stage 0-1. Paid: $5/mo + $0.50/million requests.

---

### Deno Deploy

**What it is:** Serverless platform built on Deno runtime, V8-based.

**Pros:**
- **WASM supported** (can run Rust-compiled WASM)
- Built on V8 isolates (similar to Cloudflare Workers)
- TypeScript/JavaScript first-class
- Global edge deployment (35+ regions)
- Git-based deployments

**Cons:**
- Same WASM limitations as Workers (memory, no filesystem, no Tokio)
- Smaller edge network than Cloudflare
- Less mature ecosystem
- No real advantage over Cloudflare Workers for Rust/WASM
- Cannot run Axum backend

**License:** Proprietary (platform service); Deno runtime is MIT

**Roadmap fit:**
- **ALL STAGES: SKIP.** Cloudflare Workers is superior for edge Rust/WASM. Deno Deploy adds no unique value.

---

### Fastly Compute (formerly Compute@Edge)

**What it is:** Fastly's edge compute platform, purpose-built for WASM.

**Pros:**
- **Best WASM support** of all edge platforms -- designed WASM-first
- Uses Wasmtime (Bytecode Alliance) -- most mature WASM runtime
- Sub-millisecond startup
- Rust is a first-class language (not bolted on)
- 90+ PoPs globally
- WASI support for broader system interface

**Cons:**
- Smallest free tier (limited trial only)
- Smaller developer community than Cloudflare
- Less integrated ecosystem (no equivalent of KV, D1, R2 suite)
- Same fundamental limitation: cannot run full Axum backend
- Higher pricing than Cloudflare Workers for equivalent workloads

**License:** Proprietary (platform service); Wasmtime is Apache 2.0

**Roadmap fit:**
- **ALL STAGES: SKIP** unless Cloudflare Workers proves insufficient. Cloudflare's ecosystem (R2, D1, KV) is more valuable than Fastly's WASM purity.

---

### Edge Deployment Recommendation

| Use Case | Platform | Stage |
|----------|----------|-------|
| Static asset CDN | Cloudflare Pages (see Section 5) | 0+ |
| Auth/rate-limit middleware | Cloudflare Workers | 1-2 |
| Geo-routing | Cloudflare Workers | 2-3 |
| API gateway | Cloudflare Workers | 2-3 |
| Full backend | NOT edge -- use Fly.io/K8s | All |

**Key insight:** Edge deployment for ORCHA is supplementary, not primary. The Axum backend cannot run on edge platforms due to Tokio/filesystem/database dependencies. Edge is for middleware, caching, and static assets only.

---

## 3. Database Hosting

### Neon (Serverless PostgreSQL)

**What it is:** Serverless PostgreSQL with scale-to-zero, branching, and point-in-time restore.

**Pros:**
- **Scale-to-zero:** Compute suspends when idle -- critical for Stage 0-1 cost control
- Database branching: create instant copies for testing, staging, per-PR preview
- Point-in-time restore (built-in, no additional tooling)
- Generous free tier: 0.5 GB storage, 190 compute hours/mo
- Standard PostgreSQL wire protocol -- no app code changes vs. standard Postgres
- Autoscaling: 0.25 to 8 vCPU based on load
- pg_embedding extension for vector search (useful for AI features)
- Supports pgvector for embeddings
- Read replicas for scaling reads

**Cons:**
- Cold start latency: 1-3 seconds on scale-up from zero (mitigated by keep-alive)
- Storage-compute separation means higher latency than local Postgres (~5-10ms vs ~1ms)
- Less mature than managed Postgres offerings (RDS, Cloud SQL)
- No self-hosted option (cloud-only)
- Connection pooling via their proxy (PgBouncer built-in, but adds hop)
- Not available in all regions

**License:** Apache 2.0 (Neon storage engine), Proprietary (managed service)

**Roadmap fit:**
- **Stage 0-1: PRIMARY.** Scale-to-zero = near-zero cost during dogfood. Branching enables per-PR database copies.
- **Stage 2-3: PRIMARY.** Autoscaling handles growing traffic without ops overhead.
- **Stage 4+: EVALUATE** vs. self-managed Postgres for sovereignty requirements.

**Budget impact:** Free (Stage 0), $19-69/mo (Stage 1-2), $69-300/mo (Stage 3).

---

### Supabase

**What it is:** Firebase alternative built on PostgreSQL, with auth, storage, realtime, and edge functions.

**Pros:**
- Full PostgreSQL (not a wrapper -- direct access)
- Built-in auth (could replace GitHub OAuth for self-service)
- Built-in S3-compatible storage (file uploads)
- Realtime subscriptions (PostgreSQL LISTEN/NOTIFY to WebSocket)
- Row-Level Security (RLS) as first-class feature -- ideal for multi-tenant
- Edge Functions (Deno runtime)
- Generous free tier: 500MB database, 1GB storage, 2GB bandwidth
- Self-hostable (Docker Compose)
- pgvector supported

**Cons:**
- Bundled services add complexity if you only need Postgres
- Auth/storage integration assumes Supabase client SDK -- ORCHA has its own auth
- Dashboard-oriented workflow doesn't fit Rust backend patterns
- Connection pooling via Supavisor (additional service to understand)
- Realtime relies on their WebSocket proxy -- ORCHA already has SSE
- Vendor coupling if you use multiple Supabase services

**License:** Apache 2.0 (self-hosted), Proprietary (managed service)

**Roadmap fit:**
- **Stage 0-1: POSSIBLE** but overkill. ORCHA already has auth, SSE, file handling.
- **Stage 2: EVALUATE** specifically for RLS multi-tenant patterns and self-hostable offering.
- **Stage 5: INTERESTING** for self-hosted sovereignty (Docker Compose deployment).

**Budget impact:** Free tier sufficient for Stage 0-1. Pro: $25/mo.

---

### CockroachDB

**What it is:** Distributed SQL database, PostgreSQL-compatible, with automatic sharding and multi-region.

**Pros:**
- PostgreSQL wire protocol compatible (works with SQLx)
- Automatic horizontal sharding -- no manual partitioning
- Multi-region with geo-partitioning (data residency controls)
- Survivable across AZ/region failures
- Strong consistency (serializable isolation by default)
- Serverless tier available (scale-to-zero)

**Cons:**
- Higher latency than single-node Postgres (consensus overhead, ~5-15ms writes)
- Not all PostgreSQL features supported (some extensions missing)
- Complex query performance can be worse than Postgres
- Expensive at scale (serverless: $1/M RUs, dedicated: starts at $295/mo)
- Debugging distributed query plans harder
- Overkill for < 100 orgs

**License:** BSL 1.1 (NON-PERMISSIVE) -- converts to Apache 2.0 after 3 years

**Roadmap fit:**
- **Stage 0-3: SKIP.** Overkill, expensive, BSL license.
- **Stage 4-5: EVALUATE** for multi-region sovereign deployment where geo-partitioning is critical.

---

### PlanetScale Alternatives for PostgreSQL

PlanetScale (Vitess/MySQL) has no PostgreSQL offering. PostgreSQL alternatives:

| Service | Type | Free Tier | Scale-to-Zero | Self-Host | License |
|---------|------|-----------|---------------|-----------|---------|
| **Neon** | Serverless Postgres | Yes (generous) | Yes | No | Apache 2.0 |
| **Supabase** | Postgres + BaaS | Yes | No | Yes | Apache 2.0 |
| **CockroachDB** | Distributed SQL | Yes (limited) | Yes (serverless) | Yes | BSL 1.1 |
| **Crunchy Bridge** | Managed Postgres | No | No | No | Proprietary |
| **Tembo** | Postgres platform | Yes | No | Yes | Apache 2.0 |
| **AWS RDS** | Managed Postgres | 12-mo free tier | Aurora Serverless v2 | No | Proprietary |
| **Fly Postgres** | Managed Postgres | Included with Fly | No | No | Proprietary |
| **AlloyDB** (Google) | Postgres-compatible | No | Autoscale | No | Proprietary |

---

### Database Hosting Recommendation

| Stage | Primary | Backup/DR | Rationale |
|-------|---------|-----------|-----------|
| 0 (Dogfood) | SQLite (local) + Neon (cloud) | Litestream to S3 | Near-zero cost, existing SQLite works locally |
| 1 (Pilots) | Neon | Neon PITR | Scale-to-zero, branching for testing |
| 2 (SaaS) | Neon (Pro) | Neon PITR + daily S3 dump | Autoscaling, RLS for multi-tenant |
| 3 (PMF) | Neon or Crunchy Bridge | Cross-region replica | High availability |
| 4-5 (Sovereignty) | Self-managed Postgres + Neon cloud | Multi-region replicas | Sovereignty option: customer chooses |

---

## 4. Object Storage

### MinIO

**What it is:** S3-compatible object storage, self-hosted.

**Pros:**
- Full S3 API compatibility
- Self-hostable (sovereign data)
- Excellent performance (optimized for NVMe)
- Kubernetes-native operator
- Active development, large community
- Supports erasure coding, versioning, lifecycle policies

**Cons:**
- **License: AGPL-3.0 (NON-PERMISSIVE)** -- requires releasing source of any network-connected service
- AGPL is incompatible with ORCHA's planned BSL licensing strategy
- Operating self-hosted storage adds significant ops burden
- Requires dedicated storage nodes

**License:** AGPL-3.0 (NON-PERMISSIVE) -- **DISQUALIFIED for ORCHA's licensing model**

**Roadmap fit:**
- **ALL STAGES: AVOID** due to AGPL license. Using MinIO in ORCHA's infrastructure could create license obligations that conflict with the BSL source-available strategy.

---

### S3-Compatible Alternatives (Permissive Licenses)

#### Cloudflare R2

**What it is:** S3-compatible object storage with zero egress fees.

**Pros:**
- S3 API compatible (drop-in replacement)
- **Zero egress fees** -- massive cost advantage over S3 (S3 charges $0.09/GB egress)
- Integrates with Cloudflare Workers/Pages
- Global distribution via Cloudflare network
- Free tier: 10GB storage, 10M reads, 1M writes per month
- Workers can interact with R2 at edge (transform assets, generate presigned URLs)

**Cons:**
- No self-hosted option
- Slightly higher per-operation cost than S3 for very high request volumes
- Limited lifecycle/versioning features compared to S3
- Newer service, fewer SDK integrations (but S3 SDKs work)

**License:** Proprietary (platform service)

**Budget impact:** Free (Stage 0-1), $0.015/GB/mo storage (Stage 2+). Zero egress is the killer feature.

#### SeaweedFS

**What it is:** Fast distributed storage system with S3 API compatibility.

**Pros:**
- S3 API compatible
- **License: Apache 2.0** -- permissive
- Very fast for large files (blob storage optimized)
- Supports FUSE mount, replication, erasure coding
- Kubernetes operator available
- Lower resource overhead than MinIO

**Cons:**
- Smaller community than MinIO
- Less enterprise adoption
- Fewer S3 API features (some advanced features missing)
- Documentation quality varies

**License:** Apache 2.0 (PERMISSIVE)

#### Ceph (via RADOS Gateway)

**What it is:** Enterprise distributed storage with S3 gateway.

**Pros:**
- S3 API compatible via RADOS Gateway
- **License: LGPL 2.1/3.0** -- permissive for this use case
- Battle-tested at massive scale (CERN, Red Hat)
- Block, object, and file storage in one system
- Self-hostable, no vendor lock-in

**Cons:**
- Extremely complex to operate (minimum 3 nodes, significant ops expertise needed)
- Heavy resource requirements
- Overkill for Stage 0-3
- Steep learning curve

**License:** LGPL 2.1/3.0 (PERMISSIVE for service use)

---

### Object Storage Recommendation

| Stage | Primary | Rationale |
|-------|---------|-----------|
| 0-1 | Cloudflare R2 | Zero egress, generous free tier, S3-compatible |
| 2-3 | Cloudflare R2 | Cost-effective at scale, integrates with CF Pages/Workers |
| 4-5 | R2 (cloud) + SeaweedFS (sovereign self-hosted) | Apache 2.0 license, self-hostable for sovereignty |

**Key decision:** R2's zero egress fees make it the clear winner for cloud. SeaweedFS (Apache 2.0) is the self-hosted option for Stage 5 sovereignty, avoiding MinIO's AGPL.

---

## 5. CDN and Static Hosting

### Cloudflare Pages

**What it is:** JAMstack hosting platform with global CDN, git-based deployments.

**Pros:**
- Free tier: unlimited bandwidth, 500 builds/mo
- Global CDN (330+ PoPs) -- fastest edge network
- Git-based deploy (push to GitHub to deploy)
- Preview URLs per branch/PR (critical for client review -- aligns with Priority 25)
- Custom domains with auto-SSL
- Integrates with Workers (middleware), R2 (storage), D1 (database)
- Supports SPA routing (single-page app rewrites)
- Free for unlimited sites on free plan
- Build caching and incremental deploys

**Cons:**
- Build system limitations (25MB max file size, 20K files max)
- Build environment has limited customization
- Functions (via Workers) have same limitations as Workers
- No server-side rendering without Workers

**License:** Proprietary (platform service)

**Roadmap fit:**
- **Stage 0+: PRIMARY.** Best combination of free tier, speed, and ecosystem integration.

**Budget impact:** $0 (free tier is generous enough for Stage 0-2). Pro: $20/mo if needed.

---

### Vercel

**What it is:** Frontend platform optimized for React/Next.js.

**Pros:**
- Best-in-class DX for React/Next.js
- Preview deployments per PR
- Edge Functions (V8 isolates)
- Analytics built-in
- Image optimization
- Excellent build caching

**Cons:**
- Expensive at scale ($20/user/mo Pro, bandwidth charges)
- Optimized for Next.js -- Vite/React SPA is second-class
- Bandwidth billing can spike unexpectedly
- No object storage equivalent
- Vendor lock-in on serverless functions

**License:** Proprietary (platform service)

**Budget impact:** Free (hobby), $20/user/mo (Pro) -- more expensive than CF Pages for teams.

**Roadmap fit:**
- **ALL STAGES: SKIP.** Cloudflare Pages is free with better edge network. Vercel's React DX advantage doesn't apply to a Vite SPA with a Rust backend.

---

### Netlify

**What it is:** JAMstack platform, similar to Vercel/CF Pages.

**Pros:**
- Good DX, git-based deploys
- Preview deploys per PR
- Forms, identity, and functions built-in
- Split testing (A/B) built-in

**Cons:**
- Bandwidth limits (100GB/mo free, then $55/100GB)
- Build minutes limited (300 min/mo free)
- Slower edge network than Cloudflare
- More expensive than CF Pages at every tier
- Functions less capable than Workers

**License:** Proprietary (platform service)

**Budget impact:** Free tier limited. Pro: $19/user/mo.

**Roadmap fit:**
- **ALL STAGES: SKIP.** Cloudflare Pages is strictly superior on cost and edge performance.

---

### CDN/Static Hosting Recommendation

**Cloudflare Pages for all stages.** The combination of free tier, 330+ PoP edge network, and ecosystem integration (Workers, R2, D1) makes it the clear winner. Preview URLs per PR support the client review workflow.

---

## 6. CI/CD

### GitHub Actions (Current)

**What it is:** CI/CD integrated into GitHub. ORCHA already uses it for CI and release builds.

**Current state (from infra gaps analysis):**
- `ci.yml`: Format, lint, test, typecheck (with `continue-on-error: true` -- gap)
- `build-release.yml`: Multi-platform binary builds, GitHub Releases
- Tauri desktop builds on version tags

**Optimization opportunities:**

1. **Rust build caching:** `sccache` + GitHub Actions cache. Typical Rust CI builds take 5-15 minutes; caching can reduce to 1-3 minutes.
   ```yaml
   - uses: Swatinem/rust-cache@v2  # Caches target/, registry, git deps
   ```

2. **Fix `continue-on-error: true`:** Clippy and test failures should block merges by Stage 1.

3. **Parallel jobs:** Split frontend and backend checks into parallel jobs (already partially done).

4. **Docker layer caching:** Use `docker/build-push-action` with GitHub Container Registry cache.

5. **Conditional builds:** Only rebuild backend on `crates/**` changes, frontend on `frontend/**`.

6. **E2E tests in CI:** Add Playwright tests as a required check (currently missing).

**Pros:**
- Already in use, team knows it
- Deep GitHub integration (PR checks, deployments, environments)
- 2,000 free minutes/mo (public repos unlimited)
- Large marketplace of reusable actions
- Matrix builds for multi-platform

**Cons:**
- Rust builds are slow on GitHub-hosted runners (2-core, limited cache)
- 2,000 min/mo limit tight for Rust + frontend builds
- No local runner equivalent for testing workflows
- YAML syntax can be painful for complex workflows

**License:** Proprietary (GitHub service)

**Budget impact:** Free for public repos. Private: $0.008/min (Linux). Estimated $20-80/mo at Stage 1-2 build frequency.

---

### Self-Hosted Runners

**What it is:** Run GitHub Actions workflows on your own hardware.

**Pros:**
- Unlimited minutes (no billing)
- Faster builds (use beefy hardware, NVMe cache)
- Pre-warmed Rust compilation cache (persistent between builds)
- GPU access for AI-related CI tasks
- Can use the Flox environment directly

**Cons:**
- Ops overhead (maintain the machine, update runner software)
- Security: runners have access to secrets, need hardening
- Need at least 2 runners for availability
- Ephemeral runners (recommended for security) lose cache between jobs

**Implementation:**
- Stage 1: Single self-hosted Mac Mini (M-series, ~$600 one-time) for Rust builds
- Stage 2: Dedicated Linux build server or Hetzner/OVH box (~$30-50/mo)
- Stage 3+: Auto-scaling runners via `actions-runner-controller` on K8s

**Roadmap fit:**
- **Stage 0: SKIP.** GitHub-hosted is fine for low build frequency.
- **Stage 1-2: ADOPT.** Rust builds will eat through free minutes quickly.
- **Stage 3+: SCALE.** K8s-based auto-scaling runners.

---

### Dagger.io

**What it is:** Programmable CI/CD engine. Define pipelines in code (Go, Python, TypeScript), runs anywhere.

**Pros:**
- **License: Apache 2.0** (permissive)
- Pipelines as code -- testable, composable, type-checked
- Runs locally (same pipeline local and CI -- huge DX win)
- Container-native (each step is a container)
- Caching built into the engine (content-addressable)
- Vendor-agnostic (runs on GitHub Actions, GitLab CI, locally)
- Reusable modules (Daggerverse)

**Cons:**
- Additional tool to learn and maintain
- Newer project -- ecosystem still maturing
- Adds a layer of abstraction over CI provider
- SDK overhead (need Dagger engine running)
- Community smaller than GitHub Actions

**Roadmap fit:**
- **Stage 0-1: SKIP.** GitHub Actions is sufficient and simpler.
- **Stage 2-3: EVALUATE.** When pipeline complexity grows (multi-service, E2E, GPU CI), Dagger's composability pays off.
- **Stage 4+: ADOPT.** Vendor-agnostic pipelines support multi-cloud sovereignty.

**Budget impact:** Free (open source). Engine resource costs are negligible.

---

### CI/CD Recommendation

| Stage | Primary | Enhancement | Rationale |
|-------|---------|-------------|-----------|
| 0 | GitHub Actions (fix continue-on-error) | Rust cache action | Minimal changes, fix gaps |
| 1 | GitHub Actions + self-hosted runner | Add E2E tests, Docker builds | Rust build speed, free minutes |
| 2 | GitHub Actions + self-hosted | Add staging deploys, Dagger eval | Growing pipeline complexity |
| 3+ | GitHub Actions + Dagger + K8s runners | Full multi-service pipeline | Vendor-agnostic, scalable |

---

## 7. Infrastructure as Code

### Pulumi

**What it is:** IaC using real programming languages (TypeScript, Python, Go, Rust).

**Pros:**
- **License: Apache 2.0** (permissive)
- Write IaC in TypeScript -- team already knows TypeScript
- Full programming language: loops, conditionals, functions, testing
- Type safety catches errors before deploy
- State management (Pulumi Cloud free for individuals, self-hosted backend available)
- Supports all major clouds + Kubernetes + Cloudflare
- Import existing resources (`pulumi import`)
- Policy as Code (CrossGuard) for compliance
- Automation API for building platform tools on top

**Cons:**
- Smaller community than Terraform (but growing fast)
- Some providers lag behind Terraform equivalents
- Pulumi Cloud has usage limits on free tier (200 resources)
- State management requires thought (cloud vs. self-hosted S3 backend)
- Learning curve for IaC concepts even with familiar language

**Budget impact:** Free (self-managed state) or $0 (Pulumi Cloud free tier for small projects). Team: $50/user/mo.

**Roadmap fit:**
- **Stage 1: ADOPT.** Define Fly.io, Neon, Cloudflare resources as code.
- **Stage 2+: PRIMARY.** All infrastructure managed via Pulumi.

---

### Terraform

**What it is:** Original declarative IaC tool, HCL language.

**Pros:**
- Largest ecosystem (most providers, most examples, most hiring)
- Mature and battle-tested
- Extensive documentation and community
- State management well-understood

**Cons:**
- **License: BSL 1.1 (NON-PERMISSIVE)** since August 2023
- HCL is a bespoke DSL -- team must learn new language
- Limited expressiveness (no real loops/functions until recent versions)
- No type safety (errors caught at plan time, not write time)
- HashiCorp/IBM acquisition uncertainty

**License:** BSL 1.1 (NON-PERMISSIVE)

**Roadmap fit:**
- **ALL STAGES: AVOID.** BSL license, HCL learning curve, Pulumi covers same ground with Apache 2.0 and TypeScript.

---

### OpenTofu

**What it is:** Community fork of Terraform, created after BSL relicensing.

**Pros:**
- **License: MPL 2.0** (permissive)
- Drop-in replacement for Terraform (same HCL, same providers)
- Linux Foundation backed
- Active community and development
- All Terraform providers work

**Cons:**
- Same HCL limitations as Terraform
- Smaller community than Terraform (but growing)
- Some provider features may lag if HashiCorp gates them
- Fork maintenance burden long-term
- Still HCL -- TypeScript (Pulumi) is more natural for this team

**License:** MPL 2.0 (PERMISSIVE)

**Roadmap fit:**
- **ALL STAGES: SKIP in favor of Pulumi.** OpenTofu solves the license problem but not the DX problem. Pulumi's TypeScript support and type safety are better fits for a JS/TS-familiar team.

---

### IaC Recommendation

**Pulumi (Apache 2.0) for all stages.** TypeScript IaC aligns with the team's skills, Apache 2.0 license is clean, and the programming model is more powerful than HCL.

**Implementation plan:**
- Stage 1: `pulumi/infra/` directory with Fly.io + Neon + Cloudflare stacks
- Stage 2: Add K8s provider, staging/production environments
- Stage 3+: Multi-cloud stacks, policy as code for compliance

---

## 8. Secrets Management

### HashiCorp Vault

**What it is:** Industry-standard secrets management with dynamic secrets, encryption as a service.

**Pros:**
- Most feature-rich secrets manager
- Dynamic secrets (auto-rotating database credentials)
- Transit encryption engine (encrypt/decrypt as a service)
- PKI engine (auto-generate TLS certificates)
- Audit logging
- Multi-cloud support

**Cons:**
- **License: BSL 1.1 (NON-PERMISSIVE)** since August 2023
- Complex to operate (requires HA setup, unsealing, backup)
- Significant ops overhead for small team
- HCP Vault (managed) is expensive ($0.03/secret/mo + $1.58/hr runtime)
- Overkill for Stage 0-2

**License:** BSL 1.1 (NON-PERMISSIVE)

**Roadmap fit:**
- **Stage 0-2: AVOID.** Too complex, BSL license.
- **Stage 3+: EVALUATE** if dynamic secrets or transit encryption become requirements. Consider OpenBao (fork) instead.

---

### Infisical

**What it is:** Open-source secrets management platform with SDK, CLI, and integrations.

**Pros:**
- **License: MIT** (permissive)
- Self-hostable (Docker Compose) -- sovereignty-friendly
- Clean UI for secret management
- SDKs for Rust, TypeScript, Python
- Native integrations: GitHub Actions, Kubernetes, Docker, Vercel
- Secret rotation support
- Audit logging
- Environment-based secret organization (dev/staging/prod)
- Point-in-time secret recovery
- RBAC and approval workflows

**Cons:**
- Smaller community than Vault
- Fewer advanced features (no dynamic secrets engine, no PKI)
- Self-hosted requires PostgreSQL + Redis
- Managed pricing: $6/user/mo (Team)

**License:** MIT (PERMISSIVE)

**Roadmap fit:**
- **Stage 1: ADOPT.** Replace .env files with centralized secrets. MIT license, self-hostable.
- **Stage 2+: PRIMARY.** Scale with team, add K8s integration.

**Budget impact:** Free (self-hosted) or $6/user/mo (managed). $0/mo at Stage 0-1.

---

### Doppler

**What it is:** SaaS-only secrets management platform.

**Pros:**
- Excellent DX -- CLI, SDKs, integrations
- Automatic sync to deployment targets (Fly.io, K8s, Vercel, AWS)
- Secret rotation
- Audit logging
- Free tier for small teams (5 users)

**Cons:**
- No self-hosted option (sovereignty concern)
- Proprietary -- vendor lock-in
- Free tier limited to 5 team members
- $18/user/mo (Team), $22/user/mo (Enterprise)
- Cannot inspect source code

**License:** Proprietary (SaaS only)

**Roadmap fit:**
- **Stage 0-1: POSSIBLE** as quick start. Free tier covers 3-person team.
- **Stage 2+: AVOID.** No self-hosted option conflicts with sovereignty roadmap.

---

### OpenBao

**What it is:** Community fork of Vault, created after BSL relicensing. Linux Foundation project.

**Pros:**
- **License: MPL 2.0** (permissive)
- Drop-in replacement for Vault (same API, same concepts)
- Dynamic secrets, transit encryption, PKI -- same feature set
- Linux Foundation backing
- Growing community

**Cons:**
- Very new fork (2024) -- stability still proving out
- Smaller community than Vault
- Some enterprise features may diverge
- Same operational complexity as Vault

**License:** MPL 2.0 (PERMISSIVE)

**Roadmap fit:**
- **Stage 3+: EVALUATE** as alternative to Vault when dynamic secrets become necessary.

---

### Secrets Management Recommendation

| Stage | Primary | Rationale |
|-------|---------|-----------|
| 0 | .env files (current) | Sufficient for single-developer dogfood |
| 1 | Infisical (managed or self-hosted) | Centralize secrets, MIT license |
| 2-3 | Infisical + K8s Secrets + Sealed Secrets | Multi-environment, GitOps-compatible |
| 4-5 | Infisical + OpenBao (if dynamic secrets needed) | Full sovereignty stack |

---

## 9. Multi-Tenant Isolation

### Pattern A: Namespace-Per-Tenant (Kubernetes)

**What it is:** Each tenant gets a dedicated Kubernetes namespace with resource quotas and network policies.

**Pros:**
- Strong resource isolation (CPU/memory limits per tenant)
- Network policies prevent cross-tenant traffic
- Per-tenant RBAC (K8s native)
- Independent scaling per tenant
- Easy to reason about (each namespace is a "mini-cluster")
- Natural fit for enterprise/dedicated tier

**Cons:**
- High overhead per tenant (K8s control plane resources per namespace)
- Doesn't scale well beyond ~100 tenants on single cluster
- Duplicated services per namespace (database, cache, etc.) = expensive
- Deployment complexity: N deploys instead of 1
- Migration/upgrade across all namespaces is painful
- Not viable for free/starter tier (cost per tenant too high)

**Cost model:** ~$10-30/mo per tenant minimum (K8s resources + duplicated services).

**Roadmap fit:**
- **Stage 2-3: ADOPT for Enterprise tier only.** Dedicated namespace for $199+/mo customers.
- **Stage 4-5: EXPAND.** More enterprise customers justify the pattern.

---

### Pattern B: Shared Database with Row-Level Security (RLS)

**What it is:** All tenants in same database, same tables. PostgreSQL RLS policies enforce tenant isolation at the database level.

**Pros:**
- Simplest to operate (one database, one deployment)
- Cheapest per tenant (marginal cost near zero)
- Onboarding = insert row
- Migrations ship once for all tenants
- Cross-tenant analytics/reporting possible (admin dashboards)
- PostgreSQL RLS is battle-tested (used by Supabase, Neon, etc.)
- Connection pooling efficient (one pool, not N pools)
- Natural fit for free/starter/pro tiers

**Cons:**
- RLS policy bugs can leak data (mitigated by testing, audit)
- Noisy neighbor problem (one tenant's heavy queries affect others)
- Cannot offer tenant-specific database extensions
- Harder to offer data residency (all data in same database/region)
- Performance at extreme scale (10K+ tenants in same tables) needs indexing strategy

**Implementation in PostgreSQL:**
```sql
-- Enable RLS on tables
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;

-- Policy: users can only see their org's tasks
CREATE POLICY tenant_isolation ON tasks
  USING (organization_id = current_setting('app.current_org_id')::uuid);

-- Set on each request in Axum middleware
SET app.current_org_id = '<org-uuid>';
```

**ORCHA-specific note:** The codebase already scopes most queries by `organization_id`. RLS adds database-level enforcement as a safety net.

**Roadmap fit:**
- **Stage 1-2: PRIMARY.** Cheapest, simplest, and ORCHA already scopes by org_id.
- **Stage 3+: PRIMARY for standard tiers.** Combine with namespace isolation for enterprise.

---

### Pattern C: Database-Per-Tenant

**What it is:** Each tenant gets their own database (PostgreSQL database or schema).

**Pros:**
- Strongest isolation (impossible to cross-tenant query)
- Per-tenant backup/restore
- Per-tenant performance tuning
- Data residency per tenant (different database in different region)
- Clean data export/deletion (drop database)

**Cons:**
- Connection pool explosion (N databases = N connection pools)
- Migrations across N databases is operationally painful
- Cross-tenant reporting requires federation
- Higher cost per tenant (~$5-20/mo per database)
- Not viable for free tier (too expensive per tenant)
- Schema drift risk across databases

**Roadmap fit:**
- **Stage 4-5: ADOPT for sovereignty tier.** Customers who need data residency or self-hosted get their own database.

---

### Pattern D: VM-Per-Tenant (Firecracker, Kata Containers)

**What it is:** Each tenant runs in an isolated VM or micro-VM.

**Pros:**
- Strongest possible isolation (hardware-level)
- No shared kernel (prevents container escape attacks)
- Per-tenant OS-level customization
- Compliance-friendly (HIPAA, SOC 2, etc.)

**Cons:**
- Most expensive pattern ($20-100+/mo per tenant)
- Slowest provisioning (seconds to minutes vs. milliseconds for containers)
- Highest operational complexity
- Only justified for highest-security enterprise customers
- Not available on all platforms

**Roadmap fit:**
- **Stage 5: EVALUATE** for government/compliance customers who require VM isolation.

---

### Multi-Tenant Isolation Recommendation

| Tier | Isolation Pattern | Stage |
|------|------------------|-------|
| Free / Starter | Shared DB + RLS | 1+ |
| Pro | Shared DB + RLS + resource quotas | 2+ |
| Scale / Enterprise | Shared DB + RLS + dedicated K8s namespace | 2-3+ |
| Sovereignty | Database-per-tenant + dedicated namespace | 4-5 |
| Government/Compliance | VM isolation | 5 |

**Progressive isolation:** Start with RLS (cheapest, simplest), add K8s namespace isolation for premium tiers, add database-per-tenant for sovereignty, add VM isolation for compliance.

---

## 10. Deployment Patterns

### Blue/Green Deployments

**What it is:** Run two identical production environments (blue and green). Route traffic to one while deploying to the other.

**How it works with Rust services:**
1. Blue environment running current version
2. Deploy new version to Green environment
3. Run health checks and smoke tests against Green
4. Switch load balancer/DNS to Green
5. Blue becomes standby (quick rollback by switching back)

**Implementation options:**

| Platform | Blue/Green Method | Complexity |
|----------|------------------|------------|
| Fly.io | `fly deploy --strategy bluegreen` | Low (built-in) |
| K8s | Two Deployments + Service switch | Medium |
| K8s + Argo Rollouts | Declarative blue/green with analysis | Medium-High |
| Cloudflare + Fly.io | CF Worker routes to blue or green | Low |

**Rust-specific considerations:**
- Rust binary startup is fast (< 1 second) -- quick switchover
- No JVM warmup concerns
- Compile time is the bottleneck (build in CI, deploy binary)
- SQLx compile-time query checking means type-safe DB access in both environments

**Rollback speed:** Instant (switch routing back to blue). This is the primary advantage.

**Roadmap fit:**
- **Stage 1: ADOPT** via Fly.io built-in blue/green.
- **Stage 2+: ADOPT** via K8s with Argo Rollouts.

---

### Canary Deployments

**What it is:** Route a small percentage of traffic to the new version, monitor, gradually increase.

**How it works:**
1. Deploy new version alongside current (canary pod/instance)
2. Route 5% of traffic to canary
3. Monitor error rates, latency, business metrics
4. If healthy: increase to 25% to 50% to 100%
5. If unhealthy: route 100% back to stable, investigate

**Implementation options:**

| Platform | Canary Method | Complexity |
|----------|--------------|------------|
| Fly.io | `fly deploy --strategy canary` | Low |
| K8s + Istio | VirtualService traffic splitting | High |
| K8s + Argo Rollouts | Declarative canary with AnalysisRuns | Medium |
| K8s + Flagger | Automated canary with Prometheus metrics | Medium |

**Canary analysis for Rust services:**
- Monitor: HTTP 5xx rate, P99 latency, SQLx query errors
- Rust panics surface as 500s -- canary catches before full rollout
- Memory leak detection: monitor RSS growth rate on canary
- SSE connection stability (critical for ORCHA's real-time features)

**Roadmap fit:**
- **Stage 2: EVALUATE** with Fly.io built-in canary.
- **Stage 3+: ADOPT** with Argo Rollouts + Prometheus analysis.

---

### Deployment Pattern Recommendation

| Stage | Strategy | Rationale |
|-------|----------|-----------|
| 0 | Direct deploy (single instance) | Only dogfood users, manual verification |
| 1 | Blue/green (Fly.io) | Instant rollback for pilot clients |
| 2 | Blue/green + canary (Fly.io or K8s) | SaaS reliability requirements |
| 3+ | Canary with automated analysis | Argo Rollouts + Prometheus |

---

## 11. Database Migration Strategies (Zero-Downtime)

### SQLite to PostgreSQL Migration

**ORCHA-specific context:** 215 existing migrations, SQLx with offline mode, feature-flag support for Postgres already in `crates/db`.

**Recommended approach: Phased dual-write**

1. **Phase 1: Schema setup (1 week)**
   - Generate PostgreSQL schema from SQLite migrations
   - Fix type incompatibilities (BLOB UUIDs to native UUID, INTEGER booleans to BOOLEAN, TEXT JSON to JSONB)
   - Validate all SQLx queries compile against PostgreSQL
   - Use `pgloader` for initial data migration

2. **Phase 2: Dual-write (2 weeks)**
   - Application writes to both SQLite and PostgreSQL
   - Reads from SQLite (primary)
   - Consistency checker compares both databases periodically
   - Fix any discrepancies

3. **Phase 3: Read switchover (1 week)**
   - Switch reads to PostgreSQL
   - Continue writing to both
   - Monitor query performance, error rates
   - Validate all features work against PostgreSQL

4. **Phase 4: SQLite removal (1 week)**
   - Stop writing to SQLite
   - Remove SQLite feature flag
   - Keep SQLite as cold backup for 30 days

### Zero-Downtime Schema Migrations on PostgreSQL

**Expand-Contract Pattern:**

1. **Expand:** Add new column/table (backward-compatible)
2. **Migrate:** Backfill data in batches
3. **Switch:** Application uses new schema
4. **Contract:** Remove old column/table

**Dangerous operations and safe alternatives:**

| Dangerous | Safe Alternative |
|-----------|-----------------|
| `ALTER TABLE ... ADD COLUMN ... NOT NULL` | Add nullable, backfill, add constraint |
| `ALTER TABLE ... DROP COLUMN` | Stop reading column first, then drop |
| `ALTER TABLE ... ALTER TYPE` | Add new column, dual-write, migrate, drop old |
| `CREATE INDEX` | `CREATE INDEX CONCURRENTLY` |
| `ALTER TABLE ... ADD CONSTRAINT` | `ADD CONSTRAINT ... NOT VALID`, then `VALIDATE CONSTRAINT` |

**Tooling:**
- **pgroll** (Xata, Apache 2.0): Automates expand-contract pattern
- **reshape** (Rust, MIT): Similar to pgroll, Rust-native
- **SQLx migrations:** Current tool, works but no built-in expand-contract

**Roadmap fit:**
- **Stage 1-2:** Manual expand-contract discipline with SQLx migrations
- **Stage 3+:** Adopt pgroll or reshape for automated zero-downtime migrations

---

## 12. Backup and Disaster Recovery

### Backup Strategy by Stage

#### Stage 0-1: SQLite + Neon

| Component | Method | Frequency | Retention | Tool |
|-----------|--------|-----------|-----------|------|
| SQLite (local) | Litestream to R2 | Continuous (WAL streaming) | 30 days | Litestream |
| Neon Postgres | Built-in PITR | Continuous | 7 days (free) / 30 days (pro) | Neon |
| Object storage (R2) | Cross-region replication | Continuous | 90 days | R2 built-in |
| Git repos | GitHub (already there) | Every push | Unlimited | GitHub |
| Secrets | Infisical export | Daily | 30 days | Infisical CLI |

#### Stage 2-3: PostgreSQL + K8s

| Component | Method | Frequency | Retention | Tool |
|-----------|--------|-----------|-----------|------|
| PostgreSQL | pg_dump to R2 + WAL archiving | Continuous + daily full | 90 days | pgBackRest (Apache 2.0) |
| K8s state | etcd snapshot to R2 | Hourly | 30 days | Velero (Apache 2.0) |
| Object storage | Cross-region + versioning | Continuous | 1 year | R2 |
| Application state | K8s manifests in Git | Every deploy | Unlimited | GitOps |

#### Stage 4-5: Multi-Region

| Component | Method | Frequency | Retention | Tool |
|-----------|--------|-----------|-----------|------|
| PostgreSQL | Streaming replication + cross-region standby | Continuous | 1 year | Patroni (MIT) + pgBackRest |
| K8s | Multi-cluster backup | Hourly | 90 days | Velero |
| Object storage | Multi-region replication | Continuous | 1 year+ | R2 + SeaweedFS |

### Disaster Recovery Targets

| Stage | RPO (max data loss) | RTO (recovery time) | Rationale |
|-------|---------------------|---------------------|-----------|
| 0 | 24 hours | 4 hours | Dogfood, manual recovery acceptable |
| 1 | 1 hour | 1 hour | Pilot clients expect reasonable recovery |
| 2 | 15 minutes | 30 minutes | SaaS SLA (99.5% = 3.65 hrs/mo downtime) |
| 3 | 5 minutes | 15 minutes | PMF scale, revenue depends on uptime |
| 4-5 | < 1 minute | 5 minutes | Enterprise SLA (99.9% = 43 min/mo) |

### DR Drill Schedule

- **Stage 1:** Quarterly manual DR drill (restore from backup, verify)
- **Stage 2:** Monthly automated DR test (restore to staging, run smoke tests)
- **Stage 3+:** Weekly automated DR verification + quarterly full failover drill

---

## 13. GPU Cost Optimization

### AI Inference Cost Landscape

ORCHA needs GPU for: LLM inference (via API or self-hosted), image generation, voice (Chatterbox TTS), and potentially model fine-tuning.

### Strategy: API-First, Self-Host at Scale

| Workload | Stage 0-2 | Stage 3-4 | Stage 5 |
|----------|-----------|-----------|---------|
| LLM inference | API (Anthropic, OpenAI, Google) | API + vLLM for open models | API + APN mesh for open models |
| Image generation | API (Replicate, Together) | API + self-hosted SD | APN mesh |
| Voice/TTS | Self-hosted Chatterbox (existing) | K8s GPU node pool | APN mesh |
| Fine-tuning | API (OpenAI, Anthropic) | Spot instances | Dedicated GPU cluster |

### GPU Hosting Options

#### Cloud GPU (On-Demand)

| Provider | GPU | $/hr | Spot $/hr | Best For |
|----------|-----|------|-----------|----------|
| AWS (p3.2xlarge) | V100 16GB | $3.06 | $0.92 | Established, reliable |
| GCP (a2-highgpu-1g) | A100 40GB | $3.67 | $1.10 | K8s (GKE) integration |
| Lambda Labs | A100 80GB | $1.10 | N/A | Cheapest A100 |
| RunPod | A100 80GB | $1.64 | $0.79 | Serverless GPU |
| Vast.ai | Various | $0.20-2.00 | N/A | Cheapest marketplace |
| CoreWeave | A100 80GB | $2.06 | N/A | K8s-native GPU cloud |

#### Serverless GPU

| Provider | Model | Cost | Cold Start | Best For |
|----------|-------|------|------------|----------|
| Replicate | Pay-per-prediction | $0.000225/sec (A40) | 1-5s | Low-volume inference |
| Modal | Pay-per-second | $0.000536/sec (A100) | < 1s | Batch + interactive |
| Together.ai | API pricing | ~$0.20/M tokens (Llama 3) | N/A | Open model API |
| Fireworks.ai | API pricing | ~$0.20/M tokens | N/A | Fast open model API |

### Cost Optimization Techniques

1. **Model routing (Priority 6, already planned):** Route 70% of tasks to Haiku-class ($1/MTok), 20% to Sonnet ($3/MTok), 10% to Opus ($5/MTok). Expected savings: 40-60%.

2. **Prompt caching (Priority 18, already planned):** Cache system prompts and project context. Expected savings: 50-70% on input tokens.

3. **Spot/preemptible instances:** For batch workloads (fine-tuning, bulk processing). 60-70% cheaper but can be interrupted.

4. **Right-sizing:** vLLM with quantized models (AWQ, GPTQ) on smaller GPUs. Llama-3-8B-AWQ runs on T4 (cheapest GPU, ~$0.35/hr).

5. **GPU time-sharing:** NVIDIA MPS or MIG for running multiple small models on one GPU.

6. **Batch inference:** Accumulate requests, process in batches. Higher throughput, lower per-request cost.

7. **Edge inference:** ORCHA's APN mesh (Stage 5) distributes inference across participant nodes. VIBE token settlements cover compute costs with 2x markup.

### GPU Cost Projection

| Stage | Monthly GPU Spend | Approach |
|-------|-------------------|----------|
| 0 | $0-50 | API only (Anthropic, OpenAI) |
| 1 | $50-200 | API + Replicate for image gen |
| 2 | $200-500 | API + vLLM on spot for open models |
| 3 | $500-2000 | K8s GPU node pool (spot) + API |
| 4 | $2000-8000 | Dedicated GPU nodes + API |
| 5 | $5000-15000 | Multi-source (own + APN mesh + API) |

---

## 14. Staged Recommendation Summary

### Stage 0: Dogfood (Q2 2026) -- Budget: $0-50/mo infra

| Category | Choice | Cost | License |
|----------|--------|------|---------|
| Orchestration | Fly.io (existing Dockerfile.fly) | $5-20/mo | Proprietary |
| Database | SQLite (local) + Neon free tier | $0 | Apache 2.0 |
| Object Storage | Cloudflare R2 free tier | $0 | Proprietary |
| CDN/Frontend | Cloudflare Pages free tier | $0 | Proprietary |
| CI/CD | GitHub Actions (fix continue-on-error) | $0 | Proprietary |
| IaC | Manual (Fly CLI + CF Dashboard) | $0 | N/A |
| Secrets | .env files | $0 | N/A |
| Multi-tenant | N/A (single tenant dogfood) | $0 | N/A |
| Deployment | Direct deploy to Fly.io | $0 | N/A |

### Stage 1: Pilots (Q3-Q4 2026) -- Budget: $50-200/mo infra

| Category | Choice | Cost | License |
|----------|--------|------|---------|
| Orchestration | Fly.io | $20-80/mo | Proprietary |
| Database | Neon (Launch plan) | $19/mo | Apache 2.0 |
| Object Storage | Cloudflare R2 | $5-15/mo | Proprietary |
| CDN/Frontend | Cloudflare Pages | $0 | Proprietary |
| CI/CD | GitHub Actions + self-hosted runner | $0-30/mo | Proprietary |
| IaC | Pulumi (free tier) | $0 | Apache 2.0 |
| Secrets | Infisical (self-hosted or free managed) | $0 | MIT |
| Multi-tenant | Shared DB + RLS | $0 | N/A |
| Deployment | Blue/green on Fly.io | $0 | N/A |
| Edge | Cloudflare Workers (auth/rate-limit) | $5/mo | Proprietary |

### Stage 2: SaaS Launch (Q1-Q3 2027) -- Budget: $200-800/mo infra

| Category | Choice | Cost | License |
|----------|--------|------|---------|
| Orchestration | Managed K8s (DOKS) + Fly.io (lightweight) | $100-300/mo | Apache 2.0 |
| Database | Neon (Scale plan) | $69-200/mo | Apache 2.0 |
| Object Storage | Cloudflare R2 | $15-50/mo | Proprietary |
| CDN/Frontend | Cloudflare Pages Pro | $20/mo | Proprietary |
| CI/CD | GitHub Actions + self-hosted + Dagger eval | $30-80/mo | Apache 2.0 |
| IaC | Pulumi (Team) | $50/user/mo | Apache 2.0 |
| Secrets | Infisical (Team) | $6/user/mo | MIT |
| Multi-tenant | Shared DB + RLS + K8s namespaces (enterprise) | Included | N/A |
| Deployment | Blue/green + canary | Included | N/A |
| Monitoring | Prometheus + Grafana (self-hosted on K8s) | $0 | Apache 2.0 |

### Stage 3: PMF (Q4 2027-Q2 2028) -- Budget: $800-3000/mo infra

| Category | Choice | Cost | License |
|----------|--------|------|---------|
| Orchestration | GKE/EKS (GPU node pools) | $300-1000/mo | Apache 2.0 |
| Database | Neon (Scale) or Crunchy Bridge | $200-500/mo | Apache 2.0 |
| Object Storage | R2 + cross-region replication | $50-150/mo | Proprietary |
| CI/CD | GitHub Actions + Dagger + K8s runners | $50-150/mo | Apache 2.0 |
| IaC | Pulumi | $150-250/mo | Apache 2.0 |
| Secrets | Infisical + OpenBao eval | $50-100/mo | MIT / MPL 2.0 |
| Multi-tenant | RLS + K8s namespaces + database-per-tenant (enterprise) | Included | N/A |
| Deployment | Argo Rollouts canary + analysis | $0 | Apache 2.0 |
| GPU | K8s GPU spot nodes + API | $500-2000/mo | N/A |

### Stage 4-5: Growth and Sovereignty (2028-2031) -- Budget: $5000-20000/mo infra

| Category | Choice | Cost | License |
|----------|--------|------|---------|
| Orchestration | Multi-cloud K8s + APN mesh | $2000-8000/mo | Apache 2.0 |
| Database | Self-managed Postgres (Patroni) + Neon cloud | $500-2000/mo | MIT / Apache 2.0 |
| Object Storage | R2 (cloud) + SeaweedFS (sovereign) | $200-800/mo | Apache 2.0 |
| CI/CD | Dagger + multi-cloud runners | $200-500/mo | Apache 2.0 |
| IaC | Pulumi (multi-cloud stacks) | $500-1000/mo | Apache 2.0 |
| Secrets | Infisical + OpenBao | $200-500/mo | MIT / MPL 2.0 |
| Multi-tenant | Full spectrum (RLS to namespace to DB-per-tenant to VM) | Included | N/A |
| Deployment | Multi-region canary + automated analysis | $0 | Apache 2.0 |
| GPU | Own nodes + APN mesh + API | $3000-10000/mo | N/A |

---

## License Summary

| Tool | License | Status | Alternative |
|------|---------|--------|-------------|
| Kubernetes | Apache 2.0 | SAFE | N/A |
| Pulumi | Apache 2.0 | SAFE | N/A |
| Infisical | MIT | SAFE | N/A |
| Dagger | Apache 2.0 | SAFE | N/A |
| SeaweedFS | Apache 2.0 | SAFE | N/A |
| Argo Rollouts | Apache 2.0 | SAFE | N/A |
| Velero | Apache 2.0 | SAFE | N/A |
| pgBackRest | MIT | SAFE | N/A |
| Patroni | MIT | SAFE | N/A |
| Prometheus | Apache 2.0 | SAFE | N/A |
| Grafana | AGPL-3.0 | CAUTION | Grafana Cloud (free tier) or self-host (AGPL allows for internal use) |
| OpenTofu | MPL 2.0 | SAFE | Pulumi (preferred) |
| OpenBao | MPL 2.0 | SAFE | Infisical (preferred for simplicity) |
| **MinIO** | **AGPL-3.0** | **AVOID** | SeaweedFS (Apache 2.0) or Cloudflare R2 |
| **Terraform** | **BSL 1.1** | **AVOID** | Pulumi (Apache 2.0) or OpenTofu (MPL 2.0) |
| **Vault** | **BSL 1.1** | **AVOID** | Infisical (MIT) or OpenBao (MPL 2.0) |
| **Nomad** | **BSL 1.1** | **AVOID** | Kubernetes (Apache 2.0) |

---

## Key Decision Points

### Decision 1: When to move from Fly.io to Kubernetes
**Trigger:** When you need GPU node pools, per-tenant namespace isolation, or > 5 services. Expected: Stage 2 (Q1 2027).

### Decision 2: When to migrate from SQLite to PostgreSQL
**Trigger:** When concurrent write contention degrades performance or RLS is needed for multi-tenant. Expected: Stage 1 (Q3 2026), aligned with Priority 21.

### Decision 3: When to add GPU infrastructure
**Trigger:** When API costs for AI inference exceed self-hosted cost + ops overhead. Expected crossover: ~$500-1000/mo API spend (Stage 2-3).

### Decision 4: When to add self-hosted object storage
**Trigger:** When sovereignty customers require data not to leave their infrastructure. Expected: Stage 4-5.

### Decision 5: When to adopt service mesh
**Trigger:** When service-per-agent architecture (Priority 36) is implemented and inter-service communication needs mTLS, observability, and traffic control. Expected: Stage 3-4.

---

*This research was compiled on 2026-03-19. Pricing and feature sets change frequently. Re-evaluate before each stage transition.*
