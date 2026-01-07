use axum::{
    Router,
    extract::{Path, Request, State},
    middleware::{self, Next, from_fn_with_state},
    response::Response,
};
use db::models::cms_site::CmsSite;
use uuid::Uuid;

use deployment::Deployment;
use crate::{DeploymentImpl, error::ApiError, middleware::require_auth};

pub mod faq_items;
pub mod page_sections;
pub mod products;
pub mod public;
pub mod settings;

/// Middleware to load CMS site from path parameter
pub async fn load_site_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(site_id): Path<Uuid>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let site = CmsSite::find_by_id(&deployment.db().pool, site_id)
        .await?
        .ok_or(ApiError::NotFound("Site not found".to_string()))?;

    req.extensions_mut().insert(site);
    Ok(next.run(req).await)
}

/// CMS admin router (authenticated, requires site access)
pub fn admin_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let site_routes = Router::new()
        .nest("/products", products::router(deployment))
        .nest("/faq-items", faq_items::router(deployment))
        .nest("/page-sections", page_sections::router(deployment))
        .nest("/settings", settings::router(deployment))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_site_middleware,
        ));

    Router::new()
        .nest("/sites/{site_id}", site_routes)
        .layer(from_fn_with_state(
            deployment.clone(),
            require_auth,
        ))
}

/// CMS public router (no authentication required)
pub fn public_router() -> Router<DeploymentImpl> {
    public::router()
}

/// Combined CMS router
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .nest("/cms", admin_router(deployment))
        .nest("/public", public_router())
}
