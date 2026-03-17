use super::*;

/// GET /api/organizations/:id/knowledge
/// Returns the org-scoped knowledge graph: data sources + project knowledge entries.
pub async fn get_org_knowledge(
    Path(org_id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    use serde_json::json;
    let pool = &deployment.db().pool;

    if !access_context.is_admin {
        let role = Organization::get_user_role(pool, &org_id.to_string(), access_context.user_id).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
        }
    }

    // All data sources scoped to this org
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct DsRow {
        id: Uuid,
        title: String,
        description: Option<String>,
        data_type: String,
        source_type: String,
        status: String,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    }
    let sources: Vec<DsRow> = sqlx::query_as(
        "SELECT id, title, description, data_type, source_type, status, created_at, updated_at FROM data_sources WHERE organization_id = ? AND archived_at IS NULL ORDER BY updated_at DESC"
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    // Org-scoped entries in project_knowledge_sources (owner_type='organization')
    let org_id_hex = hex::encode(org_id.as_bytes());
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct KsRow {
        id: Uuid,
        source_type: String,
        source_id: String,
        source_title: String,
        source_summary: Option<String>,
        coverage_score: f64,
        is_stale: bool,
        last_refreshed_at: chrono::DateTime<chrono::Utc>,
    }
    let knowledge_entries: Vec<KsRow> = sqlx::query_as(
        "SELECT id, source_type, source_id, source_title, source_summary, coverage_score, is_stale, last_refreshed_at FROM project_knowledge_sources WHERE owner_type = 'organization' AND owner_id = ? AND is_active = 1 ORDER BY updated_at DESC"
    )
    .bind(&org_id_hex)
    .fetch_all(pool)
    .await.unwrap_or_default();

    // Brand profile summary
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct BrandSnap {
        tagline: Option<String>,
        industry: Option<String>,
        market_position: Option<String>,
        brand_archetype: Option<String>,
        mission_statement: Option<String>,
        research_status: String,
        research_ran_at: Option<String>,
        research_summary: Option<String>,
    }
    let brand_snap: Option<BrandSnap> = sqlx::query_as(
        "SELECT tagline, industry, market_position, brand_archetype, mission_statement, research_status, research_ran_at, research_summary FROM organization_brand_profiles WHERE organization_id = ?"
    )
    .bind(org_id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await?;

    let avg_coverage = if knowledge_entries.is_empty() { 0.0 } else {
        knowledge_entries.iter().map(|e| e.coverage_score).sum::<f64>() / knowledge_entries.len() as f64
    };

    Ok(Json(ApiResponse::success(json!({
        "data_sources": sources,
        "knowledge_entries": knowledge_entries,
        "brand_summary": brand_snap,
        "stats": {
            "data_source_count": sources.len(),
            "knowledge_entry_count": knowledge_entries.len(),
            "avg_coverage": avg_coverage,
        }
    }))))
}
