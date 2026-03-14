use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Row, SqlitePool, Type, types::Json};
use ts_rs::TS;
use uuid::Uuid;

use super::{execution_summary::CompletionStatus, project::Project, task_attempt::TaskAttempt};

// ── Actor type constants (collaborators JSON) ────────────────────────────────
pub const ACTOR_TYPE_AGENT_WATCHER: &str = "agent_watcher";
pub const ACTOR_TYPE_WATCHER: &str = "watcher";
pub const ACTOR_TYPE_AGENT: &str = "agent";

// ── Watcher action constants ─────────────────────────────────────────────────
pub const WATCHER_ACTION_WATCHING: &str = "watching";
pub const WATCHER_ACTION_TRIGGERED: &str = "triggered";
pub const WATCHER_ACTION_QA_PASS: &str = "qa_pass";
pub const WATCHER_ACTION_QA_NEEDS_CHANGES: &str = "qa_needs_changes";
pub const WATCHER_ACTION_QA_FAIL: &str = "qa_fail";

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "task_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Todo,
    InProgress,
    InReview,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "priority", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "approval_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    ChangesRequested,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Task {
    pub id: String,
    pub project_id: String, // Foreign key to Project
    pub pod_id: Option<String>,
    pub board_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub parent_task_attempt: Option<String>, // Foreign key to parent TaskAttempt
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Phase A: Core Collaboration Fields
    pub priority: Priority,
    pub assignee_id: Option<String>,
    pub assignee_type: Option<String>,   // Polymorphic: "user", "agent", "team"
    pub assigned_agent: Option<String>,  // Legacy: agent name (e.g., "Nora")
    pub agent_id: Option<String>,           // New: foreign key to agents table
    pub assigned_mcps: Option<String>,    // JSON array of strings
    pub created_by: String,
    pub requires_approval: bool,
    pub approval_status: Option<ApprovalStatus>,
    pub parent_task_id: Option<String>, // For subtasks
    pub tags: Option<String>,         // JSON array of strings
    pub due_date: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, unknown> | null")]
    pub custom_properties: Option<Json<Value>>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    /// Base64 encoded screenshot image for bug reports
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// Structured success criteria for agent self-evaluation
    pub completion_criteria: Option<String>,
    /// Expected deliverable format (e.g. "markdown report", "code PR", "JSON API response")
    pub output_format: Option<String>,
}

/// Brief execution summary for task card display
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
pub struct ExecutionSummaryBrief {
    pub files_modified: i32,
    pub files_created: i32,
    pub commands_failed: i32,
    pub completion_status: CompletionStatus,
    pub tools_used: Option<Vec<String>>,
    pub human_rating: Option<i32>,
    pub execution_time_ms: i64,
}

/// Collaborator information for task display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskCollaborator {
    pub actor_id: String,
    pub actor_type: String, // "human", "agent", "mcp", "system"
    pub last_action: String,
    pub last_action_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskWithAttemptStatus {
    #[serde(flatten)]
    #[ts(flatten)]
    pub task: Task,
    pub has_in_progress_attempt: bool,
    pub has_merged_attempt: bool,
    pub last_attempt_failed: bool,
    pub executor: String,
    /// Latest execution summary for the task (populated from most recent attempt)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_execution_summary: Option<ExecutionSummaryBrief>,
    /// Collaborators who have worked on this task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collaborators: Option<Vec<TaskCollaborator>>,
    /// Total VIBE cost for this task (aggregated from vibe_transactions)
    #[serde(default)]
    pub vibe_cost: Option<i64>,
    /// Model used for the most recent vibe transaction on this task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vibe_model: Option<String>,
}

impl std::ops::Deref for TaskWithAttemptStatus {
    type Target = Task;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl std::ops::DerefMut for TaskWithAttemptStatus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskRelationships {
    pub parent_task: Option<Task>,    // The task that owns this attempt
    pub current_attempt: TaskAttempt, // The attempt we're viewing
    pub children: Vec<Task>,          // Tasks created by this attempt
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateTask {
    pub project_id: String,
    #[ts(optional)]
    pub pod_id: Option<String>,
    #[ts(optional)]
    pub board_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub parent_task_attempt: Option<String>,
    pub image_ids: Option<Vec<String>>,

    // Phase A: Core Collaboration Fields
    pub priority: Option<Priority>,
    pub assignee_id: Option<String>,
    pub assignee_type: Option<String>,   // Polymorphic: "user", "agent", "team"
    pub assigned_agent: Option<String>,  // Legacy: agent name
    pub agent_id: Option<String>,           // New: foreign key to agents table
    pub assigned_mcps: Option<Vec<String>>,
    #[serde(default)]
    pub created_by: String,
    pub requires_approval: Option<bool>,
    pub parent_task_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub custom_properties: Option<Value>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    /// Base64 encoded screenshot image for bug reports
    pub screenshot: Option<String>,
    /// Structured success criteria for agent self-evaluation
    pub completion_criteria: Option<String>,
    /// Expected deliverable format (e.g. "markdown report", "code PR", "JSON API response")
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub parent_task_attempt: Option<String>,
    pub image_ids: Option<Vec<String>>,
    #[ts(optional)]
    pub pod_id: Option<Option<String>>,
    #[ts(optional)]
    pub board_id: Option<Option<String>>,

    // Phase A: Core Collaboration Fields
    pub priority: Option<Priority>,
    pub assignee_id: Option<String>,
    pub assignee_type: Option<String>,   // Polymorphic: "user", "agent", "team"
    pub assigned_agent: Option<String>,  // Legacy: agent name
    pub agent_id: Option<Option<String>>,   // New: foreign key to agents table
    pub assigned_mcps: Option<Vec<String>>,
    pub requires_approval: Option<bool>,
    pub approval_status: Option<ApprovalStatus>,
    pub parent_task_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub custom_properties: Option<Option<Value>>,
    pub scheduled_start: Option<Option<DateTime<Utc>>>,
    pub scheduled_end: Option<Option<DateTime<Utc>>>,
    /// Structured success criteria for agent self-evaluation
    pub completion_criteria: Option<String>,
    /// Expected deliverable format
    pub output_format: Option<String>,
}

/// Intermediate row struct for the complex find_by_project_id_with_attempt_status query
#[derive(Debug, FromRow)]
struct TaskWithStatusRow {
    pub id: String,
    pub project_id: String,
    pub pod_id: Option<String>,
    pub board_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub parent_task_attempt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub priority: Priority,
    pub assignee_id: Option<String>,
    pub assignee_type: Option<String>,
    pub assigned_agent: Option<String>,
    pub agent_id: Option<String>,
    pub assigned_mcps: Option<String>,
    pub created_by: String,
    pub requires_approval: bool,
    pub approval_status: Option<ApprovalStatus>,
    pub parent_task_id: Option<String>,
    pub tags: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub custom_properties: Option<Json<Value>>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub collaborators: Option<String>,
    pub screenshot: Option<String>,
    pub completion_criteria: Option<String>,
    pub output_format: Option<String>,
    pub has_in_progress_attempt: i64,
    pub last_attempt_failed: i64,
    pub executor: String,
    pub vibe_cost: i64,
    pub vibe_model: Option<String>,
}

/// SQL fragment for selecting all Task columns (for use in query_as::<_, Task>)
const TASK_SELECT_SQL: &str = r#"
    id,
    project_id,
    pod_id,
    board_id,
    title,
    description,
    status,
    parent_task_attempt,
    created_at,
    updated_at,
    priority,
    assignee_id,
    assignee_type,
    assigned_agent,
    agent_id,
    assigned_mcps,
    created_by,
    requires_approval,
    approval_status,
    parent_task_id,
    tags,
    due_date,
    NULLIF(custom_properties, '') AS custom_properties,
    scheduled_start,
    scheduled_end,
    screenshot,
    completion_criteria,
    output_format
"#;

impl Task {
    pub fn to_prompt(&self) -> String {
        let mut parts = vec![format!("Title: {}", &self.title)];
        if let Some(description) = &self.description {
            parts.push(format!("Description: {}", description));
        }
        if let Some(criteria) = &self.completion_criteria {
            parts.push(format!("Completion Criteria: {}", criteria));
        }
        if let Some(fmt) = &self.output_format {
            parts.push(format!("Output Format: {}", fmt));
        }
        parts.join("\n\n")
    }

    fn serialize_json_array(arr: &Option<Vec<String>>) -> Option<String> {
        arr.as_ref().map(|v| serde_json::to_string(v).unwrap())
    }

    pub async fn parent_project(&self, pool: &SqlitePool) -> Result<Option<Project>, sqlx::Error> {
        Project::find_by_id(pool, &self.project_id).await
    }

    pub async fn find_by_project_id_with_attempt_status(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<TaskWithAttemptStatus>, sqlx::Error> {
        let records = sqlx::query_as::<_, TaskWithStatusRow>(
            r#"SELECT
  t.id,
  t.project_id,
  t.pod_id,
  t.board_id,
  t.title,
  t.description,
  t.status,
  t.parent_task_attempt,
  t.created_at,
  t.updated_at,
  t.priority,
  t.assignee_id,
  t.assignee_type,
  t.assigned_agent,
  t.agent_id,
  t.assigned_mcps,
  t.created_by,
  t.requires_approval,
  t.approval_status,
  t.parent_task_id,
  t.tags,
  t.due_date,
  NULLIF(t.custom_properties, '') AS custom_properties,
  t.scheduled_start,
  t.scheduled_end,

  CASE WHEN EXISTS (
    SELECT 1
      FROM task_attempts ta
      JOIN execution_processes ep
        ON ep.task_attempt_id = ta.id
     WHERE ta.task_id       = t.id
       AND ep.status        = 'running'
       AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     LIMIT 1
  ) THEN 1 ELSE 0 END            AS has_in_progress_attempt,

  CASE WHEN (
    SELECT ep.status
      FROM task_attempts ta
      JOIN execution_processes ep
        ON ep.task_attempt_id = ta.id
     WHERE ta.task_id       = t.id
     AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     ORDER BY ep.created_at DESC
     LIMIT 1
  ) IN ('failed','killed') THEN 1 ELSE 0 END
                                 AS last_attempt_failed,

  COALESCE(( SELECT ta.executor
      FROM task_attempts ta
      WHERE ta.task_id = t.id
     ORDER BY ta.created_at DESC
      LIMIT 1
    ), '')                         AS executor,

  t.collaborators,
  t.screenshot,
  t.completion_criteria,
  t.output_format,

  COALESCE(( SELECT SUM(vt.amount_vibe)
      FROM vibe_transactions vt
     WHERE vt.task_id = t.id
  ), 0)                             AS vibe_cost,

  ( SELECT vt.model
      FROM vibe_transactions vt
     WHERE vt.task_id = t.id
     ORDER BY vt.created_at DESC
     LIMIT 1
  )                                 AS vibe_model

FROM tasks t
WHERE t.project_id = $1 AND t.deleted_at IS NULL
ORDER BY t.created_at DESC"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        let tasks = records
            .into_iter()
            .map(|rec| TaskWithAttemptStatus {
                task: Task {
                    id: rec.id,
                    project_id: rec.project_id,
                    pod_id: rec.pod_id,
                    title: rec.title,
                    description: rec.description,
                    status: rec.status,
                    parent_task_attempt: rec.parent_task_attempt,
                    created_at: rec.created_at,
                    updated_at: rec.updated_at,
                    priority: rec.priority,
                    assignee_id: rec.assignee_id,
                    assignee_type: rec.assignee_type,
                    assigned_agent: rec.assigned_agent,
                    agent_id: rec.agent_id,
                    assigned_mcps: rec.assigned_mcps,
                    created_by: rec.created_by,
                    requires_approval: rec.requires_approval,
                    approval_status: rec.approval_status,
                    parent_task_id: rec.parent_task_id,
                    tags: rec.tags,
                    due_date: rec.due_date,
                    board_id: rec.board_id,
                    custom_properties: rec.custom_properties,
                    scheduled_start: rec.scheduled_start,
                    scheduled_end: rec.scheduled_end,
                    screenshot: rec.screenshot,
                    completion_criteria: rec.completion_criteria,
                    output_format: rec.output_format,
                },
                has_in_progress_attempt: rec.has_in_progress_attempt != 0,
                has_merged_attempt: false, // TODO use merges table
                last_attempt_failed: rec.last_attempt_failed != 0,
                executor: rec.executor,
                last_execution_summary: None, // Loaded separately via API when needed
                collaborators: rec.collaborators
                    .as_deref()
                    .and_then(|json| serde_json::from_str(json).ok()),
                vibe_cost: if rec.vibe_cost > 0 { Some(rec.vibe_cost) } else { None },
                vibe_model: rec.vibe_model,
            })
            .collect();

        Ok(tasks)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE id = $1 AND deleted_at IS NULL",
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_rowid(pool: &SqlitePool, rowid: i64) -> Result<Option<Self>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE rowid = $1 AND deleted_at IS NULL",
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(rowid)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_id_and_project_id(
        pool: &SqlitePool,
        id: &str,
        project_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL",
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(id)
            .bind(project_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTask,
        task_id: &str,
    ) -> Result<Self, sqlx::Error> {
        let priority = data.priority.clone().unwrap_or(Priority::Medium);
        let requires_approval = data.requires_approval.unwrap_or(false);
        let assigned_mcps_json = Self::serialize_json_array(&data.assigned_mcps);
        let tags_json = Self::serialize_json_array(&data.tags);
        let custom_properties = data.custom_properties.clone().map(Json);
        // Auto-derive assignee_type if not explicitly set
        let assignee_type = data.assignee_type.clone().or_else(|| {
            if data.assignee_id.is_some() { Some("user".to_string()) }
            else if data.agent_id.is_some() { Some("agent".to_string()) }
            else { None }
        });

        let status_str = "todo";

        let sql = format!(
            r#"INSERT INTO tasks (
                id, project_id, pod_id, board_id, title, description, status, parent_task_attempt,
                priority, assignee_id, assignee_type, assigned_agent, agent_id, assigned_mcps, created_by,
                requires_approval, parent_task_id, tags, due_date,
                custom_properties, scheduled_start, scheduled_end, screenshot,
                completion_criteria, output_format
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25)
               RETURNING {}"#,
            TASK_SELECT_SQL
        );

        sqlx::query_as::<_, Task>(&sql)
            .bind(task_id)
            .bind(&data.project_id)
            .bind(&data.pod_id)
            .bind(&data.board_id)
            .bind(&data.title)
            .bind(&data.description)
            .bind(status_str)
            .bind(&data.parent_task_attempt)
            .bind(&priority)
            .bind(&data.assignee_id)
            .bind(&assignee_type)
            .bind(&data.assigned_agent)
            .bind(&data.agent_id)
            .bind(&assigned_mcps_json)
            .bind(&data.created_by)
            .bind(requires_approval)
            .bind(&data.parent_task_id)
            .bind(&tags_json)
            .bind(&data.due_date)
            .bind(&custom_properties)
            .bind(&data.scheduled_start)
            .bind(&data.scheduled_end)
            .bind(&data.screenshot)
            .bind(&data.completion_criteria)
            .bind(&data.output_format)
            .fetch_one(pool)
            .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        project_id: &str,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        parent_task_attempt: Option<String>,
        pod_id: Option<String>,
        board_id: Option<String>,
        priority: Priority,
        assignee_id: Option<String>,
        assignee_type: Option<String>,
        assigned_agent: Option<String>,
        assigned_mcps: Option<String>,
        requires_approval: bool,
        approval_status: Option<ApprovalStatus>,
        parent_task_id: Option<String>,
        tags: Option<String>,
        due_date: Option<DateTime<Utc>>,
        custom_properties: Option<Json<Value>>,
        scheduled_start: Option<DateTime<Utc>>,
        scheduled_end: Option<DateTime<Utc>>,
        completion_criteria: Option<String>,
        output_format: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        let sql = format!(
            r#"UPDATE tasks
               SET title = $3, description = $4, status = $5, parent_task_attempt = $6,
                   pod_id = $7,
                   board_id = $8,
                   priority = $9, assignee_id = $10, assignee_type = $11,
                   assigned_agent = $12, assigned_mcps = $13,
                   requires_approval = $14, approval_status = $15, parent_task_id = $16,
                   tags = $17, due_date = $18,
                   custom_properties = $19,
                   scheduled_start = $20,
                   scheduled_end = $21,
                   completion_criteria = $22,
                   output_format = $23,
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL
               RETURNING {}"#,
            TASK_SELECT_SQL
        );

        sqlx::query_as::<_, Task>(&sql)
            .bind(id)
            .bind(project_id)
            .bind(&title)
            .bind(&description)
            .bind(&status)
            .bind(&parent_task_attempt)
            .bind(&pod_id)
            .bind(&board_id)
            .bind(&priority)
            .bind(&assignee_id)
            .bind(&assignee_type)
            .bind(&assigned_agent)
            .bind(&assigned_mcps)
            .bind(requires_approval)
            .bind(&approval_status)
            .bind(&parent_task_id)
            .bind(&tags)
            .bind(&due_date)
            .bind(&custom_properties)
            .bind(&scheduled_start)
            .bind(&scheduled_end)
            .bind(&completion_criteria)
            .bind(&output_format)
            .fetch_one(pool)
            .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: TaskStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE tasks SET status = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(id)
            .bind(&status)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Assign an agent to a task by updating its agent_id field.
    pub async fn assign_agent(
        pool: &SqlitePool,
        task_id: &str,
        agent_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE tasks SET agent_id = $2, updated_at = datetime('now', 'subsec') WHERE id = $1")
            .bind(task_id)
            .bind(agent_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Update collaborators on a task, adding or updating a collaborator entry
    pub async fn update_collaborator(
        pool: &SqlitePool,
        task_id: &str,
        actor_id: &str,
        actor_type: &str,
        action: &str,
    ) -> Result<(), sqlx::Error> {
        // Get existing collaborators
        let record = sqlx::query("SELECT collaborators FROM tasks WHERE id = $1 AND deleted_at IS NULL")
            .bind(task_id)
            .fetch_optional(pool)
            .await?;

        let mut collaborators: Vec<TaskCollaborator> = match record {
            Some(rec) => rec
                .get::<Option<String>, _>("collaborators")
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default(),
            None => return Ok(()), // Task doesn't exist
        };

        let now = Utc::now();

        // Update existing or add new collaborator
        if let Some(existing) = collaborators
            .iter_mut()
            .find(|c| c.actor_id == actor_id && c.actor_type == actor_type)
        {
            existing.last_action = action.to_string();
            existing.last_action_at = now;
        } else {
            collaborators.push(TaskCollaborator {
                actor_id: actor_id.to_string(),
                actor_type: actor_type.to_string(),
                last_action: action.to_string(),
                last_action_at: now,
            });
        }

        // Update the task
        let collaborators_json = serde_json::to_string(&collaborators)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        sqlx::query("UPDATE tasks SET collaborators = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(task_id)
            .bind(&collaborators_json)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn exists(
        pool: &SqlitePool,
        id: &str,
        project_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("SELECT id FROM tasks WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL")
            .bind(id)
            .bind(project_id)
            .fetch_optional(pool)
            .await?;
        Ok(result.is_some())
    }

    pub async fn find_children_by_attempt_id(
        pool: &SqlitePool,
        attempt_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE parent_task_attempt = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(attempt_id)
            .fetch_all(pool)
            .await
    }

    pub async fn find_relationships_for_attempt(
        pool: &SqlitePool,
        task_attempt: &TaskAttempt,
    ) -> Result<TaskRelationships, sqlx::Error> {
        // 1. Get the current task (task that owns this attempt)
        let current_task = Self::find_by_id(pool, &task_attempt.task_id.to_string())
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // 2. Get parent task (if current task was created by another task's attempt)
        let parent_task = if let Some(ref parent_attempt_id) = current_task.parent_task_attempt {
            // Find the attempt that created the current task
            // parent_attempt_id is now a String, parse to Uuid for TaskAttempt::find_by_id
            if let Ok(parent_uuid) = Uuid::parse_str(parent_attempt_id) {
                if let Ok(Some(parent_attempt)) = TaskAttempt::find_by_id(pool, parent_uuid).await
                {
                    // Find the task that owns that parent attempt - THAT's the real parent
                    Self::find_by_id(pool, &parent_attempt.task_id.to_string()).await?
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // 3. Get children tasks (created by this attempt)
        let children = Self::find_children_by_attempt_id(pool, &task_attempt.id.to_string()).await?;

        Ok(TaskRelationships {
            parent_task,
            current_attempt: task_attempt.clone(),
            children,
        })
    }

    /// Add a user as a watcher on a task
    pub async fn add_watcher(
        pool: &SqlitePool,
        task_id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        Self::update_collaborator(pool, task_id, user_id, "watcher", "watching").await
    }

    /// Remove a watcher from a task
    pub async fn remove_watcher(
        pool: &SqlitePool,
        task_id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        let record = sqlx::query("SELECT collaborators FROM tasks WHERE id = $1 AND deleted_at IS NULL")
            .bind(task_id)
            .fetch_optional(pool)
            .await?;

        let mut collaborators: Vec<TaskCollaborator> = match record {
            Some(rec) => rec
                .get::<Option<String>, _>("collaborators")
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default(),
            None => return Ok(()),
        };

        collaborators.retain(|c| !(c.actor_id == user_id && c.actor_type == "watcher"));

        let collaborators_json = serde_json::to_string(&collaborators)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        sqlx::query("UPDATE tasks SET collaborators = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(task_id)
            .bind(&collaborators_json)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Find all tasks watched by a specific user (via collaborators JSON)
    pub async fn find_watched_by_user(
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // SQLite JSON: search for watcher entries in collaborators array
        let pattern = format!("%\"actor_id\":\"{}\"%\"actor_type\":\"watcher\"%", user_id);
        let sql = format!(
            "SELECT {} FROM tasks WHERE collaborators LIKE $1 AND deleted_at IS NULL ORDER BY updated_at DESC",
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(&pattern)
            .fetch_all(pool)
            .await
    }

    // ── Agent Watcher Methods ─────────────────────────────────────────────────

    /// Add an agent as a watcher on a task (distinct from human watchers).
    /// Uses `actor_type = "agent_watcher"` so existing remove_watcher() won't touch it.
    pub async fn add_agent_watcher(
        pool: &SqlitePool,
        task_id: &str,
        agent_id: &str,
    ) -> Result<(), sqlx::Error> {
        Self::update_collaborator(
            pool,
            task_id,
            agent_id,
            ACTOR_TYPE_AGENT_WATCHER,
            WATCHER_ACTION_WATCHING,
        )
        .await
    }

    /// Remove an agent watcher from a task.
    pub async fn remove_agent_watcher(
        pool: &SqlitePool,
        task_id: &str,
        agent_id: &str,
    ) -> Result<(), sqlx::Error> {
        let record =
            sqlx::query("SELECT collaborators FROM tasks WHERE id = $1 AND deleted_at IS NULL")
                .bind(task_id)
                .fetch_optional(pool)
                .await?;

        let mut collaborators: Vec<TaskCollaborator> = match record {
            Some(rec) => rec
                .get::<Option<String>, _>("collaborators")
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default(),
            None => return Ok(()),
        };

        collaborators.retain(|c| {
            !(c.actor_id == agent_id && c.actor_type == ACTOR_TYPE_AGENT_WATCHER)
        });

        let collaborators_json = serde_json::to_string(&collaborators)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        sqlx::query(
            "UPDATE tasks SET collaborators = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(task_id)
        .bind(&collaborators_json)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Find all agent watchers on a task.
    pub async fn find_agent_watchers(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<Vec<TaskCollaborator>, sqlx::Error> {
        let record =
            sqlx::query("SELECT collaborators FROM tasks WHERE id = $1 AND deleted_at IS NULL")
                .bind(task_id)
                .fetch_optional(pool)
                .await?;

        let collaborators: Vec<TaskCollaborator> = match record {
            Some(rec) => rec
                .get::<Option<String>, _>("collaborators")
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default(),
            None => return Ok(vec![]),
        };

        Ok(collaborators
            .into_iter()
            .filter(|c| c.actor_type == ACTOR_TYPE_AGENT_WATCHER)
            .collect())
    }

    /// Find agent watchers that haven't been triggered yet (last_action == "watching").
    pub async fn find_pending_agent_watchers(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<Vec<TaskCollaborator>, sqlx::Error> {
        let watchers = Self::find_agent_watchers(pool, task_id).await?;
        Ok(watchers
            .into_iter()
            .filter(|c| c.last_action == WATCHER_ACTION_WATCHING)
            .collect())
    }

    /// Find all tasks assigned to a specific user across all projects
    pub async fn find_by_assignee(
        pool: &SqlitePool,
        assignee_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let sql = format!(
            r#"SELECT {}
               FROM tasks
               WHERE assignee_id = $1
               AND status != 'completed'
               AND deleted_at IS NULL
               ORDER BY
                 CASE priority
                   WHEN 'urgent' THEN 0
                   WHEN 'high' THEN 1
                   WHEN 'medium' THEN 2
                   WHEN 'low' THEN 3
                   ELSE 4
                 END,
                 due_date ASC NULLS LAST,
                 created_at DESC"#,
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(assignee_id)
            .fetch_all(pool)
            .await
    }

    pub async fn find_by_creator(
        pool: &SqlitePool,
        created_by: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tasks WHERE created_by = $1 AND deleted_at IS NULL ORDER BY updated_at DESC",
            TASK_SELECT_SQL
        );
        sqlx::query_as::<_, Task>(&sql)
            .bind(created_by)
            .fetch_all(pool)
            .await
    }
}
