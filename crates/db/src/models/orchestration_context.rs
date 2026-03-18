use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ContextEntryType {
    Finding,
    Decision,
    Blocker,
    IntermediateResult,
    Directive,
}

impl std::fmt::Display for ContextEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Finding => write!(f, "finding"),
            Self::Decision => write!(f, "decision"),
            Self::Blocker => write!(f, "blocker"),
            Self::IntermediateResult => write!(f, "intermediate_result"),
            Self::Directive => write!(f, "directive"),
        }
    }
}

impl std::str::FromStr for ContextEntryType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "finding" => Ok(Self::Finding),
            "decision" => Ok(Self::Decision),
            "blocker" => Ok(Self::Blocker),
            "intermediate_result" | "result" => Ok(Self::IntermediateResult),
            "directive" => Ok(Self::Directive),
            _ => Err(format!("Invalid entry type: '{}'", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ContextEntryStatus {
    Active,
    Resolved,
    Superseded,
}

impl std::fmt::Display for ContextEntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Resolved => write!(f, "resolved"),
            Self::Superseded => write!(f, "superseded"),
        }
    }
}

impl std::str::FromStr for ContextEntryStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "resolved" => Ok(Self::Resolved),
            "superseded" => Ok(Self::Superseded),
            _ => Err(format!("Invalid status: '{}'", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ContextPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl std::fmt::Display for ContextPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for ContextPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("Invalid priority: '{}'", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrchestrationContext {
    pub id: Uuid,
    pub root_task_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Option<Uuid>,
    pub entry_type: ContextEntryType,
    pub title: String,
    pub content: String,
    pub source: String,
    pub status: ContextEntryStatus,
    pub priority: ContextPriority,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateOrchestrationContext {
    pub root_task_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Option<Uuid>,
    pub entry_type: ContextEntryType,
    pub title: String,
    pub content: String,
    pub source: String,
    pub priority: ContextPriority,
    pub metadata: Option<String>,
}

/// Raw row for sqlx::query_as (avoids compile-time DATABASE_URL requirement)
#[derive(Debug, FromRow)]
struct ContextRow {
    id: Vec<u8>,
    root_task_id: Vec<u8>,
    project_id: Vec<u8>,
    task_id: Option<Vec<u8>>,
    entry_type: String,
    title: String,
    content: String,
    source: String,
    status: String,
    priority: String,
    resolved_by: Option<String>,
    resolved_at: Option<String>,
    metadata: Option<String>,
    created_at: String,
    updated_at: String,
}

fn bytes_to_uuid(bytes: &[u8]) -> Option<Uuid> {
    if bytes.len() == 16 {
        Some(Uuid::from_bytes(bytes.try_into().unwrap()))
    } else {
        // Try parsing as hex string
        let hex = String::from_utf8_lossy(bytes);
        Uuid::parse_str(&hex).ok()
    }
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .map(|d| d.and_utc()))
        .unwrap_or_else(|_| Utc::now())
}

impl From<ContextRow> for OrchestrationContext {
    fn from(row: ContextRow) -> Self {
        Self {
            id: bytes_to_uuid(&row.id).unwrap_or_else(Uuid::nil),
            root_task_id: bytes_to_uuid(&row.root_task_id).unwrap_or_else(Uuid::nil),
            project_id: bytes_to_uuid(&row.project_id).unwrap_or_else(Uuid::nil),
            task_id: row.task_id.as_deref().and_then(bytes_to_uuid),
            entry_type: row.entry_type.parse().unwrap_or(ContextEntryType::Finding),
            title: row.title,
            content: row.content,
            source: row.source,
            status: row.status.parse().unwrap_or(ContextEntryStatus::Active),
            priority: row.priority.parse().unwrap_or(ContextPriority::Normal),
            resolved_by: row.resolved_by,
            resolved_at: row.resolved_at.as_deref().map(parse_dt),
            metadata: row.metadata,
            created_at: parse_dt(&row.created_at),
            updated_at: parse_dt(&row.updated_at),
        }
    }
}

impl OrchestrationContext {
    pub async fn create(
        pool: &SqlitePool,
        data: &CreateOrchestrationContext,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_bytes = id.to_string();
        let root_bytes = data.root_task_id.to_string();
        let proj_bytes = data.project_id.to_string();
        let task_bytes = data.task_id.map(|u| u.to_string());
        let entry_type = data.entry_type.to_string();
        let status = ContextEntryStatus::Active.to_string();
        let priority = data.priority.to_string();

        sqlx::query(
            "INSERT INTO orchestration_contexts \
             (id, root_task_id, project_id, task_id, entry_type, title, content, source, status, priority, metadata) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id_bytes)
        .bind(&root_bytes)
        .bind(&proj_bytes)
        .bind(&task_bytes)
        .bind(&entry_type)
        .bind(&data.title)
        .bind(&data.content)
        .bind(&data.source)
        .bind(&status)
        .bind(&priority)
        .bind(&data.metadata)
        .execute(pool)
        .await?;

        // Return the created entry
        let row = sqlx::query_as::<_, ContextRow>(
            "SELECT id, root_task_id, project_id, task_id, entry_type, title, content, source, \
             status, priority, resolved_by, resolved_at, metadata, created_at, updated_at \
             FROM orchestration_contexts WHERE id = ?"
        )
        .bind(&id_bytes)
        .fetch_one(pool)
        .await?;

        Ok(row.into())
    }

    pub async fn find_by_root_task(
        pool: &SqlitePool,
        root_task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let root_bytes = root_task_id.to_string();
        let rows = sqlx::query_as::<_, ContextRow>(
            "SELECT id, root_task_id, project_id, task_id, entry_type, title, content, source, \
             status, priority, resolved_by, resolved_at, metadata, created_at, updated_at \
             FROM orchestration_contexts WHERE root_task_id = ? \
             ORDER BY CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END, \
             created_at DESC"
        )
        .bind(&root_bytes)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn find_active_by_root_task(
        pool: &SqlitePool,
        root_task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let root_bytes = root_task_id.to_string();
        let rows = sqlx::query_as::<_, ContextRow>(
            "SELECT id, root_task_id, project_id, task_id, entry_type, title, content, source, \
             status, priority, resolved_by, resolved_at, metadata, created_at, updated_at \
             FROM orchestration_contexts WHERE root_task_id = ? AND status = 'active' \
             ORDER BY CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END, \
             created_at DESC"
        )
        .bind(&root_bytes)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn resolve(
        pool: &SqlitePool,
        id: Uuid,
        resolved_by: &str,
        status: &ContextEntryStatus,
    ) -> Result<bool, sqlx::Error> {
        let id_bytes = id.to_string();
        let status_str = status.to_string();
        let result = sqlx::query(
            "UPDATE orchestration_contexts \
             SET status = ?, resolved_by = ?, resolved_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND status = 'active'"
        )
        .bind(&status_str)
        .bind(resolved_by)
        .bind(&id_bytes)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let proj_bytes = project_id.to_string();
        let rows = sqlx::query_as::<_, ContextRow>(
            "SELECT id, root_task_id, project_id, task_id, entry_type, title, content, source, \
             status, priority, resolved_by, resolved_at, metadata, created_at, updated_at \
             FROM orchestration_contexts WHERE project_id = ? \
             ORDER BY created_at DESC"
        )
        .bind(&proj_bytes)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let id_bytes = id.to_string();
        let result = sqlx::query("DELETE FROM orchestration_contexts WHERE id = ?")
            .bind(&id_bytes)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
