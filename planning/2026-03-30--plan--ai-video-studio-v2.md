# PCG AI Video Studio — End State Implementation Plan
**Date:** 2026-03-30
**Version:** 2.0 — Post Prototype Assessment
**Status:** Template for Production Implementation

---

## EXECUTIVE SUMMARY

The AI Video Studio is a fully automated broadcast content pipeline that generates professional talking-head news segments featuring PCG AI anchors (Sami Satoshi, Nora). A prototype was built on 2026-03-29/30 proving the concept end-to-end. This document defines the production-grade end state.

**Prototype proved:**
- ElevenLabs TTS → WAV → HeyGen CDN → talking photo video works
- FFmpeg post-production overlay (lower thirds, ticker, logo, b-roll cuts) works
- PCG brand assets (Cinzel font, black/gold, logo) integrate correctly
- Cut structure with continuous VO audio works

**Prototype gaps being addressed in this plan:**
- Lip sync quality (Talking Photo ceiling — replacing with Hedra + Sync.so)
- Intelligence pipeline (manual scripts — replacing with Perplexity/Tavily automation)
- Script engine (hardcoded — replacing with structured Claude generation)
- No dashboard UI (all bash scripts — replacing with full Video Studio page)
- No distribution wiring (social connectors exist but disconnected)
- Scene generation (not in v1 plan — adding Kling AI for action shots)

---

## SYSTEM ARCHITECTURE

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║                    PCG AI VIDEO STUDIO — PRODUCTION ARCHITECTURE                ║
╚══════════════════════════════════════════════════════════════════════════════════╝

┌─────────────────────────────────────────────────────────────────────────────────┐
│  DASHBOARD UI  /video-studio                                                    │
│                                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │ Anchors Tab │  │  Pulse Tab  │  │ Scripts Tab │  │  Jobs Tab   │           │
│  │             │  │             │  │             │  │             │           │
│  │ Sami Satoshi│  │ AI Compute  │  │ Draft #1    │  │ Job #847    │           │
│  │ Nora        │  │ Crypto News │  │ APPROVED    │  │ RENDERING   │           │
│  │ [+ New]     │  │ Data Sov.   │  │ Draft #2    │  │ Job #846    │           │
│  │             │  │ [Generate]  │  │ [Edit/Aprv] │  │ COMPLETE ▶  │           │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘           │
└─────────┼────────────────┼────────────────┼────────────────┼────────────────────┘
          │                │                │                │
          ▼                ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  REST API  /api/video-gen/*                                                     │
│                                                                                 │
│  /anchors          /pulse           /scripts         /jobs                     │
│  GET  POST         GET  POST        GET  POST        GET  POST                 │
│  GET :id           GET :id          GET :id          GET :id                   │
│  PATCH :id         PATCH :id        PATCH :id        DELETE :id                │
│  DELETE :id        DELETE :id       POST :id/approve POST :id/produce          │
└────────────────────────────────────────────┬────────────────────────────────────┘
                                             │
          ┌──────────────────────────────────┴──────────────────────────────────┐
          │                                                                     │
          ▼                                                                     ▼
┌──────────────────────────┐                              ┌─────────────────────────┐
│  crates/video_gen/       │                              │  crates/db/models/      │
│                          │                              │                         │
│  mod.rs                  │                              │  anchor_profile.rs      │
│  hedra.rs       ← NEW    │                              │  video_pulse_item.rs    │
│  synclabs.rs    ← NEW    │                              │  video_script.rs        │
│  kling.rs       ← NEW    │                              │  video_job.rs           │
│  heygen.rs      ✅ done  │                              │  video_scene.rs  ← NEW  │
│  tts.rs         ✅ done  │                              │                         │
│  script_gen.rs  ← NEW    │                              └─────────────────────────┘
│  intelligence.rs← NEW    │
│  overlay.rs     ← NEW    │
│  broll.rs       ← NEW    │
└──────────────────────────┘
```

---

## VIDEO PIPELINE FLOW — PRODUCTION

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║                       FULL PIPELINE — STAGE BY STAGE                           ║
╚══════════════════════════════════════════════════════════════════════════════════╝

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STAGE 1 — ANCHOR SETUP  (one-time per persona)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [Operator uploads portrait]
          │
          ├──▶ FLUX Pro Ultra Redux generates consistent multi-angle batch (10 images)
          │    fal-ai/flux-pro/v1.1-ultra/redux  strength=0.08  raw=true
          │
          ├──▶ Era3D reconstructs 3D multi-view geometry
          │    fal-ai/era3d  →  front / 3/4 / profile / rear views
          │
          ├──▶ Hedra Character-3 talking photo initialized
          │    POST api.hedra.com/v1/characters  aspect_ratio=9:16
          │    → anchor_profile.hedra_character_id stored in DB
          │
          ├──▶ ElevenLabs voice selected / cloned
          │    → anchor_profile.elevenlabs_voice_id stored in DB
          │
          └──▶ anchor_profiles record created with full asset set
               status: active

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STAGE 2 — INTELLIGENCE GATHERING  (cron daily 06:00 UTC or on-demand)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ┌─────────────────────────────────────────────────────────┐
  │  Parallel intel agents (intelligence.rs)                │
  │                                                         │
  │  Perplexity sonar-pro ──────────────────────┐           │
  │  model: sonar-pro                           │           │
  │  search_recency_filter: day                 │  Claude   │
  │                                             ├─▶ dedup   │
  │  Tavily ────────────────────────────────────┤  score    │
  │  search_depth: advanced                     │  tag      │
  │  topic: news                                │           │
  │                                             │           │
  │  Vertical keywords (per anchor profile) ───▶│           │
  │  e.g. "AI chips", "crypto institutional",   │           │
  │       "data sovereignty enterprise"         │           │
  └──────────────────────────────┬──────────────┘
                                 │
                                 ▼
                    video_pulse_items (DB)
                    relevance_score 0-10
                    status: pending
                                 │
                    Dashboard "Pulse Tab" surfaces ranked stories
                    Operator selects story → triggers Stage 3

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STAGE 3 — SCRIPT GENERATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Input: pulse_item + anchor identity doc + vertical + tone

  Claude claude-opus-4-6 generates structured VideoScript:
  ┌────────────────────────────────────────────────────────────────┐
  │  cold_open       (0:00–0:08)  Hook — "Breaking in [sector]..." │
  │  headline        (0:08–0:20)  Core story + key stat            │
  │  body            (0:20–1:20)  Analysis — 3 sub-beats max       │
  │  brand_tiein     (1:20–1:40)  "What this means for PCG..."     │
  │  cta_signoff     (1:40–2:00)  CTA + anchor sign-off            │
  │                                                                │
  │  lower_third_name  / lower_third_title                         │
  │  chyron_text                                                   │
  │  ticker_items[]    (3–5 scrolling headlines)                   │
  │  broll_cues[]      (timestamp → Pexels/Kling search query)     │
  │  scene_prompts[]   (timestamp → Kling generation prompt)       │
  └────────────────────────────────────────────────────────────────┘

  → video_scripts record created   status: draft
  → Operator reviews + edits in Script Editor tab
  → POST /api/video-gen/scripts/:id/approve  →  triggers Stage 4

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STAGE 4 — ANCHOR VIDEO GENERATION (Hedra + ElevenLabs)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Step 4a: ElevenLabs TTS
  ┌──────────────────────────────────────────────────────────────┐
  │  POST /v1/text-to-speech/{voice_id}/with-timestamps          │
  │  model: eleven_turbo_v2_5                                    │
  │  → MP3 + word_alignment[]  saved to disk + DB               │
  │  → MP3 → WAV (ffmpeg -ar 16000 -ac 1)                       │
  └──────────────────────────────────┬───────────────────────────┘
                                     │
  Step 4b: Hedra Character-3 generation (REPLACES HeyGen Talking Photo)
  ┌──────────────────────────────────▼───────────────────────────┐
  │  POST api.hedra.com/v1/characters                            │
  │  {                                                           │
  │    "image_url":    "{anchor portrait CDN URL}",              │
  │    "audio_url":    "{elevenlabs output URL}",                │
  │    "aspect_ratio": "9:16",                                   │
  │    "resolution":   "720p"                                    │
  │  }                                                           │
  │  → Poll GET /v1/characters/{id}                              │
  │  → Download MP4 when status = "complete"                     │
  │                                                              │
  │  WHY HEDRA: purpose-built photo→video, native 9:16,         │
  │  dramatically better lip sync than HeyGen Talking Photo      │
  └──────────────────────────────────┬───────────────────────────┘
                                     │
  Step 4c: Sync.so lip sync pass (QUALITY ENHANCEMENT)
  ┌──────────────────────────────────▼───────────────────────────┐
  │  POST fal.ai/fal-ai/sync-lipsync/v2                         │
  │  {                                                           │
  │    "video_url": "{hedra output}",                            │
  │    "audio_url": "{elevenlabs wav}"                           │
  │  }                                                           │
  │  → $3/min — optional pass for premium jobs                  │
  │  → Tightens phoneme alignment on top of Hedra output         │
  └──────────────────────────────────┬───────────────────────────┘
                                     │
                              anchor_video.mp4
                          video_jobs.status = anchor_done

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STAGE 5 — SCENE GENERATION + POST-PRODUCTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Step 5a: Action shot generation (parallel, per broll_cue)
  ┌──────────────────────────────────────────────────────────────┐
  │  Option A — Pexels stock (free, fast):                       │
  │  GET api.pexels.com/videos/search?query={cue}               │
  │  → best match clip URL stored per cue                        │
  │                                                              │
  │  Option B — Kling AI generated (premium, unique):           │
  │  POST fal.ai/fal-ai/kling-video/v1.6/standard/image-to-video│
  │  { image_url: anchor_portrait, prompt: scene_prompt }        │
  │  $0.014/sec — used for hero shots only                       │
  └──────────────────────────────────┬───────────────────────────┘
                                     │
  Step 5b: Edit assembly (overlay.rs — our proven FFmpeg pipeline)
  ┌──────────────────────────────────▼───────────────────────────┐
  │                                                              │
  │  EDIT STRUCTURE:                                             │
  │  ─────────────────────────────────────────────────────────── │
  │  [0:00–0:05]  Logo intro animation (PCG laurel + globe)      │
  │  [0:05–0:12]  ANCHOR on camera: cold_open + self-intro       │
  │  [0:12–0:20]  ACTION: b-roll cue #1 + anchor VO              │
  │  [0:20–0:35]  ANCHOR on camera: headline segment             │
  │  [0:35–0:45]  ACTION: b-roll cue #2 + anchor VO              │
  │  [0:45–1:10]  ANCHOR on camera: body (sub-beat 1+2)          │
  │  [1:10–1:20]  ACTION: b-roll cue #3 + anchor VO              │
  │  [1:20–1:40]  ANCHOR on camera: brand_tiein                  │
  │  [1:40–2:00]  ANCHOR on camera: cta_signoff                  │
  │  ─────────────────────────────────────────────────────────── │
  │                                                              │
  │  OVERLAY LAYERS (FFmpeg drawtext/drawbox):                   │
  │  ┌──────────────────────────────────────────────┐            │
  │  │  [TOP CLEAR — no bar during anchor shots]    │            │
  │  │                                              │            │
  │  │         ANCHOR VIDEO / B-ROLL                │            │
  │  │                                              │            │
  │  │  ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬ ← gold line   │            │
  │  │  SAMI SATOSHI             [black/gold bg]    │            │
  │  │  PCG TECHNOLOGY CORRESPONDENT  (Cinzel)      │            │
  │  │  ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬ ← gold line   │            │
  │  │ PCG ► [scrolling ticker text ——————————▶]   │            │
  │  └──────────────────────────────────────────────┘            │
  │                                                              │
  │  FONTS: Cinzel ExtraBold (names/titles)                      │
  │         Cinzel Regular (subtitles/ticker)                    │
  │         Fira Sans Condensed (ticker body text)               │
  │  COLORS: #B8952A gold  |  #000000 black  |  #FFFFFF white    │
  └──────────────────────────────────┬───────────────────────────┘
                                     │
                              final_video.mp4
                          video_jobs.status = produced

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STAGE 6 — DISTRIBUTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Claude generates per-platform caption + hashtags from script
  → feeds existing SocialPost system (connectors already wired)
  → operator reviews in Jobs tab → one-click publish or schedule

  Platforms: LinkedIn · Instagram Reels · YouTube Shorts · TikTok · X

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
FUTURE: SCENE ACTOR MODE (Phase 2)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  For complex anchor-in-scene shots (news desk, walking, gesturing):

  FLUX Pro Ultra → studio portrait with environment backdrop
          │
  Kling AI v2.1 image-to-video → 5-10s animated scene clip
  fal-ai/kling-video/v1.6/standard/image-to-video
          │
  LatentSync lip sync pass (for speaking clips only)
  fal-ai/latentsync
          │
  Edit into final cut as anchor scene shots
```

---

## DATABASE SCHEMA — COMPLETE

### New / Updated Tables

```sql
-- ─────────────────────────────────────────────────────────────
-- anchor_profiles  (replaces avatar_profiles, richer schema)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE anchor_profiles (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id         BLOB REFERENCES organizations(id),
    created_by              BLOB REFERENCES users(id),
    name                    TEXT NOT NULL,              -- "Sami Satoshi"
    slug                    TEXT UNIQUE,                -- "sami", "nora"
    role_title              TEXT,                       -- "PCG Technology Correspondent"
    identity_doc            TEXT,                       -- JSON: character bible
    style_notes             TEXT,                       -- wardrobe, tone
    -- Portrait assets
    reference_image_url     TEXT,                       -- original upload
    portrait_v_url          TEXT,                       -- best vertical portrait
    portrait_set_urls       TEXT DEFAULT '[]',          -- JSON array of 10 angles
    era3d_views_urls        TEXT DEFAULT '[]',          -- JSON array of 3D views
    -- Hedra (PRIMARY lip sync engine)
    hedra_character_id      TEXT,                       -- initialized character
    hedra_plan              TEXT DEFAULT 'creator',     -- free|creator|pro
    -- HeyGen (fallback / talking photo)
    heygen_talking_photo_id TEXT,
    heygen_avatar_id        TEXT,
    -- ElevenLabs voice
    elevenlabs_voice_id     TEXT NOT NULL,
    voice_name              TEXT,
    -- Broadcast defaults
    lower_third_name        TEXT,
    lower_third_title       TEXT,
    default_background_url  TEXT,
    vertical                TEXT,                       -- "technology"|"finance"|"cargo"
    status                  TEXT NOT NULL DEFAULT 'pending',
    error_message           TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- ─────────────────────────────────────────────────────────────
-- video_pulse_items  (intel feed — unchanged from v1 plan)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE video_pulse_items (
    id                  BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id     BLOB REFERENCES organizations(id),
    anchor_profile_id   BLOB REFERENCES anchor_profiles(id),
    headline            TEXT NOT NULL,
    summary             TEXT,
    full_content        TEXT,
    source_url          TEXT,
    source_name         TEXT,
    relevance_score     REAL NOT NULL DEFAULT 0.0,
    sentiment           TEXT NOT NULL DEFAULT 'neutral',
    story_type          TEXT NOT NULL DEFAULT 'trending',
    vertical_tags       TEXT NOT NULL DEFAULT '[]',
    status              TEXT NOT NULL DEFAULT 'pending',
    collected_at        TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- ─────────────────────────────────────────────────────────────
-- video_scripts  (structured script segments)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE video_scripts (
    id                  BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    anchor_profile_id   BLOB NOT NULL REFERENCES anchor_profiles(id),
    pulse_item_id       BLOB REFERENCES video_pulse_items(id),
    script_type         TEXT NOT NULL DEFAULT 'tech_briefing',
    title               TEXT NOT NULL,
    -- Structured segments
    cold_open           TEXT,
    headline_segment    TEXT,
    body_segment        TEXT,
    brand_tiein         TEXT,
    cta_signoff         TEXT,
    -- Full combined script (for TTS)
    full_script         TEXT,
    -- Overlay metadata
    lower_third_name    TEXT,
    lower_third_title   TEXT,
    chyron_text         TEXT,
    ticker_items        TEXT NOT NULL DEFAULT '[]',   -- JSON array of strings
    broll_cues          TEXT NOT NULL DEFAULT '[]',   -- JSON [{timestamp, query, type}]
    scene_prompts       TEXT NOT NULL DEFAULT '[]',   -- JSON [{timestamp, prompt}]
    estimated_duration  REAL,
    status              TEXT NOT NULL DEFAULT 'draft',
    approved_at         TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- ─────────────────────────────────────────────────────────────
-- video_jobs  (production pipeline state)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE video_jobs (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    anchor_profile_id       BLOB NOT NULL REFERENCES anchor_profiles(id),
    video_script_id         BLOB REFERENCES video_scripts(id),
    created_by              BLOB REFERENCES users(id),
    -- Script (denormalized for job portability)
    script_text             TEXT NOT NULL,
    background_url          TEXT,
    -- Stage 4a: TTS
    tts_audio_url           TEXT,
    tts_wav_url             TEXT,
    word_alignment          TEXT DEFAULT '[]',
    -- Stage 4b: Hedra generation
    hedra_character_id      TEXT,
    hedra_job_id            TEXT,
    hedra_video_url         TEXT,
    -- Stage 4c: Sync.so pass (optional)
    synclabs_job_id         TEXT,
    synclabs_video_url      TEXT,
    -- Stage 5: Scene assets
    scene_assets            TEXT DEFAULT '[]',    -- JSON [{timestamp, url, type}]
    -- Stage 5: Final edit
    raw_video_url           TEXT,
    final_video_url         TEXT,
    thumbnail_url           TEXT,
    duration                REAL,
    -- Stage 6: Distribution
    social_caption          TEXT,
    social_hashtags         TEXT DEFAULT '[]',
    distributed_at          TEXT,
    -- Pipeline state
    status                  TEXT NOT NULL DEFAULT 'pending',
    error_message           TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
```

**Status progression:**
```
pending → tts_generating → tts_done → hedra_generating → hedra_done
       → [synclabs_pass] → scenes_generating → scenes_done
       → editing → produced → distributing → distributed → failed
```

---

## API ROUTES

```
/api/video-gen/anchors
  GET    → list anchor profiles
  POST   → create anchor (uploads portrait, kicks off Hedra init)

/api/video-gen/anchors/:id
  GET    → get anchor + asset URLs
  PATCH  → update anchor
  DELETE → delete anchor

/api/video-gen/pulse
  GET    → list pulse items (sorted by relevance_score DESC)
  POST   → manually add story
  POST /api/video-gen/pulse/gather → trigger intelligence collection

/api/video-gen/pulse/:id
  GET    → get pulse item
  PATCH  → update (mark scripted/skipped)

/api/video-gen/scripts
  GET    → list scripts
  POST   → generate script from pulse_item_id + anchor_profile_id

/api/video-gen/scripts/:id
  GET    → get script with all segments
  PATCH  → edit segments
  POST .../approve → approve + trigger Stage 4

/api/video-gen/jobs
  GET    → list jobs (filterable by anchor, status)
  POST   → create job from approved script

/api/video-gen/jobs/:id
  GET    → get job + all stage URLs
  DELETE → cancel job

/video-gen/audio/:job_id  (public)  → serve TTS audio for HeyGen/Hedra
/video-gen/video/:job_id  (public)  → serve final video
```

---

## CRATE STRUCTURE — video_gen/

```
crates/video_gen/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── tts.rs          ✅ DONE — ElevenLabs TTS + word alignment
    ├── heygen.rs       ✅ DONE — upload, generate, poll (keep as fallback)
    ├── hedra.rs        ← BUILD — Hedra Character-3 init + generate + poll
    ├── synclabs.rs     ← BUILD — fal.ai sync-lipsync/v2 wrapper
    ├── kling.rs        ← BUILD — fal.ai kling image-to-video wrapper
    ├── pexels.rs       ← BUILD — video search + best clip selection
    ├── intelligence.rs ← BUILD — Perplexity + Tavily + dedup + score
    ├── script_gen.rs   ← BUILD — Claude structured script generation
    └── overlay.rs      ← BUILD — FFmpeg overlay pipeline (wraps our bash scripts)
```

---

## FRONTEND — Video Studio Page

```
/video-studio
├── VideoStudioPage.tsx         — layout + tab routing
├── tabs/
│   ├── AnchorsTab.tsx          — anchor cards + create/edit modal
│   ├── PulseTab.tsx            — ranked story feed + select/skip
│   ├── ScriptsTab.tsx          — script editor (per-segment) + approve button
│   └── JobsTab.tsx             — job list + status + video player + distribute
├── components/
│   ├── AnchorCard.tsx          — portrait + voice + status
│   ├── PulseItem.tsx           — headline + score + sentiment badge
│   ├── ScriptEditor.tsx        — segment-by-segment textarea edit
│   ├── JobStatusBar.tsx        — animated pipeline progress
│   └── VideoPlayer.tsx         — inline MP4 player + download + share
```

Sidebar entry: `Clapperboard` icon → "Video Studio" under My Workspace

---

## BUILD PHASES

```
Phase 1 — Infrastructure (est. 4h)
  ├── DB migration: anchor_profiles + video_pulse_items + video_scripts
  │   + update video_jobs to new schema
  ├── DB models: anchor_profile.rs, video_pulse_item.rs, video_script.rs
  ├── hedra.rs crate module
  ├── synclabs.rs crate module
  └── Update video_gen routes to use anchor_profiles

Phase 2 — Intelligence + Script Engine (est. 4h)
  ├── intelligence.rs: Perplexity + Tavily + Claude dedup/score
  ├── script_gen.rs: Claude structured generation
  ├── /pulse/gather endpoint + cron registration
  └── /scripts POST + /scripts/:id/approve endpoints

Phase 3 — Production Pipeline Upgrade (est. 4h)
  ├── Replace HeyGen Talking Photo → Hedra in produce_job()
  ├── Add optional Sync.so pass after Hedra
  ├── kling.rs for scene generation
  ├── pexels.rs for b-roll fetch
  └── overlay.rs: encode our proven FFmpeg pipeline as Rust function

Phase 4 — Frontend (est. 5h)
  ├── VideoStudioPage + 4 tabs
  ├── Sidebar entry
  ├── All API bindings in video-gen.ts
  └── JobStatusBar live polling

Phase 5 — Distribution Wiring (est. 2h)
  ├── Script → per-platform caption generation
  ├── VideoJob → SocialPost bridge
  └── Jobs tab distribute button

Phase 6 — TIACA Automation (est. 3h)
  ├── Vertical keywords config per anchor
  ├── Daily cron at 06:00 UTC
  └── Auto-generate script from top-scored pulse item
```

---

## TOOL COSTS (per 2-minute video)

| Stage | Tool | Cost |
|-------|------|------|
| TTS | ElevenLabs turbo_v2_5 | ~$0.08 |
| Anchor video | Hedra Character-3 Creator plan | ~$0.72 |
| Lip sync pass | Sync.so (optional) | ~$6.00 |
| Scene shots | Pexels (free) | $0 |
| Scene shots | Kling AI (premium, 3 clips) | ~$0.84 |
| Intelligence | Perplexity sonar-pro | ~$0.05 |
| Total (basic) | No Sync.so, Pexels b-roll | ~$0.85 |
| Total (premium) | With Sync.so + Kling | ~$7.69 |

---

## WHAT WAS BUILT IN PROTOTYPE (2026-03-29/30)

### Code committed / to commit:
- `crates/video_gen/` — heygen.rs, tts.rs, mod.rs
- `crates/db/migrations/20260426000000_video_studio.sql`
- `crates/db/src/models/avatar_profile.rs`
- `crates/db/src/models/video_job.rs`
- `crates/server/src/routes/video_gen.rs`
- `crates/nora/src/tools/` — GenerateImage, ApplyVideoEffect, PostProcessVideoJob

### Assets produced:
- `dev_assets/video_gen/portraits/sami_news_v2.jpg` — Sami Satoshi news anchor portrait
- `dev_assets/video_gen/portraits/nora_portrait_v4_raw.png` — Nora canonical portrait
- `dev_assets/video_gen/portraits/nora_era3d_4k_*.png` — 3D reconstruction set
- `dev_assets/video_gen/pcg_logo.png` — PCG brand asset
- `dev_assets/fonts/cinzel_*.ttf` — Brand fonts
- `dev_assets/video_gen/video/sami_pcg_techbrief_v3.mp4` — First produced segment

### Key learnings:
1. HeyGen Talking Photo lip sync quality is insufficient — migrate to Hedra
2. Scene generation (Kling) is required for production-quality cuts — not optional
3. The FFmpeg overlay pipeline is solid and should be codified in overlay.rs
4. Audio VO continuity across b-roll cuts is critical — video track / audio track must be built separately and muxed
5. Cinzel font + black/gold + PCG logo = strong brand identity confirmed
6. Era3D 3D reconstruction works but upscale step introduces illustration artifacts — use original portraits for Hedra input

---

*Next session: begin Phase 1 implementation — DB migration + Hedra integration*
