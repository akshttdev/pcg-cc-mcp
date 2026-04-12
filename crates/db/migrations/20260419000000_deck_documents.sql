-- Lux Creator Studio — Phase 0 foundation
-- Structured deck authoring: DeckDocument ↔ slides ↔ elements tree,
-- revision history, and Lux-authored suggestions awaiting operator
-- accept/reject. Asset request pipeline lands in Phase 7.

-- ─── 1. deck_documents ──────────────────────────────────────────────────────
-- One row per authored deck. `canvas_json` holds { width, height, unit, dpi }.
-- `version` is a monotonic revision counter bumped on every save; client
-- optimistic-concurrency compares against this.

CREATE TABLE IF NOT EXISTS deck_documents (
    id                   TEXT PRIMARY KEY,
    deal_id              TEXT NOT NULL REFERENCES crm_deals(id) ON DELETE CASCADE,
    version              INTEGER NOT NULL DEFAULT 1,
    brand_token_version  TEXT,
    canvas_json          TEXT NOT NULL,
    last_edited_by       TEXT NOT NULL DEFAULT 'system'
                         CHECK (last_edited_by IN ('lux', 'operator', 'system')),
    created_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_deck_documents_deal
    ON deck_documents(deal_id);

-- ─── 2. deck_slides ─────────────────────────────────────────────────────────
-- One row per slide in a deck. `elements_json` is an opaque JSON tree of
-- SlideElement nodes (Text / Image / Shape / Group); normalized storage is
-- deferred until querying by element becomes a hot path.

CREATE TABLE IF NOT EXISTS deck_slides (
    id                   TEXT PRIMARY KEY,
    deck_id              TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
    slide_index          INTEGER NOT NULL,
    name                 TEXT,
    layout_hint          TEXT
                         CHECK (layout_hint IS NULL OR
                                layout_hint IN ('title', 'content', 'split', 'cover', 'closer')),
    background_json      TEXT NOT NULL,
    elements_json        TEXT NOT NULL,
    notes                TEXT,
    origin               TEXT NOT NULL DEFAULT 'lux'
                         CHECK (origin IN ('lux', 'operator')),
    locked               INTEGER NOT NULL DEFAULT 0 CHECK (locked IN (0, 1)),
    created_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (deck_id, slide_index)
);

CREATE INDEX IF NOT EXISTS idx_deck_slides_deck_index
    ON deck_slides(deck_id, slide_index);

-- ─── 3. deck_revisions ──────────────────────────────────────────────────────
-- Immutable snapshot of the full deck at each save point. Enables time
-- travel, diff inspection, and rollback. `snapshot_json` holds the entire
-- serialized DeckDocument (slides + elements tree).

CREATE TABLE IF NOT EXISTS deck_revisions (
    id                   TEXT PRIMARY KEY,
    deck_id              TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
    version              INTEGER NOT NULL,
    author               TEXT NOT NULL,
    snapshot_json        TEXT NOT NULL,
    diff_summary         TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (deck_id, version)
);

CREATE INDEX IF NOT EXISTS idx_deck_revisions_deck_version
    ON deck_revisions(deck_id, version DESC);

-- ─── 4. deck_suggestions ────────────────────────────────────────────────────
-- Google-Docs-style proposals from Lux against existing decks. Operators
-- accept or reject; accepted ops apply the payload and bump deck.version.
-- Suggestions against a locked_by=operator element are rejected at enqueue
-- time (enforced in service layer, not schema).

CREATE TABLE IF NOT EXISTS deck_suggestions (
    id                   TEXT PRIMARY KEY,
    deck_id              TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
    target_slide_id      TEXT,
    target_element_id    TEXT,
    op                   TEXT NOT NULL
                         CHECK (op IN ('replace_text', 'restyle', 'reposition',
                                       'insert_element', 'delete_element', 'reorder_slide')),
    payload_json         TEXT NOT NULL,
    rationale            TEXT,
    status               TEXT NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending', 'accepted', 'rejected', 'superseded')),
    run_id               TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    resolved_at          TEXT,
    resolved_by          TEXT
);

CREATE INDEX IF NOT EXISTS idx_deck_suggestions_deck_status
    ON deck_suggestions(deck_id, status);

CREATE INDEX IF NOT EXISTS idx_deck_suggestions_run
    ON deck_suggestions(run_id)
    WHERE run_id IS NOT NULL;
