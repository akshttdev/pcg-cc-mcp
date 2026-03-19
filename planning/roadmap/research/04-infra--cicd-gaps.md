## Infrastructure, CI/CD, and Deployment Readiness Analysis

> **Last verified**: 2026-03-19 (codebase verification pass — all counts and claims checked against actual code)

### 1. CI/CD Pipelines (.github/workflows/)

**EXISTING:**
- **ci.yml**: Multi-stage CI pipeline running on pull requests and pushes to main
  - Rust format check (`cargo fmt`)
  - Rust linting (`cargo clippy` with `-D warnings`, continue-on-error)
  - Rust tests (`cargo test --workspace`, continue-on-error)
  - Frontend TypeScript typecheck (`tsc --noEmit`)
  - Frontend linting (`pnpm run lint`)
  - Type generation check (`npm run generate-types:check`)
  - Security audit (cargo-audit + npm audit with `--audit-level=high`)
  
- **build-release.yml**: Multi-platform binary building on main branch push
  - Builds for Linux (x86_64), macOS (ARM64/M1), Windows (x86_64)
  - Compiles two binaries: `server` and `orcha`
  - Creates GitHub Release with checksums (SHA256SUMS.txt)
  - Creates release notes with commit hash and build timestamp
  - 7-day artifact retention
  
- **build-release.yml**: Tauri desktop app build on version tags (`v*`)
  - Multi-platform desktop app builds (Linux AppImage/deb, macOS dmg/app, Windows msi/exe)
  - Artifacts uploaded to GitHub Release
  - Draft/prerelease mode

**GAPS:**
- **⚠️ P0: Clippy and tests use `continue-on-error: true`** — failures don't block merges. This means broken code can be merged to main without any CI gate. This is the single most critical CI gap and must be fixed before Stage 0 (Dogfood).
- No integration test suite in CI (backend API contract tests)
- No E2E test execution in CI pipeline (Playwright tests not run automatically)
- No frontend unit test step (no Vitest/Jest configured)
- No artifact signing for release binaries
- No automated deployment to production (manual step required)
- No SBOM (Software Bill of Materials) generation
- No license compliance scanning (critical given MIT/Apache-2.0/BSD-only policy — see Report 08)
- No `cargo-chef` or layer caching for faster CI builds (full rebuild every time)

---

### 2. Docker & Container Strategy

**DOCKERFILES:**

1. **Dockerfile** (main production image)
   - Multi-stage build: Debian bookworm builder → NVIDIA CUDA 12.1 runtime
   - Includes Rust nightly, Node 24, Python 3.11
   - Pre-pulls Ollama models (gpt-oss, deepseek-r1)
   - Runs as non-root user (appuser:1001)
   - Healthcheck: `GET /api/health` on port 3001
   - Entrypoint: `tini` + custom docker-entrypoint.sh
   - GPU support via nvidia-docker (`--gpus all`)
   - Installs: Ollama, Chatterbox TTS, Python dependencies

2. **Dockerfile.apn-bridge** (lightweight Python bridge)
   - Python 3.11-slim base
   - FastAPI + Uvicorn server
   - Healthcheck: HTTP GET on port 8000
   - Simple, stateless bridge for APN mesh networking

3. **Dockerfile.fly** (Fly.io deployment, production optimized)
   - Alpine-based build (smaller footprint)
   - No GPU/Ollama/TTS (cloud constraints)
   - Memory-constrained build (codegen-units=16, parallel=1)
   - Simplified entrypoint (no Ollama pre-pull)
   - Intended for lightweight cloud deployment

**CONTAINER ORCHESTRATION:**

**docker-compose.yml** (production stack):
- **app** service: Main PCG server (port 3001)
- **db-backup** service: Alpine container with daily DB backups (configurable interval/retention)
- **apn-bridge** service: Python bridge for mesh networking (port 8000, internal only)
- **nginx** service: Alpine reverse proxy (port 8080 → 80)
  - Routes `/api/events/*` to app (SSE support, buffering disabled)
  - Routes `/api/*` to app (general API)
  - Routes `/api/node/`, `/api/peers`, `/api/mesh/`, etc. to apn-bridge
  - Static frontend serving with 1-year cache on assets
  - Security headers: X-Frame-Options, X-Content-Type-Options, X-XSS-Protection, CSP
- **cloudflared** service: Cloudflare Tunnel for secure port forwarding (requires CLOUDFLARE_TUNNEL_TOKEN)

**docker-compose.local.yml** (dev version):
- Simplified: only app service, no cloudflared, debug logging

**GAPS:**
- No multi-replica support (single container per service)
- No resource limits defined in docker-compose (memory/CPU caps missing)
- No service mesh (Istio, Linkerd)
- Database backups run in separate container but no backup validation/verification
- No disaster recovery drills documented
- APN bridge runs on internal network only (not exposed directly)

---

### 3. Deployment Scripts & Infrastructure as Code

**EXISTING:**

**scripts/deploy.sh** (comprehensive orchestration):
- Prerequisites check (Docker, docker-compose, NVIDIA)
- Environment setup from .env.example template
- Service startup/stop/restart
- Health checks (curl endpoints)
- Manual backups
- Database operations (sqlite3 shell)
- Log tailing with timestamps
- System resource monitoring (docker stats)

**Makefile** (convenience wrapper):
- 26 targets for common operations
- Setup, deploy, start, stop, restart, status
- Logs (app, nginx, apn, cloudflared)
- Backup/update/clean commands
- Database shell access
- Health checks
- Resource monitoring

**systemd services** (deploy/systemd/):
- Service definitions for cloud server deployment
- Not actively used in Docker-based current setup

**Nginx config** (nginx/default.conf):
- Serves frontend static files from `/app/frontend/dist`
- Documentation at `/docs` (Mintlify static build)
- Proxies to app on port 3001
- Proxies to apn-bridge on port 8000
- WebSocket/SSE support for long-polling
- Gzip compression enabled
- Security headers configured
- 100M max body size for uploads
- Cache busting for static assets (1-year expires, versioned)

**GAPS:**
- No Infrastructure as Code (Terraform, Pulumi, CloudFormation)
- No Kubernetes manifests (k8s deployment would require significant refactoring)
- No monitoring/alerting infrastructure (Prometheus, Grafana, PagerDuty)
- No log aggregation (ELK, Splunk, CloudWatch)
- No automated scaling policies
- Database migrations not documented in deploy scripts
- No rollback procedures

---

### 4. Environment Management

**Configuration Files:**

1. **.flox/env/manifest.toml** (Flox dev environment)
   - Defines: Rust nightly, Node.js, pnpm, sqlx-cli, cmake, libopus, pkg-config, sccache, lld
   - Sets env vars: `SQLX_OFFLINE=true`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`, `RUSTC_WRAPPER=sccache`
   - Auto-seeds database from dev_assets_seed/db.sqlite

2. **.env** (runtime configuration)
   - BACKEND_PORT, FRONTEND_PORT, HOST, DATABASE_URL
   - RUST_LOG=info
   - All optional: OpenAI API key, Twilio, SendGrid, Google OAuth, GitHub OAuth, Zoho, Anthropic
   - Ollama config: OLLAMA_BASE_URL, OLLAMA_MODEL
   - Chatterbox TTS: CHATTERBOX_PORT, CHATTERBOX_DEVICE
   - APN config: APN_NODE_ID, APN_WALLET_ADDRESS, APN_RELAY_URL, AUTO_START_APN

3. **.env.example** (template with comprehensive documentation)
   - Well-commented, all variables documented
   - Example values provided for local development

4. **.env.integration.example** (integration test configuration)
   - Separate config for test scenarios

**GAPS:**
- No secrets management (Vault, AWS Secrets Manager)
- No config server (Spring Cloud Config, Consul)
- Environment-specific configs not segregated (dev/staging/prod all use same .env pattern)
- No encryption for sensitive env vars at rest
- No audit logging for env var access

---

### 5. Database Migrations

**Migration System: sqlx with offline mode**

**Statistics:**
- **215 total migrations** in `/crates/db/migrations/` (verified 2026-03-19)
- Latest migrations (sample):
  - 20250617183714: init.sql (baseline schema)
  - 20250620212427: execution_processes
  - 20250623120000: executor_sessions
  - 20250701000001: pr_tracking_to_task_attempts
  - 20250709000000: worktree_deleted_flag
  - 20250716161432: executor_names_to_kebab_case
  - 20250720000000: cleanup_script_to_process_type_constraint

**Tooling:**
- `sqlx-cli` for local migrations
- `SQLX_OFFLINE=true` mode (pre-computed query metadata)
- `.sqlx/` directory committed with query metadata
- Migrations can be rolled back (sqlx supports revert semantics, though reverting not common)

**GAPS:**
- No migration versioning/checksums visible in CI
- No zero-downtime migration strategy documented
- No migration dry-run or validation before apply
- No backward-compatibility testing for migrations
- Database schema version not queryable from app
- No data migration tooling (for schema changes with data reorganization)

---

### 6. Security

**Auth & Access Control:**

1. **GitHub OAuth** (primary auth method)
   - GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET configured
   - Device flow support for CLI tools
   - AccessContext middleware validates tokens

2. **Access Control Middleware** (crates/server/src/middleware/access_control.rs):
   - ProjectRole enum: Owner, Admin, Editor, Viewer
   - can_read(), can_write(), can_manage_members(), can_delete() permission checks
   - Organization-scoped access (projects filtered by organization_id)
   - Platform roles: operator, client_user, etc.

3. **Rate Limiting** (per-service):
   - Nora chat: 20 req/min (refill 1 per 3 sec)
   - Nora voice: 30 req/min (refill 1 per 2 sec)
   - TokenBucket implementation with async locks
   - Configurable presets: permissive, conservative, strict

**Input Validation:**
- Route handlers validate path/query params
- JSON payloads validated via serde + custom validators
- No explicit validator crate found, relying on type system + manual checks

**Network Security:**

1. **CORS Configuration** (crates/server/src/routes/mod.rs):
   - Whitelist-based (ALLOWED_ORIGINS env var)
   - Supports standard HTTP origins + tauri:// for desktop app
   - Predicate-based origin matching

2. **Nginx Security Headers:**
   - X-Frame-Options: SAMEORIGIN
   - X-Content-Type-Options: nosniff
   - X-XSS-Protection: 1; mode=block
   - Referrer-Policy: strict-origin-when-cross-origin
   - Permissions-Policy: geolocation=(), microphone=(), camera=()
   - server_tokens off (hide Nginx version)

3. **HTTPS/TLS:**
   - Cloudflare Tunnel for production (automatic SSL via CF)
   - Let's Encrypt via certbot for traditional deployments
   - Not enforced in local/docker setups (HTTP only)

4. **Multi-Tenancy Isolation:**
   - Organization-based scoping (organization_id in projects, CRM contacts, artifacts)
   - Database queries filter by `organization_id` on JOIN
   - No explicit tenant isolation at DB level (same SQLite for all orgs)
   - No row-level security (RLS) policies

**GAPS:**
- No WAF (Web Application Firewall)
- No SQL injection prevention explicit (relying on sqlx prepared statements)
- No API key authentication (only OAuth)
- No JWT refresh token rotation
- No secrets scanning in CI (git-secrets, detect-secrets)
- No SAST (Static Application Security Testing)
- No DAST (Dynamic Application Security Testing)
- No dependency scanning beyond npm audit / cargo audit (no Supply Chain security)
- TLS not enforced in Dockerfile (allows HTTP fallback)
- No rate limiting at nginx level (only app-level for Nora)

---

### 7. Monitoring, Observability & Logging

**Logging:**

1. **Application Logging:**
   - Uses `tracing` crate for structured logging
   - RUST_LOG env var controls level (default: info)
   - Logs to stdout (captured by Docker)

2. **Container Logging:**
   - Docker compose uses json-file driver with rotation:
     - max-size: 10m
     - max-file: 3 (30MB total per container)

3. **Entrypoint Logging:**
   - docker-entrypoint.sh logs to stderr (captured)
   - Ollama logs to /tmp/ollama.log (not persisted)
   - Chatterbox logs to /tmp/chatterbox.log (not persisted)

**Health Checks:**

1. **Kubernetes-style health checks:**
   - app: `GET /api/health` (port 3001) — every 30s, timeout 10s, retries 3, start period 60s
   - apn-bridge: `GET /health` (port 8000) — similar config
   - nginx: `GET /health` (port 80) — similar config

2. **Manual health checks:**
   - Makefile target: `make health` (curls endpoints)

**EXISTING (previously missed):**
- ✅ **Sentry error tracking** is integrated on BOTH backend and frontend:
  - Backend: `sentry_tracing::SentryLayer` in `crates/utils/src/sentry.rs`, wired into tracing subscriber in `main.rs`
  - Frontend: `@sentry/react` with `Sentry.init()` in `main.tsx` (DSN configured), `SentryRoutes` wrapping React Router
  - `update_sentry_scope()` called during deployment startup for context enrichment

**GAPS:**
- No metrics export (Prometheus metrics endpoint)
- No distributed tracing (Jaeger, Zipkin, OpenTelemetry)
- No APM (Application Performance Monitoring)
- No log aggregation (logs lost if container restarts)
- No alerting (alert rules, slack notifications, PagerDuty)
- No uptime monitoring (Ping, synthetic checks)
- No capacity planning/forecasting
- Ollama and Chatterbox logs not persisted (lost on restart)
- No request tracing context propagation (Sentry provides error tracking, not request tracing)

---

### 8. Multi-Tenancy Implementation

**Organization-Based Scoping:**

1. **Organization Table:**
   - All projects reference organization_id (nullable for legacy data)
   - Sidebar queries filter: `WHERE om.user_id = ? AND p.organization_id IS NOT NULL`
   - Projects without organization_id don't appear in multi-org dashboards

2. **Access Control:**
   - organization_members table tracks user membership
   - Projects filtered by organization in sidebar SQL

3. **CRM Data (2026-03-09 migration complete):**
   - CRM Contacts scoped by organization_id (+ optional client_id)
   - Workflows scoped by organization
   - Execution artifacts indexed by organization

4. **Artifacts Organization Scoping (2026-03-10):**
   - GET /api/artifacts/recent filters via JOIN to data_sources.organization_id
   - BLOB UUIDs in data_sources need hex-to-text conversion for matching

**Tenant Leakage Risks:**

- **MEDIUM RISK**: Database is single SQLite shared by all orgs (no hard isolation)
- **MEDIUM RISK**: No row-level security (RLS) — permissions rely on application-level queries
- **MEDIUM RISK**: If SQL injection vulnerability exists, all org data exposed
- **LOW RISK**: Sidebar queries have organization_id filters in WHERE clause (most common query pattern)
- **MEDIUM RISK**: Legacy projects with NULL organization_id visible to all users (dangerous)
- **MEDIUM RISK**: No encryption at rest — database file readable if filesystem compromised

---

### 9. Production Readiness Checklist

| Area | Status | Notes |
|------|--------|-------|
| **Graceful Shutdown** | PARTIAL | CancellationToken used in code, but Dockerfile uses tini (good). No explicit drain period for in-flight requests. |
| **Health Checks** | GOOD | HTTP health endpoints defined, Docker healthchecks configured. |
| **Backups** | GOOD | db-backup service does daily backups with retention. No backup validation. |
| **Database Migrations** | GOOD | sqlx handles migrations, but no zero-downtime strategy. |
| **Secrets Management** | POOR | All secrets in .env file. No vault, no encryption at rest. |
| **Monitoring/Alerts** | PARTIAL | Sentry error tracking (backend + frontend). No metrics export, no alerting rules, no distributed tracing. |
| **Load Balancing** | MISSING | Single container. No horizontal scaling. |
| **Auto-Recovery** | PARTIAL | restart_policy on docker-compose set to on-failure with exponential backoff. |
| **Data Encryption** | POOR | No encryption at rest (SQLite file unencrypted), TLS optional. |
| **Rate Limiting** | PARTIAL | Per-endpoint in app (Nora). No global rate limiting at nginx level. |
| **CORS/Security Headers** | GOOD | Properly configured nginx headers. CORS whitelist enforced. |
| **Deployment Strategy** | MANUAL | No CI/CD deployment automation (manual docker-compose up). |
| **Rollback Capability** | PARTIAL | Database backups exist, but app rollback manual. No versioning. |
| **Capacity Planning** | MISSING | No resource quotas, no auto-scaling. |
| **Disaster Recovery** | POOR | Backups exist, but no RTO/RPO defined, no DR drill. |

---

### 10. Desktop App (Tauri)

**Location:** `/apn-app/`

**Status:**
- Tauri-based cross-platform app (Linux, macOS, Windows)
- Bundled with APN mesh networking
- CI/CD workflow (build-release.yml) builds installers for all platforms
- Artifacts: AppImage/deb (Linux), dmg/app (macOS), msi/exe (Windows)

**Desktop-Specific Features:**
- Tauri origin support (tauri:// scheme in CORS config)
- Local APN node integration (AUTO_START_APN config)
- Desktop-specific configuration

**GAPS:**
- No auto-update mechanism (users must manually download new releases)
- No code signing/notarization for macOS/Windows
- No crash reporting for desktop
- Desktop CI/CD runs on version tag push (not every commit)

---

### 11. APN/Alpha Protocol Network

**Crates:**
- `alpha-protocol-core`: Core mesh networking (libp2p, X25519 encryption, Ed25519 keys, BIP39 mnemonics)
- `apn-client`: Client library for APN integration
- `apn-bridge`: HTTP-to-NATS bridge (FastAPI server in Dockerfile.apn-bridge)

**Architecture:**
- **Identity:** Ed25519 keypairs, BIP39 mnemonics, wallet addresses
- **Encryption:** X25519 key exchange, ChaCha20-Poly1305
- **Transport:** libp2p (gossipsub, kad, mdns, noise)
- **Relay:** NATS for NAT traversal (default: nats://nonlocal.info:4222)
- **Wire Format:** HANDSHAKE, DATA, ACK, PEER_ANNOUNCE messages
- **Economics:** Contribution tracking, reward distribution (Vibe tokens)

**Mesh Node Implementation:**
- MeshNode, AlphaNode structures in rust code
- NodeReputation, ResourceContribution tracking
- RewardDistributor, RewardTracker for incentive alignment

**Status:**
- Core protocol implemented
- Bridge server partially stubbed (Python FastAPI, not full mesh)
- Mesh networking not fully integrated into main server (optional feature)
- APN node binary available but AUTO_START_APN=false in Docker by default

**GAPS:**
- Mesh networking is optional — production readiness unclear for multi-node deployment
- APN bridge is a **functional NATS relay** (not just an echo server as previously reported) — it bridges HTTP requests to NATS messaging for NAT traversal, but full mesh protocol (gossipsub, kad) is in the Rust `alpha-protocol-core` crate, not the Python bridge
- No mesh consensus mechanism documented
- No Byzantine fault tolerance mechanisms
- Reward distribution not tested at scale
- Mesh bootstrap/peer discovery not fully implemented

---

### 12. Automation Scripts

**Key Scripts (in /scripts/):**

1. **Deployment:**
   - deploy.sh (comprehensive docker orchestration)
   - build-dashboard.sh, build-orcha.sh, build-npm-package.sh

2. **Backup & Recovery:**
   - backup-manager.sh (database backup with retention)
   - daily-backup.sh (cron-friendly backup)

3. **Testing:**
   - create-test-seed.sh (generates test database seed)
   - check-both.sh (lints + tests)
   - e2e-env-check.js (verifies E2E test prerequisites)

4. **Database:**
   - export-data.sh (backup/export utilities)
   - fix_database.sh (repair corrupted DBs)

5. **Network/APN:**
   - apn_sync_setup.sh, apn_manual_sync.sh (mesh network sync)
   - apn_bridge_server.py, apn_data_sync_receiver.py (bridge utilities)

6. **Diagnostics:**
   - diagnose-dashboard.sh, diagnose_tasks_issue.sh
   - check-network-capacity.sh

7. **Configuration:**
   - check-voice-config.sh (Ollama/Chatterbox validation)
   - check-nora-email.js, check-meet-signin.js

**GAPS:**
- Scripts are ad-hoc solutions, not standardized
- No configuration management (Ansible, Salt)
- No package management for reproducible deploys
- No version pinning for OS-level dependencies

---

## Summary: Production Readiness Assessment

### READY FOR PRODUCTION:
- CI/CD pipelines (basic, but functional)
- Docker containerization (well-structured, multi-stage)
- Database migrations (sqlx handles reliably)
- Health checks (HTTP endpoints implemented)
- Security headers (nginx configured well)
- CORS/access control (middleware present)
- Backup strategy (automated, with retention)

### NEEDS WORK:
- Monitoring & alerting (logging only, no metrics/tracing/alerts)
- Secrets management (no vault, .env files exposed)
- Horizontal scaling (no load balancer, single container)
- Multi-tenancy isolation (no RLS, shared SQLite)
- Auto-scaling (no policies)
- Disaster recovery (no RTO/RPO, no drill procedures)
- Log persistence (temporary files, not aggregated)
- Rate limiting (app-level only, no global throttling)

### NICE-TO-HAVE:
- Distributed tracing (Jaeger)
- APM (New Relic, Datadog)
- SBOM generation
- Artifact signing
- Zero-downtime deployments
- Canary deployments
- Infrastructure as Code (Terraform)
- Configuration server
- Service mesh

**Overall:** **72/100 — Production-Capable But Not Production-Hardened.** The application can run in production with the existing setup (Sentry provides error visibility), but enterprise-grade reliability, observability, and security features are missing.

---

### 13. VERIFIED CRITICAL GAPS (Cross-Referenced from Research Reports 05-25)

> These gaps were identified by cross-referencing findings from all 25 research reports against the actual infrastructure.

| Gap | Impact | Blocks Stage | Priority | Reference |
|-----|--------|-------------|----------|-----------|
| CI `continue-on-error: true` on clippy/tests | Broken code can merge to main | Stage 0 (Dogfood) | **P0** | This report |
| No secrets management (Vault, SOPS) | Credentials in `.env` files | Stage 1 (Pilot) | P0 | Report 08 (Legal) |
| No Prometheus metrics endpoint | No quantitative observability | Stage 1 (Pilot) | P1 | Report 16 (Observability) |
| No distributed tracing (OTEL) | Can't trace cross-service requests | Stage 2 (Growth) | P2 | Report 16 (Observability) |
| No zero-downtime deployment | Service interruption on deploy | Stage 1 (Pilot) | P1 | Report 05 (Deployment) |
| No `cargo-chef` / Docker layer caching | Slow CI builds (~15-20 min) | Stage 0 (Dogfood) | P2 | This report |
| No SBOM generation | Can't audit supply chain | Stage 1 (Pilot) | P1 | Report 08 (Legal) |
| No license compliance scanning | GPL/AGPL deps could violate policy | Stage 1 (Pilot) | P1 | Report 08 (Legal) |
| No tenant isolation at DB level (RLS) | Org data leakage risk | Stage 2 (Growth) | P1 | Report 08 (Legal) |
| No automated deployment pipeline | Manual docker-compose up | Stage 1 (Pilot) | P1 | Report 05 (Deployment) |
| No E2E tests in CI | Regressions not caught pre-merge | Stage 0 (Dogfood) | P1 | Report 03 (Frontend) |
| No frontend unit tests in CI | Hook/utility bugs uncaught | Stage 0 (Dogfood) | P1 | Report 03 (Frontend) |

### Corrected Assessment (from initial report)

- **Sentry**: Previously listed as "missing" — it IS integrated on both backend (`sentry-tracing` crate) and frontend (`@sentry/react`). Score adjusted upward.
- **APN Bridge**: Previously described as "echo server" — it is a functional NATS relay for NAT traversal. Score adjusted.
- **Makefile**: 26 targets (not "40+")
- **Migration count**: 215 (verified)