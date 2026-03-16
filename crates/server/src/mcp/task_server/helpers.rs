use chrono::{DateTime, Utc};
use db::models::task::{Priority, Task, TaskStatus, TaskWithAttemptStatus};
use rmcp::model::{CallToolResult, Content};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::types::TaskSummary;

// ─── Helper Functions ───────────────────────────────────────────────────────

pub(super) fn parse_task_status(status_str: &str) -> Option<TaskStatus> {
    match status_str.to_lowercase().as_str() {
        "todo" => Some(TaskStatus::Todo),
        "inprogress" | "in-progress" | "in_progress" => Some(TaskStatus::InProgress),
        "inreview" | "in-review" | "in_review" => Some(TaskStatus::InReview),
        "done" | "completed" => Some(TaskStatus::Done),
        "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

pub(super) fn task_status_to_string(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Todo => "todo".to_string(),
        TaskStatus::InProgress => "in-progress".to_string(),
        TaskStatus::InReview => "in-review".to_string(),
        TaskStatus::Done => "done".to_string(),
        TaskStatus::Cancelled => "cancelled".to_string(),
    }
}

pub(super) fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "critical" => Some(Priority::Critical),
        "high" => Some(Priority::High),
        "medium" => Some(Priority::Medium),
        "low" => Some(Priority::Low),
        _ => None,
    }
}

pub(super) fn priority_to_string(p: &Priority) -> String {
    match p {
        Priority::Critical => "critical".to_string(),
        Priority::High => "high".to_string(),
        Priority::Medium => "medium".to_string(),
        Priority::Low => "low".to_string(),
    }
}

pub(super) fn parse_iso_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first, then common ISO formats
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc());
    }
    None
}

/// Serialize a value to a pretty-printed JSON string, falling back to the error message on failure.
pub(super) fn to_json_pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!(r#"{{"error": "JSON serialization failed: {}"}}"#, e))
}

/// Serialize a value to a serde_json::Value, falling back to a null on failure.
pub(super) fn to_json_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

pub(super) fn parse_uuid(s: &str, field_name: &str) -> Result<Uuid, CallToolResult> {
    Uuid::parse_str(s).map_err(|_| {
        let err = serde_json::json!({
            "success": false,
            "error": format!("Invalid {} format. Must be a valid UUID.", field_name),
        });
        CallToolResult::error(vec![Content::text(to_json_pretty(&err))])
    })
}

pub(super) fn error_result(error: &str, details: Option<&str>) -> CallToolResult {
    let mut obj = serde_json::json!({ "success": false, "error": error });
    if let Some(d) = details {
        obj["details"] = Value::String(d.to_string());
    }
    CallToolResult::error(vec![Content::text(to_json_pretty(&obj))])
}

pub(super) fn success_json<T: Serialize>(value: &T) -> CallToolResult {
    CallToolResult::success(vec![Content::text(to_json_pretty(value))])
}

/// Build TaskSummary from a Task model
pub(super) fn task_to_summary(task: &Task) -> TaskSummary {
    let tags: Option<Vec<String>> = task
        .tags
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    TaskSummary {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: task_status_to_string(&task.status),
        priority: priority_to_string(&task.priority),
        assignee_id: task.assignee_id.clone(),
        assigned_agent: task.assigned_agent.clone(),
        tags,
        due_date: task.due_date.map(|d| d.to_rfc3339()),
        parent_task_id: task.parent_task_id.clone(),
        requires_approval: task.requires_approval,
        approval_status: task.approval_status.as_ref().map(|s| format!("{:?}", s).to_lowercase()),
        created_by: task.created_by.clone(),
        vibe_cost: None,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
        has_in_progress_attempt: None,
        has_merged_attempt: None,
        last_attempt_failed: None,
        completion_criteria: task.completion_criteria.clone(),
        output_format: task.output_format.clone(),
    }
}

/// Build TaskSummary from TaskWithAttemptStatus
pub(super) fn task_with_status_to_summary(task: &TaskWithAttemptStatus) -> TaskSummary {
    let tags: Option<Vec<String>> = task.tags.as_deref().and_then(|s| serde_json::from_str(s).ok());

    TaskSummary {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: task_status_to_string(&task.status),
        priority: priority_to_string(&task.priority),
        assignee_id: task.assignee_id.clone(),
        assigned_agent: task.assigned_agent.clone(),
        tags,
        due_date: task.due_date.map(|d| d.to_rfc3339()),
        parent_task_id: task.parent_task_id.clone(),
        requires_approval: task.requires_approval,
        approval_status: task.approval_status.as_ref().map(|s| format!("{:?}", s).to_lowercase()),
        created_by: task.created_by.clone(),
        vibe_cost: task.vibe_cost,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
        has_in_progress_attempt: Some(task.has_in_progress_attempt),
        has_merged_attempt: Some(task.has_merged_attempt),
        last_attempt_failed: Some(task.last_attempt_failed),
        completion_criteria: task.completion_criteria.clone(),
        output_format: task.output_format.clone(),
    }
}
