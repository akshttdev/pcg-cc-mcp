-- Workflow Artifacts table for storing generated conference content
-- Stores articles, thumbnails, social graphics, and social posts

CREATE TABLE IF NOT EXISTS workflow_artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_id TEXT NOT NULL REFERENCES conference_workflows(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL CHECK (artifact_type IN ('article', 'thumbnail', 'social_graphic', 'social_post')),
    title TEXT NOT NULL,
    content TEXT,  -- For articles: markdown content; for posts: caption text
    file_url TEXT, -- For graphics: URL to generated image
    metadata TEXT, -- JSON: additional data like hashtags, dimensions, etc.
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Index for finding artifacts by workflow
CREATE INDEX IF NOT EXISTS idx_workflow_artifacts_workflow_id ON workflow_artifacts(workflow_id);

-- Index for finding artifacts by type
CREATE INDEX IF NOT EXISTS idx_workflow_artifacts_type ON workflow_artifacts(workflow_id, artifact_type);
