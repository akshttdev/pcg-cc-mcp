-- Add user1 account for development/testing
-- Password: user123

INSERT OR IGNORE INTO users (
    id,
    username,
    email,
    full_name,
    password_hash,
    is_admin,
    is_active
) VALUES (
    randomblob(16),
    'user1',
    'user1@example.com',
    'User One',
    '$2b$12$OxGJBPq.U0O0KTBiSb8i/.AsxXf7rP5BPKYYiyz3Hlb4fEjXWN1OW',
    0,
    1
);
