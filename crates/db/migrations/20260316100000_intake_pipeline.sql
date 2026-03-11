-- Intake Pipeline: call transcripts / email summaries → CRM association → business reports
-- Migration: 20260316100000

-- ── Call intake items ─────────────────────────────────────────────────────────
-- Raw inbound content: email-forwarded transcripts, manual uploads, linked call_logs.
CREATE TABLE IF NOT EXISTS call_intake_items (
    id                      BLOB NOT NULL PRIMARY KEY,
    source_type             TEXT NOT NULL DEFAULT 'email',
    -- 'email' | 'upload' | 'call_log' | 'email_message'
    source_ref              TEXT,
    -- email message-id, call_log id (hex), upload filename

    -- Raw content
    raw_content             TEXT,
    subject                 TEXT,
    from_email              TEXT,
    from_name               TEXT,
    call_date               TEXT,       -- extracted or inferred ISO date
    duration_seconds        INTEGER,

    -- Processing state
    status                  TEXT NOT NULL DEFAULT 'pending',
    -- 'pending' | 'processing' | 'processed' | 'failed'
    processed_at            TEXT,
    error                   TEXT,

    -- CRM associations (populated during processing)
    person_id               BLOB REFERENCES persons(id) ON DELETE SET NULL,
    company_id              BLOB REFERENCES companies(id) ON DELETE SET NULL,

    -- Extracted structure (JSON arrays / strings)
    extracted_participants  TEXT NOT NULL DEFAULT '[]',
    -- [{name, email, company, role}]
    extracted_topics        TEXT NOT NULL DEFAULT '[]',
    extracted_action_items  TEXT NOT NULL DEFAULT '[]',
    extracted_pain_points   TEXT NOT NULL DEFAULT '[]',
    extracted_sentiment     TEXT,       -- 'positive' | 'neutral' | 'negative'
    call_summary            TEXT,

    -- Report linkage
    report_id               BLOB REFERENCES business_reports(id) ON DELETE SET NULL,

    metadata                TEXT NOT NULL DEFAULT '{}',
    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- ── Business reports ─────────────────────────────────────────────────────────
-- Structured audit + opportunity analysis generated from call context + research.
CREATE TABLE IF NOT EXISTS business_reports (
    id                      BLOB NOT NULL PRIMARY KEY,
    person_id               BLOB REFERENCES persons(id) ON DELETE SET NULL,
    company_id              BLOB REFERENCES companies(id) ON DELETE SET NULL,

    report_type             TEXT NOT NULL DEFAULT 'business_audit',
    -- 'business_audit' | 'opportunity_analysis' | 'combined'
    title                   TEXT NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'draft',
    -- 'draft' | 'ready' | 'sent'

    -- Structured sections
    executive_summary       TEXT,
    company_overview        TEXT,
    pain_points             TEXT NOT NULL DEFAULT '[]',
    -- JSON [{point, severity}]
    opportunities           TEXT NOT NULL DEFAULT '[]',
    -- JSON [{title, description, priority, estimated_value}]
    recommended_services    TEXT NOT NULL DEFAULT '[]',
    -- JSON [{name, rationale, timeline}]
    next_steps              TEXT NOT NULL DEFAULT '[]',
    -- JSON [{action, owner, deadline}]
    full_report_md          TEXT,       -- full markdown narrative

    -- Source tracking
    intake_item_ids         TEXT NOT NULL DEFAULT '[]',   -- JSON [uuid_hex, ...]
    call_log_ids            TEXT NOT NULL DEFAULT '[]',   -- JSON [uuid_hex, ...]

    created_by              BLOB REFERENCES users(id),
    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_call_intake_status   ON call_intake_items(status);
CREATE INDEX IF NOT EXISTS idx_call_intake_person   ON call_intake_items(person_id);
CREATE INDEX IF NOT EXISTS idx_business_reports_person  ON business_reports(person_id);
CREATE INDEX IF NOT EXISTS idx_business_reports_company ON business_reports(company_id);
