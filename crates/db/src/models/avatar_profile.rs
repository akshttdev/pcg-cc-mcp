use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AvatarProfile {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub name: String,
    pub slug: Option<String>,
    pub identity_doc: Option<String>,
    pub style_notes: Option<String>,
    pub heygen_avatar_id: Option<String>,
    pub heygen_avatar_type: String,
    pub elevenlabs_voice_id: String,
    pub voice_sample_url: Option<String>,
    pub reference_image_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub default_background_url: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateAvatarProfile {
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub slug: Option<String>,
    pub identity_doc: Option<String>,
    pub style_notes: Option<String>,
    pub heygen_avatar_id: Option<String>,
    pub heygen_avatar_type: Option<String>,
    pub elevenlabs_voice_id: Option<String>,
    pub reference_image_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub default_background_url: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateAvatarProfile {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub identity_doc: Option<String>,
    pub style_notes: Option<String>,
    pub heygen_avatar_id: Option<String>,
    pub heygen_avatar_type: Option<String>,
    pub elevenlabs_voice_id: Option<String>,
    pub reference_image_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub default_background_url: Option<String>,
    pub status: Option<String>,
}

impl AvatarProfile {
    pub async fn create(
        pool: &SqlitePool,
        input: CreateAvatarProfile,
        created_by: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let voice_id = input
            .elevenlabs_voice_id
            .unwrap_or_else(|| "ZtcPZrt9K4w8e1OB9M6w".to_string());
        let avatar_type = input
            .heygen_avatar_type
            .unwrap_or_else(|| "instant".to_string());

        sqlx::query(
            r#"INSERT INTO avatar_profiles
               (id, organization_id, created_by, name, slug, identity_doc, style_notes,
                heygen_avatar_id, heygen_avatar_type, elevenlabs_voice_id,
                reference_image_url, thumbnail_url, default_background_url)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(input.organization_id)
        .bind(created_by)
        .bind(&input.name)
        .bind(&input.slug)
        .bind(&input.identity_doc)
        .bind(&input.style_notes)
        .bind(&input.heygen_avatar_id)
        .bind(&avatar_type)
        .bind(&voice_id)
        .bind(&input.reference_image_url)
        .bind(&input.thumbnail_url)
        .bind(&input.default_background_url)
        .execute(pool)
        .await?;

        sqlx::query_as("SELECT * FROM avatar_profiles WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
    }

    pub async fn find(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM avatar_profiles WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list(pool: &SqlitePool, org_id: Option<Uuid>) -> Result<Vec<Self>, sqlx::Error> {
        match org_id {
            Some(oid) => sqlx::query_as(
                "SELECT * FROM avatar_profiles WHERE organization_id = ? ORDER BY created_at DESC",
            )
            .bind(oid)
            .fetch_all(pool)
            .await,
            None => {
                sqlx::query_as("SELECT * FROM avatar_profiles ORDER BY created_at DESC")
                    .fetch_all(pool)
                    .await
            }
        }
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateAvatarProfile,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query(
            r#"UPDATE avatar_profiles SET
               name                    = COALESCE(?, name),
               slug                    = COALESCE(?, slug),
               identity_doc            = COALESCE(?, identity_doc),
               style_notes             = COALESCE(?, style_notes),
               heygen_avatar_id        = COALESCE(?, heygen_avatar_id),
               heygen_avatar_type      = COALESCE(?, heygen_avatar_type),
               elevenlabs_voice_id     = COALESCE(?, elevenlabs_voice_id),
               reference_image_url     = COALESCE(?, reference_image_url),
               thumbnail_url           = COALESCE(?, thumbnail_url),
               default_background_url  = COALESCE(?, default_background_url),
               status                  = COALESCE(?, status),
               updated_at              = datetime('now','subsec')
               WHERE id = ?"#,
        )
        .bind(&input.name)
        .bind(&input.slug)
        .bind(&input.identity_doc)
        .bind(&input.style_notes)
        .bind(&input.heygen_avatar_id)
        .bind(&input.heygen_avatar_type)
        .bind(&input.elevenlabs_voice_id)
        .bind(&input.reference_image_url)
        .bind(&input.thumbnail_url)
        .bind(&input.default_background_url)
        .bind(&input.status)
        .bind(id)
        .execute(pool)
        .await?;

        Self::find(pool, id).await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE avatar_profiles SET status = ?, error_message = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(status)
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM avatar_profiles WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}
