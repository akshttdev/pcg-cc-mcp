# Avatar → Video Workflow

**Date**: 2026-05-13
**Branch**: `feat/avatar-profile-engine`
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-avatar-engine`
**Base**: `origin/main`
**Depends on**: `planning/2026-05-13--plan--avatar-profile-engine.md` (already shipped — 16-shot profile working)

## Goal

Turn any Avatar Profile into a **full-motion, lip-synced video generator** that's repeatable from the dashboard. One source image → arbitrary scenes of the same person walking, talking, gesturing, on any backdrop, reading any script.

## What we already have

```
┌─ AVATAR PROFILE ENGINE (DONE) ────────┐  ┌─ VIDEO GEN PIPELINE (DONE) ────────┐
│ • 16-shot identity sheet               │  │ • ElevenLabs TTS w/ word align     │
│ • character bible (Claude vision)      │  │ • HeyGen talking-photo render       │
│ • reference image + cdn route          │  │ • FFmpeg post-prod (overlays, cuts) │
│ • elevenlabs_voice_id field on row     │  │ • Public /api/video-gen/final/<id>  │
│ • heygen_avatar_id field on row        │  │ • video_jobs table + status flow    │
└────────────────────────────────────────┘  └──────────────────────────────────────┘
```

## What "full motion" means

A scene is a **sequence of beats**. Each beat is one of three types:

| Beat type | Source | Renderer | Output |
|---|---|---|---|
| `talking_head` | `front.png` or any identity shot | HeyGen / Hedra | Anchor on camera lip-syncing to TTS |
| `motion` | Any action shot (`walking`, `thinking`, etc.) | Kling AI v2.1 image-to-video | 5–10s clip of the anchor in motion |
| `broll` | Search query or prompt | Pexels (free) / Kling text-to-video (premium) | Topic cutaway |

All beats share **one continuous voiceover** stitched in FFmpeg. The script drives beat selection.

## Architecture

```
                     ┌─────────────────────────────────────────────┐
                     │  Dashboard /avatars/:id/videos               │
                     │  ─ "New Video" button                        │
                     │  ─ Script editor (markdown w/ beat tags)     │
                     │  ─ Voice selector (or clone from sample)     │
                     │  ─ Scene preview grid                        │
                     │  ─ "Generate" → polls job status              │
                     │  ─ Final MP4 inline player                   │
                     └────────────────────┬─────────────────────────┘
                                          │
                                          ▼
                  POST /api/video-gen/jobs (existing)
                  body: { avatar_profile_id, script_text, segments[] }
                                          │
                                          ▼
                  ┌───────────────────────────────────────┐
                  │  produce_job() async pipeline          │
                  │                                        │
                  │  1. ElevenLabs TTS → audio + word align│
                  │  2. Compute beat boundaries from align │
                  │  3. For each beat in parallel:         │
                  │     ─ talking_head → HeyGen            │
                  │     ─ motion       → Kling             │
                  │     ─ broll        → Pexels / Kling    │
                  │  4. FFmpeg compositor stitches:        │
                  │     ─ continuous VO track              │
                  │     ─ visual beats in timeline order   │
                  │     ─ overlays, ticker, lower thirds   │
                  │  5. Write final.mp4, mark job ready    │
                  └───────────────────────────────────────┘
                                          │
                                          ▼
                  GET /api/video-gen/final/<job_id> (existing)
```

## Schema additions

`video_jobs` already exists. Add one column to hold the beat plan:

```sql
ALTER TABLE video_jobs ADD COLUMN segments_json TEXT;
-- JSON: [{
--   "label": "Cold Open",
--   "approx_words": 18,
--   "beat_type": "talking_head" | "motion" | "broll",
--   "source_slot": "front" | "walking_three_quarter" | null,
--   "motion_prompt": null | "walking confidently toward camera at golden hour",
--   "broll_query": null | "abstract data flow"
-- }]
```

`segments_json` already partially exists (it's referenced in `produce_job`) — extend the schema to capture beat types not just word counts.

No new tables. Beat artifacts go in `dev_assets/video_gen/{job_id}/beats/<index>.mp4`.

## Replicable workflow — operator's POV

1. **Create avatar** (`/avatars`) — already works. Upload reference → Generate Profile → 16 shots + bible.
2. **Initialize talking-head capability** — one-click button: `POST /avatars/:id/init-talking-head`. Uploads `front.png` to HeyGen, stamps `heygen_avatar_id` on the row. (Currently building this — Phase A.)
3. **Set voice** — pick from ElevenLabs library or clone from a 3-min sample. Stored as `elevenlabs_voice_id` on the avatar.
4. **Write script** — paste script in the dashboard. Optionally tag beats: `[walking] What does this mean for you? [/walking]` or leave plain (defaults to talking-head only).
5. **Generate** — backend handles everything: TTS, per-beat renders, stitch, overlay, output MP4.
6. **Review + publish** — player on the page, share link, or push to the existing social-publishing scheduler (already wired in `social_posts.rs`).

## Phase rollout

| Phase | Scope | Effort | Ship gate |
|---|---|---|---|
| **A** | HeyGen talking-head end-to-end. Prove the pipeline produces a video. | 2–3h | Working today |
| **B** | Kling motion-clip route per slot. Render a 5s walking clip from the bible. | 3–4h | Walking clip MP4 |
| **C** | Beat composition. Script with `[walking]` / `[talking]` tags renders into multi-beat MP4. | half day | Multi-beat final.mp4 |
| **D** | Hedra Character-3 swap when key arrives. Drop-in replacement for HeyGen call site. | 1h | Same MP4, better mouth shapes |
| **E** | ElevenLabs voice clone from sample upload. New voice_id per avatar. | 2h | Voice clone working |
| **F** | Dashboard UI: scene preview, beat editor, voice picker, inline player. | 1 day | Operator-friendly UX |
| **G** | Social scheduling tie-in. Auto-caption, hashtag, schedule from finished MP4. | 4h | Post lands on IG/TikTok |

## Cost model

Per 90s video:

| Component | Phase A baseline | Phase B+ premium |
|---|---|---|
| ElevenLabs TTS | $0.06 | $0.06 |
| HeyGen talking-photo | $0.40 | (or Hedra $0.50) |
| Kling motion clips (3× 5s) | — | $0.21 |
| Sync.so polish pass (optional) | — | $4.50 |
| FFmpeg compositor | free | free |
| **Per-video total** | **~$0.46** | **~$5.20** |

Profile generation amortizes over every video an avatar produces — the 16-shot bible is a one-time ~$0.80 cost.

## Risks / open items

1. **HeyGen talking-photo quality** — known weak spot from the v2 plan prototype. Phase D (Hedra swap) is the planned fix.
2. **Beat timing edge cases** — if word-alignment puts a beat boundary mid-syllable, FFmpeg cut won't snap cleanly. Existing `snap_to_pause()` handles this for talking-head; need parallel logic for motion-beat boundaries.
3. **Motion clip identity drift** — Kling AI does identity preservation but not perfectly. Mitigation: lock with the bible block in the motion prompt, same way the profile engine does.
4. **Voice clone IP** — only allow voice cloning from samples the org owns. Add a "I confirm I own this voice" checkbox before upload.
5. **Render queue under load** — current pipeline spawns a tokio task per job, no queue. At 10 concurrent renders we'll hit HeyGen rate limits. Add a worker queue when usage demands.

## Validation milestones

- [ ] Phase A: hit `/init-talking-head` on Sami v2 → see `heygen_avatar_id` populated
- [ ] Phase A: POST `/video-gen/jobs` with `{avatar_profile_id, script_text: "Hello, I'm Sami."}` → MP4 lands in `dev_assets/video_gen/video/<job>.mp4`
- [ ] Phase A: play MP4, confirm lip sync visible
- [ ] Phase B: `/render-motion-clip` with `slot: "walking_three_quarter"` → MP4 lands in `dev_assets/avatars/<id>/motion/`
- [ ] Phase C: post script with `[walking]` tag → final MP4 contains both talking and walking beats
- [ ] Phase F: dashboard creates a video without curl

## What we DON'T do yet

- Real-time / streaming avatars (Tavus, HeyGen Interactive). Pre-rendered only.
- Multi-person scenes. One avatar per video.
- Translation / multi-language. ElevenLabs supports it but UI doesn't surface it.
- 3D / volumetric avatars. Photo→video only.
