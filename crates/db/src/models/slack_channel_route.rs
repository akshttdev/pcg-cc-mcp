use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Error)]
pub enum SlackRouteError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("slack route not found")]
    NotFound,
}

/// Closed set of events that can be routed to Slack. The DB CHECK constraint
/// must stay in sync with this enum.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SlackEventType {
    DealStageChanged,
    DealWon,
    DealLost,
    ProposalApproved,
    ProposalRejected,
    TaskAssigned,
    TaskCompleted,
    InvoicePaid,
    AgentEscalation,
}

impl SlackEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            SlackEventType::DealStageChanged => "deal_stage_changed",
            SlackEventType::DealWon => "deal_won",
            SlackEventType::DealLost => "deal_lost",
            SlackEventType::ProposalApproved => "proposal_approved",
            SlackEventType::ProposalRejected => "proposal_rejected",
            SlackEventType::TaskAssigned => "task_assigned",
            SlackEventType::TaskCompleted => "task_completed",
            SlackEventType::InvoicePaid => "invoice_paid",
            SlackEventType::AgentEscalation => "agent_escalation",
        }
    }
}

impl std::str::FromStr for SlackEventType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deal_stage_changed" => Ok(Self::DealStageChanged),
            "deal_won" => Ok(Self::DealWon),
            "deal_lost" => Ok(Self::DealLost),
            "proposal_approved" => Ok(Self::ProposalApproved),
            "proposal_rejected" => Ok(Self::ProposalRejected),
            "task_assigned" => Ok(Self::TaskAssigned),
            "task_completed" => Ok(Self::TaskCompleted),
            "invoice_paid" => Ok(Self::InvoicePaid),
            "agent_escalation" => Ok(Self::AgentEscalation),
            other => Err(format!("unknown slack event type: {other}")),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SlackChannelRoute {
    pub id: DbUuid,
    pub organization_id: DbUuid,
    pub event_type: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub enabled: i64,
    pub options: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct UpsertSlackRoute {
    pub organization_id: DbUuid,
    pub event_type: SlackEventType,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub enabled: Option<bool>,
}

impl SlackChannelRoute {
    pub async fn upsert(
        pool: &SqlitePool,
        data: UpsertSlackRoute,
    ) -> Result<Self, SlackRouteError> {
        let id = DbUuid::new();
        let row = sqlx::query_as::<_, SlackChannelRoute>(
            r#"
            INSERT INTO slack_channel_routing (
                id, organization_id, event_type, channel_id, channel_name, enabled
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(organization_id, event_type, channel_id) DO UPDATE SET
                channel_name = excluded.channel_name,
                enabled      = excluded.enabled,
                updated_at   = datetime('now','subsec')
            RETURNING id, organization_id, event_type, channel_id, channel_name,
                      enabled, options, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&data.organization_id)
        .bind(data.event_type.as_str())
        .bind(&data.channel_id)
        .bind(&data.channel_name)
        .bind(if data.enabled.unwrap_or(true) {
            1_i64
        } else {
            0
        })
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    pub async fn list_for_org(
        pool: &SqlitePool,
        organization_id: &DbUuid,
    ) -> Result<Vec<Self>, SlackRouteError> {
        let rows = sqlx::query_as::<_, SlackChannelRoute>(
            r#"
            SELECT id, organization_id, event_type, channel_id, channel_name,
                   enabled, options, created_at, updated_at
            FROM slack_channel_routing
            WHERE organization_id = ?1
            ORDER BY event_type ASC, channel_name ASC
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Lookup the active routes for one (org, event_type) pair. The dispatcher
    /// fans out to every returned channel.
    pub async fn find_active(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        event_type: SlackEventType,
    ) -> Result<Vec<Self>, SlackRouteError> {
        let rows = sqlx::query_as::<_, SlackChannelRoute>(
            r#"
            SELECT id, organization_id, event_type, channel_id, channel_name,
                   enabled, options, created_at, updated_at
            FROM slack_channel_routing
            WHERE organization_id = ?1
              AND event_type = ?2
              AND enabled = 1
            "#,
        )
        .bind(organization_id)
        .bind(event_type.as_str())
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete(pool: &SqlitePool, id: &DbUuid) -> Result<(), SlackRouteError> {
        let res = sqlx::query("DELETE FROM slack_channel_routing WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(SlackRouteError::NotFound);
        }
        Ok(())
    }
}

/// Per-dispatch log row. We record one of these per channel post — even when
/// the post fails — so operators can debug "why didn't Slack get my deal?".
pub async fn record_dispatch(
    pool: &SqlitePool,
    organization_id: &DbUuid,
    event_type: &str,
    channel_id: &str,
    slack_ts: Option<&str>,
    success: bool,
    error: Option<&str>,
    payload_summary: Option<&str>,
) -> Result<(), sqlx::Error> {
    let id = DbUuid::new();
    sqlx::query(
        r#"
        INSERT INTO slack_dispatch_log (
            id, organization_id, event_type, channel_id, slack_ts,
            success, error, payload_summary
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(&id)
    .bind(organization_id)
    .bind(event_type)
    .bind(channel_id)
    .bind(slack_ts)
    .bind(if success { 1_i64 } else { 0 })
    .bind(error)
    .bind(payload_summary)
    .execute(pool)
    .await?;
    Ok(())
}
