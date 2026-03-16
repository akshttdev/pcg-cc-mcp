-- Organization Onboarding System
-- One-time onboarding per organization, tracks setup progress across 9 segments.

CREATE TABLE IF NOT EXISTS org_onboarding (
    id BLOB PRIMARY KEY,
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'completed', 'cancelled')),
    current_phase TEXT NOT NULL DEFAULT 'context_gathering',
    context_data TEXT,
    recommendations TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id)
);

CREATE TABLE IF NOT EXISTS org_onboarding_segments (
    id BLOB PRIMARY KEY,
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    onboarding_id BLOB NOT NULL REFERENCES org_onboarding(id) ON DELETE CASCADE,
    segment_type TEXT NOT NULL,
    name TEXT NOT NULL,
    assigned_agent_id BLOB,
    assigned_agent_name TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'needs_review', 'completed', 'skipped')),
    recommendations TEXT,
    user_decisions TEXT,
    order_index INTEGER NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(onboarding_id, segment_type)
);

-- UNIQUE(organization_id) on org_onboarding already creates an implicit index
CREATE INDEX IF NOT EXISTS idx_org_onboarding_status ON org_onboarding(status);
CREATE INDEX IF NOT EXISTS idx_org_onboarding_segments_org ON org_onboarding_segments(organization_id);
CREATE INDEX IF NOT EXISTS idx_org_onboarding_segments_onboarding ON org_onboarding_segments(onboarding_id);
CREATE INDEX IF NOT EXISTS idx_org_onboarding_segments_status ON org_onboarding_segments(status);
