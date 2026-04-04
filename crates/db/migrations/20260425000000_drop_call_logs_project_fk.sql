-- Drop the broken FK constraint on call_logs.project_id.
-- projects.id is stored as TEXT but call_logs.project_id is BLOB;
-- SQLite FK checking compares them as different types so the constraint
-- always fails on INSERT. SQLite requires table recreation to drop a FK.

PRAGMA foreign_keys = OFF;

CREATE TABLE call_logs_new (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL,  -- FK removed: projects.id is TEXT, this column is BLOB

    call_sid TEXT NOT NULL UNIQUE,
    parent_call_sid TEXT,
    account_sid TEXT,

    from_number TEXT NOT NULL,
    to_number TEXT NOT NULL,
    from_formatted TEXT,
    to_formatted TEXT,
    caller_name TEXT,

    direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound', 'outbound-api', 'outbound-dial')),
    status TEXT NOT NULL CHECK (status IN (
        'queued', 'ringing', 'in-progress', 'completed', 'busy',
        'failed', 'no-answer', 'canceled'
    )),
    answered_by TEXT CHECK (answered_by IN ('human', 'machine', 'unknown')),

    start_time TEXT,
    end_time TEXT,
    duration_seconds INTEGER DEFAULT 0,

    recording_url TEXT,
    recording_sid TEXT,
    recording_duration INTEGER,

    transcription TEXT,
    transcription_status TEXT CHECK (transcription_status IN ('pending', 'completed', 'failed')),

    handled_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    conversation_id BLOB REFERENCES agent_conversations(id) ON DELETE SET NULL,
    summary TEXT,
    sentiment TEXT CHECK (sentiment IN ('positive', 'neutral', 'negative', 'unknown')),

    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE SET NULL,
    crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL,

    price REAL,
    price_unit TEXT DEFAULT 'USD',

    metadata TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

INSERT INTO call_logs_new SELECT * FROM call_logs;

DROP TABLE call_logs;
ALTER TABLE call_logs_new RENAME TO call_logs;

CREATE INDEX idx_call_logs_project ON call_logs(project_id);
CREATE INDEX idx_call_logs_call_sid ON call_logs(call_sid);
CREATE INDEX idx_call_logs_from ON call_logs(from_number);
CREATE INDEX idx_call_logs_to ON call_logs(to_number);
CREATE INDEX idx_call_logs_status ON call_logs(status);
CREATE INDEX idx_call_logs_direction ON call_logs(direction);
CREATE INDEX idx_call_logs_crm_contact ON call_logs(crm_contact_id);
CREATE INDEX idx_call_logs_start_time ON call_logs(start_time);

PRAGMA foreign_keys = ON;
