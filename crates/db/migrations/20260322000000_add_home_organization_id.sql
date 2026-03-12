-- Migration: Add home_organization_id to users table
-- Description: Formalizes the home_organization_id column on the users table.
--   This column was previously applied manually via ALTER TABLE in some environments.
--   Adding it as a proper migration ensures consistency across all deployments.

ALTER TABLE users ADD COLUMN home_organization_id BLOB REFERENCES organizations(id);
