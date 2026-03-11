-- Extend review_tokens to support artifact-based review links
-- (not just deliverable-based). deliverable_id becomes nullable;
-- artifact_id + artifact_video_url are added for task artifact reviews.

CREATE TABLE review_tokens_new (
    id              BLOB PRIMARY KEY,
    token           TEXT NOT NULL UNIQUE,
    deliverable_id  BLOB,              -- nullable: set for deliverable reviews
    artifact_id     BLOB,              -- nullable: set for artifact reviews
    artifact_video_url TEXT,           -- direct video URL for artifact reviews
    artifact_title  TEXT,              -- display title for artifact reviews
    created_by      BLOB,
    expires_at      TEXT,
    view_count      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

INSERT INTO review_tokens_new
    (id, token, deliverable_id, artifact_id, artifact_video_url, artifact_title,
     created_by, expires_at, view_count, is_active, created_at)
SELECT id, token, deliverable_id, NULL, NULL, NULL,
       created_by, expires_at, view_count, is_active, created_at
FROM review_tokens;

DROP TABLE review_tokens;
ALTER TABLE review_tokens_new RENAME TO review_tokens;
