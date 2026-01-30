-- Migration: Add Call Logs and SMS Messages Tables
-- For tracking Twilio phone calls and text messages

-- ============================================================================
-- PART 1: Call Logs (Phone calls via Twilio)
-- ============================================================================

CREATE TABLE IF NOT EXISTS call_logs (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Twilio identifiers
    call_sid TEXT NOT NULL UNIQUE,
    parent_call_sid TEXT,
    account_sid TEXT,

    -- Call parties
    from_number TEXT NOT NULL,
    to_number TEXT NOT NULL,
    from_formatted TEXT,
    to_formatted TEXT,
    caller_name TEXT,

    -- Call direction & status
    direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound', 'outbound-api', 'outbound-dial')),
    status TEXT NOT NULL CHECK (status IN (
        'queued', 'ringing', 'in-progress', 'completed', 'busy',
        'failed', 'no-answer', 'canceled'
    )),
    answered_by TEXT CHECK (answered_by IN ('human', 'machine', 'unknown')),

    -- Timing
    start_time TEXT,
    end_time TEXT,
    duration_seconds INTEGER DEFAULT 0,

    -- Recording
    recording_url TEXT,
    recording_sid TEXT,
    recording_duration INTEGER,

    -- Transcription (from speech-to-text)
    transcription TEXT,
    transcription_status TEXT CHECK (transcription_status IN ('pending', 'completed', 'failed')),

    -- AI/Nora interaction
    handled_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    conversation_id BLOB REFERENCES agent_conversations(id) ON DELETE SET NULL,
    summary TEXT,
    sentiment TEXT CHECK (sentiment IN ('positive', 'neutral', 'negative', 'unknown')),

    -- CRM linkage
    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE SET NULL,
    crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL,

    -- Cost tracking
    price REAL,
    price_unit TEXT DEFAULT 'USD',

    -- Metadata
    metadata TEXT, -- JSON for additional Twilio data

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_call_logs_project ON call_logs(project_id);
CREATE INDEX IF NOT EXISTS idx_call_logs_call_sid ON call_logs(call_sid);
CREATE INDEX IF NOT EXISTS idx_call_logs_from ON call_logs(from_number);
CREATE INDEX IF NOT EXISTS idx_call_logs_to ON call_logs(to_number);
CREATE INDEX IF NOT EXISTS idx_call_logs_status ON call_logs(status);
CREATE INDEX IF NOT EXISTS idx_call_logs_direction ON call_logs(direction);
CREATE INDEX IF NOT EXISTS idx_call_logs_crm_contact ON call_logs(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_call_logs_start_time ON call_logs(start_time);

-- ============================================================================
-- PART 2: SMS Messages (Text messages via Twilio)
-- ============================================================================

CREATE TABLE IF NOT EXISTS sms_messages (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Twilio identifiers
    message_sid TEXT NOT NULL UNIQUE,
    account_sid TEXT,
    messaging_service_sid TEXT,

    -- Message parties
    from_number TEXT NOT NULL,
    to_number TEXT NOT NULL,

    -- Content
    body TEXT NOT NULL,
    num_segments INTEGER DEFAULT 1,
    num_media INTEGER DEFAULT 0,
    media_urls TEXT, -- JSON array of media URLs

    -- Direction & status
    direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound', 'outbound-api', 'outbound-call', 'outbound-reply')),
    status TEXT NOT NULL CHECK (status IN (
        'accepted', 'queued', 'sending', 'sent', 'delivered',
        'undelivered', 'failed', 'receiving', 'received', 'read'
    )),
    error_code TEXT,
    error_message TEXT,

    -- AI/Nora interaction
    handled_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    conversation_id BLOB REFERENCES agent_conversations(id) ON DELETE SET NULL,
    auto_response TEXT, -- If Nora auto-replied
    sentiment TEXT CHECK (sentiment IN ('positive', 'neutral', 'negative', 'unknown')),

    -- CRM linkage
    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE SET NULL,
    crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL,

    -- Flags
    is_read INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0,
    needs_response INTEGER DEFAULT 0,
    responded_at TEXT,

    -- Cost tracking
    price REAL,
    price_unit TEXT DEFAULT 'USD',

    -- Timestamps
    date_sent TEXT,
    date_created TEXT,

    -- Metadata
    metadata TEXT, -- JSON for additional Twilio data

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_sms_messages_project ON sms_messages(project_id);
CREATE INDEX IF NOT EXISTS idx_sms_messages_message_sid ON sms_messages(message_sid);
CREATE INDEX IF NOT EXISTS idx_sms_messages_from ON sms_messages(from_number);
CREATE INDEX IF NOT EXISTS idx_sms_messages_to ON sms_messages(to_number);
CREATE INDEX IF NOT EXISTS idx_sms_messages_status ON sms_messages(status);
CREATE INDEX IF NOT EXISTS idx_sms_messages_direction ON sms_messages(direction);
CREATE INDEX IF NOT EXISTS idx_sms_messages_unread ON sms_messages(is_read) WHERE is_read = 0;
CREATE INDEX IF NOT EXISTS idx_sms_messages_crm_contact ON sms_messages(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_sms_messages_date_sent ON sms_messages(date_sent);
