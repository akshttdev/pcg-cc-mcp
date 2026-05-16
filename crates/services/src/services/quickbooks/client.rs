//! Minimal QBO v3 REST client.
//!
//! Token freshness is the caller's responsibility via `from_account_refreshing`,
//! which transparently rotates an expiring access token before returning the
//! client. All `query` / `post` methods then assume the held token is good.

use chrono::{Duration, Utc};
use db::models::quickbooks_account::QuickBooksAccount;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use sqlx::SqlitePool;

use super::QboError;

const MINOR_VERSION: &str = "75";

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

/// Read QBO env credentials. Missing creds surface as `NotConfigured` so
/// route handlers can return a 503-style error instead of crashing.
fn client_credentials() -> Result<(String, String), QboError> {
    let id = std::env::var("QUICKBOOKS_CLIENT_ID")
        .map_err(|_| QboError::NotConfigured("QUICKBOOKS_CLIENT_ID".into()))?;
    let secret = std::env::var("QUICKBOOKS_CLIENT_SECRET")
        .map_err(|_| QboError::NotConfigured("QUICKBOOKS_CLIENT_SECRET".into()))?;
    Ok((id, secret))
}

pub struct QboClient {
    http: Client,
    access_token: String,
    base_url: String,
    pub realm_id: String,
}

impl QboClient {
    /// Build a client for an account, refreshing the access token in-place
    /// if it's within 5 minutes of expiry.
    pub async fn from_account_refreshing(
        pool: &SqlitePool,
        account: &QuickBooksAccount,
    ) -> Result<Self, QboError> {
        let access_token = if account.needs_token_refresh() {
            refresh_and_persist(pool, account).await?
        } else {
            account
                .access_token
                .clone()
                .ok_or(QboError::MissingAccessToken)?
        };

        Ok(Self {
            http: Client::new(),
            access_token,
            base_url: account.api_base_url(),
            realm_id: account.realm_id.clone(),
        })
    }

    /// Run a QBO SQL-ish `query` (https://developer.intuit.com/.../querying-data).
    /// Returns the raw `QueryResponse` JSON object.
    pub async fn query(&self, qbo_query: &str) -> Result<Value, QboError> {
        let url = format!(
            "{}/query?minorversion={}&query={}",
            self.base_url,
            MINOR_VERSION,
            urlencoding::encode(qbo_query)
        );

        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/json")
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(QboError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let mut body: Value = resp.json().await?;
        Ok(body["QueryResponse"].take())
    }

    /// POST a JSON body to `{base}/{entity}` and return the inserted entity.
    pub async fn create_entity(&self, entity: &str, body: &Value) -> Result<Value, QboError> {
        let url = format!(
            "{}/{}?minorversion={}",
            self.base_url,
            entity.to_lowercase(),
            MINOR_VERSION
        );

        let resp = self
            .http
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .bearer_auth(&self.access_token)
            .json(body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(QboError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let value: Value = resp.json().await?;
        Ok(value)
    }
}

/// Exchange the refresh token for a new pair, persist them, return the
/// freshly-minted access token.
async fn refresh_and_persist(
    pool: &SqlitePool,
    account: &QuickBooksAccount,
) -> Result<String, QboError> {
    let refresh = account
        .refresh_token
        .as_deref()
        .ok_or(QboError::MissingRefreshToken)?;
    let (client_id, client_secret) = client_credentials()?;

    let resp = Client::new()
        .post("https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer")
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&[("grant_type", "refresh_token"), ("refresh_token", refresh)])
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(QboError::Api {
            status: status.as_u16(),
            body,
        });
    }

    let tokens: RefreshResponse = resp
        .json()
        .await
        .map_err(|e| QboError::Parse(format!("refresh response: {e}")))?;

    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in);
    QuickBooksAccount::update_tokens(
        pool,
        account.id,
        &tokens.access_token,
        Some(&tokens.refresh_token),
        Some(expires_at),
    )
    .await?;

    Ok(tokens.access_token)
}
