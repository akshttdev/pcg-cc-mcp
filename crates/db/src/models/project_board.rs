use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Board types for project boards
#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "project_board_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProjectBoardType {
    /// The main board auto-created for each project
    Default,
    /// User-created boards for specialized working groups
    Custom,
    /// Brand asset tracking board
    BrandAssets,
    /// Executive-level asset tracking board
    ExecutiveAssets,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectBoard {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub slug: String,
    pub board_type: ProjectBoardType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectBoard {
    pub project_id: String,
    pub name: String,
    pub slug: String,
    pub board_type: ProjectBoardType,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectBoard {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub board_type: Option<ProjectBoardType>,
    #[serde(default)]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub metadata: Option<Option<String>>,
}

impl ProjectBoard {
    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProjectBoard>(
            r#"SELECT id, project_id, name, slug, board_type, description, metadata,
                      created_at, updated_at
              FROM project_boards
             WHERE project_id = ?
             ORDER BY created_at"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProjectBoard>(
            r#"SELECT id, project_id, name, slug, board_type, description, metadata,
                      created_at, updated_at
              FROM project_boards
             WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_slug(
        pool: &SqlitePool,
        project_id: &str,
        slug: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProjectBoard>(
            r#"SELECT id, project_id, name, slug, board_type, description, metadata,
                      created_at, updated_at
              FROM project_boards
             WHERE project_id = ? AND slug = ?"#,
        )
        .bind(project_id)
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateProjectBoard,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query_as::<_, ProjectBoard>(
            r#"INSERT INTO project_boards
                (id, project_id, name, slug, board_type, description, metadata)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               RETURNING id, project_id, name, slug, board_type, description, metadata,
                         created_at, updated_at"#,
        )
        .bind(&id)
        .bind(&payload.project_id)
        .bind(&payload.name)
        .bind(&payload.slug)
        .bind(payload.board_type)
        .bind(&payload.description)
        .bind(&payload.metadata)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        payload: &UpdateProjectBoard,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = match Self::find_by_id(pool, id).await? {
            Some(board) => board,
            None => return Ok(None),
        };

        let name = payload.name.clone().unwrap_or(existing.name);
        let slug = payload.slug.clone().unwrap_or(existing.slug);
        let board_type = payload.board_type.unwrap_or(existing.board_type);
        let description = payload.description.clone().unwrap_or(existing.description);
        let metadata = payload.metadata.clone().unwrap_or(existing.metadata);

        sqlx::query_as::<_, ProjectBoard>(
            r#"UPDATE project_boards
                 SET name = ?,
                     slug = ?,
                     board_type = ?,
                     description = ?,
                     metadata = ?,
                     updated_at = datetime('now', 'subsec')
               WHERE id = ?
               RETURNING id, project_id, name, slug, board_type, description, metadata,
                         created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(board_type)
        .bind(&description)
        .bind(&metadata)
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM project_boards WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Creates a single "Main Board" for new projects
    pub async fn ensure_default_board(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Self, sqlx::Error> {
        // Check if default board already exists
        let existing = sqlx::query_as::<_, ProjectBoard>(
            r#"SELECT id, project_id, name, slug, board_type, description, metadata,
                      created_at, updated_at
              FROM project_boards
             WHERE project_id = ? AND board_type = 'default'"#,
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await?;

        if let Some(board) = existing {
            return Ok(board);
        }

        // Create the default main board
        let board_id = Uuid::new_v4().to_string();
        let board_type = ProjectBoardType::Default;
        sqlx::query_as::<_, ProjectBoard>(
            r#"INSERT INTO project_boards
                (id, project_id, name, slug, board_type, description)
               VALUES (?, ?, 'Main Board', 'main', ?, 'Default project board for all tasks')
               RETURNING id, project_id, name, slug, board_type, description, metadata,
                         created_at, updated_at"#,
        )
        .bind(&board_id)
        .bind(project_id)
        .bind(board_type)
        .fetch_one(pool)
        .await
    }

    /// Legacy compatibility - returns vec with single default board
    pub async fn ensure_default_boards(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let board = Self::ensure_default_board(pool, project_id).await?;
        Ok(vec![board])
    }
}
