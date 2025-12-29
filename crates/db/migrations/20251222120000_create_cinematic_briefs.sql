PRAGMA foreign_keys = ON;

CREATE TABLE cinematic_briefs (
    id              BLOB PRIMARY KEY,
    project_id      BLOB NOT NULL,
    requester_id    TEXT NOT NULL,
    nora_session_id TEXT,
    title           TEXT NOT NULL,
    summary         TEXT DEFAULT '',
    script          TEXT DEFAULT '',
    asset_ids       TEXT NOT NULL DEFAULT '[]',
    duration_seconds INTEGER DEFAULT 30,
    fps             INTEGER DEFAULT 24,
    style_tags      TEXT NOT NULL DEFAULT '[]',
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending','planning','rendering','delivered','failed','cancelled')),
    llm_notes       TEXT NOT NULL DEFAULT '',
    render_payload  TEXT NOT NULL DEFAULT '{}',
    output_assets   TEXT NOT NULL DEFAULT '[]',
    metadata        TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_cinematic_briefs_project ON cinematic_briefs(project_id);
CREATE INDEX idx_cinematic_briefs_status ON cinematic_briefs(status);

CREATE TABLE cinematic_shot_plans (
    id               BLOB PRIMARY KEY,
    brief_id         BLOB NOT NULL,
    shot_index       INTEGER NOT NULL,
    title            TEXT NOT NULL DEFAULT '',
    prompt           TEXT NOT NULL DEFAULT '',
    negative_prompt  TEXT NOT NULL DEFAULT '',
    camera_notes     TEXT NOT NULL DEFAULT '',
    duration_seconds INTEGER DEFAULT 4,
    metadata         TEXT NOT NULL DEFAULT '{}',
    status           TEXT NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending','planned','rendering','completed','failed')),
    created_at       TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (brief_id) REFERENCES cinematic_briefs(id) ON DELETE CASCADE
);

CREATE INDEX idx_cinematic_shot_plans_brief ON cinematic_shot_plans(brief_id);
CREATE INDEX idx_cinematic_shot_plans_status ON cinematic_shot_plans(status);
