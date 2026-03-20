use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    db_uuid::DbUuid,
    models::person_association::{PersonCompanyRole, PersonOrgContact},
};

/// Universal Person entity — the single canonical identity record.
///
/// All PCG users, CRM contacts, leads, contractors, and partners are
/// represented here.  Existing `users` and `crm_contacts` rows are bridged
/// via `user_id` / `crm_contact_id` foreign keys.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Person {
    pub id: DbUuid,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,

    /// 'team' | 'client' | 'contractor' | 'lead' | 'partner' | 'contact'
    pub person_type: String,

    /// 'taker' | 'giver' | 'both' | 'neutral'
    pub financial_role: String,

    /// 'startup' | 'retainer' | null
    pub client_profile: Option<String>,

    /// 'idea' | 'funding' | 'manufacturing' | 'distribution' | 'customer_acquisition'
    pub business_stage: Option<String>,

    pub lifecycle_stage: String,
    pub lead_score: i32,

    pub company_name: Option<String>,
    pub job_title: Option<String>,
    pub website: Option<String>,

    // Bridge FKs
    pub user_id: Option<DbUuid>,
    pub crm_contact_id: Option<DbUuid>,
    pub organization_id: Option<DbUuid>,

    // AI intelligence
    pub intelligence_summary: Option<String>,
    pub intelligence_raw: Option<String>,
    pub intelligence_last_run_at: Option<DateTime<Utc>>,
    pub intelligence_confidence: f64,

    pub notes: Option<String>,
    pub tags: String,
    pub custom_fields: String,

    /// How this person first engaged with PCG.
    /// 'email'|'instagram'|'whatsapp'|'linkedin'|'twitter'|'sms'|'phone'|'in_person'
    pub onboarding_channel: Option<String>,

    /// Preferred channel for outbound communication (may differ from onboarding).
    pub preferred_contact: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Rich view that joins social profiles alongside the person.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonWithSocials {
    #[serde(flatten)]
    pub person: Person,
    pub social_profiles: Vec<PersonSocialProfile>,
    pub company_roles: Vec<PersonCompanyRole>,
    pub org_contacts: Vec<PersonOrgContact>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonSocialProfile {
    pub id: DbUuid,
    pub person_id: DbUuid,

    /// 'linkedin' | 'twitter' | 'instagram' | 'youtube' | 'tiktok' | 'github' | 'facebook' | 'website'
    pub platform: String,

    pub handle: Option<String>,
    pub profile_url: Option<String>,
    pub follower_count: Option<i64>,
    pub following_count: Option<i64>,
    pub bio: Option<String>,
    pub verified: i32,
    pub raw_data: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Create / Update DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreatePerson {
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
    pub person_type: Option<String>,
    pub financial_role: Option<String>,
    pub client_profile: Option<String>,
    pub business_stage: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub lead_score: Option<i32>,
    pub company_name: Option<String>,
    pub job_title: Option<String>,
    pub website: Option<String>,
    pub organization_id: Option<Uuid>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub onboarding_channel: Option<String>,
    pub preferred_contact: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdatePerson {
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
    pub person_type: Option<String>,
    pub financial_role: Option<String>,
    pub client_profile: Option<String>,
    pub business_stage: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub lead_score: Option<i32>,
    pub company_name: Option<String>,
    pub job_title: Option<String>,
    pub website: Option<String>,
    pub organization_id: Option<Uuid>,
    pub intelligence_summary: Option<String>,
    pub intelligence_raw: Option<String>,
    pub intelligence_confidence: Option<f64>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub onboarding_channel: Option<String>,
    pub preferred_contact: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertPersonSocialProfile {
    pub platform: String,
    pub handle: Option<String>,
    pub profile_url: Option<String>,
    pub follower_count: Option<i64>,
    pub following_count: Option<i64>,
    pub bio: Option<String>,
    pub verified: Option<bool>,
    pub raw_data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListPersonsQuery {
    pub person_type: Option<String>,
    pub financial_role: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub organization_id: Option<Uuid>,
    pub query: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

impl Person {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM persons WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await
    }

    pub async fn list(pool: &SqlitePool, q: &ListPersonsQuery) -> sqlx::Result<Vec<Self>> {
        // Build a simple query; SQLite doesn't support dynamic bind well so
        // we construct the filter list and fall back to a full scan when empty.
        let mut where_parts: Vec<String> = Vec::new();
        let limit = q.limit.unwrap_or(100).min(500);
        let offset = q.offset.unwrap_or(0);

        if q.person_type.is_some() {
            where_parts.push("person_type = ?".into());
        }
        if q.financial_role.is_some() {
            where_parts.push("financial_role = ?".into());
        }
        if q.lifecycle_stage.is_some() {
            where_parts.push("lifecycle_stage = ?".into());
        }
        if q.organization_id.is_some() {
            where_parts.push(
                "(organization_id = ? OR EXISTS (SELECT 1 FROM person_organization_contacts poc WHERE poc.person_id = persons.id AND poc.organization_id = ?))"
                    .into(),
            );
        }
        if q.query.is_some() {
            where_parts.push("(full_name LIKE ? OR email LIKE ? OR company_name LIKE ?)".into());
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        let sql = format!(
            "SELECT * FROM persons {where_clause} ORDER BY full_name ASC LIMIT {limit} OFFSET {offset}"
        );

        let mut qb = sqlx::query_as::<_, Person>(&sql);

        if let Some(ref pt) = q.person_type {
            qb = qb.bind(pt);
        }
        if let Some(ref fr) = q.financial_role {
            qb = qb.bind(fr);
        }
        if let Some(ref ls) = q.lifecycle_stage {
            qb = qb.bind(ls);
        }
        if let Some(org_id) = q.organization_id {
            let s = org_id.to_string();
            qb = qb.bind(s.clone()).bind(s);
        }
        if let Some(ref query) = q.query {
            let like = format!("%{query}%");
            qb = qb.bind(like.clone()).bind(like.clone()).bind(like);
        }

        qb.fetch_all(pool).await
    }

    pub async fn create(pool: &SqlitePool, data: CreatePerson) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let person_type = data.person_type.unwrap_or_else(|| "contact".into());
        let financial_role = data.financial_role.unwrap_or_else(|| "neutral".into());
        let lifecycle_stage = data.lifecycle_stage.unwrap_or_else(|| "lead".into());
        let lead_score = data.lead_score.unwrap_or(0);
        let tags =
            serde_json::to_string(&data.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".into());
        let custom_fields = data
            .custom_fields
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".into());

        sqlx::query(
            r#"INSERT INTO persons
               (id, full_name, email, phone, avatar_url, person_type, financial_role,
                client_profile, business_stage, lifecycle_stage, lead_score,
                company_name, job_title, website, organization_id, notes, tags, custom_fields,
                onboarding_channel, preferred_contact)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)"#,
        )
        .bind(id.to_string())
        .bind(&data.full_name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.avatar_url)
        .bind(&person_type)
        .bind(&financial_role)
        .bind(&data.client_profile)
        .bind(&data.business_stage)
        .bind(&lifecycle_stage)
        .bind(lead_score)
        .bind(&data.company_name)
        .bind(&data.job_title)
        .bind(&data.website)
        .bind(data.organization_id.as_ref().map(|u| u.to_string()))
        .bind(&data.notes)
        .bind(&tags)
        .bind(&custom_fields)
        .bind(&data.onboarding_channel)
        .bind(&data.preferred_contact)
        .execute(pool)
        .await?;

        Ok(Self::find_by_id(pool, id).await?.expect("just inserted"))
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdatePerson,
    ) -> sqlx::Result<Option<Self>> {
        // Fetch current record first
        let existing = match Self::find_by_id(pool, id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let full_name = data.full_name.unwrap_or(existing.full_name);
        let person_type = data.person_type.unwrap_or(existing.person_type);
        let financial_role = data.financial_role.unwrap_or(existing.financial_role);
        let lifecycle_stage = data.lifecycle_stage.unwrap_or(existing.lifecycle_stage);
        let lead_score = data.lead_score.unwrap_or(existing.lead_score);
        let tags = data
            .tags
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()))
            .unwrap_or(existing.tags);
        let custom_fields = data
            .custom_fields
            .map(|v| v.to_string())
            .unwrap_or(existing.custom_fields);
        let intelligence_confidence = data
            .intelligence_confidence
            .unwrap_or(existing.intelligence_confidence);

        sqlx::query(
            r#"UPDATE persons SET
               full_name=?2, email=?3, phone=?4, avatar_url=?5,
               person_type=?6, financial_role=?7, client_profile=?8, business_stage=?9,
               lifecycle_stage=?10, lead_score=?11, company_name=?12, job_title=?13,
               website=?14, organization_id=?15, intelligence_summary=?16,
               intelligence_raw=?17, intelligence_confidence=?18, notes=?19,
               tags=?20, custom_fields=?21, onboarding_channel=?22, preferred_contact=?23,
               updated_at=datetime('now','subsec')
               WHERE id=?1"#,
        )
        .bind(id.to_string())
        .bind(&full_name)
        .bind(data.email.or(existing.email))
        .bind(data.phone.or(existing.phone))
        .bind(data.avatar_url.or(existing.avatar_url))
        .bind(&person_type)
        .bind(&financial_role)
        .bind(data.client_profile.or(existing.client_profile))
        .bind(data.business_stage.or(existing.business_stage))
        .bind(&lifecycle_stage)
        .bind(lead_score)
        .bind(data.company_name.or(existing.company_name))
        .bind(data.job_title.or(existing.job_title))
        .bind(data.website.or(existing.website))
        .bind(
            data.organization_id
                .map(|u| u.to_string())
                .or_else(|| existing.organization_id.map(|u| u.into_string())),
        )
        .bind(data.intelligence_summary.or(existing.intelligence_summary))
        .bind(data.intelligence_raw.or(existing.intelligence_raw))
        .bind(intelligence_confidence)
        .bind(data.notes.or(existing.notes))
        .bind(&tags)
        .bind(&custom_fields)
        .bind(data.onboarding_channel.or(existing.onboarding_channel))
        .bind(data.preferred_contact.or(existing.preferred_contact))
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM persons WHERE id = ?1")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

impl PersonSocialProfile {
    pub async fn list_for_person(pool: &SqlitePool, person_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT * FROM person_social_profiles WHERE person_id = ?1 ORDER BY platform ASC",
        )
        .bind(person_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        person_id: Uuid,
        data: UpsertPersonSocialProfile,
    ) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let raw_data = data.raw_data.map(|v| v.to_string());
        let verified = data.verified.unwrap_or(false) as i32;

        sqlx::query(
            r#"INSERT INTO person_social_profiles
               (id, person_id, platform, handle, profile_url,
                follower_count, following_count, bio, verified, raw_data,
                last_synced_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10, datetime('now','subsec'))
               ON CONFLICT(person_id, platform) DO UPDATE SET
                 handle=excluded.handle,
                 profile_url=excluded.profile_url,
                 follower_count=excluded.follower_count,
                 following_count=excluded.following_count,
                 bio=excluded.bio,
                 verified=excluded.verified,
                 raw_data=excluded.raw_data,
                 last_synced_at=datetime('now','subsec'),
                 updated_at=datetime('now','subsec')"#,
        )
        .bind(id.to_string())
        .bind(person_id.to_string())
        .bind(&data.platform)
        .bind(&data.handle)
        .bind(&data.profile_url)
        .bind(data.follower_count)
        .bind(data.following_count)
        .bind(&data.bio)
        .bind(verified)
        .bind(&raw_data)
        .execute(pool)
        .await?;

        // Fetch the row (could have been an UPDATE with a different id)
        sqlx::query_as(
            "SELECT * FROM person_social_profiles WHERE person_id = ?1 AND platform = ?2",
        )
        .bind(person_id.to_string())
        .bind(&data.platform)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, person_id: Uuid, platform: &str) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM person_social_profiles WHERE person_id = ?1 AND platform = ?2",
        )
        .bind(person_id.to_string())
        .bind(platform)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
