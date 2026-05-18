# Social Posts API + Calendar — Bug Report & Fix Plan

**Date:** 2026-05-18
**Author:** Sirak Studios (Claude session)
**Severity:** Blocker — Social Calendar feature is non-functional end-to-end
**Repro env:** `dashboard.powerclubglobal.com` (production), logged in as `sirak@sirakstudios.com`
**Reproduction context:** Attempted to populate the TIACA Social Content calendar with 18 LinkedIn posts from the May/June 2026 content guide.

---

## TL;DR

The Social Calendar feature has **four independent bugs** that compound to make the feature unusable:

1. **Create form sends platform name strings as if they were UUIDs** → every UI submit fails with 422.
2. **`POST /api/social/posts` silently ignores `status`** → records always saved as `draft` even when `status: 'scheduled'` is sent.
3. **`GET /api/social/posts` list endpoint is broken** → returns `500 DatabaseError` (no filter) or `data: []` (with `project_id`), even when records exist with that exact `project_id`. Calendar UI always shows "No posts yet".
4. **Records appear to be deleted between creation and later read.** 18 posts created via API at ~04:50 UTC verified live by both POST response and PATCH response. ~10 min later, `GET /api/social/posts/{id}` on the same IDs returns 404 for all 18. Root cause unknown — could be a janitor cron, a redeploy wiping ephemeral state, a transaction issue, or column-decode corruption that masks records.

Net effect: even when the API workaround is used, the calendar UI shows nothing AND the records vanish within minutes.

---

## Bug 1 — `platforms` UUID-vs-string mismatch

### Reproduction
```bash
POST /api/social/posts
{
  "project_id": "3999206c-9065-493e-bf48-878919cf9dad",
  "caption": "Test",
  "platforms": ["linkedin"],
  "status": "scheduled",
  "scheduled_for": "2026-05-04T09:00:00Z"
}
```

### Response
```
HTTP 422 Unprocessable Entity
Failed to deserialize the JSON body into the target type:
  platforms[0]: UUID parsing failed: invalid character: expected an optional prefix of `urn:uuid:`
  followed by [0-9a-fA-F-], found `l` at 1 at line 1 column 123
```

### Root cause
Backend deserializer expects `platforms: Vec<Uuid>` (presumably `Vec<social_account_id>`). Frontend `Create Post` form sends platform name strings: `["linkedin"]`, `["instagram"]`, etc.

### Workaround (current)
Send `platforms: []`. The record is created but with no platform marker.

### Fix options
- **A.** Change backend to accept `Vec<String>` of platform names; resolve to social_account_ids in a join.
- **B.** Change frontend to send the connected `social_account_id` for the selected platform. Requires social accounts to exist before posting — currently `GET /api/social/accounts?project_id=…` returns empty array (no accounts ever connected for TIACA).
- **C.** Add a `platform_names` companion field separate from `platforms` (social_account_ids), where the form populates `platform_names` for plan-only posts and `platforms` for connected-account posts.

**Recommended:** Option C. Lets us schedule plan-only posts (the 80% case for content-calendar ingestion) without requiring real platform OAuth.

---

## Bug 2 — `status` ignored on POST

### Reproduction
```bash
POST /api/social/posts { ..., "status": "scheduled", "scheduled_for": "2026-05-04T09:00:00Z" }
```

### Response
```json
{ "success": true, "data": { "status": "draft", "scheduled_for": "2026-05-04T09:00:00Z", ... } }
```

`scheduled_for` is honored. `status` is silently downgraded to `draft`. Sending `status: 'pending_review'` shows the same downgrade.

### Workaround
After POST, immediately `PATCH /api/social/posts/{id}` with `{ "status": "scheduled" }`. PATCH respects the value.

### Root cause hypothesis
Create handler probably hardcodes `status = 'draft'` on insert. Update handler reads the request body correctly.

### Fix
Honor `status` on create. Same validation rules as PATCH.

---

## Bug 3 — List endpoint returns empty/500

### Reproduction
```
GET /api/social/posts                                       → 500 DatabaseError
GET /api/social/posts?limit=100                             → 500 DatabaseError
GET /api/social/posts?client_id=<uuid>                      → 500 DatabaseError
GET /api/social/posts?organization_id=<uuid>                → 500 DatabaseError
GET /api/social/posts?project_id=<valid-uuid>               → 200 { data: [] }
GET /api/social/posts?project_id=<uuid>&status=draft        → 200 { data: [] }
GET /api/social/posts/<id>                                  → 200 (when record exists)
```

### Server error (no filter)
```
DatabaseError: error occurred while decoding column "id": invalid character...
```

This is the same family as the prior session's "expected 16 bytes, found 36" UUID decode error. Some rows in the `social_posts` table (or a joined table) have IDs that don't deserialize as `uuid::Uuid`.

### Why `project_id` filter "succeeds" with empty result
Filtering by `project_id` excludes the corrupt rows BUT also excludes our valid new rows. Two possibilities:
- The list query joins to a table where the new rows have NULL FKs (e.g., `social_account_id IS NULL` filters out plan-only posts).
- The list query compares `project_id` as bytes against the input string, and the column storage format mismatches.

A diagnostic post just created with `project_id = '3999206c-9065-493e-bf48-878919cf9dad'` is reachable by GET-by-ID **but invisible to the list query with that same project_id**. Confirms the list query has a filter or join bug independent of the corrupt rows.

### Fix plan
1. **Find the corrupt rows.** `SELECT id FROM social_posts WHERE LENGTH(id::text) != 36;` or equivalent. Delete or repair.
2. **Audit the list query.** Make sure the `WHERE project_id = $1` is comparing UUIDs cleanly. Avoid `LEFT JOIN social_accounts` filtering plan-only posts out by accident.
3. **Add an end-to-end test:** POST a post via API, then GET via list, assert count >= 1.

---

## Bug 4 — Records disappear between create and later read

### Reproduction (this session)
- **04:50 UTC** Bulk-created 18 posts via `POST /api/social/posts`. Each POST returned a fresh UUID and `success: true`.
- **04:51 UTC** PATCH'd each ID with `{status: 'scheduled'}`. PATCH returned the full record with status=scheduled and the correct scheduled_for.
- **~05:00 UTC** `GET /api/social/posts/{id}` on each of the 18 IDs returns **404 Not Found**.

A new diagnostic post created at 05:00 UTC is still alive when re-fetched seconds later — so the table is writable and reads-by-id work for fresh records. The 18 are simply gone.

### Hypotheses (in order of likelihood)
1. **A scheduled cleanup job** ran between :51 and :00 that purged posts matching some predicate (e.g., `status = 'scheduled' AND scheduled_for < NOW() AND social_account_id IS NULL`). Most of the 18 had `scheduled_for` dates in the past (May 4 onward) when today is May 18.
2. **Backend redeploy/restart** that wipes some kind of in-memory layer (queue, draft cache) and the records were never committed to durable storage — unlikely, GET-by-ID succeeded at PATCH time.
3. **Cascade delete** from a parent table whose row was rotated.

### Investigation steps
- Check application logs between 04:51 and 05:00 UTC for delete statements or scheduled-job runs.
- Inspect `social_posts` directly in the DB — is row count consistent with what we expect, or are there tombstone markers / soft-delete columns?
- If there's a janitor job, find it; either fix the predicate so it doesn't nuke plan-only scheduled posts, or move to soft-delete.

---

## Recovery — Repopulating the TIACA Calendar

Once Bugs 3 and 4 are fixed (list works, records persist), the calendar can be repopulated in seconds via the same bulk-API approach. The 18 captions are extracted and staged at:

- `/tmp/tiaca_posts.json` (current session)
- Or re-extract from `~/Downloads/TIACA-ES2026-LinkedIn-Execution-Guide.pdf` with `/tmp/extract_captions.py`

Per-post: 14 May posts (May 4, 6, 7, 12, 13, 14, 18, 19, 20, 21, 26, 27, 28, 31) + 4 June posts (Jun 1, 2, 3, 4), TIACA Acquisition project (`3999206c-9065-493e-bf48-878919cf9dad`).

---

## Adjacent observations (smaller, but worth filing)

- **Project ID with a literal `p`:** `a1aca00c-0000-0000-0000-000000000p01` ("TIACA — Strategic Growth Partnership"). Not a valid UUID. The frontend keeps trying to list posts for this project and gets 400 on every request. Either rename the project UUID or strip the bogus id from the list of clients.
- **The Create Post form's `Status` dropdown shows `Pending Review`** but the backend uses `pending_review` (underscore). Send/parse asymmetry needs verification.
- **The "linkedin" platform button visual state is unclear.** No `[active]` aria attribute exposed in the snapshot. Hard to tell programmatically whether the user clicked it.

---

## Proposed prioritization

| Order | Bug | Reason |
|-------|-----|--------|
| P0 | Bug 4 (records disappear) | If posts get wiped, no other fix matters |
| P0 | Bug 3 (list endpoint) | Calendar UI is the user surface; nothing visible without it |
| P1 | Bug 1 (platforms UUID) | Blocks the New Post form for any user without connected accounts |
| P2 | Bug 2 (status downgrade) | Workaroundable via PATCH |
| P3 | Project ID literal-`p` | Frontend noise, not a blocker |

---

## What this report is NOT

This is **not** a code patch. It's a triage doc the dashboard team can use to scope a sprint. Each bug has a deterministic repro that should slot into an integration test once fixed.
