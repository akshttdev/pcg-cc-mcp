-- Seed organization hierarchy: create orgs for each user and link projects
-- This migration creates organizations, assigns admin members, and links
-- existing projects to their respective organizations via owner_id.

-- ============================================================================
-- PART 1: Create users that may not exist yet
-- ============================================================================

-- travers (password: Travers123)
INSERT OR IGNORE INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
VALUES (
    X'd0d1d2d3d4d5d6d7d8d9dadbdcdddedf',
    'travers',
    'travers@powerclubglobal.com',
    'Travers',
    '$2b$12$r.nAooN5ov2dBokKgnlh3ewOsVSB6Q5z4UxQJk16l2wcHSEQFMxQO',
    0,
    1
);

-- jungle (password: Jungle123)
INSERT OR IGNORE INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
VALUES (
    X'e0e1e2e3e4e5e6e7e8e9eaebecedeeef',
    'jungle',
    'jungle@powerclubglobal.com',
    'Jungle',
    '$2b$12$r.nAooN5ov2dBokKgnlh3ewOsVSB6Q5z4UxQJk16l2wcHSEQFMxQO',
    0,
    1
);

-- andre (password: Andre123)
INSERT OR IGNORE INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
VALUES (
    X'f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff',
    'andre',
    'andre@powerclubglobal.com',
    'Andre',
    '$2b$12$r.nAooN5ov2dBokKgnlh3ewOsVSB6Q5z4UxQJk16l2wcHSEQFMxQO',
    0,
    1
);

-- ============================================================================
-- PART 2: Create organizations for each user
-- ============================================================================

-- Powerclub Global (owned by admin)
INSERT OR IGNORE INTO organizations (id, name, slug, description, owner_id, is_active)
SELECT
    X'01010101010101010101010101010101',
    'Powerclub Global',
    'powerclub-global',
    'Powerclub Global organization',
    u.id,
    1
FROM users u WHERE u.username = 'admin';

-- Sirak Studios (owned by sirak)
INSERT OR IGNORE INTO organizations (id, name, slug, description, owner_id, is_active)
SELECT
    X'02020202020202020202020202020202',
    'Sirak Studios',
    'sirak-studios',
    'Sirak Studios organization',
    u.id,
    1
FROM users u WHERE u.username = 'sirak';

-- Media Monsters (owned by travers)
INSERT OR IGNORE INTO organizations (id, name, slug, description, owner_id, is_active)
SELECT
    X'03030303030303030303030303030303',
    'Media Monsters',
    'media-monsters',
    'Media Monsters organization',
    u.id,
    1
FROM users u WHERE u.username = 'travers';

-- Jungleverse (owned by jungle)
INSERT OR IGNORE INTO organizations (id, name, slug, description, owner_id, is_active)
SELECT
    X'04040404040404040404040404040404',
    'Jungleverse',
    'jungleverse',
    'Jungleverse organization',
    u.id,
    1
FROM users u WHERE u.username = 'jungle';

-- Veratwin (owned by andre)
INSERT OR IGNORE INTO organizations (id, name, slug, description, owner_id, is_active)
SELECT
    X'05050505050505050505050505050505',
    'Veratwin',
    'veratwin',
    'Veratwin organization',
    u.id,
    1
FROM users u WHERE u.username = 'andre';

-- ============================================================================
-- PART 3: Add org owners as admin members of their organizations
-- ============================================================================

INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role)
SELECT randomblob(16), o.id, o.owner_id, 'admin'
FROM organizations o;

-- ============================================================================
-- PART 4: Set projects.owner_id from project_members where role = 'owner'
-- (Fixes the existing gap where owner_id is never populated)
-- ============================================================================

UPDATE projects
SET owner_id = (
    SELECT pm.user_id
    FROM project_members pm
    WHERE pm.project_id = projects.id AND pm.role = 'owner'
    LIMIT 1
)
WHERE owner_id IS NULL
  AND EXISTS (
    SELECT 1 FROM project_members pm
    WHERE pm.project_id = projects.id AND pm.role = 'owner'
);

-- ============================================================================
-- PART 5: Link projects to organizations via their owner's org
-- For each project with an owner_id, find the org where that user is the owner
-- ============================================================================

UPDATE projects
SET organization_id = (
    SELECT o.id
    FROM organizations o
    WHERE o.owner_id = projects.owner_id
    LIMIT 1
)
WHERE owner_id IS NOT NULL
  AND organization_id IS NULL
  AND EXISTS (
    SELECT 1 FROM organizations o WHERE o.owner_id = projects.owner_id
);
