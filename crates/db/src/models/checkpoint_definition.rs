use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CheckpointDefinitionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Checkpoint definition not found")]
    NotFound,
    #[error("Invalid checkpoint type: {0}")]
    InvalidType(String),
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "checkpoint_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CheckpointType {
    FileChange,
    ExternalCall,
    CostThreshold,
    TimeThreshold,
    Custom,
}

impl std::fmt::Display for CheckpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointType::FileChange => write!(f, "file_change"),
            CheckpointType::ExternalCall => write!(f, "external_call"),
            CheckpointType::CostThreshold => write!(f, "cost_threshold"),
            CheckpointType::TimeThreshold => write!(f, "time_threshold"),
            CheckpointType::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct CheckpointDefinition {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub checkpoint_type: CheckpointType,
    #[sqlx(default)]
    pub config: String, // JSON
    pub requires_approval: bool,
    pub auto_approve_after_minutes: Option<i32>,
    pub priority: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCheckpointDefinition {
    pub project_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub checkpoint_type: CheckpointType,
    pub config: Option<Value>,
    #[serde(default = "default_true")]
    pub requires_approval: bool,
    pub auto_approve_after_minutes: Option<i32>,
    #[serde(default)]
    pub priority: i32,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCheckpointDefinition {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<Value>,
    pub requires_approval: Option<bool>,
    pub auto_approve_after_minutes: Option<Option<i32>>,
    pub priority: Option<i32>,
    pub is_active: Option<bool>,
}

impl CheckpointDefinition {
    /// Create a new checkpoint definition
    pub async fn create(
        pool: &SqlitePool,
        data: CreateCheckpointDefinition,
    ) -> Result<Self, CheckpointDefinitionError> {
        let id = Uuid::new_v4();
        let checkpoint_type_str = data.checkpoint_type.to_string();
        let config_str = data.config.map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());

        let def = sqlx::query_as::<_, CheckpointDefinition>(
            r#"
            INSERT INTO checkpoint_definitions (
                id, project_id, name, description, checkpoint_type,
                config, requires_approval, auto_approve_after_minutes, priority
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(checkpoint_type_str)
        .bind(config_str)
        .bind(data.requires_approval)
        .bind(data.auto_approve_after_minutes)
        .bind(data.priority)
        .fetch_one(pool)
        .await?;

        Ok(def)
    }

    /// Find by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, CheckpointDefinitionError> {
        let def = sqlx::query_as::<_, CheckpointDefinition>(
            r#"SELECT * FROM checkpoint_definitions WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(def)
    }

    /// Find all active definitions for a project (including global ones)
    pub async fn find_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, CheckpointDefinitionError> {
        let defs = sqlx::query_as::<_, CheckpointDefinition>(
            r#"
            SELECT * FROM checkpoint_definitions
            WHERE (project_id = ?1 OR project_id IS NULL) AND is_active = 1
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(defs)
    }

    /// Find global definitions only
    pub async fn find_global(pool: &SqlitePool) -> Result<Vec<Self>, CheckpointDefinitionError> {
        let defs = sqlx::query_as::<_, CheckpointDefinition>(
            r#"
            SELECT * FROM checkpoint_definitions
            WHERE project_id IS NULL AND is_active = 1
            ORDER BY priority DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(defs)
    }

    /// Update a checkpoint definition
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateCheckpointDefinition,
    ) -> Result<Self, CheckpointDefinitionError> {
        // Build dynamic update query
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or(CheckpointDefinitionError::NotFound)?;

        let name = data.name.unwrap_or(current.name);
        let description = data.description.or(current.description);
        let config = data
            .config
            .map(|v| v.to_string())
            .unwrap_or(current.config);
        let requires_approval = data.requires_approval.unwrap_or(current.requires_approval);
        let auto_approve = data
            .auto_approve_after_minutes
            .unwrap_or(current.auto_approve_after_minutes);
        let priority = data.priority.unwrap_or(current.priority);
        let is_active = data.is_active.unwrap_or(current.is_active);

        let def = sqlx::query_as::<_, CheckpointDefinition>(
            r#"
            UPDATE checkpoint_definitions
            SET name = ?2, description = ?3, config = ?4, requires_approval = ?5,
                auto_approve_after_minutes = ?6, priority = ?7, is_active = ?8,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(config)
        .bind(requires_approval)
        .bind(auto_approve)
        .bind(priority)
        .bind(is_active)
        .fetch_one(pool)
        .await?;

        Ok(def)
    }

    /// Delete a checkpoint definition
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), CheckpointDefinitionError> {
        sqlx::query(r#"DELETE FROM checkpoint_definitions WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Parse config as JSON Value
    pub fn config_json(&self) -> Option<Value> {
        serde_json::from_str(&self.config).ok()
    }

    /// Check if this is a global definition
    pub fn is_global(&self) -> bool {
        self.project_id.is_none()
    }
}
