use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MarketplaceListing {
    pub id: Uuid,
    pub provider_type: String,
    pub provider_node_id: Option<String>,
    pub provider_org_id: Option<Uuid>,
    pub provider_wallet: String,
    pub service_type: String,
    pub service_name: String,
    pub service_description: Option<String>,
    pub tags: String, // JSON array
    pub pricing_model: String,
    pub price_vibe: f64,
    pub min_vibe: f64,
    pub max_concurrent: Option<i64>,
    pub endpoint_url: Option<String>,
    pub region: Option<String>,
    pub status: String,
    pub uptime_pct: Option<f64>,
    pub avg_response_ms: Option<i64>,
    pub metadata: String, // JSON object
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateListing {
    pub provider_type: Option<String>, // defaults to 'apn_node'
    pub provider_node_id: Option<String>,
    pub provider_org_id: Option<Uuid>,
    pub provider_wallet: String,
    pub service_type: String,
    pub service_name: String,
    pub service_description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pricing_model: Option<String>, // defaults to 'per-request'
    pub price_vibe: f64,
    pub min_vibe: Option<f64>,
    pub max_concurrent: Option<i64>,
    pub endpoint_url: Option<String>,
    pub region: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateListing {
    pub service_name: Option<String>,
    pub service_description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pricing_model: Option<String>,
    pub price_vibe: Option<f64>,
    pub min_vibe: Option<f64>,
    pub max_concurrent: Option<i64>,
    pub endpoint_url: Option<String>,
    pub region: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl MarketplaceListing {
    pub fn tags_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.tags).unwrap_or_default()
    }

    pub fn metadata_value(&self) -> serde_json::Value {
        serde_json::from_str(&self.metadata).unwrap_or(serde_json::json!({}))
    }

    pub async fn create(pool: &SqlitePool, data: CreateListing) -> anyhow::Result<Self> {
        let id = Uuid::new_v4();
        let provider_type = data.provider_type.unwrap_or_else(|| "apn_node".to_string());
        let pricing_model = data
            .pricing_model
            .unwrap_or_else(|| "per-request".to_string());
        let tags = serde_json::to_string(&data.tags.unwrap_or_default())?;
        let metadata = serde_json::to_string(&data.metadata.unwrap_or(serde_json::json!({})))?;
        let min_vibe = data.min_vibe.unwrap_or(0.0);

        let row = sqlx::query_as::<_, Self>(
            "INSERT INTO marketplace_listings
             (id, provider_type, provider_node_id, provider_org_id, provider_wallet,
              service_type, service_name, service_description, tags,
              pricing_model, price_vibe, min_vibe, max_concurrent,
              endpoint_url, region, status, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', ?)
             RETURNING *",
        )
        .bind(id)
        .bind(&provider_type)
        .bind(&data.provider_node_id)
        .bind(data.provider_org_id)
        .bind(&data.provider_wallet)
        .bind(&data.service_type)
        .bind(&data.service_name)
        .bind(&data.service_description)
        .bind(&tags)
        .bind(&pricing_model)
        .bind(data.price_vibe)
        .bind(min_vibe)
        .bind(data.max_concurrent)
        .bind(&data.endpoint_url)
        .bind(&data.region)
        .bind(&metadata)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn list_active(
        pool: &SqlitePool,
        service_type: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<Self>> {
        if let Some(stype) = service_type {
            sqlx::query_as::<_, Self>(
                "SELECT * FROM marketplace_listings WHERE status = 'active' AND service_type = ?
                 ORDER BY price_vibe ASC LIMIT ?",
            )
            .bind(stype)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
        } else {
            sqlx::query_as::<_, Self>(
                "SELECT * FROM marketplace_listings WHERE status = 'active'
                 ORDER BY service_type ASC, price_vibe ASC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
        }
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> anyhow::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM marketplace_listings WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_node(pool: &SqlitePool, node_id: &str) -> anyhow::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM marketplace_listings WHERE provider_node_id = ? ORDER BY created_at DESC"
        )
        .bind(node_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find best available provider for a service type (lowest price, active)
    pub async fn find_best_for_service(
        pool: &SqlitePool,
        service_type: &str,
    ) -> anyhow::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM marketplace_listings
             WHERE status = 'active' AND service_type = ?
             ORDER BY price_vibe ASC, avg_response_ms ASC NULLS LAST
             LIMIT 1",
        )
        .bind(service_type)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateListing,
    ) -> anyhow::Result<Option<Self>> {
        let current = Self::find_by_id(pool, id).await?;
        let Some(current) = current else {
            return Ok(None);
        };

        let tags = data
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_else(|_| current.tags.clone()))
            .unwrap_or(current.tags);
        let metadata = data
            .metadata
            .map(|m| serde_json::to_string(&m).unwrap_or_else(|_| current.metadata.clone()))
            .unwrap_or(current.metadata);

        let row = sqlx::query_as::<_, Self>(
            "UPDATE marketplace_listings SET
             service_name = COALESCE(?, service_name),
             service_description = COALESCE(?, service_description),
             tags = ?,
             pricing_model = COALESCE(?, pricing_model),
             price_vibe = COALESCE(?, price_vibe),
             min_vibe = COALESCE(?, min_vibe),
             max_concurrent = COALESCE(?, max_concurrent),
             endpoint_url = COALESCE(?, endpoint_url),
             region = COALESCE(?, region),
             status = COALESCE(?, status),
             metadata = ?,
             updated_at = datetime('now','subsec')
             WHERE id = ?
             RETURNING *",
        )
        .bind(data.service_name)
        .bind(data.service_description)
        .bind(&tags)
        .bind(data.pricing_model)
        .bind(data.price_vibe)
        .bind(data.min_vibe)
        .bind(data.max_concurrent)
        .bind(data.endpoint_url)
        .bind(data.region)
        .bind(data.status)
        .bind(&metadata)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn update_stats(
        pool: &SqlitePool,
        id: Uuid,
        uptime_pct: Option<f64>,
        avg_response_ms: Option<i64>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE marketplace_listings SET
             uptime_pct = COALESCE(?, uptime_pct),
             avg_response_ms = COALESCE(?, avg_response_ms),
             updated_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(uptime_pct)
        .bind(avg_response_ms)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE marketplace_listings SET status = 'inactive', updated_at = datetime('now','subsec') WHERE id = ?"
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
