# Unit Economics & Key Metrics for Multi-Agent Orchestration — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, formulas, and techniques for measuring cost, performance, and ROI in multi-agent AI orchestration systems applicable to our ORCHA agent orchestration system and task management processes.

---

## 1. Token Economics and Cost Modeling

### 1.1 Cost-Per-Task Breakdown (Real-World Data)

| Benchmark | Cost/Task | Notes |
|-----------|-----------|-------|
| Claude on SWE-bench | ~$0.30/issue | With prompt caching |
| DeepSeek on SWE-bench | ~$0.003/issue | With prompt caching |
| Unconstrained agent | $5-$8/task | No optimization |
| Production sessions | $10-$100+ | Without guardrails |

SWE-bench evaluations limit models to **2M uncached read/write tokens** and **20M cached token reads**, with a maximum of **150 steps per task**.

**Token cost structure:**
- Output tokens are priced **4x higher** than input tokens on median, with some models reaching **8x**
- In agentic coding tasks, **input tokens dominate** overall consumption and cost, even with caching
- Token usage exhibits **large variance** across runs — some runs use up to **10x more tokens** than others for the same task type

Sources: [OpenHands SWE-bench](https://openhands.dev/blog/evaluation-of-llms-as-coding-agents-on-swe-bench-at-30x-speed), [Stevens Online](https://online.stevens.edu/blog/hidden-economics-ai-agents-token-costs-latency/), [SWE-rebench](https://swe-rebench.com)

### 1.2 Token Amplification in Multi-Agent Systems

This is the critical economic factor for orchestration systems:

- Multi-agent systems require **15x more tokens** compared to standard chat (Anthropic's own data)
- Concrete example: multi-agent used **1,005 input tokens** vs **13 input tokens** for single-agent — **77x amplification** on input
- Daily cost comparison: single-agent **$0.41/day** vs multi-agent **$10.54/day** — **26x more expensive**
- A Reflexion loop running 10 cycles consumes **50x the tokens** of a single linear pass
- Token usage explains **80% of the variance** in multi-agent performance — they work *because* they spend enough tokens

**Mitigation:** Subagent/context-isolation patterns can process **67% fewer tokens** by giving each subagent only relevant context.

Sources: [Capgemini](https://www.capgemini.com/insights/expert-perspectives/ai-lab-the-efficient-use-of-tokens-for-multi-agent-systems/), [Stevens Online](https://online.stevens.edu/blog/hidden-economics-ai-agents-token-costs-latency/), [Snorkel AI](https://snorkel.ai/blog/multi-agents-in-the-context-of-enterprise-tool-use/)

### 1.3 Price-Performance Across Model Tiers (2026 Pricing)

| Tier | Models | Input/MTok | Output/MTok | Relative Cost |
|------|--------|-----------|------------|---------------|
| Premium | Claude Opus 4.5/4.6 | $5 | $25 | 1x |
| Mid-tier | Claude Sonnet 4.5/4.6 | $3 | $15 | 0.6x |
| Lightweight | Claude Haiku 4.5 | $1 | $5 | 0.2x |
| Budget | GPT-3.5, small models | $0.50-$2 | $1-$5 | 0.04-0.1x |
| Local/Open Source | Llama, Mistral | ~$0.0001 | ~$0.0001 | ~0.00002x |

The difference between premium and lightweight models is **60-300x**. Opus 4.5/4.6 at $5/$25 is **66.7% cheaper** than Opus 4/4.1 which was $15/$75.

**Application to ORCHA:** We already have multi-model support but don't tier by role. The RooRoo research (cost-tiered model routing, Backlog #4) becomes even more critical with this cost data — routing 30% of tasks to Haiku-class models saves 80% on those tasks.

### 1.4 Prompt Caching Economics

- Cache read tokens cost **10% of standard input price** (90% savings)
- 5-minute cache TTL write: **1.25x** base input price; 1-hour cache write: **2x** base price
- Cache **pays for itself after just 1 read** for 5-minute TTL, or **2 reads** for 1-hour TTL
- Example: 10,000-token system prompt used repeatedly saves **90% of processing costs** after first request
- Example: 50,000-token document with 20 questions eliminates **980,000 redundant token processings**
- With caching, Sonnet 4.6 drops to **$0.30/MTok** on input — cheaper than GPT-4.1's cached rate

### 1.5 Batch API Economics

- Both OpenAI and Anthropic offer **50% discount** on batch API for non-real-time workloads
- Combined with caching: realistic savings of **60-80% cost reduction** with minimal quality impact
- Best for: bulk task processing, overnight agent runs, non-interactive workflows

**Application to ORCHA:** Agent tasks that don't need real-time results (e.g., batch code review, documentation generation, bulk task decomposition) should use batch API pricing. Combined with prompt caching, this represents the largest single cost optimization opportunity.

---

## 2. Key Performance Metrics

### 2.1 The CAPO Metric (Cost-per-Accepted-Outcome)

The most important unit economics metric for agentic systems:

```
CAPO = Fully loaded cost / Accepted outcomes
```

CAPO includes: model inference costs + tool call costs + retry costs + infrastructure overhead + human review costs. It forces you to account for failures, retries, and quality gates rather than just raw token spend.

**Why CAPO matters:** A task that costs $0.30 in tokens but requires 3 attempts and human review to get right has a CAPO of ~$1.50 — 5x the apparent cost.

Source: [InfoWorld](https://www.infoworld.com/article/4138748/finops-for-agents-loop-limits-tool-call-caps-and-the-new-unit-economics-of-agentic-saas.html)

### 2.2 Google Cloud's Three-Pillar KPI Framework

**Pillar 1: Reliability & Operational Efficiency**
- **Cost per successful task** — pairs cost with outcomes, not just token spend
- **End-to-end trace latency** — total time from initiation to resolution (not time-to-first-token)
- **Tool call error rates**
- **Token consumption per interaction**

**Pillar 2: Adoption & Usage Patterns**
- Reactive (user-invoked) vs proactive (background) agent usage
- User feedback and conversation length

**Pillar 3: Business Value**
- Task completion rates
- Productivity multiplier
- Human escalation rate

Source: [Google Cloud](https://cloud.google.com/transform/the-kpis-that-actually-matter-for-production-ai-agents)

### 2.3 Galileo's Evaluation Framework

Three evaluation levels:
1. **Session-level**: Overall task completion, successful agentic interactions
2. **Trace-level**: Workflow quality, tool selection accuracy
3. **Span-level**: Individual tool call completion quality

Benchmarks:
- **Goal accuracy** target: **85%+** (successful completions / total attempts)
- **Task adherence**: **95%+** for production environments
- Only **15% of teams** achieve elite evaluation coverage (90-100% of behaviors tested)

Source: [Galileo](https://galileo.ai/blog/ai-agent-metrics)

### 2.4 Complete Metrics Table

| Metric | Formula | Target | Level |
|--------|---------|--------|-------|
| **CAPO** | Total loaded cost / Accepted outcomes | Varies by task type | Unit economics |
| **Goal Accuracy** | Successful completions / Total attempts | 85%+ | Session |
| **Task Adherence** | Steps following workflow / Total steps | 95%+ | Trace |
| **First-Pass Success** | Tasks completed without retry / Total tasks | Track over time | Session |
| **Human Escalation Rate** | Tasks requiring human / Total tasks | <10% production | Session |
| **Token Efficiency** | Quality score / Tokens consumed | Maximize | Trace |
| **Cost per Attempt** | Total cost / All attempts (including failures) | Minimize | Unit economics |
| **Retry Rate** | Retried tasks / Total tasks | <20% | Session |
| **Agent Utilization** | Active time / Available time | 60-80% | Operational |
| **E2E Latency** | Task start → final resolution | Task-dependent | Operational |
| **Tool Call Error Rate** | Failed tool calls / Total tool calls | <5% | Span |
| **Context Isolation Score** | Relevant tokens / Total tokens sent | >50% | Efficiency |

Sources: [Pendo](https://www.pendo.io/essential-kpis-measuring-ai-agent-performance/), [Moxo](https://www.moxo.com/blog/agentic-ai-kpis), [InfoWorld](https://www.infoworld.com/article/4138748/finops-for-agents-loop-limits-tool-call-caps-and-the-new-unit-economics-of-agentic-saas.html)

---

## 3. ROI and Value Measurement

### 3.1 Developer Productivity Data

**Positive findings:**
- Developers save an average of **3.6 hours/week** (analytics across 135K+ developers)
- AI tools boost productivity by **10-30%** on average
- Up to **60% time savings** on coding, testing, and documentation
- AI investments returned **$3.70 for every $1 invested** in 2025; top performers: **$10.30**
- **74% of executives** report achieving ROI within the first year

**Contradictory findings (critical to understand):**
- METR randomized controlled trial: experienced developers were actually **19% slower** with AI tools
- Perception gap: developers expected **24% speedup** but even after being slower, still believed AI sped them up by **20%**
- Bain's 2025 report: AI coding tools deliver only **10-15% productivity gains**
- Only **28% of global finance leaders** report clear, measurable value

**Takeaway:** Self-reported productivity gains are unreliable. Measure actual outcomes (tasks completed, code shipped, bugs fixed), not developer sentiment.

Sources: [Index.dev](https://www.index.dev/blog/ai-coding-assistants-roi-productivity), [METR](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/), [VentureBeat](https://venturebeat.com/orchestration/ai-agents-are-delivering-real-roi-heres-what-1-100-developers-and-ctos)

### 3.2 ROI Calculation Formula

```
ROI% = ((Hours_Saved x Hourly_Rate x Failure_Discount) - Monthly_Tool_Cost) / Monthly_Tool_Cost x 100
```

Where:
- `Hours_Saved` = request volume × time saved per request type
- `Failure_Discount` = accounts for post-acceptance correction time (30% discount conservative)
- Conservative scenario: 15% productivity gain, 30% failure discount, $75/hr rate = **~150% ROI**

### 3.3 Cost Comparison: Agent vs Human Developer

| Factor | AI Agent | Human Developer |
|--------|----------|-----------------|
| Cost per task | $0.50-$5 (simple) to $10-100 (complex) | $60-200/hr |
| Availability | 24/7 | ~8hr/day |
| Monthly tool cost | $20-100/seat (light), $500-1500/seat (heavy agentic) | $10K-35K/mo salary |
| Scaling cost | Near-linear with tasks | Linear with headcount |
| Quality variance | High (10x variance in token usage) | Moderate |

### 3.4 The POC-to-Production Cost Trap

Critical warning for orchestration system economics:
- A POC costing **$50** in API usage can scale to **$2.5 million/month** at production volume
- Another case: **$500 POC** became **$847,000/month** in production — a **717x increase**

**Design principle:** "Don't build a POC and scale it up. Build a production system and scale it down for the POC."

**Application to ORCHA:** Our cost modeling must account for production scale from day one. Token budgets, model routing, and caching strategies must be designed for the production volume scenario, not the demo scenario.

Source: [Medium - Token Cost Trap](https://medium.com/@klaushofenbitzer/token-cost-trap-why-your-ai-agents-roi-breaks-at-scale-and-how-to-fix-it-4e4a9f6f5b9a)

---

## 4. Cost Optimization Strategies

### 4.1 Model Routing by Task Complexity

Smart routing can cut LLM spending by **30-85%** while maintaining response quality.

Typical usage pattern breakdown:
- **30%** simple tasks (FAQ, formatting, classification) → Haiku/small models
- **50%** medium tasks (code generation, CRM ops, email) → Sonnet/mid-tier
- **20%** complex tasks (architecture, analytics, reasoning) → Opus/premium

Real-world example: Routing reduced costs from **$105/month** (uniform GPT-4o) to **$41/month** — **60% cheaper** while complex tasks got *better* quality from specialized models.

**Application to ORCHA:** Implement a task complexity classifier that routes to appropriate model tiers. This maps to Backlog #4 (Cost-Tiered Model Routing from RooRoo research). The economics strongly support this: 30% of tasks at Haiku pricing ($1/MTok input) vs Opus pricing ($5/MTok) = 80% savings on those tasks.

Sources: [MindStudio](https://www.mindstudio.ai/blog/what-is-ai-model-router-optimize-cost-llm-providers), [Atlosz](https://atlosz.hu/en/blog/llm-koltsegoptimalizalas-routing-strategia/)

### 4.2 Caching Strategies

| Strategy | Savings | Best For |
|----------|---------|----------|
| **Prompt caching** | 90% on repeated system prompts | Agents with consistent instructions |
| **Semantic caching** (Redis LangCache) | Up to 73% in high-repetition workloads | Similar queries across tasks |
| **Memory layer caching** | Latency: 30s → 300ms, cost: near zero | Repeated planning/orchestration calls |
| **Tool output caching** | Eliminates re-execution of deterministic tools | File reads, search results, API lookups |

**Application to ORCHA:** Agent system prompts, project context, and CLAUDE.md files are prime candidates for prompt caching — they change rarely but are sent with every agent call. Tool outputs from file reads and codebase searches should be cached within a task's execution.

Sources: [Redis](https://redis.io/blog/llm-token-optimization-speed-up-apps/), [Stevens Online](https://online.stevens.edu/blog/hidden-economics-ai-agents-token-costs-latency/)

### 4.3 Context Window Optimization

Poor context management causes **60-70%** of total spend. Four strategies:

1. **Write** — Save context externally (memory/DB), don't carry in prompt
2. **Select** — Pull only relevant snippets, not entire documents
3. **Compress** — Summarize conversation history instead of full replay
4. **Isolate** — Each subagent gets only task-relevant context (67% fewer tokens)

> "Teams routinely pass 4-8 long documents when only a snippet would do."

**Application to ORCHA:** Context isolation (Backlog #5 from RooRoo) has direct cost impact: 67% token reduction means 67% cost reduction on input tokens. The context compilation pipeline (from MAS research) implements this systematically.

Source: [Fast.io](https://fast.io/resources/ai-agent-token-cost-optimization/), [FlowHunt](https://www.flowhunt.io/blog/context-engineering-ai-agents-token-optimization/)

### 4.4 Five-Guardrail Framework (FinOps for Agents)

1. **Loop/step limit** — Cap planning, reflection, and verification cycles
2. **Tool-call cap** — Cap total paid actions per run, with stricter sub-caps for expensive tools
3. **Token budget** — Per-run token ceiling across calls (per-task, not global)
4. **Wall-clock timeout** — Keep interactive flows snappy
5. **Tenant budgets and concurrency** — Per-tenant caps with anomaly alerts

Implementation: Teams combining **50/80/100% budget alerts** with **3x rate-of-change detectors** catch misconfigured loops within hours. Webhook actions can automatically **downgrade model tier** when limits are hit.

**Application to ORCHA:** Every agent invocation should have a declared token budget based on task type. The orchestrator enforces the budget, with automatic model tier downgrade when approaching limits. This maps to Backlog #12 from MAS research (Explicit Termination Controls) but adds the economic dimension.

Source: [InfoWorld](https://www.infoworld.com/article/4138748/finops-for-agents-loop-limits-tool-call-caps-and-the-new-unit-economics-of-agentic-saas.html)

### 4.5 Local vs API Models Decision Framework

**Use local models when:**
- Sub-second latency required for small decisions
- SOC 2 / privacy compliance prevents external data transmission
- High-volume simple tasks where per-token API cost adds up
- Fine-tuned domain-specific models outperform general APIs

**Use API models when:**
- Tasks require frontier reasoning capability
- Volume is insufficient to justify infrastructure cost
- You need the latest model capabilities

**Application to ORCHA:** The Docker Compose three-file LLM provider strategy (Backlog #25) enables switching between local and cloud inference per environment. For the orchestrator's own reasoning (routing, triage, status checks), a local model eliminates API latency and cost entirely.

---

## 5. Observability and Measurement Infrastructure

### 5.1 Platform Comparison

| Platform | Free Tier | Best For | Key Feature |
|----------|-----------|----------|-------------|
| **Langfuse** | 50K events/mo | Self-hosted, prompt engineering | Open-source, self-hostable |
| **Helicone** | 100K requests/mo | Fast setup, cost tracking | Proxy-based 1-line integration, $25/mo flat |
| **Portkey** | Varies | Multi-provider routing | 250+ AI models, edge gateway |
| **Braintrust** | Varies | Evaluation/evals | LLM-as-judge, A/B testing |
| **Phoenix (Arize)** | Unlimited (self-hosted) | Self-hosted observability | Full control |
| **LangSmith** | 5K traces/mo | LangChain users | Deep LangChain integration |
| **Galileo** | Free tier | Agent reliability | 93-97% accurate agent metrics |

### 5.2 Trace-Level Cost Attribution

- **Helicone**: Tag requests with metadata (user, route, experiment) for granular cost attribution; agent tracing captures spans across tools and function calls
- **Portkey**: Log every LLM request with **40+ attributes**, inspect full trace lifecycles
- **Datadog/New Relic**: 2025 agentic monitoring features provide service maps across interconnected agents

### 5.3 LLM-as-Judge for Quality Evaluation

- **53.3%** of teams with deployed AI agents use LLM-as-judge (LangChain 2025 survey)
- Economics: **80% agreement** with human preferences at **500x-5,000x lower cost**
- Caveat: Position bias intensifies as candidate count increases
- Braintrust AutoEvals: scores both intermediate agent steps and final outputs

### 5.4 A/B Testing for Agent Configurations

- Prompt design variations produce performance differences of **up to 40%**
- Langfuse enables labeling prompt versions and tracking per-version metrics
- Best practice: Test against a "Golden Dataset" before production deployment

**Application to ORCHA:** Self-hosted Langfuse or Phoenix provides observability without data leaving our infrastructure. Trace-level cost attribution lets us calculate CAPO per task type, per agent, and per model tier. LLM-as-judge at 500x lower cost than human review enables automated quality gates in the orchestration pipeline.

Sources: [Softcery](https://softcery.com/lab/top-8-observability-platforms-for-ai-agents-in-2025), [Helicone](https://www.helicone.ai/blog/monitor-and-optimize-llm-costs), [Braintrust](https://www.braintrust.dev/articles/what-is-llm-as-a-judge)

---

## 6. Benchmarks and Industry Data

### 6.1 SWE-bench Cost Data

| Agent/Model | SWE-bench Score | Cost/Issue | Notes |
|-------------|----------------|------------|-------|
| Claude Opus 4.6 | #1 on leaderboard | ~$0.30 | With prompt caching |
| Claude Code (pass@5) | Highest pass@5 | Higher (5 attempts) | Best multi-attempt |
| DeepSeek | Competitive | $0.003 | With heavy caching |
| OpenHands + Sonnet 4.5 | 72% resolution | ~$0.30 | With extended thinking |

### 6.2 Framework Token Efficiency

Across 5 tasks and 2,000 runs:
- **LangChain**: Most token-efficient overall
- **AutoGen**: Best latency; 10,750 prompt tokens on complex tasks
- **LangGraph**: 15,010 prompt tokens on complex tasks (highest) due to state accumulation; best production reliability
- **CrewAI**: ~56% more tokens per request; quality self-checking doubles token usage

For simple tasks, LangChain and LangGraph both finish under 5 seconds with fewer than 900 prompt tokens.

### 6.3 Industry Spending Data

- Enterprise AI spending: **$37 billion in 2025** (3.2x increase from 2024)
- Average monthly AI spending: **$85,521** in 2025 (36% jump from 2024)
- Token costs dropped **280-fold** over two years, yet some enterprises see monthly bills in tens of millions
- Only **5% of enterprises** see real returns from AI in 2026

Source: [Deloitte](https://www.deloitte.com/us/en/insights/topics/emerging-technologies/ai-tokens-how-to-navigate-spend-dynamics.html)

---

## 7. Pricing Models and Billing Architecture

### 7.1 Eight Pricing Models for AI Agents

| Model | Example | Best For |
|-------|---------|----------|
| **Per-token/usage** | Direct API pass-through | Internal tools, developer platforms |
| **Per-task/action** | Intercom Fin at $0.99/resolution | Customer-facing agents |
| **Per-seat/subscription** | GitHub Copilot $10-19/user | Predictable budgeting |
| **Outcome-based** | Pay only for successful outcomes | High-value workflows |
| **Hybrid** | Base subscription + usage overage | Balanced risk |
| **Credit/token pool** | Pre-purchased credits | Enterprise contracts |
| **Tiered** | Volume-based pricing tiers | Scaling organizations |
| **Freemium** | Free tier + paid upgrades | Market acquisition |

### 7.2 Internal Chargeback Models

- **Token as unit of cost**: The single unit that can be tracked and attributed to individual AI use cases
- **Showback/chargeback**: Finance ties AI spend to business units based on utilization with timestamps
- **AI FinOps discipline**: Tokens, prompt design, inference patterns, and safety controls become first-class cost levers
- **Policy-as-code**: Deny prompts that reference disallowed data, exceed token ceilings, or bypass safety filters

**Industry trend:** Most 2025-2026 deployments combine **base fee + usage/output/performance layers**. As agents collaborate in multi-agent systems, pricing is moving from task-level units to broader **"work-cell" outcomes**.

**Application to ORCHA:** Track cost per task, per agent, per model tier. Enable internal chargeback by organization/project. This data feeds into the CAPO calculation and enables ROI analysis per workflow type.

Sources: [EMA.ai](https://www.ema.ai/additional-blogs/addition-blogs/ai-agents-pricing-strategies-models-guide), [Chargebee](https://www.chargebee.com/blog/pricing-ai-agents-playbook/), [FinOps.org](https://www.finops.org/wg/how-to-build-a-generative-ai-cost-and-usage-tracker/)

---

## 8. Failure Economics

### 8.1 Compound Failure (Lusser's Law)

**System success = Product of individual agent success rates**

| Agents | Per-Agent Success | System Success |
|--------|-------------------|----------------|
| 3 | 98% | 94.1% |
| 5 | 98% | 90.4% |
| 10 | 95% | 59.9% |
| 10 | 98% | 81.7% |
| 15 | 95% | 46.3% |

Every agent handoff introduces failure probability, and chaining them compounds it **multiplicatively**.

> "Without explicit fault tolerance, agentic systems are not just fragile — they are economically unsustainable."

**Application to ORCHA:** A 5-agent workflow needs each agent at 98%+ reliability to maintain 90% system reliability. This is why the validation gates between agents (from FSM research, Backlog #20) and the 5-level error handling hierarchy (MAS research, Backlog #22) are economic necessities, not just nice-to-haves.

Source: [O'Reilly Radar](https://www.oreilly.com/radar/the-hidden-cost-of-agentic-failure/)

### 8.2 Retry Economics

- Every retry is a **fresh request and fresh charge** — 5 retries = 6x expected cost
- Most dangerous pattern: **infinite loops** where an agent hits an error, retries with full growing context, hits same error
- A single stuck task can burn through an **entire daily budget** without circuit breakers
- Token budgets should be **per-task, not global** (research task: 100K tokens; formatting task: 5K tokens)

### 8.3 Early Termination Strategies

1. **Hard token ceiling per run** — When exhausted, stop and return partial result
2. **Circuit breakers** — Detect repeated identical errors and break the loop
3. **Structured error responses** — Return actionable error messages to reduce blind retries
4. **Diminishing returns detection** — Monitor quality improvement per additional token spent
5. **Per-task budget declaration** — Agent declares intended spend before starting; system enforces cap

### 8.4 Validation as Reliability Amplification

Adding validation gates between agent handoffs **fundamentally changes failure propagation** from naive multiplicative decay to controlled reliability amplification. Each gate catches failures before they cascade, making the system *more* reliable than any individual agent.

**Application to ORCHA:** The layered validation pattern (FSM research, deterministic checks before LLM review) is both a quality mechanism AND a cost optimization — catching failures early prevents wasted tokens on downstream agents processing bad inputs.

Source: [O'Reilly Radar](https://www.oreilly.com/radar/the-hidden-cost-of-agentic-failure/)

### 8.5 The 90-Day Implementation Roadmap (InfoWorld)

- **Days 0-30**: Choose 3-5 high-volume workflows, define explicit acceptance gates
- **Days 31-60**: Add routing and validation cascades, cache retrieval and tool outputs
- **Days 61-90**: Align pricing with entitlements, set anomaly alerts

---

## 9. Key Formulas Summary

| Formula | Purpose |
|---------|---------|
| `CAPO = Total loaded cost / Accepted outcomes` | Unit economics for agentic workflows |
| `System reliability = r₁ × r₂ × ... × rₙ` (Lusser's Law) | Compound failure prediction |
| `ROI% = ((Hours_Saved × Rate × Failure_Discount) - Tool_Cost) / Tool_Cost × 100` | Agent ROI calculation |
| `Token amplification = Multi-agent tokens / Single-agent tokens` | Overhead measurement (typically 15-77x) |
| `Cache ROI = (Base_price - Cache_read_price) × Cache_hits - Cache_write_premium` | Caching break-even |
| `Routing savings = Uniform_model_cost - Σ(tier_cost × tier_volume%)` | Model routing ROI |
| `Goal accuracy = Successful completions / Total attempts` | Agent effectiveness |
| `Cost per attempt = Total cost / All attempts (incl. failures)` | True cost visibility |

---

## 10. Comparison: Unit Economics Best Practices vs ORCHA Patterns

| Dimension | Industry Best Practices | ORCHA (Current) | Gap/Opportunity |
|-----------|------------------------|-----------------|-----------------|
| **Cost tracking** | Per-task CAPO with trace-level attribution | No cost tracking | Add token/cost tracking per task, agent, model |
| **Model routing** | 30-85% savings via complexity-based tier routing | Uniform model per agent type | Implement task complexity classifier → model tier |
| **Token budgets** | Per-task budgets with 50/80/100% alerts + auto-downgrade | No token budgets | Add per-task token ceilings with circuit breakers |
| **Prompt caching** | 90% savings on repeated system prompts | No caching strategy | Implement prompt caching for agent instructions + project context |
| **Context isolation** | 67% token reduction via subagent context scoping | Full context passed | Scope context per subagent (Backlog #5) |
| **Retry economics** | Per-task budgets, circuit breakers, early termination | Retry without cost awareness | Add retry budget + diminishing returns detection |
| **Quality metrics** | 85%+ goal accuracy, 95%+ task adherence targets | No quality metrics | Track first-pass success rate, human escalation rate |
| **Failure modeling** | Lusser's Law compound failure analysis | No reliability modeling | Model system reliability across agent chains |
| **Observability** | Langfuse/Helicone with LLM-as-judge quality eval | Basic execution logs | Add self-hosted observability with quality evaluation |
| **ROI measurement** | CAPO + hours saved + failure discount | No ROI measurement | Track cost per task type, compare to human baseline |
| **Batch processing** | 50% discount on non-real-time workloads | All tasks real-time | Identify batch-eligible workflows (review, docs, decomposition) |
| **Cost guardrails** | 5-guardrail framework (loops, tools, tokens, time, tenant) | No guardrails | Implement all five guardrails |

---

## 11. Actionable Recommendations for ORCHA

### High Priority

1. **Per-Task Cost Tracking and CAPO Calculation** — Instrument every agent invocation to track: input tokens, output tokens, cached tokens, tool calls, retries, model used, and wall-clock time. Calculate CAPO (Cost per Accepted Outcome) per task type: `CAPO = (inference_cost + tool_cost + retry_cost + infra_overhead) / accepted_outcomes`. This is the foundational metric — without it, all cost optimization is blind. Store in the task record alongside completion status.

2. **Five-Guardrail Cost Control System** — Implement all five guardrails from the InfoWorld FinOps framework: (1) loop/step limit per task, (2) tool-call cap with sub-caps for expensive tools, (3) per-task token budget (not global — research tasks get 100K, formatting gets 5K), (4) wall-clock timeout, (5) per-tenant/project budgets with anomaly alerts. Webhook actions auto-downgrade model tier when 80% of budget consumed. Circuit breakers stop infinite retry loops.

3. **Prompt Caching Strategy** — Implement Anthropic prompt caching for: agent system prompts (repeated every invocation), project context (CLAUDE.md, codebase summaries), and tool schemas. 90% savings on cache reads pays for itself after 1 re-use. For the orchestrator, which sends similar context with every agent dispatch, this alone could reduce input token costs by 50-70%.

4. **Task Complexity Classifier for Model Routing** — Build a lightweight classifier (can be Haiku-class) that evaluates incoming tasks and routes to appropriate model tiers: simple tasks (status updates, formatting) → Haiku ($1/MTok), medium tasks (code generation, reviews) → Sonnet ($3/MTok), complex tasks (architecture, multi-file refactors) → Opus ($5/MTok). Expected savings: 30-60% of LLM costs with equal or better quality on complex tasks.

### Medium Priority

5. **Compound Failure Modeling (Lusser's Law Dashboard)** — Track per-agent success rates and calculate system-level reliability using Lusser's Law (`system_success = Π(agent_success_rates)`). Display on a dashboard: if any agent's success rate drops below 95%, flag the compound impact. A 5-agent workflow at 95% per-agent = 77% system success. This makes the economic case for validation gates visible — adding a gate that catches 50% of failures between two 95% agents raises the pair's effective reliability from 90% to 95%.

6. **Agent Performance Dashboard** — Track and display: goal accuracy (target 85%+), first-pass success rate, human escalation rate (target <10%), token efficiency (quality/tokens), retry rate, CAPO per task type, cost trend over time. Use self-hosted Langfuse or the LGTM stack (from Docker research, Backlog P2) for the infrastructure. The dashboard should answer: "Which task types are cost-effective for agents vs humans?"

7. **Batch API for Non-Real-Time Workflows** — Identify workflows that don't need real-time results: bulk code review, documentation generation, batch task decomposition, overnight regression analysis. Route these through Anthropic's batch API for 50% discount. Combined with prompt caching (90% savings on cached reads), batch workflows could cost 5-10% of real-time equivalent.

8. **LLM-as-Judge for Automated Quality Gates** — Replace human review with LLM-as-judge for intermediate workflow steps. 53% of deployed agent teams already use this. Economics: 80% agreement with human preferences at 500x-5,000x lower cost. Use a Haiku-class model as judge — the cost is negligible and it catches structural quality issues before expensive downstream agents process bad inputs.

### Lower Priority

9. **Context Isolation Scoring** — Measure and optimize the ratio of relevant tokens to total tokens sent to each agent. Target: >50% context relevance. Current pattern of sending full conversation history to every agent wastes 60-70% of input token spend. This quantifies the ROI of context isolation (Backlog #5) and the context compilation pipeline (MAS research).

10. **ROI Calculator per Workflow Type** — For each workflow type (bug fix, feature implementation, code review, documentation), calculate: `ROI% = ((estimated_human_hours × hourly_rate × failure_discount) - agent_cost) / agent_cost × 100`. Track over time. Identify which workflow types have positive ROI and which don't. Shut down or restructure negative-ROI workflows.

11. **Anomaly Detection for Cost Spirals** — Implement 3x rate-of-change detectors on token spend per task. If a task consumes 3x the expected tokens for its type, alert and potentially terminate. The most dangerous pattern is infinite retry loops with growing context — a single stuck task can burn an entire daily budget. This is the economic enforcement of the circuit breaker pattern.

12. **Internal Chargeback by Organization/Project** — Track and attribute AI costs to specific organizations and projects. Enable showback reports: "Project X spent $Y on agent tasks this month, completing Z tasks with W% success rate." This enables data-driven decisions about which projects benefit from agent automation and which don't.

---

## Key Takeaways

1. **CAPO is the metric that matters** — Cost per Accepted Outcome includes failures, retries, and quality gates. A $0.30 task that needs 3 attempts has a true CAPO of $1.50. Tracking raw token spend without outcomes is misleading.

2. **Token amplification is the hidden cost of multi-agent** — 15-77x more tokens than single-agent. Context isolation (67% reduction) and prompt caching (90% savings on reads) are the two highest-impact mitigations.

3. **Lusser's Law makes validation gates economically necessary** — 5 agents at 95% each = 77% system success. Validation gates between agents aren't just quality measures — they're economic necessities that prevent compound failure from making the system unsustainable.

4. **Model routing saves 30-85%** — 30% of tasks are simple enough for Haiku. 50% work fine with Sonnet. Only 20% need Opus. Routing to the right tier saves 60%+ with equal or better quality on complex tasks.

5. **The POC-to-production trap is real** — A $50 demo can become $2.5M/month at scale. Design cost controls for production volume from day one, then scale down for the POC.

6. **Self-reported productivity gains are unreliable** — The METR study found experienced devs were 19% slower with AI but believed they were 20% faster. Measure actual outcomes, not sentiment.

7. **Five guardrails prevent cost spirals** — Loop limits, tool-call caps, per-task token budgets, wall-clock timeouts, and tenant budgets. Teams using 50/80/100% budget alerts with rate-of-change detectors catch problems within hours.

8. **Prompt caching pays for itself after 1 read** — At 10% of standard price, cached reads break even immediately for 5-minute TTL. Agent system prompts and project context are sent with every invocation — caching these is the lowest-effort, highest-impact optimization.

9. **LLM-as-judge enables affordable quality gates** — 80% agreement with human preferences at 500x-5,000x lower cost. 53% of deployed agent teams already use this. A Haiku-class judge is essentially free compared to the cost of downstream agents processing bad inputs.

10. **Only 5% of enterprises see real returns** — The gap between AI investment and measurable ROI is massive. Per-task cost tracking with CAPO closes this gap by making the economics transparent and actionable.

---

## Sources

- [OpenHands SWE-bench Evaluation](https://openhands.dev/blog/evaluation-of-llms-as-coding-agents-on-swe-bench-at-30x-speed)
- [SWE-rebench Leaderboard](https://swe-rebench.com)
- [Stevens Online — Hidden Economics of AI Agents](https://online.stevens.edu/blog/hidden-economics-ai-agents-token-costs-latency/)
- [Capgemini — Efficient Token Usage for Multi-Agent Systems](https://www.capgemini.com/insights/expert-perspectives/ai-lab-the-efficient-use-of-tokens-for-multi-agent-systems/)
- [Snorkel AI — Multi-Agents in Enterprise Tool Use](https://snorkel.ai/blog/multi-agents-in-the-context-of-enterprise-tool-use/)
- [Silicon Data — LLM Cost Per Token Guide](https://www.silicondata.com/blog/llm-cost-per-token)
- [OpenReview — How Do Coding Agents Spend Your Money](https://openreview.net/forum?id=1bUeVB3fov)
- [Anthropic Claude API Pricing](https://platform.claude.com/docs/en/about-claude/pricing)
- [MetaCTO — Anthropic Pricing Breakdown](https://www.metacto.com/blogs/anthropic-api-pricing-a-full-breakdown-of-costs-and-integration)
- [Prompt Builder — Prompt Caching Token Economics](https://promptbuilder.cc/blog/prompt-caching-token-economics-2025)
- [Finout — OpenAI vs Anthropic Pricing](https://www.finout.io/blog/openai-vs-anthropic-api-pricing-comparison)
- [O'Reilly Radar — The Hidden Cost of Agentic Failure](https://www.oreilly.com/radar/the-hidden-cost-of-agentic-failure/)
- [Google Cloud — KPIs for Production AI Agents](https://cloud.google.com/transform/the-kpis-that-actually-matter-for-production-ai-agents)
- [Galileo — AI Agent Metrics](https://galileo.ai/blog/ai-agent-metrics)
- [InfoWorld — FinOps for Agents](https://www.infoworld.com/article/4138748/finops-for-agents-loop-limits-tool-call-caps-and-the-new-unit-economics-of-agentic-saas.html)
- [Pendo — 10 Essential KPIs for AI Agents](https://www.pendo.io/essential-kpis-measuring-ai-agent-performance/)
- [Index.dev — AI Coding Assistants ROI](https://www.index.dev/blog/ai-coding-assistants-roi-productivity)
- [METR — AI Developer Productivity Study](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/)
- [VentureBeat — AI Agents ROI Survey](https://venturebeat.com/orchestration/ai-agents-are-delivering-real-roi-heres-what-1-100-developers-and-ctos)
- [SitePoint — AI Coding Tools ROI Calculator](https://www.sitepoint.com/ai-coding-tools-cost-analysis-roi-calculator-2026/)
- [Medium — Token Cost Trap](https://medium.com/@klaushofenbitzer/token-cost-trap-why-your-ai-agents-roi-breaks-at-scale-and-how-to-fix-it-4e4a9f6f5b9a)
- [Deloitte — AI Tokens Spend Dynamics](https://www.deloitte.com/us/en/insights/topics/emerging-technologies/ai-tokens-how-to-navigate-spend-dynamics.html)
- [MindStudio — AI Model Router](https://www.mindstudio.ai/blog/what-is-ai-model-router-optimize-cost-llm-providers)
- [Atlosz — LLM Cost Optimization Routing](https://atlosz.hu/en/blog/llm-koltsegoptimalizalas-routing-strategia/)
- [Redis — LLM Token Optimization](https://redis.io/blog/llm-token-optimization-speed-up-apps/)
- [Fast.io — AI Agent Token Cost Optimization](https://fast.io/resources/ai-agent-token-cost-optimization/)
- [FlowHunt — Context Engineering for AI Agents](https://www.flowhunt.io/blog/context-engineering-ai-agents-token-optimization/)
- [DEV Community — Stop AI Agent Cost Spirals](https://dev.to/nebulagg/how-to-stop-ai-agent-cost-spirals-before-they-start-4ce7)
- [Softcery — 8 AI Observability Platforms](https://softcery.com/lab/top-8-observability-platforms-for-ai-agents-in-2025)
- [Helicone — LLM Observability Guide](https://www.helicone.ai/blog/monitor-and-optimize-llm-costs)
- [Braintrust — LLM-as-Judge](https://www.braintrust.dev/articles/what-is-llm-as-a-judge)
- [Langfuse — A/B Testing](https://langfuse.com/docs/prompt-management/features/a-b-testing)
- [EMA.ai — AI Agent Pricing Models](https://www.ema.ai/additional-blogs/addition-blogs/ai-agents-pricing-strategies-models-guide)
- [Chargebee — Pricing AI Agents Playbook](https://www.chargebee.com/blog/pricing-ai-agents-playbook/)
- [FinOps.org — Build a Gen AI Cost Tracker](https://www.finops.org/wg/how-to-build-a-generative-ai-cost-and-usage-tracker/)
- [Cosine — AI Coding Agent Pricing](https://cosine.sh/blog/ai-coding-agent-pricing-task-vs-token)
- [Latenode — LangGraph vs AutoGen vs CrewAI](https://latenode.com/blog/platform-comparisons-alternatives/automation-platform-comparisons/langgraph-vs-autogen-vs-crewai-complete-ai-agent-framework-comparison-architecture-analysis-2025)
