//! Image Generation Service — routes image generation requests through PCG Router.
//!
//! Provides a unified interface for image generation across providers (DALL-E, etc.)
//! with database-driven API key management and cost tracking.

use db::models::pcg_router_model::PcgRouterModel;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Size options for generated images.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSize {
    /// 1024x1024 pixels (square)
    #[default]
    Size1024x1024,
    /// 1792x1024 pixels (landscape)
    Size1792x1024,
    /// 1024x1792 pixels (portrait)
    Size1024x1792,
}

impl ImageSize {
    fn to_openai_string(&self) -> &'static str {
        match self {
            ImageSize::Size1024x1024 => "1024x1024",
            ImageSize::Size1792x1024 => "1792x1024",
            ImageSize::Size1024x1792 => "1024x1792",
        }
    }
}

/// Quality options for generated images.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageQuality {
    /// Standard quality (faster, cheaper)
    #[default]
    Standard,
    /// HD quality (more detail, higher cost)
    Hd,
}

impl ImageQuality {
    fn to_openai_string(&self) -> &'static str {
        match self {
            ImageQuality::Standard => "standard",
            ImageQuality::Hd => "hd",
        }
    }
}

/// Style options for DALL-E 3.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageStyle {
    /// Vivid style (hyper-real, dramatic)
    #[default]
    Vivid,
    /// Natural style (more natural, less hyper-real)
    Natural,
}

impl ImageStyle {
    fn to_openai_string(&self) -> &'static str {
        match self {
            ImageStyle::Vivid => "vivid",
            ImageStyle::Natural => "natural",
        }
    }
}

/// Result of an image generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenResult {
    /// URL of the generated image (temporary, typically expires in 1 hour).
    pub url: String,
    /// The revised prompt used by the model (DALL-E 3 rewrites prompts).
    pub revised_prompt: Option<String>,
}

/// Metadata about the image generation request.
#[derive(Debug, Clone, Serialize)]
pub struct ImageGenMetadata {
    /// Model used for generation.
    pub model_used: String,
    /// Provider name (e.g., "openai").
    pub provider: String,
    /// Estimated cost in microdollars.
    pub estimated_cost_micros: Option<i64>,
}

/// Stateless image generation service.
///
/// Routes requests through PCG Router models for API key management
/// and cost tracking.
pub struct ImageGenService;

impl ImageGenService {
    /// Generate an image using DALL-E or compatible provider.
    ///
    /// Uses the PCG Router model table to resolve API keys and endpoints.
    /// Falls back to environment variable if no model is configured.
    pub async fn generate_image(
        pool: &SqlitePool,
        prompt: &str,
        size: ImageSize,
        quality: ImageQuality,
        style: ImageStyle,
    ) -> anyhow::Result<(ImageGenResult, ImageGenMetadata)> {
        use std::time::Instant;

        use super::editron::UsageTracker;

        // Try to find a DALL-E model in PCG Router
        let model = Self::find_image_model(pool).await?;

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let api_key = model
            .as_ref()
            .and_then(|m| m.resolve_api_key())
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());

        let Some(api_key) = api_key else {
            anyhow::bail!("No OpenAI API key configured for image generation");
        };

        let model_id = model
            .as_ref()
            .map(|m| m.model_id.as_str())
            .unwrap_or("dall-e-3");

        let base_url = model
            .as_ref()
            .map(|m| m.base_url())
            .unwrap_or("https://api.openai.com");

        let provider_name = model
            .as_ref()
            .map(|m| m.provider.clone())
            .unwrap_or_else(|| "openai".to_string());

        let payload = json!({
            "model": model_id,
            "prompt": prompt,
            "quality": quality.to_openai_string(),
            "size": size.to_openai_string(),
            "style": style.to_openai_string(),
            "n": 1,
        });

        let url = format!("{}/v1/images/generations", base_url);
        let start = Instant::now();
        let resp = http
            .post(&url)
            .bearer_auth(&api_key)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let duration_ms = start.elapsed().as_millis() as i64;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let error_msg = format!("Image generation failed {}: {}", status, text);

            // Log failed generation
            UsageTracker::log_image_gen_operation(
                pool,
                &provider_name,
                "generate",
                model_id,
                size.to_openai_string(),
                quality.to_openai_string(),
                duration_ms,
                false,
                Some(&error_msg),
                None,
            )
            .await;

            anyhow::bail!("{}", error_msg);
        }

        let body: Value = resp.json().await?;

        // Parse response
        let data = body["data"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("No image in response"))?;

        let url = data["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No URL in image response"))?
            .to_string();

        let revised_prompt = data["revised_prompt"].as_str().map(String::from);

        // Calculate cost (DALL-E 3 pricing as of 2024)
        let cost_micros = Self::calculate_cost(model_id, &size, &quality);

        // Log successful generation
        UsageTracker::log_image_gen_operation(
            pool,
            &provider_name,
            "generate",
            model_id,
            size.to_openai_string(),
            quality.to_openai_string(),
            duration_ms,
            true,
            None,
            Some(cost_micros),
        )
        .await;

        let result = ImageGenResult {
            url,
            revised_prompt,
        };

        let metadata = ImageGenMetadata {
            model_used: model_id.to_string(),
            provider: provider_name,
            estimated_cost_micros: Some(cost_micros),
        };

        tracing::info!(
            "[IMAGE_GEN] Generated image via {}/{} - cost: {} micros",
            metadata.provider,
            metadata.model_used,
            cost_micros
        );

        Ok((result, metadata))
    }

    /// Generate multiple images (for mood boards, etc.).
    pub async fn generate_images(
        pool: &SqlitePool,
        prompts: &[&str],
        size: ImageSize,
        quality: ImageQuality,
        style: ImageStyle,
    ) -> Vec<Result<(ImageGenResult, ImageGenMetadata), anyhow::Error>> {
        let mut results = Vec::with_capacity(prompts.len());

        for prompt in prompts {
            let result = Self::generate_image(pool, prompt, size, quality, style).await;
            results.push(result);
        }

        results
    }

    /// Find an image generation model in PCG Router.
    async fn find_image_model(pool: &SqlitePool) -> anyhow::Result<Option<PcgRouterModel>> {
        // Look for DALL-E models in the PCG Router table
        let models = PcgRouterModel::list_enabled(pool).await?;

        // Find a model that supports image generation
        let image_model = models.into_iter().find(|m| {
            m.model_id.contains("dall-e") || m.model_id.contains("image") || m.provider == "openai"
        });

        Ok(image_model)
    }

    /// Calculate cost in microdollars for image generation.
    fn calculate_cost(model: &str, size: &ImageSize, quality: &ImageQuality) -> i64 {
        // DALL-E 3 pricing (as of 2024):
        // Standard quality: $0.040 (1024x1024), $0.080 (1024x1792 or 1792x1024)
        // HD quality: $0.080 (1024x1024), $0.120 (1024x1792 or 1792x1024)
        if model.contains("dall-e-3") {
            match (quality, size) {
                (ImageQuality::Standard, ImageSize::Size1024x1024) => 40_000,
                (ImageQuality::Standard, _) => 80_000,
                (ImageQuality::Hd, ImageSize::Size1024x1024) => 80_000,
                (ImageQuality::Hd, _) => 120_000,
            }
        } else if model.contains("dall-e-2") {
            // DALL-E 2 is cheaper
            match size {
                ImageSize::Size1024x1024 => 20_000,
                _ => 20_000, // DALL-E 2 only supports 1024x1024
            }
        } else {
            // Unknown model, estimate conservatively
            80_000
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_string_conversion() {
        assert_eq!(ImageSize::Size1024x1024.to_openai_string(), "1024x1024");
        assert_eq!(ImageSize::Size1792x1024.to_openai_string(), "1792x1024");
        assert_eq!(ImageSize::Size1024x1792.to_openai_string(), "1024x1792");
    }

    #[test]
    fn test_cost_calculation() {
        assert_eq!(
            ImageGenService::calculate_cost(
                "dall-e-3",
                &ImageSize::Size1024x1024,
                &ImageQuality::Standard
            ),
            40_000
        );
        assert_eq!(
            ImageGenService::calculate_cost(
                "dall-e-3",
                &ImageSize::Size1792x1024,
                &ImageQuality::Hd
            ),
            120_000
        );
    }
}
