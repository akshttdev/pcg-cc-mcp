// Soft delete service for safe record deletion with audit trail
use sqlx::SqlitePool;
use uuid::Uuid;

/// Service for handling soft delete operations with audit logging
pub struct SoftDeleteService;

impl SoftDeleteService {
    /// Soft delete a project (marks as deleted, does not remove data)
    pub async fn soft_delete_project(
        pool: &SqlitePool,
        project_id: Uuid,
        deleted_by: Option<Uuid>,
    ) -> Result<u64, sqlx::Error> {
        // Use a transaction to ensure atomicity
        let mut tx = pool.begin().await?;

        // Mark project as deleted
        let result = sqlx::query(
            r#"UPDATE projects
               SET deleted_at = datetime('now'), deleted_by = ?
               WHERE id = ? AND deleted_at IS NULL"#,
        )
        .bind(deleted_by.map(|u| u.as_bytes().to_vec()))
        .bind(project_id.as_bytes().to_vec())
        .execute(&mut *tx)
        .await?;

        // Log to deletion audit
        if result.rows_affected() > 0 {
            let audit_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO deletion_audit_log (id, table_name, record_id, deleted_by, reason)
                   VALUES (?, 'projects', ?, ?, 'soft_delete')"#,
            )
            .bind(audit_id.as_bytes().to_vec())
            .bind(project_id.as_bytes().to_vec())
            .bind(deleted_by.map(|u| u.as_bytes().to_vec()))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(result.rows_affected())
    }

    /// Soft delete a task (marks as deleted, does not remove data)
    pub async fn soft_delete_task(
        pool: &SqlitePool,
        task_id: Uuid,
        deleted_by: Option<Uuid>,
    ) -> Result<u64, sqlx::Error> {
        let mut tx = pool.begin().await?;

        let result = sqlx::query(
            r#"UPDATE tasks
               SET deleted_at = datetime('now'), deleted_by = ?
               WHERE id = ? AND deleted_at IS NULL"#,
        )
        .bind(deleted_by.map(|u| u.as_bytes().to_vec()))
        .bind(task_id.as_bytes().to_vec())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() > 0 {
            let audit_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO deletion_audit_log (id, table_name, record_id, deleted_by, reason)
                   VALUES (?, 'tasks', ?, ?, 'soft_delete')"#,
            )
            .bind(audit_id.as_bytes().to_vec())
            .bind(task_id.as_bytes().to_vec())
            .bind(deleted_by.map(|u| u.as_bytes().to_vec()))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(result.rows_affected())
    }

    /// Soft delete a user (for GDPR compliance)
    pub async fn soft_delete_user(
        pool: &SqlitePool,
        user_id: Uuid,
        deleted_by: Option<Uuid>,
    ) -> Result<u64, sqlx::Error> {
        let mut tx = pool.begin().await?;

        // Mark user as deleted and anonymize PII
        let result = sqlx::query(
            r#"UPDATE users
               SET deleted_at = datetime('now'),
                   deleted_by = ?,
                   email = 'deleted_' || hex(id) || '@deleted.local',
                   full_name = 'Deleted User',
                   avatar_url = NULL,
                   is_active = 0
               WHERE id = ? AND deleted_at IS NULL"#,
        )
        .bind(deleted_by.map(|u| u.as_bytes().to_vec()))
        .bind(user_id.as_bytes().to_vec())
        .execute(&mut *tx)
        .await?;

        // Invalidate all sessions for this user
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id.as_bytes().to_vec())
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() > 0 {
            let audit_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO deletion_audit_log (id, table_name, record_id, deleted_by, reason)
                   VALUES (?, 'users', ?, ?, 'soft_delete_gdpr')"#,
            )
            .bind(audit_id.as_bytes().to_vec())
            .bind(user_id.as_bytes().to_vec())
            .bind(deleted_by.map(|u| u.as_bytes().to_vec()))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(result.rows_affected())
    }

    /// Restore a soft-deleted project
    pub async fn restore_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE projects
               SET deleted_at = NULL, deleted_by = NULL
               WHERE id = ? AND deleted_at IS NOT NULL"#,
        )
        .bind(project_id.as_bytes().to_vec())
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Restore a soft-deleted task
    pub async fn restore_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE tasks
               SET deleted_at = NULL, deleted_by = NULL
               WHERE id = ? AND deleted_at IS NOT NULL"#,
        )
        .bind(task_id.as_bytes().to_vec())
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

/// Transaction helper for atomic operations
pub struct TransactionHelper;

impl TransactionHelper {
    /// Execute a budget deduction atomically
    pub async fn deduct_vibe_budget(
        pool: &SqlitePool,
        project_id: Uuid,
        amount: i64,
        description: &str,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = pool.begin().await?;

        // Check current budget
        let project: Option<(Option<i64>, i64)> = sqlx::query_as(
            "SELECT vibe_budget_limit, COALESCE(vibe_spent_amount, 0) FROM projects WHERE id = ?",
        )
        .bind(project_id.as_bytes().to_vec())
        .fetch_optional(&mut *tx)
        .await?;

        let (limit, spent) = match project {
            Some((limit, spent)) => (limit, spent),
            None => {
                tx.rollback().await?;
                return Ok(false);
            }
        };

        // Check if within budget
        if let Some(budget_limit) = limit {
            if spent + amount > budget_limit {
                tx.rollback().await?;
                return Ok(false); // Over budget
            }
        }

        // Update spent amount
        sqlx::query(
            "UPDATE projects SET vibe_spent_amount = COALESCE(vibe_spent_amount, 0) + ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(amount)
        .bind(project_id.as_bytes().to_vec())
        .execute(&mut *tx)
        .await?;

        // Record transaction
        let tx_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO vibe_transactions (id, source_type, source_id, project_id, amount_vibe, balance_after, description)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(tx_id.as_bytes().to_vec())
        .bind(source_type)
        .bind(source_id.as_bytes().to_vec())
        .bind(project_id.as_bytes().to_vec())
        .bind(amount)
        .bind(spent + amount)
        .bind(description)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }
}
