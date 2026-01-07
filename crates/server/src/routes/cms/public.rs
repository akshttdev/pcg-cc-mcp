use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::cms_faq_item::CmsFaqItem;
use db::models::cms_page_section::CmsPageSection;
use db::models::cms_product::CmsProduct;
use db::models::cms_site::CmsSite;
use db::models::cms_site_setting::CmsSiteSetting;
use serde::Serialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use std::collections::HashMap;

use deployment::Deployment;
use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Serialize, TS)]
pub struct PublicSiteData {
    pub site: CmsSite,
    pub settings: HashMap<String, String>,
}

// Get site config and theme
pub async fn get_site(
    Path(slug): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<PublicSiteData>>, ApiError> {
    let site = CmsSite::find_by_slug(&deployment.db().pool, &slug)
        .await?
        .ok_or(ApiError::NotFound("Site not found".to_string()))?;

    if !site.is_active {
        return Err(ApiError::NotFound("Site not found".to_string()));
    }

    let settings_list = CmsSiteSetting::find_by_site(&deployment.db().pool, site.id).await?;
    let settings: HashMap<String, String> = settings_list
        .into_iter()
        .map(|s| (s.setting_key, s.setting_value))
        .collect();

    Ok(ResponseJson(ApiResponse::success(PublicSiteData { site, settings })))
}

// Get active products for a site
pub async fn get_products(
    Path(slug): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsProduct>>>, ApiError> {
    let site = CmsSite::find_by_slug(&deployment.db().pool, &slug)
        .await?
        .ok_or(ApiError::NotFound("Site not found".to_string()))?;

    if !site.is_active {
        return Err(ApiError::NotFound("Site not found".to_string()));
    }

    let products = CmsProduct::find_active_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(products)))
}

// Get single product by slug
pub async fn get_product(
    Path((site_slug, product_slug)): Path<(String, String)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CmsProduct>>, ApiError> {
    let site = CmsSite::find_by_slug(&deployment.db().pool, &site_slug)
        .await?
        .ok_or(ApiError::NotFound("Site not found".to_string()))?;

    if !site.is_active {
        return Err(ApiError::NotFound("Site not found".to_string()));
    }

    let product = CmsProduct::find_by_slug(&deployment.db().pool, site.id, &product_slug)
        .await?
        .ok_or(ApiError::NotFound("Product not found".to_string()))?;

    if !product.is_active {
        return Err(ApiError::NotFound("Product not found".to_string()));
    }

    Ok(ResponseJson(ApiResponse::success(product)))
}

// Get active FAQ items for a site
pub async fn get_faq(
    Path(slug): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsFaqItem>>>, ApiError> {
    let site = CmsSite::find_by_slug(&deployment.db().pool, &slug)
        .await?
        .ok_or(ApiError::NotFound("Site not found".to_string()))?;

    if !site.is_active {
        return Err(ApiError::NotFound("Site not found".to_string()));
    }

    let items = CmsFaqItem::find_active_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(items)))
}

// Get active page sections for a specific page
pub async fn get_page_sections(
    Path((site_slug, page_slug)): Path<(String, String)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsPageSection>>>, ApiError> {
    let site = CmsSite::find_by_slug(&deployment.db().pool, &site_slug)
        .await?
        .ok_or(ApiError::NotFound("Site not found".to_string()))?;

    if !site.is_active {
        return Err(ApiError::NotFound("Site not found".to_string()));
    }

    let sections = CmsPageSection::find_active_by_page(&deployment.db().pool, site.id, &page_slug).await?;
    Ok(ResponseJson(ApiResponse::success(sections)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/sites/{slug}", get(get_site))
        .route("/sites/{slug}/products", get(get_products))
        .route("/sites/{site_slug}/products/{product_slug}", get(get_product))
        .route("/sites/{slug}/faq", get(get_faq))
        .route("/sites/{site_slug}/page/{page_slug}", get(get_page_sections))
}
