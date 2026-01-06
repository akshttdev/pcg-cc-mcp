use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ContextInjectionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Context injection not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "injection_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InjectionType {
    Note,
    Correction,
    Approval,
    Rejection,
    Directive,
    Question,
    Answer,
}

impl std::fmt::Display for InjectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InjectionType::Note => write!(f, "note"),
            InjectionType::Correction => write!(f, "correction"),
            InjectionType::Approval => write!(f, "approval"),
            InjectionType::Rejection => write!(f, "rejection"),
            InjectionType::Directive => write!(f, "directive"),
            InjectionType::Question => write!(f, "question"),
            InjectionType::Answer => write!(f, "answer"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ContextInjection {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub injector_id: String,
    pub injector_name: Option<String>,
    pub injection_type: InjectionType,
    pub content: String,
    #[sqlx(default)]
    pub metadata: Option<String>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateContextInjection {
    pub execution_process_id: Uuid,
    pub injector_id: String,
    pub injector_name: Option<String>,
    pub injection_type: InjectionType,
    pub content: String,
    pub metadata: Option<Value>,
}

impl ContextInjection {
    /// Create a new context injection
    pub async fn create(
        pool: &SqlitePool,
        data: CreateContextInjection,
    ) -> Result<Self, ContextInjectionError> {
        let id = Uuid::new_v4();
        let injection_type_str = data.injection_type.to_string();
        let metadata_str = data.metadata.map(|v| v.to_string());

        let injection = sqlx::query_as::<_, ContextInjection>(
            r#"
            INSERT INTO context_injections (
                id, execution_process_id, injector_id, injector_name,
                injection_type, content, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(&data.injector_id)
        .bind(&data.injector_name)
        .bind(injection_type_str)
        .bind(&data.content)
        .bind(metadata_str)
        .fetch_one(pool)
        .await?;

        Ok(injection)
    }

    /// Find injection by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ContextInjectionError> {
        let injection = sqlx::query_as::<_, ContextInjection>(
            r#"SELECT * FROM context_injections WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(injection)
    }

    /// Find all injections for an execution
    pub async fn find_by_execution(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ContextInjectionError> {
        let injections = sqlx::query_as::<_, ContextInjection>(
            r#"
            SELECT * FROM context_injections
            WHERE execution_process_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(injections)
    }

    /// Find unacknowledged injections for an execution
    pub async fn find_unacknowledged(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ContextInjectionError> {
        let injections = sqlx::query_as::<_, ContextInjection>(
            r#"
            SELECT * FROM context_injections
            WHERE execution_process_id = ?1 AND acknowledged = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(injections)
    }

    /// Acknowledge an injection
    pub async fn acknowledge(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Self, ContextInjectionError> {
        let injection = sqlx::query_as::<_, ContextInjection>(
            r#"
            UPDATE context_injections
            SET acknowledged = 1, acknowledged_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(injection)
    }

    /// Acknowledge all injections for an execution
    pub async fn acknowledge_all(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<u64, ContextInjectionError> {
        let result = sqlx::query(
            r#"
            UPDATE context_injections
            SET acknowledged = 1, acknowledged_at = datetime('now', 'subsec')
            WHERE execution_process_id = ?1 AND acknowledged = 0
            "#,
        )
        .bind(execution_process_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Parse metadata as JSON Value
    pub fn metadata_json(&self) -> Option<Value> {
        self.metadata
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Check if this is a blocking injection type (requires response)
    pub fn is_blocking(&self) -> bool {
        matches!(
            self.injection_type,
            InjectionType::Question | InjectionType::Directive
        )
    }
}
