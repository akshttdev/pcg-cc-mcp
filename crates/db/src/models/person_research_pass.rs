use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// NOTE: This file uses uuid::Uuid for backward compatibility with callers.
// Will be converted to DbUuid in Phase 2 (contacts unification) when
// intelligence.rs and persons.rs are rewritten.

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PersonResearchPass {
    pub id: Uuid,
    pub person_id: Uuid,
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

impl PersonResearchPass {
    pub async fn create(
        pool: &sqlx::SqlitePool,
        person_id: Uuid,
        pass_number: i64,
        research_focus: &str,
        focus_prompt: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Self>(
            "INSERT INTO person_research_passes
             (id, person_id, pass_number, research_focus, focus_prompt, status)
             VALUES (?, ?, ?, ?, ?, 'pending')
             RETURNING *",
        )
        .bind(id)
        .bind(person_id)
        .bind(pass_number)
        .bind(research_focus)
        .bind(focus_prompt)
        .fetch_one(pool)
        .await
    }

    pub async fn list_for_person(
        pool: &sqlx::SqlitePool,
        person_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM person_research_passes WHERE person_id = ? ORDER BY pass_number ASC",
        )
        .bind(person_id)
        .fetch_all(pool)
        .await
    }

    pub async fn next_pass_number(pool: &sqlx::SqlitePool, person_id: Uuid) -> i64 {
        #[derive(sqlx::FromRow)]
        struct Row {
            n: Option<i64>,
        }

        sqlx::query_as::<_, Row>(
            "SELECT MAX(pass_number) AS n FROM person_research_passes WHERE person_id = ?",
        )
        .bind(person_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.n)
        .map(|n| n + 1)
        .unwrap_or(1)
    }
}
