use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json,
};
use db::models::model_pricing::{ModelPricing, CostEstimate, infer_provider};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::DeploymentImpl;

/// Query params for cost estimation
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CostEstimateQuery {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub provider: Option<String>,
}

/// Request body for creating/updating model pricing
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertModelPricing {
    pub model: String,
    pub provider: String,
    pub input_cost_per_million: i64,
    pub output_cost_per_million: i64,
    pub multiplier: Option<f64>,
}

/// List all model pricing entries
async fn list_pricing(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pricings = ModelPricing::list(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(pricings))
}

/// Get pricing for a specific model
async fn get_pricing(
    State(deployment): State<DeploymentImpl>,
    Path((model, provider)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pricing = ModelPricing::get_with_fallback(&deployment.db().pool, &model, &provider)
        .await
        .map_err(|e| match e {
            db::models::model_pricing::ModelPricingError::NotFound(_, _) => {
                (StatusCode::NOT_FOUND, e.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    Ok(Json(pricing))
}

/// Estimate cost for a given model and token usage
async fn estimate_cost(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<CostEstimateQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let provider = query.provider.as_deref().unwrap_or_else(|| infer_provider(&query.model));

    let pricing = ModelPricing::get_with_fallback(&deployment.db().pool, &query.model, provider)
        .await
        .map_err(|e| match e {
            db::models::model_pricing::ModelPricingError::NotFound(_, _) => {
                (StatusCode::NOT_FOUND, format!("No pricing found for model '{}' with provider '{}'", query.model, provider))
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    let estimate = pricing.calculate_cost(query.input_tokens, query.output_tokens);
    Ok(Json(estimate))
}

/// Create or update model pricing
async fn upsert_pricing(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<UpsertModelPricing>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;
    let multiplier = data.multiplier.unwrap_or(2.0);

    // Check if pricing exists for this model/provider
    let existing = ModelPricing::get(pool, &data.model, &data.provider).await.ok();

    if let Some(existing) = existing {
        // Update existing
        sqlx::query(
            r#"
            UPDATE model_pricing
            SET input_cost_per_million = $1,
                output_cost_per_million = $2,
                multiplier = $3
            WHERE id = $4
            "#,
        )
        .bind(data.input_cost_per_million)
        .bind(data.output_cost_per_million)
        .bind(multiplier)
        .bind(existing.id)
        .execute(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let updated = ModelPricing::get(pool, &data.model, &data.provider)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(updated))
    } else {
        // Create new
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier, effective_from, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, datetime('now'), datetime('now'))
            "#,
        )
        .bind(id)
        .bind(&data.model)
        .bind(&data.provider)
        .bind(data.input_cost_per_million)
        .bind(data.output_cost_per_million)
        .bind(multiplier)
        .execute(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let created = ModelPricing::get(pool, &data.model, &data.provider)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(created))
    }
}

/// Delete model pricing
async fn delete_pricing(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM model_pricing WHERE id = $1")
        .bind(id)
        .execute(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Pricing not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/model-pricing", get(list_pricing).post(upsert_pricing))
        .route("/model-pricing/estimate", get(estimate_cost))
        .route("/model-pricing/{model}/{provider}", get(get_pricing))
        .route("/model-pricing/{id}", put(upsert_pricing).delete(delete_pricing))
}
