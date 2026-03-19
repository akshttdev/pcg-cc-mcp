Now I have comprehensive data. Let me compile the full research report.

---

# Open-Source Tool Research for AI Agent Orchestration (Early 2026)

## 1. MCP SDKs and Servers

### Official Rust MCP SDK (`rmcp`)
- **License:** MIT (permissive)
- **GitHub:** [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) — ~2.7k stars
- **Rust/TS:** Rust-native, tokio async runtime
- **MCP support:** IS the official Rust MCP implementation
- **Maturity:** Production-ready, actively maintained by Anthropic
- **ORCHA fit:** Direct integration path for ORCHA's Rust/Axum backend to expose and consume MCP tools natively

### `rust-mcp-sdk` (Community)
- **License:** MIT
- **GitHub:** [rust-mcp-sdk on crates.io](https://crates.io/crates/rust-mcp-sdk)
- **MCP support:** Full MCP 2025-11-25 spec with backward compat
- **Maturity:** Actively maintained alternative to official SDK

### Official TypeScript MCP SDK
- **License:** MIT (existing code) / Apache 2.0 (new contributions)
- **GitHub:** [modelcontextprotocol/typescript-sdk](https://github.com/modelcontextprotocol/typescript-sdk)
- **ORCHA fit:** Frontend/Node.js side MCP integration; could power MCP client in ORCHA's Vite frontend or Node tooling

### Notable Open-Source MCP Servers
- **Official MCP Servers repo** ([modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers)) — reference servers for filesystem, GitHub, Postgres, SQLite, Slack, etc. MIT licensed.
- **Twenty CRM MCP Server** ([twenty-crm-mcp-server](https://github.com/mhenry3164/twenty-crm-mcp-server)) — CRUD + search across people, companies, tasks, notes. Relevant to ORCHA's CRM module.
- **Supabase MCP Server** — bridges Postgres to LLMs, relevant for ORCHA's SQLite/data layer.
- **Memory MCP Server** — knowledge-graph style memory for entity/relationship tracking (people, tasks, projects). Directly maps to ORCHA's CRM contact/deal architecture.
- **ORCHA fit:** ORCHA could expose its CRM, task management, and workflow data as MCP servers, enabling Claude/agents to interact with ORCHA data natively.

---

## 2. A2A Protocol Implementations

### Google Agent2Agent (A2A)
- **License:** Apache 2.0 (donated to Linux Foundation Feb 2026)
- **GitHub:** [a2aproject/A2A](https://github.com/a2aproject/A2A)
- **Rust/TS:** Python SDK is primary; TypeScript support exists. No official Rust SDK yet.
- **MCP support:** Designed to complement MCP (MCP = tools/context, A2A = agent-to-agent communication)
- **Maturity:** v0.3 released, backed by 50+ partners (Google, Microsoft, Salesforce, SAP, AWS, Cisco). Now under Linux Foundation governance.
- **Key features:** gRPC support, signed security cards, Agent Cards for discovery
- **ORCHA fit:** If ORCHA agents need to communicate with external agents (e.g., customer-facing bots, third-party AI services), A2A provides the interop layer. Complements MCP rather than replacing it.

---

## 3. Agent Orchestration Frameworks

### LangGraph
- **License:** MIT
- **GitHub:** [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) — ~38M monthly PyPI downloads
- **Rust/TS:** Python primary, TypeScript SDK available. No Rust SDK.
- **MCP support:** Agents deployed on LangGraph Platform auto-expose as MCP endpoints at `/mcp`
- **Maturity:** Stable 1.0 (Oct 2025). Production-ready.
- **ORCHA fit:** Could orchestrate ORCHA's AI agents in Python/TS. Graph-based state machines map well to ORCHA's workflow editor concept. MCP exposure is automatic.

### CrewAI
- **License:** MIT
- **GitHub:** [crewAIInc/crewAI](https://github.com/crewAIInc/crewAI) — ~45.9k stars
- **Rust/TS:** Python only. No Rust or TypeScript SDK.
- **MCP support:** First-class MCP integration with DSL-based server references and 3 transport layers. Also supports A2A.
- **Maturity:** v1.10.1 as of March 2026. Very active.
- **ORCHA fit:** Strong for role-based multi-agent collaboration (e.g., "researcher agent" + "writer agent" + "reviewer agent"). Python-only is a limitation for ORCHA's Rust stack.

### Microsoft Semantic Kernel / Agent Framework
- **License:** MIT
- **GitHub:** [microsoft/semantic-kernel](https://github.com/microsoft/semantic-kernel) — ~27k stars
- **Rust/TS:** C#, Python, Java. No Rust or TypeScript SDK.
- **MCP support:** First-class — can act as both MCP client and server (Python v1.28.1+). Supports stdio, SSE, WebSocket transports.
- **A2A support:** Yes, via Microsoft Agent Framework (RC Feb 2026)
- **Maturity:** Production-grade. Semantic Kernel + AutoGen merged into unified "Microsoft Agent Framework" (MIT, RC Feb 2026).
- **ORCHA fit:** Enterprise-grade, but .NET/Python focus limits direct Rust integration. Could be used as a sidecar service.

### Jido
- **License:** Apache 2.0
- **GitHub:** [agentjido/jido](https://github.com/agentjido/jido)
- **Rust/TS:** Elixir/BEAM only. No Rust or TypeScript.
- **MCP support:** Not documented
- **Maturity:** v2.0 released. Niche but production-hardened on BEAM.
- **ORCHA fit:** Interesting architecture (OTP supervisors, fault-tolerant agents) but Elixir-only. Could inspire ORCHA's agent design patterns but not directly integrate.

---

## 4. Workflow Engines

### Temporal.io
- **License:** MIT
- **GitHub:** [temporalio/temporal](https://github.com/temporalio/temporal) — ~12k+ stars
- **Rust/TS:** TypeScript SDK official. Rust SDK is community/experimental.
- **MCP support:** Not native
- **Maturity:** Industry standard for durable execution. Very mature.
- **ORCHA fit:** Excellent for long-running workflows (data source processing, CRM sync). TS SDK works with ORCHA's frontend. Heavyweight for simple tasks.

### Hatchet
- **License:** MIT (100%)
- **GitHub:** [hatchet-dev/hatchet](https://github.com/hatchet-dev/hatchet)
- **Rust/TS:** TypeScript and Python SDKs. Go backend. No Rust SDK.
- **MCP support:** Not native
- **Maturity:** Production-proven (billions of tasks/month). Postgres-backed. YC W24.
- **ORCHA fit:** Strong fit — MIT licensed, Postgres-backed (ORCHA uses SQLite but similar pattern), DAG orchestration + durable execution + task queue in one. TS SDK integrates with ORCHA frontend.

### Restate
- **License:** SDKs are MIT; Runtime is BSL 1.1 (converts to open source after 4 years)
- **GitHub:** [restatedev/restate](https://github.com/restatedev/restate)
- **Rust/TS:** Official Rust SDK AND TypeScript SDK. Runtime written in Rust.
- **MCP support:** Not native
- **Maturity:** Active development, single-binary deployment.
- **ORCHA fit:** BEST Rust-native option. Low-latency durable execution, stateful workflows, transactional state machines. SDKs are MIT. **BSL runtime is a concern** — not truly permissive until conversion date. Evaluate whether self-hosting under BSL terms is acceptable.

### Inngest
- **License:** SSPL (Server Side Public License) with delayed Apache 2.0 after 3 years
- **ORCHA fit:** **DISQUALIFIED** — SSPL is copyleft/restrictive. Not permissive.

### n8n
- **License:** Sustainable Use License (not open source, commercially restricted)
- **ORCHA fit:** **DISQUALIFIED** — Not a permissive open-source license.

### Windmill
- **License:** AGPLv3
- **ORCHA fit:** **DISQUALIFIED** — AGPL is copyleft.

---

## 5. Background Job/Task Runners (Rust/Tokio)

### apalis
- **License:** MIT OR Apache-2.0 (dual-licensed, permissive)
- **Crate:** [apalis on crates.io](https://crates.io/crates/apalis)
- **GitHub:** [apalis-dev/apalis](https://github.com/geofmureithi/apalis)
- **Maturity:** v1.0.0-rc.2 (Jan 2026). Actively maintained.
- **Features:** Tower::Service-based, supports SQLite/Postgres/Redis backends, cron scheduling, retry policies, layered middleware
- **ORCHA fit:** EXCELLENT. Native Rust/Tokio, SQLite backend support matches ORCHA's stack. Tower middleware composability. Cron for scheduled workflows. Most mature Rust job queue.

### tokio-cron-scheduler
- **License:** MIT OR Apache-2.0
- **Crate:** [tokio-cron-scheduler](https://crates.io/crates/tokio-cron-scheduler)
- **Features:** Cron-like scheduling on tokio, optional Postgres/Nats persistence
- **ORCHA fit:** Good for lightweight cron needs. Less feature-rich than apalis.

### sqlxmq
- **License:** MIT OR Apache-2.0
- **Features:** Job queue built on sqlx + PostgreSQL. Lightweight, SQL-native.
- **ORCHA fit:** Good pattern reference but PostgreSQL-only (ORCHA uses SQLite).

---

## 6. FSM Libraries for Rust

### rust-fsm
- **License:** MIT
- **GitHub:** [eugene-babichenko/rust-fsm](https://github.com/eugene-babichenko/rust-fsm)
- **Crate:** [rust-fsm on crates.io](https://crates.io/crates/rust-fsm)
- **Features:** DSL macro for readable specs, Mermaid diagram generation, no_std support
- **Maturity:** v0.8, actively maintained
- **ORCHA fit:** Good for modeling task/workflow state transitions (todo -> inprogress -> done). Lightweight, compile-time checked.

### edfsm (Event-Driven FSM)
- **License:** Check crate (likely MIT/Apache-2.0)
- **Crate:** [edfsm on crates.io](https://crates.io/crates/edfsm)
- **Features:** Event-driven state machines, more advanced patterns
- **ORCHA fit:** Better fit if ORCHA workflows are event-driven (which they are — data source events trigger workflow steps).

---

## 7. Circuit Breaker / Resilience (Rust)

### backon
- **License:** Apache 2.0
- **GitHub:** [Xuanwo/backon](https://github.com/Xuanwo/backon) — 4.4M downloads/month, used in 980 crates
- **Features:** Retry with backoff (constant, exponential, jittered). Works with async/await natively. Reached v1.0.
- **ORCHA fit:** BEST choice for retry/backoff. Production-proven, massive adoption, clean async API. Use for LLM API calls, external service retries.

### failsafe-rs
- **License:** MIT (check repo)
- **GitHub:** [dmexe/failsafe-rs](https://github.com/dmexe/failsafe-rs)
- **Features:** Circuit breaker with consecutive_failures and success_rate policies. Supports both sync and async (futures).
- **ORCHA fit:** Classic circuit breaker for LLM provider outages. Complements backon (retry + circuit breaker together).

### tower-circuitbreaker
- **License:** Check crate
- **Features:** Tower middleware circuit breaker. Composable with other Tower layers.
- **ORCHA fit:** Good if ORCHA standardizes on Tower for HTTP service middleware (Axum is Tower-based).

### tower-resilience
- **License:** Check crate
- **Features:** Comprehensive resilience toolkit inspired by Resilience4j. Circuit breaker + rate limiter + bulkhead + retry in one package.
- **ORCHA fit:** Most complete option if ORCHA wants a unified resilience layer across all Tower services.

---

## 8. Cost Tracking for LLM Calls

### Langfuse
- **License:** MIT (core), enterprise features commercially licensed
- **GitHub:** [langfuse/langfuse](https://github.com/langfuse/langfuse) — 7M+ monthly SDK installs, 8k+ self-hosted instances
- **Rust/TS:** Python and TypeScript SDKs. No Rust SDK. OpenTelemetry integration enables language-agnostic tracing.
- **Features:** Token/cost tracking with predefined model pricing (OpenAI, Anthropic, Google). Prompt management, evals, datasets, playground.
- **MCP support:** Not native
- **ORCHA fit:** BEST option. Self-hostable, MIT core. Can integrate via OpenTelemetry from Rust backend. Tracks per-call costs, token usage, latency. Dashboard for observability.

### tokencost
- **License:** MIT
- **GitHub:** [AgentOps-AI/tokencost](https://github.com/AgentOps-AI/tokencost)
- **Features:** Python library for estimating USD cost of LLM API calls (400+ models)
- **ORCHA fit:** Lightweight cost estimation. Python-only but could be used in preprocessing/analysis scripts.

---

## 9. Model Routing / LLM Gateways

### LiteLLM
- **License:** MIT (core), enterprise features separately licensed
- **GitHub:** [BerriAI/litellm](https://github.com/BerriAI/litellm) — 33k+ stars
- **Rust/TS:** Python SDK + proxy server. OpenAI-compatible API means any HTTP client (including Rust) can use it.
- **MCP support:** Yes, recent Agent Hub + MCP support added
- **Features:** 100+ LLM providers, unified OpenAI-compatible API, routing, retries, fallbacks, spend tracking per virtual key, load balancing, guardrails
- **ORCHA fit:** EXCELLENT. Deploy as a sidecar proxy. ORCHA's Rust backend calls LiteLLM's OpenAI-compatible API. Gets model routing, cost tracking, fallbacks, and rate limiting for free. MIT core means no license concerns.

### Bifrost
- **License:** Open source (check specific license)
- **Features:** High-performance AI gateway with built-in cost/usage monitoring. Unified interface across providers.
- **ORCHA fit:** Alternative to LiteLLM if lower-level gateway is preferred.

---

## 10. Prompt Caching

No standalone open-source tools exist specifically for prompt caching — it is a provider-side feature. Key patterns:

### Anthropic Prompt Caching
- **How it works:** Manual — mark sections with `cache_control` breakpoints (up to 4 per prompt). 5-minute TTL (refreshes on use). Optional 1-hour TTL at additional cost.
- **Cost impact:** Up to 90% cost reduction, 85% latency reduction for long prompts
- **Min tokens:** 1,024 tokens for caching to activate
- **ORCHA fit:** System prompts, tool definitions, and CRM context injected into agent calls should be marked as cacheable. Structure prompts so static content (system prompt, tool schemas) comes first, dynamic content (user query) last.

### OpenAI Prompt Caching
- **How it works:** Automatic — no code changes needed. Matches prefix of 1,024+ tokens in 128-token increments.
- **Cost impact:** 50% cost reduction, up to 80% latency reduction
- **ORCHA fit:** Free optimization with no code changes. Ensure system prompts are identical across calls.

### LiteLLM Prompt Caching Support
- LiteLLM's proxy transparently supports prompt caching for both Anthropic and OpenAI, passing through cache control directives.
- **ORCHA fit:** If using LiteLLM as gateway, prompt caching is handled at the proxy level.

---

## Summary: Recommended Stack for ORCHA

| Gap | Recommended Tool | License | Why |
|-----|-----------------|---------|-----|
| MCP SDK (Rust) | `rmcp` (official) | MIT | Official, tokio-native, production-ready |
| MCP SDK (TS) | `@modelcontextprotocol/sdk` | MIT | Official TypeScript SDK |
| Agent-to-Agent | A2A Protocol | Apache 2.0 | Linux Foundation backed, complements MCP |
| Agent orchestration | LangGraph (TS) or CrewAI (Python) | MIT / MIT | Best MCP integration. Python-only is the tradeoff |
| Workflow engine | Hatchet | MIT | Fully MIT, Postgres-backed, DAG + durable execution |
| Durable execution (Rust) | Restate | MIT SDKs / BSL runtime | Best Rust-native option; evaluate BSL runtime terms |
| Background jobs (Rust) | apalis | MIT/Apache-2.0 | SQLite backend, Tower middleware, cron support |
| FSM (Rust) | rust-fsm | MIT | Compile-time checked, Mermaid diagrams |
| Retry/backoff | backon | Apache 2.0 | 4.4M downloads/month, v1.0 stable |
| Circuit breaker | failsafe-rs + tower-circuitbreaker | MIT | Async-native, Tower-composable |
| LLM cost tracking | Langfuse | MIT (core) | Self-hostable, OpenTelemetry, comprehensive |
| Model routing | LiteLLM | MIT (core) | 100+ providers, cost tracking, MCP support |
| Prompt caching | Provider-native (Anthropic/OpenAI) | N/A | Use via LiteLLM proxy passthrough |

### Key Disqualifications
- **n8n** — Sustainable Use License (not permissive)
- **Windmill** — AGPLv3 (copyleft)
- **Inngest** — SSPL (copyleft)
- **Restate runtime** — BSL 1.1 (not truly open source until conversion; SDKs ARE MIT)

Sources:
- [Official Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
- [Official TypeScript MCP SDK](https://github.com/modelcontextprotocol/typescript-sdk)
- [rust-mcp-sdk on crates.io](https://crates.io/crates/rust-mcp-sdk)
- [MCP Servers Repository](https://github.com/modelcontextprotocol/servers)
- [A2A Protocol (Google/Linux Foundation)](https://github.com/a2aproject/A2A)
- [A2A Announcement - Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)
- [Linux Foundation A2A Launch](https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents)
- [LangGraph GitHub](https://github.com/langchain-ai/langgraph)
- [CrewAI GitHub (MIT)](https://github.com/crewAIInc/crewAI)
- [Semantic Kernel GitHub (MIT)](https://github.com/microsoft/semantic-kernel)
- [Jido GitHub (Apache 2.0)](https://github.com/agentjido/jido)
- [Temporal GitHub (MIT)](https://github.com/temporalio/temporal)
- [Hatchet GitHub (MIT)](https://github.com/hatchet-dev/hatchet)
- [Restate GitHub](https://github.com/restatedev/restate)
- [Restate Rust SDK](https://github.com/restatedev/sdk-rust)
- [n8n License](https://docs.n8n.io/sustainable-use-license/)
- [Windmill License (AGPL)](https://github.com/windmill-labs/windmill/blob/main/LICENSE-AGPL)
- [Inngest GitHub (SSPL)](https://github.com/inngest/inngest)
- [apalis on crates.io](https://crates.io/crates/apalis)
- [apalis GitHub](https://github.com/geofmureithi/apalis)
- [tokio-cron-scheduler](https://crates.io/crates/tokio-cron-scheduler)
- [rust-fsm GitHub](https://github.com/eugene-babichenko/rust-fsm)
- [backon GitHub (Apache 2.0)](https://github.com/Xuanwo/backon)
- [failsafe-rs GitHub](https://github.com/dmexe/failsafe-rs)
- [tower-circuitbreaker](https://lib.rs/crates/tower-circuitbreaker)
- [Langfuse GitHub (MIT core)](https://github.com/langfuse/langfuse)
- [tokencost GitHub](https://github.com/AgentOps-AI/tokencost)
- [LiteLLM GitHub (MIT core)](https://github.com/BerriAI/litellm)
- [Anthropic Prompt Caching Docs](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
- [LiteLLM Prompt Caching](https://docs.litellm.ai/docs/completion/prompt_caching)
- [Twenty CRM MCP Server](https://github.com/mhenry3164/twenty-crm-mcp-server)
- [Agent Framework Comparison 2026](https://particula.tech/blog/langgraph-vs-crewai-vs-openai-agents-sdk-2026)
- [Workflow Engine Comparison](https://dev.to/frederic_zhou/workflows-windmill-vs-n8n-vs-langflow-vs-temporal-choosing-the-right-tool-for-the-job-23h5)
- [LLM Cost Monitoring Tools 2025](https://dev.to/kuldeep_paul/the-best-tools-for-monitoring-llm-costs-and-usage-in-2025-5f3a)
- [LLM Gateways 2025](https://www.helicone.ai/blog/top-llm-gateways-comparison-2025)