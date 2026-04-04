# Phase V Priority Release Plan (PRP)
## PCG-CC-MCP Development Acceleration Strategy

**Version:** 5.0
**Date:** December 11, 2025
**Status:** 🚀 Ready for Implementation
**Target Completion:** Q1 2026 (90 days)

---

## 📋 Executive Summary

This Priority Release Plan accelerates PCG-CC-MCP from a production-ready MVP to a market-leading AI-native project management platform through focused sprints targeting:

1. **Developer Velocity Enhancement** (Weeks 1-2)
2. **Core Platform Stability** (Weeks 3-4)
3. **AI Capabilities Expansion** (Weeks 5-7)
4. **Enterprise Market Readiness** (Weeks 8-10)
5. **Scale & Performance** (Weeks 11-12)

**Success Metrics:**
- 50% reduction in time-to-deployment for new features
- 90%+ test coverage for critical paths
- Sub-500ms API response times (p95)
- 10x increase in concurrent user capacity
- Zero-downtime deployment capability

---

## 🎯 Current State Analysis

### Strengths to Leverage
✅ **Solid Foundation:** 173 Rust files, 286 TypeScript files
✅ **Feature Complete:** Phase IV delivered 4/4 features
✅ **Production Ready:** Docker deployment with backup automation
✅ **AI-Native:** MCP server, NORA assistant, AI executors

### Velocity Bottlenecks Identified
⚠️ **Testing Gap:** Frontend lacks comprehensive test suite
⚠️ **TypeScript Warnings:** 110 ESLint warnings degrading DX
⚠️ **Manual Processes:** Type generation requires manual npm run
⚠️ **No CI/CD Pipeline:** Manual deployment increases risk
⚠️ **Documentation Fragmentation:** 35+ .md files, no single source of truth

### Market Opportunities
🎯 **AI Development Teams:** Growing market for AI-augmented workflows
🎯 **Web3 Agencies:** Blockchain integration differentiator
🎯 **Enterprise SaaS:** RBAC positions for B2B sales
🎯 **Self-Hosted Demand:** Privacy-conscious organizations

---

## 🚀 Phase V Implementation Roadmap

---

## **SPRINT 1-2: Developer Velocity Foundation** (Weeks 1-2)
**Goal:** Reduce friction, increase confidence, accelerate iteration speed

### PRP-001: Automated CI/CD Pipeline
**Priority:** 🔥 Critical
**Effort:** 5 days
**Impact:** 10x faster deployment cycle

**Requirements:**
```yaml
GitHub Actions Workflow:
  - name: "Continuous Integration"
    triggers: [push, pull_request]
    jobs:
      - backend_tests:
          - cargo test --workspace
          - cargo clippy --all --all-targets -- -D warnings
          - cargo fmt --all -- --check
      - frontend_tests:
          - npm run lint
          - npm run check (TypeScript compilation)
          - npm run test (Vitest unit tests - NEW)
      - type_sync_check:
          - npm run generate-types:check
      - integration_tests:
          - Docker build test
          - Health check verification

  - name: "Continuous Deployment"
    triggers: [push to main]
    jobs:
      - build_docker_image:
          - Build multi-arch (amd64, arm64)
          - Push to Docker Hub (kingbodhi/pcg-cc-mcp)
          - Tag: latest, git-sha, semver
      - deploy_staging:
          - Auto-deploy to staging environment
          - Run smoke tests
      - notify_deployment:
          - Slack/Discord notification
```

**Success Criteria:**
- ✅ Every commit automatically tested
- ✅ Main branch always deployable
- ✅ < 10 minute CI run time
- ✅ Automated Docker Hub publishing

**Implementation Steps:**
1. Create `.github/workflows/ci.yml`
2. Create `.github/workflows/cd.yml`
3. Add Vitest configuration for frontend
4. Create smoke test suite
5. Setup Docker Hub credentials as secrets
6. Configure staging environment (DigitalOcean/Railway/Fly.io)

---

### PRP-002: Frontend Testing Infrastructure
**Priority:** 🔥 Critical
**Effort:** 4 days
**Impact:** Prevent regressions, enable confident refactoring

**Requirements:**
```typescript
Testing Stack:
  - Framework: Vitest (Vite-native, fast)
  - React Testing: @testing-library/react
  - User Interaction: @testing-library/user-event
  - API Mocking: msw (Mock Service Worker)

Initial Coverage Targets:
  - Critical User Flows: 90%
    - Authentication (login, logout, session)
    - Task CRUD operations
    - Project access control
    - NORA chat interaction
  - UI Components: 70%
    - TaskCard, ProjectCard, BoardView
    - TaskFormDialog, ProjectFormDialog
    - CustomPropertiesPanel
  - Utility Functions: 100%
    - API client (lib/api.ts)
    - Auth helpers
    - Date formatting

Test Structure:
  frontend/
    src/
      components/
        __tests__/
          TaskCard.test.tsx
          ProjectCard.test.tsx
      lib/
        __tests__/
          api.test.ts
    test/
      setup.ts
      mocks/
        handlers.ts (MSW handlers)
```

**Success Criteria:**
- ✅ 70% overall code coverage
- ✅ 90% coverage for critical paths
- ✅ < 30 second test suite execution
- ✅ Zero flaky tests

**Implementation Steps:**
1. Add Vitest, @testing-library/react, msw to package.json
2. Create `vitest.config.ts` in frontend/
3. Setup test utilities and MSW handlers
4. Write tests for 5 critical components
5. Add coverage reporting to CI
6. Document testing patterns in CLAUDE.md

---

### PRP-003: TypeScript Quality Sprint
**Priority:** 🟡 High
**Effort:** 3 days
**Impact:** Improved DX, fewer runtime errors

**Requirements:**
```bash
Current State: 110 ESLint warnings
Target State: 0 warnings, strict mode enabled

Categories to Address:
  1. Unused imports (eslint-plugin-unused-imports)
  2. Missing type annotations
  3. Implicit any types
  4. React Hook dependencies
  5. Accessibility warnings

Configuration Updates:
  - Enable TypeScript strict mode
  - Enable strictNullChecks
  - Enable noImplicitAny
  - Reduce max-warnings to 0 (from 110)
```

**Success Criteria:**
- ✅ Zero ESLint warnings
- ✅ TypeScript strict mode enabled
- ✅ No implicit `any` types
- ✅ All React hooks have correct dependencies

**Implementation Steps:**
1. Run `npm run lint:fix` to auto-fix low-hanging fruit
2. Create tracking spreadsheet of remaining warnings by category
3. Fix warnings in order: unused imports → type annotations → hooks
4. Enable strict mode incrementally (per directory)
5. Update ESLint config to enforce strict rules
6. Document type patterns in AGENTS.md

---

### PRP-004: Automated Type Generation Hook
**Priority:** 🟡 High
**Effort:** 2 days
**Impact:** Never forget to regenerate types

**Requirements:**
```rust
// Add to crates/server/build.rs or create cargo-watch hook
// Automatically run generate_types when Rust types change

Options:
  A. cargo-watch custom command:
     cargo watch -w crates -s 'cargo run --bin generate_types'

  B. Pre-commit hook (recommended):
     .git/hooks/pre-commit:
       - Check if any .rs files with #[derive(TS)] changed
       - Run npm run generate-types
       - Stage updated shared/types.ts
       - Fail if types.ts is out of sync

  C. Build.rs integration:
     - Run ts-rs export during cargo build
     - Automatically keep types in sync
```

**Success Criteria:**
- ✅ Types auto-regenerate on Rust changes
- ✅ CI fails if types are out of sync
- ✅ No manual `npm run generate-types` needed

**Implementation Steps:**
1. Create `.git/hooks/pre-commit` script
2. Add type-sync check to CI
3. Update CLAUDE.md with new workflow
4. Test with dummy type change

---

## **SPRINT 3-4: Core Platform Stability** (Weeks 3-4)
**Goal:** Production-hardened reliability, observability, error handling

### PRP-005: Comprehensive Error Handling & Recovery
**Priority:** 🔥 Critical
**Effort:** 5 days
**Impact:** Graceful degradation, better UX

**Requirements:**
```rust
Backend Error Improvements:
  1. Structured Error Types:
     - ApiError enum with proper HTTP status codes
     - User-facing error messages
     - Internal error details (logged, not exposed)

  2. Error Context:
     - Add .context() to all anyhow errors
     - Include request_id in all error logs
     - Structured logging with tracing

  3. Recovery Mechanisms:
     - Database connection retry logic
     - Git operation timeouts and cleanup
     - AI executor failure recovery
     - Rate limit backoff strategies

Frontend Error Improvements:
  1. Global Error Boundary:
     - Catch React errors gracefully
     - Show user-friendly error page
     - Log to Sentry with component stack

  2. API Error Handling:
     - Retry logic for transient failures
     - User-facing error toasts
     - Offline mode detection

  3. Loading States:
     - Skeleton loaders for all async operations
     - Timeout indicators
     - Cancel button for long operations
```

**Success Criteria:**
- ✅ Zero unhandled errors reaching users
- ✅ All errors logged to Sentry with context
- ✅ < 5 second recovery from transient failures
- ✅ User-friendly error messages (no stack traces)

---

### PRP-006: Observability & Monitoring Dashboard
**Priority:** 🟡 High
**Effort:** 4 days
**Impact:** Proactive issue detection

**Requirements:**
```yaml
Metrics Collection:
  - Prometheus metrics already in place ✅
  - Add business metrics:
    - Active users (daily, weekly, monthly)
    - Tasks created per day
    - AI executor success/failure rates
    - NORA query response times
    - Git operations duration
    - Database query performance

Monitoring Dashboard:
  - Tool: Grafana (self-hosted) or Grafana Cloud
  - Dashboards:
    1. System Health:
       - CPU, Memory, Disk usage
       - Request rate, latency (p50, p95, p99)
       - Error rate by endpoint
    2. Business Metrics:
       - User activity heatmap
       - Feature usage statistics
       - AI executor performance
    3. Database Performance:
       - Query latency
       - Connection pool utilization
       - Slow query log

Alerting:
  - Alert Manager integration
  - Alert channels: Email, Slack, PagerDuty
  - Alert conditions:
    - Error rate > 1% for 5 minutes
    - API latency p95 > 2 seconds
    - Database connections > 80%
    - Disk usage > 85%
```

**Success Criteria:**
- ✅ Real-time visibility into system health
- ✅ Alerts fire before users report issues
- ✅ 5-minute incident detection time
- ✅ Historical trend analysis available

---

### PRP-007: Database Optimization & Migration Path
**Priority:** 🟡 High
**Effort:** 4 days
**Impact:** Support scale, enable multi-tenancy

**Requirements:**
```sql
SQLite Optimization (Current):
  - Add missing indexes:
    CREATE INDEX idx_tasks_project_id ON tasks(project_id);
    CREATE INDEX idx_tasks_status ON tasks(status);
    CREATE INDEX idx_tasks_assignee ON tasks(assignee_id);
    CREATE INDEX idx_project_members_user_id ON project_members(user_id);
    CREATE INDEX idx_permission_audit_user ON permission_audit_log(user_id);

  - Enable WAL mode (Write-Ahead Logging):
    PRAGMA journal_mode=WAL;

  - Connection pooling tuning:
    max_connections: 10
    min_connections: 2
    acquire_timeout: 30s

PostgreSQL Migration Prep:
  - Document PostgreSQL feature flag usage
  - Create migration guide from SQLite → PostgreSQL
  - Test dual-database compatibility
  - Prepare Docker Compose with PostgreSQL service

Database Backup Improvements:
  - Add backup verification (restore test)
  - Compressed backups (gzip)
  - Off-site backup option (S3, Backblaze B2)
  - Point-in-time recovery capability
```

**Success Criteria:**
- ✅ 50% faster query performance on large datasets
- ✅ PostgreSQL migration path documented and tested
- ✅ Automated backup verification
- ✅ < 1 hour recovery time objective (RTO)

---

## **SPRINT 5-7: AI Capabilities Expansion** (Weeks 5-7)
**Goal:** Differentiate with advanced AI features, expand NORA capabilities

### PRP-008: NORA External Integrations
**Priority:** 🔥 Critical
**Effort:** 6 days
**Impact:** Unlock 7 blocked tools, increase utility 3x

**Requirements:**
```yaml
Tool Completion Priority:
  1. SendEmail (SMTP Integration):
     - Support: Gmail, Outlook, SendGrid, custom SMTP
     - Configuration via environment variables
     - Template system for common emails
     - Async delivery with retry logic

  2. SendSlackMessage (Slack Webhook):
     - OAuth integration or webhook URL
     - Support channels, DMs, threads
     - Rich message formatting (markdown)
     - File attachments

  3. SearchWeb (Search API Integration):
     - Providers: SerpAPI, Brave Search, Google Custom Search
     - Multi-provider fallback
     - Result caching (1 hour TTL)
     - Rate limit handling

  4. Calendar Integration (Google Calendar):
     - OAuth 2.0 flow for user calendar access
     - Create/read/update events
     - Availability checking
     - Meeting scheduling

  5. ExecuteCode Sandboxing:
     - Docker-in-Docker for code execution
     - Resource limits (CPU, memory, time)
     - Network isolation
     - Supported languages: Python, JavaScript, Rust, Go
     - Security audit before production

Configuration Management:
  - Add settings page: Settings → Integrations
  - Per-user OAuth connections
  - Admin-level default configurations
  - Secure credential storage (encrypted at rest)
```

**Success Criteria:**
- ✅ All 13 NORA tools production-ready
- ✅ < 3 minutes to configure new integration
- ✅ 99.9% tool execution success rate
- ✅ Clear error messages for misconfigured tools

---

### PRP-009: AI Executor Enhancements
**Priority:** 🟡 High
**Effort:** 5 days
**Impact:** Better AI agent results, faster execution

**Requirements:**
```rust
Executor Improvements:
  1. Parallel Execution:
     - Run multiple AI agents concurrently
     - Task dependency graph
     - Resource allocation (CPU, memory per executor)

  2. Context Awareness:
     - Include project README in context
     - Include recent git commits
     - Include related task descriptions
     - Include custom properties in prompts

  3. Prompt Engineering:
     - Versioned prompt templates
     - Project-specific prompt overrides
     - A/B testing framework for prompts
     - Success metrics per prompt version

  4. Result Quality:
     - Automatic linting of generated code
     - Test execution before PR creation
     - Code review checklist generation
     - Diff summary in human language

  5. Cost Tracking:
     - Token usage per task
     - Cost attribution per project/user
     - Budget limits and alerts
     - Monthly usage reports
```

**Success Criteria:**
- ✅ 2x faster task execution (parallel executors)
- ✅ 30% higher code quality (lint pass rate)
- ✅ Transparent cost visibility
- ✅ Zero runaway API costs

---

### PRP-010: Multi-Agent Collaboration Framework
**Priority:** 🟢 Medium
**Effort:** 6 days
**Impact:** Complex task orchestration

**Requirements:**
```yaml
Agent Orchestration:
  - Master agent coordinates sub-agents
  - Task decomposition into subtasks
  - Sub-agent specialization:
    - code_writer: Generates code
    - code_reviewer: Reviews PRs
    - test_writer: Creates test cases
    - documentation: Updates docs
  - Inter-agent communication protocol
  - Shared context store

Example Workflow:
  User creates task: "Add user profile page with avatar upload"

  Master Agent:
    1. Decomposes into subtasks:
       - Design database schema
       - Create API endpoints
       - Build frontend component
       - Write tests
       - Update documentation

    2. Assigns to specialized agents:
       - db_specialist → schema
       - api_specialist → endpoints
       - frontend_specialist → component
       - test_specialist → tests
       - doc_specialist → documentation

    3. Coordinates execution:
       - Run db_specialist first
       - Run api_specialist after schema ready
       - Run frontend_specialist after API ready
       - Run test_specialist in parallel
       - Run doc_specialist last

    4. Consolidates results:
       - Single PR with all changes
       - Coherent commit history
       - Complete documentation
```

**Success Criteria:**
- ✅ Handle tasks with 5+ subtasks
- ✅ 50% faster completion for complex tasks
- ✅ Higher quality output (all aspects covered)
- ✅ Clear visibility into orchestration progress

---

## **SPRINT 8-10: Enterprise Market Readiness** (Weeks 8-10)
**Goal:** Productize for enterprise sales, add must-have enterprise features

### PRP-011: Multi-Tenancy Architecture
**Priority:** 🔥 Critical
**Effort:** 7 days
**Impact:** Unlock SaaS business model

**Requirements:**
```rust
Architecture Changes:
  1. Organization Model:
     - Add organizations table
     - Users belong to organizations
     - Projects belong to organizations
     - Billing at organization level

  2. Data Isolation:
     - Row-Level Security (RLS) in PostgreSQL
     - All queries scoped to organization_id
     - API middleware enforces organization context
     - Cross-organization data access forbidden

  3. Organization Management:
     - Create organization (signup flow)
     - Invite users to organization
     - Organization admin role
     - Organization settings page
     - Organization usage dashboard

  4. Resource Limits:
     - Per-org project limits
     - Per-org user limits
     - Per-org storage limits
     - Per-org API rate limits
     - Upgrade prompts when limits reached

Database Schema:
  organizations:
    - id, name, slug, created_at
    - plan (free, pro, enterprise)
    - limits (projects, users, storage)

  organization_members:
    - organization_id, user_id, role
    - roles: owner, admin, member

  organization_invites:
    - email, organization_id, role
    - token, expires_at

Migration Strategy:
  - Default organization for existing users
  - Migrate existing projects to default org
  - Graceful handling of single-tenant installs
```

**Success Criteria:**
- ✅ Complete data isolation between orgs
- ✅ < 10ms overhead for org context check
- ✅ Self-service organization creation
- ✅ Zero data leakage in audit

---

### PRP-012: Advanced RBAC & Audit
**Priority:** 🟡 High
**Effort:** 4 days
**Impact:** SOC 2 compliance readiness

**Requirements:**
```yaml
Enhanced Permissions:
  - Custom roles (beyond Owner/Admin/Editor/Viewer)
  - Granular permissions:
    - Can create tasks
    - Can delete tasks
    - Can manage members
    - Can export data
    - Can view financials
    - Can manage integrations

  - Permission templates:
    - "Developer" role
    - "Project Manager" role
    - "Stakeholder" role
    - "Auditor" role

Audit Log Enhancements:
  - Comprehensive event tracking:
    - All CRUD operations
    - Permission changes
    - Configuration changes
    - API key creation/deletion
    - Export operations
    - Login/logout events

  - Audit log viewer:
    - Filter by user, action, resource
    - Date range selection
    - Export to CSV
    - Real-time stream

  - Compliance reports:
    - "Who accessed what when" reports
    - Permission change history
    - Anomaly detection (unusual access patterns)

Session Management:
  - Force logout all sessions
  - View active sessions per user
  - Session timeout configuration
  - IP address logging
  - Device fingerprinting
```

**Success Criteria:**
- ✅ Custom role creation in < 2 minutes
- ✅ Complete audit trail for compliance
- ✅ Session management for security incidents
- ✅ SOC 2 Type II ready

---

### PRP-013: SSO & Enterprise Authentication
**Priority:** 🟡 High
**Effort:** 5 days
**Impact:** Removes enterprise adoption blocker

**Requirements:**
```yaml
SSO Providers:
  - SAML 2.0:
    - Okta, Auth0, OneLogin
    - Azure AD, Google Workspace

  - OAuth 2.0 / OIDC:
    - GitHub (already implemented ✅)
    - GitLab, Bitbucket
    - Microsoft, Google

Configuration:
  - Per-organization SSO settings
  - Automatic user provisioning (SCIM)
  - Just-in-time (JIT) provisioning
  - Attribute mapping (email, name, role)
  - Fallback to password auth (optional)

Admin Experience:
  - Settings → Authentication
  - Upload IdP metadata (SAML)
  - Configure OIDC endpoints
  - Test connection button
  - User provisioning logs

Security:
  - Enforce SSO for organization (disable passwords)
  - Multi-factor authentication (MFA) support
  - Session lifetime policies
  - Conditional access (IP whitelist)
```

**Success Criteria:**
- ✅ < 15 minutes to configure SSO
- ✅ Support top 5 enterprise IdPs
- ✅ Automatic user provisioning works
- ✅ Zero login friction for SSO users

---

## **SPRINT 11-12: Scale & Performance** (Weeks 11-12)
**Goal:** Handle 10x traffic, sub-second response times

### PRP-014: Caching Layer Implementation
**Priority:** 🔥 Critical
**Effort:** 4 days
**Impact:** 10x faster common queries

**Requirements:**
```rust
Redis Integration:
  - Cache backend: Redis or Valkey (Redis fork)
  - Cache patterns:
    1. Read-through cache:
       - Check cache first
       - On miss, query DB and populate cache
       - TTL: 5-60 minutes depending on data

    2. Write-through cache:
       - Update DB and cache simultaneously
       - Invalidate dependent cache keys

    3. Cache-aside (manual):
       - Application controls caching logic
       - Fine-grained invalidation

Cache Keys Design:
  - User: user:{user_id}
  - Project: project:{project_id}
  - Project list: projects:user:{user_id}
  - Tasks: tasks:project:{project_id}
  - NORA response: nora:response:{hash}

Cache Invalidation:
  - On update: invalidate specific keys
  - On delete: cascade invalidation
  - Pub/sub for multi-instance invalidation
  - Background job for warming popular keys

Monitoring:
  - Cache hit rate (target: >80%)
  - Cache memory usage
  - Eviction rate
  - Key TTL distribution
```

**Success Criteria:**
- ✅ 80%+ cache hit rate
- ✅ Sub-50ms response for cached queries
- ✅ Graceful degradation if Redis down
- ✅ 10x reduction in database load

---

### PRP-015: API Rate Limiting & Throttling
**Priority:** 🟡 High
**Effort:** 3 days
**Impact:** Prevent abuse, fair usage

**Requirements:**
```rust
Rate Limit Tiers:
  Free Tier:
    - 100 requests/minute per user
    - 1,000 requests/hour
    - 10,000 requests/day

  Pro Tier:
    - 500 requests/minute per user
    - 10,000 requests/hour
    - 100,000 requests/day

  Enterprise Tier:
    - Custom limits
    - Burst allowance
    - Dedicated rate limit pool

Implementation:
  - Algorithm: Token bucket or leaky bucket
  - Storage: Redis for distributed rate limiting
  - Headers:
    - X-RateLimit-Limit
    - X-RateLimit-Remaining
    - X-RateLimit-Reset
  - Response: 429 Too Many Requests with Retry-After

Endpoint-Specific Limits:
  - Expensive operations (AI executors): 10/hour
  - NORA chat: 20/minute (already implemented ✅)
  - File uploads: 50/hour
  - Exports: 100/day
  - Standard CRUD: Use tier limits

Admin Controls:
  - View rate limit usage per user/org
  - Temporary limit increases
  - Whitelist specific IPs
  - Blacklist abusive clients
```

**Success Criteria:**
- ✅ Zero service degradation from abuse
- ✅ Fair resource allocation
- ✅ Clear limit communication to users
- ✅ Easy limit adjustment in admin panel

---

### PRP-016: Database Connection Pooling & Query Optimization
**Priority:** 🟡 High
**Effort:** 4 days
**Impact:** Support 10x concurrent users

**Requirements:**
```rust
Connection Pooling:
  - Library: SQLx built-in pool (already used ✅)
  - Configuration tuning:
    - max_connections: Based on (CPU cores * 2) + disk spindles
    - min_connections: max_connections / 4
    - acquire_timeout: 30 seconds
    - idle_timeout: 10 minutes
    - max_lifetime: 30 minutes

  - Health checks:
    - Test connections on acquire
    - Evict stale connections
    - Monitor connection pool metrics

Query Optimization:
  - Add EXPLAIN ANALYZE to slow queries
  - Create missing indexes (see PRP-007)
  - Optimize N+1 queries:
    - Use JOIN instead of multiple queries
    - Batch loading with dataloader pattern
  - Add query result caching (Redis)
  - Implement pagination for large lists
  - Use prepared statements (prevent SQL injection)

Monitoring:
  - Slow query log (> 100ms)
  - Connection pool saturation alerts
  - Query plan analysis
  - Index usage statistics
```

**Success Criteria:**
- ✅ < 50ms query latency (p95)
- ✅ Zero connection pool exhaustion
- ✅ Support 500 concurrent requests
- ✅ All queries under 200ms

---

### PRP-017: Horizontal Scaling Preparation
**Priority:** 🟢 Medium
**Effort:** 5 days
**Impact:** Path to 100k+ users

**Requirements:**
```yaml
Architecture Changes:
  1. Stateless Application Servers:
     - Move session storage to Redis
     - Remove in-memory caches (use Redis)
     - No local file storage (use S3/object storage)

  2. Load Balancer:
     - Tool: Traefik, Nginx, or cloud LB
     - Health check endpoint: /api/health
     - Session affinity: Not required (stateless)
     - WebSocket support for real-time features

  3. Shared File Storage:
     - Move git repositories to shared volume (NFS/EFS)
     - Or: Use S3 for git LFS objects
     - Database backups to object storage

  4. Background Job Queue:
     - Tool: Faktory, BullMQ, or Celery
     - Jobs:
       - AI executor tasks
       - Email sending
       - Report generation
       - Database cleanup
     - Worker scaling independent of web servers

Docker Compose (Multi-Instance):
  services:
    app:
      replicas: 3
      deploy:
        resources:
          limits: {cpus: '2', memory: '4G'}

    redis:
      image: redis:7-alpine
      volumes: [redis_data:/data]

    postgres:
      image: postgres:16-alpine
      volumes: [pg_data:/var/lib/postgresql/data]

    traefik:
      image: traefik:v3.0
      command:
        - --providers.docker
        - --entrypoints.web.address=:80

Kubernetes Manifests (Future):
  - Deployment with HPA (Horizontal Pod Autoscaler)
  - StatefulSet for PostgreSQL
  - Service and Ingress definitions
  - ConfigMaps and Secrets
```

**Success Criteria:**
- ✅ 3+ app instances running in parallel
- ✅ Zero downtime deployments (rolling update)
- ✅ Auto-scaling based on CPU/memory
- ✅ Support 10k concurrent users

---

## 🎯 Success Metrics & KPIs

### Development Velocity Metrics
| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Time to Deploy | 30 min | 3 min | CI/CD pipeline |
| Test Coverage | 20% | 75% | Coverage report |
| TypeScript Errors | 110 | 0 | ESLint output |
| CI Run Time | N/A | < 10 min | GitHub Actions |
| Deployment Frequency | Weekly | Daily | Git history |

### Platform Performance Metrics
| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| API Response Time (p95) | 800ms | 200ms | Prometheus |
| Database Query Time (p95) | 150ms | 50ms | Slow query log |
| Cache Hit Rate | 0% | 80% | Redis metrics |
| Concurrent Users | 10 | 500 | Load testing |
| Uptime | 99.0% | 99.9% | Monitoring |

### Business Metrics
| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| NORA Tool Success Rate | 60% | 95% | Tool execution logs |
| AI Executor Success Rate | 75% | 90% | Executor metrics |
| User Onboarding Time | 30 min | 5 min | Analytics |
| Feature Adoption Rate | N/A | 60% | Usage analytics |
| Customer NPS | N/A | 50+ | Survey |

---

## 📦 Deliverables Checklist

### Week 1-2: Developer Velocity
- [ ] GitHub Actions CI/CD pipeline
- [ ] Vitest test suite (5+ critical components)
- [ ] Zero TypeScript warnings
- [ ] Automated type generation hook
- [ ] Updated CLAUDE.md with new workflows

### Week 3-4: Core Stability
- [ ] Structured error handling in Rust
- [ ] Global error boundary in React
- [ ] Grafana monitoring dashboard
- [ ] PostgreSQL migration guide
- [ ] Database indexes optimized

### Week 5-7: AI Expansion
- [ ] NORA external integrations (5 tools)
- [ ] AI executor parallel execution
- [ ] Cost tracking per project
- [ ] Multi-agent orchestration framework
- [ ] ExecuteCode sandboxing

### Week 8-10: Enterprise Readiness
- [ ] Multi-tenancy architecture
- [ ] Custom RBAC roles
- [ ] SSO/SAML integration
- [ ] Enhanced audit logging
- [ ] Compliance reports

### Week 11-12: Scale & Performance
- [ ] Redis caching layer
- [ ] API rate limiting
- [ ] Connection pooling tuned
- [ ] Load balancer configuration
- [ ] Horizontal scaling tested

---

## 🚧 Risk Mitigation

### Technical Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| PostgreSQL migration complexity | High | Thorough testing, feature flag rollback |
| Redis cache invalidation bugs | Medium | Circuit breaker, graceful degradation |
| Multi-tenancy data leakage | Critical | Comprehensive integration tests, audit |
| SSO integration compatibility | Medium | Support top 5 IdPs, extensive testing |
| Horizontal scaling session issues | High | Stateless design, Redis session store |

### Resource Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| AI API cost overruns | High | Cost tracking, budget alerts, rate limits |
| Database storage growth | Medium | Archiving strategy, data retention policies |
| Insufficient team capacity | High | Prioritize ruthlessly, outsource non-core |
| Third-party API downtime | Medium | Fallback providers, graceful degradation |

### Business Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| Competitor launches similar features | Medium | Speed to market, unique differentiators |
| Enterprise sales cycle longer than expected | High | Start with SMB segment, freemium model |
| Self-hosted preference limits SaaS growth | Medium | Hybrid model, managed self-hosted option |

---

## 📈 Go-to-Market Strategy

### Phase V Launch Plan

**Week 8: Beta Program**
- Invite 10 design partners (AI dev agencies)
- Collect feedback on enterprise features
- Iterate based on real-world usage

**Week 10: Public Beta**
- Launch on Product Hunt, Hacker News
- Positioning: "AI-Native Project Management for Dev Teams"
- Content: Demo video, blog post, comparison table
- Free tier: 1 org, 5 users, 10 projects

**Week 12: General Availability**
- Announce 1.0 release
- Pricing tiers:
  - **Free:** $0 (1 org, 5 users, 10 projects)
  - **Pro:** $29/user/month (unlimited projects, SSO, advanced RBAC)
  - **Enterprise:** Custom (multi-tenancy, SLA, priority support)
- Sales outreach to 50 target accounts

### Marketing Channels
1. **Technical Content:**
   - "Building an AI-Native Project Management Platform" (blog series)
   - "How We Scaled to 10k Users" (technical deep-dive)
   - Open-source MCP server examples

2. **Community Building:**
   - Discord server for users and developers
   - Monthly office hours with founders
   - Customer success stories (case studies)

3. **Partnerships:**
   - Anthropic MCP marketplace listing
   - GitHub Marketplace app
   - Cloudflare Workers integration

---

## 🎓 Team & Resource Planning

### Required Roles (90-day plan)
- **1 Senior Backend Engineer (Rust):** PRP-005, 007, 011, 014, 016
- **1 Senior Frontend Engineer (React/TS):** PRP-002, 003, 012
- **1 DevOps Engineer:** PRP-001, 006, 015, 017
- **1 AI/ML Engineer:** PRP-008, 009, 010
- **0.5 Technical Writer:** Documentation, API docs, guides
- **0.5 Product Manager:** Roadmap, prioritization, stakeholder management

### Budget Estimate
| Category | Monthly | 90-Day Total |
|----------|---------|--------------|
| Team Salaries | $60k | $180k |
| Infrastructure (AWS/DO) | $2k | $6k |
| External APIs (OpenAI, etc.) | $1k | $3k |
| Tools (Grafana Cloud, etc.) | $500 | $1.5k |
| **Total** | **$63.5k** | **$190.5k** |

---

## ✅ Phase V Exit Criteria

### Must-Have (Blocking GA Release)
- ✅ Zero critical bugs
- ✅ 75% test coverage achieved
- ✅ All enterprise features functional
- ✅ Performance targets met (p95 < 200ms)
- ✅ Security audit passed
- ✅ Documentation complete
- ✅ 10 beta customers successfully onboarded

### Nice-to-Have (Post-GA)
- ⏭️ Multi-agent orchestration (move to Phase VI if needed)
- ⏭️ Kubernetes manifests
- ⏭️ Mobile app
- ⏭️ Advanced analytics dashboard

---

## 🎯 Phase VI Preview (Q2 2026)

**Theme:** Ecosystem & Intelligence

1. **Marketplace:**
   - Custom executor marketplace
   - Template marketplace
   - Integration directory

2. **AI Intelligence:**
   - Project-specific AI model fine-tuning
   - Predictive task estimation
   - Automated code review quality scores
   - Team productivity insights

3. **Collaboration:**
   - Real-time collaborative editing
   - Cursor presence
   - Voice/video calls in-app
   - Whiteboard integration

4. **Blockchain Expansion:**
   - Smart contract deployment tracking
   - On-chain milestone payments
   - Token-gated project access
   - DAO governance integration

---

## 📞 Success Checkpoints

### Week 2 Checkpoint
- ✅ CI/CD pipeline operational
- ✅ First 20 tests passing
- ✅ TypeScript warnings reduced by 50%

### Week 4 Checkpoint
- ✅ Monitoring dashboard live
- ✅ Error handling refactored
- ✅ Database optimizations deployed

### Week 7 Checkpoint
- ✅ All NORA tools functional
- ✅ Multi-agent framework tested
- ✅ Cost tracking operational

### Week 10 Checkpoint
- ✅ Multi-tenancy in production
- ✅ SSO working with 3 providers
- ✅ Beta customers onboarded

### Week 12 Checkpoint (Final)
- ✅ Performance targets achieved
- ✅ Horizontal scaling validated
- ✅ 1.0 GA launched

---

## 🚀 Next Actions

### Immediate (This Week)
1. [ ] Review and approve Phase V PRP
2. [ ] Assign engineering team to sprints
3. [ ] Setup GitHub Projects board for PRP tracking
4. [ ] Schedule daily standups
5. [ ] Create PRP-001 (CI/CD) implementation ticket

### Week 1 Kickoff
1. [ ] All-hands Phase V kickoff meeting
2. [ ] Engineer onboarding to PRP
3. [ ] Setup development environments
4. [ ] Create staging environment
5. [ ] Begin Sprint 1 work

---

**Prepared by:** Claude Code
**Approved by:** [Pending Review]
**Status:** 🟢 Ready for Implementation
**Next Review:** Week 2 Checkpoint (Dec 25, 2025)

---

*This Priority Release Plan is a living document. Update weekly based on progress and learnings.*

---

## 🔄 Workflow Session Notes — April 2026

**Session Date:** 2026-04-03 / 2026-04-04
**Context:** Nora Orchestration & Real-Time Client Workflow sprint

### What We Were Building

The core goal of this session was to close the loop on two end-to-end workflows that Bodhi/admin should be able to trigger from any channel (phone, SMS, Discord, dashboard):

1. **"Research this company and build a profile"** — Nora dispatches Scout, Scout scrapes the prospect's website, results flow into the CRM as a company record + brand guide.
2. **"Spin up a landing page for this client"** — Research → generate site code → push to GitHub → deploy to Vercel → return preview URL.

### What Was Completed This Session

#### Scout Scraping Capability (`crates/nora/`)
Scout previously had only `SearchWeb` (Exa) and `FetchWebPage` (static HTTP, no JS). Three problems blocked real prospect research:
- Wix/React/SPA sites returned empty HTML on static fetch
- No structured asset extraction (logos, colors, contact info, social links)
- Scrape results written directly to `persons.intelligence_raw` with no CRM write-back

**Changes made:**
- `tools/types.rs` — Added `ScrapePage { url, use_js, extract_assets }` variant
- `tools/schemas.rs` — Added `scrape_page` to both `get_openai_tool_schemas()` (Nora) and `get_user_tool_schemas()`
- `tools/parse.rs` — Parse `"scrape_page"` tool call from Claude's function calling
- `tools/execute_impl.rs` — Full `execute_scrape_page()`: Playwright → static fallback, extracts title/description/og:image/all-images/logos/favicon/emails/phones/social links/hex brand colors from CSS
- `tools/user_scoped.rs` — Stub returning informative error (executive-only)
- `execution/research.rs` — `ScrapedPage` struct, `extract_page_assets()` helper, `ResearchTools::scrape_url()`, and new "Website Scrape" / "Asset Collection" / "Brand Audit" stage handler in `ResearchExecutor` that auto-detects URL from context, renders via Playwright if needed, and synthesises brand intelligence via LLM

**Modi Nochi case study (same session):** Validated the workflow end-to-end manually — audited modinochi.com, created PCG company/client/person records, generated brand guide, then rebuilt the entire website as Next.js 16 with real images/content. Deployed to Vercel at `modi-nochi-eta.vercel.app`, pushed to public GitHub at `https://github.com/Bodhi1998/modi-nochi`.

#### Company model fix (`crates/db/src/models/company.rs`)
- `find_by_id()` — Fixed TEXT vs BLOB UUID mismatch: `WHERE id = ? OR lower(hex(id)) = lower(?)`
- `update()` — Restructured WHERE clause that was producing invalid SQL in same dual-format pattern

### Remaining Gaps — What Needs to Be Built Next

#### Gap 1: Nora → CRM Bridge (HIGH PRIORITY)
Nora has no tools to read or write CRM records directly. Scout research results don't automatically create company/person/client records. Required tools:
- `lookup_or_create_company(name, website)` → `Company::find_or_create()`
- `lookup_or_create_person(name, company_name)` → `Person::find_or_create()`
- `create_client(org_id, company_id)` → client record in `clients` table
- `trigger_brand_research(org_id)` → calls existing `POST /api/organizations/:id/brand-research`
- `create_intake_item(person_id, notes)` → intake pipeline entry

Without these, the SMS/phone intake flow ("Nora, research Modi Nochi") cannot automatically create a client profile. The user must visit the dashboard and trigger it manually.

#### Gap 2: Scout → Automatic CRM Write-back (HIGH PRIORITY)
After a "Website Scrape" stage completes, Scout should:
1. Call `Company::find_or_create()` with scraped name/website
2. Write intelligence fields (`intelligence_summary`, `intelligence_raw`, `intelligence_status='done'`)
3. Store logos/images as `ProjectKnowledgeSource` artifacts

Currently: results sit in `persons.intelligence_raw` only.

#### Gap 3: Auri Execution Environment (MEDIUM PRIORITY — enables landing page spin-up)
`ExecuteCode` in `execute_impl.rs` returns "requires sandboxed environment" (line 3925). Auri is recognized as `"CLAUDE_CODE"` in the agent routing but has no actual execution path. Required:
- Sandboxed shell container (Docker) with `git`, `gh`, `vercel`, `node`, `cargo` available
- `ExecuteShellCommand(command, cwd, timeout)` tool that pipes to that container
- Nora dispatches Auri with a `create_next_app` + `vercel deploy` workflow
- Returns preview URL as task output

This is what closes the "spin up a landing page in real time" use case.

#### Gap 4: Channel Round-Trips
| Channel | Inbound | Outbound | Gap |
|---------|---------|----------|-----|
| Dashboard | ✅ | ✅ | None |
| Phone (voice) | ✅ | ✅ | None |
| SMS | ✅ `twilio/sms.rs` | ✅ | Nora tool calls during SMS not streaming back to conversation |
| Discord | ✅ `routes/discord.rs` | ✅ `send_discord_message` tool | Session continuity — no persistent thread-to-session mapping |

#### Gap 5: Nora Admin Context (LOW — cleanup)
Nora's RBAC enforcement (admin/Bodhi gets full tool access vs. client gets limited) should be derived from `AccessContext.is_admin` at the agent level. Currently the system prompt enforces this at the text level but the tool schema returned is the same regardless of user role.

### Onboarding Notes for New Team Members

**Workflow to understand first:** `crates/nora/src/agent.rs` → `crates/nora/src/tools/execute_impl.rs` → `crates/nora/src/execution/` (engine, research, crawler, artifact)

**The intake pipeline:** `POST /api/call-intake/email` → auto-creates tasks from trusted sender emails. See `memory/intake-pipeline.md`.

**Key quirk:** UUID storage is inconsistent — some rows are TEXT (dashes), some are BLOB (hex). All queries on `companies`, `agents`, `deliverables` should use `WHERE id = ? OR lower(hex(id)) = lower(?)`. See `crates/db/src/models/company.rs` for the canonical pattern.

**Branch status:** This session's work (Scout scraping, company model fixes) is uncommitted on `sloperation329`. To be merged to `main` before new team member onboarding.
