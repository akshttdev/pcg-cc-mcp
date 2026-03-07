-- Scheduled meetings: links proposals to calendar events with multi-channel invite dispatch

CREATE TABLE IF NOT EXISTS scheduled_meetings (
    id           BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    proposal_id  BLOB NOT NULL REFERENCES proposals(id) ON DELETE CASCADE,
    scheduled_at TEXT NOT NULL,
    duration_min INTEGER NOT NULL DEFAULT 60,
    location     TEXT,          -- URL (Zoom/Meet/Calendly) or physical address
    agenda       TEXT,
    channel      TEXT NOT NULL, -- primary channel used to send invites
    invite_status TEXT NOT NULL DEFAULT 'pending'
        CHECK(invite_status IN ('pending','sent','accepted','declined','no_response')),
    invite_sent_at TEXT,
    notes        TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE TABLE IF NOT EXISTS scheduled_meeting_invitees (
    id                    BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    scheduled_meeting_id  BLOB NOT NULL REFERENCES scheduled_meetings(id) ON DELETE CASCADE,
    person_id             BLOB NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    channel               TEXT NOT NULL,  -- actual channel used for this invitee
    channel_address       TEXT NOT NULL,  -- email / IG handle / phone number
    status                TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','sent','accepted','declined','no_response')),
    sent_at               TEXT,
    created_at            TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_scheduled_meetings_proposal ON scheduled_meetings(proposal_id);
CREATE INDEX IF NOT EXISTS idx_smi_meeting ON scheduled_meeting_invitees(scheduled_meeting_id);
CREATE INDEX IF NOT EXISTS idx_smi_person ON scheduled_meeting_invitees(person_id);
