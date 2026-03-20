# Legal, Compliance, Security & Governance Research

**Document Type**: Research — 5-Year Product Roadmap
**Date**: 2026-03-19
**Version**: 1.0
**Author**: Roadmap Research Sprint
**Status**: Internal Planning Document

---

## Executive Summary

ORCHA is an AI orchestration SaaS platform targeting enterprise customers. It manages AI coding agents, CRM data, project management, and workflow automation. This document surveys the legal, compliance, security, and governance landscape that ORCHA must navigate across its six roadmap stages (Stage 0: Dogfood through Stage 5: Full Sovereignty).

Key findings:

1. **Data privacy (GDPR/CCPA) is a Stage 1 prerequisite** for any EU/California pilot customers. DPA templates and privacy policies must exist before first external user.
2. **EU AI Act classification** likely places ORCHA as a "limited risk" system, requiring transparency obligations but not the heavy burden of "high risk" classification — unless health/HR clients are targeted.
3. **SOC 2 Type II** is the most impactful certification for enterprise sales (Stage 2-3 timeline, 9-18 months, $30-100K).
4. **Multi-tenant security** is the highest-priority technical gap — current org-scoped queries are a start but insufficient for enterprise isolation.
5. **Open source compliance** is low-effort, high-value — SBOM generation and license scanning should start at Stage 0.
6. **All recommended tooling** uses permissive licenses (Apache 2.0 or MIT).

---

## Table of Contents

1. [Data Privacy Regulations](#1-data-privacy-regulations)
2. [AI-Specific Regulations](#2-ai-specific-regulations)
3. [Security Architecture](#3-security-architecture)
4. [Multi-Tenant Security](#4-multi-tenant-security)
5. [Open Source Compliance](#5-open-source-compliance)
6. [Insurance and Liability](#6-insurance-and-liability)
7. [Compliance Tooling](#7-compliance-tooling)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [Budget Summary](#9-budget-summary)
10. [Risk Register](#10-risk-register)

---

## 1. Data Privacy Regulations

### 1.1 GDPR (EU General Data Protection Regulation)

**Applicability**: Applies the moment ORCHA processes personal data of any EU resident — even if ORCHA has no EU entity. CRM contacts, agent-processed user data, and employee information of EU pilot clients all trigger GDPR.

| Requirement | Description | ORCHA Impact | Priority | Stage |
|-------------|-------------|--------------|----------|-------|
| **Lawful basis** | Must establish legal basis for each processing activity (consent, legitimate interest, contract) | CRM contact data needs explicit basis; agent data processing needs legitimate interest assessment | P0 | 1 |
| **Data residency** | Personal data transfers outside EU require adequacy decision or Standard Contractual Clauses (SCCs) | If hosting in US, need SCCs with EU clients. EU hosting option removes this burden entirely. | P1 | 2 |
| **Right to erasure (Art. 17)** | Data subjects can request deletion of all their personal data | Must implement cascading delete across all tables: CRM contacts, execution artifacts, audit logs, agent context. Current SQLite schema has no cascade-delete mechanism for cross-table personal data. | P0 | 1 |
| **Right to portability (Art. 20)** | Data subjects can request machine-readable export | Need structured export endpoint (JSON/CSV) for all personal data linked to an individual | P1 | 2 |
| **Data Protection Impact Assessment (DPIA)** | Required for high-risk processing (profiling, automated decision-making) | AI agent processing of CRM data likely triggers DPIA requirement. Must document before processing begins. | P0 | 1 |
| **DPO (Data Protection Officer)** | Required if: (a) core activity is large-scale processing, (b) public authority, (c) systematic monitoring | SaaS platform processing client data at scale will likely require DPO by Stage 2. Can be external/contracted initially. | P1 | 2 |
| **Breach notification** | 72-hour notification to supervisory authority; affected individuals if high risk | Need incident response plan and breach detection monitoring | P1 | 2 |
| **Records of Processing (Art. 30)** | Maintain records of all processing activities | Document what data is collected, why, retention period, who has access | P0 | 1 |

**Key considerations for ORCHA**:
- CRM data contains personal data (names, emails, phone numbers, job titles) — this is core to the product
- AI agents processing CRM data constitutes "automated processing" under GDPR
- Workflow execution artifacts may contain personal data extracted from data sources
- Agent context/memory may retain personal data across sessions — must be purgeable

**Pros of early GDPR compliance**:
- Opens EU market (significant enterprise SaaS TAM)
- Framework naturally improves data architecture (explicit data flows, retention policies)
- GDPR compliance often satisfies or partially satisfies other privacy frameworks

**Cons / challenges**:
- Significant engineering effort for right-to-erasure across 202+ migrations and 138 models
- Data residency requirements may force EU hosting infrastructure early
- DPO cost ($2-5K/month external, or internal hire)

**Budget**: $5-15K for initial legal review + DPA template; $2-5K/mo ongoing DPO; $10-20K engineering for erasure/portability endpoints

### 1.2 CCPA/CPRA (California Consumer Privacy Act / California Privacy Rights Act)

**Applicability**: Applies if ORCHA (a) does business in California, AND meets one of: $25M+ annual revenue, processes 100K+ consumers, or derives 50%+ revenue from selling personal information. At Stage 0-1, ORCHA likely falls below thresholds. Becomes relevant at Stage 2-3.

| Requirement | Description | Priority | Stage |
|-------------|-------------|----------|-------|
| **Right to know** | Consumers can request what personal info is collected | P1 | 2 |
| **Right to delete** | Similar to GDPR erasure | P1 | 2 |
| **Right to opt-out of sale** | "Do Not Sell My Personal Information" | P2 | 2 |
| **Data minimization (CPRA)** | Collect only what's necessary for stated purpose | P1 | 2 |
| **Privacy policy** | Must disclose collection categories, purposes, third parties | P0 | 1 |

**Key considerations**:
- If GDPR compliance is implemented first, CCPA/CPRA is ~70% covered
- CPRA added the California Privacy Protection Agency (CPPA) with enforcement authority
- "Business purpose" includes AI/ML processing — agent operations qualify

**Budget**: Incremental $3-5K legal review on top of GDPR work; minimal additional engineering if GDPR endpoints exist

### 1.3 SOC 2 Type II Certification

**What it is**: AICPA Trust Services Criteria assessment — the gold standard for SaaS enterprise sales. Type I = point-in-time control design. Type II = controls operating effectively over 6-12 months.

| Aspect | Detail |
|--------|--------|
| **Timeline** | Type I: 3-6 months prep + audit. Type II: 6-12 month observation + audit. Total: 12-18 months from start to Type II report. |
| **Cost** | $30-100K total (auditor fees $15-50K, tooling $5-20K/year, remediation labor) |
| **Trust Service Criteria** | Security (required), Availability, Processing Integrity, Confidentiality, Privacy (choose which apply) |
| **Observation period** | Minimum 6 months of evidence collection for Type II |
| **Annual renewal** | Yes — re-audit every 12 months ($15-30K/year ongoing) |

**Roadmap fit**:
- **Stage 1**: Begin SOC 2 readiness assessment. Implement foundational controls (access management, change management, incident response).
- **Stage 2**: Engage auditor for Type I assessment. Begin Type II observation period.
- **Stage 3**: Achieve Type II report. Use for enterprise sales.

**Pros**:
- Required by ~60% of enterprise procurement teams
- Forces good security hygiene that benefits the product
- Competitive advantage at SaaS launch (most startups defer until forced)

**Cons**:
- Expensive for a 3-5 person team
- Significant process overhead (documented change management, access reviews, etc.)
- Type II requires 6+ months of sustained evidence — cannot rush

**Recommended approach**: Use a compliance automation platform (Vanta, Drata, or Secureframe — all SaaS, $10-20K/year) to automate evidence collection. This cuts prep time by ~50%.

**Priority**: P1 | **Stage**: Begin Stage 1, achieve Stage 3

### 1.4 ISO 27001 Certification

**What it is**: International standard for Information Security Management Systems (ISMS). Broader than SOC 2 — covers organizational security management, not just controls.

| Aspect | Detail |
|--------|--------|
| **Timeline** | 6-12 months to implement ISMS + audit. Certification valid for 3 years with annual surveillance audits. |
| **Cost** | $20-50K initial certification; $10-20K/year surveillance |
| **Scope** | Can be scoped to specific systems/processes (recommended: scope to ORCHA SaaS platform only) |
| **Overlap with SOC 2** | ~60-70% control overlap. Doing both simultaneously is efficient. |

**Roadmap fit**:
- **Stage 3-4**: Pursue after SOC 2 Type II. ISO 27001 is more valued in EU/APAC markets.
- Consider implementing ISMS framework during SOC 2 prep to minimize duplicate effort.

**Pros**:
- Required for many EU government and large enterprise contracts
- 3-year certification cycle (vs SOC 2 annual) reduces audit fatigue
- International recognition

**Cons**:
- Heavier process burden than SOC 2 (full ISMS documentation, risk treatment plans)
- Less recognized in US enterprise than SOC 2
- Requires management commitment and dedicated security function

**Priority**: P2 | **Stage**: 3-4

### 1.5 HIPAA Considerations

**Applicability**: Only if ORCHA processes Protected Health Information (PHI) for covered entities (healthcare providers, insurers, clearinghouses) or their business associates.

| Requirement | Description | Priority | Stage |
|-------------|-------------|----------|-------|
| **Business Associate Agreement (BAA)** | Must sign BAA with any covered entity client | P0 (if health sector) | 2+ |
| **Technical safeguards** | Encryption at rest + transit, access controls, audit trails, integrity controls | P0 (if health sector) | 2+ |
| **Administrative safeguards** | Security officer designation, workforce training, contingency plans | P1 (if health sector) | 2+ |
| **Breach notification** | 60-day notification to HHS, affected individuals, and media (if 500+ affected) | P0 (if health sector) | 2+ |

**Recommendation**: Do NOT pursue HIPAA compliance proactively. Only invest if a specific high-value healthcare client requires it. HIPAA adds significant infrastructure requirements (dedicated encryption, audit, BAA management) that are premature for Stage 0-2.

**If needed, approximate cost**: $20-40K initial compliance engineering + $5-10K legal + ongoing $10-15K/year for compliance monitoring.

**Priority**: P2 (only if health sector targeted) | **Stage**: 3+

---

## 2. AI-Specific Regulations

### 2.1 EU AI Act

**Status**: Entered into force August 1, 2024. Phased implementation through August 2, 2027.
- **February 2, 2025**: Prohibited AI practices ban takes effect
- **August 2, 2025**: GPAI (General Purpose AI) obligations apply
- **August 2, 2026**: Most provisions including high-risk classification apply
- **August 2, 2027**: High-risk AI systems in Annex I products must comply

**ORCHA Classification Analysis**:

| Risk Category | Criteria | ORCHA Applicability |
|---------------|----------|---------------------|
| **Unacceptable risk** | Social scoring, real-time biometric identification, manipulation | NOT applicable |
| **High risk** | AI in employment, creditworthiness, education, law enforcement, critical infrastructure | POSSIBLY applicable if clients use ORCHA for: HR/recruitment decisions, credit assessment, or critical infrastructure management |
| **Limited risk** | AI systems interacting with people, generating content, detecting emotions | LIKELY applicable — AI agents interact with users, generate content, process CRM data |
| **Minimal risk** | AI spam filters, recommendation systems | Some features (content suggestions, task routing) fall here |

**Most likely classification: Limited Risk** with potential **High Risk** exposure depending on client use cases.

| Obligation (Limited Risk) | Description | Priority | Stage |
|---------------------------|-------------|----------|-------|
| **Transparency** | Users must be informed they are interacting with AI | P0 | 0 |
| **Content marking** | AI-generated content must be identifiable as such | P1 | 1 |
| **Technical documentation** | Document AI system capabilities, limitations, intended purpose | P1 | 2 |

| Obligation (High Risk, if applicable) | Description | Priority | Stage |
|----------------------------------------|-------------|----------|-------|
| **Conformity assessment** | Third-party assessment before market placement | P1 | 3 |
| **Risk management system** | Ongoing risk identification and mitigation | P1 | 3 |
| **Data governance** | Training data quality, bias testing, representativeness | P1 | 3 |
| **Human oversight** | Meaningful human control over AI decisions | P0 | 1 |
| **Accuracy, robustness, cybersecurity** | Documented performance metrics | P1 | 2 |
| **Registration** | Register in EU database of high-risk AI systems | P1 | 3 |

**GPAI Provider Obligations** (applies to Anthropic/Google, not directly to ORCHA, but ORCHA must be aware):
- Upstream LLM providers must provide technical documentation and comply with EU copyright law
- ORCHA as a "deployer" must ensure proper use and monitor for risks

**Key considerations for ORCHA**:
- Agent interactions with CRM contacts should clearly identify AI involvement
- Workflow-generated content should carry AI provenance metadata
- If enterprise clients use ORCHA for employment-related decisions (HR workflows), high-risk obligations apply
- ORCHA's "trust gates" (human approval for high-risk actions) already partially satisfy human oversight requirements

**Recommendation**: Implement transparency markers (Stage 0-1) proactively. Defer high-risk conformity assessment until enterprise client demand materializes (Stage 3+). Build the audit trail infrastructure now — it's useful regardless of classification.

### 2.2 AI Transparency Requirements

| Requirement | Implementation | Priority | Stage |
|-------------|---------------|----------|-------|
| **AI interaction disclosure** | Clear labeling when user is interacting with an agent (Nora, Scout, etc.) vs. human | P0 | 0 |
| **AI-generated content marking** | Metadata tag on all agent-produced artifacts (documents, emails, code, CRM entries) | P1 | 1 |
| **Decision explanation** | For automated decisions (deal scoring, task routing, content recommendations), provide reasoning | P1 | 2 |
| **Capability disclosure** | Document what each agent can/cannot do in user-facing docs | P1 | 2 |
| **Data usage disclosure** | Inform users what data agents access and how it's used | P0 | 1 |

**Current state**: ORCHA's named agent system (Nora, Scout, etc.) inherently provides some transparency — users know they're interacting with AI. The structured response envelope (status, message, artifacts) provides a natural place for provenance metadata.

### 2.3 Model Audit Trails and Explainability

| Component | Description | Priority | Stage |
|-----------|-------------|----------|-------|
| **Agent action log** | Record every agent action: tool called, input, output, model used, tokens consumed, timestamp | P0 | 0 |
| **Decision ledger** | For each agent decision: reasoning chain, confidence score, alternatives considered | P1 | 2 |
| **Model versioning** | Track which model version produced each output (model ID, provider, temperature, etc.) | P1 | 1 |
| **Reproducibility** | Store sufficient context to reproduce agent decisions (prompt + model + params) | P2 | 3 |
| **Audit query API** | Allow administrators to query: "What did agent X do with contact Y's data between dates A and B?" | P1 | 2 |

**Current state**: ORCHA tracks per-task cost (input/output/cached tokens, model, wall-clock time) per the CAPO system. This is a strong foundation but needs to be extended to a full audit trail.

**Key gap**: Agent context/memory may contain stale or inaccurate personal data that influenced decisions. Audit trails must capture the context state at decision time, not just the current state.

### 2.4 Bias Detection and Mitigation

| Requirement | Description | Priority | Stage |
|-------------|-------------|----------|-------|
| **CRM processing bias** | Monitor for demographic bias in deal scoring, contact prioritization, lead routing | P1 | 2 |
| **Content generation bias** | Evaluate agent-generated content for harmful stereotypes, exclusionary language | P2 | 3 |
| **Model selection bias** | Ensure cost-tiered routing doesn't systematically provide lower quality service to certain data patterns | P2 | 3 |
| **Bias testing framework** | Periodic evaluation of agent outputs across demographic dimensions | P2 | 3 |

**Practical approach**: At Stage 0-1, document known LLM biases and inform users. At Stage 2+, implement statistical monitoring of agent decision distributions. At Stage 3+, formal bias testing before major releases.

### 2.5 US Executive Orders on AI Safety

**EO 14110 (October 2023)** — "Safe, Secure, and Trustworthy AI Development and Use":
- Primarily targets foundation model developers (Anthropic, OpenAI, Google) — not deployers like ORCHA
- However, establishes norms around AI safety testing, red-teaming, and reporting
- NIST AI Risk Management Framework (AI RMF) is the referenced standard

**Practical impact on ORCHA**:
- Minimal direct regulatory obligation at current scale
- Adopt NIST AI RMF as a voluntary framework for Stage 2+ (demonstrates due diligence)
- Monitor for new legislation — several AI bills are progressing in Congress (as of March 2026)
- State-level AI laws (Colorado AI Act, Illinois AI Video Interview Act) may apply depending on client use cases

**Priority**: P2 | **Stage**: 2+ (voluntary adoption of NIST AI RMF)

---

## 3. Security Architecture

### 3.1 OWASP Top 10 for AI/LLM Applications (2025)

The OWASP Top 10 for LLM Applications provides a framework for securing AI-powered platforms. Mapped to ORCHA:

| # | Vulnerability | ORCHA Relevance | Current Mitigation | Gap | Priority | Stage |
|---|---------------|-----------------|--------------------|----|----------|-------|
| LLM01 | **Prompt Injection** | Agents process user-provided and CRM data that could contain adversarial prompts | None — agents receive raw CRM/task data | HIGH: Need input sanitization + prompt hardening for all agent contexts | P0 | 0 |
| LLM02 | **Insecure Output Handling** | Agent outputs are rendered in UI, stored in DB, and may be used in downstream workflows | Structured response envelope provides some containment | MEDIUM: Need output validation before DB writes and UI rendering | P0 | 1 |
| LLM03 | **Training Data Poisoning** | ORCHA uses third-party models — not directly applicable to training but RAG/context poisoning applies | Sovereign data ingestion from Dropbox is one-way (read-only) | LOW: Data source integrity verification for workflow inputs | P1 | 2 |
| LLM04 | **Model Denial of Service** | Adversarial inputs causing excessive token consumption or model hangs | Per-task cost tracking (CAPO) | MEDIUM: Need token budget limits per agent invocation, circuit breakers | P0 | 1 |
| LLM05 | **Supply Chain Vulnerabilities** | Dependencies on multiple LLM providers, MCP servers, npm/cargo packages | cargo-audit + npm audit in CI | MEDIUM: Need SBOM, dependency pinning, provider failover | P1 | 1 |
| LLM06 | **Sensitive Information Disclosure** | Agents may leak cross-tenant data, API keys, or internal system details in responses | Org-scoped queries | HIGH: Need output filtering for PII/secrets, tenant isolation in agent context | P0 | 1 |
| LLM07 | **Insecure Plugin Design** | MCP tools are essentially plugins — a compromised tool could access unauthorized data | MCP tools have defined schemas | MEDIUM: Need per-tool permission boundaries, least-privilege access | P1 | 2 |
| LLM08 | **Excessive Agency** | Agents with too many capabilities or insufficient guardrails | Trust gates planned (Stage 3) | HIGH: Need permission tiers before first external user | P0 | 1 |
| LLM09 | **Overreliance** | Users trusting agent outputs without verification | Named agents provide some UX cues | LOW: Documentation + UI warnings for high-stakes operations | P2 | 2 |
| LLM10 | **Model Theft** | Not directly applicable — ORCHA doesn't host models | N/A | N/A | — | — |

**Top 3 actions for Stage 0-1**:
1. Implement prompt injection defenses (input sanitization, system prompt hardening, output filtering)
2. Add token budget limits and circuit breakers per agent invocation
3. Ensure tenant context isolation — agents for Org A must never receive Org B data in context

### 3.2 Supply Chain Security

| Tool/Practice | Description | License | Priority | Stage |
|---------------|-------------|---------|----------|-------|
| **cargo-audit** | Checks Rust dependencies against RustSec Advisory Database | MIT/Apache 2.0 | P0 | 0 (already in CI) |
| **npm audit** | Checks Node dependencies against npm advisory database | Built-in | P0 | 0 (already in CI) |
| **SBOM generation** | Machine-readable inventory of all dependencies | N/A (process) | P1 | 1 |
| **cargo-sbom** | Generate CycloneDX/SPDX SBOM from Cargo.lock | Apache 2.0 | P1 | 1 |
| **syft** (Anchore) | Multi-ecosystem SBOM generation (Rust + Node + containers) | Apache 2.0 | P1 | 1 |
| **Dependabot** | Automated dependency update PRs | Free (GitHub native) | P0 | 0 |
| **Trivy** (Aqua Security) | Vulnerability scanner for dependencies, containers, IaC | Apache 2.0 | P1 | 1 |
| **Snyk** | Commercial vuln scanner + license compliance | Freemium (proprietary) | P2 | 2 |

**Current state**: cargo-audit and npm audit run in CI. No SBOM generation, no Dependabot, no container scanning.

**Recommended actions**:

| Action | Effort | Priority | Stage |
|--------|--------|----------|-------|
| Enable Dependabot for Cargo.toml + package.json | 1 hour | P0 | 0 |
| Add `cargo-sbom` to CI, output CycloneDX JSON | 2 hours | P1 | 1 |
| Add Trivy container scan to CI | 4 hours | P1 | 1 |
| Add syft to CI for comprehensive SBOM | 2 hours | P1 | 1 |
| Pin all dependency versions (remove `^` ranges in package.json) | 2 hours | P1 | 1 |
| Vendor critical dependencies or use cargo-vendor for air-gapped builds | 1 day | P2 | 3 |

**License check considerations**:
- Snyk has a free tier but is proprietary — use Trivy (Apache 2.0) as primary scanner
- For license scanning specifically, use `licensee` or `scancode-toolkit` (both Apache 2.0) rather than Snyk's license feature
- Trivy can also scan for license issues in addition to vulnerabilities

### 3.3 Dependency Scanning Deep Dive

| Tool | Ecosystems | License | SBOM | Vuln Scan | License Scan | Container Scan | Pros | Cons |
|------|-----------|---------|------|-----------|-------------|----------------|------|------|
| **Trivy** | Rust, Node, Python, Go, containers, IaC | Apache 2.0 | Yes (CycloneDX, SPDX) | Yes | Yes | Yes | All-in-one, fast, widely adopted, permissive license | Less Rust-specific depth than cargo-audit |
| **cargo-audit** | Rust only | MIT/Apache 2.0 | No | Yes | No | No | Deep Rust advisory coverage, already in CI | Rust only |
| **Dependabot** | All major | Free (GitHub) | No | Yes | No | No | Zero config, auto-PRs, free | No SBOM, no license check |
| **Snyk** | All major | Freemium/proprietary | Yes | Yes | Yes | Yes | Excellent UI, deep analysis | Proprietary, expensive at scale |
| **Grype** (Anchore) | All major | Apache 2.0 | Via syft | Yes | No | Yes | Fast, pairs with syft | No license scanning |

**Recommended stack** (all permissive-licensed):
1. **Trivy** — primary vulnerability + license + container scanner
2. **cargo-audit** — Rust-specific advisory database (keep existing)
3. **syft** — SBOM generation across all ecosystems
4. **Dependabot** — automated dependency updates
5. **scancode-toolkit** — deep license compliance scanning when needed

### 3.4 Runtime Security

| Mechanism | Description | Priority | Stage |
|-----------|-------------|----------|-------|
| **seccomp profiles** | Restrict system calls available to containers | P1 | 2 |
| **AppArmor/SELinux** | Mandatory access control for container processes | P1 | 2 |
| **Read-only filesystems** | Mount container filesystems as read-only where possible | P1 | 2 |
| **Non-root containers** | Run all containers as non-root users | P0 | 0 (partially done — Dockerfile uses appuser:1001) |
| **Network policies** | Restrict container-to-container network access | P1 | 2 |
| **Resource limits** | CPU/memory limits on all containers, especially agent execution | P0 | 1 |
| **Falco** (Sysdig) | Runtime threat detection for containers (Apache 2.0) | P2 | 3 |

**Current state**: Main Dockerfile runs as non-root (appuser:1001). No seccomp, no AppArmor, no network policies, no resource limits documented.

**Priority actions**:
- Stage 0: Verify all Docker services run as non-root
- Stage 1: Add resource limits to docker-compose (CPU, memory, PID limits)
- Stage 2: Implement seccomp profiles and network policies
- Stage 3: Deploy Falco for runtime detection

### 3.5 API Security

| Mechanism | Description | Current State | Priority | Stage |
|-----------|-------------|---------------|----------|-------|
| **Rate limiting** | Prevent abuse and DoS | Only on Nora routes (tower_governor) | P0 | 0 |
| **API key management** | Secure issuance, rotation, and revocation of API keys | No API key system | P0 | 1 |
| **OAuth 2.0/OIDC** | Standard authentication/authorization | GitHub OAuth device flow only | P0 | 1 |
| **CORS** | Cross-origin request restrictions | Configured in Axum | P1 | 0 |
| **Input validation** | Schema validation on all API inputs | No validation framework | P0 | 0 |
| **Request signing** | HMAC or JWT for webhook/API authentication | HMAC planned for webhooks | P1 | 1 |
| **API versioning** | Stable API contracts for external consumers | None | P1 | 2 |
| **Throttling per tenant** | Per-org rate limits to prevent noisy neighbor | None | P0 | 1 |

**ORCHA-specific API risks**:
- MCP tool endpoints are effectively APIs — they need the same security posture
- Agent execution endpoints can trigger expensive LLM calls — rate limiting is critical for cost control
- SSE endpoints maintain long-lived connections — need connection limits per user/tenant

**Recommended auth stack**:
- **Stage 0-1**: Expand existing GitHub OAuth to include email/password + OAuth (Google, Microsoft)
- **Stage 2**: Keycloak (Apache 2.0) for full OIDC/SAML, SSO for enterprise clients
- **Stage 3+**: API key management system for programmatic access (webhook triggers, CI/CD integrations)

### 3.6 Encryption Requirements

| Layer | Requirement | Current State | Priority | Stage |
|-------|-------------|---------------|----------|-------|
| **In transit** | TLS 1.2+ for all connections | Cloudflare Tunnel handles TLS termination | P0 | 0 (covered) |
| **At rest (database)** | Encrypt database files | SQLite: no encryption. SQLCipher available but adds complexity. PostgreSQL: transparent data encryption. | P1 | 2 |
| **At rest (files)** | Encrypt uploaded files and artifacts | No file encryption | P1 | 2 |
| **At rest (backups)** | Encrypt backup archives | Docker backup service — encryption status unknown | P1 | 1 |
| **Key management** | Centralized key management with rotation | None | P1 | 2 |
| **Client-side encryption** | End-to-end encryption for sensitive fields | None | P2 | 4 |

**PostgreSQL migration advantage**: PostgreSQL supports Transparent Data Encryption (TDE) and has mature encryption extensions (pgcrypto). This is another reason to prioritize the PostgreSQL migration.

---

## 4. Multi-Tenant Security

### 4.1 Tenant Data Isolation Patterns

| Pattern | Description | Isolation Level | Cost | ORCHA Fit |
|---------|-------------|-----------------|------|-----------|
| **Shared schema, row-level isolation** | All tenants share tables; `organization_id` on every row + enforced in every query | Low | Low | CURRENT STATE — org-scoped queries throughout |
| **Schema-per-tenant** | Each tenant gets own schema within shared database | Medium | Medium | Good fit for PostgreSQL migration — schema isolation with shared infra |
| **Database-per-tenant** | Each tenant gets own database | High | High | Overkill for Stage 2-3; consider for high-security enterprise clients at Stage 4+ |
| **Instance-per-tenant** | Separate deployment per tenant | Maximum | Maximum | Only for government/regulated clients; aligns with ORCHA's sovereignty vision |

**Current state**: ORCHA uses shared schema with `organization_id` scoping. This is verified by the org-scoped CRM queries (per MEMORY.md).

**Gaps in current isolation**:

| Gap | Risk | Priority | Stage |
|-----|------|----------|-------|
| **No row-level security (RLS) enforcement** | A bug in any query could leak cross-tenant data | P0 | 1 |
| **Agent context leakage** | An agent processing Org A's data could retain context visible to Org B | P0 | 1 |
| **Shared file storage** | Artifacts from different tenants may be in same directory/bucket | P1 | 2 |
| **No tenant-level resource quotas** | One tenant's heavy usage could degrade service for others (noisy neighbor) | P0 | 1 |
| **Shared encryption keys** | All tenant data encrypted (when implemented) with same keys | P1 | 2 |
| **Audit log co-mingling** | Audit entries from all tenants in same log stream | P1 | 2 |

**Recommended progression**:
1. **Stage 1**: Implement application-level tenant isolation middleware (every database query must include `organization_id` filter, enforced at the ORM/query-builder level, not ad-hoc)
2. **Stage 2**: PostgreSQL Row-Level Security (RLS) policies as defense-in-depth
3. **Stage 3**: Per-tenant encryption keys via KMS
4. **Stage 4+**: Schema-per-tenant option for enterprise clients; instance-per-tenant for regulated industries

### 4.2 Cross-Tenant Attack Vectors

| Vector | Description | Mitigation | Priority |
|--------|-------------|------------|----------|
| **IDOR (Insecure Direct Object Reference)** | Guessing or enumerating UUIDs to access other tenant's resources | Validate `organization_id` ownership on every resource access, not just listing | P0 |
| **Agent context injection** | Crafting CRM data that causes an agent to leak information from its context (which may include other tenant data if shared) | Strict agent context scoping — build per-invocation context from scratch, never carry state between tenants | P0 |
| **Search/filter leakage** | Full-text search or filter operations returning results across tenants | Tenant filter applied at database level, not application level. PostgreSQL RLS is ideal. | P0 |
| **Error message leakage** | Stack traces or error messages revealing other tenant's data or system internals | Sanitize all error responses; never return raw database errors to clients | P0 |
| **Side-channel timing** | Timing differences revealing existence of resources in other tenants | Use constant-time comparisons for auth; avoid early-exit patterns in tenant validation | P2 |
| **Shared cache poisoning** | If caching (Redis, in-memory), tenant A could poison cache entries read by tenant B | Namespace all cache keys with `org:{organization_id}:` prefix | P1 |
| **Webhook/SSE misdirection** | SSE events or webhook payloads delivered to wrong tenant's connection | Validate tenant ownership on every event emission, not just subscription | P0 |

### 4.3 Audit Logging Requirements

| Requirement | Description | Priority | Stage |
|-------------|-------------|----------|-------|
| **Authentication events** | Login, logout, failed login, session creation/destruction | P0 | 1 |
| **Authorization events** | Access granted/denied for resources, role changes | P0 | 1 |
| **Data access** | Who accessed what data (CRM contacts, deals, tasks) | P1 | 2 |
| **Data modification** | Who changed what, old value, new value (full change history) | P0 | 1 |
| **Agent actions** | What each agent did, on behalf of whom, which data was accessed/modified | P0 | 1 |
| **Administrative actions** | User management, org settings, billing changes | P0 | 1 |
| **Tamper resistance** | Audit logs must be immutable (append-only, cryptographic chaining) | P1 | 2 |
| **Retention** | Minimum 1 year retention; 7 years for financial/regulated industries | P1 | 2 |
| **Export** | Ability to export audit logs for external analysis or legal discovery | P1 | 2 |

**Implementation approach**:
- Stage 1: Append-only audit table in database with structured JSON payloads
- Stage 2: Separate audit log storage (not in same database as operational data) for tamper resistance
- Stage 3+: Cryptographic log chaining (hash chain) for provable integrity

### 4.4 Key Management Per Tenant

| Stage | Approach | Description |
|-------|----------|-------------|
| 1 | **Shared platform key** | Single encryption key for all tenant data. Acceptable for early stage. |
| 2 | **Per-tenant derived keys** | Master key + tenant-specific derivation (HKDF). Single master to manage, unique per-tenant keys. |
| 3 | **KMS-backed per-tenant keys** | AWS KMS, GCP KMS, or HashiCorp Vault. Each tenant gets dedicated key with independent rotation. |
| 4+ | **Bring Your Own Key (BYOK)** | Enterprise clients provide their own encryption keys. Maximum sovereignty. |

**Recommended**: Use HashiCorp Vault (BSL — note: changed from MPL to BSL in 2023) or OpenBao (fork, Apache 2.0) for key management starting at Stage 2.

---

## 5. Open Source Compliance

### 5.1 License Compatibility Matrix

ORCHA uses dependencies under MIT, Apache 2.0, BSD, and other licenses. Compatibility depends on ORCHA's own license.

**ORCHA licensing plan** (from roadmap): Proprietary now, BSL (Business Source License) at Stage 3, MIT for MCP server definitions and protocol specs.

| Dependency License | Compatible with Proprietary? | Compatible with BSL? | Compatible with MIT? | Notes |
|-------------------|------------------------------|---------------------|---------------------|-------|
| **MIT** | Yes | Yes | Yes | Most permissive; no issues |
| **Apache 2.0** | Yes | Yes | Yes | Must include NOTICE file; patent grant included |
| **BSD 2-Clause** | Yes | Yes | Yes | Minimal requirements |
| **BSD 3-Clause** | Yes | Yes | Yes | Non-endorsement clause |
| **ISC** | Yes | Yes | Yes | MIT-equivalent |
| **MPL 2.0** | Yes (with care) | Yes (with care) | Yes | File-level copyleft — modified MPL files must remain MPL. Unmodified usage is fine. |
| **LGPL 2.1/3.0** | Yes (dynamic linking) | Yes (dynamic linking) | Complicated | Must allow relinking. Static linking may trigger copyleft. |
| **GPL 2.0/3.0** | NO | NO | NO | Copyleft — cannot include in proprietary or BSL product |
| **AGPL 3.0** | NO | NO | NO | Network copyleft — even SaaS usage triggers copyleft |
| **SSPL** | NO | NO | NO | MongoDB's license — OSI does not consider it open source |

**Critical action**: Scan all Cargo.lock and package-lock.json dependencies for GPL/AGPL/SSPL licenses. Any such dependency must be replaced or isolated.

**Rust ecosystem**: Generally safe — most Rust crates are MIT and/or Apache 2.0. The `ring` crate uses a custom ISC-style license. `openssl-sys` links to OpenSSL (Apache 2.0 as of OpenSSL 3.0).

**Node ecosystem**: Higher risk — npm packages occasionally use GPL. React (MIT), Vite (MIT), Tailwind (MIT) are all safe.

### 5.2 SBOM Generation Tools

| Tool | License | Ecosystems | Output Formats | Integration | Pros | Cons |
|------|---------|-----------|----------------|-------------|------|------|
| **cargo-sbom** | Apache 2.0 | Rust only | CycloneDX, SPDX | CLI, CI integration | Rust-native, accurate Cargo.lock parsing | Rust only; need separate tool for Node |
| **syft** (Anchore) | Apache 2.0 | Rust, Node, Python, Go, containers, many more | CycloneDX, SPDX, Syft JSON | CLI, CI, Docker | Multi-ecosystem, single tool for entire stack | Less Rust-specific than cargo-sbom |
| **CycloneDX CLI** | Apache 2.0 | Multi-ecosystem via plugins | CycloneDX only | CLI, CI | OWASP standard | Requires per-ecosystem plugins |
| **trivy** | Apache 2.0 | Multi-ecosystem | CycloneDX, SPDX | CLI, CI, container | Also does vuln scanning | SBOM is secondary feature |

**Recommendation**: Use **syft** as primary SBOM generator (covers both Rust and Node in single pass), with **cargo-sbom** as Rust-specific supplement for higher fidelity.

**CI integration** (add to `.github/workflows/ci.yml`):
```yaml
- name: Generate SBOM
  run: |
    syft . -o cyclonedx-json > sbom.cdx.json
    syft . -o spdx-json > sbom.spdx.json
- name: Upload SBOM
  uses: actions/upload-artifact@v4
  with:
    name: sbom
    path: sbom.*.json
```

### 5.3 CLA vs DCO for Accepting Contributions

If/when ORCHA accepts external contributions (Stage 3+ BSL/source-available transition):

| Mechanism | Description | Pros | Cons | Recommendation |
|-----------|-------------|------|------|----------------|
| **CLA (Contributor License Agreement)** | Contributors sign legal agreement granting IP rights to ORCHA/PCG | Full IP control; can relicense; standard for commercial OSS (React, Kubernetes, etc.) | Friction for contributors; requires CLA bot/management; some devs refuse to sign CLAs | **Recommended** for BSL-licensed core (need IP control for relicensing) |
| **DCO (Developer Certificate of Origin)** | Contributors sign-off each commit certifying they have the right to submit it | Lower friction; `Signed-off-by` in commit message; no legal agreement needed | No IP assignment — PCG cannot relicense contributions; less control | **Recommended** for MIT-licensed MCP specs/protocols |

**Recommendation**:
- **CLA** for the BSL-licensed ORCHA platform core (use CLA Assistant — MIT license)
- **DCO** for MIT-licensed MCP server definitions and protocol specifications
- Implement CLA bot when external contributions begin (Stage 3)

### 5.4 OSS License Scanning Tools

| Tool | License | Method | CI Integration | Pros | Cons |
|------|---------|--------|----------------|------|------|
| **licensee** (GitHub) | MIT | Detects license from files | CLI, CI | Simple, fast, GitHub-maintained | Only detects project license, not dependencies |
| **scancode-toolkit** | Apache 2.0 | Deep file-level license/copyright detection | CLI, CI | Gold standard for license compliance; detects embedded licenses | Slow on large codebases; complex output |
| **trivy** | Apache 2.0 | Dependency-level license detection | CLI, CI | Also does vuln scanning; fast | Less thorough than scancode for file-level detection |
| **cargo-deny** | MIT/Apache 2.0 | Rust-specific license + advisory checking | CLI, CI, `deny.toml` config | Rust-native; can block specific licenses in CI | Rust only |
| **license-checker** (npm) | BSD-3 | npm package license detection | CLI, CI | Simple, fast for Node ecosystem | Node only |

**Recommended stack**:
1. **cargo-deny** — add to CI immediately (P0, Stage 0). Configure to deny GPL/AGPL.
2. **trivy** — dependency license scanning in CI (P1, Stage 1)
3. **scancode-toolkit** — deep audit before SaaS launch (P1, Stage 2)

**cargo-deny configuration** (`deny.toml`):
```toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-DFS-2016"]
deny = ["GPL-2.0", "GPL-3.0", "AGPL-3.0", "SSPL-1.0"]
copyleft = "deny"
```

---

## 6. Insurance and Liability

### 6.1 Cyber Insurance Requirements for SaaS

| Coverage Type | Description | When Needed | Estimated Premium |
|---------------|-------------|-------------|-------------------|
| **Cyber liability (first-party)** | Covers your losses from breaches: forensics, notification, business interruption | Stage 1 (first external users) | $2-5K/year for $1M coverage at early stage |
| **Cyber liability (third-party)** | Covers claims from customers whose data was breached | Stage 1 | Included in above |
| **Technology E&O** | Covers claims that your software failed to perform as promised, causing client losses | Stage 2 (SaaS launch) | $3-8K/year for $1-2M coverage |
| **General liability** | Standard business liability (not cyber-specific) | Stage 0 (business formation) | $500-1.5K/year |
| **D&O insurance** | Protects directors/officers from personal liability | Stage 1 (when taking investment) | $2-5K/year |

**Cyber insurance underwriting requirements** (what insurers will ask):
- MFA on all admin accounts
- Regular vulnerability scanning
- Incident response plan documented
- Data encryption at rest and in transit
- Employee security training
- Backup and recovery procedures tested
- Patch management process

**Key consideration**: Cyber insurance premiums drop significantly with SOC 2 Type II certification. Getting SOC 2 before purchasing cyber insurance can save 20-40% on premiums.

**Priority**: P1 | **Stage**: General liability at Stage 0; Cyber + D&O at Stage 1; Tech E&O at Stage 2

### 6.2 AI Liability Frameworks

**Current landscape** (as of March 2026):

| Jurisdiction | Framework | Status | ORCHA Impact |
|-------------|-----------|--------|--------------|
| **EU** | AI Liability Directive (proposed) | In legislative process | Would create presumption of causality for AI-caused harm; burden of proof shifts to AI deployer |
| **EU** | Product Liability Directive (revised) | Adopted 2024 | Software (including AI) explicitly classified as "product" — strict liability applies |
| **US** | No federal AI liability law | State-level patchwork | Common law negligence/product liability applies; Section 230 may not protect AI outputs |
| **UK** | AI regulatory framework (white paper approach) | Pro-innovation, sector-specific | Lighter touch; existing regulators handle AI within their domains |

**ORCHA liability exposure**:

| Scenario | Risk | Mitigation |
|----------|------|------------|
| Agent provides incorrect CRM data that leads to lost deal | Medium | Disclaimers in ToS; "AI-assisted, human-verified" workflow pattern |
| Agent-generated content contains defamatory/infringing material | Medium | Content review gates; ToS disclaimers; insurance |
| Agent makes automated decision that discriminates | High (if employment/credit) | Avoid high-risk use cases until Stage 3+; human oversight requirements |
| Agent accesses/leaks sensitive client data | High | Technical controls (tenant isolation, encryption); cyber insurance; DPA |
| Platform outage causes client business interruption | Medium | SLA with liability caps; Tech E&O insurance |

### 6.3 Terms of Service Patterns for AI Platforms

Key clauses for ORCHA's ToS:

| Clause | Purpose | Priority |
|--------|---------|----------|
| **AI output disclaimer** | "AI-generated outputs are provided as-is. User is responsible for reviewing and verifying all AI-generated content before use." | P0 |
| **Limitation of liability** | Cap liability at 12 months of fees paid. Exclude consequential/indirect damages. | P0 |
| **Acceptable use policy** | Prohibit using ORCHA for: illegal purposes, generating harmful content, circumventing safety controls, high-risk autonomous decisions without human oversight | P0 |
| **Data ownership** | "Customer retains all rights to Customer Data. ORCHA obtains a limited license to process Customer Data solely to provide the Service." | P0 |
| **AI training opt-out** | "ORCHA does not use Customer Data to train AI models. Customer Data is processed only for Service delivery." Explicit statement — critical for enterprise trust. | P0 |
| **Indemnification** | Customer indemnifies ORCHA for misuse; ORCHA indemnifies customer for IP infringement of the platform itself | P1 |
| **Data portability** | Customer can export all data at any time in standard formats | P1 |
| **Termination and data deletion** | Upon termination, customer data deleted within 30 days (GDPR alignment) | P0 |
| **SLA** | Uptime commitment with service credits (e.g., 99.5% = 5% credit, 99.0% = 10% credit) | P1 (Stage 2+) |
| **Subprocessor list** | List all third parties that process customer data (LLM providers, cloud hosting, etc.) | P0 (GDPR requirement) |

**Recommendation**: Have legal counsel draft ToS before first pilot client (Stage 1). Budget $5-10K for initial legal work. Use Anthropic's, OpenAI's, and Linear's ToS as references for AI-specific clauses.

### 6.4 Data Processing Agreements (DPA)

**Required by**: GDPR (mandatory for any processor handling EU personal data), many enterprise procurement processes.

**DPA must include**:
- Subject matter and duration of processing
- Nature and purpose of processing
- Types of personal data processed
- Categories of data subjects
- Obligations and rights of the controller (customer)
- Subprocessor list and change notification process
- Data transfer mechanisms (SCCs if outside EU)
- Data deletion/return upon termination
- Audit rights for the customer

**Subprocessor chain for ORCHA**:

| Subprocessor | Data Processed | Purpose |
|-------------|----------------|---------|
| Anthropic (Claude) | CRM data, task descriptions, workflow data sent as prompts | AI agent execution |
| Google (Gemini) | Same as above (if used) | AI agent execution |
| Cloud hosting provider (TBD) | All customer data | Infrastructure |
| SendGrid | Email addresses, email content | Email intake |
| Twilio | Phone numbers, call/SMS content | Communication intake |
| GitHub | OAuth tokens, user identity | Authentication |
| Sentry | Error traces (may contain user data) | Error monitoring |

**Critical issue**: LLM providers as subprocessors. Anthropic and other providers have their own DPAs and data handling policies. ORCHA's DPA must accurately reflect what data is sent to LLM providers and how those providers handle it. This is a major enterprise concern.

**Recommendation**:
- Stage 1: Draft DPA template using GDPR standard contractual clauses as foundation ($3-5K legal)
- Stage 1: Review and document Anthropic's data handling for API customers (Anthropic's API Terms state they do not train on API inputs — this is a key selling point)
- Stage 2: Make DPA available for self-service signing (DocuSign/PandaDoc integration)

**Budget**: $3-5K initial DPA template; $1-2K per custom DPA negotiation with enterprise clients

---

## 7. Compliance Tooling

All recommended tools use permissive licenses (Apache 2.0 or MIT).

### 7.1 Open Policy Agent (OPA)

**License**: Apache 2.0
**What it does**: Policy-as-code engine. Define authorization, compliance, and governance rules in Rego language. Evaluate policies against structured data at decision time.

| Aspect | Detail |
|--------|--------|
| **Use cases for ORCHA** | RBAC policy enforcement, tenant isolation validation, API authorization, agent permission boundaries, compliance rule evaluation |
| **Integration** | REST API, Go library, WASM (can run in Rust via wasmtime), sidecar pattern |
| **Maturity** | CNCF Graduated project — production-grade |
| **Learning curve** | Rego language is unique; 2-4 weeks for team to become productive |

**Pros**:
- Decouples policy from application code — change policies without code deployment
- Auditable — all policy decisions can be logged with full context
- Supports "policy bundles" — distribute policies across services
- Natural fit for multi-tenant authorization and agent permission boundaries
- Can enforce "agent X may only access data for organization Y" as a policy

**Cons**:
- Rego language has a learning curve
- Adds latency to every decision point (typically 1-5ms, but accumulates)
- Overkill for simple RBAC at Stage 0-1
- Requires policy lifecycle management (testing, versioning, deployment)

**Roadmap fit**:
- **Stage 1**: Evaluate OPA for tenant-level authorization
- **Stage 2**: Deploy OPA for API authorization and agent permission boundaries
- **Stage 3+**: Full policy-as-code for compliance rules (data residency, processing restrictions)

**Priority**: P1 | **Stage**: Evaluate Stage 1, deploy Stage 2

### 7.2 Falco

**License**: Apache 2.0
**What it does**: Runtime security monitoring for containers and Kubernetes. Detects unexpected behavior (file access, network connections, process execution) based on rules.

| Aspect | Detail |
|--------|--------|
| **Use cases for ORCHA** | Detect unauthorized file access in agent containers, monitor for container escape attempts, alert on unexpected network connections, detect crypto mining |
| **Integration** | Kernel module or eBPF probe; outputs to syslog, stdout, gRPC, webhook |
| **Maturity** | CNCF Incubating project |
| **Resource overhead** | ~1-5% CPU overhead for kernel module; eBPF is lighter |

**Pros**:
- Detects threats that static scanning misses (runtime anomalies)
- Extensive default ruleset for container security
- Can detect if an agent execution container does something unexpected (network connection to unauthorized endpoint, file access outside sandbox)
- CNCF ecosystem integration (Prometheus, Grafana, Kubernetes)

**Cons**:
- Requires Linux kernel access (kernel module or eBPF) — adds ops complexity
- High false-positive rate initially — requires tuning
- Not useful until containerized deployment is in production
- Overkill for single-machine Docker Compose deployment

**Roadmap fit**:
- **Stage 3**: Deploy when Docker service-per-agent architecture is in production
- **Stage 4**: Full Falco deployment with custom rules for agent container monitoring

**Priority**: P2 | **Stage**: 3-4

### 7.3 Keycloak

**License**: Apache 2.0
**What it does**: Identity and access management (IAM) server. Provides SSO, OIDC, SAML, user federation, social login, MFA, fine-grained authorization.

| Aspect | Detail |
|--------|--------|
| **Use cases for ORCHA** | Replace custom GitHub OAuth with full IAM; SSO for enterprise clients; SAML for corporate identity providers; MFA; user federation (LDAP/AD integration); fine-grained authorization services |
| **Integration** | Standalone server; OIDC/SAML adapters for applications; REST admin API |
| **Maturity** | Extremely mature — used by Red Hat, hundreds of enterprises |
| **Resource overhead** | Java-based — 512MB-2GB RAM minimum; can be resource-heavy |

**Pros**:
- Complete IAM solution — SSO, SAML, OIDC, MFA, user federation all in one
- Enterprise SSO (SAML) is a hard requirement for most enterprise clients (Priority 27 in roadmap)
- Social login providers (Google, Microsoft, GitHub) out of the box
- Admin console for user management
- Customizable login themes
- Fine-grained authorization services (can complement OPA)

**Cons**:
- Java-based — significant memory footprint (512MB-2GB)
- Complex to configure correctly; security misconfiguration risk
- Adds operational burden (another service to maintain, upgrade, backup)
- May be overkill for Stage 0-1 where simple OAuth suffices
- Quarkus-based distribution is lighter but still Java

**Alternatives**:
- **Zitadel** (Apache 2.0): Go-based, lighter, built-in OIDC/SAML. Newer but growing fast.
- **Authentik** (MIT Expat + Enterprise): Python-based, modern UI. Free core, paid enterprise features.
- **Ory** (Apache 2.0): Modular identity infrastructure (Kratos for identity, Hydra for OAuth2, Keto for authorization). Lightweight, cloud-native.

**Recommendation**: Evaluate Keycloak vs Zitadel vs Ory at Stage 1. Deploy chosen solution at Stage 2 for SaaS launch. Key decision factor: resource footprint (3-5 person team cannot afford heavy ops burden).

**Roadmap fit**:
- **Stage 1**: Evaluate identity solutions; extend current auth with email/password
- **Stage 2**: Deploy identity server for self-service signup + enterprise SSO
- **Stage 3+**: SAML federation, LDAP/AD integration for enterprise clients

**Priority**: P0 | **Stage**: Evaluate Stage 1, deploy Stage 2

### 7.4 cert-manager

**License**: Apache 2.0
**What it does**: Automated TLS certificate management for Kubernetes. Issues, renews, and manages X.509 certificates from various issuers (Let's Encrypt, Vault, custom CAs).

| Aspect | Detail |
|--------|--------|
| **Use cases for ORCHA** | Automated TLS for all services; mTLS between services; certificate rotation; Let's Encrypt integration for custom domains (hosting service) |
| **Integration** | Kubernetes-native (CRDs); works with Ingress controllers, Istio, etc. |
| **Maturity** | CNCF Graduated project |

**Pros**:
- Eliminates manual certificate management entirely
- Let's Encrypt integration means free, automated TLS for all custom domains
- Critical for the Website/Hosting Service (Priority 25) — each hosted site needs TLS
- mTLS between services improves internal security posture
- CNCF Graduated — battle-tested in production

**Cons**:
- Kubernetes-specific — not useful until ORCHA deploys on Kubernetes
- Current Cloudflare Tunnel setup handles TLS adequately for Stage 0-1
- Adds operational dependency on Kubernetes

**Roadmap fit**:
- **Stage 2**: Deploy when migrating to Kubernetes for SaaS launch
- **Stage 2+**: Essential for website/hosting service (auto-SSL for customer domains)

**Priority**: P1 | **Stage**: 2

### 7.5 Additional Compliance Tooling

| Tool | License | Purpose | Priority | Stage |
|------|---------|---------|----------|-------|
| **cargo-deny** | MIT/Apache 2.0 | Rust dependency license + advisory + ban checking | P0 | 0 |
| **Trivy** | Apache 2.0 | Comprehensive vulnerability + license + SBOM scanning | P1 | 1 |
| **syft** | Apache 2.0 | SBOM generation (CycloneDX/SPDX) | P1 | 1 |
| **grype** | Apache 2.0 | Vulnerability scanner (pairs with syft SBOMs) | P1 | 1 |
| **OpenBao** | MPL 2.0 (transitioning to Apache 2.0) | Secrets management (Vault fork) | P1 | 2 |
| **scancode-toolkit** | Apache 2.0 | Deep license compliance scanning | P1 | 2 |
| **Dex** | Apache 2.0 | Lightweight OIDC provider (alternative to Keycloak for Stage 1) | P1 | 1 |
| **kube-bench** | Apache 2.0 | CIS Kubernetes Benchmark audit | P2 | 3 |
| **Kyverno** | Apache 2.0 | Kubernetes-native policy engine (alternative to OPA for K8s) | P2 | 3 |

---

## 8. Implementation Roadmap

### Stage 0 (Dogfood) — Q2 2026

| Action | Category | Effort | Priority |
|--------|----------|--------|----------|
| AI interaction transparency labels in UI | AI Regulation | 2 hours | P0 |
| Expand rate limiting to all public endpoints | API Security | 1 day | P0 |
| Input validation framework (tower/axum middleware) | API Security | 2 days | P0 |
| Enable Dependabot for Cargo.toml + package.json | Supply Chain | 1 hour | P0 |
| Add `cargo-deny` to CI with license deny list | OSS Compliance | 2 hours | P0 |
| Prompt injection defenses for agent inputs | AI Security | 2 days | P0 |
| Document all data processing activities (Art. 30) | GDPR | 1 day | P0 |
| General liability insurance | Insurance | $500-1.5K | P1 |

**Stage 0 budget**: ~$1-2K (insurance) + ~2 weeks engineering effort

### Stage 1 (First Pilots) — Q3-Q4 2026

| Action | Category | Effort | Priority |
|--------|----------|--------|----------|
| Draft privacy policy + ToS | Legal | $5-10K (legal counsel) | P0 |
| Draft DPA template | GDPR | $3-5K (legal counsel) | P0 |
| Right-to-erasure endpoint (cascading delete) | GDPR | 1 week | P0 |
| Tenant isolation middleware (enforce org_id on all queries) | Multi-Tenant | 1 week | P0 |
| Agent context isolation (per-invocation, no cross-tenant state) | Multi-Tenant | 3 days | P0 |
| Audit logging (auth events, data modifications, agent actions) | Compliance | 1 week | P0 |
| Token budget limits + circuit breakers per agent | AI Security | 3 days | P0 |
| SBOM generation in CI (syft + cargo-sbom) | Supply Chain | 4 hours | P1 |
| Trivy vulnerability + container scanning in CI | Supply Chain | 4 hours | P1 |
| Evaluate identity solutions (Keycloak vs Zitadel vs Ory) | IAM | 1 week | P1 |
| AI-generated content metadata tagging | AI Regulation | 2 days | P1 |
| Model version tracking in agent execution logs | AI Audit | 2 days | P1 |
| Cyber + D&O insurance | Insurance | $4-10K/year | P1 |
| SOC 2 readiness assessment | Compliance | $5-10K (consultant) | P1 |
| Backup encryption | Encryption | 2 days | P1 |
| Per-tenant rate limiting | API Security | 2 days | P0 |

**Stage 1 budget**: $17-35K (legal + insurance + SOC 2 assessment) + ~6 weeks engineering

### Stage 2 (SaaS Launch) — Q1-Q3 2027

| Action | Category | Effort | Priority |
|--------|----------|--------|----------|
| Deploy identity server (Keycloak/Zitadel/Ory) with SSO | IAM | 2 weeks | P0 |
| PostgreSQL RLS policies for defense-in-depth tenant isolation | Multi-Tenant | 1 week | P0 |
| Encryption at rest (PostgreSQL TDE or application-level) | Encryption | 1 week | P1 |
| SOC 2 Type I audit | Compliance | $15-30K (auditor) | P1 |
| Begin SOC 2 Type II observation period | Compliance | Ongoing | P1 |
| Compliance automation platform (Vanta/Drata/Secureframe) | Compliance | $10-20K/year | P1 |
| OPA deployment for API authorization + agent permissions | Authorization | 2 weeks | P1 |
| cert-manager for automated TLS | Infrastructure | 1 week | P1 |
| Per-tenant encryption key derivation | Encryption | 1 week | P1 |
| Audit log tamper resistance (separate storage, hash chaining) | Audit | 1 week | P1 |
| API versioning | API Security | 1 week | P1 |
| Data residency controls (EU hosting option) | GDPR | 2 weeks | P1 |
| DPIA for AI processing | GDPR | $3-5K (consultant) | P1 |
| External DPO engagement | GDPR | $2-5K/month | P1 |
| Tech E&O insurance | Insurance | $3-8K/year | P1 |
| Deep license audit (scancode-toolkit) | OSS Compliance | 3 days | P1 |
| seccomp profiles + network policies for containers | Runtime Security | 1 week | P1 |
| Decision ledger for agent actions | AI Audit | 2 weeks | P1 |
| Bias monitoring framework (statistical distribution analysis) | AI Regulation | 2 weeks | P2 |

**Stage 2 budget**: $35-70K (auditor + compliance platform + DPO + insurance + legal) + ~12 weeks engineering

### Stage 3 (PMF & VIBE) — Q4 2027-Q2 2028

| Action | Category | Effort | Priority |
|--------|----------|--------|----------|
| SOC 2 Type II certification achieved | Compliance | $15-30K (auditor) | P1 |
| ISO 27001 ISMS implementation begin | Compliance | $20-40K | P2 |
| KMS-backed per-tenant encryption keys | Encryption | 2 weeks | P1 |
| Falco runtime security deployment | Runtime Security | 1 week | P2 |
| CLA bot for external contributions | OSS Compliance | 2 days | P1 |
| VIBE token regulatory review | Legal | $10-20K (specialized counsel) | P0 |
| NIST AI RMF voluntary adoption | AI Regulation | 2 weeks | P2 |
| EU AI Act conformity preparation (if high-risk clients) | AI Regulation | $10-20K (consultant) | P1 |
| Patent filings | IP | $15-30K per provisional | P1 |

**Stage 3 budget**: $85-180K (certifications + legal + IP) + ~6 weeks engineering

### Stage 4+ (Growth & Sovereignty)

| Action | Category | Effort | Priority |
|--------|----------|--------|----------|
| ISO 27001 certification | Compliance | $20-50K | P2 |
| BYOK (Bring Your Own Key) for enterprise | Encryption | 4 weeks | P2 |
| Instance-per-tenant option for regulated industries | Multi-Tenant | 8 weeks | P2 |
| Full EU AI Act high-risk compliance (if needed) | AI Regulation | $30-60K | P2 |
| SOC 2 annual renewal | Compliance | $15-30K/year | P1 |
| Dedicated security team hire | Organization | $100-150K/year | P1 |

---

## 9. Budget Summary

### By Stage

| Stage | Legal/Compliance | Insurance | Tooling | Engineering (weeks) | Total Non-Engineering |
|-------|-----------------|-----------|---------|--------------------|-----------------------|
| **0** | $0 | $1-2K | $0 | 2 weeks | $1-2K |
| **1** | $13-25K | $4-10K | $0 | 6 weeks | $17-35K |
| **2** | $20-35K | $3-8K | $10-20K | 12 weeks | $33-63K |
| **3** | $50-100K | ongoing | ongoing | 6 weeks | $50-100K |
| **4+** | $65-140K | ongoing | ongoing | 12+ weeks | $65-140K |

### Cumulative Through Stage 3 (PMF)

| Category | Cumulative Cost |
|----------|----------------|
| Legal counsel (ToS, DPA, VIBE token, patents) | $35-70K |
| Audit/certification (SOC 2 Type I + II, consultants) | $35-70K |
| Insurance (cyber, D&O, E&O, general) | $10-25K/year |
| Compliance tooling (Vanta/Drata) | $10-20K/year |
| DPO (external) | $24-60K/year |
| **Total through Stage 3** | **$115-245K** (over ~2 years) |

### Annual Ongoing (Post Stage 3)

| Item | Annual Cost |
|------|------------|
| SOC 2 renewal | $15-30K |
| ISO 27001 surveillance | $10-20K |
| Insurance bundle | $10-25K |
| DPO | $24-60K |
| Compliance tooling | $10-20K |
| Legal retainer | $5-15K |
| **Annual total** | **$74-170K** |

---

## 10. Risk Register

| Risk | Probability | Impact | Stage | Mitigation |
|------|------------|--------|-------|------------|
| GDPR violation before privacy framework in place | Medium | Critical | 1 | Priority: privacy policy + DPA before first EU pilot |
| Cross-tenant data leakage via agent context | Medium | Critical | 1+ | Strict per-invocation context building; no persistent agent memory across tenants |
| Prompt injection leading to data exfiltration | Medium | High | 0+ | Input sanitization, output filtering, system prompt hardening |
| GPL dependency discovered in production | Low | High | 1+ | cargo-deny + license scanning in CI; block GPL at merge gate |
| SOC 2 audit failure | Low | High | 2-3 | Early readiness assessment; compliance automation platform |
| VIBE token classified as security | Medium | Critical | 2-3 | Legal counsel before any token distribution; maintain fiat option |
| EU AI Act high-risk classification | Low | High | 2+ | Avoid high-risk use cases; document transparency measures proactively |
| Cyber insurance claim denied due to missing controls | Low | High | 1+ | Align security controls with insurer requirements before purchasing policy |
| LLM provider DPA conflict with customer DPA | Medium | Medium | 1+ | Review Anthropic API data handling; ensure DPA chain is consistent |
| Patent prior art invalidation | Medium | Medium | 3+ | Thorough prior art search before filing; focus on novel combinations |

---

## Appendix A: Priority Summary (P0 Items)

These are blockers for the respective stage — cannot proceed without them.

| Item | Stage | Category | Estimated Effort |
|------|-------|----------|-----------------|
| AI transparency labels in UI | 0 | AI Regulation | 2 hours |
| Rate limiting on all endpoints | 0 | API Security | 1 day |
| Input validation framework | 0 | API Security | 2 days |
| Dependabot enabled | 0 | Supply Chain | 1 hour |
| cargo-deny in CI | 0 | OSS Compliance | 2 hours |
| Prompt injection defenses | 0 | AI Security | 2 days |
| Records of processing | 0 | GDPR | 1 day |
| Privacy policy + ToS | 1 | Legal | $5-10K |
| DPA template | 1 | GDPR | $3-5K |
| Right-to-erasure endpoint | 1 | GDPR | 1 week |
| Tenant isolation middleware | 1 | Multi-Tenant | 1 week |
| Agent context isolation | 1 | Multi-Tenant | 3 days |
| Audit logging | 1 | Compliance | 1 week |
| Token budget limits | 1 | AI Security | 3 days |
| Per-tenant rate limiting | 1 | API Security | 2 days |
| Identity server deployment | 2 | IAM | 2 weeks |
| PostgreSQL RLS | 2 | Multi-Tenant | 1 week |
| VIBE token regulatory review | 3 | Legal | $10-20K |

## Appendix B: Tooling Decision Matrix

All tools below use permissive licenses (Apache 2.0 or MIT).

| Tool | License | Category | Stage | Priority | Annual Cost |
|------|---------|----------|-------|----------|-------------|
| cargo-deny | MIT/Apache 2.0 | License compliance | 0 | P0 | Free |
| Dependabot | Free (GitHub) | Dependency updates | 0 | P0 | Free |
| cargo-audit | MIT/Apache 2.0 | Vulnerability scan | 0 | P0 | Free (already in CI) |
| npm audit | Built-in | Vulnerability scan | 0 | P0 | Free (already in CI) |
| Trivy | Apache 2.0 | Vuln + license + container scan | 1 | P1 | Free |
| syft | Apache 2.0 | SBOM generation | 1 | P1 | Free |
| grype | Apache 2.0 | Vulnerability scanner | 1 | P1 | Free |
| cargo-sbom | Apache 2.0 | Rust SBOM generation | 1 | P1 | Free |
| OPA | Apache 2.0 | Policy-as-code | 2 | P1 | Free |
| Keycloak | Apache 2.0 | Identity management | 2 | P0 | Free (self-hosted) |
| cert-manager | Apache 2.0 | TLS automation | 2 | P1 | Free |
| scancode-toolkit | Apache 2.0 | Deep license scanning | 2 | P1 | Free |
| Falco | Apache 2.0 | Runtime security | 3 | P2 | Free |
| OpenBao | MPL 2.0 | Secrets management | 2 | P1 | Free |
| kube-bench | Apache 2.0 | K8s CIS benchmark | 3 | P2 | Free |
| Kyverno | Apache 2.0 | K8s policy engine | 3 | P2 | Free |

## Appendix C: References

- GDPR full text: https://gdpr-info.eu/
- CCPA/CPRA: https://oag.ca.gov/privacy/ccpa
- EU AI Act: https://artificialintelligenceact.eu/
- OWASP Top 10 for LLM Applications: https://owasp.org/www-project-top-10-for-large-language-model-applications/
- NIST AI Risk Management Framework: https://www.nist.gov/itl/ai-risk-management-framework
- SOC 2 Trust Services Criteria: https://www.aicpa-cima.com/topic/audit-assurance/audit-and-assurance-greater-than-soc-2
- ISO 27001: https://www.iso.org/standard/27001
- CycloneDX SBOM standard: https://cyclonedx.org/
- SPDX specification: https://spdx.dev/
- Open Policy Agent: https://www.openpolicyagent.org/
- Falco: https://falco.org/
- Keycloak: https://www.keycloak.org/
- cert-manager: https://cert-manager.io/
- Trivy: https://trivy.dev/
- syft: https://github.com/anchore/syft
- cargo-deny: https://github.com/EmbarkStudios/cargo-deny
- scancode-toolkit: https://github.com/aboutcode-org/scancode-toolkit

---

*This research document informs the 5-year product roadmap. Compliance requirements should be re-evaluated quarterly as regulations evolve. The AI regulatory landscape is changing rapidly — monitor EU AI Act implementation timelines and US state-level AI legislation closely.*
