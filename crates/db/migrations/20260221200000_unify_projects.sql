-- ============================================================================
-- Project Unification Migration
--
-- Consolidates fragmented projects (1 repo = 1 project) into unified products
-- with boards. Each logical product becomes ONE project with multiple boards.
-- Merged projects are soft-deleted after task/board migration.
--
-- BEFORE: 27 projects (most with 0 tasks)
-- AFTER:  ~14 unified projects with proper board structure
-- ============================================================================

-- ============================================================================
-- 1. POWERCLUB GLOBAL
--    Canonical: "Sovereign Stack" (05ABAAF5...) → rename to "Powerclub Global"
--    Absorb: ORCHA, pcg-admin, pcg-dashboard-web, pcg-tactical-transport,
--            powerclubglobal-website
-- ============================================================================

-- Rename the canonical project
UPDATE projects
SET name = 'Powerclub Global',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'05ABAAF5B2494D1CA980C27AA095F579';

-- Rename existing board from "Sovereign Stack Board" to "ORCHA / Dashboard"
UPDATE project_boards
SET name = 'ORCHA / Dashboard',
    slug = 'orcha-dashboard',
    description = 'Platform development — ORCHA architecture, PCG Dashboard, Sovereign Stack',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'1B49EFBE6D3549619274EB0FDDBE7911';

-- Create Website board
INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'05ABAAF5B2494D1CA980C27AA095F579',
    'Website',
    'website',
    'custom',
    'powerclubglobal.com — marketing site, content, SEO',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

-- Create Internal / Admin board
INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'05ABAAF5B2494D1CA980C27AA095F579',
    'Internal',
    'internal',
    'custom',
    'Private admin tooling — internal ops, employee-only',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

-- Create Tactical Transport board
INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'05ABAAF5B2494D1CA980C27AA095F579',
    'Tactical Transport',
    'tactical-transport',
    'custom',
    'PCG Tactical Transportation service',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

-- Move ORCHA tasks → Powerclub Global project, onto the ORCHA/Dashboard board
UPDATE tasks
SET project_id = X'05ABAAF5B2494D1CA980C27AA095F579',
    board_id = X'1B49EFBE6D3549619274EB0FDDBE7911',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE project_id = X'81609302450B4E259FD3AFF581EF7649';

-- Merge ORCHA project_members into Powerclub Global (skip dupes)
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, permissions, granted_at)
SELECT randomblob(16),
       X'05ABAAF5B2494D1CA980C27AA095F579',
       pm.user_id,
       pm.role,
       pm.permissions,
       strftime('%Y-%m-%d %H:%M:%f', 'now')
FROM project_members pm
WHERE pm.project_id = X'81609302450B4E259FD3AFF581EF7649'
  AND pm.user_id NOT IN (
      SELECT user_id FROM project_members
      WHERE project_id = X'05ABAAF5B2494D1CA980C27AA095F579'
  );

-- Soft-delete merged PCG projects
UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'81609302450B4E259FD3AFF581EF7649'; -- ORCHA
UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'239DC254A5BE4AF48E4A5E3E788EA288'; -- pcg-admin
UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'E7A5D0A3D15441049D42FDF3EE230C1A'; -- pcg-dashboard-web
UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'AE604C05932E4C8F8F4FF6A2B85215C2'; -- pcg-tactical-transport
UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'CF314D4DA0B8463F8110BFDC79CEE5E5'; -- powerclubglobal-website


-- ============================================================================
-- 2. ALPHA PROTOCOL
--    Canonical: "Alpha-Protocol-web" (BE035EF6...) → rename
--    Absorb: alpha-protocol-web-nextjs
-- ============================================================================

UPDATE projects
SET name = 'Alpha Protocol',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'BE035EF65E1C4FCDAAA7C746080F7886';

-- Create boards
INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'BE035EF65E1C4FCDAAA7C746080F7886',
    'Website',
    'website',
    'default',
    'Alpha Protocol web presence',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'BE035EF65E1C4FCDAAA7C746080F7886',
    'Next.js Rebuild',
    'nextjs-rebuild',
    'custom',
    'Next.js rewrite of the Alpha Protocol site',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'B83C8D945455483E8CE27DEC3DDDEAB0'; -- alpha-protocol-web-nextjs


-- ============================================================================
-- 3. SPECTRUM GALACTIC
--    Canonical: "spectrum-galactic-web" (8B8F9575...) → rename
--    Absorb: spectrum-admin
-- ============================================================================

UPDATE projects
SET name = 'Spectrum Galactic',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'8B8F9575E68D4741A081233A0FA940D0';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'8B8F9575E68D4741A081233A0FA940D0',
    'Website',
    'website',
    'default',
    'Spectrum Galactic public website',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'8B8F9575E68D4741A081233A0FA940D0',
    'Admin',
    'admin',
    'custom',
    'Spectrum Galactic admin panel',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'E22C48F729AC408C823255B340C10694'; -- spectrum-admin


-- ============================================================================
-- 4. PYTHIA AI
--    Canonical: "pythia-ai" (4B8B30A3...) → rename
--    Absorb: pythia-admin
-- ============================================================================

UPDATE projects
SET name = 'Pythia AI',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'4B8B30A3CEC448D7A9B18820749CCCE2';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'4B8B30A3CEC448D7A9B18820749CCCE2',
    'Core',
    'core',
    'default',
    'Pythia AI core platform',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'4B8B30A3CEC448D7A9B18820749CCCE2',
    'Admin',
    'admin',
    'custom',
    'Pythia AI admin panel',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'BEDB6A6E531545A1849FE402B92AC70D'; -- pythia-admin


-- ============================================================================
-- 5. VERITWIN
--    Canonical: "Veritwin" (72312B4C...) — has 9 tasks
--    Absorb: veritwin-website
--    Fix org: move to Veratwin org
-- ============================================================================

-- Rename existing board to "VLink Platform"
UPDATE project_boards
SET name = 'VLink Platform',
    slug = 'vlink-platform',
    description = 'VLink payment platform — merchant dashboard, checkout, settlements',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'0C004BAEE7F74BE9B67E426C44814DC5';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'72312B4C1C6148C79210EE3D8286B92A',
    'Website',
    'website',
    'custom',
    'Veritwin marketing website',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

-- Reassign to Veratwin org
UPDATE projects
SET organization_id = X'05050505050505050505050505050505',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'72312B4C1C6148C79210EE3D8286B92A';

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'8B17AC71DB9648AA9573F78BA5C10221'; -- veritwin-website


-- ============================================================================
-- 6. VIBE TOKEN
--    Canonical: "Vibe Token" (09C61720...) — has 4 tasks
--    Absorb: vibe-admin
-- ============================================================================

-- Rename existing board
UPDATE project_boards
SET name = 'Token Website',
    slug = 'token-website',
    description = 'VIBE token website — presale, landing pages',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'9CB2FA6F57AB42868EEC7F0971F14BAE';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'09C61720C92049F6B2C33EF2C6711085',
    'Admin',
    'admin',
    'custom',
    'VIBE Token admin panel and internal tooling',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'47E308DE5329449F8F6EBC8828EDEA32'; -- vibe-admin


-- ============================================================================
-- 7. YACHTMASTER
--    Canonical: "YachtMaster-App" (884BE248...) → rename
--    Absorb: YachtMaster-Web
-- ============================================================================

UPDATE projects
SET name = 'YachtMaster',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'884BE248B70E45619CB0DC0999DAA85F';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'884BE248B70E45619CB0DC0999DAA85F',
    'Mobile App',
    'mobile-app',
    'default',
    'YachtMaster mobile application',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'884BE248B70E45619CB0DC0999DAA85F',
    'Web',
    'web',
    'custom',
    'YachtMaster web application',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'AAB25F3447494CEFB451EEBBF6C4AC26'; -- YachtMaster-Web


-- ============================================================================
-- 8. WILLRISE UNLIMITED
--    Canonical: "willrise" (9732A335...) → rename
--    Absorb: skywalkerswings (Skyfox Swings derivative)
-- ============================================================================

UPDATE projects
SET name = 'WillRise Unlimited',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'9732A335374F4CA5A7EB29397BEBCCB6';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'9732A335374F4CA5A7EB29397BEBCCB6',
    'Main',
    'main',
    'default',
    'WillRise Unlimited core brand',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'9732A335374F4CA5A7EB29397BEBCCB6',
    'Skyfox Swings',
    'skyfox-swings',
    'custom',
    'Skyfox Swings — WillRise derivative brand',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'A84A1EDE4AF74359979EF8B363C54BB9'; -- skywalkerswings


-- ============================================================================
-- 9. PRIME HOSPITALITY
--    Canonical: "Prime" (C0C1C2C3...) → rename
--    Absorb: prime-website
-- ============================================================================

UPDATE projects
SET name = 'Prime Hospitality',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'C0C1C2C3C4C5C6C7C8C9CACBCCCDCECF';

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    randomblob(16),
    X'C0C1C2C3C4C5C6C7C8C9CACBCCCDCECF',
    'Website',
    'website',
    'default',
    'Prime Hospitality marketing website',
    strftime('%Y-%m-%d %H:%M:%f', 'now'),
    strftime('%Y-%m-%d %H:%M:%f', 'now')
);

UPDATE projects SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = X'9B763C80642043C8B8794210C9B760DA'; -- prime-website


-- ============================================================================
-- 10. ORG REASSIGNMENTS
-- ============================================================================

-- jungleverse → Jungleverse org
UPDATE projects
SET organization_id = X'04040404040404040404040404040404',
    updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
WHERE id = X'0851B9FDB4354B7486417BF675F2F53E';

-- resonance-website stays under PCG (shared with Sirak Studios via board_shares)
-- cable-com stays under PCG
-- virtual-world-web stays under PCG
-- sirak-studios already under Sirak Studios org


-- ============================================================================
-- 11. ADD DEFAULT BOARDS FOR STANDALONE PROJECTS THAT LACK THEM
--     (cable-com, jungleverse, resonance-website, virtual-world-web, sirak-studios)
-- ============================================================================

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
SELECT randomblob(16), p.id, p.name || ' Board', REPLACE(LOWER(p.name), ' ', '-') || '-board', 'default', NULL,
       strftime('%Y-%m-%d %H:%M:%f', 'now'), strftime('%Y-%m-%d %H:%M:%f', 'now')
FROM projects p
WHERE p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM project_boards pb WHERE pb.project_id = p.id)
  AND p.id NOT IN (
      -- Exclude the projects we just gave boards to above
      X'05ABAAF5B2494D1CA980C27AA095F579',
      X'BE035EF65E1C4FCDAAA7C746080F7886',
      X'8B8F9575E68D4741A081233A0FA940D0',
      X'4B8B30A3CEC448D7A9B18820749CCCE2',
      X'72312B4C1C6148C79210EE3D8286B92A',
      X'09C61720C92049F6B2C33EF2C6711085',
      X'884BE248B70E45619CB0DC0999DAA85F',
      X'9732A335374F4CA5A7EB29397BEBCCB6',
      X'C0C1C2C3C4C5C6C7C8C9CACBCCCDCECF'
  );


-- ============================================================================
-- 12. ENSURE SIDEBAR FILTERS OUT SOFT-DELETED PROJECTS
--     (This is a data-only migration; the query filter is added in sidebar.rs)
-- ============================================================================
-- See companion code change in crates/server/src/routes/sidebar.rs
