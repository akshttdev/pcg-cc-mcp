-- AI Video Studio — Phase 1
-- Avatar profiles and video jobs for HeyGen talking head pipeline

CREATE TABLE IF NOT EXISTS avatar_profiles (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id         BLOB REFERENCES organizations(id),
    created_by              BLOB REFERENCES users(id),
    name                    TEXT NOT NULL,
    slug                    TEXT,
    -- Identity
    identity_doc            TEXT,
    style_notes             TEXT,
    -- HeyGen
    heygen_avatar_id        TEXT,
    heygen_avatar_type      TEXT NOT NULL DEFAULT 'instant',
    -- ElevenLabs
    elevenlabs_voice_id     TEXT NOT NULL DEFAULT 'ZtcPZrt9K4w8e1OB9M6w',
    voice_sample_url        TEXT,
    -- Visuals
    reference_image_url     TEXT,
    thumbnail_url           TEXT,
    default_background_url  TEXT,
    -- Lifecycle
    status                  TEXT NOT NULL DEFAULT 'pending',
    error_message           TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_avatar_profiles_org ON avatar_profiles(organization_id);
CREATE INDEX IF NOT EXISTS idx_avatar_profiles_slug ON avatar_profiles(slug);

CREATE TABLE IF NOT EXISTS video_jobs (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    avatar_profile_id       BLOB NOT NULL REFERENCES avatar_profiles(id),
    created_by              BLOB REFERENCES users(id),
    -- Script (inline for V1)
    script_text             TEXT NOT NULL,
    background_url          TEXT,
    -- Stage: TTS
    tts_status              TEXT NOT NULL DEFAULT 'pending',
    tts_audio_url           TEXT,
    -- Stage: HeyGen
    heygen_video_id         TEXT,
    heygen_status           TEXT,
    raw_video_url           TEXT,
    -- Final output
    final_video_url         TEXT,
    thumbnail_url           TEXT,
    duration_seconds        REAL,
    -- Overall
    status                  TEXT NOT NULL DEFAULT 'pending',
    error_message           TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_video_jobs_avatar ON video_jobs(avatar_profile_id);
CREATE INDEX IF NOT EXISTS idx_video_jobs_status  ON video_jobs(status);
