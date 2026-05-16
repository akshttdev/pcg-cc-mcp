# Avatar Engine Hardening — Stability & Security Pass

**Date**: 2026-05-16
**Branch**: `feat/avatar-profile-engine`
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-avatar-engine`
**Base**: `main` (4114b556)
**PR**: #68 (draft)
**Ports**: FRONTEND 3020 / BACKEND 3022

## Goal

Take the in-flight Avatar Profile Engine PR from "works on the happy path" to "safe to merge into a multi-tenant production server." The engine ships Stage 1 of the Anchor Setup pipeline; before it can serve organizations and their clients, we need to close the access-control, input-validation, and reliability gaps surfaced in the code review of PR #68.

## Success criteria

- Every avatar handler refuses cross-tenant access (verified by negative tests)
- No `.unwrap()` / `.expect()` in non-test code on the engine paths
- All UUID handling goes through `DbUuid` + `parse_db_uuid_param` per `rust-standards.md`
- `created_by` is correctly populated on every avatar — unlocks billing attribution + audit
- Reference-image uploads are validated by magic bytes, size-capped, and never round-tripped with a falsified MIME
- Concurrent `generate_profile` requests for the same avatar collapse to a single in-flight pipeline
- Upstream API errors are scrubbed before they land in `profile_error`
- `render_motion` no longer blocks an Axum worker for up to 10 minutes
- Avatar pipelines are gated by the **organization's** VIBE balance (pre-debit estimate, settle on completion, refund on failure)
- Org admins can set **per-user-within-org** spending caps to prevent any single user from draining the org's pool
- HeyGen integration removed; engine consolidated on Fal Omnihuman for talking-head work
- E2E test covers happy path + 4 failure modes (cross-tenant, race, insufficient balance, per-user cap)
- `/check` passes (cargo fmt, clippy, tsc, eslint, generate-types:check)

## Out of scope (explicit non-goals)

- Real CDN for serving shots and motion clips — local disk + public route is fine for now
- HeyGen / Hedra path (being **removed** — see Phase 3.0; consolidating on Fal Omnihuman for motion)
- Era3D / 3D multi-view reconstruction
- Org-overridable shot prompt customization. `SHOT_SLOTS` locked for now. Future extension point: when a client asks for a custom set, add a `shot_template_id` column on `avatar_profiles` and a `shot_templates` table; no abstraction built today.
- Image-of-likeness consent flow (handled at the dashboard onboarding layer, not here)
- Replacing OpenAI `gpt-image-1` with a different model
- Per-user-across-orgs spending limits — limits are scoped within a single org by that org's admins
- Aptos on-chain VIBE settlement for these charges — uses the existing off-chain `vibe_transactions` ledger; on-chain reconciliation is handled by the existing treasury job

---

## Current state being extended

What exists today on the branch (5 commits past `origin/main`):

- **DB**: `avatar_profiles` extended with `portrait_set TEXT NOT NULL DEFAULT '[]'`, `bible_json TEXT`, `profile_status TEXT NOT NULL DEFAULT 'none'`, `profile_error TEXT`, plus an `idx_avatar_profiles_profile_status` index of dubious value
- **Backend**:
  - `crates/server/src/routes/avatar_engine.rs` — 16-slot taxonomy + parallel `gpt-image-1` shot generation + Claude vision bible
  - `crates/server/src/routes/video_gen.rs` extended with `upload_reference`, `generate_profile`, `init_talking_head`, `render_motion`, plus serve_* endpoints and helpers (`fal_storage_upload`, `elevenlabs_tts`, `avatar_dir`, `avatar_public_base`)
- **Frontend**: `frontend/src/pages/avatars.tsx`, `frontend/src/lib/api/avatars.ts`, route registered in `App.tsx`
- **Verified working**: live in dev server (3020/3022), DB schema correct, endpoints respond, lazy-route renders

Pre-existing patterns we are reusing:

- Access control: `require_org_membership` in `crates/server/src/middleware/access_control.rs` and `require_deal_org_access` pattern in `routes/crm_deals.rs`
- UUID handling: `DbUuid` (`crates/db/src/db_uuid.rs`) and `parse_db_uuid_param` (`crates/server/src/helpers/uuid_params.rs`)
- Error handling: `ApiError` variants, conversions via `From<DomainError>`
- Background jobs: the `generate_profile` `tokio::spawn` pattern is the right shape; `render_motion` needs to match it

---

## Phases

Phase 1 and 2 are **must-fix in PR #68**. Phase 3 is **stretch-in or follow-up PR**. Phase 4 is **post-merge hardening**.

### Phase 1 — Security blockers (must merge in PR #68)

**1.1 Access control on every new handler**

Add a helper modeled on `require_deal_org_access`:

```rust
// crates/server/src/helpers/avatar_access.rs
pub async fn require_avatar_org_access(
    access: &AccessContext,
    pool: &sqlx::SqlitePool,
    avatar_id: &DbUuid,
) -> Result<AvatarProfile, ApiError> {
    let avatar = AvatarProfile::find_by_id(pool, avatar_id).await?;
    if access.is_admin { return Ok(avatar); }
    require_org_membership(access, pool, avatar.organization_id.as_str()).await?;
    Ok(avatar)
}
```

Apply to every handler that touches an avatar:

| Handler | File:line | Check |
|---|---|---|
| `list_avatars` | video_gen.rs:105 | `require_org_membership` on the `org_id` query param (currently unchecked) |
| `get_avatar` | video_gen.rs:133 | `require_avatar_org_access` |
| `create_avatar` | video_gen.rs:117 | `require_org_membership` on body's `organization_id` |
| `update_avatar` / `delete_avatar` | video_gen.rs:145/158 | `require_avatar_org_access` |
| `upload_reference` | video_gen.rs:182 | `require_avatar_org_access` |
| `generate_profile` | video_gen.rs:239 | `require_avatar_org_access` |
| `init_talking_head` | video_gen.rs:286 | `require_avatar_org_access` |
| `render_motion` | video_gen.rs:426 | `require_avatar_org_access` |
| `serve_reference` / `serve_motion_clip` / `serve_shot` | v_g:381/703/809 | These are `public_router` endpoints by design — leave open for now BUT scope the path under a per-avatar signed token (Phase 3.4) before exposing externally |

**1.1.1 Fix `created_by` discard (now on the critical path for billing)**

The `let _ctx = ctx; let created_by: Option<uuid::Uuid> = None;` at v_g:117 was punted on by the original author with the comment "skip FK binding until blob/uuid alignment resolved." Reality from the model audit: **`AvatarProfile::create()` already accepts `created_by: Option<Uuid>`**. The bug is purely the caller passing `None`. Internal binding works fine — the model just stuffs the value into the BLOB column.

Why this is now blocking, not a "nice-to-have audit trail": **the org-level VIBE billing in Phase 4.1 needs to know which user triggered a charge** so org admins can enforce per-user spending caps and so usage reports show "Alice generated 14 avatars this month." Without `created_by`, every charge looks anonymous and per-user caps are unenforceable.

Fix (one-line at v_g:117):
```rust
// before
let created_by: Option<uuid::Uuid> = None;
// after
let created_by = Some(uuid::Uuid::parse_str(ctx.user_id.as_str())
    .map_err(|_| ApiError::InternalError("invalid user_id in access context".into()))?);
```

The model's internal `INSERT` already does `.bind(created_by)` against the BLOB column. (In Phase 2.2 we'll migrate the model to `DbUuid` end-to-end; for the Phase 1 fix we accept the `uuid::Uuid` interim while we wire the auth identity in.)

**1.2 Upload validation on `upload_reference`**

- Add `infer = "0.16"` to `crates/server/Cargo.toml`
- Read full body into bytes (already happening), then `infer::get(&bytes)` and reject any kind other than `image/png` or `image/jpeg`
- Cap to **10 MB** at the field level (the 20 MB body limit is a backstop, not a contract)
- Sniff vs. claimed `Content-Type` mismatch → `ApiError::BadRequest`
- Store as the actual sniffed extension (`.png` / `.jpg`), not always `reference.png` — update `serve_reference` to look up the real extension from the row

**1.3 Path-traversal hardening on `serve_motion_clip`**

`serve_motion_clip` (video_gen.rs:705) currently allows anything matching `*.mp4` without `/` or `..`. Replace with a strict whitelist:

```rust
static MOTION_CLIP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9_\-]+\.mp4$").unwrap());
if !MOTION_CLIP_RE.is_match(&clip) { return Err(ApiError::BadRequest("invalid clip name".into())); }
```

`serve_shot` already whitelists against `SHOT_SLOTS` — leave alone.

**1.4 Error sanitization**

In `generate_bible` (avatar_engine.rs:350), `generate_one_shot` (a_e:462), and `elevenlabs_tts` (video_gen.rs:801): the upstream response body is bubbled into `anyhow!`, which becomes `profile_error`, which is returned to clients. This can leak request artifacts, IP addresses, partial auth headers echoed in error JSON, or rate-limit metadata that signals our spend.

Pattern:

```rust
let status = resp.status();
let body = resp.text().await.unwrap_or_default();
tracing::error!(target: "avatar_engine", status=%status, body=%body, "openai gpt-image-1 failed");
return Err(anyhow!("image generation failed (status {})", status));  // ← scrubbed, safe to surface
```

Apply to all three call sites. Add a `// Why scrubbed: upstream body can echo our request, including data: URIs and request IDs that aid enumeration` comment so the next reader doesn't undo it.

---

### Phase 2 — Correctness blockers (must merge in PR #68)

**2.1 Race condition on `profile_status`**

Concurrent `POST /generate-profile` for the same avatar both spawn pipelines, both write shots, last writer wins. Two-layer fix:

- **DB-level guard**: in `generate_profile` handler, before spawn: `UPDATE avatar_profiles SET profile_status = 'pending' WHERE id = ? AND profile_status NOT IN ('pending','generating') RETURNING id` — if 0 rows updated, return `ApiError::Conflict("avatar profile generation already in progress")` (HTTP 409)
- **Application-level**: keep the existing spawn pattern but only on successful `UPDATE … RETURNING`

This is also a place to record `profile_pipeline_started_at TIMESTAMP` for the recovery worker (Phase 4).

**2.2 Project rule violations — UUIDs**

- `video_gen.rs:110` — `Uuid::parse_str(s)` → `DbUuid::parse(s).map_err(...)`
- `avatar_profile.rs:76` — `Uuid::new_v4()` → `DbUuid::new()`. The 18-field model uses `uuid::Uuid` throughout; convert types to `DbUuid` while we're here (rust-standards explicitly requires `DbUuid` in models)
- All `Path<Uuid>` and `Path<(Uuid, String)>` in handlers at v_g:133, 145, 158, 182, 239, 286, 381, 426, 703, 809, 893, 905 → `Path<String>` + `parse_db_uuid_param(&id, "avatar ID")?`

**2.3 No `.unwrap()` in non-test code**

Six sites in video_gen.rs (pre-existing, but in the file this PR touches):

- v_g:931, 943 (`serve_audio` Response::builder) → `unwrap_or_else(|_| Response::new(Body::empty()))`
- v_g:987, 1006 (`serve_final_video` range + tail) → same
- v_g:1051, 1058 (`audio_path.to_str().unwrap()`) → `.ok_or_else(|| anyhow!("non-utf8 path"))?`
- avatar_engine.rs:255 — `unwrap_or(usize::MAX)` is fine in context but convert to `debug_assert!` since reaching that branch indicates a bug

**2.4 Dead-branch cleanup**

avatar_engine.rs:265–269 — `final_status` has both arms returning `"ready"`. Either return `"partial"` when shots failed (informative for UI) or drop the `if`. Pick "partial" — `avatars.tsx:411`'s `canGenerate` will need a one-line update to also re-enable for `'partial'`.

---

### Phase 3 — Quality (stretch-in or follow-up)

**3.0 Remove the NEW HeyGen surface added by this PR (scope-refined per audit)**

The audit revealed HeyGen is referenced across the codebase (`crates/video_gen/src/heygen.rs`, the legacy VideoJob pipeline at video_gen.rs:1044–1132, `crates/nora/*`, `crates/topsi/*`). **Out of scope** to remove that. **In scope** is only the NEW HeyGen surface this PR added:

- Drop the `init_talking_head` handler (v_g:282–344) and its route registration
- Drop the inline `mod heygen_talking_photo` (v_g:345–368) — only `init_talking_head` calls into it
- Drop the route entry under the `/api/video-gen/avatars/:id/init-talking-head` mounting
- No frontend changes (UI never wired to `init_talking_head`)

Explicit non-changes:
- **Keep** `crates/video_gen/src/heygen.rs` — used by the legacy VideoJob talking-head pipeline at v_g:1044–1132 which is unrelated to this PR
- **Keep** `heygen_avatar_id` and `heygen_avatar_type` columns on `avatar_profiles` — legacy pipeline still reads them
- **Keep** all `crates/nora/*` and `crates/topsi/*` HeyGen references — separate concern

Follow-up PR (post-merge, not in scope here): "Consolidate VideoJob pipeline on Omnihuman" — would remove the legacy v_g:1044–1132 block and the `crates/video_gen/src/heygen.rs` module if motion via Omnihuman is judged superior enough to replace HeyGen wholesale.

**3.0.1 Fix `APP_BASE_URL` for local dev**

video_gen.rs:30 defaults to `https://dashboard.powerclubglobal.com` when `APP_BASE_URL` is unset. Result: locally-generated final-video URLs in the dashboard point at prod. Fix:

- Add `APP_BASE_URL=http://localhost:3020` to `.env` in this worktree
- Add `APP_BASE_URL=https://dashboard.powerclubglobal.com` to `.env.example` so the contract is documented
- Keep the production-URL default in the code — gives us a sane fallback if someone misses the env var, and we'd rather URLs point at prod than break

**3.1 Decompose `render_motion`**

The 270-line handler at v_g:424–700 chains 3 HTTP calls + ffmpeg + 10-min polling loop inside an Axum worker. Split into a service module `crates/services/src/services/avatar_motion.rs` with:

- `synthesize_audio(text, voice_id) -> Bytes`
- `upload_to_fal(bytes) -> Url`
- `submit_omnihuman(audio_url, image_url) -> JobId`
- `poll_omnihuman(job_id) -> Result<Url>`
- `persist_motion_clip(avatar_id, clip_name, url) -> PathBuf`

Wrap them in a `tokio::spawn` orchestrator that matches `generate_profile`'s pattern. Handler becomes ~30 lines.

**3.2 Per-engine reqwest::Client (not per-request)**

`avatar_engine.rs:314, 483` and video_gen.rs:352, 776, 1502, 1540, 1605 create ad-hoc clients. The codebase doesn't have a single app-state client (services own their clients), so:

- Add a `struct AvatarEngine { http: reqwest::Client }` with the 60s default timeout baked in
- Pass it into the spawned pipeline; reuse for bible + shot calls
- One Client per request × 16 shots × 4 concurrent = wasteful but not actively broken

**3.3 Frontend tidy**

- `avatars.tsx:294` — `URL.createObjectURL(file)` leak. Wrap in `useEffect(() => { ...; return () => URL.revokeObjectURL(url) }, [file])`
- `avatars.ts:62` — `uploadReference` uses bare `fetch`. Extend `makeRequest` to accept `FormData` or expose a `makeMultipartRequest` helper. Fix here + audit the rest of the FormData callers in a follow-up
- Add error states for `generateMutation` and `deleteMutation`
- Split `AvatarDetail` + `BibleView` + `ShotTile` out of `avatars.tsx` into a `components/avatars/` directory before the file crosses 600 lines

**3.4 Migration tweaks**

- Drop `idx_avatar_profiles_profile_status` (low cardinality — SQLite planner ignores it) UNLESS Phase 4's recovery worker queries `WHERE profile_status = 'pending'` regularly, in which case keep it
- Add `CHECK (profile_status IN ('none','pending','generating','ready','partial','failed'))` constraint
- Add `profile_pipeline_started_at TIMESTAMP NULL` for recovery worker

**3.5 Memory + constants**

- `Semaphore::new(4)` → `const SHOT_CONCURRENCY: usize = 4` near `SHOT_SLOTS`
- `"anthropic-version: 2023-06-01"` → `const ANTHROPIC_VERSION: &str = "2023-06-01"`
- Drop the source `Vec<u8>` after base64 encoding (a_e:204–207); current usage holds ~80 MB resident while 16 jobs run

**3.6 Reference image fetch timeout**

`reqwest::get(image_url)` at a_e:483 has no timeout — a hung CDN burns a JoinSet slot. Use the engine's shared client (3.2) which has a 60s default.

---

### Phase 4 — Post-merge hardening (separate PR)

**4.1 Cost protection — Vibe-token cost-gating at org level**

No fixed rate limits. If an org has enough VIBE to spin up 50 avatars in an hour, let them. Cost is the gate. Org admins set per-user caps to prevent any single user from draining the pool.

Per-pipeline upstream costs (estimated):
- `generate_profile`: 16 × gpt-image-1 (~$0.04 each) + 1 Claude vision pass (~$0.01) ≈ **$0.65**
- `render_motion`: Fal Omnihuman (~$0.50) + ElevenLabs TTS (~$0.10) ≈ **$0.60**

VIBE charge = **2 × upstream USD cost**, converted via the existing `VibePricingService` USD→VIBE rate. So ~$1.30 worth of VIBE per profile, ~$1.20 worth of VIBE per motion clip.

**4.1.1 Architecture** _(simplified after model audit — most plumbing already exists)_

Reuses existing infrastructure where possible:
- `crates/server/src/helpers/billing.rs` already has `ensure_vibe_balance(pool, project_id)` and `record_llm_vibe_usage(...)` — we add **org-scoped equivalents** beside them
- `VibeTransaction` ledger is source-agnostic (`source_type TEXT` + `source_id BLOB`) — **add `VibeSourceType::Organization` enum variant**, no schema change
- `organizations.vibe_balance REAL NOT NULL DEFAULT 0.0` already exists — atomic UPDATE on this scalar is the pre-debit path. No new `org_vibe_deposits`/`org_vibe_withdrawals` ledger needed (orgs are deposit-and-decrement, not transaction-log)
- `VibePricingService::usd_to_vibe(usd: f64) -> i64` already exists — we **call it directly**, no extension needed. Just expose two tiny helpers in a new `services/avatar_pricing.rs` that compute upstream USD totals (`profile_pipeline_usd() = 16 × 0.04 + 0.01`; `motion_clip_usd(audio_seconds) = 0.50 + 0.10`), apply the 2× markup, and call `usd_to_vibe(upstream × 2.0)`. Hardcode prices for v1; lift to `SystemSettings` later

New surface (the only net-new code in 4.1):
- `crates/server/src/helpers/billing.rs`:
  - `ensure_org_vibe_balance_and_debit(pool, org_id, user_id, estimated_vibe, linked_avatar_id) -> Result<DbUuid, ApiError>` — atomic transaction that: (a) checks user's per-org monthly cap, (b) `UPDATE organizations SET vibe_balance = vibe_balance - ? WHERE id = ? AND vibe_balance >= ?` and asserts 1 row affected, (c) inserts a `vibe_transactions` row with `source_type='organization'`, status pending, linked_avatar_id in metadata. Returns the `charge_id` (transaction id)
  - `settle_org_vibe_charge(pool, charge_id, actual_vibe) -> Result<(), ApiError>` — credits `(estimate - actual)` back to org balance + user spent counter, flips transaction status to 'settled', writes `amount_vibe = actual`
  - `refund_org_vibe_charge(pool, charge_id) -> Result<(), ApiError>` — full reversal, status 'refunded'
- `crates/services/src/services/avatar_pricing.rs` (new, ~50 lines): the two `*_usd()` functions + a thin wrapper that combines markup + USD→VIBE conversion
- `crates/db/src/models/org_member_vibe_limit.rs` (new) + migration to add the table (see 4.1.2)
- New `VibeSourceType::Organization` variant (touches ts-rs export + `Display`/`FromStr` impls + 2 pattern matches in `helpers/billing.rs`)

**4.1.2 Per-user-within-org caps**

New table `org_member_vibe_limits`:

```sql
CREATE TABLE org_member_vibe_limits (
    organization_id TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    monthly_limit_vibe       INTEGER,           -- NULL = uncapped within the org
    current_period_spent     INTEGER NOT NULL DEFAULT 0,
    period_start             TEXT NOT NULL,     -- ISO date, rolls monthly
    PRIMARY KEY (organization_id, user_id)
);
```

- Set by org admins via a new `/api/organizations/:org_id/member-limits` endpoint
- Checked alongside org balance before pre-debit: if the user's `current_period_spent + estimated_vibe > monthly_limit_vibe`, return `ApiError::Forbidden("monthly limit exceeded; ask your org admin")`
- Incremented atomically alongside the org debit
- Reverted on refund

**4.1.3 Pipeline flow with billing**

```
[POST /api/video-gen/avatars/:id/generate-profile]
  1. require_avatar_org_access → returns avatar + org_id
  2. estimate = avatar_pricing::price_profile_pipeline()    // VIBE amount
  3. charge_id = pre_debit_org_vibe(org_id, user_id, estimate)
        ├─ check user's monthly cap → 403 if exceeded
        ├─ atomic UPDATE on org balance → 402 if insufficient
        └─ insert vibe_transaction (status='pending', amount=estimate, linked_avatar_id)
  4. spawn(run_pipeline_with_billing(avatar_id, charge_id))
  5. return 202 Accepted

[run_pipeline_with_billing background task]
  on success:
    actual = sum of upstream costs × 2 × usd_to_vibe_rate
    settle_org_vibe_charge(charge_id, actual)
        ├─ refund (estimate - actual) to org balance and user's period_spent
        └─ UPDATE vibe_transaction status='settled', amount=actual
  on failure:
    refund_org_vibe_charge(charge_id)
        ├─ refund full estimate to org balance and user's period_spent
        └─ UPDATE vibe_transaction status='refunded'
```

**4.1.4 Why pre-debit instead of post-charge**

Existing LLM pattern just checks `balance > 0` then debits actuals afterward. That's safe for cheap LLM calls. For $0.65-per-pipeline jobs that's risky: 5 concurrent pipelines kicked off with $0.30 balance would overdraft by $2.95. Pre-debit makes the org's balance the source of truth and refuses what it can't afford.

**4.1.5 Debug bypass**

The existing `is_vibe_bypass_active(pool)` check from `helpers::vibe_check` continues to work — when on, all checks return Ok and no transactions are written. Useful for E2E tests and demos.

**4.2 Recovery worker**

Avatars stuck in `generating` for >30 min (server crash, panic during spawn) need to transition back to `failed` with `profile_error = 'pipeline orphaned'`. Background job polling `WHERE profile_status = 'generating' AND profile_pipeline_started_at < now() - 30min`.

**4.3 Observability**

- Counter: `avatar_engine_pipelines_total{status, org_id}`
- Histogram: `avatar_engine_pipeline_duration_seconds`
- Counter: `avatar_engine_upstream_errors_total{vendor, status_code}`
- Trace span around each pipeline with `avatar_id` + `org_id` tags

**4.4 Signed URLs for public serve routes**

Today `serve_reference` / `serve_shot` / `serve_motion_clip` are `public_router` and any UUID guesser can fetch any org's images. Add HMAC-signed short-lived tokens (~24h) on the URL; rotate signing key per org.

**4.5 Image-of-likeness consent record**

If we're generating identity-locked imagery from a reference photo, we need a record of who uploaded the source and whether they had consent. Schema: `avatar_consent_records (avatar_id, uploader_user_id, uploaded_at, consent_attestation_text, ip_address, user_agent)`. Block `generate_profile` if no consent row.

---

## Test plan

### Unit tests (cargo test)

- `require_avatar_org_access` — admin bypass, member success, non-member 403, unknown avatar 404
- `MOTION_CLIP_RE` — accepts/rejects expected patterns (backslash, NUL, double-dot, slash, encoded variants)
- Magic-byte sniff — PNG, JPEG, SVG-with-JS, MP4, EXE, zero-byte
- `final_status` returns `'partial'` when ≥1 shot failed and `'ready'` when all 16 succeeded

### Integration tests (cargo test --workspace)

- Happy path: org member uploads → bible written → 16 shots present on disk → status `ready`
- Race: two `generate_profile` POSTs in parallel → one 202, one 409
- Cross-tenant: org-A user calls `get_avatar` / `generate_profile` on org-B's avatar → 403 on every endpoint
- Rate limit: 51st `generate_profile` within 24h → 429

### E2E (Playwright)

`e2e/avatar-engine.spec.ts`:
- Navigate to `/avatars`, drop a reference image, watch progress, assert all 16 shots render in the gallery
- Negative: upload an SVG, assert toast error
- Negative: open avatar-detail of an other-org avatar (forged URL) → app routes home, no data leaked

### Manual smoke

PR description checklist already covers this. After Phase 1+2 land:

- [ ] `flox activate -- cargo sqlx migrate run` against fresh dev DB succeeds
- [ ] `pnpm run dev` — both servers boot, `/avatars` renders
- [ ] Upload reference, generate, all 16 shots appear within 90s
- [ ] Concurrent generate → second click yields 409
- [ ] Inspect a generated shot URL — fetching it as an unauthenticated user (no cookie) still works (intentional pre-Phase-4.4 behavior)

---

## Decisions locked (2026-05-16 session)

1. **`created_by` fix is in scope for PR #68** — bundle with Phase 2.2 DbUuid/blob alignment. Reason: foundation for org-level billing in Phase 4.1.
2. **`SHOT_SLOTS` locked.** No customization layer built today.
3. **HeyGen removed, Omnihuman is the canonical talking-head path.** See Phase 3.0.
4. **`APP_BASE_URL` fix**: add to `.env` and `.env.example`, keep prod-URL default for safety.
5. **Cost gating is by VIBE balance, not by rate limits.** Org-level wallet + per-user monthly caps set by org admins. 2× upstream USD cost. Pre-debit estimate, settle on completion, refund on failure.

## Still-open questions

_(2026-05-16 model audit resolved Q1 and Q2 — see notes below.)_

1. ~~Org wallet schema~~ — **RESOLVED**: `organizations.vibe_balance REAL DEFAULT 0.0` and `organizations.vibe_budget_total REAL` already exist. Project-scoped `vibe_deposits`/`vibe_withdrawals` ledgers exist for project balances; **orgs use the scalar pattern, no new deposit/withdrawal tables needed**. Atomic UPDATE on the scalar is our pre-debit/settle path.
2. ~~`VibeSourceType::Organization` enum variant~~ — **CONFIRMED low risk**: the `vibe_transactions` table is already source-agnostic (`source_type TEXT` + `source_id BLOB`). Adding the variant touches the ts-rs export and pattern matches in `helpers/billing.rs`. No DB migration.
3. **Pricing source-of-truth for upstream USD costs**: hardcode in `avatar_pricing.rs` (simple, requires a PR to change) or pull from `SystemSettings` (op-friendly, mutable at runtime)? **Recommend hardcoded** for v1; `system_settings.rs` model exists if we lift later.

## Model audit findings (2026-05-16)

Pre-flight inventory of everything Phase 4.1 depends on:

| Need | Status | Notes |
|---|---|---|
| Org VIBE balance column | ✅ exists | `organizations.vibe_balance REAL DEFAULT 0.0` (col 18) |
| Org budget cap column | ✅ exists | `organizations.vibe_budget_total REAL` (col 19) — semantics TBD; we use `vibe_balance` for the deposit-and-decrement path |
| Generic ledger | ✅ exists | `vibe_transactions` is source-agnostic (`source_type` + `source_id`) |
| USD→VIBE conversion | ✅ exists | `VibePricingService::usd_to_vibe(usd: f64) -> i64` and reverse |
| Debug bypass | ✅ exists | `helpers::vibe_check::is_vibe_bypass_active` |
| Per-org/per-user member-limits table | ❌ new | `org_member_vibe_limits` (this plan adds it) |
| `VibeSourceType::Organization` variant | ❌ new | enum extension only, no migration |
| `AccessContext.user_id` already `DbUuid` | ✅ confirmed | clean for Phase 1.1.1 fix |
| `AvatarProfile::create()` accepts `created_by` | ✅ confirmed | bug at v_g:117 is the **caller** passing `None`, not the model |
| `avatar_profiles.created_by` column type | ⚠️ BLOB | bind via `bind_uuid_blob()` per CLAUDE.md db_uuid rules |
| `avatar_profiles.project_id` or `client_id` | ❌ absent | confirms org-scoped billing is the only viable scope (no pivot to project) |
| `parse_db_uuid_param` helper | ✅ exists | `(value: &str, param_name: &str) -> Result<DbUuid, ApiError>` |
| `/api/organizations/members.rs` route surface | ✅ exists | natural home for `/member-limits` CRUD |
| `system_settings.rs` model | ✅ exists | available for future runtime-mutable pricing |

**HeyGen scope refinement:** HeyGen is referenced across the codebase (`crates/video_gen/src/heygen.rs`, `crates/nora/src/{profiles, tools/*}.rs`, `crates/topsi/src/{agent, tools/mod}.rs`, and a legacy VideoJob pipeline at video_gen.rs:1044–1132). **The PR adds new HeyGen surface — only that is in scope for removal.** The legacy VideoJob pipeline and cross-crate HeyGen usage is left alone. Schema columns `heygen_avatar_id`/`heygen_avatar_type` stay because legacy code still uses them. See Phase 3.0 update below.

---

## Sequencing & estimate

| Phase | Effort | Can defer? |
|---|---|---|
| 1.1 Access control helper + 9 handler updates | 3–4h | **No** |
| 1.2 Upload validation (`infer` crate + sniff) | 1–2h | **No** |
| 1.3 Path-traversal regex | <1h | **No** |
| 1.4 Error sanitization | 1h | **No** |
| 2.1 Race condition (DB guard) | 1–2h | **No** |
| 2.2 UUID rules (parse_db_uuid_param, DbUuid in model) | 2–3h | **No** |
| 2.3 .unwrap() removal | <1h | **No** |
| 2.4 Dead-branch fix | 15min | **No** |
| 3.0 Remove HeyGen path | 1h | Yes (stretch) |
| 3.0.1 APP_BASE_URL .env fix | 15min | Yes (stretch) |
| 3.1 `render_motion` decomposition | 3–4h | Yes (follow-up) |
| 3.2 Per-engine reqwest client | 1h | Yes |
| 3.3 Frontend tidy | 1–2h | Yes |
| 3.4 Migration tweaks | <1h | Yes |
| 3.5 Memory + constants | <1h | Yes |
| 3.6 Reference image timeout | 15min | Yes |
| 4.1 Org VIBE wallet + per-user caps + billing wiring | 1.5–2 days | Yes (post-merge, separate PR) |
| 4.2–4.5 Recovery, observability, signed URLs, consent | 1–2 days | Yes (post-merge) |
| Test plan | 3–4h | Tests gate merge |

**Phase 1+2 = ~10–14h of focused work.** Realistic to land in PR #68 over 2 working sessions.
**Phase 4.1 (Vibe billing) = ~1.5–2 days** in its own PR after #68 merges.

---

## Session handoff

Before clearing context for the next session: write `planning/SESSION-HANDOFF.md` with the current phase, what's left, and any decisions made on the open questions. Next Claude reads it first (per `CLAUDE.md`).

---

# Execution log — 2026-05-16 session

## Shipped (3 PRs, all pushed)

| PR | Branch | Scope | Status |
|---|---|---|---|
| **#68** | `feat/avatar-profile-engine` | Avatar Profile Engine + Phase 1 security + Phase 2 correctness + Phase 3 stretch (drop HeyGen + `APP_BASE_URL` fix) + 9 e2e tests | **Ready for review** |
| **#69** | `feat/avatar-engine-billing` | Phase 4.1 — Org-level VIBE billing (pre-debit/settle/refund, per-user monthly caps, 10 unit tests) | Open, stacked on #68 |
| **#70** | `feat/avatar-engine-asset-isolation` | Phase 4.4 — Auth-gate the 3 avatar serve endpoints; UUID-enumeration leak closed; +4 e2e tests | Open, stacked on #69 |

URLs:
- https://github.com/Powerclub-Global/pcg-cc-mcp/pull/68
- https://github.com/Powerclub-Global/pcg-cc-mcp/pull/69
- https://github.com/Powerclub-Global/pcg-cc-mcp/pull/70

## Phase status

| Phase | Status |
|---|---|
| 1.1 Access control + helper | ✅ shipped #68 |
| 1.1.1 `created_by` fix | ✅ shipped #68 |
| 1.2 Upload validation (`infer` crate) | ✅ shipped #68 |
| 1.3 Path-traversal regex | ✅ shipped #68 |
| 1.4 Error scrubbing | ✅ shipped #68 |
| 2.1 Race-condition guard | ✅ shipped #68 |
| 2.2 `Uuid → DbUuid` migration | ⚠️ partial — `Uuid::parse_str` for `org_id` query fixed; full model migration + 12 `Path<Uuid>→Path<String>` deferred (mechanical, no bug fixes) |
| 2.3 `.unwrap()` removal | ✅ shipped #68 (6 sites cleaned, only the `Lazy<Regex>` compile-time unwrap remains) |
| 2.4 Dead-branch fix (`partial` status) | ✅ shipped #68 (backend + frontend) |
| 3.0 Drop new HeyGen surface | ✅ shipped #68 |
| 3.0.1 `APP_BASE_URL` for local dev | ✅ shipped #68 |
| 3.1 `render_motion` decomposition | ❌ deferred — blocking render_motion billing wiring |
| 3.2 Per-engine reqwest client | ❌ deferred |
| 3.3 Frontend tidy (`uploadReference` via `makeRequest`, `URL.createObjectURL` cleanup) | ❌ deferred |
| 3.4 Migration tweaks (drop low-cardinality index, add CHECK constraint) | ❌ deferred |
| 3.5 Memory + constants extraction | ❌ deferred |
| 3.6 Reference-image fetch timeout | ❌ deferred |
| **4.1 Org VIBE billing (generate-profile)** | ✅ shipped #69 |
| 4.1 render_motion wiring | ❌ deferred to follow-up — needs Phase 3.1 decomposition first |
| 4.1 Org admin CRUD `/api/organizations/:id/member-limits` | ❌ deferred — backend supports `OrgMemberVibeLimit::upsert`, admins manage via DB until UI lands |
| 4.1 Frontend balance/estimate display | ❌ deferred |
| 4.2 Recovery worker for orphaned `generating` rows | ❌ next session candidate (~2–3h) |
| 4.3 Observability metrics | ❌ next session candidate (~2h) |
| **4.4 Asset isolation (auth-gate serves)** | ✅ shipped #70 (substituted signed URLs with auth-gating since HeyGen path is gone) |
| 4.5 Image-of-likeness consent records | ❌ next session candidate (~half day) |
| Cross-tenant 403 e2e test | ❌ blocked on missing `createTestUser` fixture in e2e helpers |

## Decisions locked this session

1. Org-level VIBE billing (NOT per-user balances). Per-user caps inside the org set by admins.
2. Pre-debit estimate, settle on completion, refund on failure.
3. 2× upstream USD markup.
4. `SHOT_SLOTS` locked for v1; no customization layer.
5. HeyGen removed from NEW avatar engine surface (legacy VideoJob pipeline untouched).
6. Asset isolation via auth-gating, not HMAC signed URLs (simpler, since no external consumer remains).
7. `created_by` foundation for billing attribution.

## Known dev-environment gotchas (not bugs in this PR)

- **flox rustfmt ≠ team rustfmt** — every cargo build/check regenerates ~30 fmt-only changes in `crates/services/`. Discard with `git checkout -- crates/services/` before commits. The team's tip commits aren't `cargo fmt --check`-clean against the flox-pinned rustfmt.
- **`/login` route returns 404 in the React app** — auth.setup.ts in e2e relies on it. My API-only test project (`--project=api`) sidesteps this; full browser-based suite needs the frontend route fixed.
- **Migration version drift on dev DB** — DB has migrations applied from feature branches that don't exist in this branch (20260417*, 20260418*, 20260419*, 20260428*). `cargo sqlx migrate run` complains. Workaround: apply new migrations via raw `sqlite3` + insert tracking row OR delete orphans from `_sqlx_migrations` and let SQLx own everything.

## Next-session candidates (in priority order)

1. **Phase 4.2 — Recovery worker** (~2–3h). Adds `profile_pipeline_started_at` column + background poll for `WHERE profile_status = 'generating' AND started_at < now() - 30min` → transition to `failed`. Improves resilience after server crashes mid-pipeline.
2. **Phase 4.3 — Observability** (~2h). Prometheus counters/histograms for pipeline_total, pipeline_duration_seconds, upstream_errors_total. Trace spans with `avatar_id` + `org_id`.
3. **Phase 4.5 — Consent records** (~half day). `avatar_consent_records (avatar_id, uploader_user_id, uploaded_at, consent_attestation_text, ip_address, user_agent)`. Block `generate_profile` if no consent row.
4. **Phase 3.1 — `render_motion` decomposition** (~3–4h). Unblocks render_motion billing wiring (in scope but couldn't fit).
5. **Phase 2.2 full** — Mechanical UUID rule cleanup (~2–3h). Style-standards alignment, no bug fixes.
6. **Org admin CRUD endpoint** for `/api/organizations/:id/member-limits` (~1h). Backend already supports the operation.
7. **Cross-tenant 403 e2e** — depends on a `createTestUser` fixture that doesn't exist yet.

## Open questions still pending answers

None. The 5 open questions from session start are all resolved. The 3 sub-decisions on VIBE billing are also resolved.
