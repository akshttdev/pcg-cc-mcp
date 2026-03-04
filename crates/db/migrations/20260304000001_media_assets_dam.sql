CREATE TABLE media_assets (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL,
    batch_id BLOB,
    filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size_bytes INTEGER DEFAULT 0,
    mime_type TEXT DEFAULT 'video/mp4',
    duration_seconds REAL,
    width INTEGER, height INTEGER,
    ai_description TEXT,
    shot_type TEXT,
    energy_level REAL DEFAULT 0.5,
    motion_intensity REAL DEFAULT 0.5,
    dominant_colors TEXT DEFAULT '[]',
    scene_tags TEXT DEFAULT '[]',
    ai_confidence REAL DEFAULT 0.0,
    analysis_status TEXT DEFAULT 'pending',
    created_at TEXT DEFAULT (datetime('now','subsec')),
    updated_at TEXT DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_media_assets_project ON media_assets(project_id);
CREATE INDEX idx_media_assets_status  ON media_assets(analysis_status);

CREATE VIRTUAL TABLE media_assets_fts USING fts5(
    asset_id UNINDEXED,
    description, shot_type, tags,
    content='', tokenize='porter unicode61'
);
CREATE TRIGGER media_assets_fts_insert AFTER INSERT ON media_assets BEGIN
    INSERT INTO media_assets_fts(asset_id, description, shot_type, tags)
    VALUES (hex(NEW.id), COALESCE(NEW.ai_description,''), COALESCE(NEW.shot_type,''), COALESCE(NEW.scene_tags,''));
END;
CREATE TRIGGER media_assets_fts_update AFTER UPDATE ON media_assets BEGIN
    DELETE FROM media_assets_fts WHERE asset_id = hex(OLD.id);
    INSERT INTO media_assets_fts(asset_id, description, shot_type, tags)
    VALUES (hex(NEW.id), COALESCE(NEW.ai_description,''), COALESCE(NEW.shot_type,''), COALESCE(NEW.scene_tags,''));
END;
CREATE TRIGGER media_assets_fts_delete AFTER DELETE ON media_assets BEGIN
    DELETE FROM media_assets_fts WHERE asset_id = hex(OLD.id);
END;
