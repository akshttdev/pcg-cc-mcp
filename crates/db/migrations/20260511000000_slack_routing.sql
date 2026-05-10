-- Slack notification routing.
--
-- Slack tokens themselves live in `integration_connections` with provider='slack'
-- (so the existing oauth_token_manager handles encryption + refresh).
--
-- This table maps an org's events → Slack channel(s). One row per (org, event_type),
-- so an org can fan out different events to different channels (#deals, #tasks, …).

CREATE TABLE IF NOT EXISTS slack_channel_routing (
    id                  TEXT PRIMARY KEY NOT NULL,
    organization_id     TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- Event type. Matches the dispatcher arms.
    event_type          TEXT NOT NULL CHECK (event_type IN (
        'deal_stage_changed',
        'deal_won',
        'deal_lost',
        'proposal_approved',
        'proposal_rejected',
        'task_assigned',
        'task_completed',
        'invoice_paid',
        'agent_escalation'
    )),
    -- Slack channel id (e.g. C0123456789). The display name is cached here so
    -- the UI can render without a Slack round-trip on every page load.
    channel_id          TEXT NOT NULL,
    channel_name        TEXT,
    -- Disable a route without deleting it (e.g. paused #deals notifications).
    enabled             INTEGER NOT NULL DEFAULT 1,
    -- For per-org/per-event tweaks: prefix tags, mention-on-failure user ids, …
    options             TEXT NOT NULL DEFAULT '{}',
    created_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id, event_type, channel_id)
);

CREATE INDEX IF NOT EXISTS idx_slack_routing_org_event
    ON slack_channel_routing(organization_id, event_type);

-- Per-message dispatch log. Useful for "did Slack get this?" debugging and
-- for at-most-once retries (we record the Slack `ts` once a post succeeds).
CREATE TABLE IF NOT EXISTS slack_dispatch_log (
    id                  TEXT PRIMARY KEY NOT NULL,
    organization_id     TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    event_type          TEXT NOT NULL,
    channel_id          TEXT NOT NULL,
    -- Slack message timestamp returned by chat.postMessage (e.g. "1645551234.567890").
    -- NULL while in flight; populated on success.
    slack_ts            TEXT,
    success             INTEGER NOT NULL DEFAULT 0,
    error               TEXT,
    payload_summary     TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_slack_dispatch_org
    ON slack_dispatch_log(organization_id, created_at DESC);
