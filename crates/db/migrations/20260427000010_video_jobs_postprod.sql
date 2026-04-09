-- Add post-production pipeline columns to video_jobs
ALTER TABLE video_jobs ADD COLUMN postprod_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE video_jobs ADD COLUMN postprod_video_path TEXT;
