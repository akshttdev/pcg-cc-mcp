-- Normalized calendar events from Google Calendar / Outlook (Microsoft Graph).
-- Tokens for the underlying calendar accounts live in `integration_connections`.
-- Per-account sync cursors live in `calendar_sync_cursor`.

CREATE TABLE IF NOT EXISTS calendar_events (
    id                       TEXT PRIMARY KEY NOT NULL,
    organization_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- integration_connections.id of the Google / Outlook account this event came from
    calendar_account_id      TEXT NOT NULL REFERENCES integration_connections(id) ON DELETE CASCADE,
    provider                 TEXT NOT NULL CHECK (provider IN ('google_calendar','outlook_calendar')),
    -- Provider-side stable id (Google "id", MS Graph "id")
    external_event_id        TEXT NOT NULL,
    -- Provider-side calendar/list id (Google "calendarId", MS Graph "calendar/id")
    external_calendar_id     TEXT,
    -- Raw status: 'confirmed' | 'tentative' | 'cancelled'
    status                   TEXT NOT NULL DEFAULT 'confirmed',
    title                    TEXT NOT NULL,
    description              TEXT,
    location                 TEXT,
    -- ISO-8601 timestamps. We store as TEXT (UTC) for portability with SQLite date functions.
    start_at                 TEXT NOT NULL,
    end_at                   TEXT NOT NULL,
    all_day                  INTEGER NOT NULL DEFAULT 0,
    timezone                 TEXT,
    organizer_email          TEXT,
    -- JSON [{email, name?, response_status?}]
    attendees                TEXT NOT NULL DEFAULT '[]',
    -- Extracted from description / hangoutLink / onlineMeeting
    meeting_url              TEXT,
    -- Resolved CRM linkages
    crm_activity_id          TEXT REFERENCES crm_activities(id) ON DELETE SET NULL,
    -- JSON array of crm_contacts.id matched from attendee emails
    crm_contact_ids          TEXT NOT NULL DEFAULT '[]',
    -- Provider raw payload for debugging / future re-derivation
    raw                      TEXT,
    created_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(calendar_account_id, external_event_id)
);

CREATE INDEX IF NOT EXISTS idx_cal_events_org_start
    ON calendar_events(organization_id, start_at);
CREATE INDEX IF NOT EXISTS idx_cal_events_account
    ON calendar_events(calendar_account_id);
CREATE INDEX IF NOT EXISTS idx_cal_events_activity
    ON calendar_events(crm_activity_id) WHERE crm_activity_id IS NOT NULL;

-- Sync cursors per account.
-- Google: nextSyncToken from /events.list
-- Outlook: @odata.deltaLink from /me/calendarview/delta
CREATE TABLE IF NOT EXISTS calendar_sync_cursor (
    calendar_account_id      TEXT PRIMARY KEY NOT NULL REFERENCES integration_connections(id) ON DELETE CASCADE,
    cursor                   TEXT,
    last_full_sync_at        TEXT,
    last_delta_sync_at       TEXT,
    last_error               TEXT,
    updated_at               TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
