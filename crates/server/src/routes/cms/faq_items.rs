use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post, put},
};
use db::models::cms_faq_item::{CmsFaqItem, CreateCmsFaqItem, UpdateCmsFaqItem, ReorderFaqItems};
use db::models::cms_site::CmsSite;
use utils::response::ApiResponse;
use uuid::Uuid;

use deployment::Deployment;
use crate::{DeploymentImpl, error::ApiError, middleware::require_auth};

pub async fn list_faq_items(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsFaqItem>>>, ApiError> {
    let items = CmsFaqItem::find_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(items)))
}

pub async fn get_faq_item(
    Extension(site): Extension<CmsSite>,
    Path(item_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CmsFaqItem>>, ApiError> {
    let item = CmsFaqItem::find_by_id(&deployment.db().pool, item_id)
        .await?
        .ok_or(ApiError::NotFound("FAQ item not found".to_string()))?;

    if item.site_id != site.id {
        return Err(ApiError::NotFound("FAQ item not found".to_string()));
    }

    Ok(ResponseJson(ApiResponse::success(item)))
}

pub async fn create_faq_item(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateCmsFaqItem>,
) -> Result<ResponseJson<ApiResponse<CmsFaqItem>>, ApiError> {
    let item = CmsFaqItem::create(&deployment.db().pool, site.id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(item)))
}

pub async fn update_faq_item(
    Extension(site): Extension<CmsSite>,
    Path(item_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateCmsFaqItem>,
) -> Result<ResponseJson<ApiResponse<CmsFaqItem>>, ApiError> {
    let existing = CmsFaqItem::find_by_id(&deployment.db().pool, item_id)
        .await?
        .ok_or(ApiError::NotFound("FAQ item not found".to_string()))?;

    if existing.site_id != site.id {
        return Err(ApiError::NotFound("FAQ item not found".to_string()));
    }

    let item = CmsFaqItem::update(&deployment.db().pool, item_id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(item)))
}

pub async fn reorder_faq_items(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ReorderFaqItems>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsFaqItem>>>, ApiError> {
    // Verify all items belong to this site
    for item_id in &payload.item_ids {
        let item = CmsFaqItem::find_by_id(&deployment.db().pool, *item_id)
            .await?
            .ok_or(ApiError::NotFound("FAQ item not found".to_string()))?;

        if item.site_id != site.id {
            return Err(ApiError::NotFound("FAQ item not found".to_string()));
        }
    }

    CmsFaqItem::reorder(&deployment.db().pool, &payload.item_ids).await?;

    let items = CmsFaqItem::find_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(items)))
}

pub async fn delete_faq_item(
    Extension(site): Extension<CmsSite>,
    Path(item_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let existing = CmsFaqItem::find_by_id(&deployment.db().pool, item_id)
        .await?
        .ok_or(ApiError::NotFound("FAQ item not found".to_string()))?;

    if existing.site_id != site.id {
        return Err(ApiError::NotFound("FAQ item not found".to_string()));
    }

    CmsFaqItem::delete(&deployment.db().pool, item_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_faq_items).post(create_faq_item))
        .route("/reorder", put(reorder_faq_items))
        .route(
            "/{item_id}",
            get(get_faq_item)
                .patch(update_faq_item)
                .delete(delete_faq_item),
        )
        .layer(from_fn_with_state(
            deployment.clone(),
            require_auth,
        ))
}
