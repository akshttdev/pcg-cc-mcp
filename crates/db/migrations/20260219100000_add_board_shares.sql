-- Board sharing table for cross-org collaboration

CREATE TABLE IF NOT EXISTS board_shares (
    id                     BLOB PRIMARY KEY NOT NULL,
    board_id               BLOB NOT NULL,
    source_organization_id BLOB NOT NULL,
    target_organization_id BLOB NOT NULL,
    permission             TEXT NOT NULL DEFAULT 'editor'
                           CHECK(permission IN ('viewer', 'editor', 'admin')),
    share_type             TEXT NOT NULL DEFAULT 'collaboration'
                           CHECK(share_type IN ('joint_venture', 'collaboration', 'review')),
    shared_by              BLOB NOT NULL,
    is_active              INTEGER NOT NULL DEFAULT 1,
    created_at             TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at             TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (board_id) REFERENCES project_boards(id) ON DELETE CASCADE,
    FOREIGN KEY (source_organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (target_organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (shared_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(board_id, target_organization_id)
);

CREATE INDEX IF NOT EXISTS idx_board_shares_target_org ON board_shares(target_organization_id);
CREATE INDEX IF NOT EXISTS idx_board_shares_board ON board_shares(board_id);
CREATE INDEX IF NOT EXISTS idx_board_shares_active ON board_shares(is_active);
