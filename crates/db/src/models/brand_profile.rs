//! Brand profile model for project brand identity data

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Brand voice options for project identity
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum BrandVoice {
    Formal,
    Casual,
    Playful,
    Authoritative,
}

impl BrandVoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrandVoice::Formal => "formal",
            BrandVoice::Casual => "casual",
            BrandVoice::Playful => "playful",
            BrandVoice::Authoritative => "authoritative",
        }
    }
}

impl std::str::FromStr for BrandVoice {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "formal" => Ok(BrandVoice::Formal),
            "casual" => Ok(BrandVoice::Casual),
            "playful" => Ok(BrandVoice::Playful),
            "authoritative" => Ok(BrandVoice::Authoritative),
            _ => Err(format!("Invalid brand voice: {}", s)),
        }
    }
}

/// Project brand profile - stores brand identity data
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BrandProfile {
    pub id: Uuid,
    pub project_id: Uuid,
    pub tagline: Option<String>,
    pub industry: Option<String>,
    pub primary_color: String,
    pub secondary_color: String,
    pub brand_voice: Option<String>,
    pub target_audience: Option<String>,
    pub logo_asset_id: Option<Uuid>,
    pub guidelines_asset_id: Option<Uuid>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// Create/update brand profile payload
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UpsertBrandProfile {
    pub tagline: Option<String>,
    pub industry: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub brand_voice: Option<String>,
    pub target_audience: Option<String>,
    pub logo_asset_id: Option<Uuid>,
    pub guidelines_asset_id: Option<Uuid>,
}

impl BrandProfile {
    /// Find brand profile by project ID
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            BrandProfile,
            r#"
            SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                tagline,
                industry,
                primary_color as "primary_color!: String",
                secondary_color as "secondary_color!: String",
                brand_voice,
                target_audience,
                logo_asset_id as "logo_asset_id?: Uuid",
                guidelines_asset_id as "guidelines_asset_id?: Uuid",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_brand_profiles
            WHERE project_id = $1
            "#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create or update brand profile for a project (upsert)
    pub async fn upsert(
        pool: &SqlitePool,
        project_id: Uuid,
        data: &UpsertBrandProfile,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let primary_color = data.primary_color.clone().unwrap_or_else(|| "#2563EB".to_string());
        let secondary_color = data.secondary_color.clone().unwrap_or_else(|| "#EC4899".to_string());

        sqlx::query_as!(
            BrandProfile,
            r#"
            INSERT INTO project_brand_profiles (
                id, project_id, tagline, industry, primary_color, secondary_color,
                brand_voice, target_audience, logo_asset_id, guidelines_asset_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT(project_id) DO UPDATE SET
                tagline = COALESCE(excluded.tagline, project_brand_profiles.tagline),
                industry = COALESCE(excluded.industry, project_brand_profiles.industry),
                primary_color = excluded.primary_color,
                secondary_color = excluded.secondary_color,
                brand_voice = COALESCE(excluded.brand_voice, project_brand_profiles.brand_voice),
                target_audience = COALESCE(excluded.target_audience, project_brand_profiles.target_audience),
                logo_asset_id = COALESCE(excluded.logo_asset_id, project_brand_profiles.logo_asset_id),
                guidelines_asset_id = COALESCE(excluded.guidelines_asset_id, project_brand_profiles.guidelines_asset_id),
                updated_at = datetime('now', 'subsec')
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                tagline,
                industry,
                primary_color as "primary_color!: String",
                secondary_color as "secondary_color!: String",
                brand_voice,
                target_audience,
                logo_asset_id as "logo_asset_id?: Uuid",
                guidelines_asset_id as "guidelines_asset_id?: Uuid",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            id,
            project_id,
            data.tagline,
            data.industry,
            primary_color,
            secondary_color,
            data.brand_voice,
            data.target_audience,
            data.logo_asset_id,
            data.guidelines_asset_id
        )
        .fetch_one(pool)
        .await
    }

    /// Delete brand profile for a project
    pub async fn delete(pool: &SqlitePool, project_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM project_brand_profiles WHERE project_id = $1",
            project_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
