//! # DbUuid — Dual-format UUID for SQLite
//!
//! A UUID type that transparently reads both BLOB (16-byte) and TEXT (36-char)
//! formats from SQLite, and always writes as TEXT.
//!
//! ## Encoding Strategy
//!
//! This bridges the hybrid state where legacy tables store UUIDs as BLOBs
//! and newer tables store them as TEXT strings.
//!
//! - **Decode (read)**: Inspects SQLite column type at runtime. TEXT columns are
//!   read as-is; BLOB columns (16 bytes) are converted to hyphenated UUID strings.
//! - **Encode (write)**: Always writes as TEXT. New data is always TEXT.
//!
//! ## When to use which bind helper
//!
//! | Column format | Bind helper | Example |
//! |---------------|-------------|---------|
//! | TEXT (new tables) | `bind_uuid()` or `.bind(&db_uuid)` | `notifications.user_id` |
//! | BLOB (legacy) | `bind_uuid_blob()` → `Result<Vec<u8>>` | `project_members.user_id` |
//!
//! ## Known BLOB columns (as of migration 20260328)
//!
//! - `users.id`
//! - `project_members.user_id`, `project_members.granted_by`
//! - `organization_members.user_id`
//! - `client_members.user_id`
//!
//! All other UUID columns have been migrated to TEXT.

use std::{fmt, ops::Deref};

use serde::{Deserialize, Serialize};
use sqlx::{
    Decode, Encode, Sqlite, Type, TypeInfo, ValueRef,
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
};

/// A UUID that transparently decodes both BLOB and TEXT from SQLite.
///
/// Always encodes as TEXT (hyphenated lowercase: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
/// Wraps a `String` internally for zero-cost interop with String-based APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DbUuid(String);

// ---------------------------------------------------------------------------
// Construction helpers
// ---------------------------------------------------------------------------

impl DbUuid {
    /// Generate a new random v4 UUID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().hyphenated().to_string())
    }

    /// Return the nil UUID (all zeros).
    pub fn nil() -> Self {
        Self("00000000-0000-0000-0000-000000000000".to_string())
    }

    /// Wrap an existing UUID string without validation.
    /// Prefer [`DbUuid::parse`] when the input is untrusted.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Parse and validate a UUID string.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        let parsed = uuid::Uuid::parse_str(s)?;
        Ok(Self(parsed.hyphenated().to_string()))
    }

    /// Convert to a `uuid::Uuid`.
    ///
    /// This is infallible when the `DbUuid` was created via [`DbUuid::parse`] or
    /// [`DbUuid::new`]. Panics only if the inner string is not a valid UUID
    /// (should never happen for properly-constructed instances).
    pub fn to_uuid(&self) -> uuid::Uuid {
        uuid::Uuid::parse_str(&self.0).expect("DbUuid contains invalid UUID string")
    }

    /// Borrow the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner `String`.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl Default for DbUuid {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Conversion traits
// ---------------------------------------------------------------------------

impl Deref for DbUuid {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for DbUuid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DbUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for DbUuid {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<DbUuid> for String {
    fn from(u: DbUuid) -> Self {
        u.0
    }
}

impl From<uuid::Uuid> for DbUuid {
    fn from(u: uuid::Uuid) -> Self {
        Self(u.hyphenated().to_string())
    }
}

impl From<DbUuid> for uuid::Uuid {
    fn from(u: DbUuid) -> Self {
        u.to_uuid()
    }
}

impl PartialEq<str> for DbUuid {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<String> for DbUuid {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl PartialEq<DbUuid> for String {
    fn eq(&self, other: &DbUuid) -> bool {
        *self == other.0
    }
}

// ---------------------------------------------------------------------------
// SQLx integration — the core hybrid decoder
// ---------------------------------------------------------------------------

impl Type<Sqlite> for DbUuid {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        // Accept both TEXT and BLOB columns
        <String as Type<Sqlite>>::compatible(ty) || <Vec<u8> as Type<Sqlite>>::compatible(ty)
    }
}

impl Decode<'_, Sqlite> for DbUuid {
    fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
        // Inspect the SQLite type to decide how to decode.
        // TEXT path: direct string (most common for new data).
        // BLOB path: 16-byte raw UUID from legacy tables.
        // Hybrid: BLOB-declared columns may contain TEXT strings after normalization
        //         migration (SQLite can't ALTER COLUMN type). Detect by length.
        let type_info = value.type_info();
        let type_name = type_info.name();
        if type_name == "TEXT" {
            let text = <String as Decode<Sqlite>>::decode(value)?;
            Ok(Self(text))
        } else {
            let bytes = <Vec<u8> as Decode<Sqlite>>::decode(value)?;
            if bytes.len() == 16 {
                // True 16-byte BLOB UUID
                let parsed = uuid::Uuid::from_slice(&bytes)?;
                Ok(Self(parsed.hyphenated().to_string()))
            } else {
                // BLOB column containing TEXT string (post-normalization)
                let text = String::from_utf8(bytes)?;
                Ok(Self(text))
            }
        }
    }
}

impl Encode<'_, Sqlite> for DbUuid {
    fn encode_by_ref(
        &self,
        args: &mut Vec<SqliteArgumentValue<'_>>,
    ) -> Result<IsNull, BoxDynError> {
        // Always encode as TEXT
        Encode::<Sqlite>::encode_by_ref(&self.0, args)
    }
}

// ---------------------------------------------------------------------------
// ts-rs integration — exports as `string` in TypeScript
// ---------------------------------------------------------------------------

impl ts_rs::TS for DbUuid {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name() -> String {
        "string".to_owned()
    }

    fn inline() -> String {
        "string".to_owned()
    }

    fn inline_flattened() -> String {
        panic!("DbUuid cannot be flattened")
    }

    fn decl() -> String {
        panic!("DbUuid cannot be declared")
    }

    fn decl_concrete() -> String {
        panic!("DbUuid cannot be declared")
    }
}

// ---------------------------------------------------------------------------
// Convenience helpers for query binding migration
// ---------------------------------------------------------------------------

/// Bind a `DbUuid` reference in a raw `sqlx::query()` call (TEXT columns).
///
/// Replaces the old `.as_bytes().as_slice()` pattern:
/// ```ignore
/// // Before:
/// .bind(user_id.as_bytes().as_slice())
/// // After:
/// .bind(bind_uuid(&user_id))
/// ```
pub fn bind_uuid(uuid: &DbUuid) -> &str {
    uuid.as_str()
}

/// Convert a `DbUuid` to 16-byte BLOB for binding to legacy BLOB UUID columns.
///
/// Use this at the bind site when inserting into a column that stores UUIDs as
/// BLOB (e.g. `users.id`, `project_members.user_id`). Push the string→blob
/// boundary as close to the DB consumer as possible; all other code should use
/// DbUuid/str.
///
/// ```ignore
/// let user_id = DbUuid::from(access_context.user_id);
/// sqlx::query("INSERT INTO project_members (user_id) VALUES (?)")
///     .bind(bind_uuid_blob(&user_id)?)
/// ```
pub fn bind_uuid_blob(uuid: &DbUuid) -> Result<Vec<u8>, uuid::Error> {
    Ok(uuid::Uuid::parse_str(uuid.as_str())?.as_bytes().to_vec())
}

/// Convert an `Option<DbUuid>` to optional 16-byte BLOB for legacy BLOB columns.
pub fn bind_optional_uuid_blob(uuid: &Option<DbUuid>) -> Result<Option<Vec<u8>>, uuid::Error> {
    uuid.as_ref().map(bind_uuid_blob).transpose()
}

/// Parse a UUID string and return 16-byte BLOB for binding to legacy BLOB columns.
///
/// Use this when a function receives a `&str` UUID and needs to query a BLOB column.
/// ```ignore
/// sqlx::query("SELECT * FROM pulse_sources WHERE organization_id = ?")
///     .bind(str_to_uuid_blob(org_id)?)
/// ```
pub fn str_to_uuid_blob(s: &str) -> Result<Vec<u8>, uuid::Error> {
    Ok(uuid::Uuid::parse_str(s)?.as_bytes().to_vec())
}

/// Parse an optional UUID string and return optional 16-byte BLOB for legacy BLOB columns.
pub fn str_to_optional_uuid_blob(s: Option<&str>) -> Result<Option<Vec<u8>>, uuid::Error> {
    s.map(str_to_uuid_blob).transpose()
}

/// Bind an `Option<DbUuid>` reference in a raw `sqlx::query()` call.
///
/// Replaces the old `.as_ref().map(|u| u.as_bytes().to_vec())` pattern:
/// ```ignore
/// // Before:
/// .bind(project_id.as_ref().map(|u| u.as_bytes().to_vec()))
/// // After:
/// .bind(bind_optional_uuid(&project_id))
/// ```
pub fn bind_optional_uuid(uuid: &Option<DbUuid>) -> Option<&str> {
    uuid.as_ref().map(|u| u.as_str())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_valid_v4() {
        let id = DbUuid::new();
        let parsed = uuid::Uuid::parse_str(id.as_str()).unwrap();
        assert_eq!(parsed.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn parse_valid_uuid() {
        let id = DbUuid::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(id.as_str(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn parse_invalid_uuid() {
        assert!(DbUuid::parse("not-a-uuid").is_err());
    }

    #[test]
    fn from_uuid_crate() {
        let raw = uuid::Uuid::new_v4();
        let db = DbUuid::from(raw);
        assert_eq!(db.as_str(), raw.hyphenated().to_string());
    }

    #[test]
    fn from_string() {
        let s = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let id = DbUuid::from(s.clone());
        assert_eq!(id.as_str(), s);
    }

    #[test]
    fn into_string() {
        let id = DbUuid::new();
        let s: String = id.clone().into();
        assert_eq!(s, id.as_str());
    }

    #[test]
    fn deref_as_str() {
        let id = DbUuid::new();
        let s: &str = &id;
        assert_eq!(s, id.as_str());
    }

    #[test]
    fn display() {
        let id = DbUuid::from_string("abc-123");
        assert_eq!(format!("{id}"), "abc-123");
    }

    #[test]
    fn partial_eq_str() {
        let id = DbUuid::from_string("test-uuid");
        assert_eq!(id, *"test-uuid");
    }

    #[test]
    fn partial_eq_string() {
        let id = DbUuid::from_string("test-uuid");
        assert_eq!(id, "test-uuid".to_string());
    }

    #[test]
    fn serde_roundtrip() {
        let id = DbUuid::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: DbUuid = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
        // Should serialize as a plain string, not an object
        assert!(json.starts_with('"'));
    }

    #[test]
    fn bind_helpers() {
        let id = DbUuid::from_string("my-uuid");
        assert_eq!(bind_uuid(&id), "my-uuid");

        let some_id = Some(DbUuid::from_string("other-uuid"));
        assert_eq!(bind_optional_uuid(&some_id), Some("other-uuid"));

        let none_id: Option<DbUuid> = None;
        assert_eq!(bind_optional_uuid(&none_id), None);
    }

    #[test]
    fn bind_blob_helpers() {
        let id = DbUuid::from_string("550e8400-e29b-41d4-a716-446655440000");
        let blob = bind_uuid_blob(&id).unwrap();
        assert_eq!(blob.len(), 16);
        // Round-trip: blob → Uuid → string should match original
        let round = uuid::Uuid::from_slice(&blob).unwrap();
        assert_eq!(round.hyphenated().to_string(), id.as_str());

        let some_id = Some(id.clone());
        assert!(bind_optional_uuid_blob(&some_id).unwrap().is_some());
        let none_id: Option<DbUuid> = None;
        assert!(bind_optional_uuid_blob(&none_id).unwrap().is_none());
    }
}
