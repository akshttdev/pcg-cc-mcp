# Seed Database Update — 2026-03-12

## What changed
- Applied all pending migrations through `20260326000000_system_settings`
- Seed DB now includes: system workflows (5), task completion criteria columns, enhanced task templates, system settings table
- Both `dev_assets_seed/duck_kanban.db` and `dev_assets_seed/db.sqlite` updated

## For production deployments
- Run `sqlx migrate run` to apply pending migrations
- Do NOT use the seed database for production — it contains test credentials (admin/admin123)

## For new developers
- The seed DB is auto-copied to `dev_assets/` on first `flox activate`
- Contains pre-configured: admin user, Powerclub Global org, Dashboard Bug Reports project, 5 system workflows
