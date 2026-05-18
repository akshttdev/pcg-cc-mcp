# Avatar Profile Engine

**Date**: 2026-05-13
**Branch**: `feat/avatar-profile-engine`
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-avatar-engine`
**Base**: `origin/main` (4114b556)
**Ports**: FRONTEND 3020 / BACKEND 3022

## Goal

Operator uploads one reference image of a person (real or AI-generated). Backend returns a "Profile" — 16 multi-angle/expression/wardrobe shots and a structured character bible — that gets reused for every future generation of that character.

This is the missing Stage 1 from `planning/2026-03-30--plan--ai-video-studio-v2.md`, scoped down to ship today and powered by OpenAI `gpt-image-1` (since the reference comes from GPT and `images/edits` preserves identity better than DALL-E 3).

## Current state being extended

- `avatar_profiles` table exists (migration `20260426000000_video_studio.sql`) with one `reference_image_url` field and a free-text `identity_doc`.
- `/api/video-gen/avatars` CRUD exists in `crates/server/src/routes/video_gen.rs`.
- Video Studio frontend (`frontend/src/pages/video-studio.tsx`) lists avatars in a tab but offers no profile concept.
- DALL-E 3 mood-board path exists in `brand.rs:1622` (stores ephemeral URLs — bad pattern, we're not repeating it).

## Out of scope

- HeyGen / Hedra character initialization from the new shots (next phase — once shots exist, those calls are trivial).
- Era3D 3D multi-view reconstruction (v2 plan, not needed for talking-head use case).
- Real CDN. Using local disk + public route, same pattern as `video_gen.rs` audio/video serving.

## End-state shape

```
┌─ /avatars ────────────────────────────────── [+ New Avatar] ─┐
│                                                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ [thumb]  │  │ [thumb]  │  │ [thumb]  │  │  + drop  │     │
│  │ Sami     │  │ Nora     │  │ <new>    │  │  image   │     │
│  │ ● ready  │  │ ● ready  │  │ ⏳ 6/16  │  │          │     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
└────────────────────────────────────────────────────────────────┘

Click an avatar → detail panel:

┌─ <name> · Profile ─────────────────────────────────────┐
│  ┌──┐  Name [____________]  Slug [____________]        │
│  │  │  Voice (ElevenLabs) [Rachel ▼]                    │
│  └──┘                                                   │
│                                                          │
│  Identity Sheet (8)                                     │
│  [ front ][ 3/4 L ][ 3/4 R ][ profile L ]              │
│  [ profile R ][ full body ][ smile ][ serious ]         │
│                                                          │
│  Wardrobe (4)                                           │
│  [ studio blk ][ editorial wht ][ casual ][ evening ]   │
│                                                          │
│  Action (4)                                             │
│  [ laugh ][ look-down ][ walk ][ thinking ]             │
│                                                          │
│  ▸ Character Bible (auto-generated, editable JSON)      │
│  [ Regenerate shot ▼ ] [ Lock as canonical ]            │
└──────────────────────────────────────────────────────────┘
```

## Schema diff

Migration `20260513000000_avatar_profile_engine.sql`:

```sql
ALTER TABLE avatar_profiles ADD COLUMN portrait_set    TEXT DEFAULT '[]';
ALTER TABLE avatar_profiles ADD COLUMN bible_json      TEXT;
ALTER TABLE avatar_profiles ADD COLUMN profile_status  TEXT DEFAULT 'none';
ALTER TABLE avatar_profiles ADD COLUMN profile_error   TEXT;
```

- `portrait_set` — JSON array of `{ slot: "front" | "three_quarter_left" | ... , url, prompt, locked, generated_at }`
- `bible_json` — structured character bible (JSON)
- `profile_status` — `none` / `pending` / `generating` / `ready` / `failed`
- `profile_error` — error text on failure

Status column already exists on the table from the original migration; this is a separate column scoped to the profile generation lifecycle so we don't mix it with avatar's overall lifecycle.

## Shot slot taxonomy (16 slots)

Identity Sheet (8):
1. `front` — straight-on headshot, neutral
2. `three_quarter_left` — head turned 45° camera-left
3. `three_quarter_right` — head turned 45° camera-right
4. `profile_left` — full side profile, camera-left
5. `profile_right` — full side profile, camera-right
6. `full_body_front` — full-body straight-on, neutral pose
7. `smile_warm` — front, warm smile
8. `serious_neutral` — front, neutral serious

Wardrobe (4):
9. `studio_black` — black studio backdrop, formal styling
10. `editorial_white` — clean white editorial background
11. `casual_outdoor` — daylight, casual outdoor context
12. `evening_dressed` — evening / dressed-up styling

Action (4):
13. `laughing` — genuine laugh, mid-action
14. `looking_down` — contemplative downward gaze
15. `walking_three_quarter` — walking toward camera, 3/4 angle
16. `thinking_pose` — hand near chin, thoughtful

Each slot has a prompt template that interpolates the character bible (e.g. `{hair}, {eyes}, {skin_tone}, {build}, {style_vibe}`).

## Pipeline

```
POST /api/video-gen/avatars/upload-reference  (multipart)
  → write to dev_assets/avatars/<id>/reference.png
  → return { reference_image_url: "/api/video-gen/avatars/<id>/reference.png" }

POST /api/video-gen/avatars/:id/generate-profile
  → set profile_status = 'generating'
  → spawn tokio task:
      1. Claude vision pass:
         - prompt Claude with the reference image
         - extract structured bible (demographics, hair, eyes, skin, build,
           wardrobe defaults, vibe, do-not-do)
         - persist to bible_json
      2. For each of 16 slots (concurrency=4):
         - render prompt using bible + slot template
         - POST OpenAI /v1/images/edits  model=gpt-image-1, image=reference.png
         - save response PNG to dev_assets/avatars/<id>/shots/<slot>.png
         - append to portrait_set: { slot, url, prompt, locked:false, generated_at }
      3. set profile_status = 'ready'; thumbnail_url = front shot URL
```

Public serving:
```
GET /api/video-gen/avatars/:id/reference.png      → reference image
GET /api/video-gen/avatars/:id/shots/:slot.png    → generated shot
```

Both are public (HeyGen / Hedra need fetchable URLs later — same pattern as existing `serve_audio`/`serve_video`).

## Files touched

Backend:
- `crates/db/migrations/20260513000000_avatar_profile_engine.sql` (new)
- `crates/db/src/models/avatar_profile.rs` (extend)
- `crates/server/src/routes/video_gen.rs` (add multipart upload, generate-profile, public shot serving)
- `crates/server/src/routes/video_gen/avatar_engine.rs` (new module — pipeline impl)
- `Cargo.toml` (workspace) — may need `axum-extra` multipart or `axum` features

Frontend:
- `frontend/src/pages/avatars.tsx` (new)
- `frontend/src/pages/avatars/AvatarDetail.tsx` (new)
- `frontend/src/lib/api/avatars.ts` (new) — typed client
- `frontend/src/App.tsx` (route registration)
- `shared/testids.ts` (testid helpers for avatars page)

## Risks / open items

1. **OPENAI_API_KEY not set** in worktree .env — required for pipeline. Caller must add before smoke test.
2. **gpt-image-1 cost** — 16 shots @ ~$0.04/shot ≈ $0.64/profile. Not free, log usage.
3. **OpenAI `images/edits` parity with gpt-image-1** — verify the request shape supports `model=gpt-image-1` with `image[]` input. If only DALL-E 2 works for edits, fall back to `images/generations` with prompt-only and pray for consistency (or move to FLUX-Redux).
4. **Identity drift across 16 shots** — main risk. Mitigation: always include the bible's full identity block in every prompt + temperature=low + `images/edits` over `generations`.
5. **Background of reference** — current reference has white tank top against grey wall. Shots that want different backgrounds (evening, outdoor) may struggle with edits. May need to mask the subject first.

## Validation

End of build:
- `cargo fmt --all -- --check`
- `cargo clippy --all --all-targets -- -D warnings`
- `cd frontend && npx tsc --noEmit && npx eslint . --ext ts,tsx`
- `npm run generate-types:check`

Smoke:
- Start dev servers on 3020/3022
- Hit `/avatars` page, drop reference image
- Click Generate Profile
- Confirm portrait_set populates and grid renders 16 thumbs
