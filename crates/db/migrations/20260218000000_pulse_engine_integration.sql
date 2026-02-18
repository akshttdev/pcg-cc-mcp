-- Pulse Engine integration tables
-- Stores content, sources, alerts, and tracking configs from the Pulse Engine
-- All tables scoped to project_id and organization_id

PRAGMA foreign_keys = ON;

-- Source configs mirrored from Pulse Engine
CREATE TABLE IF NOT EXISTS pulse_sources (
    id              TEXT PRIMARY KEY NOT NULL,
    project_id      BLOB NOT NULL,
    organization_id BLOB,
    source_id       TEXT NOT NULL,
    source_type     TEXT NOT NULL CHECK (source_type IN ('rss', 'twitter', 'youtube', 'reddit', 'web')),
    name            TEXT NOT NULL DEFAULT '',
    url             TEXT NOT NULL DEFAULT '',
    config          TEXT,  -- JSON
    enabled         INTEGER NOT NULL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'error', 'disabled')),
    last_fetch_at   TEXT,
    last_error      TEXT,
    collection_interval_secs INTEGER NOT NULL DEFAULT 300,
    category        TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pulse_sources_project_id ON pulse_sources(project_id);
CREATE INDEX IF NOT EXISTS idx_pulse_sources_org_id ON pulse_sources(organization_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_pulse_sources_project_source ON pulse_sources(project_id, source_id);

-- Collected content items stored in PCG
CREATE TABLE IF NOT EXISTS pulse_content_items (
    id                      TEXT PRIMARY KEY NOT NULL,
    project_id              BLOB NOT NULL,
    organization_id         BLOB,
    source_id               TEXT NOT NULL,
    source_type             TEXT NOT NULL,
    content_hash            TEXT NOT NULL,
    url                     TEXT NOT NULL DEFAULT '',
    title                   TEXT NOT NULL DEFAULT '',
    body                    TEXT,
    summary                 TEXT,
    author                  TEXT,
    published_at            TEXT,
    collected_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    relevance_score         REAL,
    extracted_entities      TEXT,  -- JSON
    pcg_status              TEXT NOT NULL DEFAULT 'new' CHECK (pcg_status IN ('new', 'enriched', 'alerted', 'reviewed', 'tasked', 'actioned', 'dismissed')),
    linked_task_id          BLOB,
    linked_entity_id        TEXT,
    linked_crm_contact_id   BLOB,
    enrichment_vibe_cost    REAL DEFAULT 0,
    created_at              TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (linked_task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    FOREIGN KEY (linked_crm_contact_id) REFERENCES crm_contacts(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_pulse_content_project_id ON pulse_content_items(project_id);
CREATE INDEX IF NOT EXISTS idx_pulse_content_org_id ON pulse_content_items(organization_id);
CREATE INDEX IF NOT EXISTS idx_pulse_content_status ON pulse_content_items(pcg_status);
CREATE INDEX IF NOT EXISTS idx_pulse_content_collected ON pulse_content_items(collected_at);
CREATE INDEX IF NOT EXISTS idx_pulse_content_hash ON pulse_content_items(content_hash);
CREATE INDEX IF NOT EXISTS idx_pulse_content_source ON pulse_content_items(source_id);
CREATE INDEX IF NOT EXISTS idx_pulse_content_relevance ON pulse_content_items(relevance_score);

-- Per-project alert rules
CREATE TABLE IF NOT EXISTS pulse_alert_rules (
    id              TEXT PRIMARY KEY NOT NULL,
    project_id      BLOB NOT NULL,
    organization_id BLOB,
    name            TEXT NOT NULL DEFAULT '',
    conditions      TEXT NOT NULL DEFAULT '[]',  -- JSON array
    actions         TEXT NOT NULL DEFAULT '[]',  -- JSON array
    priority        TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'critical')),
    enabled         INTEGER NOT NULL DEFAULT 1,
    trigger_count   INTEGER NOT NULL DEFAULT 0,
    last_triggered_at TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pulse_alert_rules_project ON pulse_alert_rules(project_id);

-- Triggered alert instances
CREATE TABLE IF NOT EXISTS pulse_alerts (
    id              TEXT PRIMARY KEY NOT NULL,
    project_id      BLOB NOT NULL,
    organization_id BLOB,
    rule_id         TEXT NOT NULL,
    content_item_id TEXT NOT NULL,
    priority        TEXT NOT NULL DEFAULT 'normal',
    acknowledged    INTEGER NOT NULL DEFAULT 0,
    auto_task_id    BLOB,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (rule_id) REFERENCES pulse_alert_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (content_item_id) REFERENCES pulse_content_items(id) ON DELETE CASCADE,
    FOREIGN KEY (auto_task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_pulse_alerts_project ON pulse_alerts(project_id);
CREATE INDEX IF NOT EXISTS idx_pulse_alerts_rule ON pulse_alerts(rule_id);
CREATE INDEX IF NOT EXISTS idx_pulse_alerts_content ON pulse_alerts(content_item_id);
CREATE INDEX IF NOT EXISTS idx_pulse_alerts_acknowledged ON pulse_alerts(acknowledged);

-- Collection run history
CREATE TABLE IF NOT EXISTS pulse_collection_runs (
    id              TEXT PRIMARY KEY NOT NULL,
    project_id      BLOB NOT NULL,
    organization_id BLOB,
    source_id       TEXT NOT NULL,
    started_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at    TEXT,
    items_collected INTEGER NOT NULL DEFAULT 0,
    items_new       INTEGER NOT NULL DEFAULT 0,
    items_duplicate INTEGER NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')),
    error           TEXT,
    vibe_cost       REAL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pulse_runs_project ON pulse_collection_runs(project_id);
CREATE INDEX IF NOT EXISTS idx_pulse_runs_source ON pulse_collection_runs(source_id);
CREATE INDEX IF NOT EXISTS idx_pulse_runs_status ON pulse_collection_runs(status);

-- Keywords/entities/LLM config per project
CREATE TABLE IF NOT EXISTS pulse_tracking_configs (
    id                  TEXT PRIMARY KEY NOT NULL,
    project_id          BLOB NOT NULL UNIQUE,
    organization_id     BLOB,
    keywords            TEXT NOT NULL DEFAULT '[]',  -- JSON array
    entities            TEXT NOT NULL DEFAULT '{}',  -- JSON object
    llm_enabled         INTEGER NOT NULL DEFAULT 0,
    llm_model           TEXT DEFAULT 'llama3.2',
    notification_config TEXT DEFAULT '{}',  -- JSON object
    created_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pulse_tracking_project ON pulse_tracking_configs(project_id);
