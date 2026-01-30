-- TopiClips - AI-generated artistic video clips from topology evolution
-- "Beeple Everydays from Topsi"

-- TopiClip sessions - main session tracking with status workflow
CREATE TABLE topiclip_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,

    -- Session metadata
    title TEXT NOT NULL,
    day_number INTEGER NOT NULL,  -- Beeple-style day counter for streak tracking
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('daily', 'event', 'manual')),

    -- Artistic interpretation
    primary_theme TEXT,           -- "growth", "struggle", "transformation", "connection", "loss"
    emotional_arc TEXT,           -- "triumphant", "melancholic", "tense", "peaceful", "chaotic"
    narrative_summary TEXT,       -- Human-readable story extracted from events
    artistic_prompt TEXT,         -- Full prompt for video generation
    negative_prompt TEXT,         -- Negative prompt for generation
    symbol_mapping TEXT,          -- JSON: mapping of events to symbols used

    -- Status workflow: pending -> analyzing -> interpreting -> rendering -> delivered
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending',      -- Waiting to start
        'analyzing',    -- Extracting story from topology changes
        'interpreting', -- LLM generating artistic prompts
        'rendering',    -- ComfyUI generating video
        'delivered',    -- Complete, video ready
        'failed',       -- Error during processing
        'cancelled'     -- Manually cancelled
    )),

    -- Render output
    cinematic_brief_id TEXT,      -- Reference to CinematicBrief if using cinematics pipeline
    output_asset_ids TEXT,        -- JSON array of ProjectAsset IDs
    duration_seconds INTEGER DEFAULT 4,

    -- Processing metadata
    llm_notes TEXT,               -- LLM reasoning/notes
    error_message TEXT,           -- Error details if failed
    events_analyzed INTEGER DEFAULT 0,  -- Count of topology events processed
    significance_score REAL,      -- Overall significance (0.0-1.0)

    -- Timestamps
    period_start TEXT,            -- Start of the period being visualized
    period_end TEXT,              -- End of the period being visualized
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    delivered_at TEXT,

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (cinematic_brief_id) REFERENCES cinematic_briefs(id) ON DELETE SET NULL
);

CREATE INDEX idx_topiclip_sessions_project ON topiclip_sessions(project_id);
CREATE INDEX idx_topiclip_sessions_status ON topiclip_sessions(status);
CREATE INDEX idx_topiclip_sessions_day ON topiclip_sessions(project_id, day_number);
CREATE INDEX idx_topiclip_sessions_created ON topiclip_sessions(created_at DESC);

-- TopiClip captured events - events captured for artistic interpretation
CREATE TABLE topiclip_captured_events (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,

    -- Event details (from TopologyChange)
    event_type TEXT NOT NULL,     -- NodeAdded, ClusterFormed, HealthImproved, etc.
    event_data TEXT NOT NULL,     -- JSON of full TopologyChange data

    -- Narrative interpretation
    narrative_role TEXT,          -- "protagonist", "catalyst", "victim", "beneficiary", "observer"
    significance_score REAL DEFAULT 0.5,  -- 0.0-1.0 importance rating

    -- Artistic mapping
    assigned_symbol TEXT,         -- The artistic symbol assigned (e.g., "Constellation", "Atlas")
    symbol_prompt TEXT,           -- The prompt fragment for this symbol

    -- Related entities
    affected_node_ids TEXT,       -- JSON array of topology node IDs
    affected_edge_ids TEXT,       -- JSON array of topology edge IDs

    occurred_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (session_id) REFERENCES topiclip_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_topiclip_events_session ON topiclip_captured_events(session_id);
CREATE INDEX idx_topiclip_events_type ON topiclip_captured_events(event_type);
CREATE INDEX idx_topiclip_events_significance ON topiclip_captured_events(significance_score DESC);

-- TopiClip daily schedule - Beeple-style day tracking with streak counter
CREATE TABLE topiclip_daily_schedule (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,

    -- Scheduling
    scheduled_time TEXT NOT NULL,      -- Time of day to generate (HH:MM format)
    timezone TEXT DEFAULT 'UTC',
    is_enabled INTEGER NOT NULL DEFAULT 1,

    -- Streak tracking
    current_streak INTEGER DEFAULT 0,   -- Consecutive days with clips
    longest_streak INTEGER DEFAULT 0,   -- All-time best streak
    total_clips_generated INTEGER DEFAULT 0,
    last_generation_date TEXT,          -- YYYY-MM-DD of last generation

    -- Trigger settings
    min_significance_threshold REAL DEFAULT 0.3,  -- Minimum significance to generate
    force_daily INTEGER DEFAULT 0,       -- Generate even if nothing significant happened

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id)  -- One schedule per project
);

CREATE INDEX idx_topiclip_schedule_project ON topiclip_daily_schedule(project_id);
CREATE INDEX idx_topiclip_schedule_enabled ON topiclip_daily_schedule(is_enabled);

-- TopiClip configuration - per-project configuration
CREATE TABLE topiclip_config (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,

    -- Style preferences
    default_style TEXT DEFAULT 'surreal',  -- "surreal", "abstract", "geometric", "organic", "cosmic"
    color_palette TEXT,            -- JSON array of preferred colors
    visual_density TEXT DEFAULT 'medium',  -- "minimal", "medium", "complex"
    motion_intensity TEXT DEFAULT 'moderate',  -- "subtle", "moderate", "dynamic"

    -- LLM settings
    llm_model TEXT DEFAULT 'claude-sonnet-4-20250514',
    interpretation_temperature REAL DEFAULT 0.8,  -- Higher = more creative

    -- Render settings
    output_resolution TEXT DEFAULT '768x432',
    output_fps INTEGER DEFAULT 8,
    output_format TEXT DEFAULT 'webp',  -- "webp", "mp4", "gif"

    -- Event detection settings
    significance_algorithm TEXT DEFAULT 'weighted',  -- "weighted", "simple", "ml"
    include_event_types TEXT,      -- JSON array of event types to include (null = all)
    exclude_event_types TEXT,      -- JSON array of event types to exclude

    -- Metadata
    custom_symbol_mappings TEXT,   -- JSON object of custom event->symbol mappings
    metadata TEXT,                 -- JSON for additional settings

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id)
);

CREATE INDEX idx_topiclip_config_project ON topiclip_config(project_id);

-- Default symbol mappings as a reference table
CREATE TABLE topiclip_symbol_library (
    id TEXT PRIMARY KEY NOT NULL,

    -- Symbol definition
    event_pattern TEXT NOT NULL,   -- Regex or event type pattern
    symbol_name TEXT NOT NULL,     -- Human-readable name
    symbol_description TEXT,       -- Description of the symbol

    -- Prompt templates
    prompt_template TEXT NOT NULL,  -- Template with {placeholders}
    negative_template TEXT,

    -- Categorization
    theme_affinity TEXT,           -- Which themes this symbol suits
    emotional_range TEXT,          -- JSON array of compatible emotions

    -- Visual hints
    suggested_colors TEXT,         -- JSON array of color suggestions
    motion_type TEXT,              -- "rising", "falling", "expanding", "contracting", "swirling"

    is_default INTEGER NOT NULL DEFAULT 1,  -- Whether this is a built-in symbol
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_topiclip_symbol_pattern ON topiclip_symbol_library(event_pattern);
CREATE INDEX idx_topiclip_symbol_theme ON topiclip_symbol_library(theme_affinity);

-- Insert default symbol mappings based on the plan
INSERT INTO topiclip_symbol_library (id, event_pattern, symbol_name, symbol_description, prompt_template, theme_affinity, emotional_range, motion_type) VALUES
    ('sym_cluster_formed', 'ClusterFormed', 'Constellation',
     'Agents coming together form a new constellation of collaboration',
     '{count} luminous figures rise from geometric pool, forms intertwining into brilliant constellation, ethereal glow, cosmic unity',
     'connection', '["triumphant", "peaceful", "hopeful"]', 'rising'),

    ('sym_bottleneck', 'bottleneck', 'Atlas',
     'A critical node bears the weight of the system',
     'Colossal figure strains beneath impossible pyramid of glowing cubes, cracks spreading through crystalline surface, weight of responsibility',
     'struggle', '["tense", "melancholic", "urgent"]', 'contracting'),

    ('sym_health_improved', 'HealthImproved', 'Metamorphosis',
     'System health improving, transformation and renewal',
     'Withered tree erupts into crystalline bloom, branches becoming pathways of light, renewal surging through geometric veins',
     'transformation', '["triumphant", "peaceful", "hopeful"]', 'expanding'),

    ('sym_route_created', 'RouteCreated', 'Golden Thread',
     'New pathways weaving through the topology',
     'Golden thread weaves through {count} floating monoliths, each awakening with inner light as the path connects them',
     'connection', '["peaceful", "hopeful", "harmonious"]', 'swirling'),

    ('sym_node_removed', 'NodeRemoved', 'Empty Gallery',
     'Nodes departing, leaving spaces in the topology',
     'Empty pedestals in infinite gallery, dissolving particle clouds drift toward horizon, memories fading into light',
     'loss', '["melancholic", "peaceful", "contemplative"]', 'falling'),

    ('sym_node_added', 'NodeAdded', 'Emergence',
     'New entities entering the topology',
     'Luminous form crystallizes from swirling motes of light, geometry solidifying into presence, birth of new potential',
     'growth', '["hopeful", "triumphant", "curious"]', 'rising'),

    ('sym_edge_degraded', 'EdgeStatusChanged:degraded', 'Fraying Bond',
     'Connections weakening between nodes',
     'Golden filaments between floating orbs begin to flicker, strands unraveling into scattered sparks, tension in the void',
     'struggle', '["tense", "melancholic", "uncertain"]', 'contracting'),

    ('sym_cluster_dissolved', 'ClusterDissolved', 'Supernova',
     'A cluster breaking apart, scattering its members',
     'Brilliant sphere fragments into countless shards of light, each shard carrying a piece of shared memory into the cosmos',
     'loss', '["melancholic", "chaotic", "bittersweet"]', 'expanding'),

    ('sym_route_completed', 'RouteCompleted', 'Arrival',
     'A journey through the topology reaching its destination',
     'Beam of light pierces through layered geometric planes, illuminating final destination in radiant achievement',
     'growth', '["triumphant", "peaceful", "satisfied"]', 'rising'),

    ('sym_invariant_violation', 'InvariantViolation', 'Fracture',
     'Rules of the topology being broken',
     'Cracks of crimson light spread across perfect crystalline surface, order giving way to chaos, warning pulses through the structure',
     'struggle', '["tense", "chaotic", "urgent"]', 'expanding');
