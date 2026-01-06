use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Represents a connection between a PCG project and an Airtable base
#[derive(Debug, Clone, Serialize, Deserialize, TS, FromRow)]
pub struct AirtableBase {
    pub id: Uuid,
    pub project_id: Uuid,
    pub airtable_base_id: String,
    pub airtable_base_name: Option<String>,
    pub sync_enabled: bool,
    pub default_table_id: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateAirtableBase {
    pub project_id: Uuid,
    pub airtable_base_id: String,
    pub airtable_base_name: Option<String>,
    pub default_table_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateAirtableBase {
    pub airtable_base_name: Option<String>,
    pub sync_enabled: Option<bool>,
    pub default_table_id: Option<String>,
}

impl AirtableBase {
    /// Find all Airtable base connections for a project
    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableBase,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                airtable_base_id,
                airtable_base_name,
                sync_enabled as "sync_enabled!: bool",
                default_table_id,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_bases
            WHERE project_id = ?
            ORDER BY created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find an Airtable base connection by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableBase,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                airtable_base_id,
                airtable_base_name,
                sync_enabled as "sync_enabled!: bool",
                default_table_id,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_bases
            WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find a connection by Airtable base ID
    pub async fn find_by_airtable_base_id(
        pool: &SqlitePool,
        airtable_base_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableBase,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                airtable_base_id,
                airtable_base_name,
                sync_enabled as "sync_enabled!: bool",
                default_table_id,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_bases
            WHERE airtable_base_id = ?"#,
            airtable_base_id
        )
        .fetch_optional(pool)
        .await
    }

    /// List all Airtable base connections
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableBase,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                airtable_base_id,
                airtable_base_name,
                sync_enabled as "sync_enabled!: bool",
                default_table_id,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_bases
            ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Create a new Airtable base connection
    pub async fn create(pool: &SqlitePool, data: CreateAirtableBase) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query!(
            r#"INSERT INTO airtable_bases (
                id, project_id, airtable_base_id, airtable_base_name,
                default_table_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            id,
            data.project_id,
            data.airtable_base_id,
            data.airtable_base_name,
            data.default_table_id,
            now,
            now
        )
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Update an Airtable base connection
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateAirtableBase,
    ) -> Result<Self, sqlx::Error> {
        let now = Utc::now();
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let airtable_base_name = data.airtable_base_name.or(current.airtable_base_name);
        let sync_enabled = data.sync_enabled.unwrap_or(current.sync_enabled);
        let default_table_id = data.default_table_id.or(current.default_table_id);

        sqlx::query!(
            r#"UPDATE airtable_bases SET
                airtable_base_name = ?,
                sync_enabled = ?,
                default_table_id = ?,
                updated_at = ?
            WHERE id = ?"#,
            airtable_base_name,
            sync_enabled,
            default_table_id,
            now,
            id
        )
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Mark a base as synced
    pub async fn mark_synced(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query!(
            "UPDATE airtable_bases SET last_synced_at = ?, updated_at = ? WHERE id = ?",
            now,
            now,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete an Airtable base connection
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM airtable_bases WHERE id = ?", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
