-- Add validation_errors column to workflow_output_staging
-- Stores JSON array of validation error strings, or NULL if record is valid
ALTER TABLE workflow_output_staging ADD COLUMN validation_errors TEXT;
