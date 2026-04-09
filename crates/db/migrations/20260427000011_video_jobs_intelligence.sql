-- Phase 1: segments metadata (topic structure for smart post-production)
ALTER TABLE video_jobs ADD COLUMN segments_json TEXT;

-- Phase 2: TTS timing data (word timestamps + computed cut points)
ALTER TABLE video_jobs ADD COLUMN word_timestamps_json TEXT;
ALTER TABLE video_jobs ADD COLUMN cut_points_json TEXT;
