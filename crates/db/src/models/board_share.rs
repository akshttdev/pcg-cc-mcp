use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BoardShare {
    pub id: String,
    pub board_id: String,
    pub source_organization_id: String,
    pub target_organization_id: String,
    pub permission: String,
    pub share_type: String,
    pub shared_by: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateBoardShare {
    pub board_id: String,
    pub target_organization_id: String,
    pub permission: Option<String>,
    pub share_type: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateBoardShare {
    pub permission: Option<String>,
    pub share_type: Option<String>,
    pub is_active: Option<bool>,
}

impl BoardShare {
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, BoardShare>(
            r#"SELECT id, board_id, source_organization_id, target_organization_id,
                      permission, share_type, shared_by, is_active, created_at, updated_at
               FROM board_shares WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_board(pool: &SqlitePool, board_id: &str) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, BoardShare>(
            r#"SELECT id, board_id, source_organization_id, target_organization_id,
                      permission, share_type, shared_by, is_active, created_at, updated_at
               FROM board_shares WHERE board_id = ? AND is_active = 1
               ORDER BY created_at ASC"#,
        )
        .bind(board_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_target_org(
        pool: &SqlitePool,
        target_org_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, BoardShare>(
            r#"SELECT id, board_id, source_organization_id, target_organization_id,
                      permission, share_type, shared_by, is_active, created_at, updated_at
               FROM board_shares WHERE target_organization_id = ? AND is_active = 1
               ORDER BY created_at ASC"#,
        )
        .bind(target_org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_source_org(
        pool: &SqlitePool,
        source_org_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, BoardShare>(
            r#"SELECT id, board_id, source_organization_id, target_organization_id,
                      permission, share_type, shared_by, is_active, created_at, updated_at
               FROM board_shares WHERE source_organization_id = ? AND is_active = 1
               ORDER BY created_at ASC"#,
        )
        .bind(source_org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_board_and_target(
        pool: &SqlitePool,
        board_id: &str,
        target_org_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, BoardShare>(
            r#"SELECT id, board_id, source_organization_id, target_organization_id,
                      permission, share_type, shared_by, is_active, created_at, updated_at
               FROM board_shares WHERE board_id = ? AND target_organization_id = ?"#,
        )
        .bind(board_id)
        .bind(target_org_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        id: &str,
        data: &CreateBoardShare,
        source_org_id: &str,
        shared_by: &str,
    ) -> Result<Self, sqlx::Error> {
        let permission = data.permission.as_deref().unwrap_or("editor");
        let share_type = data.share_type.as_deref().unwrap_or("collaboration");

        sqlx::query_as::<_, BoardShare>(
            r#"INSERT INTO board_shares (id, board_id, source_organization_id, target_organization_id,
                                         permission, share_type, shared_by)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               RETURNING id, board_id, source_organization_id, target_organization_id,
                         permission, share_type, shared_by, is_active, created_at, updated_at"#,
        )
        .bind(id)
        .bind(&data.board_id)
        .bind(source_org_id)
        .bind(&data.target_organization_id)
        .bind(permission)
        .bind(share_type)
        .bind(shared_by)
        .fetch_one(pool)
        .await
    }

    pub async fn update(pool: &SqlitePool, id: &str, data: &UpdateBoardShare) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let permission = data.permission.as_deref().unwrap_or(&existing.permission);
        let share_type = data.share_type.as_deref().unwrap_or(&existing.share_type);
        let is_active = data.is_active.unwrap_or(existing.is_active);

        sqlx::query_as::<_, BoardShare>(
            r#"UPDATE board_shares SET permission = ?, share_type = ?, is_active = ?,
                      updated_at = datetime('now')
               WHERE id = ?
               RETURNING id, board_id, source_organization_id, target_organization_id,
                         permission, share_type, shared_by, is_active, created_at, updated_at"#,
        )
        .bind(permission)
        .bind(share_type)
        .bind(is_active)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM board_shares WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Check if a user has access to a board via any board_share + org membership
    /// Returns the permission level if access is granted
    pub async fn check_user_share_access(
        pool: &SqlitePool,
        board_id: &str,
        user_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        #[derive(FromRow)]
        struct PermRow {
            permission: String,
        }

        let result: Option<PermRow> = sqlx::query_as(
            r#"SELECT bs.permission
               FROM board_shares bs
               INNER JOIN organization_members om
                   ON om.organization_id = bs.target_organization_id AND om.user_id = ?
               WHERE bs.board_id = ? AND bs.is_active = 1
               LIMIT 1"#,
        )
        .bind(user_id)
        .bind(board_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.permission))
    }
}
