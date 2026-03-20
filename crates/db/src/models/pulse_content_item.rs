use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PulseContentStatus {
    New,
    Enriched,
    Alerted,
    Reviewed,
    Tasked,
    Actioned,
    Dismissed,
}

impl std::fmt::Display for PulseContentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::New => "new",
            Self::Enriched => "enriched",
            Self::Alerted => "alerted",
            Self::Reviewed => "reviewed",
            Self::Tasked => "tasked",
            Self::Actioned => "actioned",
            Self::Dismissed => "dismissed",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseContentItem {
    pub id: String,
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source_id: String,
    pub source_type: String,
    pub content_hash: String,
    pub url: String,
    pub title: String,
    pub body: Option<String>,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub relevance_score: Option<f64>,
    pub extracted_entities: Option<String>,
    pub pcg_status: String,
    pub linked_task_id: Option<Uuid>,
    pub linked_entity_id: Option<String>,
    pub linked_crm_contact_id: Option<Uuid>,
    pub enrichment_vibe_cost: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePulseContentItem {
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source_id: String,
    pub source_type: String,
    pub content_hash: String,
    pub url: String,
    pub title: String,
    pub body: Option<String>,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub relevance_score: Option<f64>,
    pub extracted_entities: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ContentQuery {
    pub project_id: Option<Uuid>,
    pub keyword: Option<String>,
    pub source_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ContentActionRequest {
    pub action: String,
    pub task_title: Option<String>,
    pub task_description: Option<String>,
    pub crm_contact_id: Option<Uuid>,
}

impl PulseContentItem {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_content_items WHERE project_id = ? ORDER BY collected_at DESC LIMIT ? OFFSET ?",
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    pub async fn find_latest(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_content_items WHERE project_id = ? ORDER BY collected_at DESC LIMIT ?",
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM pulse_content_items WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn search(
        pool: &SqlitePool,
        project_id: Uuid,
        keyword: Option<&str>,
        source_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let mut sql = String::from("SELECT * FROM pulse_content_items WHERE project_id = ?");
        if keyword.is_some() {
            sql.push_str(" AND (title LIKE '%' || ? || '%' OR body LIKE '%' || ? || '%')");
        }
        if source_id.is_some() {
            sql.push_str(" AND source_id = ?");
        }
        if status.is_some() {
            sql.push_str(" AND pcg_status = ?");
        }
        sql.push_str(" ORDER BY collected_at DESC LIMIT ?");

        let mut query = sqlx::query_as::<_, Self>(&sql).bind(project_id);
        if let Some(kw) = keyword {
            query = query.bind(kw).bind(kw);
        }
        if let Some(sid) = source_id {
            query = query.bind(sid);
        }
        if let Some(st) = status {
            query = query.bind(st);
        }
        query = query.bind(limit);
        query.fetch_all(pool).await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreatePulseContentItem,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let entities_json = data.extracted_entities.as_ref().map(|v| v.to_string());

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO pulse_content_items
               (id, project_id, organization_id, source_id, source_type, content_hash, url, title, body, summary, author, published_at, collected_at, relevance_score, extracted_entities)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(data.project_id)
        .bind(data.organization_id)
        .bind(&data.source_id)
        .bind(&data.source_type)
        .bind(&data.content_hash)
        .bind(&data.url)
        .bind(&data.title)
        .bind(&data.body)
        .bind(&data.summary)
        .bind(&data.author)
        .bind(&data.published_at)
        .bind(&data.collected_at)
        .bind(data.relevance_score)
        .bind(&entities_json)
        .fetch_one(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE pulse_content_items SET pcg_status = ?, updated_at = datetime('now', 'subsec')
               WHERE id = ? RETURNING *"#,
        )
        .bind(status)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn link_task(
        pool: &SqlitePool,
        id: &str,
        task_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE pulse_content_items SET linked_task_id = ?, pcg_status = 'tasked', updated_at = datetime('now', 'subsec')
               WHERE id = ? RETURNING *"#,
        )
        .bind(task_id)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn link_crm_contact(
        pool: &SqlitePool,
        id: &str,
        contact_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE pulse_content_items SET linked_crm_contact_id = ?, updated_at = datetime('now', 'subsec')
               WHERE id = ? RETURNING *"#,
        )
        .bind(contact_id)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn count_by_project(pool: &SqlitePool, project_id: Uuid) -> Result<i64, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pulse_content_items WHERE project_id = ?")
                .bind(project_id)
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }

    pub async fn exists_by_hash(
        pool: &SqlitePool,
        project_id: Uuid,
        content_hash: &str,
    ) -> Result<bool, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pulse_content_items WHERE project_id = ? AND content_hash = ?",
        )
        .bind(project_id)
        .bind(content_hash)
        .fetch_one(pool)
        .await?;
        Ok(row.0 > 0)
    }
}
