use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Json, FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "cinematic_brief_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CinematicBriefStatus {
    Pending,
    Planning,
    Rendering,
    Delivered,
    Failed,
    Cancelled,
}

impl Default for CinematicBriefStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct CinematicBrief {
    pub id: Uuid,
    pub project_id: Uuid,
    pub requester_id: String,
    pub nora_session_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub script: String,
    #[ts(type = "string[]")]
    pub asset_ids: Json<Value>,
    pub duration_seconds: i64,
    pub fps: i64,
    #[ts(type = "string[]")]
    pub style_tags: Json<Value>,
    pub status: CinematicBriefStatus,
    pub llm_notes: String,
    #[ts(type = "Record<string, unknown>")]
    pub render_payload: Json<Value>,
    #[ts(type = "string[]")]
    pub output_assets: Json<Value>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Json<Value>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct CinematicShotPlan {
    pub id: Uuid,
    pub brief_id: Uuid,
    pub shot_index: i64,
    pub title: String,
    pub prompt: String,
    pub negative_prompt: String,
    pub camera_notes: String,
    pub duration_seconds: i64,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Json<Value>,
    pub status: CinematicShotPlanStatus,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "cinematic_shot_plan_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CinematicShotPlanStatus {
    Pending,
    Planned,
    Rendering,
    Completed,
    Failed,
}

impl Default for CinematicShotPlanStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCinematicBrief {
    pub project_id: Uuid,
    pub requester_id: String,
    pub nora_session_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub script: Option<String>,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub fps: Option<i64>,
    #[serde(default)]
    pub style_tags: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCinematicBriefStatus {
    pub status: CinematicBriefStatus,
    #[serde(default)]
    pub llm_notes: Option<String>,
    #[serde(default)]
    pub render_payload: Option<Value>,
    #[serde(default)]
    pub output_assets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCinematicShotPlan {
    pub brief_id: Uuid,
    pub shot_index: i64,
    pub title: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub camera_notes: Option<String>,
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub metadata: Option<Value>,
    pub status: Option<CinematicShotPlanStatus>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCinematicShotPlanStatus {
    pub status: CinematicShotPlanStatus,
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl CinematicBrief {
    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateCinematicBrief,
        brief_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let asset_ids_value = Value::Array(
            payload
                .asset_ids
                .iter()
                .map(|id| Value::String(id.to_string()))
                .collect(),
        );
        let style_tags_value = Value::Array(
            payload
                .style_tags
                .iter()
                .map(|tag| Value::String(tag.to_string()))
                .collect(),
        );
        let metadata_value = payload
            .metadata
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let script = payload.script.clone().unwrap_or_default();
        let asset_ids_json = Json(asset_ids_value);
        let duration_seconds = payload.duration_seconds.unwrap_or(30);
        let fps = payload.fps.unwrap_or(24);
        let style_tags_json = Json(style_tags_value);
        let pending_notes = String::new();
        let initial_render_payload = Json(Value::Object(serde_json::Map::new()));
        let empty_output_assets = Json(Value::Array(vec![]));
        let metadata_json = Json(metadata_value);

        sqlx::query_as!(
            CinematicBrief,
            r#"
            INSERT INTO cinematic_briefs (
                id,
                project_id,
                requester_id,
                nora_session_id,
                title,
                summary,
                script,
                asset_ids,
                duration_seconds,
                fps,
                style_tags,
                status,
                llm_notes,
                render_payload,
                output_assets,
                metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12, $13, $14, $15, $16
            )
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                requester_id as "requester_id!: String",
                nora_session_id,
                title as "title!: String",
                summary as "summary!: String",
                script as "script!: String",
                asset_ids as "asset_ids: Json<Value>",
                duration_seconds as "duration_seconds!: i64",
                fps as "fps!: i64",
                style_tags as "style_tags: Json<Value>",
                status as "status!: CinematicBriefStatus",
                llm_notes as "llm_notes!: String",
                render_payload as "render_payload: Json<Value>",
                output_assets as "output_assets: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            brief_id,
            payload.project_id,
            payload.requester_id,
            payload.nora_session_id,
            payload.title,
            payload.summary,
            script,
            asset_ids_json,
            duration_seconds,
            fps,
            style_tags_json,
            CinematicBriefStatus::Pending,
            pending_notes,
            initial_render_payload,
            empty_output_assets,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CinematicBrief,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                requester_id as "requester_id!: String",
                nora_session_id,
                title as "title!: String",
                summary as "summary!: String",
                script as "script!: String",
                asset_ids as "asset_ids: Json<Value>",
                duration_seconds as "duration_seconds!: i64",
                fps as "fps!: i64",
                style_tags as "style_tags: Json<Value>",
                status as "status!: CinematicBriefStatus",
                llm_notes as "llm_notes!: String",
                render_payload as "render_payload: Json<Value>",
                output_assets as "output_assets: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cinematic_briefs WHERE id = $1"#,
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
            CinematicBrief,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                requester_id as "requester_id!: String",
                nora_session_id,
                title as "title!: String",
                summary as "summary!: String",
                script as "script!: String",
                asset_ids as "asset_ids: Json<Value>",
                duration_seconds as "duration_seconds!: i64",
                fps as "fps!: i64",
                style_tags as "style_tags: Json<Value>",
                status as "status!: CinematicBriefStatus",
                llm_notes as "llm_notes!: String",
                render_payload as "render_payload: Json<Value>",
                output_assets as "output_assets: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cinematic_briefs
            WHERE project_id = $1
            ORDER BY created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateCinematicBriefStatus,
    ) -> Result<Self, sqlx::Error> {
        let llm_notes = payload.llm_notes.clone().unwrap_or_default();
        let render_payload = payload
            .render_payload
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let output_assets = payload
            .output_assets
            .clone()
            .unwrap_or_default();
        let render_payload_json = Json(render_payload);
        let output_assets_json = Json(Value::from(output_assets));
        let status = payload.status.clone();

        sqlx::query_as!(
            CinematicBrief,
            r#"
            UPDATE cinematic_briefs
            SET
                status = $2,
                llm_notes = $3,
                render_payload = $4,
                output_assets = $5,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                requester_id as "requester_id!: String",
                nora_session_id,
                title as "title!: String",
                summary as "summary!: String",
                script as "script!: String",
                asset_ids as "asset_ids: Json<Value>",
                duration_seconds as "duration_seconds!: i64",
                fps as "fps!: i64",
                style_tags as "style_tags: Json<Value>",
                status as "status!: CinematicBriefStatus",
                llm_notes as "llm_notes!: String",
                render_payload as "render_payload: Json<Value>",
                output_assets as "output_assets: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            id,
            status,
            llm_notes,
            render_payload_json,
            output_assets_json
        )
        .fetch_one(pool)
        .await
    }
}

impl CinematicShotPlan {
    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateCinematicShotPlan,
        shot_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let negative_prompt = payload.negative_prompt.clone().unwrap_or_default();
        let camera_notes = payload.camera_notes.clone().unwrap_or_default();
        let duration_seconds = payload.duration_seconds.unwrap_or(4);
        let metadata_json = Json(
            payload
                .metadata
                .clone()
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
        );
        let status = payload.status.clone().unwrap_or_default();

        sqlx::query_as!(
            CinematicShotPlan,
            r#"
            INSERT INTO cinematic_shot_plans (
                id,
                brief_id,
                shot_index,
                title,
                prompt,
                negative_prompt,
                camera_notes,
                duration_seconds,
                metadata,
                status
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            )
            RETURNING
                id as "id!: Uuid",
                brief_id as "brief_id!: Uuid",
                shot_index as "shot_index!: i64",
                title as "title!: String",
                prompt as "prompt!: String",
                negative_prompt as "negative_prompt!: String",
                camera_notes as "camera_notes!: String",
                duration_seconds as "duration_seconds!: i64",
                metadata as "metadata: Json<Value>",
                status as "status!: CinematicShotPlanStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            shot_id,
            payload.brief_id,
            payload.shot_index,
            payload.title,
            payload.prompt,
            negative_prompt,
            camera_notes,
            duration_seconds,
            metadata_json,
            status
        )
        .fetch_one(pool)
        .await
    }

    pub async fn list_by_brief(
        pool: &SqlitePool,
        brief_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CinematicShotPlan,
            r#"SELECT
                id as "id!: Uuid",
                brief_id as "brief_id!: Uuid",
                shot_index as "shot_index!: i64",
                title as "title!: String",
                prompt as "prompt!: String",
                negative_prompt as "negative_prompt!: String",
                camera_notes as "camera_notes!: String",
                duration_seconds as "duration_seconds!: i64",
                metadata as "metadata: Json<Value>",
                status as "status!: CinematicShotPlanStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM cinematic_shot_plans
            WHERE brief_id = $1
            ORDER BY shot_index ASC"#,
            brief_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        shot_id: Uuid,
        payload: &UpdateCinematicShotPlanStatus,
    ) -> Result<Self, sqlx::Error> {
        let metadata_value = payload
            .metadata
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let metadata_json = Json(metadata_value);
        let status = payload.status.clone();

        sqlx::query_as!(
            CinematicShotPlan,
            r#"
            UPDATE cinematic_shot_plans
            SET
                status = $2,
                metadata = $3,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                brief_id as "brief_id!: Uuid",
                shot_index as "shot_index!: i64",
                title as "title!: String",
                prompt as "prompt!: String",
                negative_prompt as "negative_prompt!: String",
                camera_notes as "camera_notes!: String",
                duration_seconds as "duration_seconds!: i64",
                metadata as "metadata: Json<Value>",
                status as "status!: CinematicShotPlanStatus",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            shot_id,
            status,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }
}
