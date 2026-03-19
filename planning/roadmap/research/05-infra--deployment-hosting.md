# Deployment & Hosting Platform Research

**Date**: 2026-03-19
**Scope**: Deployment, hosting, and infrastructure patterns for ORCHA (Rust/Axum + React/Vite)

---

## 1. Fly.io (Current Setup)

**Current config**: `fly.toml` deploys to `orcha-staging` in `iad` (Ashburn) with shared-cpu-1x / 1GB RAM VM + persistent volume for SQLite.

**Pricing**:
- Shared-cpu-1x / 256MB: ~$2.02/mo; current 1GB config: ~$5-7/mo
- Performance-1x / 2GB: ~$32/mo
- Volumes: $0.15/GB/mo
- Dedicated IPv4: $2/mo
- Egress: $0.02/GB (NA/EU), $0.04/GB (APAC)
- Machine reservations: 40% discount

**Managed Postgres**: $38/mo (Basic, shared-2x/1GB) up to $1,922/mo (Performance-8x/64GB). Storage: $0.28/GB/mo. Available in 12 regions. Includes auto-backups, failover, pgvector, PostGIS.

**LiteFS (SQLite replication)**: Pre-1.0, explicitly unsupported. Fly.io warns: "We are not able to provide support or guidance for this product." Incompatible with auto-stop/auto-start — risks data loss from stale lease acquisition.

**GPU support**: DEPRECATED. GPUs unavailable after August 1, 2025. No GPU path on Fly.io.

**Rust build times**: `Dockerfile.fly` already limits to `CARGO_BUILD_JOBS=1` with 16 codegen units. Expect 10-20min builds on shared builders.

### Pros
- Already configured and working for staging
- Simple deployment model (single binary + volume)
- Good managed Postgres offering
- Global edge network

### Cons
- No GPU support (deprecated)
- LiteFS abandoned — can't do multi-region SQLite
- Build times slow on shared builders
- Gets expensive at scale vs self-hosted

### Roadmap Stage: Stage 0-1 (Dogfood/Pilots)
### Budget Impact: $7-70/mo depending on tier
### Rationale: Keep for staging. Not viable for production GPU workloads.

---

## 2. Cloudflare Ecosystem

### Workers (Edge Compute)
- Free: 100K requests/day, 10ms CPU/invocation
- Paid ($5/mo): 10M requests/mo, $0.30/million after. 30s CPU default, up to 5min max
- **Not suitable for long-running Axum server** — request-response only

### R2 (Object Storage) — Best for sovereign media/backup
- Storage: $0.015/GB/mo
- **Egress: FREE** (killer feature vs S3)
- Free tier: 10GB storage, 1M writes, 10M reads/mo
- Class A (writes): $4.50/million; Class B (reads): $0.36/million

### D1 (Edge SQLite)
- Free: 5GB storage, 5M reads/day, 100K writes/day
- Paid: $0.75/GB/mo storage
- Designed for Workers runtime, not traditional Axum backend

### Tunnels
- **FREE** on all plans
- Runs `cloudflared` daemon creating outbound connections to CF edge
- No inbound ports needed — perfect for exposing Hetzner/self-hosted

### Pages
- Free for static sites. Ideal for React frontend build output.

### Pros
- R2 free egress is unbeatable for media serving
- Tunnels eliminate need for static IP/exposed ports
- Pages is excellent free CDN for frontend
- Global edge network

### Cons
- Workers/D1 not compatible with Axum architecture
- Vendor lock-in on proprietary services
- D1 still maturing

### Roadmap Stage: Stage 0+ (R2/Tunnels/Pages immediately useful)
### Budget Impact: $0-5/mo (free tiers cover early usage)
### Rationale: R2 for object storage, Tunnels for secure exposure, Pages for frontend CDN.

---

## 3. Railway

**Pricing**:
- Hobby: $5/mo (includes $5 credits); up to 48 vCPU / 48GB RAM, 5GB storage
- Pro: $20/mo (includes $20 credits); up to 1000 vCPU / 1TB RAM, 1TB storage
- Usage: CPU $0.000463/vCPU-min, Memory $0.000231/GB-min, Egress $0.05/GB

**Features**: Full Docker deployment from GitHub, built-in PostgreSQL, object storage at $0.015/GB/mo with free egress.

### Pros
- Simple developer-friendly deployment
- Built-in managed PostgreSQL
- Good pricing for small teams
- Docker support

### Cons
- No GPU support
- Per-second billing can surprise
- Less control than self-hosted

### Roadmap Stage: Stage 0-1 (alternative to Fly.io)
### Budget Impact: $20-40/mo
### Rationale: Good alternative if Fly.io becomes insufficient. Not a long-term production choice.

---

## 4. Render

**Pricing**:
- Free tier (spins down after inactivity)
- Standard: $25/mo (1 CPU, 2GB RAM)
- Pro: $85/mo (2 CPU, 4GB RAM) up to Pro Ultra $450/mo (8 CPU, 32GB)
- Managed PostgreSQL: Free (30-day limit), Basic $6/mo, Pro from $55/mo

### Pros
- Auto-deploy from GitHub
- Horizontal scaling on Professional tier
- Good managed Postgres

### Cons
- More expensive than Fly.io/Railway for equivalent compute
- No GPU
- Free tier spins down (cold starts)

### Roadmap Stage: Stage 1 (alternative for managed deployment)
### Budget Impact: $25-85/mo
### Rationale: More expensive, less compelling than Fly.io or Hetzner.

---

## 5. Hetzner (Self-Hosted / Bare Metal) — RECOMMENDED FOR PRODUCTION

**Dedicated Servers (AX-series)**:
- AX42 (Ryzen 7 PRO 8700GE, 64GB DDR5, 2x512GB NVMe): ~EUR 54/mo
- AX102 (Ryzen 9 7950X3D, 128GB DDR5 ECC, 2x1.92TB NVMe): EUR 104-116/mo + EUR 269 setup

**Cloud VPS**: Starting from ~EUR 3.79/mo. General Purpose (dedicated vCPU) from ~EUR 17/mo.

**GPU Servers**: Typically EUR 200-400/mo for dedicated GPU servers. Significantly cheaper than cloud GPU.

**Bandwidth**: Unlimited traffic on dedicated servers (1 Gbit/s). Cloud VPS includes 20TB+ traffic.

**Locations**: Germany (Falkenstein, Nuremberg), Finland (Helsinki), US (Oregon, Virginia), Singapore.

### Pros
- Best price-performance ratio by far (AX102 at EUR 104/mo = 16 cores, 128GB RAM, 3.84TB NVMe)
- GPU servers available at reasonable cost
- Unlimited bandwidth on dedicated
- US Virginia location for NA users
- Full control over infrastructure

### Cons
- Self-managed (need Coolify/Kamal for deployment)
- Setup fee on some dedicated servers
- No managed services (must run own Postgres, etc.)
- EU data center locations may add latency for US users (mitigated by Virginia DC)

### Roadmap Stage: Stage 1-2 (Production deployment)
### Budget Impact: EUR 54-454/mo (app + optional GPU)
### Rationale: Best value for production. An AX102 gives more compute than $500/mo of cloud VMs.

---

## 6. Coolify (Self-Hosted PaaS)

- **License**: Apache-2.0
- **GitHub Stars**: 51.9K
- **What**: Docker-based deployment platform — essentially self-hosted Heroku/Vercel

**Features**:
- Git integration (GitHub, GitLab, Bitbucket)
- Auto Let's Encrypt SSL
- Server monitoring, real-time terminal
- Pull request preview deployments
- 280+ one-click services
- Multi-server support
- Built-in database management with auto-backups to S3-compatible storage

### Pros
- Apache-2.0 — fully permissive
- Web UI for deployment management
- Built-in monitoring and SSL
- PR preview deployments
- Massive community (51.9K stars)

### Cons
- PHP/Laravel codebase (not Rust-native, but you just use it)
- Another service to maintain on your server
- Less battle-tested than Kubernetes for large scale

### Roadmap Stage: Stage 1 (pair with Hetzner)
### Budget Impact: $0 (self-hosted)
### Rationale: Simplifies Hetzner management. Install once, get auto-deploys, SSL, monitoring.

---

## 7. Kamal (Docker Deployment Tool)

- **License**: MIT
- **GitHub Stars**: 13.9K
- **Maintainer**: 37signals (makers of Basecamp/HEY)

**Features**:
- Zero-downtime deploys via `kamal-proxy`
- Multi-server orchestration via SSH
- Auto-provisions Docker on vanilla Ubuntu
- Config-file driven

### Pros
- MIT license
- Lightweight — deployment tool, not a platform
- Zero-downtime deploys
- Perfect for bare metal (Hetzner)
- Backed by 37signals (battle-tested at HEY scale)

### Cons
- No web UI, no built-in monitoring
- Ruby dependency
- CLI-only workflow
- Less functionality than Coolify

### Roadmap Stage: Stage 1 (alternative to Coolify if CLI-preferred)
### Budget Impact: $0
### Rationale: Lighter weight than Coolify. Best for teams comfortable with CLI-driven deploys.

---

## 8. Object Storage Comparison

| Feature | Cloudflare R2 | MinIO (Self-Hosted) | AWS S3 |
|---------|--------------|---------------------|--------|
| Storage | $0.015/GB/mo | Free (your hardware) | $0.023/GB/mo |
| Egress | **FREE** | Free (your bandwidth) | $0.09/GB |
| Free tier | 10GB + ops | N/A | 5GB (12 months) |
| S3 compatible | Yes | Yes | Yes (native) |
| License | Proprietary | **AGPL-3.0** | Proprietary |
| Data sovereignty | CF edge | Full control | AWS regions |

**Recommendation**: R2 for external-facing assets (free egress). MinIO on Hetzner for sovereign/internal storage (note AGPL license — use as separate service, don't embed).

---

## 9. CDN Strategy

- **Frontend static assets**: Cloudflare Pages (free, global CDN, auto-deploy from Git)
- **API**: Cloudflare Tunnels to Hetzner (free, secure, no exposed ports)
- **Media/uploads**: Serve from R2 with Cloudflare CDN (automatic, zero egress)

**Recommended**: Cloudflare Pages for frontend + Cloudflare Tunnels to Hetzner for API.

---

## 10. Cost Comparison (Monthly)

### Scenario: 20-200 orgs, Rust/Axum backend, React frontend, PostgreSQL, Ollama for LLM

| Component | Fly.io Only | Hetzner + Coolify | Railway | Hybrid (Recommended) |
|-----------|------------|-------------------|---------|---------------------|
| Compute (app) | $32-65/mo | EUR 54/mo (AX42) | $20-40/mo | EUR 54/mo (Hetzner AX42) |
| Database | $38-72/mo (MPG) | Included | $10-20/mo | Included (self-hosted PG) |
| GPU/LLM | Not available | EUR 200-400/mo | Not available | EUR 200-400/mo |
| Object storage | N/A | Free (MinIO) | $5-10/mo | Free (R2, 10GB) or MinIO |
| CDN/Frontend | Included | Free (CF Pages) | Included | Free (CF Pages) |
| SSL/Domain | $0-2/mo | Free (Coolify LE) | Included | Free (Cloudflare) |
| Tunnels | N/A | Free (CF Tunnels) | N/A | Free (CF Tunnels) |
| **Total (no GPU)** | **$70-140/mo** | **EUR 54/mo (~$58)** | **$35-70/mo** | **~$58/mo** |
| **Total (with GPU)** | **Not possible** | **EUR 254-454/mo** | **Not possible** | **EUR 254-454/mo** |

---

## 11. Recommended Architecture by Phase

### Phase 1 (Now — SQLite, staging)
- Keep Fly.io for staging (~$7/mo)
- Add Cloudflare Pages for frontend CDN (free)
- Use R2 for media/backups (free tier)

### Phase 2 (Production — PostgreSQL, no GPU)
- Hetzner AX42 (EUR 54/mo) + Coolify
- Cloudflare Tunnels (free) for secure exposure
- Self-hosted PostgreSQL on same box
- Cloudflare Pages for frontend, R2 for media
- **Total: ~EUR 54/mo + Fly.io $7/mo staging = ~$65/mo**

### Phase 3 (GPU/LLM inference)
- Add Hetzner GPU dedicated server (EUR 200-400/mo)
- Run Ollama on GPU box, connect via private network
- Existing `Dockerfile` already has Ollama/CUDA setup

### Key Decisions
- **Coolify vs Kamal**: Coolify for web UI; Kamal for CLI-only. Both work on Hetzner.
- **LiteFS is abandoned** — do not rely on it. Migrate to PostgreSQL.
- **Fly.io GPU is deprecated** — Hetzner is the viable option for self-hosted GPU.

---

## 12. Infrastructure as Code

| Tool | License | Notes |
|------|---------|-------|
| **OpenTofu** | MPL 2.0 | Terraform fork, Linux Foundation governed |
| **Pulumi** | Apache 2.0 | Infrastructure as real code (TypeScript/Python/Go) |
| **Terraform** | **BSL** (since Aug 2023) | Non-permissive — use OpenTofu instead |

**Recommendation**: Pulumi (Apache 2.0) if you want to write infra in TypeScript alongside your frontend. OpenTofu if you prefer HCL/Terraform ecosystem without the BSL license.

---

## 13. Secrets Management

| Tool | License | Notes |
|------|---------|-------|
| **Infisical** | MIT (core) | Secrets management with CLI, SDK, CI integrations |
| **OpenBao** | MPL 2.0 | Vault fork, Linux Foundation, full secrets engine |
| **HashiCorp Vault** | **BSL** | Non-permissive — use OpenBao instead |

**Recommendation**: Infisical (MIT) for simplicity. OpenBao for Vault-compatible infrastructure.

---

## 14. Zero-Downtime Deployment Patterns

- **Blue/Green**: Run two identical environments, switch traffic atomically. Coolify and Kamal both support this.
- **Canary**: Route small % of traffic to new version, monitor, then promote. Requires load balancer support.
- **Rolling**: Update instances one at a time. Default in most container orchestrators.

For Rust services: Blue/Green is simplest with Kamal's `kamal-proxy`. Canary requires Traefik (MIT) or similar.

---

## 15. Database Migration (Zero-Downtime)

1. **Expand**: Add new columns/tables without removing old ones
2. **Migrate**: Copy data, update application to use new schema
3. **Contract**: Remove old columns after all services updated

Tools: pgloader (PostgreSQL License) for SQLite→PostgreSQL one-shot migration. sqlx migrations for ongoing schema changes.
