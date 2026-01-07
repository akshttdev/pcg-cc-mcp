use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::cms_page_section::{CmsPageSection, CreateCmsPageSection, UpdateCmsPageSection};
use db::models::cms_site::CmsSite;
use utils::response::ApiResponse;
use uuid::Uuid;

use deployment::Deployment;
use crate::{DeploymentImpl, error::ApiError, middleware::require_auth};

pub async fn list_page_sections(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsPageSection>>>, ApiError> {
    let sections = CmsPageSection::find_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(sections)))
}

pub async fn get_page_sections(
    Extension(site): Extension<CmsSite>,
    Path(page_slug): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsPageSection>>>, ApiError> {
    let sections = CmsPageSection::find_by_page(&deployment.db().pool, site.id, &page_slug).await?;
    Ok(ResponseJson(ApiResponse::success(sections)))
}

pub async fn get_section(
    Extension(site): Extension<CmsSite>,
    Path(section_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CmsPageSection>>, ApiError> {
    let section = CmsPageSection::find_by_id(&deployment.db().pool, section_id)
        .await?
        .ok_or(ApiError::NotFound("Page section not found".to_string()))?;

    if section.site_id != site.id {
        return Err(ApiError::NotFound("Page section not found".to_string()));
    }

    Ok(ResponseJson(ApiResponse::success(section)))
}

pub async fn create_or_update_section(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateCmsPageSection>,
) -> Result<ResponseJson<ApiResponse<CmsPageSection>>, ApiError> {
    let section = CmsPageSection::upsert(&deployment.db().pool, site.id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(section)))
}

pub async fn update_section(
    Extension(site): Extension<CmsSite>,
    Path(section_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateCmsPageSection>,
) -> Result<ResponseJson<ApiResponse<CmsPageSection>>, ApiError> {
    let existing = CmsPageSection::find_by_id(&deployment.db().pool, section_id)
        .await?
        .ok_or(ApiError::NotFound("Page section not found".to_string()))?;

    if existing.site_id != site.id {
        return Err(ApiError::NotFound("Page section not found".to_string()));
    }

    let section = CmsPageSection::update(&deployment.db().pool, section_id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(section)))
}

pub async fn delete_section(
    Extension(site): Extension<CmsSite>,
    Path(section_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let existing = CmsPageSection::find_by_id(&deployment.db().pool, section_id)
        .await?
        .ok_or(ApiError::NotFound("Page section not found".to_string()))?;

    if existing.site_id != site.id {
        return Err(ApiError::NotFound("Page section not found".to_string()));
    }

    CmsPageSection::delete(&deployment.db().pool, section_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_page_sections).post(create_or_update_section))
        .route("/page/{page_slug}", get(get_page_sections))
        .route(
            "/{section_id}",
            get(get_section)
                .patch(update_section)
                .delete(delete_section),
        )
        .layer(from_fn_with_state(
            deployment.clone(),
            require_auth,
        ))
}
