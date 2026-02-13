//! Access Control for Topsi
//!
//! Implements containerized access control to ensure:
//! - Client data isolation (users only see their projects)
//! - Admin full visibility (master credential)
//! - No cross-client data leakage

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::{Result, TopsiError};

/// User context for access control decisions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UserContext {
    /// User's unique identifier
    pub user_id: String,
    /// Whether the user is an admin (master credential)
    pub is_admin: bool,
    /// User's email (for audit logging)
    pub email: Option<String>,
    /// Session ID for request tracking
    pub session_id: String,
}

impl UserContext {
    /// Create a new user context
    pub fn new(user_id: impl Into<String>, is_admin: bool) -> Self {
        Self {
            user_id: user_id.into(),
            is_admin,
            email: None,
            session_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create an admin context
    pub fn admin(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            is_admin: true,
            email: None,
            session_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create a regular user context
    pub fn user(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            is_admin: false,
            email: None,
            session_id: Uuid::new_v4().to_string(),
        }
    }

    /// Set email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }
}

/// Project access information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAccess {
    /// Project ID
    pub project_id: Uuid,
    /// Project name (for display)
    pub project_name: String,
    /// Access role for this project
    pub role: ProjectRole,
    /// When access was granted
    pub granted_at: chrono::DateTime<chrono::Utc>,
}

/// Role a user has on a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    /// Full access - can do everything
    Owner,
    /// Can edit but not manage permissions
    Editor,
    /// Can only view
    Viewer,
    /// Can execute tasks but not edit
    Executor,
}

impl ProjectRole {
    /// Check if role can edit
    pub fn can_edit(&self) -> bool {
        matches!(self, ProjectRole::Owner | ProjectRole::Editor)
    }

    /// Check if role can view
    pub fn can_view(&self) -> bool {
        true // All roles can view
    }

    /// Check if role can execute
    pub fn can_execute(&self) -> bool {
        matches!(self, ProjectRole::Owner | ProjectRole::Editor | ProjectRole::Executor)
    }

    /// Check if role can manage permissions
    pub fn can_manage(&self) -> bool {
        matches!(self, ProjectRole::Owner)
    }
}

/// Access scope determined by user context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessScope {
    /// Admin - full platform access
    Admin,
    /// Access to specific projects
    Projects(HashSet<Uuid>),
    /// Access to a single project
    SingleProject(Uuid),
    /// No access
    None,
}

/// Access Control Manager
///
/// Manages user-to-project access mappings and enforces data isolation.
/// CRITICAL: This component ensures no client data leaks between users.
pub struct AccessControl {
    /// User to project access mappings
    /// Key: user_id, Value: map of project_id to access info
    user_access: Arc<RwLock<HashMap<String, HashMap<Uuid, ProjectAccess>>>>,
    /// Cache of admin user IDs for fast lookup
    admin_users: Arc<RwLock<HashSet<String>>>,
    /// Audit log of access checks (for security review)
    audit_log: Arc<RwLock<Vec<AccessAuditEntry>>>,
}

/// Audit entry for access checks
#[derive(Debug, Clone, Serialize)]
struct AccessAuditEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    user_id: String,
    action: String,
    project_id: Option<Uuid>,
    granted: bool,
    reason: String,
}

impl AccessControl {
    /// Create a new AccessControl instance
    pub fn new() -> Self {
        Self {
            user_access: Arc::new(RwLock::new(HashMap::new())),
            admin_users: Arc::new(RwLock::new(HashSet::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Sync access permissions from database
    pub async fn sync_from_database(&self, pool: &SqlitePool) -> Result<()> {
        tracing::info!("Syncing access permissions from database");

        // Load admin users using runtime query (avoids compile-time sqlx check)
        #[derive(sqlx::FromRow)]
        struct AdminRow {
            id: Vec<u8>,
            email: String,
        }

        let admin_rows: Vec<AdminRow> = sqlx::query_as(
            "SELECT id, email FROM users WHERE is_admin = 1"
        )
        .fetch_all(pool)
        .await
        .map_err(TopsiError::DatabaseError)?;

        let mut admins = self.admin_users.write().await;
        admins.clear();
        for row in admin_rows {
            // Convert 16-byte UUID blob to UUID string for consistent key format
            if let Ok(user_uuid) = Uuid::from_slice(&row.id) {
                let user_id_str = user_uuid.to_string();
                admins.insert(user_id_str.clone());
                tracing::debug!("Registered admin user: {} ({})", row.email, user_id_str);
            }
        }

        // Load user-project memberships from project_members table
        let mut user_access = self.user_access.write().await;
        user_access.clear();

        #[derive(sqlx::FromRow)]
        struct MemberRow {
            user_id: Vec<u8>,
            project_id: String,
            role: String,
            granted_at: String,
            project_name: String,
        }

        let member_result: std::result::Result<Vec<MemberRow>, sqlx::Error> = sqlx::query_as(
            r#"
            SELECT pm.user_id, pm.project_id, pm.role, pm.granted_at, p.name as project_name
            FROM project_members pm
            JOIN projects p ON p.id = pm.project_id
            "#
        )
        .fetch_all(pool)
        .await;

        if let Ok(member_rows) = member_result {
            for row in member_rows {
                let user_id_str = if let Ok(user_uuid) = Uuid::from_slice(&row.user_id) {
                    user_uuid.to_string()
                } else {
                    String::from_utf8_lossy(&row.user_id).to_string()
                };

                if let Ok(project_id) = Uuid::parse_str(&row.project_id) {
                    let role = match row.role.as_str() {
                        "owner" => ProjectRole::Owner,
                        "editor" => ProjectRole::Editor,
                        "executor" => ProjectRole::Executor,
                        _ => ProjectRole::Viewer,
                    };

                    let granted_at = chrono::DateTime::parse_from_rfc3339(&row.granted_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    let access = ProjectAccess {
                        project_id,
                        project_name: row.project_name,
                        role,
                        granted_at,
                    };

                    user_access
                        .entry(user_id_str)
                        .or_insert_with(HashMap::new)
                        .insert(project_id, access);
                }
            }
        } else {
            tracing::debug!("project_members table not found, skipping membership sync");
        }

        tracing::info!(
            "Synced {} admin users and {} user-project mappings",
            admins.len(),
            user_access.values().map(|m| m.len()).sum::<usize>()
        );

        Ok(())
    }

    /// Get the access scope for a user
    pub async fn get_access_scope(&self, context: &UserContext) -> AccessScope {
        // Admins get full access
        if context.is_admin || self.admin_users.read().await.contains(&context.user_id) {
            self.log_access_check(&context.user_id, "get_scope", None, true, "admin_access").await;
            return AccessScope::Admin;
        }

        // Get user's accessible projects
        let user_access = self.user_access.read().await;
        if let Some(projects) = user_access.get(&context.user_id) {
            let project_ids: HashSet<Uuid> = projects.keys().copied().collect();

            if project_ids.is_empty() {
                self.log_access_check(&context.user_id, "get_scope", None, false, "no_projects").await;
                return AccessScope::None;
            }

            if project_ids.len() == 1 {
                let pid = *project_ids.iter().next().unwrap();
                self.log_access_check(&context.user_id, "get_scope", Some(pid), true, "single_project").await;
                return AccessScope::SingleProject(pid);
            }

            self.log_access_check(&context.user_id, "get_scope", None, true, &format!("{}_projects", project_ids.len())).await;
            return AccessScope::Projects(project_ids);
        }

        self.log_access_check(&context.user_id, "get_scope", None, false, "no_access_entry").await;
        AccessScope::None
    }

    /// Check if a user can access a specific project
    pub async fn can_access_project(&self, context: &UserContext, project_id: Uuid) -> bool {
        // Admins can access everything
        if context.is_admin || self.admin_users.read().await.contains(&context.user_id) {
            self.log_access_check(&context.user_id, "project_access", Some(project_id), true, "admin").await;
            return true;
        }

        // Check user's project access
        let user_access = self.user_access.read().await;
        let has_access = user_access
            .get(&context.user_id)
            .map(|projects| projects.contains_key(&project_id))
            .unwrap_or(false);

        self.log_access_check(
            &context.user_id,
            "project_access",
            Some(project_id),
            has_access,
            if has_access { "granted" } else { "denied" }
        ).await;

        has_access
    }

    /// Check if user can perform a specific action on a project
    pub async fn can_perform_action(
        &self,
        context: &UserContext,
        project_id: Uuid,
        action: &str,
    ) -> bool {
        // Admins can do everything
        if context.is_admin || self.admin_users.read().await.contains(&context.user_id) {
            return true;
        }

        // Get user's role on the project
        let user_access = self.user_access.read().await;
        let role = user_access
            .get(&context.user_id)
            .and_then(|projects| projects.get(&project_id))
            .map(|access| access.role);

        match role {
            Some(r) => match action {
                "view" => r.can_view(),
                "edit" => r.can_edit(),
                "execute" => r.can_execute(),
                "manage" => r.can_manage(),
                _ => false,
            },
            None => false,
        }
    }

    /// Get all projects a user can access
    pub async fn get_accessible_projects(&self, context: &UserContext) -> Vec<ProjectAccess> {
        // Admins case would need to query all projects from DB
        // For now, return user's explicit access
        let user_access = self.user_access.read().await;
        user_access
            .get(&context.user_id)
            .map(|projects| projects.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Grant project access to a user
    pub async fn grant_access(
        &self,
        user_id: &str,
        project_id: Uuid,
        project_name: &str,
        role: ProjectRole,
    ) -> Result<()> {
        let access = ProjectAccess {
            project_id,
            project_name: project_name.to_string(),
            role,
            granted_at: chrono::Utc::now(),
        };

        let mut user_access = self.user_access.write().await;
        user_access
            .entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(project_id, access);

        tracing::info!(
            "Granted {:?} access to project {} for user {}",
            role, project_id, user_id
        );

        Ok(())
    }

    /// Revoke project access from a user
    pub async fn revoke_access(&self, user_id: &str, project_id: Uuid) -> Result<()> {
        let mut user_access = self.user_access.write().await;
        if let Some(projects) = user_access.get_mut(user_id) {
            projects.remove(&project_id);
            tracing::info!(
                "Revoked access to project {} for user {}",
                project_id, user_id
            );
        }
        Ok(())
    }

    /// Register an admin user
    pub async fn register_admin(&self, user_id: &str) {
        let mut admins = self.admin_users.write().await;
        admins.insert(user_id.to_string());
        tracing::info!("Registered admin user: {}", user_id);
    }

    /// Remove admin privileges
    pub async fn unregister_admin(&self, user_id: &str) {
        let mut admins = self.admin_users.write().await;
        admins.remove(user_id);
        tracing::info!("Removed admin privileges from user: {}", user_id);
    }

    /// Log an access check for audit purposes
    async fn log_access_check(
        &self,
        user_id: &str,
        action: &str,
        project_id: Option<Uuid>,
        granted: bool,
        reason: &str,
    ) {
        let entry = AccessAuditEntry {
            timestamp: chrono::Utc::now(),
            user_id: user_id.to_string(),
            action: action.to_string(),
            project_id,
            granted,
            reason: reason.to_string(),
        };

        let mut log = self.audit_log.write().await;

        // Keep log bounded (last 10000 entries)
        if log.len() >= 10000 {
            log.remove(0);
        }

        log.push(entry);
    }

    /// Alias for get_access_scope (convenience method)
    pub async fn get_scope(&self, context: &UserContext) -> AccessScope {
        self.get_access_scope(context).await
    }

    /// Log an access attempt for audit purposes
    pub async fn log_access(&self, context: &UserContext, action: &str, granted: bool) {
        self.log_access_check(
            &context.user_id,
            action,
            None,
            granted,
            if granted { "Access granted" } else { "Access denied" },
        ).await;
    }

    /// Get recent audit entries (for security review)
    pub async fn get_audit_log(&self, limit: usize) -> Vec<serde_json::Value> {
        let log = self.audit_log.read().await;
        log.iter()
            .rev()
            .take(limit)
            .map(|e| serde_json::to_value(e).unwrap_or_default())
            .collect()
    }

    /// Validate that a user's access hasn't been tampered with
    /// Returns true if access is valid and consistent
    pub async fn validate_access_integrity(&self, context: &UserContext) -> bool {
        // Cross-check admin status
        if context.is_admin {
            return self.admin_users.read().await.contains(&context.user_id);
        }

        // For regular users, verify they have at least one valid access entry
        let user_access = self.user_access.read().await;
        user_access.contains_key(&context.user_id)
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_admin_access() {
        let ac = AccessControl::new();
        ac.register_admin("admin-user").await;

        let ctx = UserContext::user("admin-user");
        let scope = ac.get_access_scope(&ctx).await;

        assert!(matches!(scope, AccessScope::Admin));
    }

    #[tokio::test]
    async fn test_user_project_access() {
        let ac = AccessControl::new();
        let project_id = Uuid::new_v4();

        ac.grant_access("user-1", project_id, "Test Project", ProjectRole::Editor)
            .await
            .unwrap();

        let ctx = UserContext::user("user-1");

        assert!(ac.can_access_project(&ctx, project_id).await);
        assert!(!ac.can_access_project(&ctx, Uuid::new_v4()).await);
    }

    #[tokio::test]
    async fn test_data_isolation() {
        let ac = AccessControl::new();
        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();

        ac.grant_access("user-a", project_a, "Project A", ProjectRole::Owner)
            .await
            .unwrap();
        ac.grant_access("user-b", project_b, "Project B", ProjectRole::Owner)
            .await
            .unwrap();

        let ctx_a = UserContext::user("user-a");
        let ctx_b = UserContext::user("user-b");

        // User A can only see Project A
        assert!(ac.can_access_project(&ctx_a, project_a).await);
        assert!(!ac.can_access_project(&ctx_a, project_b).await);

        // User B can only see Project B
        assert!(!ac.can_access_project(&ctx_b, project_a).await);
        assert!(ac.can_access_project(&ctx_b, project_b).await);
    }

    #[tokio::test]
    async fn test_role_permissions() {
        let ac = AccessControl::new();
        let project_id = Uuid::new_v4();

        ac.grant_access("viewer", project_id, "Test", ProjectRole::Viewer)
            .await
            .unwrap();
        ac.grant_access("editor", project_id, "Test", ProjectRole::Editor)
            .await
            .unwrap();
        ac.grant_access("owner", project_id, "Test", ProjectRole::Owner)
            .await
            .unwrap();

        let viewer_ctx = UserContext::user("viewer");
        let editor_ctx = UserContext::user("editor");
        let owner_ctx = UserContext::user("owner");

        // Viewer can only view
        assert!(ac.can_perform_action(&viewer_ctx, project_id, "view").await);
        assert!(!ac.can_perform_action(&viewer_ctx, project_id, "edit").await);

        // Editor can view and edit
        assert!(ac.can_perform_action(&editor_ctx, project_id, "view").await);
        assert!(ac.can_perform_action(&editor_ctx, project_id, "edit").await);
        assert!(!ac.can_perform_action(&editor_ctx, project_id, "manage").await);

        // Owner can do everything
        assert!(ac.can_perform_action(&owner_ctx, project_id, "view").await);
        assert!(ac.can_perform_action(&owner_ctx, project_id, "edit").await);
        assert!(ac.can_perform_action(&owner_ctx, project_id, "manage").await);
    }
}
