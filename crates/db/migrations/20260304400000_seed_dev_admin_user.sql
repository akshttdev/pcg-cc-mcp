-- Seed a default admin user for local development
-- This migration only runs in dev environments where no users exist
--
-- Default credentials:
--   Username: admin
--   Password: admin123
--
-- IMPORTANT: This should NOT be used in production. The production database
-- should have users created through proper onboarding/invitation flows.

-- Static UUID for the dev admin user: 00000000-0000-0000-0000-000000000099
-- Password hash is bcrypt with cost 12 for "admin123"

INSERT INTO users (
    id,
    username,
    email,
    password_hash,
    full_name,
    avatar_url,
    is_active,
    is_admin,
    created_at,
    updated_at
)
SELECT
    X'00000000000000000000000000000099',
    'admin',
    'admin@localhost',
    -- bcrypt hash of "admin123" with cost 12 (generated via AuthService::hash_password)
    '$2b$12$Qd1dkIsKPSHe4w/g67ZoOew7UzOtWbsRV4wTyLTxwg/VsWhbrFtd.',
    'Local Admin',
    NULL,
    1,
    1,
    datetime('now', 'subsec'),
    datetime('now', 'subsec')
WHERE NOT EXISTS (
    SELECT 1 FROM users WHERE username = 'admin'
)
AND NOT EXISTS (
    -- Don't seed if there are any real users already (e.g., production)
    SELECT 1 FROM users LIMIT 1
);
