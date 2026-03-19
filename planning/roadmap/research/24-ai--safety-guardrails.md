# Research: AI Safety, Guardrails, and Responsible AI Patterns

**Date:** 2026-03-19
**Type:** Research
**Scope:** AI safety frameworks, prompt security, output validation, agent sandboxing, audit/compliance, cost control, and evaluation/monitoring for the ORCHA platform (Rust/Axum + React/Vite)

---

## Table of Contents

1. [AI Guardrails Frameworks](#1-ai-guardrails-frameworks)
2. [Prompt Security](#2-prompt-security)
3. [Output Safety](#3-output-safety)
4. [Agent Safety](#4-agent-safety)
5. [Audit and Transparency](#5-audit-and-transparency)
6. [Cost Control](#6-cost-control)
7. [Evaluation and Monitoring](#7-evaluation-and-monitoring)
8. [Rust/ORCHA Integration Considerations](#8-rustorcha-integration-considerations)
9. [Consolidated Roadmap Recommendations](#9-consolidated-roadmap-recommendations)

---

## 1. AI Guardrails Frameworks

### 1.1 NVIDIA NeMo Guardrails

- **License:** Apache 2.0
- **Language:** Python
- **GitHub:** [NVIDIA-NeMo/Guardrails](https://github.com/NVIDIA-NeMo/Guardrails)
- **Docs:** [NVIDIA NeMo Guardrails](https://docs.nvidia.com/nemo-guardrails/index.html)

**What it does:**
NeMo Guardrails is an open-source toolkit for adding programmable guardrails to LLM-based conversational systems. It supports five rail types: input rails, dialog rails, retrieval rails, execution rails, and output rails. It acts as a central hub routing requests to different models and services for content safety and topic control.

**Key capabilities:**
- Topic control (keeping conversations on-topic)
- PII detection
- RAG grounding checks
- Jailbreak prevention
- Multilingual/multimodal content safety
- Integrates with LangChain, LangGraph, LlamaIndex
- Deploy via local API server, Docker, or production microservices
- GPU acceleration for low-latency performance

**Pros:**
- Most comprehensive guardrails framework available
- Colang scripting language for defining conversational flows
- Multi-agent deployment support
- Strong community and NVIDIA backing
- Extensible architecture with pluggable rail types

**Cons:**
- Python-only (requires sidecar service for Rust integration)
- GPU dependency for best performance adds infrastructure cost
- Colang learning curve
- Heavyweight for simple use cases
- Latency overhead for all five rail types

**Roadmap stage:** Stage 2-3 (after basic agent safety is in place)
**Rationale:** Full NeMo deployment is overkill for Stage 0-1. Start with simpler input/output validation, then layer NeMo for dialog and retrieval rails as agents become more autonomous.
**Budget/timeline:** 2-3 weeks for sidecar integration; GPU costs add $200-500/month depending on throughput.

---

### 1.2 Guardrails AI

- **License:** Apache 2.0
- **Language:** Python
- **GitHub:** [guardrails-ai/guardrails](https://github.com/guardrails-ai/guardrails)

**What it does:**
A Python framework for building reliable AI applications through input/output guards. Defines validators via Pydantic or RAIL file format that inspect and approve/reject LLM output. Guardrails Hub provides a community collection of pre-built validators.

**Key capabilities:**
- Structured output validation (JSON schema, Pydantic models)
- Input/output guards with multiple validators composable into pipelines
- LiteLLM integration for multi-provider support (OpenAI, Anthropic, Cohere, etc.)
- Streaming validation (fast rule-based per chunk, slower LLM-based on complete response)
- Custom validator authoring and Hub sharing

**Pros:**
- Simpler than NeMo for output validation use cases
- Rich validator ecosystem on Guardrails Hub
- Good for structured data extraction from LLMs
- Multi-model support via LiteLLM
- Active community

**Cons:**
- Python-only (sidecar needed)
- Validator quality varies on Hub
- Less comprehensive than NeMo for dialog/topic control
- Some validators are slow (LLM-based)

**Roadmap stage:** Stage 1-2
**Rationale:** Output validation is a Stage 1 concern. Guardrails AI is simpler to integrate than NeMo for validating agent outputs match expected schemas.
**Budget/timeline:** 1-2 weeks integration; minimal infrastructure cost (runs alongside existing services).

---

### 1.3 LLM Guard

- **License:** MIT
- **Language:** Python
- **GitHub:** [protectai/llm-guard](https://github.com/protectai/llm-guard)
- **Docs:** [LLM Guard](https://protectai.com/llm-guard)

**What it does:**
Open-source security toolkit providing 15 input scanners and 20 output scanners for LLM applications. Scans text independent of the model provider. Self-hosted, so data never leaves your infrastructure.

**Key capabilities:**
- **Input scanners:** Prompt injection detection, PII anonymization, toxicity detection, topic banning, language detection, code detection, secrets detection
- **Output scanners:** Bias detection, malicious URL detection, factual consistency, sensitive data leakage, toxicity, relevance scoring, deanonymization
- Integrates with Prometheus and DataDog for monitoring
- Provider-agnostic (scans text, not model)
- 2.5k+ GitHub stars

**Pros:**
- MIT license (most permissive)
- Self-hosted (data sovereignty)
- Comprehensive scanner coverage (35 total)
- Provider-agnostic
- Good monitoring integration
- Lightweight compared to NeMo

**Cons:**
- Python-only
- Some scanners have performance overhead
- Less battle-tested than NeMo in production
- No dialog/flow control capabilities

**Roadmap stage:** Stage 1 (immediate adoption candidate)
**Rationale:** Best balance of simplicity, coverage, and license for early-stage safety. Input/output scanning is a foundational requirement. MIT license is ideal.
**Budget/timeline:** 1 week for sidecar integration; negligible infrastructure cost.

---

### 1.4 Rebuff

- **License:** Apache 2.0
- **Language:** Python
- **GitHub:** [protectai/rebuff](https://github.com/protectai/rebuff)

**What it does:**
Self-hardening prompt injection detection framework with 4 defense layers: heuristics, LLM-based detection, vector DB similarity matching, and canary tokens.

**Key capabilities:**
- Heuristic filtering of malicious input
- Dedicated LLM analysis for attack detection
- Vector DB storage of previous attacks for pattern matching
- Canary token injection to detect prompt leakage
- SDK available for Python

**Pros:**
- Multi-layered defense approach
- Self-improving (learns from attacks via vector DB)
- Canary tokens are a unique detection mechanism
- Focused scope (prompt injection only)

**Cons:**
- Still in alpha/prototype stage
- Cannot guarantee 100% protection
- False positives/negatives common
- Requires vector DB infrastructure
- Python-only
- Limited maintenance activity

**Roadmap stage:** Stage 2-3
**Rationale:** Prompt injection detection is critical but Rebuff's alpha status makes it risky for Stage 1. LLM Guard's prompt injection scanner is more production-ready. Revisit when Rebuff stabilizes.
**Budget/timeline:** 1-2 weeks; requires vector DB (Qdrant/Pinecone).

---

## 2. Prompt Security

### 2.1 Attack Vectors (2026 Landscape)

Prompt injection is the #1 AI security threat in 2026, appearing in 73% of production AI deployments assessed during security audits (OWASP). Key vectors:

| Attack Type | Description | Severity |
|---|---|---|
| **Direct injection** | Malicious instructions via user input, chat, or API | High |
| **Indirect injection** | Malicious content embedded in external data (web pages, emails, documents) | Critical |
| **Multimodal injection** | Adversarial instructions embedded in images bypassing text-layer sanitization | High |
| **MCP/Tool poisoning** | Malicious tool descriptions or parameters in agent tool registries | Critical |
| **Hybrid attacks** | Prompt injection combined with XSS, CSRF, SQL injection | Critical |
| **System prompt extraction** | Techniques to make LLM reveal its system prompt | Medium |

**Real-world CVEs (2025-2026):**
- Microsoft Copilot: CVSS 9.3
- GitHub Copilot: CVSS 9.6
- Cursor IDE: CVSS 9.8

**OWASP Top 10 for LLMs (2025 update):** Includes new entries for System Prompt Leakage (LLM07:2025) and Vector/Embedding Weaknesses (LLM08:2025).

### 2.2 Defense Strategies

**No single defense is sufficient.** Defense-in-depth is the only viable strategy. Even frontier models from OpenAI, Google, and Anthropic remain vulnerable after applying best defenses.

**Layered defense architecture for ORCHA:**

| Layer | Mechanism | Tools | Stage |
|---|---|---|---|
| **L1: Input validation** | Pattern matching, regex, blocklists | LLM Guard input scanners, custom Rust validators | Stage 1 |
| **L2: Prompt structure** | System prompt isolation, instruction hierarchy | Architectural pattern (no external tool) | Stage 0 |
| **L3: LLM-as-critic** | Secondary LLM validates input before primary processing | NeMo Guardrails, custom | Stage 2 |
| **L4: Output validation** | Check outputs for injection artifacts, data leakage | LLM Guard output scanners, Guardrails AI | Stage 1 |
| **L5: Canary tokens** | Embed detectable markers to identify leakage | Rebuff, custom implementation | Stage 2 |
| **L6: Session isolation** | Separate contexts per user/agent to prevent cross-contamination | Architectural pattern | Stage 1 |
| **L7: AI content tagging** | Mark AI-generated content to prevent downstream injection | Custom implementation | Stage 3 |

### 2.3 Input Sanitization for AI Agents

**Recommended patterns for ORCHA:**

1. **Token limit enforcement** — Hard cap on input token count per request, configurable per tenant/agent type. Block oversized inputs before they reach the LLM (proactive, not reactive).
2. **Content-type validation** — Validate that inputs match expected format (e.g., code review input should contain code, not prose instructions).
3. **Encoding normalization** — Normalize Unicode, strip control characters, detect encoded injection attempts.
4. **Context boundary markers** — Use clear delimiters between system instructions, user input, and retrieved context. Never concatenate untrusted input directly into prompts.
5. **Rate limiting** — Per-user, per-agent rate limits to prevent brute-force injection attempts.

### 2.4 Sandboxed Execution for AI-Generated Code

See Section 4 (Agent Safety) for detailed sandboxing analysis.

**Roadmap stage:** L1-L2 at Stage 0, L4-L6 at Stage 1, L3/L5/L7 at Stage 2-3
**Budget/timeline:** L1-L2 are architectural (1-2 days). External tool integration adds 1-2 weeks per layer.

---

## 3. Output Safety

### 3.1 PII Detection and Redaction — Microsoft Presidio

- **License:** MIT
- **Language:** Python
- **GitHub:** [microsoft/presidio](https://github.com/microsoft/presidio)

**What it does:**
Open-source framework for detecting, redacting, masking, and anonymizing sensitive data across text, images, and structured data. Uses NER models, regex, and checksum validation.

**Key capabilities:**
- Detects: credit card numbers, names, locations, SSNs, bitcoin wallets, phone numbers, financial data, email addresses, and more
- Anonymization methods: redaction, hashing, masking, encryption
- Image PII redaction (including DICOM medical images)
- Customizable entity recognizers
- Deployment: Python, PySpark, Docker, Kubernetes

**Pros:**
- MIT license
- Microsoft-backed, widely adopted
- Comprehensive entity coverage
- Multiple anonymization strategies
- Image support
- Customizable recognizers for domain-specific PII

**Cons:**
- Python-only (sidecar needed for Rust)
- NER model loading adds latency
- English-centric (other languages require additional models)
- No streaming support

**ORCHA integration pattern:**
- Deploy as sidecar microservice
- Scan all agent outputs before returning to user
- Scan all inputs to prevent PII in prompts sent to external LLMs
- Store anonymized versions in audit logs
- Reversible anonymization for authorized users

**Roadmap stage:** Stage 1 (foundational for any platform handling user data)
**Budget/timeline:** 1 week integration; negligible cost.

### 3.2 Content Moderation

| Service | Type | Capabilities | License/Pricing | Stage |
|---|---|---|---|---|
| **OpenAI Moderation API** | API | Hate, violence, self-harm, sexual content; multimodal (text+image); GPT-based classifiers | Free with OpenAI account | Stage 1 |
| **Perspective API** (Google/Jigsaw) | API | Toxicity, insult, profanity, threat, identity attack scoring | Free (rate-limited) | Stage 1 |
| **LLM Guard output scanners** | Self-hosted | Bias, toxicity, malicious URLs, factual consistency, relevance | MIT, self-hosted | Stage 1 |

**Recommendation:** Use LLM Guard as primary (self-hosted, no data leaves infrastructure) and OpenAI Moderation API as secondary validation for high-risk outputs. Perspective API for toxicity scoring on user-generated content.

### 3.3 Hallucination Detection

**Current state of the art (2026):**

| Approach | Description | Maturity | Stage |
|---|---|---|---|
| **Factual consistency scoring** | Compare output claims against source documents (RAG grounding) | Production-ready | Stage 2 |
| **Attention pathway analysis** (FACTUM) | Contextual Alignment Score from model internals | Research | Stage 4+ |
| **Metamorphic testing** (MetaQA) | Slight prompt rewordings to detect inconsistent outputs | Research | Stage 3 |
| **Entity-level detection** | Real-time identification of hallucinated names, dates, citations | Emerging | Stage 3 |
| **Cross-reference validation** | Check citations/URLs actually exist and support claims | Implementable | Stage 2 |

**Practical approach for ORCHA:**
- Stage 1-2: Implement RAG grounding checks (does the output cite retrievable sources?)
- Stage 2: Cross-reference validation for URLs and citations
- Stage 3: Metamorphic testing for critical agent decisions
- Stage 4+: Internal state analysis when running local models

### 3.4 Output Format Validation

**Guardrails AI** is purpose-built for this. Key patterns:

1. **JSON schema validation** — Ensure agent outputs match expected Pydantic/JSON schemas
2. **Type coercion** — Auto-fix minor type mismatches
3. **Re-ask on failure** — Automatically re-prompt the LLM when output doesn't validate
4. **Streaming validation** — Fast rule-based checks per chunk, full LLM validation on complete response
5. **Custom validators** — Domain-specific validators for ORCHA (e.g., validate CRM field formats, workflow step schemas)

**Roadmap stage:** Stage 1
**Budget/timeline:** 1-2 weeks; Guardrails AI sidecar or custom Rust JSON schema validators.

---

## 4. Agent Safety

### 4.1 Permission Boundaries

**Tiered permission model for ORCHA agents:**

| Tier | Access Level | Example Agents | Approval Required |
|---|---|---|---|
| **Read-only** | Query data, read files, search | Research agents, analytics | No |
| **Write-scoped** | Modify specific resources (tasks, CRM records) | CRM automation, task management | Configurable |
| **Code execution** | Run code in sandbox | Coding agents, workflow automation | Yes (first run) |
| **System** | Infrastructure access, API keys, external services | Deployment agents, integration agents | Always |
| **Admin** | User management, configuration changes | Platform management | Always + multi-approval |

**Implementation pattern:**
- Define permissions as capability tokens attached to agent sessions
- Enforce at the API gateway layer (Axum middleware)
- Log all permission checks to audit trail
- Time-boxed permissions (auto-expire after session/task completion)

### 4.2 Resource Limits

| Resource | Mechanism | Implementation |
|---|---|---|
| **CPU** | cgroups v2 CPU quotas | Container/sandbox level |
| **Memory** | cgroups v2 memory limits + OOM kill | Container/sandbox level |
| **Network** | Network policies, egress allow-lists | Kubernetes NetworkPolicy / iptables |
| **File system** | Read-only root, writable tmpfs with size limit | Container security context |
| **Token budget** | Per-request and per-session token caps | Application layer (see Section 6) |
| **Time** | Execution timeout per agent action | Application layer + container level |
| **API calls** | Rate limiting per agent per external service | Application layer middleware |

### 4.3 Human-in-the-Loop Approval Workflows

**Pattern: Propose-Review-Commit**

The most reliable HITL pattern separates "propose" from "commit":

1. **Propose:** Agent stores a structured action payload in a durable store (SQLite/Postgres) and presents it to a human reviewer via the ORCHA UI.
2. **Review:** Human can approve as-is, modify before execution, or reject with feedback.
3. **Commit:** Execute with strict checks — idempotency keys, precondition validation, post-action verification.

**Decision types:**
- Approve (execute as proposed)
- Modify (edit action parameters, then execute)
- Reject (with feedback sent back to agent)

**Approval routing:**
- Risk-based: Low-risk actions auto-approve, high-risk require human review
- Configurable per organization, project, or agent type
- Escalation chains: if primary reviewer doesn't respond within SLA, escalate

**Audit:** Every propose/review/commit creates an immutable audit record linking agent action, human decision, and outcome.

### 4.4 Rollback Capabilities

| Strategy | Description | Use Case |
|---|---|---|
| **Soft delete** | Mark records as deleted, retain data | CRM changes, task modifications |
| **Event sourcing** | Store all state changes as events, replay to any point | Complex workflow state |
| **Compensating transactions** | Execute reverse operations | External API calls, integrations |
| **Canary deployment** | Test agent changes on small slice before full rollout | Agent behavior changes |
| **Snapshot/restore** | Database snapshots before bulk operations | Batch CRM imports, data migrations |

**Recommendation for ORCHA:** Event sourcing for agent actions is the most flexible approach. Store every agent action as an event with full payload. Enables replay, rollback, and audit trail in one system.

### 4.5 Agent Sandboxing

**Three-tier isolation model (from research):**

| Tier | Technology | Security Level | Use Case | Overhead |
|---|---|---|---|---|
| **Tier 1: MicroVMs** | Firecracker, Kata Containers | Highest (hardware boundary) | Untrusted code execution, external agent code | ~125ms boot, ~30MB memory |
| **Tier 2: User-space kernel** | gVisor (runsc) | High (syscall interception) | Compute-heavy agents with limited I/O | Moderate |
| **Tier 3: Hardened containers** | Docker + seccomp + AppArmor + capability dropping | Medium | Trusted internal automation | Low |

**Critical warning:** Containers alone are NOT sufficient for executing untrusted AI-generated code. The shared kernel architecture makes them unsuitable for adversarial code execution. Use microVMs (Firecracker) for any agent that generates and executes code from untrusted inputs.

**Defense-in-depth for sandboxes:**
1. **Seccomp-BPF profiles** — Custom deny-by-default syscall filtering. Docker's default blocks many syscalls, but custom profiles provide stronger protection. Advanced: seccomp user notifications (SECCOMP_RET_USER_NOTIF) for brokered syscalls.
2. **Network isolation** — No network access by default; explicit allow-list for required endpoints.
3. **Filesystem isolation** — Read-only root filesystem, writable tmpfs with size limits.
4. **Time limits** — Hard execution timeouts with SIGKILL escalation.
5. **Monitoring** — System call auditing, resource usage tracking.

**Roadmap stage:** Tier 3 at Stage 1, Tier 2 at Stage 2, Tier 1 at Stage 3
**Budget/timeline:** Tier 3: 1 week. Tier 2: 2 weeks + gVisor ops. Tier 1: 3-4 weeks + Firecracker infrastructure.

---

## 5. Audit and Transparency

### 5.1 Decision Audit Trails

**Requirements for ORCHA:**

Every agent action must produce an immutable audit record containing:
- Timestamp (UTC, nanosecond precision)
- Agent ID, session ID, user ID
- Action type and full payload
- Input context (what information the agent had)
- Decision rationale (model output, chain-of-thought if available)
- Approval status (auto-approved, human-approved, rejected)
- Outcome (success, failure, rollback)
- Model version, prompt version, temperature/params

**Storage pattern:**
- Append-only table in primary database (SQLite for dev, Postgres for production)
- Signed log entries (HMAC or digital signature) for tamper detection
- Retention policy aligned with compliance requirements (EU AI Act: minimum documentation period)

**Roadmap stage:** Stage 1 (foundational requirement)
**Budget/timeline:** 1-2 weeks for core implementation.

### 5.2 Explainability

| Approach | Description | Complexity | Stage |
|---|---|---|---|
| **Chain-of-thought logging** | Store model's reasoning steps alongside decisions | Low | Stage 1 |
| **Input attribution** | Track which inputs most influenced the output | Medium | Stage 2 |
| **Decision tree visualization** | UI showing agent's decision path | Medium | Stage 2-3 |
| **Counterfactual explanations** | "Output would have been X if input Y changed" | High | Stage 4+ |
| **SHAP/LIME for embeddings** | Feature importance for embedding-based decisions | High | Stage 4+ |

**Practical for ORCHA Stage 1-2:** Chain-of-thought logging (store the model's "thinking" alongside the action) and input attribution (which documents/context influenced the decision).

### 5.3 Model Cards

Model cards are "nutrition labels for AI systems." By 2026, 70%+ of enterprises require vendors to provide model cards (Gartner).

**ORCHA should generate model cards for:**
- Each agent type (what model, what training data, what limitations)
- Each workflow definition (what models used, what data processed, what guardrails applied)
- Each LLM provider integration (capabilities, limitations, known biases)

**Template fields:** Model name, version, intended use, out-of-scope uses, training data summary, evaluation metrics, ethical considerations, known limitations.

**Roadmap stage:** Stage 2-3
**Budget/timeline:** 1 week for template + auto-generation from agent configs.

### 5.4 Compliance Reporting

**Key frameworks for 2026:**

| Framework | Scope | Relevance to ORCHA |
|---|---|---|
| **EU AI Act** | All AI systems in EU market | High (if serving EU customers) |
| **NIST AI RMF** | US AI risk management | High |
| **ISO/IEC 42001** | AI management system | Medium |
| **SOC 2 Type II** | Service organization controls | High (SaaS) |

**Required compliance deliverables:**
1. Control catalog — each safeguard and how it's enforced at runtime
2. Compliance matrix — map controls to EU AI Act, NIST RMF, ISO 42001 clauses
3. Risk register — owners, mitigations, evidence for each risk

**Roadmap stage:** Stage 3 (before enterprise customers)
**Budget/timeline:** 4-6 weeks for initial compliance package. Ongoing maintenance.

---

## 6. Cost Control

### 6.1 Token Budget Management

**Hierarchical budget architecture:**

```
Organization
  |-- Team
       |-- User / Virtual Key
            |-- Agent Session
                 |-- Individual Request
```

Each level has:
- **Rate limits** (tokens per minute/hour) for spike protection
- **Quotas** (tokens per day/month) for hard budget caps
- **Tracking dimensions:** Input tokens, output tokens, total tokens (tracked independently)

**Proactive token estimation:**
- Estimate token count BEFORE sending to LLM
- Block abusive requests preemptively (don't wait for the bill)
- Use tiktoken or similar for fast client-side estimation

**Implementation in Rust/Axum:**
- Middleware layer that intercepts all LLM proxy requests
- Token counting via tiktoken-rs crate
- Budget state in Redis or SQLite (per-user/team/org counters)
- Configurable limits via admin UI

### 6.2 Circuit Breakers

| Trigger | Action | Recovery |
|---|---|---|
| **Budget exhausted** | Block requests, return graceful error | Manual budget increase or next billing period |
| **Error rate spike** | Switch to fallback model | Auto-retry primary after cooldown |
| **Latency spike** | Timeout + fallback to cheaper/faster model | Auto-retry primary after cooldown |
| **Runaway agent** (token burn rate anomaly) | Kill agent session, alert admin | Manual review required |
| **Global spend cap** | Redirect all requests to cheapest model | Admin override |

**Pattern for ORCHA:**
- Implement as Axum middleware on the LLM proxy endpoint
- State machine: Closed -> Open -> Half-Open (standard circuit breaker)
- Configurable thresholds per organization

### 6.3 Model Fallback Strategies

**Cascade pattern (expensive to cheap):**

```
Primary: Claude Opus / GPT-4o (complex reasoning)
  |-- Fallback 1: Claude Sonnet / GPT-4o-mini (moderate tasks)
       |-- Fallback 2: Claude Haiku / GPT-3.5-turbo (simple tasks)
            |-- Fallback 3: Local model (emergency/offline)
```

**Routing logic:**
- Default to cheapest model that meets task requirements
- Upgrade to expensive model only when cheap model fails quality threshold
- Degrade gracefully when budget is constrained
- Track cost per model per task type for optimization

### 6.4 Cost Alerts

- Real-time cost tracking dashboard in ORCHA admin UI
- Configurable alert thresholds (50%, 75%, 90%, 100% of budget)
- Notification channels: in-app, email, webhook
- Daily/weekly cost summary reports

**Roadmap stage:** Basic budget tracking at Stage 1, circuit breakers at Stage 2, full cascade/alerts at Stage 3
**Budget/timeline:** Stage 1: 1-2 weeks. Stage 2: 2-3 weeks. Stage 3: 2 weeks.

---

## 7. Evaluation and Monitoring

### 7.1 Langfuse (LLM Observability)

- **License:** MIT (self-hosted), commercial cloud option
- **Language:** TypeScript/Node.js (server), SDKs for Python/JS/OpenTelemetry
- **GitHub:** [langfuse/langfuse](https://github.com/langfuse/langfuse)

**What it does:**
Open-source LLM engineering platform for observability, metrics, evals, prompt management, and datasets. Captures traces of LLM calls, retrieval, embedding, and agent actions.

**Key capabilities:**
- **Tracing:** Full request lifecycle with latency, cost, token usage per step
- **Prompt management:** Version control, caching, A/B testing of prompts
- **Evaluations:** LLM-as-a-judge, user feedback, manual labeling, custom eval pipelines
- **Datasets:** Test set management with versioning
- **Self-hosting:** Docker Compose or Kubernetes (Helm chart)
- **Integrations:** OpenTelemetry, LangChain, OpenAI SDK, LiteLLM
- **Security:** Deployable within VPC / on-premises

**Pros:**
- MIT license
- Self-hostable (data sovereignty)
- Comprehensive feature set (tracing + prompts + evals)
- OpenTelemetry native integration
- Active development (v1.12+ in 2026)
- Strong Rust compatibility via OpenTelemetry SDK

**Cons:**
- Requires Postgres + Redis infrastructure
- UI is functional but not beautiful
- Some features (advanced analytics) are cloud-only
- Learning curve for full feature utilization

**ORCHA integration:**
- Self-host alongside ORCHA backend
- Instrument all LLM calls via OpenTelemetry (Rust tracing crate -> OTLP -> Langfuse)
- Use prompt management for all agent system prompts
- A/B test prompt versions with quality metrics
- Feed evaluation scores back into cost optimization

**Roadmap stage:** Stage 1 (observability is foundational)
**Budget/timeline:** 1-2 weeks for deployment + instrumentation; infrastructure: Postgres + 1-2GB RAM.

### 7.2 LangSmith (Reference Architecture)

- **License:** Proprietary (LangChain Inc.)
- **Relevance:** Important reference for feature parity, not for adoption

**Key patterns to replicate in Langfuse/custom:**
- Run-level tracing with parent/child spans
- Feedback collection (thumbs up/down, corrections)
- Automated evaluation pipelines triggered on trace ingestion
- Dataset-driven regression testing
- Hub for shared prompts and evaluations

### 7.3 Prompt Versioning and A/B Testing

**Pattern for ORCHA:**

1. **Version control:** Every prompt is versioned (semver). Store in database with metadata.
2. **Labeling:** Tag versions (e.g., "production", "canary", "experiment-A").
3. **Traffic splitting:** Route N% of requests to canary prompt version.
4. **Metrics collection:** Per-version tracking of latency, cost, token usage, quality scores.
5. **Statistical significance:** Wait for sufficient sample size before promoting/rolling back.
6. **Traceability:** Every evaluation score links back to exact prompt version + model + dataset.

**Langfuse native support:** Langfuse provides built-in prompt versioning and A/B testing with metric tracking per version.

### 7.4 Quality Metrics

| Metric | Description | Measurement Method |
|---|---|---|
| **Accuracy** | Correctness of factual claims | RAG grounding score, human eval |
| **Relevance** | Output answers the actual question | LLM-as-judge, cosine similarity |
| **Completeness** | All required information present | Schema validation, checklist |
| **Toxicity** | Harmful content score | LLM Guard, Perspective API |
| **Hallucination rate** | % of outputs containing fabricated info | Cross-reference validation |
| **Latency** | Time to first token, total response time | OpenTelemetry spans |
| **Cost** | Tokens consumed per request | Token counting middleware |
| **User satisfaction** | Thumbs up/down, corrections | In-app feedback collection |

### 7.5 Drift Detection

**What to monitor for drift:**
- **Input distribution shift:** Are users asking different types of questions?
- **Output quality shift:** Are quality scores trending down over time?
- **Cost drift:** Is average cost per request increasing?
- **Latency drift:** Is response time increasing?
- **Error rate drift:** Are more requests failing?

**Implementation:**
- Rolling window statistics (mean, P50, P95, P99) for all metrics
- Alerting when metrics deviate beyond configurable thresholds
- Automatic comparison against baseline period
- Langfuse dashboards for visualization

**Roadmap stage:** Basic metrics at Stage 1, A/B testing at Stage 2, drift detection at Stage 3
**Budget/timeline:** Incremental with Langfuse adoption.

---

## 8. Rust/ORCHA Integration Considerations

### 8.1 The Python Sidecar Problem

Most guardrails frameworks (NeMo, Guardrails AI, LLM Guard, Presidio, Rebuff) are Python-only. For a Rust/Axum backend, integration options:

| Approach | Description | Pros | Cons |
|---|---|---|---|
| **Python sidecar service** | Run Python framework as separate HTTP microservice | Full feature access, clean separation | Operational complexity, latency overhead, two runtimes |
| **PyO3/Maturin** | Call Python from Rust via FFI | Single process, lower latency | Complex build, GIL contention, deployment complexity |
| **Rust-native reimplementation** | Rewrite needed guardrails in Rust | Best performance, single runtime | High dev effort, maintenance burden |
| **WASM plugins** | Compile guardrails to WASM, run in Rust | Sandboxed, portable | Limited library support, WASM ecosystem immature |
| **gRPC/protobuf** | Python service with gRPC interface | Strong typing, efficient serialization | Still two runtimes, gRPC complexity |

**Recommendation:** Start with **Python sidecar** (HTTP microservice) for rapid feature adoption. Incrementally rewrite performance-critical guardrails in Rust as the platform matures.

### 8.2 Rust-Native Capabilities (No External Dependencies)

These can be implemented directly in the Axum backend:

| Capability | Rust Implementation |
|---|---|
| Token counting | `tiktoken-rs` crate |
| JSON schema validation | `jsonschema` crate |
| Rate limiting | `governor` crate or custom middleware |
| Input sanitization | Custom regex + `unicode-normalization` crate |
| Budget tracking | SQLite counters + Axum middleware |
| Audit logging | `tracing` crate + structured logging |
| Circuit breakers | Custom state machine in middleware |
| Permission checks | Axum extractors + middleware |
| Request/response logging | `tower-http` tracing layer |

### 8.3 OpenTelemetry for Unified Observability

ORCHA's Rust backend can emit traces via the `opentelemetry` and `tracing-opentelemetry` crates, which Langfuse can ingest natively. This provides:
- End-to-end request tracing (frontend -> backend -> LLM -> guardrails)
- Automatic latency/error tracking
- Custom span attributes for safety metrics
- No Python dependency for observability

---

## 9. Consolidated Roadmap Recommendations

### Stage 0 (Foundation — Now)

| Item | Tool/Pattern | Effort | Priority |
|---|---|---|---|
| Prompt structure isolation | Architectural (system/user/context separation) | 1-2 days | P0 |
| Input length limits | Rust middleware | 1 day | P0 |
| Basic rate limiting | `governor` crate | 1 day | P0 |
| Structured logging | `tracing` crate (already in place) | Existing | P0 |

### Stage 1 (Core Safety — Next 1-2 Months)

| Item | Tool/Pattern | Effort | Priority |
|---|---|---|---|
| **Langfuse deployment** | Self-hosted Docker | 1-2 weeks | P0 |
| **LLM Guard sidecar** | Python microservice (input/output scanning) | 1 week | P0 |
| **Presidio PII scanning** | Python sidecar or bundled with LLM Guard | 1 week | P0 |
| **Audit trail table** | SQLite/Postgres append-only log | 1 week | P0 |
| **Token budget tracking** | Rust middleware + `tiktoken-rs` | 1-2 weeks | P1 |
| **Permission model** | Axum middleware + capability tokens | 1-2 weeks | P1 |
| **Output schema validation** | `jsonschema` crate (Rust-native) | 1 week | P1 |
| **OpenAI Moderation API** | HTTP client integration | 2-3 days | P1 |
| **Hardened containers** (Tier 3) | Docker + seccomp + AppArmor | 1 week | P1 |
| **Chain-of-thought logging** | Store reasoning in audit trail | 3-5 days | P2 |

### Stage 2 (Enhanced Safety — Months 3-6)

| Item | Tool/Pattern | Effort | Priority |
|---|---|---|---|
| **Guardrails AI sidecar** | Output validation with Pydantic validators | 1-2 weeks | P1 |
| **HITL approval workflows** | Propose-Review-Commit in ORCHA UI | 2-3 weeks | P0 |
| **Circuit breakers** | Rust state machine middleware | 1-2 weeks | P1 |
| **Model fallback cascade** | LLM proxy with fallback logic | 1-2 weeks | P1 |
| **Prompt versioning + A/B testing** | Langfuse prompt management | 1 week | P1 |
| **RAG grounding checks** | Factual consistency scoring | 2 weeks | P2 |
| **gVisor sandboxing** (Tier 2) | For compute-heavy agent tasks | 2 weeks | P2 |
| **Cost alerts + dashboard** | Admin UI + notification system | 1-2 weeks | P2 |
| **Input attribution logging** | Track which context influenced output | 1-2 weeks | P2 |

### Stage 3 (Enterprise Safety — Months 6-12)

| Item | Tool/Pattern | Effort | Priority |
|---|---|---|---|
| **NeMo Guardrails** | Full dialog/retrieval rail integration | 2-3 weeks | P1 |
| **Firecracker microVMs** (Tier 1) | Untrusted code execution | 3-4 weeks | P1 |
| **Compliance reporting** | EU AI Act, NIST RMF, ISO 42001 | 4-6 weeks | P1 |
| **Model cards** | Auto-generated from agent configs | 1 week | P2 |
| **Bias detection** | LLM Guard + custom pipelines | 2 weeks | P2 |
| **Canary tokens** | Rebuff or custom implementation | 1-2 weeks | P2 |
| **Drift detection** | Statistical monitoring on Langfuse metrics | 2 weeks | P2 |
| **Event sourcing for rollback** | Append-only event store for agent actions | 3-4 weeks | P2 |

### Stage 4-5 (Advanced — 12+ Months)

| Item | Tool/Pattern | Effort | Priority |
|---|---|---|---|
| Attention pathway analysis (FACTUM) | Hallucination detection from model internals | Research | P3 |
| Metamorphic testing (MetaQA) | Prompt mutation for consistency testing | 2-3 weeks | P3 |
| Counterfactual explanations | "What if" analysis for agent decisions | Research | P3 |
| WASM plugin system for guardrails | Sandboxed, portable guardrail execution | 4-6 weeks | P3 |
| Rust-native guardrails rewrite | Replace Python sidecars with Rust | Ongoing | P3 |

---

## License Summary

| Tool | License | Commercial Use | Modification | Distribution |
|---|---|---|---|---|
| NeMo Guardrails | Apache 2.0 | Yes | Yes | Yes (with notice) |
| Guardrails AI | Apache 2.0 | Yes | Yes | Yes (with notice) |
| LLM Guard | MIT | Yes | Yes | Yes |
| Rebuff | Apache 2.0 | Yes | Yes | Yes (with notice) |
| Microsoft Presidio | MIT | Yes | Yes | Yes |
| Langfuse | MIT (self-hosted) | Yes | Yes | Yes |
| OpenAI Moderation | API (free) | Yes | N/A | N/A |
| Perspective API | API (free) | Yes | N/A | N/A |

All recommended tools use permissive licenses (MIT or Apache 2.0) compatible with commercial use and ORCHA's licensing model.

---

## Key Takeaways

1. **No single tool solves AI safety.** Defense-in-depth with layered guardrails is the only viable strategy. Even frontier models remain vulnerable to prompt injection.

2. **Start with observability.** Langfuse (MIT, self-hosted) should be Stage 1 priority. You can't improve what you can't measure.

3. **LLM Guard is the best Stage 1 guardrails choice** — MIT licensed, self-hosted, 35 scanners, provider-agnostic, and lightweight enough for early adoption.

4. **The Python sidecar approach is pragmatic.** Most guardrails tools are Python. Fighting this reality with Rust-native rewrites is premature. Deploy Python services alongside the Rust backend and optimize later.

5. **Human-in-the-loop is non-negotiable for agent platforms.** The Propose-Review-Commit pattern should be a Stage 2 priority with the ORCHA UI built to support it.

6. **Cost control prevents existential risk.** A runaway agent can burn through thousands of dollars of API credits in minutes. Token budget enforcement and circuit breakers are Stage 1 requirements.

7. **Audit trails are foundational.** Every agent action needs an immutable record. This is both a compliance requirement (EU AI Act) and an engineering necessity for debugging.

8. **Agent sandboxing must match the threat model.** Hardened containers for trusted agents (Stage 1), gVisor for semi-trusted (Stage 2), Firecracker microVMs for untrusted code execution (Stage 3).

---

## Sources

- [NVIDIA NeMo Guardrails — Developer](https://developer.nvidia.com/nemo-guardrails)
- [NVIDIA NeMo Guardrails — GitHub](https://github.com/NVIDIA-NeMo/Guardrails)
- [NVIDIA NeMo Guardrails — Overview](https://docs.nvidia.com/nemo/guardrails/latest/about/overview.html)
- [Guardrails AI — GitHub](https://github.com/guardrails-ai/guardrails)
- [AI Guardrails: Complete Guide for LLMs (Jan 2026)](https://www.openlayer.com/blog/post/ai-guardrails-llm-guide)
- [AI Agent Guardrails: Production Guide for 2026](https://authoritypartners.com/insights/ai-agent-guardrails-production-guide-for-2026/)
- [LLM Guard — Protect AI](https://protectai.com/llm-guard)
- [LLM Guard 2026: Open-Source Safety](https://appsecsanta.com/llm-guard)
- [Rebuff — GitHub](https://github.com/protectai/rebuff)
- [Rebuff: Detecting Prompt Injection Attacks — LangChain Blog](https://blog.langchain.com/rebuff/)
- [Microsoft Presidio — Home](https://microsoft.github.io/presidio/)
- [Microsoft Presidio — GitHub](https://github.com/microsoft/presidio)
- [Langfuse — Self-Hosting](https://langfuse.com/self-hosting)
- [Langfuse — Observability Overview](https://langfuse.com/docs/observability/overview)
- [Langfuse — A/B Testing](https://langfuse.com/docs/prompt-management/features/a-b-testing)
- [Langfuse — LLM Security Monitoring](https://langfuse.com/guides/cookbook/example_llm_security_monitoring)
- [Prompt Injection: Comprehensive Review (MDPI)](https://www.mdpi.com/2078-2489/17/1/54)
- [Prompt Injection: Top AI Threat 2026](https://dev.to/cyberpath/prompt-injection-attacks-the-top-ai-threat-in-2026-and-how-to-defend-against-it-an0)
- [MCP Security Vulnerabilities 2026](https://www.practical-devsecops.com/mcp-security-vulnerabilities/)
- [How to Sandbox AI Agents in 2026 — Northflank](https://northflank.com/blog/how-to-sandbox-ai-agents)
- [Best Code Execution Sandbox for AI Agents 2026 — Northflank](https://northflank.com/blog/best-code-execution-sandbox-for-ai-agents)
- [Container Escape Vulnerabilities: AI Agent Security 2026 — Blaxel](https://blaxel.ai/blog/container-escape)
- [Human-in-the-Loop AI Agents — StackAI](https://www.stackai.com/insights/human-in-the-loop-ai-agents-how-to-design-approval-workflows-for-safe-and-scalable-automation)
- [Human-in-the-Loop Best Practices — Permit.io](https://www.permit.io/blog/human-in-the-loop-for-ai-agents-best-practices-frameworks-use-cases-and-demo)
- [Human-in-the-Loop — Temporal](https://docs.temporal.io/ai-cookbook/human-in-the-loop-python)
- [FACTUM: Citation Hallucination Detection](https://arxiv.org/pdf/2601.05866)
- [LLM Hallucinations Guide — Lakera](https://www.lakera.ai/blog/guide-to-hallucinations-in-large-language-models)
- [AI Regulations and Governance 2026](https://sombrainc.com/blog/ai-regulations-2026-eu-ai-act)
- [AI Audit Trails — Aptus Data Labs](https://www.aptusdatalabs.com/thought-leadership/the-rise-of-ai-audit-trails-ensuring-traceability-in-decision-making)
- [Top 13 AI Compliance Tools 2026](https://www.centraleyes.com/top-ai-compliance-tools/)
- [LLM Cost Control Guide 2026 — Techdim](https://techdim.com/llm-cost-control-for-your-business-practical-guide-for-2026/)
- [LLM Token Optimization 2026 — Redis](https://redis.io/blog/llm-token-optimization-speed-up-apps/)
- [Traefik Labs: Token-Level Cost Controls](https://aijourn.com/traefik-labs-advances-llm-and-mcp-runtime-governance-with-composable-safety-pipeline-multi-provider-resilience-and-token-level-cost-controls/)
- [8 LLM Evaluation Tools 2026 — TechHQ](https://techhq.com/news/8-llm-evaluation-tools-you-should-know-in-2026/)
- [OpenAI Moderation API](https://developers.openai.com/api/docs/guides/moderation)
- [OpenAI Safety Best Practices](https://platform.openai.com/docs/guides/safety-best-practices)
- [Best LLM Security Tools 2026 — Deepchecks](https://www.deepchecks.com/top-llm-security-tools-frameworks/)
