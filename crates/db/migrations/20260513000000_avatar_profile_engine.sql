-- Avatar Profile Engine — Stage 1 of the Anchor Setup pipeline
-- Extends avatar_profiles with a multi-shot portrait set, character bible,
-- and a dedicated lifecycle status for profile generation.

ALTER TABLE avatar_profiles ADD COLUMN portrait_set   TEXT NOT NULL DEFAULT '[]';
ALTER TABLE avatar_profiles ADD COLUMN bible_json     TEXT;
ALTER TABLE avatar_profiles ADD COLUMN profile_status TEXT NOT NULL DEFAULT 'none';
ALTER TABLE avatar_profiles ADD COLUMN profile_error  TEXT;

CREATE INDEX IF NOT EXISTS idx_avatar_profiles_profile_status
    ON avatar_profiles(profile_status);
