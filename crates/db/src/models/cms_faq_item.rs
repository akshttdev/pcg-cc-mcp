use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CmsFaqItemError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("FAQ item not found")]
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CmsFaqItem {
    pub id: Uuid,
    pub site_id: Uuid,
    pub category: Option<String>,
    pub question: String,
    pub answer: String,
    pub sort_order: i64,
    pub is_active: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCmsFaqItem {
    pub category: Option<String>,
    pub question: String,
    pub answer: String,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCmsFaqItem {
    pub category: Option<String>,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub sort_order: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ReorderFaqItems {
    pub item_ids: Vec<Uuid>,
}

impl CmsFaqItem {
    pub async fn find_by_site(pool: &SqlitePool, site_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsFaqItem,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                category,
                question,
                answer,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_faq_items
            WHERE site_id = $1
            ORDER BY sort_order ASC"#,
            site_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_active_by_site(pool: &SqlitePool, site_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsFaqItem,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                category,
                question,
                answer,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_faq_items
            WHERE site_id = $1 AND is_active = 1
            ORDER BY sort_order ASC"#,
            site_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CmsFaqItem,
            r#"SELECT
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                category,
                question,
                answer,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cms_faq_items
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        site_id: Uuid,
        data: &CreateCmsFaqItem,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let sort_order = data.sort_order.unwrap_or(0);

        sqlx::query_as!(
            CmsFaqItem,
            r#"INSERT INTO cms_faq_items (id, site_id, category, question, answer, sort_order)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                category,
                question,
                answer,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            site_id,
            data.category,
            data.question,
            data.answer,
            sort_order
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateCmsFaqItem,
    ) -> Result<Self, sqlx::Error> {
        let current = Self::find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;

        let category = data.category.as_ref().or(current.category.as_ref());
        let question = data.question.as_ref().unwrap_or(&current.question);
        let answer = data.answer.as_ref().unwrap_or(&current.answer);
        let sort_order = data.sort_order.unwrap_or(current.sort_order);
        let is_active = data.is_active.unwrap_or(current.is_active);

        sqlx::query_as!(
            CmsFaqItem,
            r#"UPDATE cms_faq_items SET
                category = $2, question = $3, answer = $4, sort_order = $5, is_active = $6,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                site_id as "site_id!: Uuid",
                category,
                question,
                answer,
                sort_order as "sort_order!: i64",
                is_active as "is_active!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            category,
            question,
            answer,
            sort_order,
            is_active
        )
        .fetch_one(pool)
        .await
    }

    pub async fn reorder(pool: &SqlitePool, item_ids: &[Uuid]) -> Result<(), sqlx::Error> {
        for (index, item_id) in item_ids.iter().enumerate() {
            let order = index as i64;
            sqlx::query!(
                "UPDATE cms_faq_items SET sort_order = $2, updated_at = datetime('now', 'subsec') WHERE id = $1",
                item_id,
                order
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM cms_faq_items WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
