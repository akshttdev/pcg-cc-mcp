//! Deck documents — structured slide authoring for the Lux Creator Studio.
//!
//! ## Storage pattern
//!
//! The DB layer stores `canvas_json`, `background_json`, and `elements_json`
//! as opaque TEXT. The strongly-typed domain model (`Canvas`, `Slide`,
//! `SlideElement`, etc.) lives in this module and is exported via ts-rs so
//! the frontend can consume it directly.
//!
//! We deliberately keep the element tree as opaque JSON in the DB for Phase 0
//! rather than normalizing into an `elements` table. Rationale: the
//! mutations Lux and the studio perform are almost always whole-slide or
//! whole-tree writes, not element-level queries. If we find ourselves
//! wanting to query "all text elements referencing token X across a deal's
//! decks", that's the signal to normalize. Until then, opaque JSON wins on
//! simplicity, round-trip fidelity (rich text runs, nested groups), and
//! migration-free evolution of the element schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

use crate::db_uuid::DbUuid;

// ─── DB row structs ─────────────────────────────────────────────────────────

const DOC_COLUMNS: &str = "id, deal_id, version, brand_token_version, canvas_json, last_edited_by, created_at, updated_at";

/// Top-level deck document row. Slides live in a separate table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeckDocument {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub version: i64,
    pub brand_token_version: Option<String>,
    /// Opaque serialized `Canvas`. Use [`DeckDocument::canvas`] to parse.
    pub canvas_json: String,
    /// Who last touched the deck: `lux` | `operator` | `system`.
    pub last_edited_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const SLIDE_COLUMNS: &str = "id, deck_id, slide_index, name, layout_hint, background_json, \
                             elements_json, notes, origin, locked, created_at, updated_at";

/// Slide row. `elements_json` is a serialized `Vec<SlideElement>`.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeckSlide {
    pub id: Uuid,
    pub deck_id: Uuid,
    pub slide_index: i64,
    pub name: Option<String>,
    pub layout_hint: Option<String>,
    /// Opaque serialized `Fill`.
    pub background_json: String,
    /// Opaque serialized `Vec<SlideElement>`.
    pub elements_json: String,
    pub notes: Option<String>,
    /// `lux` | `operator`.
    pub origin: String,
    pub locked: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const REVISION_COLUMNS: &str =
    "id, deck_id, version, author, snapshot_json, diff_summary, created_at";

/// Immutable deck snapshot at a point in time.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeckRevision {
    pub id: Uuid,
    pub deck_id: Uuid,
    pub version: i64,
    /// `lux` | `operator` | user UUID string.
    pub author: String,
    /// Full serialized `DeckSnapshot` for restore/diff.
    pub snapshot_json: String,
    pub diff_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

const SUGGESTION_COLUMNS: &str = "id, deck_id, target_slide_id, target_element_id, op, \
                                  payload_json, rationale, status, run_id, created_at, \
                                  resolved_at, resolved_by";

/// Lux-authored proposal awaiting operator accept/reject.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeckSuggestion {
    pub id: Uuid,
    pub deck_id: Uuid,
    pub target_slide_id: Option<Uuid>,
    pub target_element_id: Option<Uuid>,
    pub op: String,
    pub payload_json: String,
    pub rationale: Option<String>,
    pub status: String,
    pub run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

// ─── Typed domain model ─────────────────────────────────────────────────────

/// Canvas metadata. Stored inside `deck_documents.canvas_json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Canvas {
    pub width: f64,
    pub height: f64,
    /// `px` | `pt`.
    pub unit: String,
    pub dpi: f64,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            width: 1920.0,
            height: 1080.0,
            unit: "px".to_string(),
            dpi: 72.0,
        }
    }
}

/// RGBA color, optionally linked back to a design token name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
    #[ts(optional)]
    pub token: Option<String>,
}

/// Fill variants for backgrounds and shape fills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Fill {
    Solid {
        color: Color,
    },
    LinearGradient {
        stops: Vec<GradientStop>,
        angle: f64,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    /// `solid` | `dashed` | `dotted`.
    pub style: String,
}

/// Axis-aligned bounding box with optional rotation (degrees, clockwise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[ts(optional)]
    pub rotation: Option<f64>,
}

/// Element discriminated by `type`. Mirrors the frontend TS union.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlideElement {
    Text(TextElement),
    Image(ImageElement),
    Shape(ShapeElement),
    Group(GroupElement),
}

impl SlideElement {
    /// Return the element's UUID regardless of variant.
    pub fn id(&self) -> Uuid {
        match self {
            Self::Text(e) => e.id,
            Self::Image(e) => e.id,
            Self::Shape(e) => e.id,
            Self::Group(e) => e.id,
        }
    }
}

/// Fields common to every element variant. Kept flat on each variant so the
/// serde tag discriminator remains clean.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextElement {
    pub id: Uuid,
    pub bbox: BBox,
    pub z: i64,
    /// `lux` | `operator`.
    pub origin: String,
    #[ts(optional)]
    pub locked_by: Option<String>,
    #[ts(optional)]
    pub opacity: Option<f64>,
    #[ts(optional)]
    pub token_refs: Option<std::collections::BTreeMap<String, String>>,

    pub text: String,
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: i64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub color: Color,
    /// `left` | `center` | `right` | `justify`.
    pub align: String,
    #[ts(optional)]
    pub runs: Option<Vec<TextRun>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextRun {
    pub start: i64,
    pub end: i64,
    /// Partial overrides — stored as a free-form JSON map.
    pub overrides: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImageElement {
    pub id: Uuid,
    pub bbox: BBox,
    pub z: i64,
    pub origin: String,
    #[ts(optional)]
    pub locked_by: Option<String>,
    #[ts(optional)]
    pub opacity: Option<f64>,
    #[ts(optional)]
    pub token_refs: Option<std::collections::BTreeMap<String, String>>,

    /// FK to `media_assets.id`. Nullable while the asset is pending a Maci
    /// generation (see Phase 7).
    #[ts(optional)]
    pub asset_id: Option<Uuid>,
    /// `pending` | `resolved` | `failed`.
    pub asset_status: String,
    #[ts(optional)]
    pub asset_request_id: Option<Uuid>,
    /// `cover` | `contain` | `fill`.
    pub fit: String,
    #[ts(optional)]
    pub crop: Option<BBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShapeElement {
    pub id: Uuid,
    pub bbox: BBox,
    pub z: i64,
    pub origin: String,
    #[ts(optional)]
    pub locked_by: Option<String>,
    #[ts(optional)]
    pub opacity: Option<f64>,
    #[ts(optional)]
    pub token_refs: Option<std::collections::BTreeMap<String, String>>,

    /// `rect` | `ellipse` | `line` | `path`.
    pub shape: String,
    #[ts(optional)]
    pub path: Option<String>,
    pub fill: Fill,
    #[ts(optional)]
    pub stroke: Option<Stroke>,
    #[ts(optional)]
    pub corner_radius: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GroupElement {
    pub id: Uuid,
    pub bbox: BBox,
    pub z: i64,
    pub origin: String,
    #[ts(optional)]
    pub locked_by: Option<String>,
    #[ts(optional)]
    pub opacity: Option<f64>,
    #[ts(optional)]
    pub token_refs: Option<std::collections::BTreeMap<String, String>>,

    pub children: Vec<SlideElement>,
}

/// Full deck assembled from `DeckDocument` + its ordered `DeckSlide` rows,
/// with elements and canvas parsed into their typed forms. This is the shape
/// the frontend and renderer consume.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeckSnapshot {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub version: i64,
    #[ts(optional)]
    pub brand_token_version: Option<String>,
    pub canvas: Canvas,
    pub slides: Vec<SlideSnapshot>,
    pub last_edited_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SlideSnapshot {
    pub id: Uuid,
    pub slide_index: i64,
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub layout_hint: Option<String>,
    pub background: Fill,
    pub elements: Vec<SlideElement>,
    #[ts(optional)]
    pub notes: Option<String>,
    pub origin: String,
    pub locked: bool,
}

// ─── Create / Update DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateDeckDocument {
    pub deal_id: Uuid,
    #[ts(optional)]
    pub canvas: Option<Canvas>,
    #[ts(optional)]
    pub brand_token_version: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateDeckDocument {
    #[ts(optional)]
    pub canvas: Option<Canvas>,
    #[ts(optional)]
    pub brand_token_version: Option<String>,
    #[ts(optional)]
    pub last_edited_by: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateDeckSlide {
    pub deck_id: Uuid,
    pub slide_index: i64,
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub layout_hint: Option<String>,
    pub background: Fill,
    pub elements: Vec<SlideElement>,
    #[ts(optional)]
    pub notes: Option<String>,
    #[ts(optional)]
    pub origin: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateDeckSlide {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub layout_hint: Option<String>,
    #[ts(optional)]
    pub background: Option<Fill>,
    #[ts(optional)]
    pub elements: Option<Vec<SlideElement>>,
    #[ts(optional)]
    pub notes: Option<String>,
    #[ts(optional)]
    pub locked: Option<bool>,
}

// ─── DeckDocument queries ───────────────────────────────────────────────────

impl DeckDocument {
    /// Parse the `canvas_json` column into its typed form.
    pub fn canvas(&self) -> Result<Canvas, serde_json::Error> {
        serde_json::from_str(&self.canvas_json)
    }

    pub async fn create(pool: &SqlitePool, input: CreateDeckDocument) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        let canvas = input.canvas.unwrap_or_default();
        let canvas_json = serde_json::to_string(&canvas).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;

        sqlx::query(
            r#"INSERT INTO deck_documents
                 (id, deal_id, canvas_json, brand_token_version)
                 VALUES (?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(input.deal_id.to_string())
        .bind(&canvas_json)
        .bind(&input.brand_token_version)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id.to_uuid())
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {DOC_COLUMNS} FROM deck_documents WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
    }

    /// Return the most recent deck for a given deal, if any.
    pub async fn find_latest_for_deal(
        pool: &SqlitePool,
        deal_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {DOC_COLUMNS} FROM deck_documents \
             WHERE deal_id = ? ORDER BY updated_at DESC LIMIT 1"
        ))
        .bind(deal_id.to_string())
        .fetch_optional(pool)
        .await
    }

    pub async fn list_for_deal(pool: &SqlitePool, deal_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {DOC_COLUMNS} FROM deck_documents \
             WHERE deal_id = ? ORDER BY created_at DESC"
        ))
        .bind(deal_id.to_string())
        .fetch_all(pool)
        .await
    }

    /// Apply partial updates and bump the deck version in one statement.
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateDeckDocument,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::new(
            "UPDATE deck_documents SET updated_at = datetime('now','subsec'), \
             version = version + 1",
        );
        if let Some(canvas) = input.canvas {
            let canvas_json = serde_json::to_string(&canvas).map_err(|e| {
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )))
            })?;
            qb.push(", canvas_json = ").push_bind(canvas_json);
        }
        if let Some(v) = input.brand_token_version {
            qb.push(", brand_token_version = ").push_bind(v);
        }
        if let Some(v) = input.last_edited_by {
            qb.push(", last_edited_by = ").push_bind(v);
        }
        qb.push(" WHERE id = ").push_bind(id.to_string());
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM deck_documents WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Assemble the full `DeckSnapshot` for a deck — loads all slides, parses
    /// canvas + elements, and returns the shape the frontend consumes.
    pub async fn snapshot(&self, pool: &SqlitePool) -> Result<DeckSnapshot, sqlx::Error> {
        let slides = DeckSlide::list_for_deck(pool, self.id).await?;
        let canvas = self.canvas().map_err(deck_json_err)?;
        let slide_snapshots = slides
            .into_iter()
            .map(|s| s.into_snapshot())
            .collect::<Result<Vec<_>, _>>()
            .map_err(deck_json_err)?;

        Ok(DeckSnapshot {
            id: self.id,
            deal_id: self.deal_id,
            version: self.version,
            brand_token_version: self.brand_token_version.clone(),
            canvas,
            slides: slide_snapshots,
            last_edited_by: self.last_edited_by.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// ─── DeckSlide queries ──────────────────────────────────────────────────────

impl DeckSlide {
    pub fn background(&self) -> Result<Fill, serde_json::Error> {
        serde_json::from_str(&self.background_json)
    }

    pub fn elements(&self) -> Result<Vec<SlideElement>, serde_json::Error> {
        serde_json::from_str(&self.elements_json)
    }

    pub fn into_snapshot(self) -> Result<SlideSnapshot, serde_json::Error> {
        let background = self.background()?;
        let elements = self.elements()?;
        Ok(SlideSnapshot {
            id: self.id,
            slide_index: self.slide_index,
            name: self.name,
            layout_hint: self.layout_hint,
            background,
            elements,
            notes: self.notes,
            origin: self.origin,
            locked: self.locked != 0,
        })
    }

    pub async fn create(pool: &SqlitePool, input: CreateDeckSlide) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        let background_json = serde_json::to_string(&input.background).map_err(deck_json_err)?;
        let elements_json = serde_json::to_string(&input.elements).map_err(deck_json_err)?;
        let origin = input.origin.unwrap_or_else(|| "lux".to_string());

        sqlx::query(
            r#"INSERT INTO deck_slides
                 (id, deck_id, slide_index, name, layout_hint,
                  background_json, elements_json, notes, origin)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(input.deck_id.to_string())
        .bind(input.slide_index)
        .bind(&input.name)
        .bind(&input.layout_hint)
        .bind(&background_json)
        .bind(&elements_json)
        .bind(&input.notes)
        .bind(&origin)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id.to_uuid())
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {SLIDE_COLUMNS} FROM deck_slides WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
    }

    pub async fn list_for_deck(pool: &SqlitePool, deck_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {SLIDE_COLUMNS} FROM deck_slides \
             WHERE deck_id = ? ORDER BY slide_index ASC"
        ))
        .bind(deck_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateDeckSlide,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut qb =
            sqlx::QueryBuilder::new("UPDATE deck_slides SET updated_at = datetime('now','subsec')");
        if let Some(v) = input.name {
            qb.push(", name = ").push_bind(v);
        }
        if let Some(v) = input.layout_hint {
            qb.push(", layout_hint = ").push_bind(v);
        }
        if let Some(v) = input.background {
            let json = serde_json::to_string(&v).map_err(deck_json_err)?;
            qb.push(", background_json = ").push_bind(json);
        }
        if let Some(v) = input.elements {
            let json = serde_json::to_string(&v).map_err(deck_json_err)?;
            qb.push(", elements_json = ").push_bind(json);
        }
        if let Some(v) = input.notes {
            qb.push(", notes = ").push_bind(v);
        }
        if let Some(v) = input.locked {
            qb.push(", locked = ").push_bind(if v { 1_i64 } else { 0 });
        }
        qb.push(" WHERE id = ").push_bind(id.to_string());
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM deck_slides WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}

// ─── DeckRevision queries ───────────────────────────────────────────────────

impl DeckRevision {
    /// Insert a new revision row with a full serialized snapshot.
    pub async fn create(
        pool: &SqlitePool,
        deck_id: Uuid,
        version: i64,
        author: &str,
        snapshot: &DeckSnapshot,
        diff_summary: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        let snapshot_json = serde_json::to_string(snapshot).map_err(deck_json_err)?;

        sqlx::query(
            r#"INSERT INTO deck_revisions
                 (id, deck_id, version, author, snapshot_json, diff_summary)
                 VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(deck_id.to_string())
        .bind(version)
        .bind(author)
        .bind(&snapshot_json)
        .bind(diff_summary)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id.to_uuid())
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {REVISION_COLUMNS} FROM deck_revisions WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
    }

    pub async fn list_for_deck(
        pool: &SqlitePool,
        deck_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {REVISION_COLUMNS} FROM deck_revisions \
             WHERE deck_id = ? ORDER BY version DESC LIMIT ?"
        ))
        .bind(deck_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

// ─── DeckSuggestion queries ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateDeckSuggestion {
    pub deck_id: Uuid,
    #[ts(optional)]
    pub target_slide_id: Option<Uuid>,
    #[ts(optional)]
    pub target_element_id: Option<Uuid>,
    pub op: String,
    pub payload: serde_json::Value,
    #[ts(optional)]
    pub rationale: Option<String>,
    #[ts(optional)]
    pub run_id: Option<String>,
}

impl DeckSuggestion {
    pub async fn create(
        pool: &SqlitePool,
        input: CreateDeckSuggestion,
    ) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        let payload_json = serde_json::to_string(&input.payload).map_err(deck_json_err)?;

        sqlx::query(
            r#"INSERT INTO deck_suggestions
                 (id, deck_id, target_slide_id, target_element_id, op,
                  payload_json, rationale, run_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(input.deck_id.to_string())
        .bind(input.target_slide_id.map(|u| u.to_string()))
        .bind(input.target_element_id.map(|u| u.to_string()))
        .bind(&input.op)
        .bind(&payload_json)
        .bind(&input.rationale)
        .bind(&input.run_id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id.to_uuid())
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {SUGGESTION_COLUMNS} FROM deck_suggestions WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
    }

    pub async fn list_pending_for_deck(
        pool: &SqlitePool,
        deck_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(&format!(
            "SELECT {SUGGESTION_COLUMNS} FROM deck_suggestions \
             WHERE deck_id = ? AND status = 'pending' ORDER BY created_at ASC"
        ))
        .bind(deck_id.to_string())
        .fetch_all(pool)
        .await
    }

    pub async fn resolve(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        resolved_by: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query(
            r#"UPDATE deck_suggestions
               SET status = ?, resolved_at = datetime('now','subsec'), resolved_by = ?
               WHERE id = ? AND status = 'pending'"#,
        )
        .bind(status)
        .bind(resolved_by)
        .bind(id.to_string())
        .execute(pool)
        .await?;
        Self::find_by_id(pool, id).await
    }

    /// Supersede all pending suggestions from a previous Lux run on the same deck.
    pub async fn supersede_run(
        pool: &SqlitePool,
        deck_id: Uuid,
        run_id: &str,
    ) -> Result<u64, sqlx::Error> {
        let r = sqlx::query(
            r#"UPDATE deck_suggestions
               SET status = 'superseded', resolved_at = datetime('now','subsec'),
                   resolved_by = 'system'
               WHERE deck_id = ? AND status = 'pending' AND run_id = ?"#,
        )
        .bind(deck_id.to_string())
        .bind(run_id)
        .execute(pool)
        .await?;
        Ok(r.rows_affected())
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn deck_json_err(e: serde_json::Error) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        e.to_string(),
    )))
}

// ─── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_text() -> SlideElement {
        SlideElement::Text(TextElement {
            id: Uuid::new_v4(),
            bbox: BBox {
                x: 120.0,
                y: 80.0,
                w: 1680.0,
                h: 160.0,
                rotation: None,
            },
            z: 1,
            origin: "lux".into(),
            locked_by: None,
            opacity: None,
            token_refs: None,
            text: "Hello Lux".into(),
            font_family: "Inter".into(),
            font_size: 72.0,
            font_weight: 700,
            line_height: 1.15,
            letter_spacing: -0.01,
            color: Color {
                r: 10.0,
                g: 31.0,
                b: 59.0,
                a: 1.0,
                token: Some("color.brand.primary".into()),
            },
            align: "left".into(),
            runs: None,
        })
    }

    fn sample_shape() -> SlideElement {
        SlideElement::Shape(ShapeElement {
            id: Uuid::new_v4(),
            bbox: BBox {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0,
                rotation: None,
            },
            z: 0,
            origin: "lux".into(),
            locked_by: None,
            opacity: None,
            token_refs: None,
            shape: "rect".into(),
            path: None,
            fill: Fill::Solid {
                color: Color {
                    r: 255.0,
                    g: 255.0,
                    b: 255.0,
                    a: 1.0,
                    token: None,
                },
            },
            stroke: None,
            corner_radius: None,
        })
    }

    #[test]
    fn slide_element_text_round_trip() {
        let el = sample_text();
        let json = serde_json::to_string(&el).expect("serialize");
        let back: SlideElement = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(el, back);
    }

    #[test]
    fn slide_element_shape_round_trip() {
        let el = sample_shape();
        let json = serde_json::to_string(&el).expect("serialize");
        let back: SlideElement = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(el, back);
    }

    #[test]
    fn slide_element_group_round_trip() {
        let group = SlideElement::Group(GroupElement {
            id: Uuid::new_v4(),
            bbox: BBox {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0,
                rotation: None,
            },
            z: 2,
            origin: "lux".into(),
            locked_by: None,
            opacity: None,
            token_refs: None,
            children: vec![sample_text(), sample_shape()],
        });
        let json = serde_json::to_string(&group).expect("serialize");
        let back: SlideElement = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(group, back);
    }

    #[test]
    fn fill_discriminator_uses_kind() {
        let fill = Fill::Solid {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
                token: None,
            },
        };
        let json = serde_json::to_string(&fill).expect("serialize");
        assert!(json.contains("\"kind\":\"solid\""));

        let gradient = Fill::LinearGradient {
            stops: vec![GradientStop {
                offset: 0.0,
                color: Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                    token: None,
                },
            }],
            angle: 45.0,
        };
        let json = serde_json::to_string(&gradient).expect("serialize");
        assert!(json.contains("\"kind\":\"linear-gradient\""));
    }

    #[test]
    fn slide_element_discriminator_uses_type() {
        let json = serde_json::to_string(&sample_text()).expect("serialize");
        assert!(json.contains("\"type\":\"text\""));
    }

    #[test]
    fn canvas_default_matches_plan_decision() {
        let c = Canvas::default();
        assert_eq!(c.width, 1920.0);
        assert_eq!(c.height, 1080.0);
        assert_eq!(c.unit, "px");
        assert_eq!(c.dpi, 72.0);
    }

    #[test]
    fn canvas_round_trip() {
        let c = Canvas::default();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Canvas = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
    }

    #[test]
    fn empty_slide_elements_round_trip() {
        let empty: Vec<SlideElement> = vec![];
        let json = serde_json::to_string(&empty).expect("serialize");
        assert_eq!(json, "[]");
        let back: Vec<SlideElement> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(empty, back);
    }

    #[test]
    fn slide_element_id_accessor_returns_correct_uuid() {
        let text = sample_text();
        let expected = match &text {
            SlideElement::Text(e) => e.id,
            _ => unreachable!(),
        };
        assert_eq!(text.id(), expected);

        let shape = sample_shape();
        let expected = match &shape {
            SlideElement::Shape(e) => e.id,
            _ => unreachable!(),
        };
        assert_eq!(shape.id(), expected);
    }
}
