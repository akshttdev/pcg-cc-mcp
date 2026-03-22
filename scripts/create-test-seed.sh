#!/usr/bin/env bash
# create-test-seed.sh — Creates a minimal test seed database for E2E testing.
#
# Usage: ./scripts/create-test-seed.sh
#
# Output: dev_assets_seed/test-seed.sqlite
#
# This seed contains:
#   - Full schema (all migrations applied)
#   - Admin user (admin/admin123)
#   - Default org (02020202-0202-0202-0202-020202020202 / Sirak Studios)
#   - Default CRM pipeline with 9 stages (dealflow v2)
#   - NO entity data (deals, contacts, companies — tests create these via API)
#
# Tests should use this seed by:
#   1. Copying test-seed.sqlite → dev_assets/test-db.sqlite before test run
#   2. Starting backend with DATABASE_URL=sqlite:dev_assets/test-db.sqlite

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SEED_DIR="$PROJECT_ROOT/dev_assets_seed"
OUTPUT="$SEED_DIR/test-seed.sqlite"
TEMP_DB="/tmp/test-seed-$$-temp.sqlite"

echo "Creating test seed database..."

# Clean up any previous temp file
rm -f "$TEMP_DB" "$TEMP_DB-shm" "$TEMP_DB-wal"

# 1. Run all migrations on a fresh DB
echo "  Running migrations..."
cd "$PROJECT_ROOT"
# Copy the running dev DB (which has all migrations applied including BLOB→TEXT).
# The seed DB may be behind on migrations, and data-only migrations (20260406 BLOB→TEXT)
# fail on fresh schemas. The dev DB is always current.
if [ -f "$PROJECT_ROOT/dev_assets/db.sqlite" ]; then
  echo "  Using dev_assets/db.sqlite as base (all migrations applied)..."
  cp "$PROJECT_ROOT/dev_assets/db.sqlite" "$TEMP_DB"
elif [ -f "$SEED_DIR/db.sqlite" ]; then
  echo "  Using dev_assets_seed/db.sqlite as base..."
  cp "$SEED_DIR/db.sqlite" "$TEMP_DB"
else
  echo "ERROR: No source database found. Run 'flox activate' first to seed dev_assets/."
  exit 1
fi

# Wipe all entity data (keep schema + _sqlx_migrations)
# Uses individual statements so missing tables don't block others
echo "  Wiping entity data..."
for table in tasks crm_deals crm_contacts crm_activities crm_pipeline_stages \
  crm_pipelines companies persons projects deliverables business_reports \
  deal_transcripts organization_members organizations users execution_artifacts \
  data_sources workflow_definitions workflow_triggers trigger_executions \
  company_brand_profiles vibe_transactions notifications; do
  sqlite3 "$TEMP_DB" "DELETE FROM $table;" 2>/dev/null || true
done
sqlite3 "$TEMP_DB" "VACUUM;"

# 1b. Patch schema gaps — columns that exist in code but may be missing
#     from older seed DBs that didn't get the full BLOB→TEXT rebuild.
echo "  Patching schema gaps..."
sqlite3 "$TEMP_DB" "ALTER TABLE projects ADD COLUMN slug TEXT;" 2>/dev/null || true
sqlite3 "$TEMP_DB" "ALTER TABLE sessions ADD COLUMN token_hash TEXT;" 2>/dev/null || true
sqlite3 "$TEMP_DB" "ALTER TABLE sessions ADD COLUMN last_used_at TEXT;" 2>/dev/null || true
sqlite3 "$TEMP_DB" "CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash);" 2>/dev/null || true

# 2. Insert minimal test fixtures
echo "  Inserting test fixtures..."
sqlite3 "$TEMP_DB" <<'SQL'
-- All TEXT UUIDs. Auth code now uses TEXT binds (no more bind_uuid_blob).
-- DbUuid decodes both BLOB and TEXT transparently.

-- Admin user
INSERT INTO users (id, username, email, password_hash, full_name, is_admin, is_active, created_at, updated_at)
VALUES (
  '07192211-ae5c-f20b-42bd-546422d71a23',
  'admin',
  'admin@test.local',
  '$2b$12$MU1G/VfH8R8pFa/aPuGM/uduO3HrHafI.m9srBUlJR9AKd8EGBrIa',
  'Test Administrator',
  1, 1,
  datetime('now'), datetime('now')
);

-- Sirak Studios organization
INSERT INTO organizations (id, name, slug, owner_id, is_active, created_at, updated_at)
VALUES (
  '02020202-0202-0202-0202-020202020202',
  'Sirak Studios',
  'sirak-studios',
  '07192211-ae5c-f20b-42bd-546422d71a23',
  1,
  datetime('now'), datetime('now')
);

-- Set admin's home org
UPDATE users SET home_organization_id = '02020202-0202-0202-0202-020202020202'
WHERE username = 'admin';

-- Organization membership (Sirak Studios)
INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role)
VALUES (
  'mem-admin-sirak-001',
  '02020202-0202-0202-0202-020202020202',
  '07192211-ae5c-f20b-42bd-546422d71a23',
  'admin'
);

-- Powerclub Global organization
INSERT INTO organizations (id, name, slug, owner_id, is_active, created_at, updated_at)
VALUES (
  '01010101-0101-0101-0101-010101010101',
  'Powerclub Global',
  'powerclub-global',
  '07192211-ae5c-f20b-42bd-546422d71a23',
  1,
  datetime('now'), datetime('now')
);

-- Organization membership (Powerclub Global)
INSERT OR IGNORE INTO organization_members (id, organization_id, user_id, role)
VALUES (
  'mem-admin-pcg-001',
  '01010101-0101-0101-0101-010101010101',
  '07192211-ae5c-f20b-42bd-546422d71a23',
  'admin'
);

-- Default CRM pipeline (dealflow v2 stages)
INSERT INTO crm_pipelines (id, organization_id, name, pipeline_type, is_active, is_default, created_at, updated_at)
VALUES (
  '138ff8ec-6d65-493e-b6a9-0f9fef409968',
  '02020202-0202-0202-0202-020202020202',
  'Sirak Studios Acquisition',
  'sales',
  1, 1,
  datetime('now'), datetime('now')
);

-- Pipeline stages (matching dealflow v2 migration)
INSERT INTO crm_pipeline_stages (id, pipeline_id, name, stage_type, color, position, is_closed, is_won, probability, created_at, updated_at) VALUES
  ('a1000001-0000-0000-0000-000000000001', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Lead',              'lead',              '#94A3B8', 0, 0, 0, 10, datetime('now'), datetime('now')),
  ('a1000002-0000-0000-0000-000000000002', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Intel',             'intel',             '#6366F1', 1, 0, 0, 20, datetime('now'), datetime('now')),
  ('a1000003-0000-0000-0000-000000000003', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Business Analysis', 'business_analysis', '#8B5CF6', 2, 0, 0, 35, datetime('now'), datetime('now')),
  ('a1000004-0000-0000-0000-000000000004', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Proposal',          'proposal',          '#EC4899', 3, 0, 0, 50, datetime('now'), datetime('now')),
  ('a1000005-0000-0000-0000-000000000005', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Polish',            'polish',            '#F59E0B', 4, 0, 0, 65, datetime('now'), datetime('now')),
  ('a1000006-0000-0000-0000-000000000006', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Invoice',           'invoice',           '#14B8A6', 5, 0, 0, 80, datetime('now'), datetime('now')),
  ('a1000007-0000-0000-0000-000000000007', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Negotiation',       'negotiation',       '#F97316', 6, 0, 0, 90, datetime('now'), datetime('now')),
  ('a1000008-0000-0000-0000-000000000008', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Won',               'won',               '#22C55E', 7, 1, 1, 100, datetime('now'), datetime('now')),
  ('a1000009-0000-0000-0000-000000000009', '138ff8ec-6d65-493e-b6a9-0f9fef409968', 'Lost',              'lost',              '#EF4444', 8, 1, 0, 0, datetime('now'), datetime('now'));

SQL

echo "  Verifying fixtures..."
USERS=$(sqlite3 "$TEMP_DB" "SELECT COUNT(*) FROM users")
ORGS=$(sqlite3 "$TEMP_DB" "SELECT COUNT(*) FROM organizations")
STAGES=$(sqlite3 "$TEMP_DB" "SELECT COUNT(*) FROM crm_pipeline_stages")
echo "  Users: $USERS, Orgs: $ORGS, Pipeline stages: $STAGES"

# 3. Move to final location
mv "$TEMP_DB" "$OUTPUT"
rm -f "$TEMP_DB-shm" "$TEMP_DB-wal"

echo "Test seed created: $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
echo "Done."
