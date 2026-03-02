-- Fix empty string values in JSON columns that should be NULL
-- This prevents "EOF while parsing a value" errors when SQLx tries to decode Json<Value>

-- Fix tasks.custom_properties: empty strings should be NULL
UPDATE tasks
SET custom_properties = NULL
WHERE custom_properties = '';

-- Also fix any other JSON-like text columns that might have the same issue
UPDATE tasks
SET tags = NULL
WHERE tags = '';

UPDATE tasks
SET assigned_mcps = NULL
WHERE assigned_mcps = '';

UPDATE tasks
SET collaborators = NULL
WHERE collaborators = '';
