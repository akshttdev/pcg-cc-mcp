-- Add missing slug column to projects table.
-- Referenced by user_onboarding.rs but never added via migration.
ALTER TABLE projects ADD COLUMN slug TEXT;
