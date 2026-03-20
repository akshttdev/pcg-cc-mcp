use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GatewayRequest {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub listing_id: Option<Uuid>,
    pub service_type: String,
    pub provider_node_id: Option<String>,
    pub provider_wallet: Option<String>,
    pub request_method: Option<String>,
    pub request_path: Option<String>,
    pub request_size_bytes: i64,
    pub response_size_bytes: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub vibe_charged: f64,
    pub vibe_rewarded: f64,
    pub response_ms: Option<i64>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date | null")]
    pub fulfilled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct GatewayStats {
    pub total_requests: i64,
    pub fulfilled_requests: i64,
    pub failed_requests: i64,
    pub total_vibe_charged: f64,
    pub total_vibe_rewarded: f64,
    pub avg_response_ms: Option<f64>,
    pub active_providers: i64,
}

impl GatewayRequest {
    pub async fn create(
        pool: &SqlitePool,
        subscription_id: Uuid,
        listing_id: Option<Uuid>,
        service_type: &str,
        request_method: Option<&str>,
        request_path: Option<&str>,
        request_size_bytes: i64,
    ) -> anyhow::Result<Self> {
        let id = Uuid::new_v4();

        sqlx::query_as::<_, Self>(
            "INSERT INTO gateway_requests
             (id, subscription_id, listing_id, service_type, request_method, request_path, request_size_bytes, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')
             RETURNING *"
        )
        .bind(id)
        .bind(subscription_id)
        .bind(listing_id)
        .bind(service_type)
        .bind(request_method)
        .bind(request_path)
        .bind(request_size_bytes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn mark_routing(pool: &SqlitePool, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE gateway_requests SET status = 'routing' WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // TODO: refactor into struct
    #[allow(clippy::too_many_arguments)]
    pub async fn mark_fulfilled(
        pool: &SqlitePool,
        id: Uuid,
        provider_node_id: &str,
        provider_wallet: &str,
        listing_id: Uuid,
        response_size_bytes: i64,
        response_ms: i64,
        vibe_charged: f64,
        vibe_rewarded: f64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE gateway_requests SET
             status = 'fulfilled',
             provider_node_id = ?,
             provider_wallet = ?,
             listing_id = ?,
             response_size_bytes = ?,
             response_ms = ?,
             vibe_charged = ?,
             vibe_rewarded = ?,
             fulfilled_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(provider_node_id)
        .bind(provider_wallet)
        .bind(listing_id)
        .bind(response_size_bytes)
        .bind(response_ms)
        .bind(vibe_charged)
        .bind(vibe_rewarded)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(pool: &SqlitePool, id: Uuid, error: &str) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE gateway_requests SET status = 'failed', error_message = ?, fulfilled_at = datetime('now','subsec') WHERE id = ?"
        )
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_timeout(pool: &SqlitePool, id: Uuid) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE gateway_requests SET status = 'timeout', fulfilled_at = datetime('now','subsec') WHERE id = ?"
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_for_subscription(
        pool: &SqlitePool,
        subscription_id: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM gateway_requests WHERE subscription_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(subscription_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_stats(pool: &SqlitePool) -> anyhow::Result<GatewayStats> {
        let row = sqlx::query_as::<_, (i64, i64, i64, f64, f64, Option<f64>, i64)>(
            "SELECT
               COUNT(*) as total,
               SUM(CASE WHEN status = 'fulfilled' THEN 1 ELSE 0 END) as fulfilled,
               SUM(CASE WHEN status IN ('failed','timeout') THEN 1 ELSE 0 END) as failed,
               COALESCE(SUM(vibe_charged), 0.0) as total_charged,
               COALESCE(SUM(vibe_rewarded), 0.0) as total_rewarded,
               AVG(CASE WHEN status = 'fulfilled' THEN response_ms END) as avg_ms,
               COUNT(DISTINCT provider_node_id) as active_providers
             FROM gateway_requests",
        )
        .fetch_one(pool)
        .await?;

        Ok(GatewayStats {
            total_requests: row.0,
            fulfilled_requests: row.1,
            failed_requests: row.2,
            total_vibe_charged: row.3,
            total_vibe_rewarded: row.4,
            avg_response_ms: row.5,
            active_providers: row.6,
        })
    }

    pub async fn get_subscription_stats(
        pool: &SqlitePool,
        subscription_id: Uuid,
    ) -> anyhow::Result<GatewayStats> {
        let row = sqlx::query_as::<_, (i64, i64, i64, f64, f64, Option<f64>, i64)>(
            "SELECT
               COUNT(*) as total,
               SUM(CASE WHEN status = 'fulfilled' THEN 1 ELSE 0 END) as fulfilled,
               SUM(CASE WHEN status IN ('failed','timeout') THEN 1 ELSE 0 END) as failed,
               COALESCE(SUM(vibe_charged), 0.0) as total_charged,
               COALESCE(SUM(vibe_rewarded), 0.0) as total_rewarded,
               AVG(CASE WHEN status = 'fulfilled' THEN response_ms END) as avg_ms,
               COUNT(DISTINCT provider_node_id) as active_providers
             FROM gateway_requests WHERE subscription_id = ?",
        )
        .bind(subscription_id)
        .fetch_one(pool)
        .await?;

        Ok(GatewayStats {
            total_requests: row.0,
            fulfilled_requests: row.1,
            failed_requests: row.2,
            total_vibe_charged: row.3,
            total_vibe_rewarded: row.4,
            avg_response_ms: row.5,
            active_providers: row.6,
        })
    }
}
