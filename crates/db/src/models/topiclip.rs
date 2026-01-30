//! TopiClips - AI-generated artistic video clips from topology evolution
//! "Beeple Everydays from Topsi"

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Json, FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

// ============================================================================
// TopiClip Session
// ============================================================================

#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "topiclip_session_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopiClipSessionStatus {
    Pending,
    Analyzing,
    Interpreting,
    Rendering,
    Delivered,
    Failed,
    Cancelled,
}

impl Default for TopiClipSessionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "topiclip_trigger_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopiClipTriggerType {
    Daily,
    Event,
    Manual,
}

impl Default for TopiClipTriggerType {
    fn default() -> Self {
        Self::Manual
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub day_number: i64,
    pub trigger_type: TopiClipTriggerType,
    pub primary_theme: Option<String>,
    pub emotional_arc: Option<String>,
    pub narrative_summary: Option<String>,
    pub artistic_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    #[ts(type = "Record<string, unknown> | null")]
    pub symbol_mapping: Option<Json<Value>>,
    pub status: TopiClipSessionStatus,
    pub cinematic_brief_id: Option<Uuid>,
    #[ts(type = "string[] | null")]
    pub output_asset_ids: Option<Json<Value>>,
    pub duration_seconds: Option<i64>,
    pub llm_notes: Option<String>,
    pub error_message: Option<String>,
    pub events_analyzed: i64,
    pub significance_score: Option<f64>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateTopiClipSession {
    pub project_id: Uuid,
    pub title: String,
    pub day_number: i64,
    #[serde(default)]
    pub trigger_type: TopiClipTriggerType,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTopiClipSessionStatus {
    pub status: TopiClipSessionStatus,
    #[serde(default)]
    pub primary_theme: Option<String>,
    #[serde(default)]
    pub emotional_arc: Option<String>,
    #[serde(default)]
    pub narrative_summary: Option<String>,
    #[serde(default)]
    pub artistic_prompt: Option<String>,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub symbol_mapping: Option<Value>,
    #[serde(default)]
    pub cinematic_brief_id: Option<Uuid>,
    #[serde(default)]
    pub output_asset_ids: Option<Vec<String>>,
    #[serde(default)]
    pub llm_notes: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub events_analyzed: Option<i64>,
    #[serde(default)]
    pub significance_score: Option<f64>,
}

impl TopiClipSession {
    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateTopiClipSession,
        session_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopiClipSession,
            r#"
            INSERT INTO topiclip_sessions (
                id, project_id, title, day_number, trigger_type,
                period_start, period_end
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                title as "title!: String",
                day_number as "day_number!: i64",
                trigger_type as "trigger_type!: TopiClipTriggerType",
                primary_theme,
                emotional_arc,
                narrative_summary,
                artistic_prompt,
                negative_prompt,
                symbol_mapping as "symbol_mapping: Json<Value>",
                status as "status!: TopiClipSessionStatus",
                cinematic_brief_id as "cinematic_brief_id: Uuid",
                output_asset_ids as "output_asset_ids: Json<Value>",
                duration_seconds,
                llm_notes,
                error_message,
                events_analyzed as "events_analyzed!: i64",
                significance_score,
                period_start,
                period_end,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                delivered_at
            "#,
            session_id,
            payload.project_id,
            payload.title,
            payload.day_number,
            payload.trigger_type,
            payload.period_start,
            payload.period_end
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipSession,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                title as "title!: String",
                day_number as "day_number!: i64",
                trigger_type as "trigger_type!: TopiClipTriggerType",
                primary_theme,
                emotional_arc,
                narrative_summary,
                artistic_prompt,
                negative_prompt,
                symbol_mapping as "symbol_mapping: Json<Value>",
                status as "status!: TopiClipSessionStatus",
                cinematic_brief_id as "cinematic_brief_id: Uuid",
                output_asset_ids as "output_asset_ids: Json<Value>",
                duration_seconds,
                llm_notes,
                error_message,
                events_analyzed as "events_analyzed!: i64",
                significance_score,
                period_start,
                period_end,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                delivered_at
            FROM topiclip_sessions WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipSession,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                title as "title!: String",
                day_number as "day_number!: i64",
                trigger_type as "trigger_type!: TopiClipTriggerType",
                primary_theme,
                emotional_arc,
                narrative_summary,
                artistic_prompt,
                negative_prompt,
                symbol_mapping as "symbol_mapping: Json<Value>",
                status as "status!: TopiClipSessionStatus",
                cinematic_brief_id as "cinematic_brief_id: Uuid",
                output_asset_ids as "output_asset_ids: Json<Value>",
                duration_seconds,
                llm_notes,
                error_message,
                events_analyzed as "events_analyzed!: i64",
                significance_score,
                period_start,
                period_end,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                delivered_at
            FROM topiclip_sessions
            WHERE project_id = $1
            ORDER BY day_number DESC, created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_latest_day_number(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar!(
            r#"SELECT COALESCE(MAX(day_number), 0) as "day_number!: i64"
            FROM topiclip_sessions WHERE project_id = $1"#,
            project_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result)
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateTopiClipSessionStatus,
    ) -> Result<Self, sqlx::Error> {
        let symbol_mapping_json = payload
            .symbol_mapping
            .clone()
            .map(Json);
        let output_assets_json = payload
            .output_asset_ids
            .clone()
            .map(|v| Json(Value::from(v)));

        sqlx::query_as!(
            TopiClipSession,
            r#"
            UPDATE topiclip_sessions
            SET
                status = $2,
                primary_theme = COALESCE($3, primary_theme),
                emotional_arc = COALESCE($4, emotional_arc),
                narrative_summary = COALESCE($5, narrative_summary),
                artistic_prompt = COALESCE($6, artistic_prompt),
                negative_prompt = COALESCE($7, negative_prompt),
                symbol_mapping = COALESCE($8, symbol_mapping),
                cinematic_brief_id = COALESCE($9, cinematic_brief_id),
                output_asset_ids = COALESCE($10, output_asset_ids),
                llm_notes = COALESCE($11, llm_notes),
                error_message = COALESCE($12, error_message),
                events_analyzed = COALESCE($13, events_analyzed),
                significance_score = COALESCE($14, significance_score),
                updated_at = datetime('now', 'subsec'),
                delivered_at = CASE WHEN $2 = 'delivered' THEN datetime('now', 'subsec') ELSE delivered_at END
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                title as "title!: String",
                day_number as "day_number!: i64",
                trigger_type as "trigger_type!: TopiClipTriggerType",
                primary_theme,
                emotional_arc,
                narrative_summary,
                artistic_prompt,
                negative_prompt,
                symbol_mapping as "symbol_mapping: Json<Value>",
                status as "status!: TopiClipSessionStatus",
                cinematic_brief_id as "cinematic_brief_id: Uuid",
                output_asset_ids as "output_asset_ids: Json<Value>",
                duration_seconds,
                llm_notes,
                error_message,
                events_analyzed as "events_analyzed!: i64",
                significance_score,
                period_start,
                period_end,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                delivered_at
            "#,
            id,
            payload.status,
            payload.primary_theme,
            payload.emotional_arc,
            payload.narrative_summary,
            payload.artistic_prompt,
            payload.negative_prompt,
            symbol_mapping_json,
            payload.cinematic_brief_id,
            output_assets_json,
            payload.llm_notes,
            payload.error_message,
            payload.events_analyzed,
            payload.significance_score
        )
        .fetch_one(pool)
        .await
    }
}

// ============================================================================
// TopiClip Captured Events
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipCapturedEvent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub event_type: String,
    #[ts(type = "Record<string, unknown>")]
    pub event_data: Json<Value>,
    pub narrative_role: Option<String>,
    pub significance_score: Option<f64>,
    pub assigned_symbol: Option<String>,
    pub symbol_prompt: Option<String>,
    #[ts(type = "string[] | null")]
    pub affected_node_ids: Option<Json<Value>>,
    #[ts(type = "string[] | null")]
    pub affected_edge_ids: Option<Json<Value>>,
    pub occurred_at: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateTopiClipCapturedEvent {
    pub session_id: Uuid,
    pub event_type: String,
    pub event_data: Value,
    pub narrative_role: Option<String>,
    pub significance_score: Option<f64>,
    pub assigned_symbol: Option<String>,
    pub symbol_prompt: Option<String>,
    pub affected_node_ids: Option<Vec<String>>,
    pub affected_edge_ids: Option<Vec<String>>,
    pub occurred_at: String,
}

impl TopiClipCapturedEvent {
    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateTopiClipCapturedEvent,
        event_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let event_data_json = Json(payload.event_data.clone());
        let node_ids_json = payload
            .affected_node_ids
            .clone()
            .map(|v| Json(Value::from(v)));
        let edge_ids_json = payload
            .affected_edge_ids
            .clone()
            .map(|v| Json(Value::from(v)));

        sqlx::query_as!(
            TopiClipCapturedEvent,
            r#"
            INSERT INTO topiclip_captured_events (
                id, session_id, event_type, event_data,
                narrative_role, significance_score,
                assigned_symbol, symbol_prompt,
                affected_node_ids, affected_edge_ids, occurred_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id as "id!: Uuid",
                session_id as "session_id!: Uuid",
                event_type as "event_type!: String",
                event_data as "event_data!: Json<Value>",
                narrative_role,
                significance_score,
                assigned_symbol,
                symbol_prompt,
                affected_node_ids as "affected_node_ids: Json<Value>",
                affected_edge_ids as "affected_edge_ids: Json<Value>",
                occurred_at as "occurred_at!: String",
                created_at as "created_at!: DateTime<Utc>"
            "#,
            event_id,
            payload.session_id,
            payload.event_type,
            event_data_json,
            payload.narrative_role,
            payload.significance_score,
            payload.assigned_symbol,
            payload.symbol_prompt,
            node_ids_json,
            edge_ids_json,
            payload.occurred_at
        )
        .fetch_one(pool)
        .await
    }

    pub async fn list_by_session(
        pool: &SqlitePool,
        session_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipCapturedEvent,
            r#"SELECT
                id as "id!: Uuid",
                session_id as "session_id!: Uuid",
                event_type as "event_type!: String",
                event_data as "event_data!: Json<Value>",
                narrative_role,
                significance_score,
                assigned_symbol,
                symbol_prompt,
                affected_node_ids as "affected_node_ids: Json<Value>",
                affected_edge_ids as "affected_edge_ids: Json<Value>",
                occurred_at as "occurred_at!: String",
                created_at as "created_at!: DateTime<Utc>"
            FROM topiclip_captured_events
            WHERE session_id = $1
            ORDER BY significance_score DESC NULLS LAST, occurred_at ASC"#,
            session_id
        )
        .fetch_all(pool)
        .await
    }
}

// ============================================================================
// TopiClip Daily Schedule
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipDailySchedule {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scheduled_time: String,
    pub timezone: Option<String>,
    pub is_enabled: bool,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub total_clips_generated: i64,
    pub last_generation_date: Option<String>,
    pub min_significance_threshold: Option<f64>,
    pub force_daily: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateTopiClipDailySchedule {
    pub project_id: Uuid,
    pub scheduled_time: String,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub min_significance_threshold: Option<f64>,
    #[serde(default)]
    pub force_daily: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTopiClipDailySchedule {
    pub scheduled_time: Option<String>,
    pub timezone: Option<String>,
    pub is_enabled: Option<bool>,
    pub min_significance_threshold: Option<f64>,
    pub force_daily: Option<bool>,
}

impl TopiClipDailySchedule {
    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateTopiClipDailySchedule,
        schedule_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let force_daily = payload.force_daily.unwrap_or(false);

        sqlx::query_as!(
            TopiClipDailySchedule,
            r#"
            INSERT INTO topiclip_daily_schedule (
                id, project_id, scheduled_time, timezone,
                min_significance_threshold, force_daily
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                scheduled_time as "scheduled_time!: String",
                timezone,
                is_enabled as "is_enabled!: bool",
                current_streak as "current_streak!: i64",
                longest_streak as "longest_streak!: i64",
                total_clips_generated as "total_clips_generated!: i64",
                last_generation_date,
                min_significance_threshold,
                force_daily as "force_daily!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            schedule_id,
            payload.project_id,
            payload.scheduled_time,
            payload.timezone,
            payload.min_significance_threshold,
            force_daily
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipDailySchedule,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                scheduled_time as "scheduled_time!: String",
                timezone,
                is_enabled as "is_enabled!: bool",
                current_streak as "current_streak!: i64",
                longest_streak as "longest_streak!: i64",
                total_clips_generated as "total_clips_generated!: i64",
                last_generation_date,
                min_significance_threshold,
                force_daily as "force_daily!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topiclip_daily_schedule WHERE project_id = $1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list_enabled(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipDailySchedule,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                scheduled_time as "scheduled_time!: String",
                timezone,
                is_enabled as "is_enabled!: bool",
                current_streak as "current_streak!: i64",
                longest_streak as "longest_streak!: i64",
                total_clips_generated as "total_clips_generated!: i64",
                last_generation_date,
                min_significance_threshold,
                force_daily as "force_daily!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topiclip_daily_schedule WHERE is_enabled = 1"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update_streak(
        pool: &SqlitePool,
        id: Uuid,
        generation_date: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopiClipDailySchedule,
            r#"
            UPDATE topiclip_daily_schedule
            SET
                current_streak = current_streak + 1,
                longest_streak = MAX(longest_streak, current_streak + 1),
                total_clips_generated = total_clips_generated + 1,
                last_generation_date = $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                scheduled_time as "scheduled_time!: String",
                timezone,
                is_enabled as "is_enabled!: bool",
                current_streak as "current_streak!: i64",
                longest_streak as "longest_streak!: i64",
                total_clips_generated as "total_clips_generated!: i64",
                last_generation_date,
                min_significance_threshold,
                force_daily as "force_daily!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            id,
            generation_date
        )
        .fetch_one(pool)
        .await
    }

    pub async fn reset_streak(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopiClipDailySchedule,
            r#"
            UPDATE topiclip_daily_schedule
            SET
                current_streak = 0,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                scheduled_time as "scheduled_time!: String",
                timezone,
                is_enabled as "is_enabled!: bool",
                current_streak as "current_streak!: i64",
                longest_streak as "longest_streak!: i64",
                total_clips_generated as "total_clips_generated!: i64",
                last_generation_date,
                min_significance_threshold,
                force_daily as "force_daily!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }
}

// ============================================================================
// TopiClip Config
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipConfig {
    pub id: Uuid,
    pub project_id: Uuid,
    pub default_style: Option<String>,
    #[ts(type = "string[] | null")]
    pub color_palette: Option<Json<Value>>,
    pub visual_density: Option<String>,
    pub motion_intensity: Option<String>,
    pub llm_model: Option<String>,
    pub interpretation_temperature: Option<f64>,
    pub output_resolution: Option<String>,
    pub output_fps: Option<i64>,
    pub output_format: Option<String>,
    pub significance_algorithm: Option<String>,
    #[ts(type = "string[] | null")]
    pub include_event_types: Option<Json<Value>>,
    #[ts(type = "string[] | null")]
    pub exclude_event_types: Option<Json<Value>>,
    #[ts(type = "Record<string, unknown> | null")]
    pub custom_symbol_mappings: Option<Json<Value>>,
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<Json<Value>>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateTopiClipConfig {
    pub project_id: Uuid,
    #[serde(default)]
    pub default_style: Option<String>,
    #[serde(default)]
    pub color_palette: Option<Vec<String>>,
    #[serde(default)]
    pub visual_density: Option<String>,
    #[serde(default)]
    pub motion_intensity: Option<String>,
    #[serde(default)]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub interpretation_temperature: Option<f64>,
    #[serde(default)]
    pub output_resolution: Option<String>,
    #[serde(default)]
    pub output_fps: Option<i64>,
    #[serde(default)]
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTopiClipConfig {
    pub default_style: Option<String>,
    pub color_palette: Option<Vec<String>>,
    pub visual_density: Option<String>,
    pub motion_intensity: Option<String>,
    pub llm_model: Option<String>,
    pub interpretation_temperature: Option<f64>,
    pub output_resolution: Option<String>,
    pub output_fps: Option<i64>,
    pub output_format: Option<String>,
    pub significance_algorithm: Option<String>,
    pub include_event_types: Option<Vec<String>>,
    pub exclude_event_types: Option<Vec<String>>,
    pub custom_symbol_mappings: Option<Value>,
}

impl TopiClipConfig {
    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateTopiClipConfig,
        config_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let color_palette_json = payload
            .color_palette
            .clone()
            .map(|v| Json(Value::from(v)));

        sqlx::query_as!(
            TopiClipConfig,
            r#"
            INSERT INTO topiclip_config (
                id, project_id, default_style, color_palette,
                visual_density, motion_intensity, llm_model,
                interpretation_temperature, output_resolution,
                output_fps, output_format
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                default_style,
                color_palette as "color_palette: Json<Value>",
                visual_density,
                motion_intensity,
                llm_model,
                interpretation_temperature,
                output_resolution,
                output_fps,
                output_format,
                significance_algorithm,
                include_event_types as "include_event_types: Json<Value>",
                exclude_event_types as "exclude_event_types: Json<Value>",
                custom_symbol_mappings as "custom_symbol_mappings: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            config_id,
            payload.project_id,
            payload.default_style,
            color_palette_json,
            payload.visual_density,
            payload.motion_intensity,
            payload.llm_model,
            payload.interpretation_temperature,
            payload.output_resolution,
            payload.output_fps,
            payload.output_format
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipConfig,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                default_style,
                color_palette as "color_palette: Json<Value>",
                visual_density,
                motion_intensity,
                llm_model,
                interpretation_temperature,
                output_resolution,
                output_fps,
                output_format,
                significance_algorithm,
                include_event_types as "include_event_types: Json<Value>",
                exclude_event_types as "exclude_event_types: Json<Value>",
                custom_symbol_mappings as "custom_symbol_mappings: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topiclip_config WHERE project_id = $1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        payload: &CreateTopiClipConfig,
        config_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let color_palette_json = payload
            .color_palette
            .clone()
            .map(|v| Json(Value::from(v)));

        sqlx::query_as!(
            TopiClipConfig,
            r#"
            INSERT INTO topiclip_config (
                id, project_id, default_style, color_palette,
                visual_density, motion_intensity, llm_model,
                interpretation_temperature, output_resolution,
                output_fps, output_format
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT(project_id) DO UPDATE SET
                default_style = COALESCE(excluded.default_style, topiclip_config.default_style),
                color_palette = COALESCE(excluded.color_palette, topiclip_config.color_palette),
                visual_density = COALESCE(excluded.visual_density, topiclip_config.visual_density),
                motion_intensity = COALESCE(excluded.motion_intensity, topiclip_config.motion_intensity),
                llm_model = COALESCE(excluded.llm_model, topiclip_config.llm_model),
                interpretation_temperature = COALESCE(excluded.interpretation_temperature, topiclip_config.interpretation_temperature),
                output_resolution = COALESCE(excluded.output_resolution, topiclip_config.output_resolution),
                output_fps = COALESCE(excluded.output_fps, topiclip_config.output_fps),
                output_format = COALESCE(excluded.output_format, topiclip_config.output_format),
                updated_at = datetime('now', 'subsec')
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                default_style,
                color_palette as "color_palette: Json<Value>",
                visual_density,
                motion_intensity,
                llm_model,
                interpretation_temperature,
                output_resolution,
                output_fps,
                output_format,
                significance_algorithm,
                include_event_types as "include_event_types: Json<Value>",
                exclude_event_types as "exclude_event_types: Json<Value>",
                custom_symbol_mappings as "custom_symbol_mappings: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            config_id,
            payload.project_id,
            payload.default_style,
            color_palette_json,
            payload.visual_density,
            payload.motion_intensity,
            payload.llm_model,
            payload.interpretation_temperature,
            payload.output_resolution,
            payload.output_fps,
            payload.output_format
        )
        .fetch_one(pool)
        .await
    }
}

// ============================================================================
// TopiClip Symbol Library
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipSymbol {
    pub id: String,
    pub event_pattern: String,
    pub symbol_name: String,
    pub symbol_description: Option<String>,
    pub prompt_template: String,
    pub negative_template: Option<String>,
    pub theme_affinity: Option<String>,
    #[ts(type = "string[] | null")]
    pub emotional_range: Option<Json<Value>>,
    #[ts(type = "string[] | null")]
    pub suggested_colors: Option<Json<Value>>,
    pub motion_type: Option<String>,
    pub is_default: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl TopiClipSymbol {
    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipSymbol,
            r#"SELECT
                id as "id!: String",
                event_pattern as "event_pattern!: String",
                symbol_name as "symbol_name!: String",
                symbol_description,
                prompt_template as "prompt_template!: String",
                negative_template,
                theme_affinity,
                emotional_range as "emotional_range: Json<Value>",
                suggested_colors as "suggested_colors: Json<Value>",
                motion_type,
                is_default as "is_default!: bool",
                created_at as "created_at!: DateTime<Utc>"
            FROM topiclip_symbol_library
            ORDER BY event_pattern"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_pattern(
        pool: &SqlitePool,
        pattern: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipSymbol,
            r#"SELECT
                id as "id!: String",
                event_pattern as "event_pattern!: String",
                symbol_name as "symbol_name!: String",
                symbol_description,
                prompt_template as "prompt_template!: String",
                negative_template,
                theme_affinity,
                emotional_range as "emotional_range: Json<Value>",
                suggested_colors as "suggested_colors: Json<Value>",
                motion_type,
                is_default as "is_default!: bool",
                created_at as "created_at!: DateTime<Utc>"
            FROM topiclip_symbol_library
            WHERE event_pattern = $1"#,
            pattern
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_theme(
        pool: &SqlitePool,
        theme: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopiClipSymbol,
            r#"SELECT
                id as "id!: String",
                event_pattern as "event_pattern!: String",
                symbol_name as "symbol_name!: String",
                symbol_description,
                prompt_template as "prompt_template!: String",
                negative_template,
                theme_affinity,
                emotional_range as "emotional_range: Json<Value>",
                suggested_colors as "suggested_colors: Json<Value>",
                motion_type,
                is_default as "is_default!: bool",
                created_at as "created_at!: DateTime<Utc>"
            FROM topiclip_symbol_library
            WHERE theme_affinity = $1"#,
            theme
        )
        .fetch_all(pool)
        .await
    }
}

// ============================================================================
// Gallery and Timeline Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipGalleryResponse {
    pub sessions: Vec<TopiClipSession>,
    pub schedule: Option<TopiClipDailySchedule>,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub total_clips: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopiClipTimelineEntry {
    pub session: TopiClipSession,
    pub events: Vec<TopiClipCapturedEvent>,
    pub asset_urls: Vec<String>,
}
