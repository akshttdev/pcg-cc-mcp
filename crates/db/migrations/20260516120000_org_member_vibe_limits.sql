-- Per-user-within-org monthly VIBE spending caps. Set by org admins to
-- prevent any single member from draining the org's vibe_balance pool.
--
-- NULL monthly_limit_vibe = uncapped within the org (the org's vibe_balance
-- is still the ultimate ceiling).
--
-- current_period_spent rolls over when period_start passes the start of a
-- new month — application-level reconciliation, no scheduled DB job.

CREATE TABLE IF NOT EXISTS org_member_vibe_limits (
    organization_id          TEXT NOT NULL,
    user_id                  BLOB NOT NULL,
    monthly_limit_vibe       INTEGER,
    current_period_spent     INTEGER NOT NULL DEFAULT 0,
    period_start             TEXT NOT NULL DEFAULT (datetime('now','start of month')),
    created_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    PRIMARY KEY (organization_id, user_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_org_member_vibe_limits_org
    ON org_member_vibe_limits(organization_id);
