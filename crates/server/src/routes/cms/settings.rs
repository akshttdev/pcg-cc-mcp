use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, put, delete},
};
use db::models::cms_site_setting::{CmsSiteSetting, SetCmsSiteSetting};
use db::models::cms_site::CmsSite;
use serde::Serialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use std::collections::HashMap;

use crate::{DeploymentImpl, error::ApiError, middleware::require_auth};

#[derive(Debug, Serialize, TS)]
pub struct SettingsMap {
    pub settings: HashMap<String, String>,
}

pub async fn list_settings(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CmsSiteSetting>>>, ApiError> {
    let settings = CmsSiteSetting::find_by_site(&deployment.db().pool, site.id).await?;
    Ok(ResponseJson(ApiResponse::success(settings)))
}

pub async fn get_settings_map(
    Extension(site): Extension<CmsSite>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<SettingsMap>>, ApiError> {
    let settings = CmsSiteSetting::find_by_site(&deployment.db().pool, site.id).await?;
    let map: HashMap<String, String> = settings
        .into_iter()
        .map(|s| (s.setting_key, s.setting_value))
        .collect();
    Ok(ResponseJson(ApiResponse::success(SettingsMap { settings: map })))
}

pub async fn get_setting(
    Extension(site): Extension<CmsSite>,
    Path(key): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Option<String>>>, ApiError> {
    let value = CmsSiteSetting::get_value(&deployment.db().pool, site.id, &key).await?;
    Ok(ResponseJson(ApiResponse::success(value)))
}

pub async fn set_setting(
    Extension(site): Extension<CmsSite>,
    Path(key): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SetCmsSiteSetting>,
) -> Result<ResponseJson<ApiResponse<CmsSiteSetting>>, ApiError> {
    let setting = CmsSiteSetting::set(&deployment.db().pool, site.id, &key, &payload.value).await?;
    Ok(ResponseJson(ApiResponse::success(setting)))
}

pub async fn delete_setting(
    Extension(site): Extension<CmsSite>,
    Path(key): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    CmsSiteSetting::delete(&deployment.db().pool, site.id, &key).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_settings))
        .route("/map", get(get_settings_map))
        .route(
            "/{key}",
            get(get_setting)
                .put(set_setting)
                .delete(delete_setting),
        )
        .layer(from_fn_with_state(
            deployment.clone(),
            require_auth,
        ))
}
