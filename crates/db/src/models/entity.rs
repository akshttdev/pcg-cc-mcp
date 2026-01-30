//! Entity model for reusable entities (speakers, brands, venues, etc.)
//!
//! Entities are shared across conferences and can be reused to avoid
//! duplicate research. Each entity has a canonical name and slug for
//! deduplication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Type of entity
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "entity_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Speaker,
    Brand,
    Sponsor,
    Venue,
    ProductionCompany,
    Organizer,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Speaker => write!(f, "speaker"),
            EntityType::Brand => write!(f, "brand"),
            EntityType::Sponsor => write!(f, "sponsor"),
            EntityType::Venue => write!(f, "venue"),
            EntityType::ProductionCompany => write!(f, "production_company"),
            EntityType::Organizer => write!(f, "organizer"),
        }
    }
}

/// Social media profile for an entity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SocialProfile {
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
    pub followers: Option<i64>,
    pub verified: Option<bool>,
}

/// External IDs for linking to other platforms
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExternalIds {
    pub linkedin: Option<String>,
    pub twitter: Option<String>,
    pub website: Option<String>,
    pub youtube: Option<String>,
    pub github: Option<String>,
    pub crunchbase: Option<String>,
}

/// Cached social media analysis
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SocialAnalysis {
    pub total_followers: i64,
    pub engagement_rate: Option<f64>,
    pub posting_frequency: Option<String>,
    pub top_topics: Vec<String>,
    pub sentiment: Option<String>,
    pub influence_score: Option<f64>,
    pub analyzed_at: DateTime<Utc>,
}

/// Full Entity record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub canonical_name: String,
    pub slug: String,
    pub external_ids: Option<String>,  // JSON
    pub bio: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub photo_url: Option<String>,
    pub social_profiles: Option<String>,  // JSON
    pub social_analysis: Option<String>,  // JSON
    pub data_completeness: f64,
    pub last_researched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity {
    /// Parse external IDs from JSON string
    pub fn external_ids_parsed(&self) -> Option<ExternalIds> {
        self.external_ids.as_deref().and_then(|s| serde_json::from_str(s).ok())
    }

    /// Parse social profiles from JSON string
    pub fn social_profiles_parsed(&self) -> Option<Vec<SocialProfile>> {
        self.social_profiles.as_deref().and_then(|s| serde_json::from_str(s).ok())
    }

    /// Parse social analysis from JSON string
    pub fn social_analysis_parsed(&self) -> Option<SocialAnalysis> {
        self.social_analysis.as_deref().and_then(|s| serde_json::from_str(s).ok())
    }

    /// Check if research is fresh (within given duration)
    pub fn is_research_fresh(&self, max_age: chrono::Duration) -> bool {
        self.last_researched_at
            .map(|researched| Utc::now() - researched < max_age)
            .unwrap_or(false)
    }
}

/// Entity with parsed JSON fields for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EntityWithParsedFields {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub canonical_name: String,
    pub slug: String,
    pub external_ids: Option<ExternalIds>,
    pub bio: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub photo_url: Option<String>,
    pub social_profiles: Option<Vec<SocialProfile>>,
    pub social_analysis: Option<SocialAnalysis>,
    pub data_completeness: f64,
    pub last_researched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Entity> for EntityWithParsedFields {
    fn from(e: Entity) -> Self {
        Self {
            id: e.id,
            entity_type: e.entity_type.clone(),
            canonical_name: e.canonical_name.clone(),
            slug: e.slug.clone(),
            external_ids: e.external_ids_parsed(),
            bio: e.bio.clone(),
            title: e.title.clone(),
            company: e.company.clone(),
            photo_url: e.photo_url.clone(),
            social_profiles: e.social_profiles_parsed(),
            social_analysis: e.social_analysis_parsed(),
            data_completeness: e.data_completeness,
            last_researched_at: e.last_researched_at,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

/// Create a new entity
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntity {
    pub entity_type: EntityType,
    pub canonical_name: String,
    pub slug: Option<String>,
    pub external_ids: Option<ExternalIds>,
    pub bio: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub photo_url: Option<String>,
    pub social_profiles: Option<Vec<SocialProfile>>,
}

/// Update an existing entity
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntity {
    pub canonical_name: Option<String>,
    pub external_ids: Option<ExternalIds>,
    pub bio: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub photo_url: Option<String>,
    pub social_profiles: Option<Vec<SocialProfile>>,
    pub social_analysis: Option<SocialAnalysis>,
    pub data_completeness: Option<f64>,
}

/// Brief entity info for lists
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EntityBrief {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub canonical_name: String,
    pub slug: String,
    pub title: Option<String>,
    pub company: Option<String>,
    pub photo_url: Option<String>,
    pub data_completeness: f64,
}

impl From<Entity> for EntityBrief {
    fn from(e: Entity) -> Self {
        Self {
            id: e.id,
            entity_type: e.entity_type,
            canonical_name: e.canonical_name,
            slug: e.slug,
            title: e.title,
            company: e.company,
            photo_url: e.photo_url,
            data_completeness: e.data_completeness,
        }
    }
}

/// Generate a slug from a name
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

impl Entity {
    /// Find all entities
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            ORDER BY canonical_name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find entities by type
    pub async fn find_by_type(pool: &SqlitePool, entity_type: EntityType) -> Result<Vec<Self>, sqlx::Error> {
        let type_str = entity_type.to_string();
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            WHERE entity_type = $1
            ORDER BY canonical_name ASC"#,
            type_str
        )
        .fetch_all(pool)
        .await
    }

    /// Find entity by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find entity by slug
    pub async fn find_by_slug(pool: &SqlitePool, slug: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            WHERE slug = $1"#,
            slug
        )
        .fetch_optional(pool)
        .await
    }

    /// Find entity by name (case-insensitive)
    pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            WHERE LOWER(canonical_name) = LOWER($1)"#,
            name
        )
        .fetch_optional(pool)
        .await
    }

    /// Find or create entity by name and type
    pub async fn find_or_create(pool: &SqlitePool, entity_type: EntityType, name: &str) -> Result<Self, sqlx::Error> {
        // Try to find by name first
        if let Some(entity) = Self::find_by_name(pool, name).await? {
            return Ok(entity);
        }

        // Try to find by slug
        let slug = slugify(name);
        if let Some(entity) = Self::find_by_slug(pool, &slug).await? {
            return Ok(entity);
        }

        // Create new entity
        let create = CreateEntity {
            entity_type,
            canonical_name: name.to_string(),
            slug: Some(slug),
            external_ids: None,
            bio: None,
            title: None,
            company: None,
            photo_url: None,
            social_profiles: None,
        };
        Self::create(pool, &create).await
    }

    /// Search entities by name (fuzzy search)
    pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        let pattern = format!("%{}%", query.to_lowercase());
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            WHERE LOWER(canonical_name) LIKE $1
               OR LOWER(company) LIKE $1
               OR LOWER(bio) LIKE $1
            ORDER BY data_completeness DESC, canonical_name ASC
            LIMIT $2"#,
            pattern,
            limit
        )
        .fetch_all(pool)
        .await
    }

    /// Create a new entity
    pub async fn create(pool: &SqlitePool, data: &CreateEntity) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let slug = data.slug.clone().unwrap_or_else(|| slugify(&data.canonical_name));
        let external_ids_json = data.external_ids.as_ref().map(|e| serde_json::to_string(e).unwrap());
        let social_profiles_json = data.social_profiles.as_ref().map(|s| serde_json::to_string(s).unwrap());
        let entity_type_str = data.entity_type.to_string();

        sqlx::query_as!(
            Entity,
            r#"INSERT INTO entities (
                id, entity_type, canonical_name, slug, external_ids,
                bio, title, company, photo_url, social_profiles
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10
            )
            RETURNING
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            entity_type_str,
            data.canonical_name,
            slug,
            external_ids_json,
            data.bio,
            data.title,
            data.company,
            data.photo_url,
            social_profiles_json
        )
        .fetch_one(pool)
        .await
    }

    /// Update an existing entity
    pub async fn update(pool: &SqlitePool, id: Uuid, data: &UpdateEntity) -> Result<Self, sqlx::Error> {
        let external_ids_json = data.external_ids.as_ref().map(|e| serde_json::to_string(e).unwrap());
        let social_profiles_json = data.social_profiles.as_ref().map(|s| serde_json::to_string(s).unwrap());
        let social_analysis_json = data.social_analysis.as_ref().map(|s| serde_json::to_string(s).unwrap());

        sqlx::query_as!(
            Entity,
            r#"UPDATE entities SET
                canonical_name = COALESCE($2, canonical_name),
                external_ids = COALESCE($3, external_ids),
                bio = COALESCE($4, bio),
                title = COALESCE($5, title),
                company = COALESCE($6, company),
                photo_url = COALESCE($7, photo_url),
                social_profiles = COALESCE($8, social_profiles),
                social_analysis = COALESCE($9, social_analysis),
                data_completeness = COALESCE($10, data_completeness),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.canonical_name,
            external_ids_json,
            data.bio,
            data.title,
            data.company,
            data.photo_url,
            social_profiles_json,
            social_analysis_json,
            data.data_completeness
        )
        .fetch_one(pool)
        .await
    }

    /// Mark entity as researched
    pub async fn mark_researched(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE entities SET
                last_researched_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete an entity
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM entities WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Get entities needing research (low data completeness or never researched)
    pub async fn find_needing_research(pool: &SqlitePool, threshold: f64, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Entity,
            r#"SELECT
                id as "id!: Uuid",
                entity_type as "entity_type!: EntityType",
                canonical_name,
                slug,
                external_ids,
                bio,
                title,
                company,
                photo_url,
                social_profiles,
                social_analysis,
                data_completeness as "data_completeness!: f64",
                last_researched_at as "last_researched_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM entities
            WHERE data_completeness < $1 OR last_researched_at IS NULL
            ORDER BY data_completeness ASC, created_at ASC
            LIMIT $2"#,
            threshold,
            limit
        )
        .fetch_all(pool)
        .await
    }

    /// Find all entities linked to a specific board via entity_appearances
    pub async fn find_by_board(pool: &SqlitePool, board_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        // Use runtime query to avoid SQLx offline cache issues
        let board_id_str = board_id.to_string();
        let rows = sqlx::query(
            r#"SELECT DISTINCT
                e.id,
                e.entity_type,
                e.canonical_name,
                e.slug,
                e.external_ids,
                e.bio,
                e.title,
                e.company,
                e.photo_url,
                e.social_profiles,
                e.social_analysis,
                e.data_completeness,
                e.last_researched_at,
                e.created_at,
                e.updated_at
            FROM entities e
            INNER JOIN entity_appearances ea ON e.id = ea.entity_id
            WHERE ea.board_id = ?
            ORDER BY e.canonical_name ASC"#
        )
        .bind(&board_id_str)
        .fetch_all(pool)
        .await?;

        let mut entities = Vec::new();
        for row in rows {
            use sqlx::Row;
            let id_str: String = row.get("id");
            let entity_type_str: String = row.get("entity_type");
            let entity_type = match entity_type_str.as_str() {
                "speaker" => EntityType::Speaker,
                "brand" => EntityType::Brand,
                "sponsor" => EntityType::Sponsor,
                "venue" => EntityType::Venue,
                "production_company" => EntityType::ProductionCompany,
                "organizer" => EntityType::Organizer,
                _ => EntityType::Speaker, // default
            };
            let entity = Entity {
                id: Uuid::parse_str(&id_str).unwrap_or_default(),
                entity_type,
                canonical_name: row.get("canonical_name"),
                slug: row.get("slug"),
                external_ids: row.get("external_ids"),
                bio: row.get("bio"),
                title: row.get("title"),
                company: row.get("company"),
                photo_url: row.get("photo_url"),
                social_profiles: row.get("social_profiles"),
                social_analysis: row.get("social_analysis"),
                data_completeness: row.get("data_completeness"),
                last_researched_at: row.get("last_researched_at"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            entities.push(entity);
        }
        Ok(entities)
    }
}
