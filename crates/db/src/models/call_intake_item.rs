use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CallIntakeItem {
    pub id: Uuid,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub raw_content: Option<String>,
    pub subject: Option<String>,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub call_date: Option<String>,
    pub duration_seconds: Option<i64>,
    pub status: String,
    pub processed_at: Option<String>,
    pub error: Option<String>,
    pub person_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
    pub extracted_participants: String,
    pub extracted_topics: String,
    pub extracted_action_items: String,
    pub extracted_pain_points: String,
    pub extracted_sentiment: Option<String>,
    pub call_summary: Option<String>,
    pub report_id: Option<Uuid>,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
    pub extracted_individuals: Option<String>,
    pub extracted_businesses: Option<String>,
    pub crm_deal_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCallIntakeItem {
    pub source_type: String,
    pub source_ref: Option<String>,
    pub raw_content: Option<String>,
    pub subject: Option<String>,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub call_date: Option<String>,
    pub duration_seconds: Option<i64>,
    pub metadata: Option<String>,
}

impl CallIntakeItem {
    pub async fn create(
        pool: &sqlx::SqlitePool,
        data: CreateCallIntakeItem,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Self>(
            "INSERT INTO call_intake_items
             (id, source_type, source_ref, raw_content, subject, from_email, from_name,
              call_date, duration_seconds, status, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?)
             RETURNING *",
        )
        .bind(id)
        .bind(&data.source_type)
        .bind(&data.source_ref)
        .bind(&data.raw_content)
        .bind(&data.subject)
        .bind(&data.from_email)
        .bind(&data.from_name)
        .bind(&data.call_date)
        .bind(data.duration_seconds)
        .bind(data.metadata.as_deref().unwrap_or("{}"))
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &sqlx::SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM call_intake_items WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM call_intake_items ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_by_person(
        pool: &sqlx::SqlitePool,
        person_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM call_intake_items WHERE person_id = ? ORDER BY created_at DESC",
        )
        .bind(person_id)
        .fetch_all(pool)
        .await
    }
}
