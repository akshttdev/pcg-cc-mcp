//! Deployment Tools Implementation
//!
//! Tools for infrastructure deployment, DNS management, and domain configuration.
//! Used primarily by Launch agent for production deployments.

use crate::services::agent_tools::ToolResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// Cloudflare Client
// ============================================================================

/// Cloudflare API client configuration
#[derive(Debug, Clone)]
pub struct CloudflareConfig {
    pub api_token: String,
    pub account_id: String,
    pub zone_id: Option<String>,
}

/// Cloudflare DNS record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: String,
    pub record_type: String,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
}

/// Cloudflare zone information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareZone {
    pub id: String,
    pub name: String,
    pub status: String,
    pub nameservers: Vec<String>,
}

/// Cloudflare Pages deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagesDeployment {
    pub id: String,
    pub project_name: String,
    pub url: String,
    pub production_url: Option<String>,
    pub status: String,
    pub created_on: String,
}

/// Cloudflare API client
#[derive(Clone)]
pub struct CloudflareClient {
    config: CloudflareConfig,
    client: reqwest::Client,
}

impl CloudflareClient {
    const API_BASE: &'static str = "https://api.cloudflare.com/client/v4";

    pub fn new(config: CloudflareConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_token)
    }

    /// List all zones in account
    pub async fn list_zones(&self) -> Result<Vec<CloudflareZone>, CloudflareError> {
        let url = format!("{}/zones?account.id={}", Self::API_BASE, self.config.account_id);

        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let zones = data.get("result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| CloudflareError::ParseError("Missing result array".to_string()))?;

        let mut result = Vec::new();
        for zone in zones {
            result.push(CloudflareZone {
                id: zone.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: zone.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                status: zone.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                nameservers: zone.get("name_servers")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
            });
        }

        Ok(result)
    }

    /// Create a new DNS record
    pub async fn create_dns_record(
        &self,
        zone_id: &str,
        record_type: &str,
        name: &str,
        content: &str,
        ttl: u32,
        proxied: bool,
    ) -> Result<DnsRecord, CloudflareError> {
        let url = format!("{}/zones/{}/dns_records", Self::API_BASE, zone_id);

        let body = serde_json::json!({
            "type": record_type,
            "name": name,
            "content": content,
            "ttl": ttl,
            "proxied": proxied
        });

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CloudflareError::ApiError(format!("API error: {}", error_text)));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let record = data.get("result")
            .ok_or_else(|| CloudflareError::ParseError("Missing result".to_string()))?;

        Ok(DnsRecord {
            id: record.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            record_type: record.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: record.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content: record.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ttl: record.get("ttl").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
            proxied: record.get("proxied").and_then(|v| v.as_bool()).unwrap_or(false),
        })
    }

    /// List DNS records for a zone
    pub async fn list_dns_records(&self, zone_id: &str) -> Result<Vec<DnsRecord>, CloudflareError> {
        let url = format!("{}/zones/{}/dns_records", Self::API_BASE, zone_id);

        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let records = data.get("result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| CloudflareError::ParseError("Missing result array".to_string()))?;

        let mut result = Vec::new();
        for record in records {
            result.push(DnsRecord {
                id: record.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                record_type: record.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: record.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                content: record.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                ttl: record.get("ttl").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                proxied: record.get("proxied").and_then(|v| v.as_bool()).unwrap_or(false),
            });
        }

        Ok(result)
    }

    /// Delete a DNS record
    pub async fn delete_dns_record(&self, zone_id: &str, record_id: &str) -> Result<(), CloudflareError> {
        let url = format!("{}/zones/{}/dns_records/{}", Self::API_BASE, zone_id, record_id);

        let response = self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        Ok(())
    }

    /// Create a Pages project
    pub async fn create_pages_project(
        &self,
        name: &str,
        production_branch: &str,
    ) -> Result<serde_json::Value, CloudflareError> {
        let url = format!("{}/accounts/{}/pages/projects", Self::API_BASE, self.config.account_id);

        let body = serde_json::json!({
            "name": name,
            "production_branch": production_branch
        });

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CloudflareError::ApiError(format!("API error: {}", error_text)));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        data.get("result").cloned()
            .ok_or_else(|| CloudflareError::ParseError("Missing result".to_string()))
    }

    /// Get Pages deployment status
    pub async fn get_pages_deployment(
        &self,
        project_name: &str,
        deployment_id: &str,
    ) -> Result<PagesDeployment, CloudflareError> {
        let url = format!(
            "{}/accounts/{}/pages/projects/{}/deployments/{}",
            Self::API_BASE, self.config.account_id, project_name, deployment_id
        );

        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let deployment = data.get("result")
            .ok_or_else(|| CloudflareError::ParseError("Missing result".to_string()))?;

        Ok(PagesDeployment {
            id: deployment.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            project_name: project_name.to_string(),
            url: deployment.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            production_url: deployment.get("aliases")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(String::from),
            status: deployment.get("latest_stage")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            created_on: deployment.get("created_on").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        })
    }

    /// Purge cache for a zone
    pub async fn purge_cache(&self, zone_id: &str, purge_all: bool) -> Result<(), CloudflareError> {
        let url = format!("{}/zones/{}/purge_cache", Self::API_BASE, zone_id);

        let body = if purge_all {
            serde_json::json!({"purge_everything": true})
        } else {
            serde_json::json!({})
        };

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum CloudflareError {
    RequestFailed(String),
    ApiError(String),
    ParseError(String),
}

// ============================================================================
// SSL Certificate Manager
// ============================================================================

/// SSL certificate status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslStatus {
    pub zone_id: String,
    pub status: String,
    pub certificate_authority: String,
    pub validation_method: String,
    pub hosts: Vec<String>,
}

/// SSL certificate manager using Cloudflare
pub struct SslManager {
    cloudflare: CloudflareClient,
}

impl SslManager {
    pub fn new(cloudflare: CloudflareClient) -> Self {
        Self { cloudflare }
    }

    /// Get SSL status for a zone
    pub async fn get_status(&self, zone_id: &str) -> Result<SslStatus, CloudflareError> {
        let url = format!(
            "{}/zones/{}/ssl/certificate_packs?status=active",
            CloudflareClient::API_BASE, zone_id
        );

        let response = self.cloudflare.client
            .get(&url)
            .header("Authorization", self.cloudflare.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let packs = data.get("result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| CloudflareError::ParseError("Missing result array".to_string()))?;

        if let Some(pack) = packs.first() {
            Ok(SslStatus {
                zone_id: zone_id.to_string(),
                status: pack.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                certificate_authority: pack.get("certificate_authority")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                validation_method: pack.get("validation_method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                hosts: pack.get("hosts")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
            })
        } else {
            Ok(SslStatus {
                zone_id: zone_id.to_string(),
                status: "no_certificate".to_string(),
                certificate_authority: "none".to_string(),
                validation_method: "none".to_string(),
                hosts: vec![],
            })
        }
    }

    /// Order an advanced SSL certificate
    pub async fn order_certificate(
        &self,
        zone_id: &str,
        hosts: &[&str],
    ) -> Result<SslStatus, CloudflareError> {
        let url = format!("{}/zones/{}/ssl/certificate_packs/order", CloudflareClient::API_BASE, zone_id);

        let body = serde_json::json!({
            "type": "advanced",
            "hosts": hosts,
            "validation_method": "txt",
            "validity_days": 365,
            "certificate_authority": "lets_encrypt"
        });

        let response = self.cloudflare.client
            .post(&url)
            .header("Authorization", self.cloudflare.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CloudflareError::ApiError(format!("API error: {}", error_text)));
        }

        // Return status after ordering
        self.get_status(zone_id).await
    }
}

// ============================================================================
// Domain Registrar Interface
// ============================================================================

/// Domain availability check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainAvailability {
    pub domain: String,
    pub available: bool,
    pub premium: bool,
    pub price: Option<f64>,
    pub currency: String,
}

/// Domain registration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRegistration {
    pub domain: String,
    pub status: String,
    pub expires_at: String,
    pub auto_renew: bool,
    pub nameservers: Vec<String>,
}

/// Domain registrar client (using Cloudflare Registrar)
pub struct DomainRegistrar {
    cloudflare: CloudflareClient,
}

impl DomainRegistrar {
    pub fn new(cloudflare: CloudflareClient) -> Self {
        Self { cloudflare }
    }

    /// Check domain availability
    pub async fn check_availability(&self, domain: &str) -> Result<DomainAvailability, CloudflareError> {
        let url = format!(
            "{}/accounts/{}/registrar/domains/availability?domain={}",
            CloudflareClient::API_BASE, self.cloudflare.config.account_id, domain
        );

        let response = self.cloudflare.client
            .get(&url)
            .header("Authorization", self.cloudflare.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let result = data.get("result")
            .ok_or_else(|| CloudflareError::ParseError("Missing result".to_string()))?;

        Ok(DomainAvailability {
            domain: domain.to_string(),
            available: result.get("available").and_then(|v| v.as_bool()).unwrap_or(false),
            premium: result.get("premium").and_then(|v| v.as_bool()).unwrap_or(false),
            price: result.get("price").and_then(|v| v.as_f64()),
            currency: result.get("currency").and_then(|v| v.as_str()).unwrap_or("USD").to_string(),
        })
    }

    /// Register a domain
    pub async fn register_domain(
        &self,
        domain: &str,
        auto_renew: bool,
    ) -> Result<DomainRegistration, CloudflareError> {
        let url = format!(
            "{}/accounts/{}/registrar/domains",
            CloudflareClient::API_BASE, self.cloudflare.config.account_id
        );

        let body = serde_json::json!({
            "domain": domain,
            "auto_renew": auto_renew
        });

        let response = self.cloudflare.client
            .post(&url)
            .header("Authorization", self.cloudflare.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CloudflareError::ApiError(format!("API error: {}", error_text)));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let result = data.get("result")
            .ok_or_else(|| CloudflareError::ParseError("Missing result".to_string()))?;

        Ok(DomainRegistration {
            domain: domain.to_string(),
            status: result.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string(),
            expires_at: result.get("expires_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            auto_renew,
            nameservers: result.get("name_servers")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }

    /// Get domain status
    pub async fn get_domain(&self, domain: &str) -> Result<DomainRegistration, CloudflareError> {
        let url = format!(
            "{}/accounts/{}/registrar/domains/{}",
            CloudflareClient::API_BASE, self.cloudflare.config.account_id, domain
        );

        let response = self.cloudflare.client
            .get(&url)
            .header("Authorization", self.cloudflare.auth_header())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CloudflareError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CloudflareError::ParseError(e.to_string()))?;

        let result = data.get("result")
            .ok_or_else(|| CloudflareError::ParseError("Missing result".to_string()))?;

        Ok(DomainRegistration {
            domain: domain.to_string(),
            status: result.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            expires_at: result.get("expires_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            auto_renew: result.get("auto_renew").and_then(|v| v.as_bool()).unwrap_or(false),
            nameservers: result.get("name_servers")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }
}

// ============================================================================
// Deployment Tools (High-level interface)
// ============================================================================

/// High-level deployment tools interface for Launch agent
pub struct DeploymentTools {
    pub cloudflare: Option<CloudflareClient>,
    pub ssl_manager: Option<SslManager>,
    pub domain_registrar: Option<DomainRegistrar>,
}

impl DeploymentTools {
    pub fn new(cloudflare_config: Option<CloudflareConfig>) -> Self {
        let cloudflare = cloudflare_config.clone().map(CloudflareClient::new);
        let ssl_manager = cloudflare.clone().map(SslManager::new);
        let domain_registrar = cloudflare.clone().map(DomainRegistrar::new);

        Self {
            cloudflare,
            ssl_manager,
            domain_registrar,
        }
    }

    /// Execute a deployment tool by name
    pub async fn execute(&self, tool_name: &str, params: serde_json::Value) -> ToolResult {
        let start = Instant::now();

        let result = match tool_name {
            "check_domain" => self.execute_check_domain(params).await,
            "create_dns_record" => self.execute_create_dns_record(params).await,
            "list_dns_records" => self.execute_list_dns_records(params).await,
            "get_ssl_status" => self.execute_get_ssl_status(params).await,
            "purge_cache" => self.execute_purge_cache(params).await,
            "list_zones" => self.execute_list_zones().await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(data) => ToolResult {
                success: true,
                tool_name: tool_name.to_string(),
                agent_name: "deployment".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data,
                error: None,
            },
            Err(error) => ToolResult {
                success: false,
                tool_name: tool_name.to_string(),
                agent_name: "deployment".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data: serde_json::json!({}),
                error: Some(error),
            },
        }
    }

    async fn execute_check_domain(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let domain = params.get("domain")
            .and_then(|v| v.as_str())
            .ok_or("Missing domain parameter")?;

        let registrar = self.domain_registrar.as_ref()
            .ok_or("Domain registrar not configured")?;

        let result = registrar.check_availability(domain).await
            .map_err(|e| format!("Check failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn execute_create_dns_record(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let zone_id = params.get("zone_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing zone_id parameter")?;
        let record_type = params.get("type")
            .and_then(|v| v.as_str())
            .ok_or("Missing type parameter")?;
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing name parameter")?;
        let content = params.get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing content parameter")?;
        let ttl = params.get("ttl")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        let proxied = params.get("proxied")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let cloudflare = self.cloudflare.as_ref()
            .ok_or("Cloudflare not configured")?;

        let result = cloudflare.create_dns_record(zone_id, record_type, name, content, ttl, proxied).await
            .map_err(|e| format!("Create DNS record failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn execute_list_dns_records(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let zone_id = params.get("zone_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing zone_id parameter")?;

        let cloudflare = self.cloudflare.as_ref()
            .ok_or("Cloudflare not configured")?;

        let result = cloudflare.list_dns_records(zone_id).await
            .map_err(|e| format!("List DNS records failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn execute_get_ssl_status(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let zone_id = params.get("zone_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing zone_id parameter")?;

        let ssl_manager = self.ssl_manager.as_ref()
            .ok_or("SSL manager not configured")?;

        let result = ssl_manager.get_status(zone_id).await
            .map_err(|e| format!("Get SSL status failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn execute_purge_cache(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let zone_id = params.get("zone_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing zone_id parameter")?;
        let purge_all = params.get("purge_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let cloudflare = self.cloudflare.as_ref()
            .ok_or("Cloudflare not configured")?;

        cloudflare.purge_cache(zone_id, purge_all).await
            .map_err(|e| format!("Purge cache failed: {:?}", e))?;

        Ok(serde_json::json!({"status": "purged", "zone_id": zone_id}))
    }

    async fn execute_list_zones(&self) -> Result<serde_json::Value, String> {
        let cloudflare = self.cloudflare.as_ref()
            .ok_or("Cloudflare not configured")?;

        let result = cloudflare.list_zones().await
            .map_err(|e| format!("List zones failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }
}
