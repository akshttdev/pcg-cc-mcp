-- Migration: Add Social Platform Tables
-- Implements social account management, post scheduling, and engagement tracking

-- ============================================================================
-- PART 1: Social Accounts (Platform Connections)
-- ============================================================================

CREATE TABLE IF NOT EXISTS social_accounts (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Platform info
    platform TEXT NOT NULL CHECK (platform IN (
        'instagram', 'linkedin', 'twitter', 'tiktok', 'youtube',
        'facebook', 'threads', 'bluesky', 'pinterest'
    )),
    account_type TEXT NOT NULL DEFAULT 'personal' CHECK (account_type IN (
        'personal', 'business', 'creator'
    )),

    -- Account identifiers
    platform_account_id TEXT NOT NULL,
    username TEXT,
    display_name TEXT,
    profile_url TEXT,
    avatar_url TEXT,

    -- OAuth tokens (encrypted in production)
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TEXT,

    -- Account metadata
    follower_count INTEGER,
    following_count INTEGER,
    post_count INTEGER,
    metadata TEXT, -- JSON for platform-specific data

    -- Status
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN (
        'active', 'inactive', 'expired', 'error', 'pending_auth'
    )),
    last_sync_at TEXT,
    last_error TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, platform, platform_account_id)
);

CREATE INDEX IF NOT EXISTS idx_social_accounts_project ON social_accounts(project_id);
CREATE INDEX IF NOT EXISTS idx_social_accounts_platform ON social_accounts(platform);
CREATE INDEX IF NOT EXISTS idx_social_accounts_status ON social_accounts(status);

-- ============================================================================
-- PART 2: Social Posts (Content & Scheduling)
-- ============================================================================

CREATE TABLE IF NOT EXISTS social_posts (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    social_account_id BLOB REFERENCES social_accounts(id) ON DELETE SET NULL,
    task_id BLOB REFERENCES tasks(id) ON DELETE SET NULL,

    -- Content
    content_type TEXT NOT NULL DEFAULT 'post' CHECK (content_type IN (
        'post', 'story', 'reel', 'carousel', 'thread', 'video', 'article'
    )),
    caption TEXT,
    content_blocks TEXT, -- JSON: structured content blocks
    media_urls TEXT, -- JSON: array of media URLs
    hashtags TEXT, -- JSON: array of hashtags
    mentions TEXT, -- JSON: array of @mentions

    -- Platform targeting
    platforms TEXT NOT NULL, -- JSON: array of platform IDs to publish to
    platform_specific TEXT, -- JSON: platform-specific adaptations

    -- Scheduling
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN (
        'draft', 'pending_review', 'approved', 'scheduled', 'publishing',
        'published', 'failed', 'cancelled'
    )),
    scheduled_for TEXT,
    published_at TEXT,

    -- Category & Queue (GoHighLevel style)
    category TEXT,
    queue_position INTEGER,
    is_evergreen INTEGER NOT NULL DEFAULT 0,
    recycle_after_days INTEGER,
    last_recycled_at TEXT,

    -- Agent tracking
    created_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    approved_by TEXT,
    approved_at TEXT,

    -- Platform response
    platform_post_id TEXT,
    platform_url TEXT,
    publish_error TEXT,

    -- Metrics (populated after publishing)
    impressions INTEGER DEFAULT 0,
    reach INTEGER DEFAULT 0,
    likes INTEGER DEFAULT 0,
    comments INTEGER DEFAULT 0,
    shares INTEGER DEFAULT 0,
    saves INTEGER DEFAULT 0,
    clicks INTEGER DEFAULT 0,
    engagement_rate REAL DEFAULT 0.0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_social_posts_project ON social_posts(project_id);
CREATE INDEX IF NOT EXISTS idx_social_posts_account ON social_posts(social_account_id);
CREATE INDEX IF NOT EXISTS idx_social_posts_task ON social_posts(task_id);
CREATE INDEX IF NOT EXISTS idx_social_posts_status ON social_posts(status);
CREATE INDEX IF NOT EXISTS idx_social_posts_scheduled ON social_posts(scheduled_for) WHERE status = 'scheduled';
CREATE INDEX IF NOT EXISTS idx_social_posts_category ON social_posts(category);
CREATE INDEX IF NOT EXISTS idx_social_posts_evergreen ON social_posts(is_evergreen) WHERE is_evergreen = 1;

-- ============================================================================
-- PART 3: Social Mentions (Inbox & Engagement)
-- ============================================================================

CREATE TABLE IF NOT EXISTS social_mentions (
    id BLOB PRIMARY KEY,
    social_account_id BLOB NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Source info
    mention_type TEXT NOT NULL CHECK (mention_type IN (
        'comment', 'mention', 'dm', 'reply', 'quote', 'tag', 'review'
    )),
    platform TEXT NOT NULL,
    platform_mention_id TEXT NOT NULL,

    -- Author info
    author_username TEXT,
    author_display_name TEXT,
    author_avatar_url TEXT,
    author_follower_count INTEGER,
    author_is_verified INTEGER DEFAULT 0,

    -- Content
    content TEXT,
    media_urls TEXT, -- JSON
    parent_post_id BLOB REFERENCES social_posts(id) ON DELETE SET NULL,
    parent_platform_id TEXT,

    -- Status
    status TEXT NOT NULL DEFAULT 'unread' CHECK (status IN (
        'unread', 'read', 'replied', 'archived', 'flagged'
    )),
    sentiment TEXT CHECK (sentiment IN ('positive', 'neutral', 'negative', 'unknown')),
    priority TEXT DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'urgent')),

    -- Response tracking
    replied_at TEXT,
    replied_by TEXT, -- user_id or agent_id
    reply_content TEXT,

    -- Agent handling
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    auto_response_sent INTEGER DEFAULT 0,

    received_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(social_account_id, platform_mention_id)
);

CREATE INDEX IF NOT EXISTS idx_social_mentions_account ON social_mentions(social_account_id);
CREATE INDEX IF NOT EXISTS idx_social_mentions_project ON social_mentions(project_id);
CREATE INDEX IF NOT EXISTS idx_social_mentions_status ON social_mentions(status);
CREATE INDEX IF NOT EXISTS idx_social_mentions_type ON social_mentions(mention_type);
CREATE INDEX IF NOT EXISTS idx_social_mentions_priority ON social_mentions(priority) WHERE priority IN ('high', 'urgent');
CREATE INDEX IF NOT EXISTS idx_social_mentions_received ON social_mentions(received_at);

-- ============================================================================
-- PART 4: Category Queues (GoHighLevel-style scheduling)
-- ============================================================================

CREATE TABLE IF NOT EXISTS category_queues (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    social_account_id BLOB REFERENCES social_accounts(id) ON DELETE CASCADE,

    -- Queue config
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    description TEXT,
    color TEXT, -- Hex color for UI

    -- Scheduling rules
    schedule_rule TEXT, -- JSON: { days: [], times: [], timezone: "" }
    posts_per_slot INTEGER DEFAULT 1,
    prioritize_new INTEGER DEFAULT 1, -- New content before recycled

    -- Rotation settings
    rotation_enabled INTEGER DEFAULT 0,
    shuffle_on_recycle INTEGER DEFAULT 0,
    min_days_between_repeats INTEGER DEFAULT 30,

    -- Stats
    total_posts INTEGER DEFAULT 0,
    posts_published INTEGER DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, social_account_id, category)
);

CREATE INDEX IF NOT EXISTS idx_category_queues_project ON category_queues(project_id);
CREATE INDEX IF NOT EXISTS idx_category_queues_account ON category_queues(social_account_id);
CREATE INDEX IF NOT EXISTS idx_category_queues_category ON category_queues(category);

-- ============================================================================
-- PART 5: Content Templates
-- ============================================================================

CREATE TABLE IF NOT EXISTS content_templates (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Template info
    name TEXT NOT NULL,
    description TEXT,
    template_type TEXT NOT NULL CHECK (template_type IN (
        'caption', 'hook', 'cta', 'carousel', 'thread', 'full_post'
    )),

    -- Content
    content_blocks TEXT NOT NULL, -- JSON: array of content blocks
    variables TEXT, -- JSON: { name: type } for dynamic insertion

    -- Categorization
    categories TEXT, -- JSON: applicable categories
    platforms TEXT, -- JSON: applicable platforms

    -- Usage tracking
    times_used INTEGER DEFAULT 0,
    last_used_at TEXT,

    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_content_templates_project ON content_templates(project_id);
CREATE INDEX IF NOT EXISTS idx_content_templates_type ON content_templates(template_type);
