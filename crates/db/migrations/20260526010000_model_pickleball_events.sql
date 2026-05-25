-- Model Pickleball — Phase 1: events, venues, sponsors
--
-- Backs the GET endpoints the model-pickleball-web Astro site reads at
-- build time when PUBLIC_USE_API_EVENTS=true:
--   GET /api/public/model-pickleball/events
--   GET /api/public/model-pickleball/sponsors
--
-- Until this migration ships the frontend uses src/lib/events.bootstrap.ts
-- as a hardcoded fallback (loader handles fetch failures gracefully).
--
-- Companion forms migration: 20260526000000_model_pickleball_forms.sql.
-- Tenancy: organization_id BLOB FK (decision recorded — matches forms PR).

PRAGMA foreign_keys = ON;

-- -----------------------------------------------------------------------------
-- venues
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mp_venues (
    id              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id BLOB NOT NULL REFERENCES organizations(id),
    name            TEXT NOT NULL,
    city            TEXT NOT NULL,
    address         TEXT,
    lat             REAL,
    lng             REAL,
    capacity        INTEGER,
    image           TEXT,
    blurb           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at      TEXT
);
CREATE INDEX IF NOT EXISTS idx_mp_venues_org  ON mp_venues(organization_id);
CREATE INDEX IF NOT EXISTS idx_mp_venues_city ON mp_venues(city);

-- -----------------------------------------------------------------------------
-- events — one row per tour stop (NYC, LA, Miami, ...)
-- slug is globally unique because it maps to the public /tour/[slug] route.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mp_events (
    id              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id BLOB NOT NULL REFERENCES organizations(id),
    slug            TEXT NOT NULL UNIQUE,
    city            TEXT NOT NULL,
    starts_at       TEXT NOT NULL,
    ends_at         TEXT,
    venue_id        BLOB REFERENCES mp_venues(id),
    status          TEXT NOT NULL DEFAULT 'draft'
                      CHECK (status IN ('draft','announced','live','completed','archived')),
    hero_image      TEXT,
    eventbrite_url  TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at      TEXT
);
CREATE INDEX IF NOT EXISTS idx_mp_events_org    ON mp_events(organization_id);
CREATE INDEX IF NOT EXISTS idx_mp_events_venue  ON mp_events(venue_id);
CREATE INDEX IF NOT EXISTS idx_mp_events_status ON mp_events(status, starts_at);

-- -----------------------------------------------------------------------------
-- sponsors — event_id NULL = season-wide sponsor.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mp_sponsors (
    id              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id BLOB NOT NULL REFERENCES organizations(id),
    event_id        BLOB REFERENCES mp_events(id),
    name            TEXT NOT NULL,
    logo_url        TEXT NOT NULL,
    tier            TEXT NOT NULL
                      CHECK (tier IN ('venue','equipment','brand','presenting','associate','activation')),
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at      TEXT
);
CREATE INDEX IF NOT EXISTS idx_mp_sponsors_org   ON mp_sponsors(organization_id);
CREATE INDEX IF NOT EXISTS idx_mp_sponsors_event ON mp_sponsors(event_id);
CREATE INDEX IF NOT EXISTS idx_mp_sponsors_tier  ON mp_sponsors(tier);
