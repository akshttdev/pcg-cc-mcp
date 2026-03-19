# Multi-Agent System (MAS) Design Research

**Date**: 2026-03-18
**Type**: Reference
**Purpose**: Deep research on multi-agent architecture patterns, coordination mechanisms, and fault tolerance strategies applicable to AI agent orchestration and task management.

---

## Table of Contents

1. [Multi-Agent Architecture Patterns](#1-multi-agent-architecture-patterns)
2. [Modern AI Multi-Agent Frameworks](#2-modern-ai-multi-agent-frameworks)
3. [Agent Communication Patterns](#3-agent-communication-patterns)
4. [Coordination and Consensus](#4-coordination-and-consensus)
5. [Supervision and Fault Tolerance](#5-supervision-and-fault-tolerance)
6. [Memory and Context Management](#6-memory-and-context-management)
7. [Academic/Industry Surveys](#7-academicindustry-surveys)
8. [Extractable Principles for Agent Orchestration](#8-extractable-principles-for-agent-orchestration)

---

## 1. Multi-Agent Architecture Patterns

### 1.1 Reactive vs Deliberative vs Hybrid Architectures

**Reactive architectures** map perceptions directly to actions through condition-action rules or learned policies. Strengths: low latency, robustness to minor distributional shifts. Weaknesses: brittleness when tasks require look-ahead, long-horizon planning, or reasoning about hidden state.

**Deliberative architectures** maintain explicit world models and use search/planning for action selection (goal decomposition, symbolic planners). Strengths: explainability, goal consistency. Weaknesses: latency, model mismatch.

**Hybrid architectures** combine both: reactive layers handle tight control loops while deliberative layers provide goals, constraints, and re-planning. This is "the most practical template for agentic AI" because it separates fast safety actions from slow reasoning in supervisory layers.

> **Applicability**: An orchestrator managing coding agents should use a hybrid approach -- fast reactive responses for health checks, timeouts, and status updates; deliberative planning for task decomposition, agent assignment, and conflict resolution.

Source: [Chapter 3: Architectures for Building Agentic AI](https://arxiv.org/html/2512.09458v1)

### 1.2 Belief-Desire-Intention (BDI) Model

The BDI framework structures agency around three mental constructs:

- **Beliefs** -- world state and memory (scratchpads, episodic logs, semantic stores)
- **Desires** -- admissible goals from users, alerts, or policies
- **Intentions** -- adopted plans/tool calls the system commits to executing

Two key contributions: (1) **commitment strategies** create predictability by preventing constant replanning, and (2) **intention revision** allows agents to adapt when conditions change. The framework helps "articulate the boundary with generative components" by treating LLM outputs as proposals requiring adoption through explicit rules.

> **Applicability**: Map to an orchestrator: Beliefs = current project state, agent availability, task status. Desires = user goals, sprint objectives. Intentions = assigned tasks, active agent sessions. The commitment strategy prevents thrashing between plans.

Sources: [BDI Architecture - ScienceDirect](https://www.sciencedirect.com/topics/computer-science/belief-desire-intention-architecture), [Agent Architecture Blog](https://medium.com/@rahulkrish28/smart-by-design-demystifying-the-architecture-of-ai-agents-blog-4-6b0acdbe0469)

### 1.3 Blackboard Architecture

A shared knowledge base where agents post and retrieve information asynchronously. Agents collaborate without direct communication -- multiple specialists contribute partial solutions into a shared workspace; other agents critique, refine, or build on contributions.

**How it works**: A central "blackboard" data structure holds the evolving problem state. Agents (knowledge sources) monitor the blackboard for patterns they can contribute to. A controller determines which agent acts next based on current state.

**MCP Integration**: Model Context Protocol adds standardized primitives for blackboard access -- structured JSON-RPC exchanges, explicit context weighting, tool invocation, and memory management across multi-level context hierarchies.

> **Applicability**: Ideal for coding tasks where multiple agents work on different files/modules. The blackboard holds the project state (file changes, test results, build status), and agents read/write as they complete sub-tasks.

Sources: [Blackboard Pattern with MCPs](https://medium.com/@dp2580/building-intelligent-multi-agent-systems-with-mcps-and-the-blackboard-pattern-to-build-systems-a454705d5672), [MCP Multi-Agent Architecture](https://arxiv.org/html/2504.21030v1)

### 1.4 Broker/Mediator Pattern

Communication via message brokers (Kafka, Redis Streams). Agents communicate indirectly through a mediator that routes messages based on topic/capability matching. Decouples producers from consumers.

### 1.5 Contract Net Protocol (CNP)

A manager agent announces tasks; potential contractor agents evaluate and submit bids; the manager awards the contract to the most suitable bidder. Phases: announcement, bidding, awarding. Enables dynamic task assignment based on agent capabilities, workload, and availability.

> **Applicability**: For assigning coding tasks to agents -- the orchestrator announces "implement feature X", agents bid based on their current load and capability match, the best-fit agent gets the assignment.

### 1.6 Publish-Subscribe Agent Coordination

Agents subscribe to event topics. When events occur (task completed, build failed, test passed), subscribers react. Enables loose coupling and event-driven coordination without direct agent-to-agent dependencies.

### 1.7 Hierarchical Task Decomposition

Complex problems decompose through:
1. **Initial analysis** identifying subtasks and dependencies
2. **Agent capability matching** assigning tasks to specialists
3. **Dependency resolution** sequencing based on information flow
4. **Progress coordination** monitoring intermediate results

**Hierarchical Task DAG (HTDAG)** -- Deep Agent's approach: recursively represents sub-tasks and dependencies across multiple layers of a Directed Acyclic Graph. The planner creates next-level sub-task DAGs only when necessary (lazy decomposition), preventing premature over-planning.

**Key principle**: Tasks can be either **atomic** (irreducible, like a single file edit) or **non-atomic** (decomposable into further sub-tasks), triggering additional planner-executor cycles.

**Error containment**: Hierarchical design "localizes failures within specific levels, preventing cascade effects throughout the task graph." Failed components replan while successful ones persist.

> **Applicability**: Core pattern for coding agent orchestration. A feature request decomposes into: analyze requirements -> design approach -> implement per-file changes -> write tests -> run CI. Each level can decompose further.

Sources: [Deep Agent HTDAG](https://arxiv.org/html/2502.07056v1), [Task Decomposition for Coding Agents](https://mgx.dev/insights/task-decomposition-for-coding-agents-architectures-advancements-and-future-directions/a95f933f2c6541fc9e1fb352b429da15)

---

## 2. Modern AI Multi-Agent Frameworks

### 2.1 Anthropic's Patterns (Building Effective Agents)

Six composable patterns from simple to complex:

**Pattern 1: Augmented LLM** (Building Block)
LLM enhanced with retrieval, tools, and memory. The model actively generates search queries, selects tools, and determines what to retain.

**Pattern 2: Prompt Chaining** (Workflow)
Sequential steps where each LLM call processes previous outputs. Programmatic gates verify intermediate results. Use when tasks decompose cleanly into fixed subtasks.

**Pattern 3: Routing** (Workflow)
Classifies inputs and directs them to specialized downstream handlers with custom prompts/tools. Decision point determines a single downstream path per request.

**Pattern 4: Parallelization** (Workflow)
Two variations: **sectioning** (independent subtasks in parallel) and **voting** (same task multiple times for consensus). "LLMs generally perform better when each consideration is handled by a separate LLM call."

**Pattern 5: Orchestrator-Workers** (Workflow)
Central LLM dynamically decomposes tasks, delegates to worker LLMs, synthesizes results. Suited for tasks where subtasks cannot be predicted in advance (e.g., coding -- the number of files that need changing depends on the task).

**Pattern 6: Evaluator-Optimizer** (Workflow)
One LLM generates responses; another evaluates and provides feedback in iterative loops. Requires clear evaluation criteria and demonstrable improvement with feedback.

**Pattern 7: Autonomous Agents** (System)
LLMs use tools based on environmental feedback in loops. "Agents gain ground truth from the environment at each step (tool results, code execution) to assess progress." Require stopping conditions (max iterations) to maintain control.

**Core principle**: "Start simple; add complexity only when demonstrably improving outcomes." Invest more in tool design (Agent-Computer Interface) than overall prompts.

Source: [Anthropic - Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)

### 2.2 Anthropic's Multi-Agent Research System

**Architecture**: Orchestrator-worker pattern with Lead Researcher coordinating subagents in parallel.

**Lead Researcher**: Analyzes queries, creates investigation plans, manages memory for context persistence, decides resource allocation, synthesizes results.

**Subagents**: Operate in parallel with independent contexts. Each receives: concrete objective, task boundaries, output format, tool guidance.

**Citation Agent**: Post-research validation of every claim against sources. Ensures traceability.

**Effort Scaling Rules embedded in prompts**:
- Simple fact-checks: 1 agent, 3-10 tool calls
- Direct comparisons: 2-4 subagents, 10-15 calls each
- Complex research: 10+ subagents with divided responsibilities

**Performance**: 90.2% improvement over single-agent (Opus 4 lead + Sonnet 4 subagents). Tokens: ~15x chat baseline.

**Key lessons**:
- Token usage explains ~80% of performance variance
- Agents are stateful and errors compound -- combine AI adaptability with deterministic safeguards (retry logic, checkpoints)
- Non-determinism makes traditional debugging ineffective -- need production tracing
- Related failures risk: agents using similar algorithms/data may fail simultaneously
- Rainbow deployments for stateful agent systems
- Self-improvement: agents can act as their own prompt engineers, reducing completion times by ~40%

Sources: [Anthropic - Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system), [ByteByteGo Analysis](https://blog.bytebytego.com/p/how-anthropic-built-a-multi-agent), [Reflections Analysis](https://llmmultiagents.com/en/blogs/anthropic-multi-agent-system-reflection)

### 2.3 OpenAI Swarm / Agents SDK

**Core abstractions**: Agents, Handoffs, and Routines.

**Agent**: Has a system prompt (instructions) and a set of tools. Lightweight and stateless.

**Routine**: Natural language instructions + tools needed to complete them. Structured workflows with adaptive flexibility.

**Handoff**: Agent transfers active conversation to another agent. "Since the system has no persistent state between calls, every handoff must include all context the next agent needs -- no hidden variables, no magical memory."

**Design philosophy**: Trades opaque automation for clarity/observability. You control exactly when control moves, what context travels with it, and how each specialist handles its job.

**Evolution**: Swarm was educational; replaced by the production-ready OpenAI Agents SDK.

> **Applicability**: The handoff pattern is powerful for coding workflows -- a planning agent hands off to an implementation agent, which hands off to a review agent. Explicit context passing prevents information loss.

Sources: [OpenAI Swarm GitHub](https://github.com/openai/swarm), [OpenAI Cookbook - Orchestrating Agents](https://cookbook.openai.com/examples/orchestrating_agents)

### 2.4 LangGraph

**Architecture**: Graph-based -- nodes are functions (agents, tools, logic), edges define control flow. Directed graphs with explicit state passing.

**State Management**: Shared state explicitly passed through workflow. Native checkpointing for persistence and resumable conversations. Uses reducer logic to merge concurrent updates.

**Coordination**: Centralized control via conditional routing (`add_conditional_edges()`). Deterministic workflow paths.

**Strengths**: Production-ready (v1.0 late 2025), excellent visual debugging (Mermaid diagrams), audit-friendly, token-efficient (~2,000 tokens).

**Weaknesses**: Steep learning curve.

Source: [LangGraph vs CrewAI vs AutoGen Comparison](https://dev.to/pockit_tools/langgraph-vs-crewai-vs-autogen-the-complete-multi-agent-ai-orchestration-guide-for-2026-2d63)

### 2.5 CrewAI

**Architecture**: Role-based team model. Agents have roles, goals, and backstories. Tasks are assignments with expected outputs. Crews organize agents.

**Coordination**: Sequential and hierarchical processes. Hierarchical mode uses a manager agent that auto-coordinates the team.

**Strengths**: Beginner-friendly, natural team metaphor, growing tools ecosystem.

**Weaknesses**: Research-oriented maturity, limited explicit error handling, token-heavy (~3,500 tokens).

### 2.6 AutoGen / Microsoft Agent Framework

**Architecture**: Conversational model -- agents send messages until termination conditions met. Group chats or pairwise conversations.

**Coordination**: Message passing with speaker selection (round-robin, LLM-based auto, or custom).

**State Management**: Manual, no native persistence. Conversation history as implicit state.

**Evolution**: Merged with Semantic Kernel into unified Microsoft Agent Framework (GA Q1 2026). Multi-language (C#, Python, Java), Azure integration, production SLAs.

**Weaknesses**: Token-expensive (~8,000 tokens), challenging debugging, research-oriented.

### 2.7 Google ADK (Agent Development Kit)

**Architecture**: Code-first framework with three agent types:
- **LLM Agents**: Use language models for reasoning and dynamic decision-making
- **Workflow Agents**: SequentialAgent, ParallelAgent, LoopAgent -- deterministic flow control without LLM
- **Custom Agents**: Purpose-built for specific needs

**8 Design Patterns**:

1. **Sequential Pipeline**: Linear assembly line. State via `output_key` writes to shared `session.state`.
2. **Coordinator/Dispatcher**: Central LLM-driven delegation using AutoFlow. Routes based on sub-agent descriptions.
3. **Parallel Fan-Out/Gather**: `ParallelAgent` runs sub-agents in separate threads. Unique output keys prevent race conditions.
4. **Hierarchical Decomposition**: Sub-agents wrapped as `AgentTool`. Parent treats entire sub-agent workflow as single function call.
5. **Generator and Critic**: `LoopAgent` enforces conditional looping. Critic outputs PASS/FAIL. Loop exits on PASS.
6. **Iterative Refinement**: Generator -> Critic -> Refiner in LoopAgent. `max_iterations` limit + early exit via `escalate=True`.
7. **Human-in-the-Loop**: Custom tools trigger external approval systems. Execution pauses for human review.
8. **Composite Patterns**: Nesting multiple patterns. Coordinator routes to parallel branches feeding generator/critic loops.

**Context Management**: Four-layer tiered storage: Working Context (ephemeral), Session (event log), Memory (long-lived searchable), Artifacts (versioned objects). Context compiled through ordered LLM Flows with preprocessors/postprocessors.

**Context Handoff**: `include_contents` parameter controls history flow between agents. Prior assistant messages recast as narrative context to avoid attribution confusion.

**Optimization**: Context caching splits stable prefixes from variable suffixes for computation reuse. Large payloads referenced by handle, loaded on-demand.

Sources: [Google ADK Docs](https://google.github.io/adk-docs/), [Multi-Agent Patterns in ADK](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/), [Context-Aware Framework](https://developers.googleblog.com/en/architecting-efficient-context-aware-multi-agent-framework-for-production/)

---

## 3. Agent Communication Patterns

### 3.1 Protocol Landscape (2025-2026)

Four complementary protocols have emerged:

| Protocol | Scope | Architecture | Message Format | Primary Use |
|----------|-------|-------------|----------------|-------------|
| **MCP** | Tool integration | Client-Server, JSON-RPC | JSON-RPC 2.0 | Augment LLMs with external capabilities |
| **ACP** | Agent messaging | Brokered client-server | Multipart MIME | Typed, multimodal agent messaging |
| **A2A** | Enterprise orchestration | Peer-to-peer | JSON-RPC + Artifacts | Multi-agent workflow orchestration |
| **ANP** | Open internet | Decentralized P2P | JSON-LD + Schema.org | Decentralized agent marketplaces |

**Recommended adoption sequence**:
1. MCP -- establish tool invocation foundation
2. ACP -- layer asynchronous multimodal messaging
3. A2A -- enable enterprise task delegation
4. ANP -- scale to decentralized discovery

Source: [Protocol Survey](https://arxiv.org/html/2505.02279v1)

### 3.2 Historical Foundation: FIPA-ACL

FIPA Agent Communication Language defines communicative acts with precise pre/post-condition semantics grounded in agents' mental states (beliefs, desires, intentions). Performatives include: agree, refuse, request, inform, propose, accept-proposal, reject-proposal.

Interaction protocols: contract net, iterated contract net, subscribe/notify.

### 3.3 Communication Mechanisms

**Direct Messaging**: Agents send structured messages to specific recipients. Simple but creates tight coupling.

**Shared Memory / Blackboard**: Agents read/write to common state store. Low-latency but requires concurrency control.

**Message Passing (Queues)**: Information flows through persistent queues. Decouples components, supports fault recovery.

**Event-Driven / Pub-Sub**: Agents subscribe to event topics. Loose coupling, no direct dependencies.

**Direct Agent-to-Agent Calls**: One agent invokes another as a tool/function. Natural for orchestrator-specialist relationships.

### 3.4 Protocol Enforcement

Structured communication protocols modeled as finite-state machines (FSMs), state-transition systems, or Petri nets. Provides explicit specification of message types, sequencing, roles, and valid actions.

> **Applicability**: For a coding agent orchestrator, a hybrid approach works best: direct messaging for task assignment (orchestrator -> agent), pub-sub for status updates (agent -> all listeners), and shared memory/blackboard for project state (code changes, test results).

Sources: [Agent Communication Protocols Explained](https://www.digitalocean.com/community/tutorials/agent-communication-protocols-explained), [FIPA ACL](https://www.iiia.csic.es/~puyol/SEIAD2001/publicacions/ACL-FIPA.doc.pdf)

---

## 4. Coordination and Consensus

### 4.1 Task Allocation Mechanisms

**Centralized Allocation**: Hub-spoke model. One coordinator assigns work, tracks outputs, handles failures. Simple, consistent, but bottleneck at scale.

**Market-Based Bidding / Auctions**: Agents submit bids (estimated cost, confidence, capability match). Task allocation mechanism selects winners. Distributes decisions, adapts to load, prevents overloading.

**Contract Net Protocol (CNP)**: Manager announces tasks -> contractors bid -> manager awards. Multi-round variants improve allocation quality.

**Hierarchical Decomposition**: Top-level orchestrators manage mid-level agents that manage specialized workers. Each tier handles different decision granularity.

### 4.2 Consensus Mechanisms

**Voting**: Multiple agents independently evaluate the same question. Final answer by majority, supermajority, or weighted vote based on confidence. Reduces hallucination risk through independent validation.

**Deliberation**: Agents exchange reasoning, not just outputs. One produces an answer with reasoning; others evaluate, critique, propose alternatives. More latency but catches errors through multi-perspective evaluation.

**Byzantine Fault Tolerance**: Works even when some agents produce wrong outputs. For LLM agents, valuable for reducing hallucination.

### 4.3 Conflict Resolution

- **Divergence flagging**: Disagreement signals uncertainty, may trigger human escalation
- **Confidence thresholds**: Low confidence triggers human review
- **Graceful degradation**: "degrade gracefully rather than produce confident-looking garbage"
- **Optimistic concurrency**: Merge strategies when conflicts are rare
- **Escalation**: Route conflicts to supervisors or humans
- **Serialization**: Restrict simultaneous updates on critical state

**Critical insight**: "Silent last-write-wins is almost never correct -- it corrupts shared truth without leaving evidence."

### 4.4 Shared State Management

**Three approaches**:
1. Shared memory with transactional concurrency control
2. Message passing through persistent queues (decoupled, fault-recoverable)
3. Direct agent-to-agent calls (tight coupling, doesn't scale)

**Production essentials**: Structured logging with correlation IDs across agent boundaries, latency budgeting for sequential pipelines, explicit human oversight checkpoints for high-stakes decisions, unified state infrastructure.

Sources: [Coordination Mechanisms in MAS](https://apxml.com/courses/agentic-llm-memory-architectures/chapter-5-multi-agent-systems/coordination-mechanisms-mas), [Agent Coordination Patterns](https://tacnode.io/post/multi-agent-coordination), [Galileo Coordination Strategies](https://galileo.ai/blog/multi-agent-coordination-strategies)

---

## 5. Supervision and Fault Tolerance

### 5.1 Erlang/OTP Supervision Trees

**Core philosophy**: Process-oriented programming + "let it crash" + automatic fault recovery.

**Supervision strategies**:
- **One-for-one**: Restart only the failing child
- **One-for-all**: Restart all children if one fails (for interdependent processes)
- **Rest-for-one**: Restart the failing process and all started after it
- **Simple-one-for-one**: Dynamic children with the same specification

**Hierarchy design principle**: Fragile components move deeper into the hierarchy; stable/critical components stay higher up. Supervision structures encode partial failures and fault propagation paths.

**Circuit breaker integration**: Supervisors can apply policies: exponential backoff, circuit breaker trip/untrip, wait for external service health, or request human input.

> **Applicability**: Map directly to agent orchestration. Supervisor = orchestrator. Workers = coding agents. One-for-one restart = retry a failed agent task. One-for-all = restart entire feature branch work if a critical agent fails. Rest-for-one = restart downstream agents when an upstream dependency fails.

Sources: [Adopting Erlang - Supervision Trees](https://adoptingerlang.org/docs/development/supervision_trees/), [OTP Design Principles](https://medium.com/@matheuscamarques/building-fault-tolerant-systems-inside-the-otp-design-principles-of-erlang-8aed442d4a84), [Erlang Design Patterns](https://softwarepatternslexicon.com/erlang/)

### 5.2 Circuit Breaker Pattern for Agent Calls

**Three states**: Closed (normal), Open (failing, reject calls), Half-Open (testing recovery).

**Monitors**: Failed request counts, failure rates over time, specific error codes (429, 502, 503).

**When thresholds exceeded**: Breaker trips, removes failing provider from routing, implements cooldown before recovery attempts.

**Layered resilience approach**:
1. **Retries** handle minor transients (exponential backoff, Retry-After header support)
2. **Fallbacks** provide Plan B routing (switch to secondary provider)
3. **Circuit breakers** prevent systemic collapse (proactive traffic stoppage)

**Key insight on retries**: "Retries don't know when a failure is persistent. If the provider is down, retries just keep hammering the same endpoint." Circuit breakers solve this by proactively stopping traffic.

> **Applicability**: Essential for LLM API calls in agent orchestration. Agent task -> retry with backoff on transient failures -> fall back to alternative model/agent -> circuit breaker trips if systematic failure detected.

Sources: [Circuit Breakers for Agentic AI](https://medium.com/@michael.hannecke/resilience-circuit-breakers-for-agentic-ai-cc7075101486), [Retries, Fallbacks, Circuit Breakers in LLM Apps](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)

### 5.3 Error Recovery Strategies for AI Agents

**Error categories**:
- **Execution-level**: Tool failures, API issues, permission errors (trapable)
- **Semantic**: LLM outputs syntactically valid but logically incorrect (hallucinated API calls, wrong queries)
- **State errors**: Agent beliefs diverge from actual system conditions (cascading failures)
- **Timeouts**: Long-running tasks hang without recovery paths
- **Dependency errors**: External service failures beyond agent control

**Recovery strategies**:

1. **Tool invocation wrapping**: Input validation (JSON schema), retry with backoff, circuit breaker -- prevents transient failures from propagating
2. **Semantic fallback with prompt variants**: Multiple prompt templates; failed outputs trigger alternative prompt paths
3. **Schema enforcement**: Pydantic-style validation; on failure, route to sanitation agent or retry
4. **Modular agent design**: Primary reasoning -> recovery agent -> rule-based fallback (decreasing complexity, graceful degradation)
5. **State verification & self-healing**: Post-action assertions; trigger re-plan or rollback if verification fails
6. **Checkpointing**: Save completion markers after each step; enable rewind to recent checkpoints
7. **Human escalation**: For high-stakes operations, escalate with diffs and execution logs

**Design principles**:
- Observability first (structured logs with retry counts, fallback paths)
- Validation over trust (always validate LLM outputs before execution)
- Treat fallback as foundational (design recovery paths early)
- Failure injection testing (simulate tool failures, rate limits, timeouts)

Source: [Error Recovery in AI Agent Development](https://www.gocodeo.com/post/error-recovery-and-fallback-strategies-in-ai-agent-development)

### 5.4 The Reliability-First Architecture Principle

"Reliability is, first and foremost, an architectural property" -- emerging from component decomposition, interface specifications, and control loops rather than individual models.

**Core formula**: "Models propose, architectures dispose -- with contracts, governors, and sandboxes that convert open-ended reasoning into reliable action."

**Recurring safeguards across all patterns**:
- Simulate-before-actuate for high-risk actions
- Typed contracts enforcing deterministic accept/refuse decisions
- Idempotency and transactional semantics for recovery
- Least-privilege tool access permissions
- Budget and termination rules preventing runaway loops
- End-to-end observability enabling audit and replay

Source: [Architectures for Agentic AI](https://arxiv.org/html/2512.09458v1)

---

## 6. Memory and Context Management

### 6.1 Memory Taxonomy for Multi-Agent Systems

| Memory Type | Function | Retention | Access Pattern |
|------------|----------|-----------|----------------|
| **Working Memory** | Transient state during task execution | Task duration | Fast, narrow scope |
| **Episodic Memory** | Task histories, interaction logs | Weeks to months | Debugging, learning |
| **Semantic Memory** | Durable facts, domain models | Indefinite | Broad retrieval |
| **Procedural Memory** | Learned workflows, successful strategies | Indefinite | Pattern matching |
| **Shared Memory** | Common ground across all agents | Varies | Coordination |

**Critical insight**: Without lifecycle rules, systems accumulate noise alongside signal. Transient data gets over-persisted while durable knowledge gets lost.

### 6.2 The 36.9% Failure Problem

Research shows **36.9% of multi-agent failures stem from interagent misalignment** -- agents lacking visibility into what other agents have completed. Not communication breakdowns but state visibility gaps.

### 6.3 Five Pillars of Memory Engineering

1. **Memory Taxonomy** -- Different types serve different functions; don't treat uniformly
2. **Persistence Architecture** -- Explicit lifecycle policies governing retention per memory type
3. **Retrieval Strategy** -- Multi-dimensional (recency, contextual relevance, scope variance), not just semantic similarity
4. **Coordination Boundaries** -- Define which agents access which memories, read vs write, scope nesting
5. **Consistency Guarantees** -- Optimistic concurrency, escalation, serialization for concurrent updates

### 6.4 Google ADK's Four-Layer Storage Model

- **Working Context**: Ephemeral prompt per model invocation (instructions, identity, selected history, tool outputs)
- **Session**: Durable event log (messages, tool calls, results, errors)
- **Memory**: Long-lived searchable knowledge outliving sessions (preferences, past decisions)
- **Artifacts**: Versioned binary/text objects separate from prompts

**Key innovation**: Context compilation through ordered processors (not ad-hoc prompt templating) -- observable, testable transformation stages.

### 6.5 Context Management Strategies

**Two retrieval approaches**:
- **Reactive recall**: Agents explicitly invoke memory tools when recognizing knowledge gaps
- **Proactive recall**: System pre-fetches likely-relevant snippets via similarity search before model invocation

**Context compaction**: Asynchronous LLM-based summarization of older events within sliding windows. Summaries replace raw events.

**Context caching**: Divide windows into stable prefixes (instructions, identity) and variable suffixes (latest turns) for computation reuse.

**Artifact externalization**: Large payloads referenced by handle, loaded on-demand, automatically offloaded post-completion.

### 6.6 Why Context Engineering Alone Fails for Multi-Agent

Single-agent context engineering doesn't scale because:
1. Context must be **shared across agents** with consistency guarantees
2. **Context degradation is contagious**: Agent A's degraded output enters Agent B's context as ground truth
3. **More context makes problems worse**: Transcript replay accumulates old mistakes; retrieval surfaces outdated state
4. **Token economics**: Single agents use ~4x tokens over chat; multi-agent uses ~15x

**Memory eliminates multipliers**: Reduces redundant retrieval, context re-explanation, and duplicate work from visibility gaps.

### 6.7 Heterogeneous Agent Teams

Memory engineering enables cost-effective architectures:
- Small models (7B) for routine execution; large models (70-175B) for complex reasoning
- Cost difference: 10-30x between serving tiers
- Small models depend on external memory for coordination -- can't maintain full context independently
- Without shared memory, every agent must be large enough for self-sufficiency

Sources: [Memory Engineering for MAS (O'Reilly)](https://www.oreilly.com/radar/why-multi-agent-systems-need-memory-engineering/), [Context-Aware Multi-Agent Framework (Google)](https://developers.googleblog.com/en/architecting-efficient-context-aware-multi-agent-framework-for-production/), [Memory Survey Paper](https://github.com/Shichun-Liu/Agent-Memory-Paper-List)

---

## 7. Academic/Industry Surveys

### 7.1 Multi-Agent Collaboration Mechanisms Survey (Jan 2025)

**Taxonomy dimensions**:
- **Actors**: Agents involved in collaboration
- **Types**: Cooperation, Competition, Coopetition
- **Structures**: Centralized, Decentralized, Hierarchical
- **Strategies**: Rule-based, Role-based, Model-based
- **Coordination**: Static or dynamic orchestration

**Collaboration types**:
- **Cooperation**: Aligned objectives toward shared goals
- **Competition**: Conflicting objectives (debate, adversarial refinement)
- **Coopetition**: Blend of both (mixture-of-experts: compete via gating, cooperate toward unified output)

**Communication structures**:
- **Centralized**: Hub coordinator, simple but single point of failure
- **Decentralized**: Peer-to-peer, scalable but communication overhead
- **Hierarchical**: Layered authority, reduced bottlenecks but latency

**Strategies**:
- **Rule-based**: Predefined interaction rules. Efficient, predictable, but rigid
- **Role-based**: Segmented objectives by domain. Modular, reusable, but rigid structure
- **Model-based**: Probabilistic models with uncertainty handling. Adaptive, robust, but complex

Source: [Multi-Agent Collaboration Mechanisms Survey](https://arxiv.org/abs/2501.06322)

### 7.2 LLM-based Multi-Agent Systems Survey

Five key components: profile, perception, self-action, mutual interaction, and evolution. Applications: (i) solving complex tasks, (ii) simulating scenarios, (iii) evaluating generative agents.

Source: [LLM-based MAS Survey](https://link.springer.com/article/10.1007/s44336-024-00009-2)

### 7.3 Agentic LLMs Survey (March 2025)

Research focus areas: reasoning/reflection/retrieval for decision-making; action models/robots/tools for assistants; multi-agent systems for collaborative task solving and emergent social behavior.

Source: [Agentic LLMs Survey](https://arxiv.org/abs/2503.23037)

### 7.4 Architecture Families (Chapter 3 Survey)

Five architecture families for agentic AI:

1. **Tool-Using Agents**: MRKL (route to specialized modules), ReAct (alternate reasoning + tool calls), ReWOO (separate plan from observation)
2. **Memory-Augmented Agents**: Working/episodic/semantic memory layers with provenance tracking and poisoning defenses
3. **Planning and Self-Improvement**: Tree of Thoughts, Graph of Thoughts, Program-Aided Language, Reflexion
4. **Multi-Agent Systems**: Supervisor-worker, peer collaboration, role-play topologies
5. **Embodied and Web Agents**: Simulation-before-actuation, narrow capability tokens, reversible actions

**Architectural building blocks spine**: Goal manager, Planner, Tool router, Executor with sandboxing, Memory layers, Verifiers/critics, Safety monitor, Telemetry.

Source: [Architectures for Building Agentic AI](https://arxiv.org/html/2512.09458v1)

### 7.5 Research Scale

Papers on agentic and multi-agent systems: 820 in 2024 -> 2,500+ in 2025, indicating primary research focus across top labs and universities.

---

## 8. Extractable Principles for Agent Orchestration

### 8.1 Architecture Selection

| Scenario | Best Pattern | Why |
|----------|-------------|-----|
| Predictable workflow steps | Sequential Pipeline / Prompt Chaining | Deterministic, easy to debug |
| Unknown number of subtasks | Orchestrator-Workers | Dynamic decomposition |
| Quality-critical output | Evaluator-Optimizer / Generator-Critic | Iterative refinement |
| Speed-critical parallel work | Parallelization / Fan-Out-Gather | Concurrent execution |
| Open-ended exploration | Autonomous Agent loops | Environmental feedback |
| Multiple specialist domains | Routing / Coordinator | Intent-based delegation |
| High-stakes operations | Human-in-the-Loop | Approval gates |
| Real-world complex systems | Composite Patterns | Multiple patterns combined |

### 8.2 Core Design Principles

1. **"Models propose, architectures dispose"** -- Treat LLM outputs as proposals requiring validation through contracts, governors, and sandboxes.

2. **Start simple, add complexity only when demonstrably improving outcomes** -- Prompt chaining before orchestrator-workers before autonomous agents.

3. **Separate storage from presentation** -- Durable schemas evolve independently from prompt formats.

4. **Explicit transformations** -- Named, ordered processors replace string concatenation for context management.

5. **Scope by default** -- Agents receive minimum required context; must explicitly request additional data.

6. **Invest in Agent-Computer Interface** -- Tool design and documentation matter more than overall prompt engineering.

7. **Simulate before actuate** -- Dry-run high-risk actions. Reversible/compensating actions where possible.

8. **Typed contracts** -- Schema-constrained outputs, capability-scoped permissions, idempotent tools.

9. **Budget and termination rules** -- Max iterations, token budgets, time limits prevent runaway agents.

10. **End-to-end observability** -- Structured event logs with correlation IDs for audit, replay, and debugging.

### 8.3 Supervision Hierarchy for Coding Agents

Mapping OTP supervision to agent orchestration:

```
System Supervisor (restart policies, circuit breakers)
  |
  +-- Orchestrator (task decomposition, assignment, synthesis)
  |     |
  |     +-- Planning Agent (analyze requirements, design approach)
  |     +-- Implementation Agent(s) (code changes, parallel by file/module)
  |     +-- Test Agent (write/run tests)
  |     +-- Review Agent (evaluate changes, suggest improvements)
  |
  +-- Memory Service (shared state, project context, episodic logs)
  +-- Tool Service (git, build, test runners, file system)
  +-- Health Monitor (agent status, resource usage, circuit breakers)
```

**Restart strategies**:
- Agent task failure -> retry with backoff (one-for-one)
- Shared state corruption -> restart all agents on that task (one-for-all)
- Build/test infrastructure failure -> restart downstream agents (rest-for-one)

### 8.4 Error Handling Hierarchy

```
Level 1: Tool-level retries (exponential backoff, 3 attempts)
Level 2: Agent-level fallback (alternative prompt, simpler approach)
Level 3: Orchestrator re-assignment (different agent, decompose differently)
Level 4: Circuit breaker (stop trying, report degraded capability)
Level 5: Human escalation (present context, diffs, logs for decision)
```

### 8.5 Memory Architecture for Coding Agents

```
Working Memory: Current task context, active file contents, recent tool outputs
  |-- Per-agent, ephemeral, discarded after task completion

Episodic Memory: Task execution logs, decisions made, errors encountered
  |-- Per-task, retained for learning and debugging

Semantic Memory: Codebase knowledge, architecture patterns, API contracts
  |-- Project-level, persists indefinitely, updated on significant changes

Shared Memory: Project state, file change tracker, test results, build status
  |-- Cross-agent, transactional updates, real-time visibility
```

### 8.6 Task Decomposition Strategy

1. **Lazy decomposition** -- Only break tasks into next-level subtasks when necessary (HTDAG principle)
2. **DAG representation** -- Subtasks as nodes, dependencies as directed edges
3. **Atomic vs non-atomic** -- Irreducible actions vs further-decomposable tasks
4. **Parallel where possible** -- Independent subtasks execute concurrently
5. **Replanning on failure** -- Failed nodes trigger context-aware re-planning; successful nodes persist
6. **Escalation** -- Unresolved lower-level failures escalate to parent nodes for higher-level strategy

### 8.7 Communication Pattern Selection

| Need | Pattern | Trade-off |
|------|---------|-----------|
| Task assignment | Direct messaging | Simple but coupled |
| Status updates | Pub-sub / Events | Decoupled but eventual consistency |
| Project state | Shared memory / Blackboard | Fast but needs concurrency control |
| Agent negotiation | Request-response | Synchronous overhead |
| Result synthesis | Fan-in aggregation | Requires coordination point |

---

## 9. Comparison: MAS Patterns vs ORCHA Patterns

| Dimension | MAS Best Practices | ORCHA (Current) | Gap/Opportunity |
|-----------|-------------------|-----------------|-----------------|
| **Architecture style** | Hybrid reactive/deliberative — fast reactive layer (health, timeouts) + deliberative planning layer | Single-layer: agent flows defined as data model, no runtime engine | Add two-layer architecture: reactive event loop + deliberative planner |
| **Agent mental model** | BDI — explicit beliefs, desires, intentions with commitment strategies | Implicit — agent state is conversation history | Formalize agent state: beliefs (project context), desires (task goals), intentions (current plan) |
| **Task decomposition** | Hierarchical Task DAG with lazy decomposition, atomic/non-atomic distinction | Flat task list, manual decomposition | Add HTDAG: recursive decomposition, lazy sub-task generation, DAG dependencies |
| **Task allocation** | Contract Net Protocol / capability-based bidding | Static assignment by user or hardcoded routing | Add capability matching + load-aware assignment |
| **Communication** | Hybrid: direct messaging (assignment) + pub-sub (status) + blackboard (shared state) | Direct only: orchestrator calls agent, agent returns result | Add event bus for status updates + blackboard for shared project state |
| **Supervision** | OTP supervision trees — one-for-one, one-for-all, rest-for-one restart strategies | No supervision — failed agents are just "failed" | Add supervisor hierarchy with typed restart strategies |
| **Fault tolerance** | 5-level hierarchy: retry → fallback → reassign → circuit breaker → human escalation | Binary: succeed or fail, manual retry | Add layered error handling with circuit breakers and automatic fallback |
| **Memory architecture** | 4-5 layer taxonomy: working, episodic, semantic, procedural, shared | Execution artifacts (blobs) + conversation history | Add structured memory layers with lifecycle policies |
| **Context management** | Context compilation via ordered processors, caching, compaction | Raw conversation replay | Add context compilation pipeline with summarization + caching |
| **Parallel execution** | Fan-out/gather with unique output keys, parallel state regions | Serial agent execution | Add parallel agent dispatch with synchronization barriers |
| **Observability** | Structured event logs with correlation IDs across agent boundaries | Basic execution logs | Add correlation IDs, structured telemetry, cross-agent tracing |
| **Agent-Computer Interface** | Tool design > prompt engineering — typed contracts, idempotent tools, schema-constrained outputs | MCP tools with informal contracts | Formalize tool contracts: schemas, idempotency, capability scoping |
| **Conflict resolution** | Divergence flagging, confidence thresholds, optimistic concurrency, escalation | No multi-agent conflict handling | Add conflict detection + resolution strategies for concurrent agent work |
| **Termination control** | Budget/iteration/time limits, circuit breakers | Implicit timeout only | Add explicit budget controls: max iterations, token limits, time bounds |

---

## 10. Actionable Recommendations for ORCHA

### High Priority

1. **Hybrid Reactive/Deliberative Architecture** — Split the orchestration engine into two layers: (a) a **reactive event loop** that handles health checks, timeouts, status transitions, and circuit breaker logic with sub-second latency; (b) a **deliberative planner** that handles task decomposition, agent selection, re-planning on failure, and workflow progression. The reactive layer runs continuously; the deliberative layer activates on significant events (task completion, failure, new work). This is "the most practical template for agentic AI" per the architecture survey.

2. **OTP-Style Supervision Hierarchy** — Implement Erlang/OTP supervision tree patterns for agent lifecycle management. The orchestrator is the supervisor; agents are workers. Three restart strategies: **one-for-one** (retry the failed agent), **one-for-all** (restart all agents on a task when shared state is corrupted), **rest-for-one** (restart downstream agents when an upstream dependency fails). Fragile components (individual agent tasks) go deep in the tree; stable components (the orchestrator, memory service, tool service) stay shallow. This provides automatic fault recovery without manual intervention.

3. **5-Level Error Handling Hierarchy** — Replace binary succeed/fail with layered error handling: (1) tool-level retries with exponential backoff (3 attempts), (2) agent-level fallback (alternative prompt or simpler approach), (3) orchestrator re-assignment (different agent or decompose differently), (4) circuit breaker (stop trying, report degraded capability), (5) human escalation (present context + diffs + logs for decision). Each level is progressively more expensive and slower but handles progressively harder failures.

4. **Blackboard Pattern for Shared Project State** — Implement a shared knowledge store (blackboard) where agents post and retrieve project state asynchronously: file changes, test results, build status, discovered patterns, error contexts. Agents don't need to communicate directly — they read/write to the blackboard. This solves the 36.9% failure rate from interagent misalignment (agents lacking visibility into what others have completed). Converges with ATLAS's knowledge-as-first-class-entity recommendation.

### Medium Priority

5. **Hierarchical Task DAG with Lazy Decomposition** — Replace flat task lists with a recursive task DAG. Tasks are either **atomic** (single file edit, single test) or **non-atomic** (decomposable into sub-tasks). Decomposition happens lazily — only break down the next level when an agent begins work, preventing premature over-planning. Failed nodes trigger context-aware re-planning while successful nodes persist. This is the Deep Agent HTDAG pattern — the most battle-tested approach to coding task decomposition.

6. **Structured Memory Layers with Lifecycle Policies** — Implement four memory tiers: (a) **Working memory** — per-agent, ephemeral, current task context only; (b) **Episodic memory** — per-task execution logs, decisions, errors (retained for weeks); (c) **Semantic memory** — project-level codebase knowledge, architecture patterns (persists indefinitely); (d) **Shared memory** — cross-agent project state with transactional updates. Each tier has explicit retention and garbage collection policies. Without lifecycle rules, systems accumulate noise alongside signal.

7. **Circuit Breaker Pattern for LLM API Calls** — Add circuit breakers (closed → open → half-open) to all LLM provider integrations. Monitor failure rates, specific error codes (429, 502, 503). When thresholds exceed, trip the breaker: stop sending requests to the failing provider, route to fallback (alternative model), implement cooldown before recovery attempts. "Retries don't know when a failure is persistent" — circuit breakers do.

8. **Context Compilation Pipeline** — Replace raw conversation replay with ordered context processors: (a) compile stable prefix (instructions, identity, tool schemas) separately from variable suffix (latest turns, tool outputs); (b) summarize older events via sliding-window compaction; (c) externalize large payloads by handle, load on-demand. Google ADK's context compilation approach reduces token costs while maintaining agent accuracy.

### Lower Priority

9. **Evaluator-Optimizer Loops for Quality-Critical Output** — For quality-critical workflow steps (code review, spec validation), implement generator-critic loops: one agent produces output, a second evaluates against criteria. Loop continues until the critic passes or `max_iterations` is reached. Google ADK's LoopAgent pattern with PASS/FAIL critics + `escalate=True` early exit.

10. **Contract Net Protocol for Dynamic Task Assignment** — Instead of static agent assignment, let the orchestrator announce tasks and agents bid based on capability match, current load, and estimated cost. The orchestrator awards to the best bidder. Enables natural load balancing and capability-based routing without hardcoded assignment rules.

11. **Correlation IDs for Cross-Agent Tracing** — Every agent operation carries a correlation ID linking it to the parent task, workflow, and originating user request. Structured event logs with these IDs enable: tracing a failure back through the full agent chain, calculating per-task token costs, identifying bottleneck agents, and auditing decision pathways.

12. **Explicit Termination Controls** — Every agent invocation must have explicit bounds: max iterations, max tokens, max wall-clock time. Budget controls prevent runaway agents without heavy infrastructure. The orchestrator tracks cumulative costs per task and per workflow, escalating when budgets approach limits.

---

## Key Takeaways

1. **"Models propose, architectures dispose"** — The single most important principle. LLM outputs are proposals, not commands. Typed contracts, validation gates, and sandboxes convert open-ended reasoning into reliable action. The architecture provides reliability — the model provides intelligence.

2. **Hybrid architecture is the practical default** — Fast reactive layer (health, timeouts, status) + slow deliberative layer (planning, assignment, re-planning). Neither pure reactive nor pure deliberative works for real agent systems.

3. **36.9% of multi-agent failures are visibility gaps** — Not communication breakdowns but agents lacking awareness of what other agents have done. Shared state (blackboard pattern) is not optional for multi-agent coordination.

4. **Supervision trees formalize fault recovery** — OTP's one-for-one/one-for-all/rest-for-one strategies map directly to agent orchestration. Encode which failures propagate and which are isolated. Fragile components go deep; stable components stay shallow.

5. **5-level error handling beats binary succeed/fail** — Retry → fallback → reassign → circuit break → human escalate. Each level is progressively more expensive but handles harder failures. Most failures resolve at levels 1-2 (cheap); levels 4-5 activate rarely but prevent catastrophic outcomes.

6. **Lazy decomposition prevents premature planning** — Don't decompose all tasks upfront. Break down the next level only when an agent begins work. Failed nodes replan while successful ones persist — partial progress is preserved.

7. **Context engineering alone fails for multi-agent** — Context degradation is contagious (Agent A's bad output becomes Agent B's ground truth). Token economics multiply (15x baseline). Structured memory with lifecycle policies is the solution.

8. **Tool design matters more than prompt engineering** — "Invest more in Agent-Computer Interface than overall prompts." Well-designed tools with schema-constrained outputs, clear error messages, and idempotent semantics reduce agent errors more than prompt optimization.

9. **Start simple, add complexity only when demonstrably improving outcomes** — Prompt chaining before orchestrator-workers before autonomous agents. The simplest pattern that solves the problem is the right one.

10. **Observability is non-negotiable** — Structured event logs with correlation IDs, latency budgeting, retry counts, and fallback paths. Non-determinism makes traditional debugging impossible — production tracing is the only way to understand multi-agent behavior.

---

## Source Index

### Architecture & Patterns
- [Anthropic - Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)
- [Anthropic - Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Architectures for Building Agentic AI (arxiv)](https://arxiv.org/html/2512.09458v1)
- [Four Design Patterns for Event-Driven MAS (Confluent)](https://www.confluent.io/blog/event-driven-multi-agent-systems/)
- [AI Agent Architecture Patterns 2025 (NexAI)](https://nexaitech.com/multi-ai-agent-architecutre-patterns-for-scale/)
- [Blackboard Pattern with MCPs](https://medium.com/@dp2580/building-intelligent-multi-agent-systems-with-mcps-and-the-blackboard-pattern-to-build-systems-a454705d5672)

### Frameworks
- [Google ADK Multi-Agent Patterns](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/)
- [Google ADK Context-Aware Framework](https://developers.googleblog.com/en/architecting-efficient-context-aware-multi-agent-framework-for-production/)
- [LangGraph vs CrewAI vs AutoGen 2026](https://dev.to/pockit_tools/langgraph-vs-crewai-vs-autogen-the-complete-multi-agent-ai-orchestration-guide-for-2026-2d63)
- [OpenAI Swarm GitHub](https://github.com/openai/swarm)
- [OpenAI Cookbook - Orchestrating Agents](https://cookbook.openai.com/examples/orchestrating_agents)
- [Top AI Agent Frameworks 2026 (aimultiple)](https://aimultiple.com/agentic-frameworks)
- [Multi-Agent Frameworks for Enterprise (adopt.ai)](https://www.adopt.ai/blog/multi-agent-frameworks)

### Communication Protocols
- [MCP, ACP, A2A, ANP Survey (arxiv)](https://arxiv.org/html/2505.02279v1)
- [MCP Multi-Agent Architecture (arxiv)](https://arxiv.org/html/2504.21030v1)
- [Agent Communication Protocols (DigitalOcean)](https://www.digitalocean.com/community/tutorials/agent-communication-protocols-explained)
- [FIPA ACL Standards](https://smythos.com/developers/agent-development/fipa-agent-communication-language/)

### Coordination & Consensus
- [Coordination Mechanisms in MAS](https://apxml.com/courses/agentic-llm-memory-architectures/chapter-5-multi-agent-systems/coordination-mechanisms-mas)
- [Agent Coordination Patterns (Tacnode)](https://tacnode.io/post/multi-agent-coordination)
- [Multi-Agent Coordination Survey (arxiv)](https://arxiv.org/html/2502.14743v2)
- [Decentralized Task Allocation (Nature)](https://www.nature.com/articles/s41598-025-21709-9)

### Fault Tolerance
- [OTP Supervision Trees (Adopting Erlang)](https://adoptingerlang.org/docs/development/supervision_trees/)
- [Erlang Design Patterns](https://softwarepatternslexicon.com/erlang/)
- [Retries, Fallbacks, Circuit Breakers (Portkey)](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)
- [Error Recovery in AI Agents (GoCodeo)](https://www.gocodeo.com/post/error-recovery-and-fallback-strategies-in-ai-agent-development)

### Memory & Context
- [Memory Engineering for MAS (O'Reilly)](https://www.oreilly.com/radar/why-multi-agent-systems-need-memory-engineering/)
- [MAGMA Memory Architecture (arxiv)](https://arxiv.org/html/2601.03236v1)
- [A-Mem Agentic Memory (arxiv)](https://arxiv.org/html/2502.12110v1)
- [Memory Survey Paper List (GitHub)](https://github.com/Shichun-Liu/Agent-Memory-Paper-List)

### Task Decomposition
- [Deep Agent HTDAG (arxiv)](https://arxiv.org/html/2502.07056v1)
- [Task Decomposition for Coding Agents (MGX)](https://mgx.dev/insights/task-decomposition-for-coding-agents-architectures-advancements-and-future-directions/a95f933f2c6541fc9e1fb352b429da15)
- [AgentOrchestra Hierarchical Framework (arxiv)](https://arxiv.org/html/2506.12508v1)
- [TDAG Dynamic Decomposition (arxiv)](https://arxiv.org/abs/2402.10178)

### Surveys
- [Multi-Agent Collaboration Survey (arxiv)](https://arxiv.org/abs/2501.06322)
- [LLM-based MAS Survey (Springer)](https://link.springer.com/article/10.1007/s44336-024-00009-2)
- [Agentic LLMs Survey (arxiv)](https://arxiv.org/abs/2503.23037)
- [LLM Agents Survey 2022-2026](https://llm-agents-survey.github.io/)
- [Anthropic Multi-Agent Reflections](https://llmmultiagents.com/en/blogs/anthropic-multi-agent-system-reflection)
- [ByteByteGo Analysis](https://blog.bytebytego.com/p/how-anthropic-built-a-multi-agent)
