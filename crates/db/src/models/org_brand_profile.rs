use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgBrandProfile {
    pub id: Uuid,
    pub organization_id: Uuid,
    // Visual
    pub tagline: Option<String>,
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: Option<String>,
    pub typography_heading: Option<String>,
    pub typography_body: Option<String>,
    pub logo_url: Option<String>,
    // Positioning
    pub industry: Option<String>,
    pub market_position: Option<String>,
    pub unique_value_proposition: Option<String>,
    pub mission_statement: Option<String>,
    pub vision_statement: Option<String>,
    pub brand_values: Option<String>,
    pub brand_voice: Option<String>,
    pub brand_archetype: Option<String>,
    // Audience
    pub target_audience: Option<String>,
    pub icp_description: Option<String>,
    pub icp_company_size: Option<String>,
    pub icp_industries: Option<String>,
    // Competitive
    pub competitor_brands: Option<String>,
    pub differentiators: Option<String>,
    // Content & Social
    pub content_pillars: Option<String>,
    pub content_tone: Option<String>,
    pub website_url: Option<String>,
    pub social_instagram: Option<String>,
    pub social_twitter: Option<String>,
    pub social_linkedin: Option<String>,
    pub social_facebook: Option<String>,
    pub social_youtube: Option<String>,
    pub social_tiktok: Option<String>,
    // Research
    pub research_status: String,
    pub research_ran_at: Option<String>,
    pub research_summary: Option<String>,
    pub mood_board_urls: Option<String>,
    pub clearbit_logo_url: Option<String>,
    pub brand_photography_notes: Option<String>,
    pub research_iterations: i64,
    pub research_depth: i64,
    pub founder_name: Option<String>,
    pub founding_year: Option<String>,
    pub key_clients: Option<String>,
    pub estimated_team_size: Option<String>,
    pub tech_stack: Option<String>,
    pub geographic_focus: Option<String>,
    pub funding_stage: Option<String>,
    pub content_strategy_notes: Option<String>,
    pub awards_and_recognition: Option<String>,
    pub brand_gap_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertOrgBrandProfile {
    pub tagline: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub accent_color: Option<String>,
    pub typography_heading: Option<String>,
    pub typography_body: Option<String>,
    pub logo_url: Option<String>,
    pub industry: Option<String>,
    pub market_position: Option<String>,
    pub unique_value_proposition: Option<String>,
    pub mission_statement: Option<String>,
    pub vision_statement: Option<String>,
    pub brand_values: Option<String>,
    pub brand_voice: Option<String>,
    pub brand_archetype: Option<String>,
    pub target_audience: Option<String>,
    pub icp_description: Option<String>,
    pub icp_company_size: Option<String>,
    pub icp_industries: Option<String>,
    pub competitor_brands: Option<String>,
    pub differentiators: Option<String>,
    pub content_pillars: Option<String>,
    pub content_tone: Option<String>,
    pub website_url: Option<String>,
    pub social_instagram: Option<String>,
    pub social_twitter: Option<String>,
    pub social_linkedin: Option<String>,
    pub social_facebook: Option<String>,
    pub social_youtube: Option<String>,
    pub social_tiktok: Option<String>,
}

impl OrgBrandProfile {
    pub async fn find_by_org(
        pool: &SqlitePool,
        org_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let org_bytes = org_id.as_bytes().to_vec();
        sqlx::query_as::<_, OrgBrandProfile>(
            "SELECT id, organization_id, tagline, primary_color, secondary_color, accent_color,
             typography_heading, typography_body, logo_url, industry, market_position,
             unique_value_proposition, mission_statement, vision_statement, brand_values,
             brand_voice, brand_archetype, target_audience, icp_description, icp_company_size,
             icp_industries, competitor_brands, differentiators, content_pillars, content_tone,
             website_url, social_instagram, social_twitter, social_linkedin, social_facebook,
             social_youtube, social_tiktok, research_status, research_ran_at, research_summary,
             mood_board_urls, clearbit_logo_url, brand_photography_notes,
             research_iterations, research_depth, founder_name, founding_year, key_clients,
             estimated_team_size, tech_stack, geographic_focus, funding_stage,
             content_strategy_notes, awards_and_recognition, brand_gap_notes,
             created_at, updated_at
             FROM organization_brand_profiles WHERE organization_id = ?"
        )
        .bind(org_bytes)
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        org_id: Uuid,
        data: &UpsertOrgBrandProfile,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_bytes = id.as_bytes().to_vec();
        let org_bytes = org_id.as_bytes().to_vec();
        let primary = data.primary_color.clone().unwrap_or_else(|| "#2563EB".to_string());
        let secondary = data.secondary_color.clone().unwrap_or_else(|| "#EC4899".to_string());

        sqlx::query(
            "INSERT INTO organization_brand_profiles (
                id, organization_id, tagline, primary_color, secondary_color, accent_color,
                typography_heading, typography_body, logo_url, industry, market_position,
                unique_value_proposition, mission_statement, vision_statement, brand_values,
                brand_voice, brand_archetype, target_audience, icp_description, icp_company_size,
                icp_industries, competitor_brands, differentiators, content_pillars, content_tone,
                website_url, social_instagram, social_twitter, social_linkedin, social_facebook,
                social_youtube, social_tiktok
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            ON CONFLICT(organization_id) DO UPDATE SET
                tagline = COALESCE(excluded.tagline, organization_brand_profiles.tagline),
                primary_color = excluded.primary_color,
                secondary_color = excluded.secondary_color,
                accent_color = COALESCE(excluded.accent_color, organization_brand_profiles.accent_color),
                typography_heading = COALESCE(excluded.typography_heading, organization_brand_profiles.typography_heading),
                typography_body = COALESCE(excluded.typography_body, organization_brand_profiles.typography_body),
                logo_url = COALESCE(excluded.logo_url, organization_brand_profiles.logo_url),
                industry = COALESCE(excluded.industry, organization_brand_profiles.industry),
                market_position = COALESCE(excluded.market_position, organization_brand_profiles.market_position),
                unique_value_proposition = COALESCE(excluded.unique_value_proposition, organization_brand_profiles.unique_value_proposition),
                mission_statement = COALESCE(excluded.mission_statement, organization_brand_profiles.mission_statement),
                vision_statement = COALESCE(excluded.vision_statement, organization_brand_profiles.vision_statement),
                brand_values = COALESCE(excluded.brand_values, organization_brand_profiles.brand_values),
                brand_voice = COALESCE(excluded.brand_voice, organization_brand_profiles.brand_voice),
                brand_archetype = COALESCE(excluded.brand_archetype, organization_brand_profiles.brand_archetype),
                target_audience = COALESCE(excluded.target_audience, organization_brand_profiles.target_audience),
                icp_description = COALESCE(excluded.icp_description, organization_brand_profiles.icp_description),
                icp_company_size = COALESCE(excluded.icp_company_size, organization_brand_profiles.icp_company_size),
                icp_industries = COALESCE(excluded.icp_industries, organization_brand_profiles.icp_industries),
                competitor_brands = COALESCE(excluded.competitor_brands, organization_brand_profiles.competitor_brands),
                differentiators = COALESCE(excluded.differentiators, organization_brand_profiles.differentiators),
                content_pillars = COALESCE(excluded.content_pillars, organization_brand_profiles.content_pillars),
                content_tone = COALESCE(excluded.content_tone, organization_brand_profiles.content_tone),
                website_url = COALESCE(excluded.website_url, organization_brand_profiles.website_url),
                social_instagram = COALESCE(excluded.social_instagram, organization_brand_profiles.social_instagram),
                social_twitter = COALESCE(excluded.social_twitter, organization_brand_profiles.social_twitter),
                social_linkedin = COALESCE(excluded.social_linkedin, organization_brand_profiles.social_linkedin),
                social_facebook = COALESCE(excluded.social_facebook, organization_brand_profiles.social_facebook),
                social_youtube = COALESCE(excluded.social_youtube, organization_brand_profiles.social_youtube),
                social_tiktok = COALESCE(excluded.social_tiktok, organization_brand_profiles.social_tiktok),
                updated_at = datetime('now', 'subsec')"
        )
        .bind(&id_bytes).bind(&org_bytes).bind(&data.tagline).bind(&primary).bind(&secondary)
        .bind(&data.accent_color).bind(&data.typography_heading).bind(&data.typography_body)
        .bind(&data.logo_url).bind(&data.industry).bind(&data.market_position)
        .bind(&data.unique_value_proposition).bind(&data.mission_statement)
        .bind(&data.vision_statement).bind(&data.brand_values).bind(&data.brand_voice)
        .bind(&data.brand_archetype).bind(&data.target_audience).bind(&data.icp_description)
        .bind(&data.icp_company_size).bind(&data.icp_industries).bind(&data.competitor_brands)
        .bind(&data.differentiators).bind(&data.content_pillars).bind(&data.content_tone)
        .bind(&data.website_url).bind(&data.social_instagram).bind(&data.social_twitter)
        .bind(&data.social_linkedin).bind(&data.social_facebook).bind(&data.social_youtube)
        .bind(&data.social_tiktok)
        .execute(pool)
        .await?;

        Self::find_by_org(pool, org_id).await.map(|o| o.unwrap())
    }
}
