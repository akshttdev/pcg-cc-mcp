//! User Onboarding Service
//!
//! Handles automatic setup when a user is created or first logs in:
//! - Creates a home project workspace
//! - Creates a personal Orcha agent

use db::models::user::User;
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use super::agent_registry::AgentRegistryService;

pub struct OnboardingResult {
    pub home_project_id: Uuid,
    pub orcha_agent_id: Uuid,
}

pub struct UserOnboardingService;

impl UserOnboardingService {
    /// Run onboarding for a user: create home project + Orcha agent
    pub async fn onboard_user(
        pool: &SqlitePool,
        user_id: Uuid,
        username: &str,
    ) -> anyhow::Result<OnboardingResult> {
        // 1. Check if user already has projects (via project_members)
        let has_projects: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM project_members WHERE user_id = ?",
        )
        .bind(user_id.as_bytes().as_slice())
        .fetch_one(pool)
        .await
        .map(|c| c > 0)
        .unwrap_or(false);

        let home_project_id = if !has_projects {
            // 2. Create home project
            let project_id = Uuid::new_v4();
            let project_name = format!("{}'s Workspace", username);
            let slug = format!("{}-workspace", username.to_lowercase().replace(' ', "-"));

            sqlx::query(
                r#"INSERT INTO projects (id, name, slug, git_repo_path, created_at, updated_at)
                   VALUES (?, ?, ?, ?, datetime('now', 'subsec'), datetime('now', 'subsec'))"#,
            )
            .bind(project_id.as_bytes().as_slice())
            .bind(&project_name)
            .bind(&slug)
            .bind(format!("/home/{}", slug))
            .execute(pool)
            .await?;

            // Add user as owner in project_members
            let member_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO project_members (id, project_id, user_id, role, created_at)
                   VALUES (?, ?, ?, 'owner', datetime('now', 'subsec'))"#,
            )
            .bind(member_id.as_bytes().as_slice())
            .bind(project_id.as_bytes().as_slice())
            .bind(user_id.as_bytes().as_slice())
            .execute(pool)
            .await?;

            info!(
                "Created home project '{}' (ID: {}) for user {}",
                project_name, project_id, user_id
            );

            // 3. Set home_project_id on user
            User::set_home_project(pool, user_id, project_id).await?;

            project_id
        } else {
            // User already has projects - find one to use as home
            let existing_project_id: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT project_id FROM project_members WHERE user_id = ? LIMIT 1",
            )
            .bind(user_id.as_bytes().as_slice())
            .fetch_optional(pool)
            .await?;

            match existing_project_id {
                Some(bytes) => {
                    let id = Uuid::from_slice(&bytes).unwrap_or_else(|_| Uuid::new_v4());
                    User::set_home_project(pool, user_id, id).await?;
                    id
                }
                None => {
                    warn!("User {} has project members but no project_id found", user_id);
                    Uuid::nil()
                }
            }
        };

        // 4. Create Orcha agent for user
        let orcha = AgentRegistryService::get_or_create_user_orcha(pool, user_id).await?;

        info!(
            "Onboarding complete for user {}: home_project={}, orcha={}",
            user_id, home_project_id, orcha.id
        );

        Ok(OnboardingResult {
            home_project_id,
            orcha_agent_id: orcha.id,
        })
    }
}
