# Model Pickleball — Phase 1 Events / Venues / Sponsors

**Date**: 2026-05-26
**Branch**: `feat/model-pickleball-events`
**Worktree**: `/Users/akshat/Documents/projects/work/pcg-cc-mcp` (root)
**Base**: `main`
**Companion repo**: `Powerclub-Global/model-pickleball-web` PR #3
**Sibling PR**: `Powerclub-Global/pcg-cc-mcp` PR #76 (forms migration)

## Why

The Astro site at model-pickleball.com fetches tour stops and sponsors at
build time when `PUBLIC_USE_API_EVENTS=true`. Today the flag is OFF and
the loader uses `src/lib/events.bootstrap.ts` hardcoded data because the
backend endpoints don't exist. This migration + handlers unblock the
flag, letting marketing edit events via DB instead of code.

Tenancy decision (`organization_id`) recorded on PR #76. This PR matches.

## Scope

- **Migration** `crates/db/migrations/20260526010000_model_pickleball_events.sql`
  adds three tables: `mp_venues`, `mp_events`, `mp_sponsors`.
  - All prefixed `mp_` to avoid collision with any future PCG-wide
    `events` / `venues` / `sponsors` tables.
  - BLOB ids + `organization_id` FK, matches forms migration.
  - `mp_events.slug` is UNIQUE (drives the public `/tour/[slug]` route).
  - `status` enum CHECK constrained.
- **Handlers** `crates/server/src/routes/public_model_pickleball_events.rs`
  - `GET /api/public/model-pickleball/events` — joined with `mp_venues`,
    filtered to `announced` + `live`, ordered by `starts_at ASC`.
  - `GET /api/public/model-pickleball/sponsors` — ordered by tier then name.
  - Response shapes match `ApiEvent` / `ApiVenue` / `ApiSponsor` in
    `model-pickleball-web/src/lib/events.ts` exactly so the adapters keep
    working unchanged.
- **Router wiring** in `crates/server/src/routes/mod.rs` — merged into
  `base_routes` alongside other public routers.

## Out of scope

- Admin CRUD (Phase 3 admin panel).
- Real-time invalidation — the frontend fetches at build time only.
- Past events table (`PAST_EVENTS` stays bootstrap-only per §4 — not in
  Phase 1 API).
- ts-rs derives — frontend types are TypeScript-side and adapters
  decouple them.

## Test plan

- [ ] CI green (compile, clippy, fmt, sqlx-prepare)
- [ ] `flox activate -- sqlx migrate run` succeeds (forms + events both apply cleanly)
- [ ] With `MODEL_PICKLEBALL_ORGANIZATION_ID` set: seed one event +
      venue + sponsor via raw `sqlite3`; `curl /api/public/model-pickleball/events`
      returns the row with nested venue; same for sponsors
- [ ] Flip `PUBLIC_USE_API_EVENTS=true` in `model-pickleball-web/.env`;
      `pnpm build` succeeds and the rendered HTML contains the seeded
      event (and falls back to bootstrap if the backend is down)

## Decisions used

1. **Tenancy**: `organization_id` (recorded by Akshat on this date).
2. **Public visibility filter**: `status IN ('announced','live')`. `draft`
   stays admin-only; `completed`/`archived` belong to past-events UI.
3. **Table names**: prefixed `mp_` to namespace MP tables and avoid
   collisions with hypothetical future PCG-wide events tables.

## Open questions (non-blocking)

- Should sponsors be event-scoped or season-scoped by default? Schema
  supports both (`event_id` nullable). Admin UI decision deferred.
- Should we paginate `/events` once we accumulate past stops? Probably
  yes once `status='completed'` is exposed.
