-- Per-org Stripe subscription state.
-- Created on first checkout, updated by Stripe webhooks (no client writes).
-- Free tier orgs get a row implicitly via `OrgSubscription::ensure_free`.

CREATE TABLE IF NOT EXISTS org_subscriptions (
    id                          TEXT PRIMARY KEY NOT NULL,
    organization_id             TEXT NOT NULL UNIQUE
                                  REFERENCES organizations(id) ON DELETE CASCADE,
    -- 'free' | 'starter' | 'pro' | 'scale' | 'enterprise'
    plan                        TEXT NOT NULL DEFAULT 'free'
                                  CHECK (plan IN ('free','starter','pro','scale','enterprise')),
    -- 'active' | 'trialing' | 'past_due' | 'canceled' | 'incomplete' | 'incomplete_expired' | 'unpaid'
    status                      TEXT NOT NULL DEFAULT 'active',
    -- Stripe identifiers (NULL for free-tier orgs that never went through checkout)
    stripe_customer_id          TEXT,
    stripe_subscription_id      TEXT,
    stripe_price_id             TEXT,
    -- Cached limits (refreshed on plan change so we don't lookup the table on every request)
    seat_limit                  INTEGER NOT NULL DEFAULT 2,
    action_limit                INTEGER NOT NULL DEFAULT 500,
    allow_overage               INTEGER NOT NULL DEFAULT 0,
    -- Current billing period (mirrors the Stripe subscription period)
    current_period_start        TEXT,
    current_period_end          TEXT,
    cancel_at_period_end        INTEGER NOT NULL DEFAULT 0,
    metadata                    TEXT NOT NULL DEFAULT '{}',
    created_at                  TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at                  TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_org_subs_customer ON org_subscriptions(stripe_customer_id);
CREATE INDEX IF NOT EXISTS idx_org_subs_subscription ON org_subscriptions(stripe_subscription_id);

-- Per-action metering. One row per (org, period_start) — we increment in
-- place rather than appending an event per action so it stays cheap.
CREATE TABLE IF NOT EXISTS usage_records (
    id                  TEXT PRIMARY KEY NOT NULL,
    organization_id     TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start        TEXT NOT NULL,    -- ISO date 'YYYY-MM-DD'
    period_end          TEXT NOT NULL,    -- ISO date
    action_count        INTEGER NOT NULL DEFAULT 0,
    -- Cumulative VIBE spend in this period (mirrors vibe_transactions but rolled up)
    vibe_spent          REAL NOT NULL DEFAULT 0.0,
    -- Whether we've already pushed an 80% warning notification
    warned_at_80pct     INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id, period_start)
);

CREATE INDEX IF NOT EXISTS idx_usage_records_period
    ON usage_records(organization_id, period_start);

-- Webhook event log. Stripe webhooks must be idempotent — we record the
-- event id, drop duplicates with INSERT OR IGNORE.
CREATE TABLE IF NOT EXISTS stripe_webhook_events (
    id                  TEXT PRIMARY KEY NOT NULL,    -- Stripe event id (`evt_...`)
    event_type          TEXT NOT NULL,
    received_at         TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    processed           INTEGER NOT NULL DEFAULT 0,
    last_error          TEXT,
    payload             TEXT NOT NULL                  -- raw JSON for debugging
);

CREATE INDEX IF NOT EXISTS idx_stripe_events_type ON stripe_webhook_events(event_type);
