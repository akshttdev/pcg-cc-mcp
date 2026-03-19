//! Nora classifier prediction log — shadow infrastructure.
//!
//! These endpoints expose the prediction log written by the Discord bot's
//! context-aware "should Nora speak?" classifier.  The classifier runs in
//! shadow mode only: predictions are stored here for accuracy analysis but
//! they never alter Nora's wake-word-gated behaviour.
//!
//! Protected routes (JWT required):
//!   GET  /nora-classifier/predictions          — list recent predictions + accuracy stats
//!   PATCH /nora-classifier/predictions/:id     — manually correct ground-truth label
//!   GET  /nora-classifier/stats                — overall accuracy across all sessions
//!
//! Admin-key route (no JWT, X-Admin-Key header):
//!   POST /nora-classifier/predictions/log      — ingest a prediction from the Discord bot

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, patch, post},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ─── DB row ──────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierPrediction {
    pub id: String,
    pub meeting_session_id: String,
    pub segment_index: i64,
    pub speaker_label: String,
    pub utterance: String,
    pub context_json: String,
    pub predicted_speak: i64,
    pub confidence: String,
    pub reasoning: Option<String>,
    pub was_wake_word_addressed: Option<i64>,
    pub created_at: String,
}

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    pub session_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccuracyStats {
    pub total: i64,
    pub labelled: i64,
    pub correct: i64,
    /// true-positive rate among labelled rows where predicted_speak = 1
    pub precision: f64,
    /// recall = correct positive predictions / total actual positives
    pub recall: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse {
    pub predictions: Vec<ClassifierPrediction>,
    pub accuracy: AccuracyStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchLabelBody {
    pub was_correct: bool,
}

/// Payload sent by the Discord bot (fire-and-forget).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPredictionBody {
    pub meeting_session_id: String,
    pub segment_index: i64,
    pub speaker_label: String,
    pub utterance: String,
    pub context_json: String,
    pub predicted_speak: bool,
    pub confidence: String,
    pub reasoning: Option<String>,
    /// Whether wake-word was actually detected for this utterance.
    pub was_wake_word_addressed: bool,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn validate_admin_key(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = std::env::var("ADMIN_API_KEY").unwrap_or_default();
    let provided = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if expected.is_empty() || provided != expected {
        return Err(ApiError::Unauthorized("Admin key required".into()));
    }
    Ok(())
}

async fn compute_accuracy(
    pool: &sqlx::SqlitePool,
    session_filter: Option<&str>,
) -> Result<AccuracyStats, ApiError> {
    #[derive(FromRow)]
    struct Row {
        total: i64,
        labelled: i64,
        correct: i64,
        true_positives: i64,
        predicted_positives: i64,
        actual_positives: i64,
    }

    let row: Row = if let Some(sid) = session_filter {
        sqlx::query_as(
            r#"
            SELECT
                COUNT(*)                                                          AS total,
                COUNT(was_wake_word_addressed)                                    AS labelled,
                SUM(CASE WHEN was_wake_word_addressed IS NOT NULL
                          AND predicted_speak = was_wake_word_addressed
                     THEN 1 ELSE 0 END)                                          AS correct,
                SUM(CASE WHEN predicted_speak = 1
                          AND was_wake_word_addressed = 1 THEN 1 ELSE 0 END)     AS true_positives,
                SUM(CASE WHEN predicted_speak = 1 THEN 1 ELSE 0 END)             AS predicted_positives,
                SUM(CASE WHEN was_wake_word_addressed = 1 THEN 1 ELSE 0 END)     AS actual_positives
            FROM nora_classifier_predictions
            WHERE meeting_session_id = ?
            "#,
        )
        .bind(sid)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    } else {
        sqlx::query_as(
            r#"
            SELECT
                COUNT(*)                                                          AS total,
                COUNT(was_wake_word_addressed)                                    AS labelled,
                SUM(CASE WHEN was_wake_word_addressed IS NOT NULL
                          AND predicted_speak = was_wake_word_addressed
                     THEN 1 ELSE 0 END)                                          AS correct,
                SUM(CASE WHEN predicted_speak = 1
                          AND was_wake_word_addressed = 1 THEN 1 ELSE 0 END)     AS true_positives,
                SUM(CASE WHEN predicted_speak = 1 THEN 1 ELSE 0 END)             AS predicted_positives,
                SUM(CASE WHEN was_wake_word_addressed = 1 THEN 1 ELSE 0 END)     AS actual_positives
            FROM nora_classifier_predictions
            "#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    };

    let precision = if row.predicted_positives > 0 {
        row.true_positives as f64 / row.predicted_positives as f64
    } else {
        0.0
    };
    let recall = if row.actual_positives > 0 {
        row.true_positives as f64 / row.actual_positives as f64
    } else {
        0.0
    };

    Ok(AccuracyStats {
        total: row.total,
        labelled: row.labelled,
        correct: row.correct,
        precision,
        recall,
    })
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /nora-classifier/predictions
async fn list_predictions(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = q.limit.unwrap_or(50).min(500);

    let predictions: Vec<ClassifierPrediction> = if let Some(ref sid) = q.session_id {
        sqlx::query_as(
            "SELECT * FROM nora_classifier_predictions WHERE meeting_session_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(sid)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    } else {
        sqlx::query_as("SELECT * FROM nora_classifier_predictions ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
    };

    let accuracy = compute_accuracy(pool, q.session_id.as_deref()).await?;
    Ok(Json(ListResponse {
        predictions,
        accuracy,
    }))
}

/// PATCH /nora-classifier/predictions/:id
async fn patch_prediction(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<PatchLabelBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let ground_truth: i64 = if body.was_correct { 1 } else { 0 };

    // was_correct means the predicted_speak matched reality; derive actual value
    // by XOR-ing with prediction.  Fetch prediction first.
    #[derive(FromRow)]
    struct Row {
        predicted_speak: i64,
    }
    let row: Row =
        sqlx::query_as("SELECT predicted_speak FROM nora_classifier_predictions WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::NotFound(format!("Prediction {} not found", id)))?;

    // If was_correct=true → was_wake_word_addressed = predicted_speak
    // If was_correct=false → was_wake_word_addressed = 1 - predicted_speak
    let actual = if ground_truth == 1 {
        row.predicted_speak
    } else {
        1 - row.predicted_speak
    };

    sqlx::query("UPDATE nora_classifier_predictions SET was_wake_word_addressed = ? WHERE id = ?")
        .bind(actual)
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /nora-classifier/stats
async fn get_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<AccuracyStats>, ApiError> {
    let pool = &deployment.db().pool;
    let stats = compute_accuracy(pool, None).await?;
    Ok(Json(stats))
}

/// POST /nora-classifier/predictions/log  (admin-key auth, called by Discord bot)
async fn log_prediction(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<LogPredictionBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate_admin_key(&headers)?;

    let pool = &deployment.db().pool;
    let id = Uuid::new_v4().to_string();
    let predicted_speak: i64 = if body.predicted_speak { 1 } else { 0 };
    let was_wake_word: i64 = if body.was_wake_word_addressed { 1 } else { 0 };

    // Validate confidence value
    if !["high", "medium", "low"].contains(&body.confidence.as_str()) {
        return Err(ApiError::BadRequest(
            "confidence must be 'high', 'medium', or 'low'".into(),
        ));
    }

    sqlx::query(
        r#"INSERT INTO nora_classifier_predictions
           (id, meeting_session_id, segment_index, speaker_label, utterance,
            context_json, predicted_speak, confidence, reasoning, was_wake_word_addressed)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&body.meeting_session_id)
    .bind(body.segment_index)
    .bind(&body.speaker_label)
    .bind(&body.utterance)
    .bind(&body.context_json)
    .bind(predicted_speak)
    .bind(&body.confidence)
    .bind(&body.reasoning)
    .bind(was_wake_word)
    .execute(pool)
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(serde_json::json!({ "id": id })))
}

// ─── Router ──────────────────────────────────────────────────────────────────

/// Protected sub-router (JWT required) — merged into protected_routes.
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/nora-classifier/predictions", get(list_predictions))
        .route("/nora-classifier/predictions/{id}", patch(patch_prediction))
        .route("/nora-classifier/stats", get(get_stats))
        .with_state(deployment.clone())
}

/// Public sub-router (admin-key only) — merged into base_routes.
pub fn public_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/nora-classifier/predictions/log", post(log_prediction))
        .with_state(deployment.clone())
}
