use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectFolder {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub client_id: Option<Uuid>,
    pub name: String,
    pub sort_order: i32,
    pub is_active: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectFolder {
    pub name: String,
    pub client_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectFolder {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

impl ProjectFolder {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProjectFolder>(
            r#"SELECT id, organization_id, client_id, name, sort_order, is_active, created_at, updated_at
               FROM project_folders WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProjectFolder>(
            r#"SELECT id, organization_id, client_id, name, sort_order, is_active, created_at, updated_at
               FROM project_folders WHERE organization_id = ? AND is_active = 1
               ORDER BY sort_order ASC, name ASC"#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_client(
        pool: &SqlitePool,
        client_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProjectFolder>(
            r#"SELECT id, organization_id, client_id, name, sort_order, is_active, created_at, updated_at
               FROM project_folders WHERE client_id = ? AND is_active = 1
               ORDER BY sort_order ASC, name ASC"#,
        )
        .bind(client_id)
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        id: Uuid,
        organization_id: Uuid,
        data: &CreateProjectFolder,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, ProjectFolder>(
            r#"INSERT INTO project_folders (id, organization_id, client_id, name)
               VALUES (?, ?, ?, ?)
               RETURNING id, organization_id, client_id, name, sort_order, is_active, created_at, updated_at"#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(data.client_id)
        .bind(&data.name)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateProjectFolder,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_deref().unwrap_or(&existing.name);
        let sort_order = data.sort_order.unwrap_or(existing.sort_order);
        let is_active = data.is_active.unwrap_or(existing.is_active);

        sqlx::query_as::<_, ProjectFolder>(
            r#"UPDATE project_folders SET name = ?, sort_order = ?, is_active = ?,
                      updated_at = datetime('now')
               WHERE id = ?
               RETURNING id, organization_id, client_id, name, sort_order, is_active, created_at, updated_at"#,
        )
        .bind(name)
        .bind(sort_order)
        .bind(is_active)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM project_folders WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
