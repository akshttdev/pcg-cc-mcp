use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ModelPricingError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Model pricing not found for {0}/{1}")]
    NotFound(String, String),
}

/// VIBE token value in USD
pub const VIBE_USD_VALUE: f64 = 0.001;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ModelPricing {
    pub id: Uuid,
    pub model: String,
    pub provider: String,
    /// Cost in cents per 1 million input tokens (already includes multiplier)
    pub input_cost_per_million: i64,
    /// Cost in cents per 1 million output tokens (already includes multiplier)
    pub output_cost_per_million: i64,
    /// Multiplier applied to market rate (e.g., 2.0 = 2x)
    pub multiplier: f64,
    pub effective_from: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CostEstimate {
    pub model: String,
    pub provider: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    /// Cost in cents
    pub cost_cents: i64,
    /// Cost in VIBE tokens
    pub cost_vibe: i64,
    /// Cost in USD
    pub cost_usd: f64,
}

impl ModelPricing {
    /// Get pricing for a specific model
    pub async fn get(
        pool: &SqlitePool,
        model: &str,
        provider: &str,
    ) -> Result<Self, ModelPricingError> {
        let pricing = sqlx::query_as::<_, ModelPricing>(
            r#"
            SELECT * FROM model_pricing
            WHERE model = $1 AND provider = $2
            ORDER BY effective_from DESC
            LIMIT 1
            "#,
        )
        .bind(model)
        .bind(provider)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ModelPricingError::NotFound(model.to_string(), provider.to_string()))?;

        Ok(pricing)
    }

    /// Get pricing with fallback for similar models
    pub async fn get_with_fallback(
        pool: &SqlitePool,
        model: &str,
        provider: &str,
    ) -> Result<Self, ModelPricingError> {
        // Try exact match first
        if let Ok(pricing) = Self::get(pool, model, provider).await {
            return Ok(pricing);
        }

        // Try to find a similar model by prefix matching
        let normalized = model.to_lowercase();
        let pricing = sqlx::query_as::<_, ModelPricing>(
            r#"
            SELECT * FROM model_pricing
            WHERE (LOWER(model) LIKE $1 OR $2 LIKE LOWER(model) || '%')
              AND provider = $3
            ORDER BY effective_from DESC
            LIMIT 1
            "#,
        )
        .bind(format!("{}%", normalized))
        .bind(&normalized)
        .bind(provider)
        .fetch_optional(pool)
        .await?;

        pricing.ok_or_else(|| ModelPricingError::NotFound(model.to_string(), provider.to_string()))
    }

    /// List all pricing entries
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Self>, ModelPricingError> {
        let pricings = sqlx::query_as::<_, ModelPricing>(
            r#"
            SELECT * FROM model_pricing
            ORDER BY provider, model
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(pricings)
    }

    /// Calculate cost for a given token usage
    pub fn calculate_cost(&self, input_tokens: i64, output_tokens: i64) -> CostEstimate {
        // Cost in cents
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million as f64;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million as f64;
        let total_cents = (input_cost + output_cost).ceil() as i64;

        // Convert to USD
        let cost_usd = total_cents as f64 / 100.0;

        // Convert to VIBE (1 VIBE = $0.001)
        let cost_vibe = (cost_usd / VIBE_USD_VALUE).ceil() as i64;

        CostEstimate {
            model: self.model.clone(),
            provider: self.provider.clone(),
            input_tokens,
            output_tokens,
            cost_cents: total_cents,
            cost_vibe,
            cost_usd,
        }
    }

    /// Calculate VIBE cost directly
    pub fn calculate_vibe_cost(&self, input_tokens: i64, output_tokens: i64) -> i64 {
        self.calculate_cost(input_tokens, output_tokens).cost_vibe
    }
}

/// Infer provider from model name
pub fn infer_provider(model: &str) -> &'static str {
    let lower = model.to_lowercase();

    if lower.contains("claude") {
        "anthropic"
    } else if lower.contains("gpt") || lower.contains("codex") {
        "openai"
    } else if lower.contains("gemini") {
        "google"
    } else if lower.contains("llama") || lower.contains("mistral") || lower.contains("qwen") {
        "ollama"
    } else if lower.contains("gpt-oss") {
        "ollama"
    } else {
        "openai" // Default fallback
    }
}

/// Get a cost estimate for a model (convenience function)
pub async fn estimate_cost(
    pool: &SqlitePool,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
) -> Result<CostEstimate, ModelPricingError> {
    let provider = infer_provider(model);
    let pricing = ModelPricing::get_with_fallback(pool, model, provider).await?;
    Ok(pricing.calculate_cost(input_tokens, output_tokens))
}
