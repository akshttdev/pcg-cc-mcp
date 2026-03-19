//! Data Sources — uploadable knowledge with flexible metadata.
//!
//! Each data source has a `data_type` (conversation, document, transcript, etc.)
//! and an optional `file_type` (mime). A JSON `metadata` blob stores type-specific
//! fields whose schema varies by data_type + file_type combination.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

// ── Row type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataSource {
    pub id: String,
    pub organization_id: Option<String>,
    pub project_id: Option<String>,
    pub created_by: Option<String>,

    pub title: String,
    pub description: Option<String>,
    pub data_type: String,
    /// How data enters the system: "file", "text", or "integration"
    pub source_type: String,
    pub file_type: Option<String>,

    /// Raw text content (for source_type = "text")
    pub content: Option<String>,

    pub file_name: Option<String>,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub file_hash: Option<String>,

    /// JSON blob — schema varies by data_type + file_type
    pub metadata: String,

    pub status: String,
    pub processing_error: Option<String>,

    /// Slash-delimited folder path, e.g. "Meetings/Google Meet"
    pub folder: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

// ── Create input ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateDataSource {
    pub organization_id: Option<String>,
    pub project_id: Option<String>,
    pub created_by: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub data_type: String,
    /// "file", "text", or "integration"
    pub source_type: Option<String>,
    pub file_type: Option<String>,
    /// Raw text content (for source_type = "text")
    pub content: Option<String>,
    pub file_name: Option<String>,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub file_hash: Option<String>,
    pub metadata: Option<String>,
    pub folder: Option<String>,
}

// ── Update input ────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateDataSource {
    pub title: Option<String>,
    pub description: Option<String>,
    pub data_type: Option<String>,
    pub source_type: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<String>,
    pub status: Option<String>,
    pub processing_error: Option<String>,
    pub folder: Option<String>,
}

// ── Metadata templates by data_type ─────────────────────────────────────────

/// Returns a JSON-schema-like template describing the expected metadata fields
/// for a given `data_type` + `source_type` combination.
/// Frontends can use this to render dynamic forms.
pub fn metadata_template(data_type: &str, source_type: &str) -> serde_json::Value {
    let mut fields = serde_json::Map::new();

    // ── Source-type-specific fields ─────────────────────────────────────
    match source_type {
        "file" => {
            fields.insert(
                "file_name".into(),
                serde_json::json!({ "type": "string", "description": "Original filename" }),
            );
            fields.insert("file_mime".into(), serde_json::json!({ "type": "string", "description": "MIME type (e.g. application/pdf)" }));
            fields.insert(
                "file_size_bytes".into(),
                serde_json::json!({ "type": "number", "description": "File size in bytes" }),
            );
            fields.insert(
                "file_hash".into(),
                serde_json::json!({ "type": "string", "description": "SHA-256 hash for dedup" }),
            );
        }
        "integration" => {
            fields.insert("integration_name".into(), serde_json::json!({ "type": "string", "description": "Name of the integration (slack, hubspot, etc.)" }));
            fields.insert(
                "external_id".into(),
                serde_json::json!({ "type": "string", "description": "ID in the external system" }),
            );
            fields.insert("external_url".into(), serde_json::json!({ "type": "string", "description": "URL to the source in the external system" }));
            fields.insert("synced_at".into(), serde_json::json!({ "type": "string", "format": "date-time", "description": "Last sync timestamp" }));
        }
        // "text" — no extra source fields needed
        _ => {}
    }

    // ── Data-type-specific fields ──────────────────────────────────────
    match data_type {
        "conversation" => {
            fields.insert("participants".into(), serde_json::json!({ "type": "array", "items": { "type": "string" }, "description": "People in the conversation" }));
            fields.insert("channel".into(), serde_json::json!({ "type": "string", "description": "Channel or medium (email, slack, phone, in-person, zoom)" }));
            fields.insert("date".into(), serde_json::json!({ "type": "string", "format": "date", "description": "Date of conversation" }));
            fields.insert(
                "duration_minutes".into(),
                serde_json::json!({ "type": "number", "description": "Duration in minutes" }),
            );
            fields.insert("topics".into(), serde_json::json!({ "type": "array", "items": { "type": "string" }, "description": "Key topics discussed" }));
            fields.insert("sentiment".into(), serde_json::json!({ "type": "string", "enum": ["positive", "neutral", "negative"], "description": "Overall sentiment" }));
            fields.insert("action_items".into(), serde_json::json!({ "type": "array", "items": { "type": "string" }, "description": "Action items from conversation" }));
        }
        "document" => {
            fields.insert(
                "author".into(),
                serde_json::json!({ "type": "string", "description": "Document author" }),
            );
            fields.insert(
                "version".into(),
                serde_json::json!({ "type": "string", "description": "Document version" }),
            );
            fields.insert(
                "page_count".into(),
                serde_json::json!({ "type": "number", "description": "Number of pages" }),
            );
            fields.insert("language".into(), serde_json::json!({ "type": "string", "description": "Language code (en, es, etc.)" }));
            fields.insert("tags".into(), serde_json::json!({ "type": "array", "items": { "type": "string" }, "description": "Document tags" }));
        }
        "transcript" => {
            fields.insert("speakers".into(), serde_json::json!({ "type": "array", "items": { "type": "string" }, "description": "Speakers in the transcript" }));
            fields.insert("source".into(), serde_json::json!({ "type": "string", "description": "Source (meeting, interview, podcast, webinar)" }));
            fields.insert("date".into(), serde_json::json!({ "type": "string", "format": "date", "description": "Date of recording" }));
            fields.insert(
                "duration_minutes".into(),
                serde_json::json!({ "type": "number", "description": "Duration in minutes" }),
            );
            fields.insert(
                "language".into(),
                serde_json::json!({ "type": "string", "description": "Language code" }),
            );
        }
        "report" => {
            fields.insert("report_type".into(), serde_json::json!({ "type": "string", "description": "Type of report (financial, progress, research, analysis)" }));
            fields.insert(
                "period".into(),
                serde_json::json!({ "type": "string", "description": "Reporting period" }),
            );
            fields.insert(
                "author".into(),
                serde_json::json!({ "type": "string", "description": "Report author" }),
            );
            fields.insert("confidentiality".into(), serde_json::json!({ "type": "string", "enum": ["public", "internal", "confidential"], "description": "Confidentiality level" }));
        }
        "dataset" => {
            fields.insert(
                "row_count".into(),
                serde_json::json!({ "type": "number", "description": "Number of rows/records" }),
            );
            fields.insert(
                "column_count".into(),
                serde_json::json!({ "type": "number", "description": "Number of columns/fields" }),
            );
            fields.insert("columns".into(), serde_json::json!({ "type": "array", "items": { "type": "string" }, "description": "Column/field names" }));
            fields.insert(
                "source_system".into(),
                serde_json::json!({ "type": "string", "description": "Originating system" }),
            );
            fields.insert(
                "date_range".into(),
                serde_json::json!({ "type": "string", "description": "Date range covered" }),
            );
        }
        "media" => {
            fields.insert("media_type".into(), serde_json::json!({ "type": "string", "enum": ["audio", "video", "image"], "description": "Media type" }));
            fields.insert("duration_seconds".into(), serde_json::json!({ "type": "number", "description": "Duration in seconds (audio/video)" }));
            fields.insert(
                "resolution".into(),
                serde_json::json!({ "type": "string", "description": "Resolution (video/image)" }),
            );
            fields.insert(
                "codec".into(),
                serde_json::json!({ "type": "string", "description": "Codec or format details" }),
            );
        }
        _ => {}
    }

    serde_json::Value::Object(fields)
}

// ── Queries ─────────────────────────────────────────────────────────────────

impl DataSource {
    /// Create a new data source
    pub async fn create(pool: &SqlitePool, input: CreateDataSource) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let metadata = input.metadata.unwrap_or_else(|| "{}".to_string());
        let source_type = input.source_type.unwrap_or_else(|| "file".to_string());
        let folder = input.folder.unwrap_or_else(|| "Unfiled".to_string());

        sqlx::query(
            r#"INSERT INTO data_sources
                (id, organization_id, project_id, created_by,
                 title, description, data_type, source_type, file_type,
                 content, file_name, file_path, file_size_bytes, file_hash,
                 metadata, status, folder)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?)"#,
        )
        .bind(&id)
        .bind(&input.organization_id)
        .bind(&input.project_id)
        .bind(&input.created_by)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.data_type)
        .bind(&source_type)
        .bind(&input.file_type)
        .bind(&input.content)
        .bind(&input.file_name)
        .bind(&input.file_path)
        .bind(input.file_size_bytes)
        .bind(&input.file_hash)
        .bind(&metadata)
        .bind(&folder)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Find by ID
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM data_sources WHERE id = ? AND archived_at IS NULL")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List by organization
    pub async fn find_by_organization(
        pool: &SqlitePool,
        org_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM data_sources
            WHERE organization_id = ? AND archived_at IS NULL
            ORDER BY created_at DESC"#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// List by project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM data_sources
            WHERE project_id = ? AND archived_at IS NULL
            ORDER BY created_at DESC"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    /// List by organization (including project-scoped ones for org's projects)
    pub async fn find_by_organization_all(
        pool: &SqlitePool,
        org_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT ds.* FROM data_sources ds
            WHERE ds.archived_at IS NULL
              AND (ds.organization_id = ?
                   OR ds.project_id IN (
                       SELECT id FROM projects WHERE organization_id = ?
                   ))
            ORDER BY ds.created_at DESC"#,
        )
        .bind(org_id)
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Update a data source
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        input: UpdateDataSource,
    ) -> Result<Option<Self>, sqlx::Error> {
        // Build dynamic SET clause
        let mut sets = Vec::new();
        let mut binds: Vec<Option<String>> = Vec::new();

        if let Some(ref title) = input.title {
            sets.push("title = ?");
            binds.push(Some(title.clone()));
        }
        if let Some(ref desc) = input.description {
            sets.push("description = ?");
            binds.push(Some(desc.clone()));
        }
        if let Some(ref dt) = input.data_type {
            sets.push("data_type = ?");
            binds.push(Some(dt.clone()));
        }
        if let Some(ref st) = input.source_type {
            sets.push("source_type = ?");
            binds.push(Some(st.clone()));
        }
        if let Some(ref c) = input.content {
            sets.push("content = ?");
            binds.push(Some(c.clone()));
        }
        if let Some(ref meta) = input.metadata {
            sets.push("metadata = ?");
            binds.push(Some(meta.clone()));
        }
        if let Some(ref status) = input.status {
            sets.push("status = ?");
            binds.push(Some(status.clone()));
        }
        if let Some(ref err) = input.processing_error {
            sets.push("processing_error = ?");
            binds.push(Some(err.clone()));
        }
        if let Some(ref f) = input.folder {
            sets.push("folder = ?");
            binds.push(Some(f.clone()));
        }

        if sets.is_empty() {
            return Self::find_by_id(pool, id).await;
        }

        sets.push("updated_at = datetime('now', 'subsec')");
        let query_str = format!("UPDATE data_sources SET {} WHERE id = ?", sets.join(", "));

        let mut query = sqlx::query(&query_str);
        for val in &binds {
            query = query.bind(val);
        }
        query = query.bind(id);
        query.execute(pool).await?;

        Self::find_by_id(pool, id).await
    }

    /// Soft-delete
    pub async fn archive(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE data_sources SET archived_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update file info after upload
    pub async fn set_file_info(
        pool: &SqlitePool,
        id: &str,
        file_name: &str,
        file_path: &str,
        file_size_bytes: i64,
        file_hash: &str,
        file_type: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE data_sources SET
                file_name = ?, file_path = ?, file_size_bytes = ?,
                file_hash = ?, file_type = COALESCE(?, file_type),
                status = 'ready',
                updated_at = datetime('now', 'subsec')
            WHERE id = ?"#,
        )
        .bind(file_name)
        .bind(file_path)
        .bind(file_size_bytes)
        .bind(file_hash)
        .bind(file_type)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
