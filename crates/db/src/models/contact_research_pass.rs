use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct ContactResearchPass {
    pub id: DbUuid,
    pub crm_contact_id: DbUuid,
    pub pass_number: i64,
    pub research_focus: String,
    pub focus_prompt: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub raw_results: Option<String>,
    pub key_findings: String,
    pub search_queries: String,
    pub confidence_delta: f64,
    pub agent_used: Option<String>,
    pub tokens_used: Option<i64>,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

impl ContactResearchPass {
    pub async fn create(
        pool: &sqlx::SqlitePool,
        crm_contact_id: &DbUuid,
        pass_number: i64,
        research_focus: &str,
        focus_prompt: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        sqlx::query_as::<_, Self>(
            "INSERT INTO contact_research_passes
             (id, crm_contact_id, pass_number, research_focus, focus_prompt, status)
             VALUES (?, ?, ?, ?, ?, 'pending')
             RETURNING *",
        )
        .bind(&id)
        .bind(crm_contact_id)
        .bind(pass_number)
        .bind(research_focus)
        .bind(focus_prompt)
        .fetch_one(pool)
        .await
    }

    pub async fn list_for_contact(
        pool: &sqlx::SqlitePool,
        crm_contact_id: &DbUuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM contact_research_passes WHERE crm_contact_id = ? ORDER BY pass_number ASC",
        )
        .bind(crm_contact_id)
        .fetch_all(pool)
        .await
    }

    pub async fn next_pass_number(pool: &sqlx::SqlitePool, crm_contact_id: &DbUuid) -> i64 {
        #[derive(sqlx::FromRow)]
        struct Row {
            n: Option<i64>,
        }

        sqlx::query_as::<_, Row>(
            "SELECT MAX(pass_number) AS n FROM contact_research_passes WHERE crm_contact_id = ?",
        )
        .bind(crm_contact_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.n)
        .map(|n| n + 1)
        .unwrap_or(1)
    }
}
