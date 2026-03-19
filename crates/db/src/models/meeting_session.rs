use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum MeetingSessionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Meeting session not found")]
    NotFound,
    #[error("Access denied")]
    AccessDenied,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStatus {
    Active,
    Paused,
    Ended,
    Cancelled,
}

impl std::fmt::Display for MeetingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingStatus::Active => write!(f, "active"),
            MeetingStatus::Paused => write!(f, "paused"),
            MeetingStatus::Ended => write!(f, "ended"),
            MeetingStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MeetingSession {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub started_by: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<i32>,
    pub participant_count: Option<i32>,
    pub participants: Option<String>,
    pub transcript: Option<String>,
    pub notes: Option<String>,
    pub metadata: Option<String>,
    pub topology_node_id: Option<String>,
    pub access_level: Option<String>,
    pub shared_with: Option<String>,
    pub linked_person_ids: String, // JSON array e.g. '["uuid1","uuid2"]'
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MeetingSegment {
    pub id: String,
    pub meeting_session_id: String,
    pub segment_index: i32,
    pub speaker_label: Option<String>,
    pub text: String,
    pub confidence: Option<f64>,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub is_topsi_addressed: bool,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateMeetingSession {
    pub project_id: String,
    pub title: Option<String>,
    pub started_by: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateMeetingSegment {
    pub meeting_session_id: String,
    pub segment_index: i32,
    pub speaker_label: Option<String>,
    pub text: String,
    pub confidence: Option<f64>,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub is_topsi_addressed: bool,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateMeetingSession {
    pub title: Option<String>,
    pub status: Option<MeetingStatus>,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<i32>,
    pub participant_count: Option<i32>,
    pub participants: Option<String>,
    pub transcript: Option<String>,
    pub notes: Option<String>,
    pub topology_node_id: Option<String>,
    pub shared_with: Option<String>,
    pub linked_person_ids: Option<String>,
}

impl MeetingSession {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateMeetingSession,
    ) -> Result<Self, MeetingSessionError> {
        let id = Uuid::new_v4().to_string();
        let title = data.title.unwrap_or_else(|| "Untitled Meeting".to_string());

        let session = sqlx::query_as::<_, MeetingSession>(
            r#"
            INSERT INTO meeting_sessions (id, project_id, title, started_by, participant_count)
            VALUES (?1, ?2, ?3, ?4, 1)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&data.project_id)
        .bind(&title)
        .bind(&data.started_by)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Self, MeetingSessionError> {
        sqlx::query_as::<_, MeetingSession>(r#"SELECT * FROM meeting_sessions WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(MeetingSessionError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, MeetingSessionError> {
        let sessions = sqlx::query_as::<_, MeetingSession>(
            r#"
            SELECT * FROM meeting_sessions
            WHERE project_id = ?1
            ORDER BY started_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// List all meetings, optionally filtered by project_id
    pub async fn list(
        pool: &SqlitePool,
        project_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, MeetingSessionError> {
        let sessions = if let Some(pid) = project_id {
            sqlx::query_as::<_, MeetingSession>(
                r#"
                SELECT * FROM meeting_sessions
                WHERE project_id = ?1
                ORDER BY started_at DESC
                LIMIT ?2 OFFSET ?3
                "#,
            )
            .bind(pid)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, MeetingSession>(
                r#"
                SELECT * FROM meeting_sessions
                ORDER BY started_at DESC
                LIMIT ?1 OFFSET ?2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        };

        Ok(sessions)
    }

    pub async fn find_active_by_project(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<Self>, MeetingSessionError> {
        let sessions = sqlx::query_as::<_, MeetingSession>(
            r#"
            SELECT * FROM meeting_sessions
            WHERE project_id = ?1 AND status = 'active'
            ORDER BY started_at DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        data: UpdateMeetingSession,
    ) -> Result<Self, MeetingSessionError> {
        let status = data.status.map(|s| s.to_string());

        sqlx::query_as::<_, MeetingSession>(
            r#"
            UPDATE meeting_sessions SET
                title = COALESCE(?2, title),
                status = COALESCE(?3, status),
                ended_at = COALESCE(?4, ended_at),
                duration_seconds = COALESCE(?5, duration_seconds),
                participant_count = COALESCE(?6, participant_count),
                participants = COALESCE(?7, participants),
                transcript = COALESCE(?8, transcript),
                notes = COALESCE(?9, notes),
                topology_node_id = COALESCE(?10, topology_node_id),
                shared_with = COALESCE(?11, shared_with),
                linked_person_ids = COALESCE(?12, linked_person_ids),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&status)
        .bind(&data.ended_at)
        .bind(data.duration_seconds)
        .bind(data.participant_count)
        .bind(&data.participants)
        .bind(&data.transcript)
        .bind(&data.notes)
        .bind(&data.topology_node_id)
        .bind(&data.shared_with)
        .bind(&data.linked_person_ids)
        .fetch_optional(pool)
        .await?
        .ok_or(MeetingSessionError::NotFound)
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), MeetingSessionError> {
        let result = sqlx::query(r#"DELETE FROM meeting_sessions WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(MeetingSessionError::NotFound);
        }
        Ok(())
    }

    /// Check if a user has access to this meeting (sync — owner/admin/shared only)
    pub fn has_access(&self, user_id: &str, is_admin: bool) -> bool {
        if is_admin {
            return true;
        }
        if self.started_by == user_id {
            return true;
        }
        if let Some(ref shared) = self.shared_with {
            if let Ok(shared_users) = serde_json::from_str::<Vec<String>>(shared) {
                if shared_users.contains(&user_id.to_string()) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a user is a member of the project this meeting belongs to.
    /// project_members.project_id and .user_id are stored as BLOBs;
    /// meeting_sessions.project_id is stored as TEXT (UUID with dashes).
    pub async fn is_project_member(pool: &SqlitePool, session_id: &str, user_id: &str) -> bool {
        sqlx::query(
            r#"
            SELECT 1 FROM meeting_sessions ms
            JOIN project_members pm
                ON lower(hex(pm.project_id)) = lower(replace(ms.project_id, '-', ''))
            WHERE ms.id = ?1
              AND lower(hex(pm.user_id)) = lower(replace(?2, '-', ''))
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map(|r| r.is_some())
        .unwrap_or(false)
    }

    /// Link CRM persons to this meeting session by overwriting the linked_person_ids JSON array.
    pub async fn link_persons(
        pool: &SqlitePool,
        id: &str,
        person_ids: &[String],
    ) -> Result<Self, MeetingSessionError> {
        let json = serde_json::to_string(person_ids).unwrap_or_else(|_| "[]".to_string());
        MeetingSession::update(
            pool,
            id,
            UpdateMeetingSession {
                linked_person_ids: Some(json),
                ..Default::default()
            },
        )
        .await
    }

    /// Bump updated_at to keep the session alive (used by heartbeat endpoint).
    pub async fn heartbeat(pool: &SqlitePool, id: &str) -> Result<(), MeetingSessionError> {
        sqlx::query(
            r#"UPDATE meeting_sessions
               SET updated_at = datetime('now','subsec')
               WHERE id = ?1 AND status = 'active'"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Return all active sessions whose updated_at is older than `stale_secs` seconds.
    pub async fn find_stale_active(
        pool: &SqlitePool,
        stale_secs: i64,
    ) -> Result<Vec<Self>, MeetingSessionError> {
        let sessions = sqlx::query_as::<_, MeetingSession>(
            r#"
            SELECT * FROM meeting_sessions
            WHERE status = 'active'
              AND (unixepoch('now') - unixepoch(updated_at)) > ?1
            "#,
        )
        .bind(stale_secs)
        .fetch_all(pool)
        .await?;
        Ok(sessions)
    }
}

impl MeetingSegment {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateMeetingSegment,
    ) -> Result<Self, MeetingSessionError> {
        let id = Uuid::new_v4().to_string();

        let segment = sqlx::query_as::<_, MeetingSegment>(
            r#"
            INSERT INTO meeting_segments (
                id, meeting_session_id, segment_index, speaker_label,
                text, confidence, start_time_ms, end_time_ms, is_topsi_addressed
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&data.meeting_session_id)
        .bind(data.segment_index)
        .bind(&data.speaker_label)
        .bind(&data.text)
        .bind(data.confidence)
        .bind(data.start_time_ms)
        .bind(data.end_time_ms)
        .bind(data.is_topsi_addressed)
        .fetch_one(pool)
        .await?;

        Ok(segment)
    }

    pub async fn find_by_session(
        pool: &SqlitePool,
        meeting_session_id: &str,
    ) -> Result<Vec<Self>, MeetingSessionError> {
        let segments = sqlx::query_as::<_, MeetingSegment>(
            r#"
            SELECT * FROM meeting_segments
            WHERE meeting_session_id = ?1
            ORDER BY segment_index ASC
            "#,
        )
        .bind(meeting_session_id)
        .fetch_all(pool)
        .await?;

        Ok(segments)
    }

    pub async fn find_by_session_range(
        pool: &SqlitePool,
        meeting_session_id: &str,
        from_index: i32,
        limit: i64,
    ) -> Result<Vec<Self>, MeetingSessionError> {
        let segments = sqlx::query_as::<_, MeetingSegment>(
            r#"
            SELECT * FROM meeting_segments
            WHERE meeting_session_id = ?1 AND segment_index >= ?2
            ORDER BY segment_index ASC
            LIMIT ?3
            "#,
        )
        .bind(meeting_session_id)
        .bind(from_index)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(segments)
    }

    pub async fn count_by_session(
        pool: &SqlitePool,
        meeting_session_id: &str,
    ) -> Result<i64, MeetingSessionError> {
        #[derive(FromRow)]
        struct CountRow {
            count: i64,
        }

        let row = sqlx::query_as::<_, CountRow>(
            r#"SELECT COUNT(*) as count FROM meeting_segments WHERE meeting_session_id = ?1"#,
        )
        .bind(meeting_session_id)
        .fetch_one(pool)
        .await?;

        Ok(row.count)
    }
}
