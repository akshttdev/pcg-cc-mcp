//! Entity Appearance model for tracking entity appearances at conferences
//!
//! Links entities to specific conference boards with appearance type
//! and speaker-specific details like talk title and description.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Type of appearance at a conference
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "appearance_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AppearanceType {
    Speaker,
    Sponsor,
    Exhibitor,
    Organizer,
    Panelist,
    Keynote,
    WorkshopLeader,
}

impl std::fmt::Display for AppearanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppearanceType::Speaker => write!(f, "speaker"),
            AppearanceType::Sponsor => write!(f, "sponsor"),
            AppearanceType::Exhibitor => write!(f, "exhibitor"),
            AppearanceType::Organizer => write!(f, "organizer"),
            AppearanceType::Panelist => write!(f, "panelist"),
            AppearanceType::Keynote => write!(f, "keynote"),
            AppearanceType::WorkshopLeader => write!(f, "workshop_leader"),
        }
    }
}

/// Status of the appearance tracking
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "appearance_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AppearanceStatus {
    Discovered,
    Researching,
    Researched,
    Verified,
    ContentCreated,
}

impl Default for AppearanceStatus {
    fn default() -> Self {
        Self::Discovered
    }
}

impl std::fmt::Display for AppearanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppearanceStatus::Discovered => write!(f, "discovered"),
            AppearanceStatus::Researching => write!(f, "researching"),
            AppearanceStatus::Researched => write!(f, "researched"),
            AppearanceStatus::Verified => write!(f, "verified"),
            AppearanceStatus::ContentCreated => write!(f, "content_created"),
        }
    }
}

/// Full Entity Appearance record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EntityAppearance {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub conference_board_id: Uuid,
    pub appearance_type: AppearanceType,
    pub talk_title: Option<String>,
    pub talk_description: Option<String>,
    pub talk_slot: Option<String>,
    pub research_artifact_id: Option<Uuid>,
    pub status: AppearanceStatus,
    pub created_at: DateTime<Utc>,
}

/// Create a new entity appearance
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntityAppearance {
    pub entity_id: Uuid,
    pub conference_board_id: Uuid,
    pub appearance_type: AppearanceType,
    pub talk_title: Option<String>,
    pub talk_description: Option<String>,
    pub talk_slot: Option<String>,
}

/// Update an entity appearance
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntityAppearance {
    pub talk_title: Option<String>,
    pub talk_description: Option<String>,
    pub talk_slot: Option<String>,
    pub research_artifact_id: Option<Uuid>,
    pub status: Option<AppearanceStatus>,
}

/// Entity appearance with entity details joined
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EntityAppearanceWithEntity {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub conference_board_id: Uuid,
    pub appearance_type: AppearanceType,
    pub talk_title: Option<String>,
    pub talk_description: Option<String>,
    pub talk_slot: Option<String>,
    pub status: AppearanceStatus,
    pub entity_name: String,
    pub entity_title: Option<String>,
    pub entity_company: Option<String>,
    pub entity_photo_url: Option<String>,
}

impl EntityAppearance {
    /// Find all appearances
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            EntityAppearance,
            r#"SELECT
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>"
            FROM entity_appearances
            ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find appearances by conference board
    pub async fn find_by_conference(pool: &SqlitePool, board_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            EntityAppearance,
            r#"SELECT
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>"
            FROM entity_appearances
            WHERE conference_board_id = $1
            ORDER BY created_at DESC"#,
            board_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find appearances by entity
    pub async fn find_by_entity(pool: &SqlitePool, entity_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            EntityAppearance,
            r#"SELECT
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>"
            FROM entity_appearances
            WHERE entity_id = $1
            ORDER BY created_at DESC"#,
            entity_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find appearances with entity details for a conference
    pub async fn find_with_entities(pool: &SqlitePool, board_id: Uuid) -> Result<Vec<EntityAppearanceWithEntity>, sqlx::Error> {
        sqlx::query_as!(
            EntityAppearanceWithEntity,
            r#"SELECT
                ea.id as "id!: Uuid",
                ea.entity_id as "entity_id!: Uuid",
                ea.conference_board_id as "conference_board_id!: Uuid",
                ea.appearance_type as "appearance_type!: AppearanceType",
                ea.talk_title,
                ea.talk_description,
                ea.talk_slot,
                ea.status as "status!: AppearanceStatus",
                e.canonical_name as "entity_name!",
                e.title as entity_title,
                e.company as entity_company,
                e.photo_url as entity_photo_url
            FROM entity_appearances ea
            JOIN entities e ON ea.entity_id = e.id
            WHERE ea.conference_board_id = $1
            ORDER BY ea.created_at DESC"#,
            board_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find appearance by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            EntityAppearance,
            r#"SELECT
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>"
            FROM entity_appearances
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find or create appearance
    pub async fn find_or_create(
        pool: &SqlitePool,
        entity_id: Uuid,
        board_id: Uuid,
        appearance_type: AppearanceType,
    ) -> Result<Self, sqlx::Error> {
        let type_str = appearance_type.to_string();

        // Try to find existing
        if let Some(existing) = sqlx::query_as!(
            EntityAppearance,
            r#"SELECT
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>"
            FROM entity_appearances
            WHERE entity_id = $1 AND conference_board_id = $2 AND appearance_type = $3"#,
            entity_id,
            board_id,
            type_str
        )
        .fetch_optional(pool)
        .await? {
            return Ok(existing);
        }

        // Create new
        Self::create(pool, &CreateEntityAppearance {
            entity_id,
            conference_board_id: board_id,
            appearance_type,
            talk_title: None,
            talk_description: None,
            talk_slot: None,
        }).await
    }

    /// Create a new appearance
    pub async fn create(pool: &SqlitePool, data: &CreateEntityAppearance) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let type_str = data.appearance_type.to_string();
        let status_str = AppearanceStatus::default().to_string();

        sqlx::query_as!(
            EntityAppearance,
            r#"INSERT INTO entity_appearances (
                id, entity_id, conference_board_id, appearance_type,
                talk_title, talk_description, talk_slot, status
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8
            )
            RETURNING
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.entity_id,
            data.conference_board_id,
            type_str,
            data.talk_title,
            data.talk_description,
            data.talk_slot,
            status_str
        )
        .fetch_one(pool)
        .await
    }

    /// Update an appearance
    pub async fn update(pool: &SqlitePool, id: Uuid, data: &UpdateEntityAppearance) -> Result<Self, sqlx::Error> {
        let status_str = data.status.as_ref().map(|s| s.to_string());

        sqlx::query_as!(
            EntityAppearance,
            r#"UPDATE entity_appearances SET
                talk_title = COALESCE($2, talk_title),
                talk_description = COALESCE($3, talk_description),
                talk_slot = COALESCE($4, talk_slot),
                research_artifact_id = COALESCE($5, research_artifact_id),
                status = COALESCE($6, status)
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                entity_id as "entity_id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                appearance_type as "appearance_type!: AppearanceType",
                talk_title,
                talk_description,
                talk_slot,
                research_artifact_id as "research_artifact_id: Uuid",
                status as "status!: AppearanceStatus",
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.talk_title,
            data.talk_description,
            data.talk_slot,
            data.research_artifact_id,
            status_str
        )
        .fetch_one(pool)
        .await
    }

    /// Update status
    pub async fn update_status(pool: &SqlitePool, id: Uuid, status: AppearanceStatus) -> Result<(), sqlx::Error> {
        let status_str = status.to_string();
        sqlx::query!(
            "UPDATE entity_appearances SET status = $2 WHERE id = $1",
            id,
            status_str
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete an appearance
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM entity_appearances WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Count appearances by conference
    pub async fn count_by_conference(pool: &SqlitePool, board_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM entity_appearances WHERE conference_board_id = $1",
            board_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count as i64)
    }

    /// Count speakers for a conference
    pub async fn count_speakers(pool: &SqlitePool, board_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM entity_appearances
            WHERE conference_board_id = $1
            AND appearance_type IN ('speaker', 'keynote', 'panelist', 'workshop_leader')"#,
            board_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count as i64)
    }
}
