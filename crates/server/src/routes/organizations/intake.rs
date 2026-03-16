use super::*;

// ── Client Intake ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct IntakeTokenResponse {
    pub token: String,
    pub url: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct IntakeContextResponse {
    pub org_name: String,
    pub org_id: String,
    pub existing: IntakeExisting,
}

#[derive(Debug, Serialize)]
pub struct IntakeExisting {
    pub tagline: Option<String>,
    pub industry: Option<String>,
    pub mission_statement: Option<String>,
    pub brand_voice: Option<String>,
    pub website_url: Option<String>,
    pub social_instagram: Option<String>,
    pub social_linkedin: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IntakeSubmission {
    // Foundation
    pub company_name: Option<String>,
    pub tagline: Option<String>,
    pub mission_statement: Option<String>,
    pub vision_statement: Option<String>,
    pub unique_value_proposition: Option<String>,
    pub elevator_pitch: Option<String>,
    // Story & Values
    pub brand_values: Option<String>,        // comma-separated
    pub brand_archetype: Option<String>,
    pub brand_voice: Option<String>,
    pub content_tone: Option<String>,
    // Audience
    pub target_audience: Option<String>,
    pub icp_description: Option<String>,
    pub icp_company_size: Option<String>,
    pub icp_industries: Option<String>,      // comma-separated
    // Competitors
    pub competitor_brands: Option<String>,   // comma-separated
    pub differentiators: Option<String>,     // comma-separated
    // Colours & Visual
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub accent_color: Option<String>,
    pub typography_heading: Option<String>,
    pub typography_body: Option<String>,
    // Online
    pub website_url: Option<String>,
    pub social_instagram: Option<String>,
    pub social_linkedin: Option<String>,
    pub social_twitter: Option<String>,
    pub social_facebook: Option<String>,
    pub social_youtube: Option<String>,
    pub social_tiktok: Option<String>,
    // Content
    pub content_pillars: Option<String>,     // comma-separated
    pub market_position: Option<String>,
    pub industry: Option<String>,
}

/// POST /api/organizations/:id/intake-token — generate a shareable intake link
pub async fn generate_intake_token(
    State(deployment): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<IntakeTokenResponse>>, ApiError> {
    let _ = access_context;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    BrandIntakeToken::create(&deployment.db().pool, org_id, &token, expires_at)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create token: {e}")))?;

    let base_url = std::env::var("PUBLIC_BASE_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".to_string());
    let url = format!("{}/intake/{}", base_url, token);

    Ok(Json(ApiResponse::success(IntakeTokenResponse {
        token,
        url,
        expires_at: expires_at.to_rfc3339(),
    })))
}

/// GET /api/intake/:token — public: get org context for the intake form
pub async fn get_intake_context(
    State(deployment): State<DeploymentImpl>,
    Path(token): Path<String>,
) -> Result<Json<ApiResponse<IntakeContextResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let intake = BrandIntakeToken::find_by_token(pool, &token)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Intake link not found or expired".into()))?;

    if intake.expires_at < chrono::Utc::now() {
        return Err(ApiError::BadRequest("This intake link has expired".into()));
    }

    let org: Organization = Organization::find_by_id(pool, &intake.organization_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Organization not found".into()))?;

    let profile = OrgBrandProfile::find_by_org(pool, intake.organization_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    Ok(Json(ApiResponse::success(IntakeContextResponse {
        org_name: org.name,
        org_id: intake.organization_id.to_string(),
        existing: IntakeExisting {
            tagline: profile.as_ref().and_then(|p| p.tagline.clone()),
            industry: profile.as_ref().and_then(|p| p.industry.clone()),
            mission_statement: profile.as_ref().and_then(|p| p.mission_statement.clone()),
            brand_voice: profile.as_ref().and_then(|p| p.brand_voice.clone()),
            website_url: profile.as_ref().and_then(|p| p.website_url.clone()),
            social_instagram: profile.as_ref().and_then(|p| p.social_instagram.clone()),
            social_linkedin: profile.as_ref().and_then(|p| p.social_linkedin.clone()),
        },
    })))
}

/// POST /api/intake/:token — public: client submits their brand questionnaire
pub async fn submit_intake(
    State(deployment): State<DeploymentImpl>,
    Path(token): Path<String>,
    Json(body): Json<IntakeSubmission>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let intake = BrandIntakeToken::find_by_token(pool, &token)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?
        .ok_or_else(|| ApiError::NotFound("Intake link not found".into()))?;

    if intake.expires_at < chrono::Utc::now() {
        return Err(ApiError::BadRequest("This intake link has expired".into()));
    }

    // Convert comma-separated values to JSON arrays
    let to_json_arr = |s: Option<&String>| -> Option<String> {
        s.map(|v| {
            let items: Vec<&str> = v.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
        })
    };

    let upsert = UpsertOrgBrandProfile {
        tagline: body.tagline.or_else(|| body.elevator_pitch.clone()),
        primary_color: body.primary_color,
        secondary_color: body.secondary_color,
        accent_color: body.accent_color,
        typography_heading: body.typography_heading,
        typography_body: body.typography_body,
        logo_url: None,
        industry: body.industry,
        market_position: body.market_position,
        unique_value_proposition: body.unique_value_proposition,
        mission_statement: body.mission_statement,
        vision_statement: body.vision_statement,
        brand_values: to_json_arr(body.brand_values.as_ref()),
        brand_voice: body.brand_voice,
        brand_archetype: body.brand_archetype,
        target_audience: body.target_audience,
        icp_description: body.icp_description,
        icp_company_size: body.icp_company_size,
        icp_industries: to_json_arr(body.icp_industries.as_ref()),
        competitor_brands: to_json_arr(body.competitor_brands.as_ref()),
        differentiators: to_json_arr(body.differentiators.as_ref()),
        content_pillars: to_json_arr(body.content_pillars.as_ref()),
        content_tone: body.content_tone,
        website_url: body.website_url,
        social_instagram: body.social_instagram,
        social_twitter: body.social_twitter,
        social_linkedin: body.social_linkedin,
        social_facebook: body.social_facebook,
        social_youtube: body.social_youtube,
        social_tiktok: body.social_tiktok,
    };

    OrgBrandProfile::upsert(pool, intake.organization_id, &upsert)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to save intake: {e}")))?;

    BrandIntakeToken::mark_used(pool, &token)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Thank you! Your brand information has been received."
    }))))
}
