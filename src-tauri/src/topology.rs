// ═══════════════════════════════════════════════════════════════════════════
// PHASE 2: TOPOLOGY-AWARE ACCESS CONTROL
// Multi-tenant privacy enforcement through topological security
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::fmt;

/// Access levels for resources
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessLevel {
    None,
    Viewer,
    Contributor,
    Editor,
    Owner,
}

impl fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessLevel::None => write!(f, "none"),
            AccessLevel::Viewer => write!(f, "viewer"),
            AccessLevel::Contributor => write!(f, "contributor"),
            AccessLevel::Editor => write!(f, "editor"),
            AccessLevel::Owner => write!(f, "owner"),
        }
    }
}

impl AccessLevel {
    /// Check if this access level can read
    pub fn can_read(&self) -> bool {
        !matches!(self, AccessLevel::None)
    }

    /// Check if this access level can write
    pub fn can_write(&self) -> bool {
        matches!(self, AccessLevel::Contributor | AccessLevel::Editor | AccessLevel::Owner)
    }

    /// Check if this access level can modify
    pub fn can_modify(&self) -> bool {
        matches!(self, AccessLevel::Editor | AccessLevel::Owner)
    }

    /// Check if this access level has full control
    pub fn is_owner(&self) -> bool {
        matches!(self, AccessLevel::Owner)
    }
}

/// Topology-aware access control service
pub struct TopologyAccessControl;

impl TopologyAccessControl {
    /// Get topology-aware WHERE clause for SQL queries
    ///
    /// This clause ensures users can only see:
    /// 1. Projects they own
    /// 2. Projects explicitly shared with them via project_members
    ///
    /// # Example
    /// ```sql
    /// SELECT p.* FROM projects p
    /// WHERE p.deleted_at IS NULL
    ///   AND (p.owner_id = :user_id
    ///        OR EXISTS (SELECT 1 FROM project_members pm
    ///                   WHERE pm.project_id = p.id
    ///                     AND pm.user_id = :user_id))
    /// ```
    pub fn get_project_filter_clause() -> &'static str {
        r#"
        AND (
            p.owner_id = :user_id
            OR EXISTS (
                SELECT 1 FROM project_members pm
                WHERE pm.project_id = p.id
                  AND pm.user_id = :user_id
            )
        )
        "#
    }

    /// Get topology-aware WHERE clause for tasks
    pub fn get_task_filter_clause() -> &'static str {
        r#"
        AND EXISTS (
            SELECT 1 FROM projects p
            WHERE t.project_id = p.id
              AND p.deleted_at IS NULL
              AND (
                  p.owner_id = :user_id
                  OR EXISTS (
                      SELECT 1 FROM project_members pm
                      WHERE pm.project_id = p.id
                        AND pm.user_id = :user_id
                  )
              )
        )
        "#
    }

    /// Check if user has access to a project
    ///
    /// Returns AccessLevel based on:
    /// 1. If user owns project → Owner
    /// 2. If user is in project_members → Role from table
    /// 3. Otherwise → None
    pub async fn check_project_access(
        db: &rusqlite::Connection,
        user_id: &[u8],
        project_id: &[u8],
    ) -> Result<AccessLevel, rusqlite::Error> {
        // Check if user owns the project
        let mut stmt = db.prepare(
            "SELECT 1 FROM projects WHERE id = ?1 AND owner_id = ?2 AND deleted_at IS NULL"
        )?;

        let is_owner = stmt.exists([project_id, user_id])?;

        if is_owner {
            return Ok(AccessLevel::Owner);
        }

        // Check if project is shared with user
        let mut stmt = db.prepare(
            "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2"
        )?;

        let role: Option<String> = stmt
            .query_row([project_id, user_id], |row| row.get(0))
            .optional()?;

        match role.as_deref() {
            Some("editor") => Ok(AccessLevel::Editor),
            Some("contributor") => Ok(AccessLevel::Contributor),
            Some("viewer") => Ok(AccessLevel::Viewer),
            Some("owner") => Ok(AccessLevel::Owner),
            _ => Ok(AccessLevel::None),
        }
    }

    /// Check if user can perform an action on a project
    pub async fn can_perform_action(
        db: &rusqlite::Connection,
        user_id: &[u8],
        project_id: &[u8],
        action: &str,
    ) -> Result<bool, rusqlite::Error> {
        let access = Self::check_project_access(db, user_id, project_id).await?;

        let can_perform = match action {
            "read" | "view" => access.can_read(),
            "create" | "write" | "contribute" => access.can_write(),
            "update" | "modify" | "delete" => access.can_modify(),
            "admin" | "manage" | "share" => access.is_owner(),
            _ => false,
        };

        Ok(can_perform)
    }

    /// Share a project with a user
    pub async fn share_project(
        db: &rusqlite::Connection,
        project_id: &[u8],
        user_id: &[u8],
        role: &str,
        granted_by: &[u8],
    ) -> Result<(), rusqlite::Error> {
        // Verify grantor has permission to share
        let grantor_access = Self::check_project_access(db, granted_by, project_id).await?;

        if !grantor_access.is_owner() {
            return Err(rusqlite::Error::InvalidQuery);
        }

        // Validate role
        if !["editor", "contributor", "viewer"].contains(&role) {
            return Err(rusqlite::Error::InvalidParameterName("Invalid role".to_string()));
        }

        // Create membership
        let membership_id = uuid::Uuid::new_v4().as_bytes().to_vec();

        db.execute(
            r#"
            INSERT OR REPLACE INTO project_members (
                id, project_id, user_id, role, granted_by
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            [
                &membership_id[..],
                project_id,
                user_id,
                role.as_bytes(),
                granted_by,
            ],
        )?;

        Ok(())
    }

    /// Revoke project access from a user
    pub async fn revoke_project_access(
        db: &rusqlite::Connection,
        project_id: &[u8],
        user_id: &[u8],
        revoked_by: &[u8],
    ) -> Result<(), rusqlite::Error> {
        // Verify revoker has permission
        let revoker_access = Self::check_project_access(db, revoked_by, project_id).await?;

        if !revoker_access.is_owner() {
            return Err(rusqlite::Error::InvalidQuery);
        }

        db.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            [project_id, user_id],
        )?;

        Ok(())
    }

    /// Get all users with access to a project
    pub async fn get_project_members(
        db: &rusqlite::Connection,
        project_id: &[u8],
    ) -> Result<Vec<(Vec<u8>, String)>, rusqlite::Error> {
        let mut stmt = db.prepare(
            r#"
            SELECT pm.user_id, pm.role, u.username
            FROM project_members pm
            JOIN users u ON pm.user_id = u.id
            WHERE pm.project_id = ?1
            ORDER BY pm.granted_at DESC
            "#,
        )?;

        let members = stmt
            .query_map([project_id], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(members)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SQL QUERY HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Build topology-aware project query
pub fn build_project_query(user_id: &[u8], additional_where: Option<&str>) -> String {
    let filter_clause = TopologyAccessControl::get_project_filter_clause();

    let base_query = format!(
        r#"
        SELECT p.*
        FROM projects p
        WHERE p.deleted_at IS NULL
        {}
        {}
        ORDER BY p.updated_at DESC
        "#,
        filter_clause,
        additional_where.unwrap_or("")
    );

    base_query
}

/// Build topology-aware task query
pub fn build_task_query(user_id: &[u8], additional_where: Option<&str>) -> String {
    let filter_clause = TopologyAccessControl::get_task_filter_clause();

    let base_query = format!(
        r#"
        SELECT t.*
        FROM tasks t
        WHERE t.deleted_at IS NULL
        {}
        {}
        ORDER BY t.created_at DESC
        "#,
        filter_clause,
        additional_where.unwrap_or("")
    );

    base_query
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_level_permissions() {
        assert!(!AccessLevel::None.can_read());
        assert!(AccessLevel::Viewer.can_read());
        assert!(!AccessLevel::Viewer.can_write());
        assert!(AccessLevel::Editor.can_write());
        assert!(AccessLevel::Editor.can_modify());
        assert!(AccessLevel::Owner.is_owner());
    }

    #[test]
    fn test_filter_clauses() {
        let project_filter = TopologyAccessControl::get_project_filter_clause();
        assert!(project_filter.contains("owner_id = :user_id"));
        assert!(project_filter.contains("project_members"));

        let task_filter = TopologyAccessControl::get_task_filter_clause();
        assert!(task_filter.contains("project_id"));
        assert!(task_filter.contains("project_members"));
    }
}
