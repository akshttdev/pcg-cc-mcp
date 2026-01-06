use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

const AIRTABLE_API_BASE: &str = "https://api.airtable.com/v0";
const AIRTABLE_META_API_BASE: &str = "https://api.airtable.com/v0/meta";

#[derive(Debug, Error, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirtableServiceError {
    #[error("Airtable Personal Access Token not configured")]
    NotConfigured,
    #[error("Airtable authentication failed - invalid token")]
    AuthFailed,
    #[error("Airtable rate limit exceeded")]
    RateLimited,
    #[error("Airtable base not found: {0}")]
    BaseNotFound(String),
    #[error("Airtable record not found: {0}")]
    RecordNotFound(String),
    #[error("Airtable table not found: {0}")]
    TableNotFound(String),
    #[error("Airtable API error: {0}")]
    ApiError(String),
    #[ts(skip)]
    #[serde(skip)]
    #[error("HTTP request error: {0}")]
    Request(String),
}

impl AirtableServiceError {
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            AirtableServiceError::RateLimited | AirtableServiceError::Request(_)
        )
    }

    pub fn is_api_error(&self) -> bool {
        matches!(
            self,
            AirtableServiceError::AuthFailed
                | AirtableServiceError::BaseNotFound(_)
                | AirtableServiceError::RecordNotFound(_)
                | AirtableServiceError::TableNotFound(_)
        )
    }
}

impl From<reqwest::Error> for AirtableServiceError {
    fn from(err: reqwest::Error) -> Self {
        AirtableServiceError::Request(err.to_string())
    }
}

// Airtable API response types

/// User info returned from whoami endpoint
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableUserInfo {
    pub id: String,
    pub email: Option<String>,
}

/// Base info returned from list bases
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableBaseInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "permissionLevel")]
    pub permission_level: String,
}

/// Response wrapper for bases list
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BasesResponse {
    bases: Vec<AirtableBaseInfo>,
    offset: Option<String>,
}

/// Table info
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableTable {
    pub id: String,
    pub name: String,
    #[serde(rename = "primaryFieldId")]
    pub primary_field_id: Option<String>,
    #[serde(default)]
    pub fields: Vec<AirtableField>,
    #[serde(default)]
    pub views: Vec<AirtableView>,
}

/// Tables response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TablesResponse {
    tables: Vec<AirtableTable>,
}

/// Field info
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableField {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub options: Option<serde_json::Value>,
}

/// View info
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableView {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub view_type: String,
}

/// Record from Airtable
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableRecord {
    pub id: String,
    pub fields: serde_json::Value,
    #[serde(rename = "createdTime")]
    pub created_time: String,
}

/// Response wrapper for records list
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordsResponse {
    records: Vec<AirtableRecord>,
    offset: Option<String>,
}

/// Response for creating/updating a record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct RecordResponse {
    id: String,
    fields: serde_json::Value,
    #[serde(rename = "createdTime")]
    created_time: String,
}

/// Comment on an Airtable record
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableComment {
    pub id: String,
    pub author: AirtableCommentAuthor,
    pub text: String,
    #[serde(rename = "createdTime")]
    pub created_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AirtableCommentAuthor {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Request to create a record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecordRequest {
    pub fields: serde_json::Value,
}

/// Request to update a record
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
pub struct AirtableRecordUpdate {
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct AirtableService {
    client: Client,
    token: String,
}

impl AirtableService {
    /// Create a new Airtable service with a Personal Access Token
    pub fn new(token: &str) -> Result<Self, AirtableServiceError> {
        if token.is_empty() {
            return Err(AirtableServiceError::NotConfigured);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AirtableServiceError::Request(e.to_string()))?;

        Ok(Self {
            client,
            token: token.to_string(),
        })
    }

    /// Build authorization header value
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Verify the API token by fetching user info
    pub async fn verify_credentials(&self) -> Result<AirtableUserInfo, AirtableServiceError> {
        let url = format!("{}/whoami", AIRTABLE_META_API_BASE);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(AirtableServiceError::AuthFailed);
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))
    }

    /// Get all bases for the authenticated user
    pub async fn list_my_bases(&self) -> Result<Vec<AirtableBaseInfo>, AirtableServiceError> {
        (|| async { self.list_my_bases_internal().await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .notify(|err: &AirtableServiceError, dur: Duration| {
                tracing::warn!(
                    "Airtable API call failed, retrying after {:.2}s: {}",
                    dur.as_secs_f64(),
                    err
                );
            })
            .await
    }

    async fn list_my_bases_internal(&self) -> Result<Vec<AirtableBaseInfo>, AirtableServiceError> {
        let url = format!("{}/bases", AIRTABLE_META_API_BASE);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == 401 || response.status() == 403 {
            return Err(AirtableServiceError::AuthFailed);
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        let bases_response: BasesResponse = response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))?;

        Ok(bases_response.bases)
    }

    /// Get tables in a base
    pub async fn get_base_tables(
        &self,
        base_id: &str,
    ) -> Result<Vec<AirtableTable>, AirtableServiceError> {
        (|| async { self.get_base_tables_internal(base_id).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .notify(|err: &AirtableServiceError, dur: Duration| {
                tracing::warn!(
                    "Airtable API call failed, retrying after {:.2}s: {}",
                    dur.as_secs_f64(),
                    err
                );
            })
            .await
    }

    async fn get_base_tables_internal(
        &self,
        base_id: &str,
    ) -> Result<Vec<AirtableTable>, AirtableServiceError> {
        let url = format!("{}/bases/{}/tables", AIRTABLE_META_API_BASE, base_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::BaseNotFound(base_id.to_string()));
        }

        if response.status() == 401 || response.status() == 403 {
            return Err(AirtableServiceError::AuthFailed);
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        let tables_response: TablesResponse = response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))?;

        Ok(tables_response.tables)
    }

    /// Get records from a table
    pub async fn get_table_records(
        &self,
        base_id: &str,
        table_id: &str,
    ) -> Result<Vec<AirtableRecord>, AirtableServiceError> {
        (|| async { self.get_table_records_internal(base_id, table_id).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .await
    }

    async fn get_table_records_internal(
        &self,
        base_id: &str,
        table_id: &str,
    ) -> Result<Vec<AirtableRecord>, AirtableServiceError> {
        let url = format!("{}/{}/{}", AIRTABLE_API_BASE, base_id, table_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::TableNotFound(table_id.to_string()));
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        let records_response: RecordsResponse = response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))?;

        Ok(records_response.records)
    }

    /// Get a single record by ID
    pub async fn get_record(
        &self,
        base_id: &str,
        table_id: &str,
        record_id: &str,
    ) -> Result<AirtableRecord, AirtableServiceError> {
        let url = format!(
            "{}/{}/{}/{}",
            AIRTABLE_API_BASE, base_id, table_id, record_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::RecordNotFound(record_id.to_string()));
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))
    }

    /// Create a new record in a table
    pub async fn create_record(
        &self,
        base_id: &str,
        table_id: &str,
        fields: serde_json::Value,
    ) -> Result<AirtableRecord, AirtableServiceError> {
        let url = format!("{}/{}/{}", AIRTABLE_API_BASE, base_id, table_id);

        let body = serde_json::json!({
            "fields": fields
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::TableNotFound(table_id.to_string()));
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))
    }

    /// Update a record
    pub async fn update_record(
        &self,
        base_id: &str,
        table_id: &str,
        record_id: &str,
        fields: serde_json::Value,
    ) -> Result<AirtableRecord, AirtableServiceError> {
        let url = format!(
            "{}/{}/{}/{}",
            AIRTABLE_API_BASE, base_id, table_id, record_id
        );

        let body = serde_json::json!({
            "fields": fields
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::RecordNotFound(record_id.to_string()));
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))
    }

    /// Add a comment to a record
    /// Note: Comments API requires base ID and record ID
    pub async fn add_record_comment(
        &self,
        base_id: &str,
        record_id: &str,
        text: &str,
    ) -> Result<AirtableComment, AirtableServiceError> {
        // Airtable comments use a different endpoint structure
        let comments_url = format!(
            "https://api.airtable.com/v0/{}/comments/{}",
            base_id, record_id
        );

        let body = serde_json::json!({
            "text": text
        });

        let response = self
            .client
            .post(&comments_url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::RecordNotFound(record_id.to_string()));
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| AirtableServiceError::ApiError(e.to_string()))
    }

    /// Delete a record
    pub async fn delete_record(
        &self,
        base_id: &str,
        table_id: &str,
        record_id: &str,
    ) -> Result<(), AirtableServiceError> {
        let url = format!(
            "{}/{}/{}/{}",
            AIRTABLE_API_BASE, base_id, table_id, record_id
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == 404 {
            return Err(AirtableServiceError::RecordNotFound(record_id.to_string()));
        }

        if response.status() == 429 {
            return Err(AirtableServiceError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AirtableServiceError::ApiError(error_text));
        }

        Ok(())
    }
}

/// Helper to extract the primary field (name) from a record's fields
pub fn get_record_name(record: &AirtableRecord, primary_field_name: &str) -> Option<String> {
    record
        .fields
        .get(primary_field_name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Helper to extract a text field from a record
pub fn get_record_field_text(record: &AirtableRecord, field_name: &str) -> Option<String> {
    record
        .fields
        .get(field_name)
        .and_then(|v| {
            if v.is_string() {
                v.as_str().map(|s| s.to_string())
            } else {
                // Could be rich text or other format
                Some(v.to_string())
            }
        })
}

/// Build a URL to view a record in Airtable
pub fn build_record_url(base_id: &str, table_id: &str, record_id: &str) -> String {
    format!(
        "https://airtable.com/{}/{}/{}",
        base_id, table_id, record_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_should_retry() {
        assert!(AirtableServiceError::RateLimited.should_retry());
        assert!(AirtableServiceError::Request("timeout".to_string()).should_retry());
        assert!(!AirtableServiceError::AuthFailed.should_retry());
        assert!(!AirtableServiceError::BaseNotFound("123".to_string()).should_retry());
    }

    #[test]
    fn test_create_service_empty_token() {
        let result = AirtableService::new("");
        assert!(matches!(result, Err(AirtableServiceError::NotConfigured)));
    }

    #[test]
    fn test_build_record_url() {
        let url = build_record_url("appXXX", "tblYYY", "recZZZ");
        assert_eq!(url, "https://airtable.com/appXXX/tblYYY/recZZZ");
    }
}
