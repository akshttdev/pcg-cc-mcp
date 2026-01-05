use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionSlotError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Execution slot not found")]
    NotFound,
    #[error("No available slots for type: {0}")]
    NoAvailableSlots(String),
    #[error("Slot already released")]
    AlreadyReleased,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "slot_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SlotType {
    CodingAgent,
    BrowserAgent,
    Script,
}

impl std::fmt::Display for SlotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotType::CodingAgent => write!(f, "coding_agent"),
            SlotType::BrowserAgent => write!(f, "browser_agent"),
            SlotType::Script => write!(f, "script"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionSlot {
    pub id: Uuid,
    pub task_attempt_id: Uuid,
    pub slot_type: SlotType,
    pub resource_weight: i32,
    pub acquired_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateExecutionSlot {
    pub task_attempt_id: Uuid,
    pub slot_type: SlotType,
    pub resource_weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProjectCapacity {
    pub project_id: Uuid,
    pub max_concurrent_agents: i32,
    pub max_concurrent_browser_agents: i32,
    pub active_agent_slots: i32,
    pub active_browser_slots: i32,
    pub available_agent_slots: i32,
    pub available_browser_slots: i32,
}

impl ExecutionSlot {
    /// Create a new execution slot (acquire a slot)
    pub async fn create(
        pool: &SqlitePool,
        data: CreateExecutionSlot,
    ) -> Result<Self, ExecutionSlotError> {
        let id = Uuid::new_v4();
        let slot_type_str = data.slot_type.to_string();
        let resource_weight = data.resource_weight.unwrap_or(1);

        let slot = sqlx::query_as::<_, ExecutionSlot>(
            r#"
            INSERT INTO execution_slots (id, task_attempt_id, slot_type, resource_weight)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.task_attempt_id)
        .bind(slot_type_str)
        .bind(resource_weight)
        .fetch_one(pool)
        .await?;

        Ok(slot)
    }

    /// Release a slot (mark as released)
    pub async fn release(pool: &SqlitePool, id: Uuid) -> Result<Self, ExecutionSlotError> {
        let slot = sqlx::query_as::<_, ExecutionSlot>(
            r#"
            UPDATE execution_slots
            SET released_at = datetime('now', 'subsec')
            WHERE id = ?1 AND released_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ExecutionSlotError::AlreadyReleased)?;

        Ok(slot)
    }

    /// Find an active slot by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ExecutionSlotError> {
        let slot = sqlx::query_as::<_, ExecutionSlot>(
            r#"SELECT * FROM execution_slots WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(slot)
    }

    /// Find active slot for a task attempt
    pub async fn find_active_by_task_attempt(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Option<Self>, ExecutionSlotError> {
        let slot = sqlx::query_as::<_, ExecutionSlot>(
            r#"
            SELECT * FROM execution_slots
            WHERE task_attempt_id = ?1 AND released_at IS NULL
            ORDER BY acquired_at DESC
            LIMIT 1
            "#,
        )
        .bind(task_attempt_id)
        .fetch_optional(pool)
        .await?;

        Ok(slot)
    }

    /// Count active slots by type for a project
    pub async fn count_active_by_project_and_type(
        pool: &SqlitePool,
        project_id: Uuid,
        slot_type: SlotType,
    ) -> Result<i32, ExecutionSlotError> {
        let slot_type_str = slot_type.to_string();

        let count: (i32,) = sqlx::query_as(
            r#"
            SELECT COALESCE(COUNT(*), 0) as count
            FROM execution_slots es
            JOIN task_attempts ta ON es.task_attempt_id = ta.id
            JOIN tasks t ON ta.task_id = t.id
            WHERE t.project_id = ?1
              AND es.slot_type = ?2
              AND es.released_at IS NULL
            "#,
        )
        .bind(project_id)
        .bind(slot_type_str)
        .fetch_one(pool)
        .await?;

        Ok(count.0)
    }

    /// Get all active slots for a project
    pub async fn find_active_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionSlotError> {
        let slots = sqlx::query_as::<_, ExecutionSlot>(
            r#"
            SELECT es.*
            FROM execution_slots es
            JOIN task_attempts ta ON es.task_attempt_id = ta.id
            JOIN tasks t ON ta.task_id = t.id
            WHERE t.project_id = ?1 AND es.released_at IS NULL
            ORDER BY es.acquired_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(slots)
    }

    /// Release all slots for a task attempt
    pub async fn release_all_for_task_attempt(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<u64, ExecutionSlotError> {
        let result = sqlx::query(
            r#"
            UPDATE execution_slots
            SET released_at = datetime('now', 'subsec')
            WHERE task_attempt_id = ?1 AND released_at IS NULL
            "#,
        )
        .bind(task_attempt_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get project capacity information
    pub async fn get_project_capacity(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<ProjectCapacity, ExecutionSlotError> {
        // Get project limits
        let limits: (i32, i32) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(max_concurrent_agents, 3) as max_agents,
                COALESCE(max_concurrent_browser_agents, 1) as max_browser
            FROM projects
            WHERE id = ?1
            "#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        let active_agent_slots =
            Self::count_active_by_project_and_type(pool, project_id, SlotType::CodingAgent).await?;
        let active_browser_slots =
            Self::count_active_by_project_and_type(pool, project_id, SlotType::BrowserAgent)
                .await?;

        Ok(ProjectCapacity {
            project_id,
            max_concurrent_agents: limits.0,
            max_concurrent_browser_agents: limits.1,
            active_agent_slots,
            active_browser_slots,
            available_agent_slots: limits.0 - active_agent_slots,
            available_browser_slots: limits.1 - active_browser_slots,
        })
    }

    /// Check if a slot can be acquired for the given type
    pub async fn can_acquire(
        pool: &SqlitePool,
        project_id: Uuid,
        slot_type: SlotType,
    ) -> Result<bool, ExecutionSlotError> {
        let capacity = Self::get_project_capacity(pool, project_id).await?;

        match slot_type {
            SlotType::CodingAgent | SlotType::Script => {
                Ok(capacity.available_agent_slots > 0)
            }
            SlotType::BrowserAgent => Ok(capacity.available_browser_slots > 0),
        }
    }
}
