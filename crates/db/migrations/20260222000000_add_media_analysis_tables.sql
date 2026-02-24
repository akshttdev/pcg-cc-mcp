-- Per-file scene analysis results from SceneAnalysisEngine
CREATE TABLE IF NOT EXISTS media_file_scene_analyses (
    id TEXT PRIMARY KEY NOT NULL,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    batch_id TEXT NOT NULL REFERENCES media_batches(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    overall_energy REAL NOT NULL DEFAULT 0.0,
    peak_energy_timestamp REAL NOT NULL DEFAULT 0.0,
    dominant_content_type TEXT NOT NULL DEFAULT 'ambient',
    usable BOOLEAN NOT NULL DEFAULT 1,
    segments TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Per-file visual QC results from VisualQcEngine
CREATE TABLE IF NOT EXISTS media_file_visual_qcs (
    id TEXT PRIMARY KEY NOT NULL,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    batch_id TEXT NOT NULL REFERENCES media_batches(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    best_in_point REAL NOT NULL DEFAULT 0.0,
    best_composition_score REAL NOT NULL DEFAULT 0.0,
    qc_passed BOOLEAN NOT NULL DEFAULT 0,
    summary TEXT NOT NULL DEFAULT '',
    analyzed_frames TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Per-file beat analysis results from BeatAnalysisEngine
CREATE TABLE IF NOT EXISTS media_file_beat_analyses (
    id TEXT PRIMARY KEY NOT NULL,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    batch_id TEXT NOT NULL REFERENCES media_batches(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    bpm REAL NOT NULL DEFAULT 120.0,
    beat_interval REAL NOT NULL DEFAULT 0.5,
    total_beats INTEGER NOT NULL DEFAULT 0,
    beats_per_bar INTEGER NOT NULL DEFAULT 4,
    beats TEXT NOT NULL DEFAULT '[]',
    sections TEXT NOT NULL DEFAULT '[]',
    energy_curve TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Semantic content tags per media file
CREATE TABLE IF NOT EXISTS media_file_tags (
    id TEXT PRIMARY KEY NOT NULL,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    batch_id TEXT NOT NULL REFERENCES media_batches(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('heuristic','scene_analysis','visual_qc','beat_analysis','manual')),
    confidence REAL NOT NULL DEFAULT 1.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for lookup performance
CREATE INDEX idx_scene_analyses_file ON media_file_scene_analyses(media_file_id);
CREATE INDEX idx_scene_analyses_batch ON media_file_scene_analyses(batch_id);
CREATE INDEX idx_visual_qcs_file ON media_file_visual_qcs(media_file_id);
CREATE INDEX idx_visual_qcs_batch ON media_file_visual_qcs(batch_id);
CREATE INDEX idx_beat_analyses_file ON media_file_beat_analyses(media_file_id);
CREATE INDEX idx_beat_analyses_batch ON media_file_beat_analyses(batch_id);
CREATE INDEX idx_tags_file ON media_file_tags(media_file_id);
CREATE INDEX idx_tags_tag ON media_file_tags(tag);

-- Add analysis status columns to media_files
ALTER TABLE media_files ADD COLUMN scene_analysis_status TEXT DEFAULT NULL;
ALTER TABLE media_files ADD COLUMN visual_qc_status TEXT DEFAULT NULL;
ALTER TABLE media_files ADD COLUMN beat_analysis_status TEXT DEFAULT NULL;
