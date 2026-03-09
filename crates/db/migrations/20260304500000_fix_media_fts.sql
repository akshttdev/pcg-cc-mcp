-- Fix media_assets FTS5: recreate as content-storing table so asset_id is readable.
-- The original contentless (content='') table cannot return stored column values,
-- which breaks the JOIN in search queries.

DROP TRIGGER IF EXISTS media_assets_fts_insert;
DROP TRIGGER IF EXISTS media_assets_fts_update;
DROP TRIGGER IF EXISTS media_assets_fts_delete;
DROP TABLE IF EXISTS media_assets_fts;

CREATE VIRTUAL TABLE media_assets_fts USING fts5(
    asset_id UNINDEXED,
    description, shot_type, tags,
    tokenize='porter unicode61'
);

-- Re-populate from existing data
INSERT INTO media_assets_fts(asset_id, description, shot_type, tags)
SELECT hex(id),
       COALESCE(ai_description,''),
       COALESCE(shot_type,''),
       COALESCE(scene_tags,'')
FROM media_assets;

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
