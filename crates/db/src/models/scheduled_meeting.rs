use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScheduledMeeting {
    pub id: String,
    pub proposal_id: String,
    pub scheduled_at: String,
    pub duration_min: i32,
    pub location: Option<String>,
    pub agenda: Option<String>,
    pub channel: String,
    pub invite_status: String,
    pub invite_sent_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScheduledMeetingInvitee {
    pub id: String,
    pub scheduled_meeting_id: String,
    pub person_id: String,
    pub channel: String,
    pub channel_address: String,
    pub status: String,
    pub sent_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateScheduledMeeting {
    pub proposal_id: Uuid,
    pub scheduled_at: String,
    pub duration_min: Option<i32>,
    pub location: Option<String>,
    pub agenda: Option<String>,
    pub channel: String,
    pub invitees: Vec<CreateScheduledMeetingInvitee>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreateScheduledMeetingInvitee {
    pub person_id: Uuid,
    pub channel: String,
    pub channel_address: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ScheduledMeetingWithInvitees {
    #[serde(flatten)]
    pub meeting: ScheduledMeeting,
    pub invitees: Vec<ScheduledMeetingInvitee>,
}

impl ScheduledMeeting {
    pub async fn create(
        pool: &SqlitePool,
        data: &CreateScheduledMeeting,
    ) -> sqlx::Result<ScheduledMeetingWithInvitees> {
        let id = Uuid::new_v4();
        let duration_min = data.duration_min.unwrap_or(60);

        sqlx::query(
            r#"INSERT INTO scheduled_meetings
               (id, proposal_id, scheduled_at, duration_min, location, agenda, channel)
               VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
        )
        .bind(id.as_bytes().as_slice())
        .bind(data.proposal_id.as_bytes().as_slice())
        .bind(&data.scheduled_at)
        .bind(duration_min)
        .bind(&data.location)
        .bind(&data.agenda)
        .bind(&data.channel)
        .execute(pool)
        .await?;

        let mut invitees = Vec::new();
        for inv in &data.invitees {
            let inv_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO scheduled_meeting_invitees
                   (id, scheduled_meeting_id, person_id, channel, channel_address)
                   VALUES (?1,?2,?3,?4,?5)"#,
            )
            .bind(inv_id.as_bytes().as_slice())
            .bind(id.as_bytes().as_slice())
            .bind(inv.person_id.as_bytes().as_slice())
            .bind(&inv.channel)
            .bind(&inv.channel_address)
            .execute(pool)
            .await?;

            let row: ScheduledMeetingInvitee = sqlx::query_as(
                "SELECT * FROM scheduled_meeting_invitees WHERE id = ?1",
            )
            .bind(inv_id.as_bytes().as_slice())
            .fetch_one(pool)
            .await?;
            invitees.push(row);
        }

        let meeting: ScheduledMeeting = sqlx::query_as(
            "SELECT * FROM scheduled_meetings WHERE id = ?1",
        )
        .bind(id.as_bytes().as_slice())
        .fetch_one(pool)
        .await?;

        Ok(ScheduledMeetingWithInvitees { meeting, invitees })
    }

    pub async fn find_by_proposal(
        pool: &SqlitePool,
        proposal_id: Uuid,
    ) -> sqlx::Result<Vec<ScheduledMeetingWithInvitees>> {
        let meetings: Vec<ScheduledMeeting> = sqlx::query_as(
            "SELECT * FROM scheduled_meetings WHERE proposal_id = ?1 ORDER BY scheduled_at ASC",
        )
        .bind(proposal_id.as_bytes().as_slice())
        .fetch_all(pool)
        .await?;

        let mut result = Vec::new();
        for m in meetings {
            let invitees: Vec<ScheduledMeetingInvitee> = sqlx::query_as(
                "SELECT * FROM scheduled_meeting_invitees WHERE scheduled_meeting_id = ?1",
            )
            .bind(m.id.as_bytes())
            .fetch_all(pool)
            .await
            .unwrap_or_default();
            result.push(ScheduledMeetingWithInvitees { meeting: m, invitees });
        }
        Ok(result)
    }

    pub async fn mark_invitee_sent(
        pool: &SqlitePool,
        invitee_id: &str,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE scheduled_meeting_invitees SET status='sent', sent_at=datetime('now','subsec') WHERE id=?1",
        )
        .bind(invitee_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: &str,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE scheduled_meetings SET invite_status=?2, invite_sent_at=datetime('now','subsec'), updated_at=datetime('now','subsec') WHERE id=?1",
        )
        .bind(id)
        .bind(status)
        .execute(pool)
        .await?;
        Ok(())
    }
}
