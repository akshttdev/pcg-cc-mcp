use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Simplified board types - just Default (auto-created) and Custom (user-created)
#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "project_board_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProjectBoardType {
    /// The main board auto-created for each project
    Default,
    /// User-created boards for specialized working groups
    Custom,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectBoard {
    pub id: Uuid,
    pub project_id: Uuid,
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
    pub project_id: Uuid,
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
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectBoard,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM project_boards
             WHERE project_id = $1
             ORDER BY created_at"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectBoard,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM project_boards
             WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_slug(
        pool: &SqlitePool,
        project_id: Uuid,
        slug: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectBoard,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM project_boards
             WHERE project_id = $1 AND slug = $2"#,
            project_id,
            slug
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateProjectBoard,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            ProjectBoard,
            r#"INSERT INTO project_boards
                (id, project_id, name, slug, board_type, description, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            payload.project_id,
            payload.name,
            payload.slug,
            payload.board_type,
            payload.description,
            payload.metadata
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateProjectBoard,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = match sqlx::query_as!(
            ProjectBoard,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM project_boards
             WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await?
        {
            Some(board) => board,
            None => return Ok(None),
        };

        let name = payload.name.clone().unwrap_or(existing.name);
        let slug = payload.slug.clone().unwrap_or(existing.slug);
        let board_type = payload.board_type.unwrap_or(existing.board_type);
        let description = payload.description.clone().unwrap_or(existing.description);
        let metadata = payload.metadata.clone().unwrap_or(existing.metadata);

        sqlx::query_as!(
            ProjectBoard,
            r#"UPDATE project_boards
                 SET name = $2,
                     slug = $3,
                     board_type = $4,
                     description = $5,
                     metadata = $6,
                     updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            slug,
            board_type,
            description,
            metadata
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM project_boards WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Creates a single "Main Board" for new projects
    pub async fn ensure_default_board(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        // Check if default board already exists
        let existing = sqlx::query_as!(
            ProjectBoard,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM project_boards
             WHERE project_id = $1 AND board_type = 'default'"#,
            project_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(board) = existing {
            return Ok(board);
        }

        // Create the default main board
        let board_id = Uuid::new_v4();
        let board_type = ProjectBoardType::Default;
        sqlx::query_as!(
            ProjectBoard,
            r#"INSERT INTO project_boards
                (id, project_id, name, slug, board_type, description)
               VALUES ($1, $2, 'Main Board', 'main', $3, 'Default project board for all tasks')
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                slug,
                board_type as "board_type!: ProjectBoardType",
                description,
                metadata,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            board_id,
            project_id,
            board_type
        )
        .fetch_one(pool)
        .await
    }

    /// Legacy compatibility - returns vec with single default board
    pub async fn ensure_default_boards(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let board = Self::ensure_default_board(pool, project_id).await?;
        Ok(vec![board])
    }
}
