Now I have comprehensive research across all 10 topics. Here is the full synthesis.

---

# Workflow Engine & Orchestration Research (Early 2026)

## 1. Temporal.io

**Architecture**: Temporal uses a server cluster + worker process model. The server durably persists workflow state, and workers long-poll for tasks. Workflows must be deterministic (for replay); Activities handle non-deterministic side effects (API calls, LLM invocations). Communication happens via gRPC.

**Rust SDK**: Temporal's Core SDK is written in Rust and serves as the foundation for all language-specific SDKs. Language SDKs communicate with the Rust core via C bindings. There is a `temporal-sdk-core` crate and a `temporalio-sdk` crate on crates.io. However, the Rust SDK is primarily an internal building block -- most users write workflows in TypeScript, Python, Go, or Java rather than authoring workflows directly in Rust.

**AI/Agent Orchestration** (major 2025 push): Temporal launched an OpenAI Agents SDK integration in 2025. Their recommended pattern: Workflows = deterministic orchestration layer (the "blueprint"); Activities = non-deterministic work (LLM calls, tool invocations). This maps cleanly to AI agent orchestration where the workflow graph is deterministic but individual steps are stochastic. Temporal reports that Gartner predicts 40% of enterprise apps will feature task-specific AI agents by end of 2026.

**Long-running workflows**: Workflows can sleep for hours, days, or months without consuming compute. State is durably persisted. When a signal or timer fires, the workflow replays from its event history and resumes exactly where it stopped.

**2026 platform status**: Temporal Nexus (cross-namespace workflow linking) reached GA. Multi-Region Replication went GA with 99.99% SLA. New SDKs for Ruby and .NET in pre-release/beta.

**Relevance to ORCHA**: Strong fit for the orchestration backbone -- deterministic workflow graph with non-deterministic LLM activity steps. Main drawback is operational complexity (requires running a Temporal cluster or paying for Temporal Cloud).

## 2. Inngest

**Architecture**: Serverless, event-driven. No stateful backend to manage -- runs on AWS Lambda, Cloudflare Workers, or your own containers. Functions are triggered by events and execute as multi-step durable functions. Uses native language primitives (no special runtime proxy), making debugging simpler than Temporal.

**Comparison with Temporal**:
- Inngest: simpler ops, serverless-native, event-first model, smaller community (5k GitHub stars vs Temporal's 19k)
- Temporal: battle-tested durability, richer workflow patterns, larger ecosystem, but heavier operational burden
- Inngest favors choreography; Temporal favors orchestration

**Human-in-the-loop**: `step.waitForEvent()` pauses execution until a matching event arrives or timeout. Supports CEL expressions for conditional matching. AgentKit (Inngest's agent framework) uses this for developer approval workflows.

**Relevance to ORCHA**: Good inspiration for the event-driven trigger model and the `waitForEvent` pattern. The serverless model is less relevant since ORCHA runs its own backend, but the API design patterns are worth borrowing.

## 3. Restate.dev (Notable Addition)

**Architecture**: A Rust-native durable execution engine, shipping as a single binary. Built around a command log and event processor using Tokio. Has first-class Rust SDK (`restate-sdk` crate). Push-based model (vs Temporal's pull/poll model) gives lower latency.

**Key differentiator**: Checkpoint-based recovery rather than deterministic replay. This means your workflow code doesn't need to be deterministic -- Restate journals the results of side effects and replays from checkpoints. This is a significant advantage for AI workflows where LLM calls are inherently non-deterministic.

**Relevance to ORCHA**: Strongest architectural fit. Rust-native, embeddable, no external cluster needed, checkpoint-based (not replay-based). Could potentially be embedded directly into the ORCHA backend. Worth deep investigation.

## 4. FSM Libraries in Rust

**rust-fsm** (most mature): Macro-based DSL for defining state machines with readable specifications. Defines states, events, and transitions declaratively. Active on crates.io.

**finny.rs**: Builder-style API with procedural macros, supports generics in states and contexts. Shares context across guards, actions, and transitions.

**Type State Pattern**: Uses Rust's type system to enforce valid transitions at compile time -- invalid state transitions become compilation errors. No runtime overhead. This is the idiomatic Rust approach for smaller, well-defined state machines.

**Relevance to ORCHA**: The Type State pattern is ideal for workflow step status tracking (e.g., `Pending -> Running -> Completed | Failed`). For the broader workflow execution engine, a custom FSM using enums + match is likely more flexible than any library. The libraries are better suited for protocol-level state machines (connection states, etc.).

## 5. Event Sourcing + CQRS in Rust

**cqrs-es crate**: Lightweight, opinionated framework targeting serverless architectures. Separates command (write) and query (read) models. Events are the source of truth; state is derived by replaying events.

**Other frameworks**: `event_sourcing.rs` (Prima, uses sqlx), `eventmill`, `eventsourcing` crate.

**Key pattern for workflows**: Instead of storing "workflow is at step 3", store events: `WorkflowStarted`, `Step1Completed`, `Step2Started`, `Step2Failed`, `Step2Retried`, `Step2Completed`, `Step3Started`... Current state is derived by folding events. This gives you: full audit trail, time-travel debugging, ability to rebuild state from scratch, natural fit for workflow observability.

**Relevance to ORCHA**: Event sourcing is the natural persistence model for workflow execution. Each workflow run produces an event stream. The current execution state is a projection. Combined with CQRS, you get a fast read model for the UI (kanban board showing execution status) separate from the write model (appending execution events). The `cqrs-es` crate is a reasonable starting point, though a custom implementation may be simpler for ORCHA's specific needs.

## 6. Workflow Marketplace/Templates

**n8n**: 8,515+ community workflow templates. Node-based visual editor. Workflows are JSON documents describing node graphs with connections. Third-party marketplaces (haveworkflow.com, n8nmarket.com, FlowTemplate) sell premium templates.

**Structure patterns**:
- Workflows are serialized as JSON/YAML with nodes (steps) and edges (connections)
- Templates include metadata: name, description, category, required credentials/integrations
- Import/export via copy-paste or marketplace API
- Version pinning for template compatibility

**Relevance to ORCHA**: ORCHA's workflow definitions are already JSON-based. To build a template marketplace: (1) add metadata schema (author, category, required integrations, version), (2) parameterize credentials and org-specific values, (3) provide import/export with variable substitution. The n8n model of community-contributed templates with an affiliate program is a proven growth strategy.

## 7. DAG-Based Task Execution

**Airflow 3.0** (April 2025 -- biggest update ever): Introduced DAG versioning, multi-language Task SDKs, and event-driven scheduling with Data Assets. 30% of users now use it for MLOps, 10% for GenAI. 80,000+ organizations, 30M monthly downloads.

**Dagster**: Asset-centric model -- data assets (tables, models, reports) are first-class citizens rather than tasks. Better lineage tracking. Components framework reached GA October 2025.

**Prefect**: Python-native, focused on resilient execution. Strongest when workflows are dynamic and not strictly DAG-shaped.

**Key patterns**:
- DAG = nodes (tasks) + directed edges (dependencies). Executor resolves which tasks can run in parallel based on dependency satisfaction
- Fan-out/fan-in: parallel branches that converge at a join point
- Dynamic DAGs: generated at runtime based on parameters (Prefect excels here)
- Data passing: task outputs feed into downstream task inputs (Dagster materializes these as "assets")

**Relevance to ORCHA**: The DAG model maps directly to workflow step dependencies. Key design decision: static DAGs (defined at design time, like Airflow) vs dynamic DAGs (where an LLM agent decides the next step at runtime). ORCHA likely needs both -- a static DAG backbone with dynamic branching at agent decision points. Dagster's asset-centric thinking is interesting for workflows that produce artifacts.

## 8. Cron/Event Triggers

**Three trigger types** (as standardized by Kestra and others):
1. **Cron/Schedule**: Standard cron expressions, one-off scheduled events
2. **Webhook/HTTP**: Inbound POST requests triggering immediate execution
3. **Event/Message**: Kafka topics, pub/sub subscriptions, inter-workflow triggers

**Combining patterns**:
- **Flow triggers**: Execute when another workflow completes (chaining)
- **Polling triggers**: Periodically check external systems for new data
- **Realtime triggers**: WebSocket/SSE for millisecond-latency responses
- **Hybrid**: Cron as fallback, webhook as primary (ensures data is processed even if webhook fails)

**Relevance to ORCHA**: ORCHA already has webhook triggers for data source workflows. Adding cron scheduling and inter-workflow triggers (flow triggers) would enable: periodic data refresh workflows, workflow chaining (output of one feeds into another), and event-driven reactions to CRM changes.

## 9. Workflow Versioning

**Temporal's two approaches**:
1. **Worker Versioning** (GA expected Q4 2025, experimental support removed March 2026): Tag workers with version identifiers. Old workers run old code, new workers run new code. Server routes tasks to the appropriate version.
2. **Patching**: Add version-aware branches in code. New executions take the new path; in-flight executions continue on the old path. Deprecate patches once no old executions remain.

**General patterns**:
- **Immutable definitions**: Each workflow definition version is immutable. New versions get new IDs (e.g., `workflow-v2`). In-flight executions continue on their original version.
- **Migration windows**: Track which versions have active executions. Only retire a version when all its executions complete.
- **Schema evolution**: Add fields with defaults (backward compatible). Removing or renaming fields requires a new major version.

**Relevance to ORCHA**: ORCHA's workflow definitions should be versioned immutably. When a user edits a workflow, create a new version. Running executions continue on their original version. The UI shows "v3 (current)" with ability to view older versions. This is simpler than Temporal's patching approach since ORCHA workflows are data-driven (JSON definitions) rather than code-driven.

## 10. Human-in-the-Loop

**Temporal pattern**: Workflow calls `wait_condition()` or waits for a Signal. Execution is suspended (no compute consumed). External system (e.g., approval UI) sends a Signal to resume. Durable timers enforce deadlines. The workflow can sleep for days waiting for human input, surviving any infrastructure failures.

**Inngest pattern**: `step.waitForEvent("approval/received", { timeout: "7d", match: "data.ticketId" })` pauses until matching event arrives. Events fan out to multiple functions. Built-in audit trail since events are stored in OLAP.

**Cloudflare Workflows**: Similar `waitForEvent` pattern with built-in webhook endpoints for triggering resume events.

**Common implementation**: (1) Workflow reaches approval step, (2) sends notification (email/Slack/UI), (3) suspends execution, (4) external action sends event/signal with decision, (5) workflow resumes with the decision data and branches accordingly.

**Relevance to ORCHA**: Critical for AI orchestration where agents need human approval before taking consequential actions. Implementation: workflow step type `"human_approval"` that suspends execution, creates a notification, and waits for an API call to `/api/workflows/executions/:id/approve` or `/reject`. Store the pending approval as a workflow event. Resume by replaying from the approval event.

## 11. Workflow Observability

**OpenTelemetry** is the converged standard (2025-2026). Three pillars: metrics (something's wrong), traces (where it is), logs (why it happened).

**Distributed tracing model**: Each workflow execution = a trace. Each step = a span. Spans nest (sub-workflows, retries). Each span records: start time, duration, status, metadata (LLM model used, token count, cost).

**Tools**: SigNoz (full-stack, self-hostable), Jaeger 2.0 (OTel-native, released late 2024), Honeycomb (trace-first analysis).

**AI-specific observability** (emerging 2025-2026): Braintrust, Maxim, and others offer agent-specific tracing -- tracking LLM calls, tool invocations, reasoning chains, token usage, and cost per step.

**Relevance to ORCHA**: Emit OpenTelemetry spans for each workflow step execution. Key attributes: `workflow.id`, `step.id`, `step.type` (deterministic vs LLM), `llm.model`, `llm.tokens`, `llm.cost`, `step.duration_ms`, `step.status`. This enables: flame-chart visualization of workflow execution, cost attribution per step/workflow, failure analysis (which step failed, retry count), performance regression detection.

---

## Synthesis: Recommended Architecture for ORCHA

Given ORCHA's needs (Rust backend, AI agent orchestration, deterministic + LLM steps, human approval), the strongest architectural patterns are:

1. **Execution engine**: Restate-inspired checkpoint-based durable execution (Rust-native, no deterministic replay requirement). Alternatively, a custom event-sourced engine using the workflow definition JSON as the DAG specification.

2. **State management**: Event sourcing for execution state. Each step completion/failure is an event. Current state derived by projection. CQRS for separating the execution write path from the UI read path.

3. **Step types**: Deterministic (code/transform), LLM (agent call), Human-in-the-loop (waitForEvent pattern), Sub-workflow (Temporal Nexus-style linking).

4. **Triggers**: Cron (schedule), Webhook (HTTP POST), Event (inter-workflow, CRM changes), Manual (UI button).

5. **Versioning**: Immutable versioned definitions. Running executions pinned to their creation-time version.

6. **Observability**: OpenTelemetry spans per step, with AI-specific attributes (model, tokens, cost).

7. **Workflow templates**: JSON definitions with parameterized credentials and org-specific values. Metadata schema for marketplace discovery.

---

Sources:
- [Temporal Core SDK (Rust)](https://github.com/temporalio/sdk-core)
- [Why Rust Powers Temporal's Core SDK](https://temporal.io/blog/why-rust-powers-core-sdk)
- [Temporal SDK Core Architecture](https://github.com/temporalio/sdk-core/blob/master/ARCHITECTURE.md)
- [Temporal for AI](https://temporal.io/solutions/ai)
- [Building Durable AI Agents with Temporal](https://temporal.io/blog/build-resilient-agentic-ai-with-temporal)
- [Temporal + OpenAI Agents SDK Integration](https://temporal.io/blog/announcing-openai-agents-sdk-integration)
- [Durable Multi-Agent AI Architecture with Temporal](https://temporal.io/blog/using-multi-agent-architectures-with-temporal)
- [Inngest vs Temporal Comparison (2026)](https://openalternative.co/compare/inngest/vs/temporal)
- [Inngest vs Temporal (Akka)](https://akka.io/blog/inngest-vs-temporal)
- [Inngest vs Temporal (Inngest)](https://www.inngest.com/compare-to-temporal)
- [Inngest step.waitForEvent Documentation](https://www.inngest.com/docs/features/inngest-functions/steps-workflows/wait-for-event)
- [Inngest AgentKit Human-in-the-Loop](https://agentkit.inngest.com/advanced-patterns/human-in-the-loop)
- [Restate.dev - Building a Modern Durable Execution Engine](https://www.restate.dev/blog/building-a-modern-durable-execution-engine-from-first-principles)
- [Restate Rust SDK](https://github.com/restatedev/sdk-rust)
- [Restate Workflows Documentation](https://docs.restate.dev/use-cases/workflows)
- [Rise of Durable Execution Engines (Temporal, Restate)](https://www.kai-waehner.de/blog/2025/06/05/the-rise-of-the-durable-execution-engine-temporal-restate-in-an-event-driven-architecture-apache-kafka/)
- [rust-fsm Crate](https://crates.io/crates/rust-fsm)
- [finny.rs FSM Framework](https://github.com/hashmismatch/finny.rs)
- [Rust Type State Pattern for FSMs](https://medium.com/@alfred.weirich/generic-finite-state-machines-with-rusts-type-state-pattern-04593bba34a8)
- [Pretty State Machine Patterns in Rust](https://hoverbear.org/blog/rust-state-machine-pattern/)
- [CQRS and Event Sourcing in Rust (cqrs-es)](https://doc.rust-cqrs.org/)
- [Event Sourcing + CQRS Rust Guide (2026)](https://oneuptime.com/blog/post/2026-01-25-event-sourcing-cqrs-rust/view)
- [cqrs-es Crate](https://crates.io/crates/cqrs-es)
- [n8n Workflow Templates](https://n8n.io/workflows/)
- [n8n Marketplace Community Discussion](https://community.n8n.io/t/new-workflow-templates-marketplace/92871)
- [Airflow vs Dagster vs Prefect (2026)](https://bix-tech.com/airflow-vs-dagster-vs-prefect-which-workflow-orchestrator-should-you-choose-in-2026/)
- [Dagster vs Airflow Feature Comparison](https://dagster.io/blog/dagster-airflow)
- [Temporal Workflow Versioning (TypeScript)](https://docs.temporal.io/develop/typescript/versioning)
- [Temporal Worker Versioning Announcement](https://temporal.io/blog/announcing-worker-versioning-public-preview-pin-workflows-to-a-single-code-version)
- [Temporal Human-in-the-Loop Tutorial](https://learn.temporal.io/tutorials/ai/building-durable-ai-applications/human-in-the-loop/)
- [Temporal Human-in-the-Loop AI Agent](https://docs.temporal.io/ai-cookbook/human-in-the-loop-python)
- [Kestra Trigger Types](https://kestra.io/docs/workflow-components/triggers)
- [OpenTelemetry + Distributed Tracing Tools (2026)](https://signoz.io/blog/distributed-tracing-tools/)
- [AI Observability Tools Buyer's Guide (2026)](https://www.braintrust.dev/articles/best-ai-observability-tools-2026)
- [Agent Tracing for Multi-Agent Systems](https://www.getmaxim.ai/articles/agent-tracing-for-debugging-multi-agent-ai-systems/)