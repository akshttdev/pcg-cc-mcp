# Model Pickleball — Phase 1 Forms Module

**Date**: 2026-05-26
**Branch**: `feat/model-pickleball-forms`
**Worktree**: `/Users/akshat/Documents/projects/work/pcg-cc-mcp` (root)
**Base**: `main`
**Companion repo**: `Powerclub-Global/model-pickleball-web` branch `feat/architecture-and-events-day1`

## Why

The Astro site at model-pickleball.com already POSTs to 8 endpoints under
`/api/public/model-pickleball/*` (newsletter, contact, model & agency
registrations, brand/sponsor/media/venue inquiries). None of these
endpoints exist in `pcg-cc-mcp` today — every form on the live site
currently 404s. This is a Sprint 1 prerequisite and the precondition for
any later MP work (events, live shopping, ticketing).

Discovery details: see `ARCHITECTURE.md` §0a in `model-pickleball-web`.

## Scope (this PR)

- **Migration**: `crates/db/migrations/20260526000000_model_pickleball_forms.sql`
  adds 4 tables — `newsletter_subscribers`, `mp_contact_submissions`,
  `mp_registrations` (kind=model|agency), `mp_inquiries` (kind=brand|
  sponsor|media|venue).
- Uses BLOB ids + `organization_id` BLOB FK to match recent practice
  (`20260426000000_video_studio.sql`).
- CHECK constraints on kind/status enums; soft-delete via `deleted_at`;
  index on every FK + frequently-queried column.

## Out of scope (follow-up PRs)

- Rust route handlers (`crates/server/src/routes/public_model_pickleball.rs`)
  — coming as a follow-up commit on this same branch once the migration is
  green.
- Cloudflare Turnstile verification (columns reserved; verification logic
  deferred).
- Central PCG contacts knowledge-base upsert (`crm_contacts` is
  project-scoped; needs Auri's call on whether MP should upsert there or
  in a dedicated MP project_id).
- Events / venues / sponsors tables (a separate migration once forms ship;
  draft sits in `model-pickleball-web/docs/schema-draft-phase1.sql`).

## Open questions for review

1. **Tenancy**: assumed `organization_id`. Confirm vs `client_id`.
2. **Contacts KB target**: where do form-submitter emails get upserted?
   `crm_contacts` is `project_id`-scoped — is there a canonical PCG-wide
   contacts table, or do we create a Fusion project_id constant?
3. **Newsletter unique key**: drafted `(organization_id, email)`. Should
   `source` be part of it?
4. **Spam mitigation**: columns reserved for Turnstile token, IP, UA. OK
   to defer actual verification to a follow-up PR?

## Test plan

- [ ] `flox activate -- sqlx migrate run` against fresh `dev_assets/db.sqlite` succeeds
- [ ] `sqlite3 dev_assets/db.sqlite ".schema newsletter_subscribers"` shows the table
- [ ] Same for `mp_contact_submissions`, `mp_registrations`, `mp_inquiries`
- [ ] All 4 tables show indexes via `.indexes <table>`
- [ ] (Follow-up commit) `cargo test --workspace` passes after route handlers land
- [ ] (Follow-up commit) End-to-end: model-pickleball-web `.env` points at this
      backend, every form submits and returns 200 with a row written

## Coordination

- Web-side PR (frontend events loader + ARCHITECTURE.md v1.1):
  `https://github.com/Powerclub-Global/model-pickleball-web/compare/main...akshttdev:feat/architecture-and-events-day1`
  Safe to merge independently of this PR.
