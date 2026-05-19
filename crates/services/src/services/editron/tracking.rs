//! Usage tracking for external services
//!
//! Logs API calls to the `service_usage_log` table for cost tracking and auditing.

use sqlx::SqlitePool;

/// Helper for logging service usage to the database.
///
/// All logging methods are fire-and-forget - they won't fail the parent operation
/// if logging fails.
pub struct UsageTracker;

impl UsageTracker {
    /// Log a music service operation (search, download, etc.)
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `provider` - Music provider name (e.g., "artlist", "epidemic", "soundstripe")
    /// * `operation` - Operation type (e.g., "search", "get_track", "download")
    /// * `input_units` - Request size (e.g., per_page for searches)
    /// * `output_units` - Response size (e.g., number of tracks returned)
    /// * `duration_ms` - API call duration in milliseconds
    /// * `success` - Whether the operation succeeded
    /// * `error_message` - Error message if operation failed
    /// * `metadata` - Additional context as JSON
    #[allow(clippy::too_many_arguments)]
    pub async fn log_music_operation(
        pool: &SqlitePool,
        provider: &str,
        operation: &str,
        input_units: Option<i64>,
        output_units: Option<i64>,
        duration_ms: i64,
        success: bool,
        error_message: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) {
        let metadata_str = metadata.map(|m| m.to_string());
        let success_int: i32 = if success { 1 } else { 0 };

        let result = sqlx::query(
            r#"
            INSERT INTO service_usage_log
            (service_type, provider, operation, input_units, output_units,
             duration_ms, success, error_message, metadata_json)
            VALUES ('music', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(provider)
        .bind(operation)
        .bind(input_units)
        .bind(output_units)
        .bind(duration_ms)
        .bind(success_int)
        .bind(error_message)
        .bind(metadata_str)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                provider = provider,
                operation = operation,
                error = %e,
                "Failed to log service usage (non-fatal)"
            );
        }
    }

    /// Log a TTS (text-to-speech) operation
    pub async fn log_tts_operation(
        pool: &SqlitePool,
        provider: &str,
        operation: &str,
        input_chars: Option<i64>,
        output_duration_ms: Option<i64>,
        api_duration_ms: i64,
        success: bool,
        error_message: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) {
        let metadata_str = metadata.map(|m| m.to_string());
        let success_int: i32 = if success { 1 } else { 0 };

        let result = sqlx::query(
            r#"
            INSERT INTO service_usage_log
            (service_type, provider, operation, input_units, output_units,
             duration_ms, success, error_message, metadata_json)
            VALUES ('tts', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(provider)
        .bind(operation)
        .bind(input_chars)
        .bind(output_duration_ms)
        .bind(api_duration_ms)
        .bind(success_int)
        .bind(error_message)
        .bind(metadata_str)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                provider = provider,
                operation = operation,
                error = %e,
                "Failed to log TTS usage (non-fatal)"
            );
        }
    }

    /// Log a video generation operation (e.g., HeyGen)
    pub async fn log_video_gen_operation(
        pool: &SqlitePool,
        provider: &str,
        operation: &str,
        input_duration_sec: Option<i64>,
        output_duration_sec: Option<i64>,
        api_duration_ms: i64,
        success: bool,
        error_message: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) {
        let metadata_str = metadata.map(|m| m.to_string());
        let success_int: i32 = if success { 1 } else { 0 };

        let result = sqlx::query(
            r#"
            INSERT INTO service_usage_log
            (service_type, provider, operation, input_units, output_units,
             duration_ms, success, error_message, metadata_json)
            VALUES ('video_gen', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(provider)
        .bind(operation)
        .bind(input_duration_sec)
        .bind(output_duration_sec)
        .bind(api_duration_ms)
        .bind(success_int)
        .bind(error_message)
        .bind(metadata_str)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                provider = provider,
                operation = operation,
                error = %e,
                "Failed to log video generation usage (non-fatal)"
            );
        }
    }

    /// Log a web search operation (e.g., Exa)
    pub async fn log_search_operation(
        pool: &SqlitePool,
        provider: &str,
        operation: &str,
        results_requested: Option<i64>,
        results_returned: Option<i64>,
        duration_ms: i64,
        success: bool,
        error_message: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) {
        let metadata_str = metadata.map(|m| m.to_string());
        let success_int: i32 = if success { 1 } else { 0 };

        let result = sqlx::query(
            r#"
            INSERT INTO service_usage_log
            (service_type, provider, operation, input_units, output_units,
             duration_ms, success, error_message, metadata_json)
            VALUES ('search', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(provider)
        .bind(operation)
        .bind(results_requested)
        .bind(results_returned)
        .bind(duration_ms)
        .bind(success_int)
        .bind(error_message)
        .bind(metadata_str)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                provider = provider,
                operation = operation,
                error = %e,
                "Failed to log search usage (non-fatal)"
            );
        }
    }

    /// Log an image generation operation (e.g., DALL-E)
    #[allow(clippy::too_many_arguments)]
    pub async fn log_image_gen_operation(
        pool: &SqlitePool,
        provider: &str,
        operation: &str,
        model: &str,
        size: &str,
        quality: &str,
        duration_ms: i64,
        success: bool,
        error_message: Option<&str>,
        cost_microdollars: Option<i64>,
    ) {
        let metadata = serde_json::json!({
            "model": model,
            "size": size,
            "quality": quality,
            "cost_microdollars": cost_microdollars
        });
        let metadata_str = Some(metadata.to_string());
        let success_int: i32 = if success { 1 } else { 0 };

        let result = sqlx::query(
            r#"
            INSERT INTO service_usage_log
            (service_type, provider, operation, input_units, output_units,
             duration_ms, success, error_message, metadata_json)
            VALUES ('image_gen', ?, ?, NULL, 1, ?, ?, ?, ?)
            "#,
        )
        .bind(provider)
        .bind(operation)
        .bind(duration_ms)
        .bind(success_int)
        .bind(error_message)
        .bind(metadata_str)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                provider = provider,
                operation = operation,
                error = %e,
                "Failed to log image generation usage (non-fatal)"
            );
        }
    }

    /// Log a STT (speech-to-text) operation
    #[allow(clippy::too_many_arguments)]
    pub async fn log_stt_operation(
        pool: &SqlitePool,
        provider: &str,
        operation: &str,
        audio_duration_ms: Option<i64>,
        output_chars: Option<i64>,
        api_duration_ms: i64,
        success: bool,
        error_message: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) {
        let metadata_str = metadata.map(|m| m.to_string());
        let success_int: i32 = if success { 1 } else { 0 };

        let result = sqlx::query(
            r#"
            INSERT INTO service_usage_log
            (service_type, provider, operation, input_units, output_units,
             duration_ms, success, error_message, metadata_json)
            VALUES ('stt', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(provider)
        .bind(operation)
        .bind(audio_duration_ms)
        .bind(output_chars)
        .bind(api_duration_ms)
        .bind(success_int)
        .bind(error_message)
        .bind(metadata_str)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                provider = provider,
                operation = operation,
                error = %e,
                "Failed to log STT usage (non-fatal)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_tracker_compiles() {
        // Just a compile-time check that the struct and methods are valid
        let _ = UsageTracker;
    }
}
