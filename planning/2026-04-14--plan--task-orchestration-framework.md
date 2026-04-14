# Task Orchestration Framework — Detailed Implementation Plan

**Date**: 2026-04-14
**Branch**: `feat/task-orchestration`
**Worktree**: `/Users/madhav/development/GitHub/pcg-cc-mcp`
**Base**: `main`

---

## Overview

Replicate demo_impl orchestration patterns into PCG-CC-MCP:
- **All 7 task types** (local_bash, local_agent, remote_agent, in_process_teammate, local_workflow, monitor_mcp, dream)
- **Extend AgentFlowExecutor** (not replace)
- **Max 5 concurrent tasks** per flow (conservative)

---

## Step-by-Step Implementation

Each step is small enough to fit in context and includes its own test.

---

### Step 1: OrchestrationTaskType Enum

**Time**: 15 min
**File**: `crates/db/src/models/orchestration_task.rs` (new)

```rust
use serde::{Deserialize, Serialize};
use sqlx::Type;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize, TS)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrchestrationTaskType {
    LocalBash,
    LocalAgent,
    RemoteAgent,
    InProcessTeammate,
    LocalWorkflow,
    MonitorMcp,
    Dream,
}
```

**Test**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_serialization() {
        assert_eq!(serde_json::to_string(&OrchestrationTaskType::LocalBash).unwrap(), "\"local_bash\"");
        assert_eq!(serde_json::to_string(&OrchestrationTaskType::InProcessTeammate).unwrap(), "\"in_process_teammate\"");
    }

    #[test]
    fn test_task_type_deserialization() {
        let parsed: OrchestrationTaskType = serde_json::from_str("\"remote_agent\"").unwrap();
        assert_eq!(parsed, OrchestrationTaskType::RemoteAgent);
    }
}
```

---

### Step 2: OrchestrationTaskStatus Enum

**Time**: 15 min
**File**: `crates/db/src/models/orchestration_task.rs` (append)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize, TS)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrchestrationTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Killed,
}

impl OrchestrationTaskStatus {
    /// True when task is in terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Killed)
    }
}
```

**Test**:
```rust
#[test]
fn test_status_is_terminal() {
    assert!(!OrchestrationTaskStatus::Pending.is_terminal());
    assert!(!OrchestrationTaskStatus::Running.is_terminal());
    assert!(OrchestrationTaskStatus::Completed.is_terminal());
    assert!(OrchestrationTaskStatus::Failed.is_terminal());
    assert!(OrchestrationTaskStatus::Killed.is_terminal());
}
```

---

### Step 3: Database Migration

**Time**: 20 min
**File**: `crates/db/migrations/YYYYMMDDHHMMSS_add_orchestration_tasks.sql`

```sql
-- Orchestration tasks: child entities of agent_flows for parallel execution
CREATE TABLE orchestration_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    agent_flow_id TEXT NOT NULL,
    task_type TEXT NOT NULL CHECK (task_type IN (
        'local_bash', 'local_agent', 'remote_agent',
        'in_process_teammate', 'local_workflow', 'monitor_mcp', 'dream'
    )),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'running', 'completed', 'failed', 'killed'
    )),
    description TEXT NOT NULL,
    tool_use_id TEXT,
    start_time_ms INTEGER,
    end_time_ms INTEGER,
    output TEXT,
    error TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    is_concurrency_safe INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    FOREIGN KEY (agent_flow_id) REFERENCES agent_flows(id) ON DELETE CASCADE
);

CREATE INDEX idx_orchestration_tasks_flow ON orchestration_tasks(agent_flow_id);
CREATE INDEX idx_orchestration_tasks_status ON orchestration_tasks(status);
CREATE INDEX idx_orchestration_tasks_flow_status ON orchestration_tasks(agent_flow_id, status);
```

**Test**: Run migration and verify table exists:
```bash
sqlx migrate run
sqlite3 dev_assets/db.sqlite ".schema orchestration_tasks"
```

---

### Step 4: OrchestrationTask Struct

**Time**: 20 min
**File**: `crates/db/src/models/orchestration_task.rs` (append)

```rust
use crate::db_uuid::DbUuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrchestrationTask {
    pub id: String,
    pub agent_flow_id: String,
    pub task_type: OrchestrationTaskType,
    pub status: OrchestrationTaskStatus,
    pub description: String,
    pub tool_use_id: Option<String>,
    pub start_time_ms: Option<i64>,
    pub end_time_ms: Option<i64>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub position: i32,
    pub is_concurrency_safe: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

**Test**:
```rust
#[test]
fn test_orchestration_task_default_values() {
    let task = OrchestrationTask {
        id: "test-id".to_string(),
        agent_flow_id: "flow-id".to_string(),
        task_type: OrchestrationTaskType::LocalBash,
        status: OrchestrationTaskStatus::Pending,
        description: "Run tests".to_string(),
        tool_use_id: None,
        start_time_ms: None,
        end_time_ms: None,
        output: None,
        error: None,
        position: 0,
        is_concurrency_safe: true,
        created_at: "2026-04-14".to_string(),
        updated_at: "2026-04-14".to_string(),
    };
    assert_eq!(task.task_type, OrchestrationTaskType::LocalBash);
    assert!(!task.status.is_terminal());
}
```

---

### Step 5: OrchestrationTask::create()

**Time**: 25 min
**File**: `crates/db/src/models/orchestration_task.rs` (append impl block)

```rust
impl OrchestrationTask {
    pub async fn create(
        pool: &SqlitePool,
        agent_flow_id: &str,
        task_type: OrchestrationTaskType,
        description: &str,
        is_concurrency_safe: bool,
        position: i32,
    ) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new().to_string();
        let task_type_str = serde_json::to_string(&task_type)
            .unwrap()
            .trim_matches('"')
            .to_string();

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO orchestration_tasks
                (id, agent_flow_id, task_type, description, is_concurrency_safe, position)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(agent_flow_id)
        .bind(&task_type_str)
        .bind(description)
        .bind(is_concurrency_safe)
        .bind(position)
        .fetch_one(pool)
        .await
    }
}
```

**Test** (integration):
```rust
#[sqlx::test]
async fn test_create_orchestration_task(pool: SqlitePool) {
    // Setup: create agent_flow first (or use test fixture)
    let task = OrchestrationTask::create(
        &pool,
        "test-flow-id",
        OrchestrationTaskType::LocalBash,
        "echo hello",
        true,
        0,
    ).await.unwrap();

    assert_eq!(task.task_type, OrchestrationTaskType::LocalBash);
    assert_eq!(task.status, OrchestrationTaskStatus::Pending);
    assert!(task.is_concurrency_safe);
}
```

---

### Step 6: OrchestrationTask::find_by_flow_id()

**Time**: 15 min
**File**: `crates/db/src/models/orchestration_task.rs` (append to impl)

```rust
impl OrchestrationTask {
    // ... create() ...

    pub async fn find_by_flow_id(
        pool: &SqlitePool,
        agent_flow_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM orchestration_tasks
            WHERE agent_flow_id = ?1
            ORDER BY position ASC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_pending_by_flow_id(
        pool: &SqlitePool,
        agent_flow_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM orchestration_tasks
            WHERE agent_flow_id = ?1 AND status = 'pending'
            ORDER BY position ASC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await
    }
}
```

**Test**:
```rust
#[sqlx::test]
async fn test_find_by_flow_id(pool: SqlitePool) {
    let flow_id = "test-flow";

    // Create 3 tasks
    for i in 0..3 {
        OrchestrationTask::create(&pool, flow_id, OrchestrationTaskType::LocalBash, &format!("task {}", i), true, i).await.unwrap();
    }

    let tasks = OrchestrationTask::find_by_flow_id(&pool, flow_id).await.unwrap();
    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].position, 0);
    assert_eq!(tasks[2].position, 2);
}
```

---

### Step 7: OrchestrationTask::update_status()

**Time**: 15 min
**File**: `crates/db/src/models/orchestration_task.rs` (append to impl)

```rust
impl OrchestrationTask {
    // ... previous methods ...

    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: OrchestrationTaskStatus,
    ) -> Result<Self, sqlx::Error> {
        let status_str = serde_json::to_string(&status)
            .unwrap()
            .trim_matches('"')
            .to_string();

        let now_ms = if status == OrchestrationTaskStatus::Running {
            Some(chrono::Utc::now().timestamp_millis())
        } else {
            None
        };

        let end_ms = if status.is_terminal() {
            Some(chrono::Utc::now().timestamp_millis())
        } else {
            None
        };

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET status = ?2,
                start_time_ms = COALESCE(?3, start_time_ms),
                end_time_ms = COALESCE(?4, end_time_ms),
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&status_str)
        .bind(now_ms)
        .bind(end_ms)
        .fetch_one(pool)
        .await
    }
}
```

**Test**:
```rust
#[sqlx::test]
async fn test_update_status_sets_timestamps(pool: SqlitePool) {
    let task = OrchestrationTask::create(&pool, "flow", OrchestrationTaskType::LocalBash, "test", true, 0).await.unwrap();
    assert!(task.start_time_ms.is_none());

    let running = OrchestrationTask::update_status(&pool, &task.id, OrchestrationTaskStatus::Running).await.unwrap();
    assert!(running.start_time_ms.is_some());
    assert!(running.end_time_ms.is_none());

    let completed = OrchestrationTask::update_status(&pool, &task.id, OrchestrationTaskStatus::Completed).await.unwrap();
    assert!(completed.end_time_ms.is_some());
}
```

---

### Step 8: OrchestrationTask::set_output() and set_error()

**Time**: 15 min
**File**: `crates/db/src/models/orchestration_task.rs` (append to impl)

```rust
impl OrchestrationTask {
    // ... previous methods ...

    pub async fn set_output(
        pool: &SqlitePool,
        id: &str,
        output: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET output = ?2, updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(output)
        .fetch_one(pool)
        .await
    }

    pub async fn set_error(
        pool: &SqlitePool,
        id: &str,
        error: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE orchestration_tasks
            SET error = ?2,
                status = 'failed',
                end_time_ms = ?3,
                updated_at = datetime('now','subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(error)
        .bind(chrono::Utc::now().timestamp_millis())
        .fetch_one(pool)
        .await
    }
}
```

**Test**:
```rust
#[sqlx::test]
async fn test_set_output_and_error(pool: SqlitePool) {
    let task = OrchestrationTask::create(&pool, "flow", OrchestrationTaskType::LocalBash, "test", true, 0).await.unwrap();

    let with_output = OrchestrationTask::set_output(&pool, &task.id, "hello world").await.unwrap();
    assert_eq!(with_output.output, Some("hello world".to_string()));

    let task2 = OrchestrationTask::create(&pool, "flow", OrchestrationTaskType::LocalBash, "fail", true, 1).await.unwrap();
    let with_error = OrchestrationTask::set_error(&pool, &task2.id, "command failed").await.unwrap();
    assert_eq!(with_error.status, OrchestrationTaskStatus::Failed);
    assert_eq!(with_error.error, Some("command failed".to_string()));
}
```

---

### Step 9: Register Model in db/mod.rs

**Time**: 5 min
**File**: `crates/db/src/models/mod.rs`

```rust
pub mod orchestration_task;
pub use orchestration_task::*;
```

**Test**: Compile check
```bash
SQLX_OFFLINE=true cargo check -p db
```

---

### Step 10: ToolBatch Struct and partition_tool_calls()

**Time**: 30 min
**File**: `crates/server/src/tool_partitioner.rs` (new)

```rust
//! Tool Call Partitioner
//!
//! Partitions tool calls into batches based on concurrency safety.
//! Mirrors demo_impl/src/services/tools/toolOrchestration.ts

use serde_json::Value;
use std::collections::HashMap;

/// A batch of tool calls that can be executed together
#[derive(Debug, Clone)]
pub struct ToolBatch {
    /// If true, all calls in this batch can run concurrently
    pub is_concurrency_safe: bool,
    /// Tool calls in this batch
    pub calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Metadata about a tool's concurrency characteristics
#[derive(Debug, Clone, Default)]
pub struct ToolMetadata {
    pub is_concurrency_safe: bool,
    pub is_read_only: bool,
}

/// Partition tool calls into batches:
/// - Consecutive safe calls grouped together
/// - Each unsafe call in its own batch
pub fn partition_tool_calls(
    calls: Vec<ToolCall>,
    tool_metadata: &HashMap<String, ToolMetadata>,
) -> Vec<ToolBatch> {
    calls.into_iter().fold(Vec::new(), |mut acc, call| {
        let is_safe = tool_metadata
            .get(&call.name)
            .map(|m| m.is_concurrency_safe)
            .unwrap_or(false); // Conservative: unknown tools are unsafe

        match acc.last_mut() {
            Some(batch) if batch.is_concurrency_safe && is_safe => {
                // Append to existing safe batch
                batch.calls.push(call);
            }
            _ => {
                // Create new batch
                acc.push(ToolBatch {
                    is_concurrency_safe: is_safe,
                    calls: vec![call],
                });
            }
        }
        acc
    })
}
```

**Test**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_call(name: &str) -> ToolCall {
        ToolCall {
            id: format!("id-{}", name),
            name: name.to_string(),
            input: serde_json::json!({}),
        }
    }

    #[test]
    fn test_partition_all_safe() {
        let mut metadata = HashMap::new();
        metadata.insert("read".to_string(), ToolMetadata { is_concurrency_safe: true, is_read_only: true });
        metadata.insert("glob".to_string(), ToolMetadata { is_concurrency_safe: true, is_read_only: true });

        let calls = vec![make_call("read"), make_call("glob"), make_call("read")];
        let batches = partition_tool_calls(calls, &metadata);

        assert_eq!(batches.len(), 1);
        assert!(batches[0].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 3);
    }

    #[test]
    fn test_partition_mixed() {
        let mut metadata = HashMap::new();
        metadata.insert("read".to_string(), ToolMetadata { is_concurrency_safe: true, is_read_only: true });
        metadata.insert("write".to_string(), ToolMetadata { is_concurrency_safe: false, is_read_only: false });

        let calls = vec![
            make_call("read"),
            make_call("read"),
            make_call("write"),  // unsafe - breaks batch
            make_call("read"),
            make_call("read"),
        ];
        let batches = partition_tool_calls(calls, &metadata);

        assert_eq!(batches.len(), 3);
        assert!(batches[0].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 2);
        assert!(!batches[1].is_concurrency_safe);
        assert_eq!(batches[1].calls.len(), 1);
        assert!(batches[2].is_concurrency_safe);
        assert_eq!(batches[2].calls.len(), 2);
    }

    #[test]
    fn test_partition_unknown_tools_are_unsafe() {
        let metadata = HashMap::new(); // No metadata

        let calls = vec![make_call("unknown")];
        let batches = partition_tool_calls(calls, &metadata);

        assert_eq!(batches.len(), 1);
        assert!(!batches[0].is_concurrency_safe);
    }
}
```

---

### Step 11: Register tool_partitioner in lib.rs

**Time**: 5 min
**File**: `crates/server/src/lib.rs`

```rust
pub mod tool_partitioner;
```

**Test**: Compile check
```bash
SQLX_OFFLINE=true cargo check -p server
```

---

### Step 12: Default Tool Metadata Registry

**Time**: 20 min
**File**: `crates/server/src/tool_partitioner.rs` (append)

```rust
/// Build default tool metadata for PCG-CC-MCP's built-in tools
pub fn default_tool_metadata() -> HashMap<String, ToolMetadata> {
    let mut m = HashMap::new();

    // Read-only, concurrency-safe tools
    for name in ["get_deal_context", "read_file", "glob", "grep", "list_tasks"] {
        m.insert(name.to_string(), ToolMetadata {
            is_concurrency_safe: true,
            is_read_only: true,
        });
    }

    // Stateful but concurrency-safe (each operates on different resource)
    for name in ["save_artifact"] {
        m.insert(name.to_string(), ToolMetadata {
            is_concurrency_safe: true,
            is_read_only: false,
        });
    }

    // Unsafe tools (must run serially)
    for name in ["update_deal_field", "write_file", "bash", "submit_response"] {
        m.insert(name.to_string(), ToolMetadata {
            is_concurrency_safe: false,
            is_read_only: false,
        });
    }

    m
}
```

**Test**:
```rust
#[test]
fn test_default_metadata_has_expected_tools() {
    let metadata = default_tool_metadata();

    assert!(metadata.get("get_deal_context").unwrap().is_concurrency_safe);
    assert!(!metadata.get("update_deal_field").unwrap().is_concurrency_safe);
    assert!(metadata.get("glob").unwrap().is_read_only);
}
```

---

### Step 13: OrchestrationEvent Enum

**Time**: 15 min
**File**: `crates/server/src/orchestration_events.rs` (new)

```rust
//! Orchestration Events for SSE streaming

use db::OrchestrationTaskType;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum OrchestrationEvent {
    TaskQueued {
        task_id: String,
        task_type: OrchestrationTaskType,
        description: String,
        position: i32,
    },
    TaskStarted {
        task_id: String,
        task_type: OrchestrationTaskType,
    },
    TaskProgress {
        task_id: String,
        progress: String,
    },
    TaskCompleted {
        task_id: String,
        output: String,
        duration_ms: i64,
    },
    TaskFailed {
        task_id: String,
        error: String,
    },
    TaskKilled {
        task_id: String,
    },
    FlowCompleted {
        flow_id: String,
        total_duration_ms: i64,
    },
}
```

**Test**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = OrchestrationEvent::TaskStarted {
            task_id: "task-1".to_string(),
            task_type: OrchestrationTaskType::LocalBash,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"task_started\""));
        assert!(json.contains("\"task_type\":\"local_bash\""));
    }
}
```

---

### Step 14: Register orchestration_events in lib.rs

**Time**: 5 min
**File**: `crates/server/src/lib.rs`

```rust
pub mod orchestration_events;
```

---

### Step 15: execute_tool_calls_batched() in AgentFlowExecutor

**Time**: 40 min
**File**: `crates/server/src/agent_flow_executor.rs` (add method)

```rust
use crate::tool_partitioner::{partition_tool_calls, default_tool_metadata, ToolCall, ToolBatch};

impl AgentFlowExecutor {
    /// Execute tool calls with concurrency partitioning
    /// Safe tools run in parallel (max 5), unsafe tools run serially
    async fn execute_tool_calls_batched(
        &self,
        tool_calls: Vec<ToolCall>,
        flow_id: &str,
    ) -> Vec<(String, String)> {
        let metadata = default_tool_metadata();
        let batches = partition_tool_calls(tool_calls, &metadata);
        let mut results = Vec::new();

        const MAX_CONCURRENT: usize = 5;

        for batch in batches {
            if batch.is_concurrency_safe {
                // Run safe tools concurrently in chunks of MAX_CONCURRENT
                for chunk in batch.calls.chunks(MAX_CONCURRENT) {
                    let futures: Vec<_> = chunk
                        .iter()
                        .map(|call| self.execute_single_tool_call(call, flow_id))
                        .collect();

                    let chunk_results = futures::future::join_all(futures).await;
                    results.extend(chunk_results);
                }
            } else {
                // Run unsafe tools serially
                for call in &batch.calls {
                    let result = self.execute_single_tool_call(call, flow_id).await;
                    results.push(result);
                }
            }
        }

        results
    }

    async fn execute_single_tool_call(
        &self,
        call: &ToolCall,
        flow_id: &str,
    ) -> (String, String) {
        // Execute the tool and return (tool_use_id, result)
        let result = match self.execute_tool(&call.name, &call.input, flow_id).await {
            Ok(output) => output,
            Err(e) => format!("Error: {}", e),
        };
        (call.id.clone(), result)
    }
}
```

**Test**:
```rust
#[tokio::test]
async fn test_execute_tool_calls_batched_ordering() {
    // Mock test verifying:
    // 1. Safe tools in same batch run concurrently
    // 2. Unsafe tools run one at a time
    // 3. Results maintain call order
}
```

---

### Step 16-20: Remaining Steps (Overview)

**Step 16**: OrchestrationCoordinator background worker (extend BackgroundWorker trait)
**Step 17**: SSE endpoint for orchestration events
**Step 18**: REST endpoints for task management (create, list, kill)
**Step 19**: Wire up in main.rs
**Step 20**: Integration test with real flow

---

## Checkpoint Summary

After implementing Steps 1-15, we have:

| Component | Status | Test |
|-----------|--------|------|
| OrchestrationTaskType enum | Done | Unit test |
| OrchestrationTaskStatus enum | Done | Unit test |
| Database migration | Done | Schema check |
| OrchestrationTask model | Done | Unit test |
| CRUD methods | Done | sqlx::test |
| Tool partitioner | Done | Unit test |
| Default tool metadata | Done | Unit test |
| OrchestrationEvent enum | Done | Unit test |
| execute_tool_calls_batched() | Done | Mock test |

---

## Files Created/Modified

### New Files
- `crates/db/src/models/orchestration_task.rs`
- `crates/db/migrations/YYYYMMDDHHMMSS_add_orchestration_tasks.sql`
- `crates/server/src/tool_partitioner.rs`
- `crates/server/src/orchestration_events.rs`

### Modified Files
- `crates/db/src/models/mod.rs` — export orchestration_task
- `crates/server/src/lib.rs` — export tool_partitioner, orchestration_events
- `crates/server/src/agent_flow_executor.rs` — add execute_tool_calls_batched()

---

## Verification Commands

```bash
# After each step, run:
SQLX_OFFLINE=true cargo check -p db -p server

# Run unit tests:
cargo test -p db orchestration_task
cargo test -p server tool_partitioner
cargo test -p server orchestration_events

# Run integration tests (requires test DB):
cargo test -p db --test orchestration_task_integration

# Generate TypeScript types:
npm run generate-types
```
