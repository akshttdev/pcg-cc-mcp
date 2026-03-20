//! UUID parameter parsing helpers
//!
//! Centralizes the common pattern of parsing a string path parameter into a DbUuid
//! with a consistent BadRequest error message.

use db::db_uuid::DbUuid;

use crate::error::ApiError;

/// Parse a string parameter (typically from a URL path) into a `DbUuid`.
///
/// Returns `ApiError::BadRequest` with a descriptive message on failure.
///
/// # Example
/// ```ignore
/// let id = parse_db_uuid_param(&id, "deal ID")?;
/// ```
pub fn parse_db_uuid_param(value: &str, param_name: &str) -> Result<DbUuid, ApiError> {
    DbUuid::parse(value)
        .map_err(|_| ApiError::BadRequest(format!("Invalid {}: not a valid UUID", param_name)))
}
