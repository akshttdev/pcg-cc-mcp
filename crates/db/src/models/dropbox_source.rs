use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, FromRow)]
pub struct DropboxSource {
    pub id: Uuid,
    pub account_id: String,
    pub label: String,
    pub source_url: Option<String>,
    pub project_id: Option<Uuid>,
    pub storage_tier: String,
    pub checksum_required: bool,
    pub reference_name_template: Option<String>,
    pub ingest_strategy: String,
    pub access_token: Option<String>,
    pub cursor: Option<String>,
    pub last_processed_at: Option<DateTime<Utc>>,
    pub auto_ingest: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DropboxSource {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            DropboxSource,
            r#"SELECT id as "id!: Uuid", account_id, label, source_url, project_id as "project_id?: Uuid", storage_tier, checksum_required, reference_name_template, ingest_strategy, access_token, cursor, last_processed_at as "last_processed_at?: DateTime<Utc>", auto_ingest, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>" FROM dropbox_sources ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            DropboxSource,
            r#"SELECT id as "id!: Uuid", account_id, label, source_url, project_id as "project_id?: Uuid", storage_tier, checksum_required, reference_name_template, ingest_strategy, access_token, cursor, last_processed_at as "last_processed_at?: DateTime<Utc>", auto_ingest, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>" FROM dropbox_sources WHERE id = ?"#,
            id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_accounts(
        pool: &SqlitePool,
        accounts: &[String],
    ) -> Result<Vec<Self>, sqlx::Error> {
        if accounts.is_empty() {
            return Ok(vec![]);
        }

        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT id as \"id!: Uuid\", account_id, label, source_url, project_id as \"project_id?: Uuid\", storage_tier, checksum_required, reference_name_template, ingest_strategy, access_token, cursor, last_processed_at as \"last_processed_at?: DateTime<Utc>\", auto_ingest, created_at as \"created_at!: DateTime<Utc>\", updated_at as \"updated_at!: DateTime<Utc>\" FROM dropbox_sources WHERE account_id IN (",
        );

        builder.push_tuples(accounts.iter(), |mut b, account| {
            b.push_bind(account);
        });
        builder.push(")");

        builder
            .build_query_as::<DropboxSource>()
            .fetch_all(pool)
            .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM dropbox_sources WHERE id = ?", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn create(
        pool: &SqlitePool,
        payload: CreateDropboxSource,
    ) -> Result<Self, sqlx::Error> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO dropbox_sources (
                id, account_id, label, source_url, project_id, storage_tier,
                checksum_required, reference_name_template, ingest_strategy,
                access_token, cursor, last_processed_at, auto_ingest, created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            payload.account_id,
            payload.label,
            payload.source_url,
            payload.project_id,
            payload.storage_tier,
            payload.checksum_required,
            payload.reference_name_template,
            payload.ingest_strategy,
            payload.access_token,
            payload.cursor,
            payload.last_processed_at,
            payload.auto_ingest,
            now,
            now,
        )
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id).await
    }

    pub async fn mark_processed(
        pool: &SqlitePool,
        id: Uuid,
        cursor: Option<String>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE dropbox_sources SET cursor = ?, last_processed_at = ?, updated_at = ? WHERE id = ?",
            cursor,
            timestamp,
            timestamp,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub fn reference_name(&self) -> String {
        render_reference_name(self.reference_name_template.as_deref(), &self.label)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateDropboxSource {
    pub account_id: String,
    pub label: String,
    pub source_url: Option<String>,
    pub project_id: Option<Uuid>,
    pub storage_tier: String,
    #[serde(default = "default_true")]
    pub checksum_required: bool,
    pub reference_name_template: Option<String>,
    #[serde(default = "default_shared_link")]
    pub ingest_strategy: String,
    pub access_token: Option<String>,
    pub cursor: Option<String>,
    pub last_processed_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    pub auto_ingest: bool,
}

fn default_true() -> bool {
    true
}

fn default_shared_link() -> String {
    "shared_link".to_string()
}

pub fn render_reference_name(template: Option<&str>, label: &str) -> String {
    let now = Utc::now();
    if let Some(tpl) = template {
        let mut rendered = tpl.replace("{date}", &now.format("%Y-%m-%d").to_string());
        rendered = rendered.replace(
            "{datetime}",
            &now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        );
        rendered = rendered.replace("{label}", label);
        if rendered.trim().is_empty() {
            label.to_string()
        } else {
            rendered
        }
    } else {
        label.to_string()
    }
}
