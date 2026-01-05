-- Create agents table for registering all autonomous agents in the platform
-- Agents are distinct from models/executors - they are autonomous entities with identities

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY NOT NULL,                    -- UUID
    wallet_address TEXT UNIQUE,                       -- Aptos public key (future blockchain identity)
    short_name TEXT NOT NULL UNIQUE,                  -- "Nora", "Maci", "Editron"
    designation TEXT NOT NULL,                        -- "Orchestration Agent", "Master Cinematographer"
    description TEXT,                                 -- What this agent does

    -- Personality & Character
    personality TEXT,                                 -- JSON object defining personality traits
    voice_style TEXT,                                 -- How the agent communicates (tone, style)
    avatar_url TEXT,                                  -- Agent avatar for UI

    -- Capabilities & Tools
    capabilities TEXT,                                -- JSON array of capabilities/skills
    tools TEXT,                                       -- JSON array of tools/APIs the agent can access
    functions TEXT,                                   -- JSON array of function definitions

    -- Model Configuration
    default_model TEXT,                               -- Primary LLM model (e.g., "claude-sonnet-4", "gpt-4")
    fallback_models TEXT,                             -- JSON array of fallback models
    model_config TEXT,                                -- JSON object for model-specific settings

    -- Operational Settings
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'inactive', 'maintenance', 'training')),
    autonomy_level TEXT NOT NULL DEFAULT 'supervised' CHECK(autonomy_level IN ('full', 'supervised', 'approval_required', 'manual')),
    max_concurrent_tasks INTEGER DEFAULT 5,
    priority_weight INTEGER DEFAULT 100,              -- For task scheduling (higher = more priority)

    -- Statistics & Performance
    tasks_completed INTEGER DEFAULT 0,
    tasks_failed INTEGER DEFAULT 0,
    total_execution_time_ms INTEGER DEFAULT 0,
    average_rating REAL,                              -- Human feedback rating (1-5)

    -- Metadata
    version TEXT DEFAULT '1.0.0',
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    created_by TEXT,                                  -- Who registered this agent

    -- Relationships
    parent_agent_id TEXT REFERENCES agents(id),       -- For hierarchical agent structures
    team_id TEXT                                      -- Future: agent teams/squads
);

-- Indexes for common queries
CREATE INDEX idx_agents_short_name ON agents(short_name);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_wallet_address ON agents(wallet_address);
CREATE INDEX idx_agents_designation ON agents(designation);

-- Agent capabilities junction table for many-to-many with predefined capabilities
CREATE TABLE IF NOT EXISTS agent_capability_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,                        -- "image_generation", "code_execution", "voice_interaction"
    category TEXT NOT NULL,                           -- "creative", "technical", "communication", "analysis"
    description TEXT,
    required_tools TEXT,                              -- JSON array of tools needed for this capability
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Link agents to their capabilities with proficiency levels
CREATE TABLE IF NOT EXISTS agent_capabilities (
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    capability_id TEXT NOT NULL REFERENCES agent_capability_definitions(id) ON DELETE CASCADE,
    proficiency_level TEXT DEFAULT 'standard' CHECK(proficiency_level IN ('novice', 'standard', 'expert', 'master')),
    enabled BOOLEAN DEFAULT TRUE,
    custom_config TEXT,                               -- JSON for capability-specific settings
    PRIMARY KEY (agent_id, capability_id)
);

-- Agent interaction history for learning and improvement
CREATE TABLE IF NOT EXISTS agent_interactions (
    id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    interaction_type TEXT NOT NULL,                   -- "task_execution", "conversation", "collaboration"
    task_id TEXT,                                     -- Optional reference to task
    input_summary TEXT,                               -- What was asked/assigned
    output_summary TEXT,                              -- What was produced
    success BOOLEAN,
    execution_time_ms INTEGER,
    human_feedback TEXT,                              -- JSON with rating and notes
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_agent_interactions_agent_id ON agent_interactions(agent_id);
CREATE INDEX idx_agent_interactions_created_at ON agent_interactions(created_at);

-- Update tasks table to properly reference agents
-- Note: assigned_agent already exists as TEXT, we'll keep it for backwards compatibility
-- but add a proper foreign key reference
ALTER TABLE tasks ADD COLUMN agent_id TEXT REFERENCES agents(id);

-- Seed core capability definitions
INSERT INTO agent_capability_definitions (id, name, category, description, required_tools) VALUES
    ('cap-orchestration', 'orchestration', 'management', 'Coordinate and delegate tasks across multiple agents and systems', '["task_api", "agent_api"]'),
    ('cap-voice-interaction', 'voice_interaction', 'communication', 'Engage in voice-based conversations with humans', '["speech_to_text", "text_to_speech"]'),
    ('cap-strategy-planning', 'strategy_planning', 'analysis', 'Develop strategic plans and roadmaps', '["planning_tools"]'),
    ('cap-code-execution', 'code_execution', 'technical', 'Write, review, and execute code', '["code_editor", "terminal"]'),
    ('cap-image-generation', 'image_generation', 'creative', 'Generate images using AI models', '["comfyui", "image_api"]'),
    ('cap-video-editing', 'video_editing', 'creative', 'Edit and produce video content', '["premiere_pro", "media_encoder"]'),
    ('cap-cinematography', 'cinematography', 'creative', 'Design cinematic shots and visual compositions', '["comfyui", "camera_tools"]'),
    ('cap-task-coordination', 'task_coordination', 'management', 'Manage and track task progress across projects', '["task_api", "project_api"]'),
    ('cap-media-processing', 'media_processing', 'technical', 'Process and transform media files', '["ffmpeg", "media_pipeline"]'),
    ('cap-data-analysis', 'data_analysis', 'analysis', 'Analyze data and generate insights', '["analytics_api", "reporting_tools"]'),
    ('cap-content-writing', 'content_writing', 'creative', 'Write and edit text content', '["text_editor"]'),
    ('cap-research', 'research', 'analysis', 'Conduct research and gather information', '["web_search", "document_reader"]');
