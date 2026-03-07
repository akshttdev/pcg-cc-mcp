use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum UserKnowledgeSourceError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Not found")]
    NotFound,
}

/// source_type values for user knowledge
/// 'skill' | 'interaction' | 'note' | 'bookmark' | 'research' |
/// 'conversation' | 'artifact' | 'context_injection'
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserKnowledgeSource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_type: String,
    pub source_id: String,
    pub source_title: String,
    pub source_summary: Option<String>,
    pub related_company_id: Option<Uuid>,
    pub related_person_id: Option<Uuid>,
    pub related_project_id: Option<Uuid>,
    pub coverage_score: f64,
    pub is_active: bool,
    pub is_stale: bool,
    pub last_refreshed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateUserKnowledgeSource {
    pub user_id: Uuid,
    pub source_type: String,
    pub source_id: String,
    pub source_title: String,
    pub source_summary: Option<String>,
    pub related_company_id: Option<Uuid>,
    pub related_person_id: Option<Uuid>,
    pub related_project_id: Option<Uuid>,
    pub coverage_score: Option<f64>,
}

impl UserKnowledgeSource {
    pub async fn find_by_user(
        pool: &SqlitePool,
        user_id: Uuid,
    ) -> Result<Vec<Self>, UserKnowledgeSourceError> {
        let rows = sqlx::query_as::<_, Self>(
            r#"SELECT * FROM user_knowledge_sources
               WHERE user_id = ?1 AND is_active = 1
               ORDER BY updated_at DESC"#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn find_by_user_and_type(
        pool: &SqlitePool,
        user_id: Uuid,
        source_type: &str,
    ) -> Result<Vec<Self>, UserKnowledgeSourceError> {
        let rows = sqlx::query_as::<_, Self>(
            r#"SELECT * FROM user_knowledge_sources
               WHERE user_id = ?1 AND source_type = ?2 AND is_active = 1
               ORDER BY coverage_score DESC"#,
        )
        .bind(user_id)
        .bind(source_type)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Upsert — inserts or updates by (user_id, source_type, source_id)
    pub async fn upsert(
        pool: &SqlitePool,
        data: CreateUserKnowledgeSource,
    ) -> Result<Self, UserKnowledgeSourceError> {
        let id = Uuid::new_v4();
        let coverage = data.coverage_score.unwrap_or(0.0);
        sqlx::query(
            r#"INSERT INTO user_knowledge_sources
               (id, user_id, source_type, source_id, source_title, source_summary,
                related_company_id, related_person_id, related_project_id, coverage_score)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
               ON CONFLICT(user_id, source_type, source_id) DO UPDATE SET
                 source_title     = excluded.source_title,
                 source_summary   = excluded.source_summary,
                 coverage_score   = excluded.coverage_score,
                 is_stale         = 0,
                 last_refreshed_at= datetime('now','subsec'),
                 updated_at       = datetime('now','subsec')"#,
        )
        .bind(id)
        .bind(data.user_id)
        .bind(&data.source_type)
        .bind(&data.source_id)
        .bind(&data.source_title)
        .bind(&data.source_summary)
        .bind(data.related_company_id)
        .bind(data.related_person_id)
        .bind(data.related_project_id)
        .bind(coverage)
        .execute(pool)
        .await?;

        let row = sqlx::query_as::<_, Self>(
            r#"SELECT * FROM user_knowledge_sources
               WHERE user_id = ?1 AND source_type = ?2 AND source_id = ?3"#,
        )
        .bind(data.user_id)
        .bind(&data.source_type)
        .bind(&data.source_id)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn mark_stale_for_user(
        pool: &SqlitePool,
        user_id: Uuid,
    ) -> Result<(), UserKnowledgeSourceError> {
        sqlx::query(
            r#"UPDATE user_knowledge_sources SET is_stale = 1, updated_at = datetime('now','subsec')
               WHERE user_id = ?1"#,
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
