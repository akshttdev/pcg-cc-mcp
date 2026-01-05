use chrono::{DateTime, Utc};
use glob::Pattern;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BrowserAllowlistError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Allowlist entry not found")]
    NotFound,
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("URL not allowed: {0}")]
    UrlNotAllowed(String),
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "pattern_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    Glob,
    Regex,
    Exact,
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::Glob => write!(f, "glob"),
            PatternType::Regex => write!(f, "regex"),
            PatternType::Exact => write!(f, "exact"),
        }
    }
}

impl Default for PatternType {
    fn default() -> Self {
        PatternType::Glob
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct BrowserAllowlist {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub pattern: String,
    pub pattern_type: PatternType,
    pub description: Option<String>,
    pub is_global: bool,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateBrowserAllowlist {
    pub project_id: Option<Uuid>,
    pub pattern: String,
    #[serde(default)]
    pub pattern_type: PatternType,
    pub description: Option<String>,
    #[serde(default)]
    pub is_global: bool,
    pub created_by: Option<String>,
}

impl BrowserAllowlist {
    /// Create a new allowlist entry
    pub async fn create(
        pool: &SqlitePool,
        data: CreateBrowserAllowlist,
    ) -> Result<Self, BrowserAllowlistError> {
        // Validate pattern
        Self::validate_pattern(&data.pattern, &data.pattern_type)?;

        let id = Uuid::new_v4();
        let pattern_type_str = data.pattern_type.to_string();

        let entry = sqlx::query_as::<_, BrowserAllowlist>(
            r#"
            INSERT INTO browser_allowlist (id, project_id, pattern, pattern_type, description, is_global, created_by)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.pattern)
        .bind(pattern_type_str)
        .bind(&data.description)
        .bind(data.is_global)
        .bind(&data.created_by)
        .fetch_one(pool)
        .await?;

        Ok(entry)
    }

    /// Find entry by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, BrowserAllowlistError> {
        let entry = sqlx::query_as::<_, BrowserAllowlist>(
            r#"SELECT * FROM browser_allowlist WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(entry)
    }

    /// Get all entries for a project (including global entries)
    pub async fn find_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, BrowserAllowlistError> {
        let entries = sqlx::query_as::<_, BrowserAllowlist>(
            r#"
            SELECT * FROM browser_allowlist
            WHERE project_id = ?1 OR is_global = 1
            ORDER BY is_global DESC, created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(entries)
    }

    /// Get global entries only
    pub async fn find_global(pool: &SqlitePool) -> Result<Vec<Self>, BrowserAllowlistError> {
        let entries = sqlx::query_as::<_, BrowserAllowlist>(
            r#"
            SELECT * FROM browser_allowlist
            WHERE is_global = 1
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(entries)
    }

    /// Get project-specific entries only
    pub async fn find_project_only(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, BrowserAllowlistError> {
        let entries = sqlx::query_as::<_, BrowserAllowlist>(
            r#"
            SELECT * FROM browser_allowlist
            WHERE project_id = ?1 AND is_global = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(entries)
    }

    /// Delete an allowlist entry
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), BrowserAllowlistError> {
        sqlx::query(r#"DELETE FROM browser_allowlist WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Validate a pattern based on its type
    fn validate_pattern(pattern: &str, pattern_type: &PatternType) -> Result<(), BrowserAllowlistError> {
        match pattern_type {
            PatternType::Glob => {
                Pattern::new(pattern)
                    .map_err(|e| BrowserAllowlistError::InvalidPattern(e.to_string()))?;
            }
            PatternType::Regex => {
                Regex::new(pattern)
                    .map_err(|e| BrowserAllowlistError::InvalidPattern(e.to_string()))?;
            }
            PatternType::Exact => {
                // Exact patterns are always valid
            }
        }
        Ok(())
    }

    /// Check if a URL matches this pattern
    pub fn matches(&self, url: &str) -> bool {
        // Extract host:port from URL for matching
        let host_port = match Url::parse(url) {
            Ok(parsed) => {
                let host = parsed.host_str().unwrap_or("");
                let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
                format!("{}{}", host, port)
            }
            Err(_) => url.to_string(),
        };

        match self.pattern_type {
            PatternType::Glob => {
                Pattern::new(&self.pattern)
                    .map(|p| p.matches(&host_port) || p.matches(url))
                    .unwrap_or(false)
            }
            PatternType::Regex => {
                Regex::new(&self.pattern)
                    .map(|r| r.is_match(&host_port) || r.is_match(url))
                    .unwrap_or(false)
            }
            PatternType::Exact => {
                host_port == self.pattern || url == self.pattern
            }
        }
    }

    /// Check if a URL is allowed by any of the provided allowlist entries
    pub fn is_url_allowed(entries: &[Self], url: &str) -> bool {
        entries.iter().any(|entry| entry.matches(url))
    }

    /// Check URL and return error if not allowed
    pub fn check_url(entries: &[Self], url: &str) -> Result<(), BrowserAllowlistError> {
        if Self::is_url_allowed(entries, url) {
            Ok(())
        } else {
            Err(BrowserAllowlistError::UrlNotAllowed(url.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_pattern_matches() {
        let entry = BrowserAllowlist {
            id: Uuid::new_v4(),
            project_id: None,
            pattern: "localhost:*".to_string(),
            pattern_type: PatternType::Glob,
            description: None,
            is_global: true,
            created_by: None,
            created_at: Utc::now(),
        };

        assert!(entry.matches("http://localhost:3000"));
        assert!(entry.matches("http://localhost:8080/some/path"));
        assert!(!entry.matches("http://example.com:3000"));
    }

    #[test]
    fn test_exact_pattern_matches() {
        let entry = BrowserAllowlist {
            id: Uuid::new_v4(),
            project_id: None,
            pattern: "staging.myapp.com".to_string(),
            pattern_type: PatternType::Exact,
            description: None,
            is_global: false,
            created_by: None,
            created_at: Utc::now(),
        };

        assert!(entry.matches("https://staging.myapp.com/path"));
        assert!(!entry.matches("https://prod.myapp.com"));
    }
}
