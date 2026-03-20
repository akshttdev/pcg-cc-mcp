use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

use super::project_onboarding::{OnboardingStatus, SegmentStatus, SegmentType};

/// Main organization onboarding record
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrgOnboarding {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub status: OnboardingStatus,
    pub current_phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<String>,
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Individual organization onboarding segment (carousel item)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrgOnboardingSegment {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub onboarding_id: Uuid,
    pub segment_type: SegmentType,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_agent_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_agent_name: Option<String>,
    pub status: SegmentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_decisions: Option<String>,
    pub order_index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateOrgOnboarding {
    pub organization_id: Uuid,
    #[serde(default)]
    pub context_data: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateOrgOnboarding {
    #[serde(default)]
    pub status: Option<OnboardingStatus>,
    #[serde(default)]
    pub current_phase: Option<String>,
    #[serde(default)]
    pub context_data: Option<String>,
    #[serde(default)]
    pub recommendations: Option<String>,
}

/// Helper to build an OrgOnboarding from a sqlx::Row
fn org_onboarding_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<OrgOnboarding, sqlx::Error> {
    Ok(OrgOnboarding {
        id: row.try_get("id")?,
        organization_id: row.try_get("organization_id")?,
        status: row.try_get("status")?,
        current_phase: row.try_get("current_phase")?,
        context_data: row.try_get("context_data")?,
        recommendations: row.try_get("recommendations")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Helper to build an OrgOnboardingSegment from a sqlx::Row
fn org_onboarding_segment_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<OrgOnboardingSegment, sqlx::Error> {
    Ok(OrgOnboardingSegment {
        id: row.try_get("id")?,
        organization_id: row.try_get("organization_id")?,
        onboarding_id: row.try_get("onboarding_id")?,
        segment_type: row.try_get("segment_type")?,
        name: row.try_get("name")?,
        assigned_agent_id: row.try_get("assigned_agent_id")?,
        assigned_agent_name: row.try_get("assigned_agent_name")?,
        status: row.try_get("status")?,
        recommendations: row.try_get("recommendations")?,
        user_decisions: row.try_get("user_decisions")?,
        order_index: row.try_get("order_index")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

impl OrgOnboarding {
    /// Find onboarding by organization ID
    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT
                id, organization_id, status, current_phase,
                context_data, recommendations,
                started_at, completed_at, created_at, updated_at
            FROM org_onboarding
            WHERE organization_id = $1"#,
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(org_onboarding_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// Find onboarding by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT
                id, organization_id, status, current_phase,
                context_data, recommendations,
                started_at, completed_at, created_at, updated_at
            FROM org_onboarding
            WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(org_onboarding_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// Create a new onboarding record and default segments (transactional)
    pub async fn create_with_segments(
        pool: &SqlitePool,
        payload: &CreateOrgOnboarding,
    ) -> Result<Self, sqlx::Error> {
        let mut tx = pool.begin().await?;

        let id = Uuid::new_v4();
        let status = "active";
        let current_phase = "context_gathering";

        let row = sqlx::query(
            r#"INSERT INTO org_onboarding
                (id, organization_id, status, current_phase, context_data)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING
                id, organization_id, status, current_phase,
                context_data, recommendations,
                started_at, completed_at, created_at, updated_at"#,
        )
        .bind(id)
        .bind(payload.organization_id)
        .bind(status)
        .bind(current_phase)
        .bind(&payload.context_data)
        .fetch_one(&mut *tx)
        .await?;

        let onboarding = org_onboarding_from_row(&row)?;

        // Create default segments (9 segment types, excluding Custom)
        let default_segments = [
            (SegmentType::Research, 0),
            (SegmentType::Brand, 1),
            (SegmentType::Website, 2),
            (SegmentType::Crm, 3),
            (SegmentType::Email, 4),
            (SegmentType::Legal, 5),
            (SegmentType::Intelligence, 6),
            (SegmentType::Integrations, 7),
            (SegmentType::Social, 8),
        ];

        for (segment_type, order_index) in default_segments {
            let segment_id = Uuid::new_v4();
            let name = segment_type.display_name();
            let agent_name = segment_type.default_agent_name();
            let seg_status = "pending";
            let segment_type_str = serde_json::to_value(segment_type)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "custom".to_string());

            sqlx::query(
                r#"INSERT INTO org_onboarding_segments
                    (id, organization_id, onboarding_id, segment_type, name, assigned_agent_name, status, order_index)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
            )
            .bind(segment_id)
            .bind(payload.organization_id)
            .bind(id)
            .bind(segment_type_str)
            .bind(name)
            .bind(agent_name)
            .bind(seg_status)
            .bind(order_index)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(onboarding)
    }

    /// Update onboarding status
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateOrgOnboarding,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = Self::find_by_id(pool, id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let status = payload.status.unwrap_or(existing.status);
        let current_phase = payload
            .current_phase
            .clone()
            .unwrap_or(existing.current_phase);
        let context_data = payload.context_data.clone().or(existing.context_data);
        let recommendations = payload.recommendations.clone().or(existing.recommendations);
        let completed_at =
            if status == OnboardingStatus::Completed && existing.completed_at.is_none() {
                Some(Utc::now())
            } else {
                existing.completed_at
            };

        let status_str = serde_json::to_value(status)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "active".to_string());

        let row = sqlx::query(
            r#"UPDATE org_onboarding
               SET status = $2, current_phase = $3, context_data = $4,
                   recommendations = $5, completed_at = $6, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id, organization_id, status, current_phase,
                context_data, recommendations,
                started_at, completed_at, created_at, updated_at"#,
        )
        .bind(id)
        .bind(status_str)
        .bind(current_phase)
        .bind(context_data)
        .bind(recommendations)
        .bind(completed_at)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(org_onboarding_from_row(&row)?)),
            None => Ok(None),
        }
    }
}

impl OrgOnboardingSegment {
    /// List all segments for an onboarding
    pub async fn list_by_onboarding(
        pool: &SqlitePool,
        onboarding_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT
                id, organization_id, onboarding_id, segment_type, name,
                assigned_agent_id, assigned_agent_name,
                status, recommendations, user_decisions,
                order_index, started_at, completed_at,
                created_at, updated_at
            FROM org_onboarding_segments
            WHERE onboarding_id = $1
            ORDER BY order_index"#,
        )
        .bind(onboarding_id)
        .fetch_all(pool)
        .await?;

        rows.iter().map(org_onboarding_segment_from_row).collect()
    }

    /// Find a segment by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT
                id, organization_id, onboarding_id, segment_type, name,
                assigned_agent_id, assigned_agent_name,
                status, recommendations, user_decisions,
                order_index, started_at, completed_at,
                created_at, updated_at
            FROM org_onboarding_segments
            WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(org_onboarding_segment_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// Update a segment
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        payload: &super::project_onboarding::UpdateOnboardingSegment,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = Self::find_by_id(pool, id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let status = payload.status.unwrap_or(existing.status);
        let recommendations = payload.recommendations.clone().or(existing.recommendations);
        let user_decisions = payload.user_decisions.clone().or(existing.user_decisions);
        let assigned_agent_id = payload.assigned_agent_id.or(existing.assigned_agent_id);
        let assigned_agent_name = payload
            .assigned_agent_name
            .clone()
            .or(existing.assigned_agent_name);

        let started_at = if status == SegmentStatus::InProgress && existing.started_at.is_none() {
            Some(Utc::now())
        } else {
            existing.started_at
        };

        let completed_at = if status == SegmentStatus::Completed && existing.completed_at.is_none()
        {
            Some(Utc::now())
        } else {
            existing.completed_at
        };

        let status_str = serde_json::to_value(status)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "pending".to_string());

        let row = sqlx::query(
            r#"UPDATE org_onboarding_segments
               SET status = $2, recommendations = $3, user_decisions = $4,
                   assigned_agent_id = $5, assigned_agent_name = $6,
                   started_at = $7, completed_at = $8, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id, organization_id, onboarding_id, segment_type, name,
                assigned_agent_id, assigned_agent_name,
                status, recommendations, user_decisions,
                order_index, started_at, completed_at,
                created_at, updated_at"#,
        )
        .bind(id)
        .bind(status_str)
        .bind(recommendations)
        .bind(user_decisions)
        .bind(assigned_agent_id)
        .bind(assigned_agent_name)
        .bind(started_at)
        .bind(completed_at)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(org_onboarding_segment_from_row(&row)?)),
            None => Ok(None),
        }
    }
}
