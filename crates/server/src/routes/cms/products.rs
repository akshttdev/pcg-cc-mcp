use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::cms_product::{CmsProduct, CreateCmsProduct, UpdateCmsProduct};
use db::models::cms_site::CmsSite;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::require_auth};

pub async fn list_products(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsProduct>>>, ApiError> {
    let products = CmsProduct::find_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(products)))
}

pub async fn get_product(
    Extension(site): Extension<CmsSite>,
    Path(product_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CmsProduct>>, ApiError> {
    let product = CmsProduct::find_by_id(&deployment.db().pool, product_id)
        .await?
        .ok_or(ApiError::NotFound("Product not found".to_string()))?;

    // Verify product belongs to site
    if product.site_id != site.id {
        return Err(ApiError::NotFound("Product not found".to_string()));
    }

    Ok(ResponseJson(ApiResponse::success(product)))
}

pub async fn create_product(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateCmsProduct>,
) -> Result<ResponseJson<ApiResponse<CmsProduct>>, ApiError> {
    let product = CmsProduct::create(&deployment.db().pool, site.id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(product)))
}

pub async fn update_product(
    Extension(site): Extension<CmsSite>,
    Path(product_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateCmsProduct>,
) -> Result<ResponseJson<ApiResponse<CmsProduct>>, ApiError> {
    // Verify product belongs to site
    let existing = CmsProduct::find_by_id(&deployment.db().pool, product_id)
        .await?
        .ok_or(ApiError::NotFound("Product not found".to_string()))?;

    if existing.site_id != site.id {
        return Err(ApiError::NotFound("Product not found".to_string()));
    }

    let product = CmsProduct::update(&deployment.db().pool, product_id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(product)))
}

pub async fn delete_product(
    Extension(site): Extension<CmsSite>,
    Path(product_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Verify product belongs to site
    let existing = CmsProduct::find_by_id(&deployment.db().pool, product_id)
        .await?
        .ok_or(ApiError::NotFound("Product not found".to_string()))?;

    if existing.site_id != site.id {
        return Err(ApiError::NotFound("Product not found".to_string()));
    }

    CmsProduct::delete(&deployment.db().pool, product_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_products).post(create_product))
        .route(
            "/{product_id}",
            get(get_product)
                .patch(update_product)
                .delete(delete_product),
        )
        .layer(from_fn_with_state(
            deployment.clone(),
            require_auth,
        ))
}
