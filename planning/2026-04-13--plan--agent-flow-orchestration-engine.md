# Agent Flow Orchestration Engine — Implementation Plan

**Date**: 2026-04-13
**Branch**: `feature/agent-flow-orchestration`
**Worktree**: TBD
**Base**: `main`
**Status**: PLANNING
**Effort**: 2-3 weeks (5 phases)

---

## Context

The Agent Flow Orchestration Engine is the #1 architectural gap blocking autonomous CRM pipeline progression. The data model and background worker exist (80% complete), but the system requires manual review task completion at each stage, preventing true autonomy.

**Research reference**: `research/02-arch--backend.md §13`, `research/10-ai--agent-orchestration.md`

---

## Current State Audit

### What's Implemented (80%)

| Component | Status | Lines | Location |
|-----------|--------|-------|----------|
| Data Model | ✅ Complete | 1,006 | `db/models/agent_flow*.rs` |
| Background Worker | ✅ Complete | 1,411 | `agent_flow_executor.rs` |
| State Machine (8 statuses) | ✅ Complete | 70 | `FlowStatus` enum |
| LLM Integration + retry | ✅ Complete | 400+ | `call_llm_with_retry()` |
| Stage Transition Triggers | ✅ Complete | 914 | `stage_transition.rs` |
| Event System | ✅ Complete | 350 | `agent_flow_event.rs` |

### What's Missing (Critical Gaps)

| Component | Status | Impact |
|-----------|--------|--------|
| Review Task Bypass | ❌ Missing | **Blocks autonomy** — review tasks gate all stage advances |
| Phase Progression | ❌ Skipped | "v1 execute-only" — no Planner/Verifier phases |
| Structured Response Envelope | ❌ Missing | Raw LLM output, no parsing |
| 5-Level Error Handling | ❌ Missing | Only retry→fail exists |
| TaskScheduler | ⚠️ Dead code | 310 lines, never called from `main.rs` |

### Key Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `ENABLE_AGENT_FLOW_ENGINE` | `0` | Activate background worker |
| `AGENT_FLOW_POLL_INTERVAL` | `15` | Seconds between ticks |
| `AGENT_FLOW_MAX_CONCURRENT` | `5` | Max flows per tick |
| `SIMULATE_LLM` | `0` | Use fake LLM responses |

---

## Architecture Overview

### FlowStatus State Machine

```
Planning ──────────────────────────────────────────┐
    │                                              │
    ├── Executing ──────────────────────┐          │
    │       │                           │          │
    │       ├── Verifying ──────────┐   │          │
    │       │       │               │   │          │
    │       │       ├── Completed ◄─┘   │          │
    │       │       │                   │          │
    │       │       └── Failed ◄────────┤          │
    │       │                           │          │
    │       ├── Paused ─────────────────┤          │
    │       │                           │          │
    │       └── NeedsClarification ─────┼──────────┤
    │                                   │          │
    └── AwaitingApproval ───────────────┴──────────┘
```

### Three-Phase Agent Model (Target State)

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   PLANNER   │────▶│  EXECUTOR   │────▶│  VERIFIER   │
│  Generate   │     │  Execute    │     │  Validate   │
│  task plan  │     │  each step  │     │  quality    │
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       │                   │                   │
       ▼                   ▼                   ▼
  agent_task_plans    flow_config.output   verification_score
```

### Background Worker Flow

```
[15s interval] ──▶ find_pending_flows(limit=5)
                          │
                          ▼
              ┌───────────────────────┐
              │ For each flow:        │
              │  - Planning → execute │
              │  - Executing → LLM    │
              │  - Verifying → score  │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ On completion:        │
              │  - Chain next agent   │
              │  - OR auto-advance    │
              └───────────────────────┘
```

---

## Phase 1: Enable Basic Autonomy (2-3 days)

### Goal
Allow agent flows to complete without manual review blocking when configured.

### Work Items

#### W1.1: Add `auto_complete_review` to StageConfig

**File**: `crates/server/src/stage_transition.rs`
**Lines**: 29-55

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StageConfig {
    pub assigned_agent: Option<String>,
    pub auto_trigger: bool,
    pub cancel_window_secs: u32,
    pub required_fields: Vec<String>,
    pub approval_gate: bool,
    pub on_enter_actions: Vec<StageAction>,
    pub on_exit_validations: Vec<StageValidation>,
    pub review_assignee: Option<String>,
    pub auto_skip: bool,

    // NEW: Bypass review task gate when agent completes successfully
    #[serde(default)]
    pub auto_complete_review: bool,
}
```

**No migration needed** — field lives in JSON `stage_config` column.

#### W1.2: Auto-complete review tasks on flow success

**File**: `crates/server/src/agent_flow_executor.rs`
**Lines**: 708-778 (`complete_flow` function)

Add after review task promotion (line 777):

```rust
// Auto-complete review tasks if stage config allows
if let Some(deal_id) = flow.crm_deal_id.as_deref() {
    if let Some(stage_config) = self.load_stage_config_for_deal(deal_id).await {
        if stage_config.auto_complete_review {
            tracing::info!(
                "[AgentFlowEngine] Auto-completing review tasks for deal {}",
                deal_id
            );
            sqlx::query(
                "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec')
                 WHERE crm_deal_id = ?1
                 AND status IN ('waiting', 'todo')
                 AND deleted_at IS NULL
                 AND title LIKE '%review%'"
            )
            .bind(deal_id)
            .execute(&self.pool)
            .await
            .ok();
        }
    }
}
```

Add helper method:

```rust
async fn load_stage_config_for_deal(&self, deal_id: &str) -> Option<StageConfig> {
    let config_json: Option<String> = sqlx::query_scalar(
        "SELECT s.stage_config FROM crm_deals d
         JOIN crm_pipeline_stages s ON d.crm_stage_id = s.id
         WHERE d.id = ?1"
    )
    .bind(deal_id)
    .fetch_optional(&self.pool)
    .await
    .ok()
    .flatten();

    config_json.and_then(|c| serde_json::from_str(&c).ok())
}
```

#### W1.3: Enable engine by default in dev

**File**: `crates/server/src/main.rs`
**Lines**: 321-327

```rust
// Current:
if std::env::var("ENABLE_AGENT_FLOW_ENGINE").unwrap_or_default() == "1" {

// Change to:
let enable_engine = std::env::var("ENABLE_AGENT_FLOW_ENGINE")
    .unwrap_or_else(|_| if cfg!(debug_assertions) { "1" } else { "0" }.to_string()) == "1";
if enable_engine {
```

#### W1.4: Add SSE endpoint for flow progress

**Create**: `crates/server/src/routes/agent_flow_events.rs` (new file OR extend existing)

Pattern follows `crates/server/src/routes/events.rs`

```rust
/// GET /api/events/agent-flows/:flow_id/stream
/// Real-time SSE stream of flow events
pub async fn flow_progress_stream(
    State(deployment): State<DeploymentImpl>,
    Path(flow_id): Path<String>,
    Extension(access_context): Extension<AccessContext>,
) -> Result<Sse<impl Stream<Item = Result<Event, BoxError>>>, ApiError> {
    let flow_id = parse_db_uuid_param(&flow_id, "flow_id")?;
    let pool = deployment.db().pool.clone();

    let stream = async_stream::try_stream! {
        let mut last_seen_id: Option<String> = None;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            let events = AgentFlowEvent::find_since(&pool, &flow_id, last_seen_id.as_deref()).await?;

            for event in events {
                last_seen_id = Some(event.id.to_string());
                yield Event::default()
                    .event("flow_event")
                    .data(serde_json::to_string(&event)?);
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
```

Register route in `routes/mod.rs`:
```rust
.route("/api/events/agent-flows/:flow_id/stream", get(agent_flow_events::flow_progress_stream))
```

### Testing Strategy

1. Update stage config for Intel stage: `"auto_complete_review": true`
2. Create deal, move to Intel stage
3. Verify Scout agent runs (SIMULATE_LLM=1)
4. Verify review task auto-completes
5. Verify deal auto-advances to next stage
6. Verify SSE endpoint streams events

---

## Phase 2: Structured Response Protocol (2 days)

### Goal
Parse LLM responses into structured envelopes for predictable state handling.

### Work Items

#### W2.1: Define AgentResponse struct

**File**: `crates/db/src/models/agent_flow.rs`
**Location**: After line 224 (UpdateAgentFlow struct)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentResponse {
    pub status: AgentResponseStatus,
    pub message: String,
    pub artifacts: Vec<AgentArtifact>,
    pub clarification: Option<ClarificationRequest>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AgentResponseStatus {
    Success,
    Partial,
    NeedsClarification,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentArtifact {
    pub artifact_type: String,  // "report", "proposal", "deck", etc.
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClarificationRequest {
    pub question: String,
    pub context: String,
    pub options: Option<Vec<String>>,
}
```

#### W2.2: Add `submit_response` tool

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: Line 1387 (`build_agent_tools` function)

```rust
// Add to tool definitions
ToolDefinition {
    name: "submit_response".to_string(),
    description: "Submit your final structured response. Call this when your work is complete.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "enum": ["success", "partial", "needs_clarification", "failed"],
                "description": "Outcome of your work"
            },
            "message": {
                "type": "string",
                "description": "Summary of what you accomplished or why you need clarification"
            },
            "clarification_question": {
                "type": "string",
                "description": "Question for the user (only if status is needs_clarification)"
            },
            "clarification_context": {
                "type": "string",
                "description": "Context for why you need clarification"
            }
        },
        "required": ["status", "message"]
    }),
}
```

#### W2.3: Handle submit_response tool call

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: Line 406-478 (`execute_tool_call` function)

Add case for `submit_response`:

```rust
"submit_response" => {
    let status_str = call.arguments.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("success");

    let response_status = match status_str {
        "success" => AgentResponseStatus::Success,
        "partial" => AgentResponseStatus::Partial,
        "needs_clarification" => AgentResponseStatus::NeedsClarification,
        "failed" => AgentResponseStatus::Failed,
        _ => AgentResponseStatus::Success,
    };

    let message = call.arguments.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Store structured response in flow_config
    let response = AgentResponse {
        status: response_status.clone(),
        message: message.clone(),
        artifacts: vec![],
        clarification: None,
        next_action: None,
    };

    // Update flow_config with structured response
    sqlx::query(
        "UPDATE agent_flows SET
         flow_config = json_set(COALESCE(flow_config, '{}'), '$.structured_response', json(?1)),
         updated_at = datetime('now', 'subsec')
         WHERE id = ?2"
    )
    .bind(serde_json::to_string(&response).unwrap_or_default())
    .bind(flow_id)
    .execute(&self.pool)
    .await
    .ok();

    // Handle needs_clarification status
    if response_status == AgentResponseStatus::NeedsClarification {
        let clarification = ClarificationRequest {
            question: call.arguments.get("clarification_question")
                .and_then(|v| v.as_str())
                .unwrap_or("Please provide more information")
                .to_string(),
            context: call.arguments.get("clarification_context")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            options: None,
        };

        self.request_clarification_internal(flow_id, &clarification).await;
    }

    format!("Response recorded: {}", message)
}
```

#### W2.4: Add clarification request handler

**File**: `crates/server/src/agent_flow_executor.rs`

Add new method:

```rust
async fn request_clarification_internal(&self, flow_id: &str, clarification: &ClarificationRequest) {
    let clarification_json = serde_json::to_string(clarification).unwrap_or_default();

    sqlx::query(
        "UPDATE agent_flows SET
         status = 'needs_clarification',
         clarification_request = ?1,
         updated_at = datetime('now', 'subsec')
         WHERE id = ?2"
    )
    .bind(&clarification_json)
    .bind(flow_id)
    .execute(&self.pool)
    .await
    .ok();

    // Emit event for UI notification
    AgentFlowEvent::create(&self.pool, CreateFlowEvent {
        agent_flow_id: DbUuid::from_string(flow_id.to_string()),
        event_type: FlowEventType::FlowPaused,
        event_data: FlowEventPayload::FlowPaused {
            reason: Some("Awaiting clarification".to_string()),
            paused_by: Some("agent".to_string()),
        },
    })
    .await
    .ok();

    tracing::info!(
        "[AgentFlowEngine] Flow {} needs clarification: {}",
        flow_id, clarification.question
    );
}
```

### Testing Strategy

1. Create agent prompt that intentionally triggers clarification
2. Verify flow transitions to `NeedsClarification` status
3. Call `/api/agent-flows/{id}/respond-clarification` endpoint
4. Verify flow resumes execution
5. Verify structured response stored in `flow_config.structured_response`

---

## Phase 3: Enhanced Error Handling (3-4 days)

### Goal
Implement multi-level error handling beyond simple retry-fail.

### 5-Level Error Hierarchy

| Level | Trigger | Action |
|-------|---------|--------|
| 1 | First failure | Retry with same prompt (existing) |
| 2 | Second failure | Retry with cheaper model (existing) |
| 3 | Third failure | Reassign to fallback agent |
| 4 | 3+ failures in 10 min | Circuit breaker — pause all flows |
| 5 | 3+ deal failures in 24h | Human escalation — create incident task |

### Work Items

#### W3.1: Level 3 — Fallback agent reassignment

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: Lines 264-342 (`call_llm_with_retry`)

After line 341 (all retries exhausted), before returning error:

```rust
// After all retries exhausted, try fallback agent
if let Some(fallback) = self.get_fallback_agent(flow).await {
    tracing::info!(
        "[AgentFlowEngine] Reassigning flow {} to fallback agent: {}",
        flow.id, fallback
    );

    sqlx::query(
        "UPDATE agent_flows SET
         flow_config = json_set(COALESCE(flow_config, '{}'), '$.agent', ?1),
         retry_count = 0,
         last_error = 'Reassigned to fallback after primary failure',
         updated_at = datetime('now', 'subsec')
         WHERE id = ?2"
    )
    .bind(&fallback)
    .bind(&flow.id)
    .execute(&self.pool)
    .await
    .ok();

    // Flow will be picked up on next tick with new agent
    return Err(anyhow::anyhow!("Reassigned to fallback agent, will retry"));
}
```

Add helper:

```rust
async fn get_fallback_agent(&self, flow: &AgentFlow) -> Option<String> {
    // Check flow_config for fallback_agent
    let config = flow.config_json()?;
    if let Some(fallback) = config.get("fallback_agent").and_then(|v| v.as_str()) {
        return Some(fallback.to_string());
    }

    // Default fallbacks
    let current_agent = config.get("agent").and_then(|v| v.as_str())?;
    match current_agent {
        "scout" => Some("astra".to_string()),  // Research fallback to analysis
        "astra" => Some("scout".to_string()),  // Analysis fallback to research
        "cash" => Some("astra".to_string()),   // Proposal fallback to analysis
        "lux" => Some("cash".to_string()),     // Creative fallback to proposal
        _ => None,
    }
}
```

#### W3.2: Level 4 — Circuit breaker

**Create**: `crates/server/src/workers/circuit_breaker.rs`

```rust
use crate::workers::BackgroundWorker;
use async_trait::async_trait;
use sqlx::SqlitePool;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

pub struct CircuitBreakerConfig {
    /// Number of failures to trigger circuit break
    pub failure_threshold: i32,
    /// Time window in minutes to count failures
    pub window_minutes: i32,
    /// Cooldown period in minutes before auto-reset
    pub cooldown_minutes: i32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            window_minutes: 10,
            cooldown_minutes: 30,
        }
    }
}

pub struct CircuitBreakerWorker {
    pool: SqlitePool,
    config: CircuitBreakerConfig,
}

impl CircuitBreakerWorker {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            config: CircuitBreakerConfig::default(),
        }
    }
}

#[async_trait]
impl BackgroundWorker for CircuitBreakerWorker {
    fn name(&self) -> &str {
        "circuit_breaker"
    }

    async fn run(&self, shutdown: CancellationToken) {
        let mut ticker = interval(Duration::from_secs(60));
        ticker.tick().await; // Skip first immediate tick

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.check_circuit_state().await;
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[CircuitBreaker] Shutting down");
                    break;
                }
            }
        }
    }
}

impl CircuitBreakerWorker {
    async fn check_circuit_state(&self) {
        // Count recent failures
        let failures: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_flows
             WHERE status = 'failed'
             AND updated_at > datetime('now', ?1)"
        )
        .bind(format!("-{} minutes", self.config.window_minutes))
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if failures >= self.config.failure_threshold as i64 {
            tracing::warn!(
                "[CircuitBreaker] Threshold exceeded ({} failures in {} minutes), opening circuit",
                failures, self.config.window_minutes
            );
            self.open_circuit().await;
        }
    }

    async fn open_circuit(&self) {
        // Pause all pending flows
        let paused_count = sqlx::query(
            "UPDATE agent_flows SET
             status = 'paused',
             last_error = 'Circuit breaker triggered — multiple agent failures detected',
             updated_at = datetime('now', 'subsec')
             WHERE status IN ('planning', 'executing')"
        )
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);

        tracing::info!(
            "[CircuitBreaker] Paused {} pending flows",
            paused_count
        );

        // Create incident task for ops
        self.create_incident_task().await;
    }

    async fn create_incident_task(&self) {
        let task_id = db::db_uuid::DbUuid::new();

        sqlx::query(
            "INSERT INTO tasks (id, title, description, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'todo', 'critical', datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(&task_id)
        .bind("[INCIDENT] Agent Circuit Breaker Triggered")
        .bind(format!(
            "The agent flow circuit breaker has triggered due to {} failures in {} minutes.\n\n\
             All pending agent flows have been paused.\n\n\
             **Action required:**\n\
             1. Review failed flows in agent_flows table\n\
             2. Check LLM API status and quotas\n\
             3. Resume flows once issue is resolved\n\n\
             Auto-generated at: {}",
            self.config.failure_threshold,
            self.config.window_minutes,
            chrono::Utc::now().to_rfc3339()
        ))
        .execute(&self.pool)
        .await
        .ok();

        tracing::info!("[CircuitBreaker] Created incident task {}", task_id);
    }
}
```

**Register in main.rs** (after line 362):

```rust
// Register circuit breaker worker
registry.spawn_worker(workers::circuit_breaker::CircuitBreakerWorker::new(
    deployment.db().pool.clone()
)).await;
```

**Add to workers/mod.rs**:

```rust
pub mod circuit_breaker;
```

#### W3.3: Level 5 — Human escalation

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: In `fail_flow` function (lines 780-813)

Add escalation check:

```rust
async fn fail_flow(&self, flow: &AgentFlow, error: &str) {
    // Existing fail logic...

    // Check if escalation needed (3+ failures for same deal in 24h)
    if let Some(deal_id) = flow.crm_deal_id.as_deref() {
        let deal_failure_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_flows
             WHERE crm_deal_id = ?1
             AND status = 'failed'
             AND updated_at > datetime('now', '-24 hours')"
        )
        .bind(deal_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if deal_failure_count >= 3 {
            self.create_escalation_task(flow, error).await;
        }
    }
}

async fn create_escalation_task(&self, flow: &AgentFlow, error: &str) {
    let task_id = DbUuid::new();
    let deal_id = flow.crm_deal_id.as_deref().unwrap_or("unknown");

    // Get deal name for context
    let deal_name: String = sqlx::query_scalar(
        "SELECT name FROM crm_deals WHERE id = ?1"
    )
    .bind(deal_id)
    .fetch_optional(&self.pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "Unknown Deal".to_string());

    sqlx::query(
        "INSERT INTO tasks (id, title, description, status, crm_deal_id, priority, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'todo', ?4, 'critical', datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(&task_id)
    .bind(format!("[ESCALATION] Agent failures on: {}", deal_name))
    .bind(format!(
        "Multiple agent failures have occurred for this deal.\n\n\
         **Deal:** {} ({})\n\
         **Flow ID:** {}\n\
         **Last Error:** {}\n\n\
         **Suggested Actions:**\n\
         1. Review deal data for issues\n\
         2. Check linked data sources\n\
         3. Consider manual progression\n\n\
         Auto-escalated at: {}",
        deal_name, deal_id, flow.id, error, chrono::Utc::now().to_rfc3339()
    ))
    .bind(deal_id)
    .execute(&self.pool)
    .await
    .ok();

    tracing::warn!(
        "[AgentFlowEngine] Created escalation task {} for deal {}",
        task_id, deal_id
    );
}
```

### Testing Strategy

1. **Level 3**: Configure fallback agent, fail primary 3x, verify reassignment
2. **Level 4**: Trigger 3+ rapid failures, verify circuit opens, verify incident task
3. **Level 5**: Fail 3+ flows for same deal, verify escalation task created

---

## Phase 4: Three-Phase Orchestration (1 week)

### Goal
Implement full Planner → Executor → Verifier pipeline.

### Work Items

#### W4.1: Planner phase implementation

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: Lines 97-148 (`handle_planning_flow`)

Replace immediate execution with actual planning:

```rust
async fn handle_planning_flow(&self, flow: &AgentFlow) {
    tracing::info!(
        "[AgentFlowEngine] Starting PLANNING phase for flow {} (deal: {:?})",
        flow.id, flow.crm_deal_id
    );

    // Emit planning started event
    AgentFlowEvent::create(&self.pool, CreateFlowEvent {
        agent_flow_id: flow.id.clone(),
        event_type: FlowEventType::PhaseStarted,
        event_data: FlowEventPayload::PhaseStarted {
            phase: "planning".to_string(),
            agent_name: self.get_agent_name(flow),
        },
    })
    .await
    .ok();

    // Build planning context
    let deal_context = self.load_deal_context(&flow.id.to_string(), flow.crm_deal_id.as_deref()).await;
    let agent_name = self.get_agent_name(flow);

    let planning_prompt = format!(
        "You are a planning agent. Your job is to create a detailed execution plan.\n\n\
         Agent: {}\n\
         Deal Context: {}\n\n\
         Create a step-by-step plan for completing this work. Be specific about:\n\
         1. What information you need to gather\n\
         2. What tools you will use\n\
         3. What output you will produce\n\
         4. How long each step should take",
        agent_name,
        serde_json::to_string_pretty(&deal_context).unwrap_or_default()
    );

    let messages = vec![
        WorkflowLLMService::system_message(&planning_prompt),
        WorkflowLLMService::user_message("Create your execution plan."),
    ];

    let planning_tools = vec![
        ToolDefinition {
            name: "submit_plan".to_string(),
            description: "Submit your execution plan".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "steps": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "step_number": { "type": "integer" },
                                "description": { "type": "string" },
                                "tool_to_use": { "type": "string" },
                                "expected_output": { "type": "string" },
                                "estimated_minutes": { "type": "integer" }
                            },
                            "required": ["step_number", "description"]
                        }
                    },
                    "total_estimated_minutes": { "type": "integer" },
                    "risks": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["steps"]
            }),
        }
    ];

    match self.call_llm_once(messages, &planning_tools, None, "", &flow.id.to_string()).await {
        Ok(plan_json) => {
            // Store plan in flow_config
            sqlx::query(
                "UPDATE agent_flows SET
                 flow_config = json_set(COALESCE(flow_config, '{}'), '$.plan', json(?1)),
                 planning_completed_at = datetime('now', 'subsec'),
                 updated_at = datetime('now', 'subsec')
                 WHERE id = ?2"
            )
            .bind(&plan_json)
            .bind(&flow.id)
            .execute(&self.pool)
            .await
            .ok();

            // Transition to executing
            if let Err(e) = AgentFlow::transition_to_phase(
                &self.pool,
                &flow.id,
                AgentPhase::Execution,
                Some(FlowStatus::Planning)
            ).await {
                tracing::error!("[AgentFlowEngine] Failed to transition to executing: {}", e);
            }
        }
        Err(e) => {
            self.fail_flow(flow, &format!("Planning failed: {}", e)).await;
        }
    }
}
```

#### W4.2: Add verifying status handling in tick()

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: Lines 66-94 (`tick`)

```rust
for flow in flows {
    match flow.status {
        FlowStatus::Planning => self.handle_planning_flow(&flow).await,
        FlowStatus::Executing => self.handle_executing_flow(&flow).await,
        FlowStatus::Verifying => self.handle_verifying_flow(&flow).await,  // NEW
        _ => {}
    }
}
```

#### W4.3: Verifier phase implementation

**File**: `crates/server/src/agent_flow_executor.rs`

Add new method:

```rust
async fn handle_verifying_flow(&self, flow: &AgentFlow) {
    tracing::info!(
        "[AgentFlowEngine] Starting VERIFICATION phase for flow {}",
        flow.id
    );

    // Emit verification started event
    AgentFlowEvent::create(&self.pool, CreateFlowEvent {
        agent_flow_id: flow.id.clone(),
        event_type: FlowEventType::PhaseStarted,
        event_data: FlowEventPayload::PhaseStarted {
            phase: "verification".to_string(),
            agent_name: Some("verifier".to_string()),
        },
    })
    .await
    .ok();

    // Get the output to verify
    let flow_config = self.parse_flow_config(flow);
    let output = flow_config
        .get("output")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let plan = flow_config
        .get("plan")
        .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
        .unwrap_or_default();

    let verification_prompt = format!(
        "You are a quality verification agent. Review this agent output and score it.\n\n\
         ORIGINAL PLAN:\n{}\n\n\
         AGENT OUTPUT:\n{}\n\n\
         Evaluate:\n\
         1. Completeness: Did the agent complete all planned steps?\n\
         2. Quality: Is the output well-structured and useful?\n\
         3. Accuracy: Does the output appear factually correct?\n\
         4. Relevance: Is the output relevant to the deal context?\n\n\
         Provide a score from 0.0 to 1.0 where:\n\
         - 0.8-1.0: Excellent, ready to proceed\n\
         - 0.6-0.8: Good, minor improvements possible\n\
         - 0.4-0.6: Fair, consider retry with feedback\n\
         - 0.0-0.4: Poor, needs significant rework",
        plan, output
    );

    let messages = vec![
        WorkflowLLMService::system_message(&verification_prompt),
        WorkflowLLMService::user_message("Verify this output and provide your assessment."),
    ];

    let verify_tools = vec![
        ToolDefinition {
            name: "submit_verification".to_string(),
            description: "Submit your verification result".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "score": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 1,
                        "description": "Quality score from 0.0 to 1.0"
                    },
                    "passed": {
                        "type": "boolean",
                        "description": "Whether the output passes quality threshold"
                    },
                    "feedback": {
                        "type": "string",
                        "description": "Specific feedback for improvement"
                    },
                    "retry_recommended": {
                        "type": "boolean",
                        "description": "Whether to retry execution with feedback"
                    }
                },
                "required": ["score", "passed"]
            }),
        }
    ];

    // Use cheaper/faster model for verification
    match self.call_llm_once(
        messages,
        &verify_tools,
        Some("claude-haiku-4-5"),
        "",
        &flow.id.to_string()
    ).await {
        Ok(result) => {
            let verification: Value = serde_json::from_str(&result).unwrap_or_default();
            let score = verification.get("score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5);
            let passed = verification.get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let retry = verification.get("retry_recommended")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let feedback = verification.get("feedback")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Store verification result
            sqlx::query(
                "UPDATE agent_flows SET
                 verification_score = ?1,
                 flow_config = json_set(COALESCE(flow_config, '{}'), '$.verification', json(?2)),
                 verification_completed_at = datetime('now', 'subsec'),
                 updated_at = datetime('now', 'subsec')
                 WHERE id = ?3"
            )
            .bind(score)
            .bind(&result)
            .bind(&flow.id)
            .execute(&self.pool)
            .await
            .ok();

            if passed {
                self.complete_flow_with_score(flow, score).await;
            } else if retry && flow.retry_count < 2 {
                // Re-execute with feedback
                self.retry_execution_with_feedback(flow, feedback).await;
            } else {
                self.fail_flow(flow, &format!("Verification failed (score: {:.2}): {}", score, feedback)).await;
            }
        }
        Err(e) => {
            // Verification failure shouldn't block completion — proceed with warning
            tracing::warn!(
                "[AgentFlowEngine] Verification failed for flow {}: {}, proceeding anyway",
                flow.id, e
            );
            self.complete_flow(flow).await;
        }
    }
}

async fn complete_flow_with_score(&self, flow: &AgentFlow, score: f64) {
    tracing::info!(
        "[AgentFlowEngine] Flow {} completed with verification score: {:.2}",
        flow.id, score
    );

    // Complete the flow with verification score already stored
    self.complete_flow(flow).await;
}

async fn retry_execution_with_feedback(&self, flow: &AgentFlow, feedback: &str) {
    tracing::info!(
        "[AgentFlowEngine] Retrying flow {} with feedback: {}",
        flow.id, feedback
    );

    // Store feedback and reset to executing
    sqlx::query(
        "UPDATE agent_flows SET
         status = 'executing',
         flow_config = json_set(COALESCE(flow_config, '{}'), '$.retry_feedback', ?1),
         retry_count = retry_count + 1,
         updated_at = datetime('now', 'subsec')
         WHERE id = ?2"
    )
    .bind(feedback)
    .bind(&flow.id)
    .execute(&self.pool)
    .await
    .ok();
}
```

#### W4.4: Modify execution completion to transition to verification

**File**: `crates/server/src/agent_flow_executor.rs`
**Location**: Line 234 (end of `execute_flow`)

```rust
// Change from:
// self.complete_flow(flow).await;

// To:
if self.should_verify(flow) {
    tracing::info!("[AgentFlowEngine] Transitioning flow {} to verification", flow.id);
    AgentFlow::transition_to_phase(
        &self.pool,
        &flow.id,
        AgentPhase::Verification,
        Some(FlowStatus::Executing)
    )
    .await
    .ok();
} else {
    self.complete_flow(flow).await;
}
```

Add helper:

```rust
fn should_verify(&self, flow: &AgentFlow) -> bool {
    // Check flow_config for verification settings
    let config = flow.config_json();

    // Default: verify unless explicitly disabled
    config
        .and_then(|c| c.get("skip_verification").and_then(|v| v.as_bool()))
        .map(|skip| !skip)
        .unwrap_or(true)
}
```

### Testing Strategy

1. Enable three-phase flow (default)
2. Create deal, trigger agent
3. Verify planning phase generates plan (stored in `flow_config.plan`)
4. Verify execution phase uses plan
5. Verify verification phase scores output (stored in `verification_score`)
6. Test retry-on-low-score path (verification returns `retry_recommended: true`)
7. Test skip_verification config option

---

## Phase 5: Cleanup (1 day)

### Goal
Remove dead code, improve documentation.

### Work Items

#### W5.1: Delete TaskScheduler

**Delete**: `crates/server/src/task_scheduler.rs` (311 lines)

```bash
rm crates/server/src/task_scheduler.rs
```

**Remove from lib.rs** (if exported):
```rust
// Remove: pub mod task_scheduler;
```

**Verify no references**:
```bash
grep -r "TaskScheduler\|task_scheduler" crates/ --include="*.rs"
```

#### W5.2: Remove duplicate code paths

**File**: `crates/server/src/agent_flow_executor.rs`

Review and remove:
- Duplicate `flow_deal_id` extraction (lines 275-288)
- Unused imports after TaskScheduler removal

#### W5.3: Documentation updates

**Update**: `docs/PIPELINE.md`
- Add agent flow orchestration section
- Document `auto_complete_review` in stage config
- Add three-phase orchestration diagram

**Create**: `docs/AGENT-FLOWS.md`
- Flow lifecycle documentation
- State machine diagram
- Configuration options
- Troubleshooting guide

**Update**: `CLAUDE.md`
- Add environment variables section for agent flow engine

---

## Database Migrations

| Timestamp | Purpose | Risk |
|-----------|---------|------|
| No schema change | `auto_complete_review` in JSON | None |
| No schema change | `AgentResponse` in JSON | None |
| `_add_circuit_breaker_incidents.sql` | Track circuit breaker events | Low |
| `_seed_auto_complete_review.sql` | Update stage configs | Medium |

### Sample seed migration

```sql
-- Update Intel stage to auto-complete reviews
UPDATE crm_pipeline_stages
SET stage_config = json_set(
    COALESCE(stage_config, '{}'),
    '$.auto_complete_review',
    json('true')
)
WHERE name = 'Intel';

-- Update BA stage to auto-complete reviews
UPDATE crm_pipeline_stages
SET stage_config = json_set(
    COALESCE(stage_config, '{}'),
    '$.auto_complete_review',
    json('true')
)
WHERE name = 'BA';
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Three-phase adds latency | Medium | Medium | Make verification opt-in via `skip_verification` |
| Circuit breaker triggers too often | Low | High | Conservative thresholds (3 failures / 10 min) |
| Auto-complete bypasses necessary reviews | Medium | High | Explicit opt-in per stage, audit trail via events |
| LLM structured response parsing fails | Medium | Medium | Fallback to text response, retry logic |
| Verification LLM costs | Low | Low | Use Haiku (cheap) for verification |

---

## Files to Modify Summary

| File | Phase | Changes |
|------|-------|---------|
| `agent_flow_executor.rs` | 1,2,3,4 | Primary target — all phases |
| `stage_transition.rs` | 1 | Add `auto_complete_review` |
| `agent_flow.rs` | 2 | Add `AgentResponse` struct |
| `workers/mod.rs` | 3 | Export circuit_breaker |
| `workers/circuit_breaker.rs` | 3 | **NEW** |
| `routes/agent_flow_events.rs` | 1 | **NEW or extend** |
| `main.rs` | 1,3 | Enable engine, register workers |
| `task_scheduler.rs` | 5 | **DELETE** |

---

## Verification Checklist

### Phase 1
- [ ] Stage config accepts `auto_complete_review`
- [ ] Review tasks auto-complete when agent succeeds
- [ ] Engine enabled by default in dev builds
- [ ] SSE endpoint streams flow events
- [ ] Deals auto-advance after agent completion

### Phase 2
- [ ] `submit_response` tool available to agents
- [ ] Structured response stored in flow_config
- [ ] Clarification requests transition flow to `NeedsClarification`
- [ ] `/respond-clarification` endpoint resumes flow

### Phase 3
- [ ] Fallback agent assigned after 3 failures
- [ ] Circuit breaker pauses flows on threshold
- [ ] Incident task created on circuit break
- [ ] Escalation task created on 3+ deal failures

### Phase 4
- [ ] Planning phase generates task breakdown
- [ ] Execution phase follows plan
- [ ] Verification phase scores output
- [ ] Low-score triggers retry with feedback
- [ ] `skip_verification` config option works

### Phase 5
- [ ] TaskScheduler deleted
- [ ] No remaining references
- [ ] Documentation updated

---

## Definition of Done

1. All verification checklist items pass
2. `/check` passes (clippy, tsc, eslint, fmt)
3. E2E test: deal flows through pipeline without manual intervention
4. CI passes
5. Documentation updated
