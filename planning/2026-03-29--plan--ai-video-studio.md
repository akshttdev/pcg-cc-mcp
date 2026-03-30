# AI Video Studio — Implementation Plan
**Date:** 2026-03-29
**Scope:** PCG Dashboard — Full AI Video Generation Pipeline
**Primary Use Case:** Nora VSL + TIACA Air Industry News Reports
**Priority:** PCG Product Feature

---

## TOOL STACK SELECTION

### Best-in-class choices with most documentation + customization:

| Stage | Tool | Why Chosen | API Docs |
|-------|------|------------|----------|
| Talking Head Video | **HeyGen v2** | Best lip sync, reference image → custom avatar, REST API, 1080p, background swap | api.heygen.com/v2 |
| Voice | **ElevenLabs** | Already in stack, voice clone from samples, broadcast quality | api.elevenlabs.io/v1 |
| Post-Production | **Creatomate** | JSON-driven template rendering, dynamic text injection, full layer system, best docs | api.creatomate.com/v1 |
| News Intelligence | **Perplexity API** | Real-time web search with citations, best for "what's happening now" | api.perplexity.ai |
| News Intelligence (2) | **Tavily API** | Search-optimized for agents, structured output | api.tavily.com |
| Script Generation | **Claude claude-opus-4-6** | Already in stack | ✅ |
| B-Roll | **Pexels API** | Free, large library, REST API, no watermark | api.pexels.com/videos |
| Distribution | **Existing social connectors** | LinkedIn, Instagram, YouTube, Twitter, TikTok already wired | ✅ |

---

## SYSTEM ARCHITECTURE

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║                        PCG AI VIDEO STUDIO — SYSTEM MAP                        ║
╠══════════════════════════════════════════════════════════════════════════════════╣
║                                                                                 ║
║  ┌─────────────────────────────────────────────────────────────────────────┐   ║
║  │  DASHBOARD UI  (frontend/src/pages/video-studio/)                       │   ║
║  │                                                                         │   ║
║  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │   ║
║  │  │  Avatars Tab │  │  Pulse Tab   │  │  Scripts Tab │  │  Jobs Tab  │ │   ║
║  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘ │   ║
║  └─────────┼─────────────────┼─────────────────┼────────────────┼────────┘   ║
║            │                 │                 │                │             ║
║  ┌─────────▼─────────────────▼─────────────────▼────────────────▼────────┐   ║
║  │  REST API  /api/video-gen/*                                            │   ║
║  │                                                                        │   ║
║  │  /avatars         /pulse          /scripts        /jobs                │   ║
║  │  POST  GET        POST  GET       POST  GET       POST  GET            │   ║
║  │  GET :id          GET :id         GET :id         GET :id              │   ║
║  │  PATCH :id        PATCH :id       PATCH :id       PATCH :id           │   ║
║  │  DELETE :id                       POST :id/approve POST :id/produce   │   ║
║  └─────────┬─────────────────────────────────────┬────────────────────────┘   ║
║            │                                     │                             ║
║  ┌─────────▼─────────────────┐   ┌───────────────▼──────────────────────────┐ ║
║  │  crates/video_gen/        │   │  crates/db/src/models/                   │ ║
║  │                           │   │                                          │ ║
║  │  mod.rs (VideoGenService) │   │  avatar_profile.rs                       │ ║
║  │  heygen.rs                │   │  video_pulse_item.rs                     │ ║
║  │  creatomate.rs            │   │  video_script.rs                         │ ║
║  │  script_gen.rs            │   │  video_job.rs                            │ ║
║  │  intelligence.rs          │   │                                          │ ║
║  │  voice.rs (ElevenLabs)    │   └──────────────────────────────────────────┘ ║
║  │  broll.rs (Pexels)        │                                                ║
║  └─────────┬─────────────────┘                                                ║
║            │                                                                   ║
║  ┌─────────▼─────────────────────────────────────────────────────────────────┐ ║
║  │  EXTERNAL SERVICES                                                         │ ║
║  │                                                                            │ ║
║  │  HeyGen         ElevenLabs      Creatomate       Perplexity    Pexels     │ ║
║  │  (talking head) (voice TTS)     (post-prod)      (news intel)  (b-roll)   │ ║
║  └────────────────────────────────────────────────────────────────────────────┘ ║
╚══════════════════════════════════════════════════════════════════════════════════╝
```

---

## VIDEO PIPELINE FLOW

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║                          PIPELINE — STAGE BY STAGE                             ║
╚══════════════════════════════════════════════════════════════════════════════════╝

  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  STAGE 1 — AVATAR SETUP  (one-time per persona)                             │
  │                                                                             │
  │  USER INPUT:                  SYSTEM PROCESS:              OUTPUT:          │
  │  ┌──────────────────┐         ┌──────────────────────┐    ┌──────────────┐ │
  │  │ • Avatar name    │         │ Claude writes         │    │ AvatarProfile│ │
  │  │ • Brand brief    │──────▶  │ character bible:      │───▶│ stored in DB │ │
  │  │ • Tone/style     │         │  - personality        │    │              │ │
  │  │ • Reference      │         │  - speaking style     │    │ heygen_id    │ │
  │  │   images (upload)│         │  - catchphrases       │    │ voice_id     │ │
  │  │ • Voice samples  │         │  - visual notes       │    │ thumbnail    │ │
  │  └──────────────────┘         └──────────┬───────────┘    └──────────────┘ │
  │                                          │                                  │
  │                               ┌──────────▼───────────┐                     │
  │                               │ HeyGen Instant Avatar │                     │
  │                               │ POST /v2/avatar/      │                     │
  │                               │ upload_reference_img  │                     │
  │                               │ → avatar_id stored    │                     │
  │                               └──────────┬───────────┘                     │
  │                                          │                                  │
  │                               ┌──────────▼───────────┐                     │
  │                               │ ElevenLabs Voice Clone│                     │
  │                               │ POST /v1/voices/add   │                     │
  │                               │ → voice_id stored     │                     │
  │                               └──────────────────────┘                     │
  └─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  STAGE 2 — INTELLIGENCE GATHERING  (cron daily 6am OR on-demand)           │
  │                                                                             │
  │  ┌─────────────────────────────────────────────────────────────────────┐   │
  │  │  Parallel search agents:                                            │   │
  │  │                                                                     │   │
  │  │  Perplexity API ──────────────────────────────────┐                 │   │
  │  │  POST /chat/completions                           │                 │   │
  │  │  model: sonar-pro, search_recency: day            │                 │   │
  │  │                                                   │                 │   │
  │  │  Tavily API ─────────────────────────────────────▶│  Claude Agent   │   │
  │  │  POST /search                                     │  deduplicates   │   │
  │  │  search_depth: advanced, include_answer: true     │  + scores       │   │
  │  │                                                   │  + tags         │   │
  │  │  RSS Feeds (vertical-specific) ─────────────────▶│                 │   │
  │  │  Polled via reqwest XML parser                    │                 │   │
  │  └─────────────────────────────────────┬─────────────┘                 │   │
  │                                        │                               │   │
  │                                        ▼                               │   │
  │                     ┌─────────────────────────────┐                   │   │
  │                     │ video_pulse_items table      │                   │   │
  │                     │ • headline, summary, url     │                   │   │
  │                     │ • relevance_score (0-10)     │                   │   │
  │                     │ • sentiment, story_type      │                   │   │
  │                     │ • status: pending            │                   │   │
  │                     └─────────────────────────────┘                   │   │
  │                                                                        │   │
  │  Dashboard UI: "Pulse Feed" shows ranked stories → operator selects   │   │
  └─────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼ (story selected OR VSL request)
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  STAGE 3 — SCRIPT GENERATION                                                │
  │                                                                             │
  │  Input: selected pulse item + avatar identity doc + brand voice            │
  │                                                                             │
  │  Claude claude-opus-4-6 generates structured VideoScript:                  │
  │  ┌──────────────────────────────────────────────────────────────────┐      │
  │  │  cold_open     (0:00-0:08)   "Breaking in [industry]..."         │      │
  │  │  headline      (0:08-0:20)   "[Company X] announces..."          │      │
  │  │  body          (0:20-1:20)   Full story + analysis               │      │
  │  │  brand_tiein   (1:20-1:40)   "What this means for [audience]..." │      │
  │  │  cta_signoff   (1:40-2:00)   CTA + avatar sign-off               │      │
  │  │  lower_third_name / lower_third_title                            │      │
  │  │  chyron_text                                                     │      │
  │  │  broll_cues[]  → fed to Pexels search in Stage 5                 │      │
  │  └──────────────────────────────────────────────────────────────────┘      │
  │                                                                             │
  │  Human review: operator edits segments in Script Editor UI                 │
  │  → POST /api/video-gen/scripts/:id/approve                                 │
  └─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼ (on script approval)
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  STAGE 4 — VIDEO GENERATION (HeyGen + ElevenLabs)                          │
  │                                                                             │
  │  ┌──────────────────────────────────────────────────────────────────────┐  │
  │  │  Step 4a: ElevenLabs TTS (already in stack)                         │  │
  │  │  POST /v1/text-to-speech/{voice_id}                                 │  │
  │  │  • model_id: eleven_turbo_v2_5 (lowest latency, broadcast quality)  │  │
  │  │  • voice_settings: { stability: 0.5, similarity_boost: 0.75,        │  │
  │  │                       style: 0.5, use_speaker_boost: true }         │  │
  │  │  → Output: MP3/WAV saved to local storage                           │  │
  │  └──────────────────────────────────────────────┬───────────────────────┘  │
  │                                                 │                           │
  │  ┌──────────────────────────────────────────────▼───────────────────────┐  │
  │  │  Step 4b: HeyGen Talking Head                                        │  │
  │  │  POST /v2/video/generate                                             │  │
  │  │  {                                                                   │  │
  │  │    "video_inputs": [{                                                │  │
  │  │      "character": {                                                  │  │
  │  │        "type": "avatar",                                             │  │
  │  │        "avatar_id": "{heygen_avatar_id}",                           │  │
  │  │        "avatar_style": "normal"                                      │  │
  │  │      },                                                              │  │
  │  │      "voice": {                                                      │  │
  │  │        "type": "audio",                                              │  │
  │  │        "audio_url": "{elevenlabs_output_url}"  ← lip-syncs to this  │  │
  │  │      },                                                              │  │
  │  │      "background": {                                                 │  │
  │  │        "type": "image",                                              │  │
  │  │        "url": "{studio_background_url}"                              │  │
  │  │      }                                                               │  │
  │  │    }],                                                               │  │
  │  │    "dimension": { "width": 1920, "height": 1080 },                  │  │
  │  │    "aspect_ratio": null                                              │  │
  │  │  }                                                                   │  │
  │  │                                                                      │  │
  │  │  Poll: GET /v1/video_status.get?video_id={id}                       │  │
  │  │  Download: GET video_url when status = "completed"                  │  │
  │  │  Latency: 3-8 min                                                   │  │
  │  └──────────────────────────────────────────────────────────────────────┘  │
  └─────────────────────────────────────────────────────────────────────────────┘
                                        │ raw MP4
                                        ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  STAGE 5 — POST-PRODUCTION (Creatomate + Pexels)                           │
  │                                                                             │
  │  ┌──────────────────────────────────────────────────────────────────────┐  │
  │  │  B-Roll Fetch (optional, high production value)                      │  │
  │  │  Pexels API: GET /videos/search?query={broll_cue}&per_page=3         │  │
  │  │  → Select best match per cue → download clip URLs                    │  │
  │  └──────────────────────────────────────────────────────────────────────┘  │
  │                                                                             │
  │  ┌──────────────────────────────────────────────────────────────────────┐  │
  │  │  Creatomate Render  POST /v1/renders                                 │  │
  │  │  {                                                                   │  │
  │  │    "template_id": "{news_template_id}",                              │  │
  │  │    "modifications": {                                                │  │
  │  │      "Anchor-Video": { "source": "{heygen_video_url}" },            │  │
  │  │      "Brand-Logo": { "source": "{org_logo_url}" },                  │  │
  │  │      "Headline-Text": { "text": "{chyron_text}" },                  │  │
  │  │      "Lower-Third-Name": { "text": "{lower_third_name}" },          │  │
  │  │      "Lower-Third-Title": { "text": "{lower_third_title}" },        │  │
  │  │      "Ticker-Text": { "text": "{ticker_content}" },                 │  │
  │  │      "Background-Music": { "source": "{music_url}", "volume": 0.15} │  │
  │  │    },                                                                │  │
  │  │    "output_format": "mp4"                                            │  │
  │  │  }                                                                   │  │
  │  │                                                                      │  │
  │  │  Visual layout:                                                      │  │
  │  │  ┌──────────────────────────────────────────────────────────┐       │  │
  │  │  │ [BRAND LOGO]                          [LIVE  16:42 GMT]  │       │  │
  │  │  │                                                          │       │  │
  │  │  │              ┌──────────────────┐                        │       │  │
  │  │  │              │                  │                        │       │  │
  │  │  │              │  ANCHOR VIDEO    │                        │       │  │
  │  │  │              │  (HeyGen output) │                        │       │  │
  │  │  │              └──────────────────┘                        │       │  │
  │  │  │                                                          │       │  │
  │  │  │  ┌──────────────────────────────────────────────────┐   │       │  │
  │  │  │  │ BREAKING: [CHYRON TEXT]          [BRAND] NEWS   │   │       │  │
  │  │  │  │ [Lower Third: Anchor Name | Title]               │   │       │  │
  │  │  │  └──────────────────────────────────────────────────┘   │       │  │
  │  │  └──────────────────────────────────────────────────────────┘       │  │
  │  │                                                                      │  │
  │  │  Poll: GET /v1/renders/{id}  → download when status="succeeded"     │  │
  │  └──────────────────────────────────────────────────────────────────────┘  │
  └─────────────────────────────────────────────────────────────────────────────┘
                                        │ final broadcast MP4
                                        ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  STAGE 6 — DISTRIBUTION (existing social connectors)                       │
  │                                                                             │
  │  Claude generates per-platform caption + hashtags                          │
  │  → Feeds into existing SocialPost system                                   │
  │  → Existing publish automations handle scheduling                          │
  │                                                                             │
  │  Platforms: LinkedIn, Instagram (Reels), YouTube, X/Twitter, TikTok        │
  └─────────────────────────────────────────────────────────────────────────────┘
```

---

## DATABASE SCHEMA

### Migration: `crates/db/migrations/20260329000000_video_studio.sql`

```sql
-- ─────────────────────────────────────────────────
-- Avatar Profiles  (one per persona/brand)
-- ─────────────────────────────────────────────────
CREATE TABLE avatar_profiles (
    id                   BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id      BLOB REFERENCES organizations(id),
    created_by           BLOB REFERENCES users(id),
    name                 TEXT NOT NULL,
    slug                 TEXT UNIQUE,                  -- "nora", "tiaca-anchor"
    identity_doc         TEXT,                         -- JSON: character bible
    style_notes          TEXT,                         -- wardrobe, tone, setting
    reference_image_url  TEXT,                         -- uploaded source image
    thumbnail_url        TEXT,                         -- preview
    -- HeyGen
    heygen_avatar_id     TEXT,
    heygen_avatar_type   TEXT DEFAULT 'instant',       -- instant|studio|custom
    -- ElevenLabs
    elevenlabs_voice_id  TEXT,
    voice_sample_url     TEXT,
    -- Defaults for video generation
    default_background_url TEXT,
    default_template_id    TEXT,                       -- Creatomate template ID
    status               TEXT NOT NULL DEFAULT 'pending',  -- pending|generating|active|failed
    error_message        TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_avatar_profiles_org ON avatar_profiles(organization_id);

-- ─────────────────────────────────────────────────
-- Video Intelligence Pulse  (news gathering)
-- ─────────────────────────────────────────────────
CREATE TABLE video_pulse_items (
    id                   BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id      BLOB REFERENCES organizations(id),
    avatar_profile_id    BLOB REFERENCES avatar_profiles(id),
    headline             TEXT NOT NULL,
    summary              TEXT,
    full_content         TEXT,
    source_url           TEXT,
    source_name          TEXT,
    relevance_score      REAL NOT NULL DEFAULT 0.0,
    sentiment            TEXT NOT NULL DEFAULT 'neutral',
    story_type           TEXT NOT NULL DEFAULT 'trending',
    vertical_tags        TEXT NOT NULL DEFAULT '[]',   -- JSON
    status               TEXT NOT NULL DEFAULT 'pending',  -- pending|scripted|produced|distributed|skipped
    collected_at         TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_pulse_org ON video_pulse_items(organization_id);
CREATE INDEX idx_pulse_status ON video_pulse_items(status);

-- ─────────────────────────────────────────────────
-- Video Scripts
-- ─────────────────────────────────────────────────
CREATE TABLE video_scripts (
    id                   BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    avatar_profile_id    BLOB NOT NULL REFERENCES avatar_profiles(id),
    pulse_item_id        BLOB REFERENCES video_pulse_items(id),  -- NULL for VSL
    script_type          TEXT NOT NULL DEFAULT 'news_report',    -- news_report|vsl|announcement|explainer
    title                TEXT NOT NULL,
    -- Structured segments
    cold_open            TEXT,
    headline_segment     TEXT,
    body_segment         TEXT,
    brand_tiein          TEXT,
    cta_signoff          TEXT,
    -- Overlay metadata
    lower_third_name     TEXT,
    lower_third_title    TEXT,
    chyron_text          TEXT,
    ticker_text          TEXT,
    broll_cues           TEXT NOT NULL DEFAULT '[]',  -- JSON: [{cue, timestamp_hint}]
    graphic_specs        TEXT NOT NULL DEFAULT '{}',  -- JSON
    -- Assembled
    full_script          TEXT,
    estimated_duration_s REAL DEFAULT 120.0,
    -- Lifecycle
    status               TEXT NOT NULL DEFAULT 'draft',  -- draft|approved|producing|done
    approved_by          BLOB REFERENCES users(id),
    approved_at          TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_scripts_avatar ON video_scripts(avatar_profile_id);

-- ─────────────────────────────────────────────────
-- Video Jobs  (one per produced video)
-- ─────────────────────────────────────────────────
CREATE TABLE video_jobs (
    id                   BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    avatar_profile_id    BLOB NOT NULL REFERENCES avatar_profiles(id),
    script_id            BLOB REFERENCES video_scripts(id),
    project_id           BLOB REFERENCES projects(id),
    created_by           BLOB REFERENCES users(id),
    job_type             TEXT NOT NULL DEFAULT 'talking_head',

    -- Stage 4: ElevenLabs TTS
    tts_audio_url        TEXT,
    tts_status           TEXT DEFAULT 'pending',

    -- Stage 4: HeyGen
    heygen_video_id      TEXT,
    heygen_status        TEXT,
    raw_video_url        TEXT,

    -- Stage 5: Creatomate post-production
    creatomate_render_id TEXT,
    creatomate_status    TEXT,
    final_video_url      TEXT,
    thumbnail_url        TEXT,
    duration_seconds     REAL,

    -- Stage 6: Distribution
    distribution_targets TEXT NOT NULL DEFAULT '[]',   -- JSON: ["linkedin","instagram","youtube"]
    distribution_status  TEXT NOT NULL DEFAULT '{}',   -- JSON: { platform: "pending|sent|failed" }

    -- Overall
    status               TEXT NOT NULL DEFAULT 'pending',
    -- pending → tts_generating → avatar_generating → post_producing → ready → distributing → done | failed
    error_message        TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_video_jobs_avatar ON video_jobs(avatar_profile_id);
CREATE INDEX idx_video_jobs_status ON video_jobs(status);
```

---

## BACKEND CRATE: `crates/video_gen/`

```
crates/video_gen/
├── Cargo.toml
└── src/
    ├── lib.rs              → VideoGenService trait + public re-exports
    ├── config.rs           → VideoGenConfig { heygen_api_key, creatomate_api_key,
    │                                          perplexity_api_key, tavily_api_key,
    │                                          pexels_api_key }
    ├── heygen.rs           → HeyGenClient
    │   POST /v2/video/generate
    │   GET  /v1/video_status.get
    │   POST /v2/avatar/upload (instant avatar from reference)
    │
    ├── creatomate.rs       → CreatomateClient
    │   POST /v1/renders
    │   GET  /v1/renders/{id}
    │   POST /v1/templates (manage templates)
    │
    ├── script_gen.rs       → ScriptGenAgent
    │   build_prompt(pulse_item, avatar_identity, brand_voice)
    │   parse_response → VideoScript segments
    │   assemble_full_script() → single string for TTS
    │
    ├── intelligence.rs     → PulseAgent
    │   run_perplexity_search(vertical, keywords)
    │   run_tavily_search(query)
    │   deduplicate_and_score(Vec<RawStory>) → Vec<VideoP ulseItem>
    │
    ├── voice.rs            → wraps existing ElevenLabs TTS in crates/nora/src/voice/tts.rs
    │   generate_tts(script_text, voice_id) → audio_url
    │
    └── broll.rs            → PexelsClient
        search_videos(query, per_page) → Vec<BrollClip>
```

### New Route File: `crates/server/src/routes/video_gen.rs`

```
GET    /video-gen/avatars                     → list avatars (org-scoped)
POST   /video-gen/avatars                     → create avatar profile
GET    /video-gen/avatars/:id                 → get avatar + status
PATCH  /video-gen/avatars/:id                 → update avatar
DELETE /video-gen/avatars/:id                 → delete
POST   /video-gen/avatars/:id/generate-heygen → trigger HeyGen instant avatar creation

GET    /video-gen/pulse                       → list pulse items (org-scoped, paginated)
POST   /video-gen/pulse/gather                → manual trigger intelligence run
PATCH  /video-gen/pulse/:id                   → skip or select item

POST   /video-gen/scripts                     → create script (auto-generates via Claude)
GET    /video-gen/scripts/:id                 → get script
PATCH  /video-gen/scripts/:id                 → edit script segments
POST   /video-gen/scripts/:id/approve         → approve → auto-creates video_job

GET    /video-gen/jobs                        → list jobs (org-scoped, paginated)
POST   /video-gen/jobs                        → create job manually
GET    /video-gen/jobs/:id                    → get job + stage status
POST   /video-gen/jobs/:id/produce            → trigger pipeline from approved script
POST   /video-gen/jobs/:id/distribute         → trigger distribution
DELETE /video-gen/jobs/:id                    → cancel/delete
```

### Registration in `mod.rs`:
```rust
pub mod video_gen;
// in protected_router():
.merge(video_gen::router())
```

### ENV variables to add to `.env`:
```
HEYGEN_API_KEY=
CREATOMATE_API_KEY=
PERPLEXITY_API_KEY=
TAVILY_API_KEY=
PEXELS_API_KEY=
CREATOMATE_NEWS_TEMPLATE_ID=    # template pre-built in Creatomate dashboard
```

---

## FRONTEND — WHERE EVERYTHING LIVES IN THE DASHBOARD

### 1. New Global Page: `/video-studio`

**File:** `frontend/src/pages/video-studio/index.tsx`

```
frontend/src/pages/video-studio/
├── index.tsx          → VideoStudioPage (shell + tabs)
├── AvatarsTab.tsx     → Avatar manager
├── PulseTab.tsx       → News intelligence feed
├── ScriptsTab.tsx     → Script browser + editor
└── JobsTab.tsx        → Video job tracker
```

**Page Layout:**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│  🎬 VIDEO STUDIO                         [Org Selector ▼]  [+ New Video]    │
├──────────────────────────────────────────────────────────────────────────────┤
│  [Avatars]  [Pulse Feed]  [Scripts]  [Jobs]                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  <<tab content>>                                                             │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 2. Avatars Tab

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  AVATARS                                                        [+ New Avatar]│
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌──────────────────────┐ │
│  │  [thumbnail]        │  │  [thumbnail]        │  │  + Create Avatar     │ │
│  │  ● Nora             │  │  ○ TIACA Anchor      │  │                      │ │
│  │  PCG · Active       │  │  TIACA · Generating  │  │  Upload reference    │ │
│  │  HeyGen: av_xxx     │  │  HeyGen: pending     │  │  image to start      │ │
│  │  Voice: ElevenLabs  │  │  Voice: pending      │  │                      │ │
│  │  [Use] [Edit]       │  │  [View Progress]     │  │                      │ │
│  └─────────────────────┘  └─────────────────────┘  └──────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Create Avatar Drawer (slide-in from right):**
```
┌────────────────────────────────────────────┐
│  NEW AVATAR PROFILE                   [✕]  │
├────────────────────────────────────────────┤
│                                            │
│  Name *          [Nora              ]      │
│  Org             [Sirak Studios   ▼]      │
│                                            │
│  Reference Image *                         │
│  ┌─────────────────────────────────────┐   │
│  │    Drag & drop photo here            │   │
│  │    or click to browse               │   │
│  │    (JPG/PNG, face clearly visible)  │   │
│  └─────────────────────────────────────┘   │
│                                            │
│  Voice Sample (optional)                   │
│  ┌─────────────────────────────────────┐   │
│  │    Upload MP3/WAV (30-60 seconds)   │   │
│  └─────────────────────────────────────┘   │
│                                            │
│  ── Identity (Claude will generate) ──     │
│  Brand Brief *   [textarea                ]│
│  Tone            [◉ Authoritative  ○ Warm ]│
│                  [○ Casual  ○ Energetic   ]│
│  Style Notes     [textarea - wardrobe etc ]│
│                                            │
│  Default Background URL                    │
│                  [https://...           ]  │
│                                            │
│            [Cancel]  [Generate Avatar →]   │
└────────────────────────────────────────────┘
```

---

### 3. Pulse Feed Tab

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  PULSE FEED  — Air Cargo Industry                 [Gather Now]  [Filter ▼]   │
│  Last run: 2 hours ago · 14 stories collected                                │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  🔴 BREAKING  ████████████████████ 9.2  TIACA Aviation                │  │
│  │  IATA reports record cargo volumes in Q1 2026                         │  │
│  │  Source: IATA.org · 2h ago  · positive                                │  │
│  │  [Generate Script →]  [Skip]                                          │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  📈 TRENDING  ███████████████░░░░░ 7.8  Air Cargo News                │  │
│  │  New e-commerce fulfillment partnerships reshape last-mile logistics   │  │
│  │  Source: CargoNews.aero · 5h ago  · neutral                           │  │
│  │  [Generate Script →]  [Skip]                                          │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  ♻️ EVERGREEN  ████████████░░░░░░░ 6.1  FreightWaves                  │  │
│  │  How AI is transforming air freight pricing models                    │  │
│  │  Source: freightwaves.com · 1d ago  · positive  · ✅ scripted         │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 4. Scripts Tab

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  SCRIPTS                                     [Filter: All ▼]  [+ New Script] │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  📄 Nora VSL — Introducing Herself            draft    2026-03-29      │  │
│  │  Type: VSL · Avatar: Nora · ~120s                                      │  │
│  │  [Edit & Approve →]                                                    │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  📄 IATA Q1 2026 Cargo Volume Report          approved  2026-03-29     │  │
│  │  Type: News Report · Avatar: TIACA Anchor · ~120s                      │  │
│  │  [Produce Video →]                                                     │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Script Editor Drawer:**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│  SCRIPT: Nora VSL — Introducing Herself                                 [✕]  │
├──────────────────────────────────────────────────────────────────────────────┤
│  Type: VSL  · Avatar: Nora  · Est. 2:00                                      │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  COLD OPEN  (0:00 – 0:08)                                                   │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ "Hello, I'm Nora — and I was built different."                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  HEADLINE  (0:08 – 0:20)                                                    │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ "I'm an AI executive assistant — and today I'm here to show you..."   │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  BODY  (0:20 – 1:20)                                                        │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ [textarea — editable]                                                  │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  BRAND TIE-IN  (1:20 – 1:40)                                                │
│  [textarea]                                                                  │
│                                                                              │
│  CTA + SIGNOFF  (1:40 – 2:00)                                               │
│  [textarea]                                                                  │
│                                                                              │
│  ─── Overlay Data ───────────────────────────────────────────────────────    │
│  Lower Third Name   [Nora                    ]                               │
│  Lower Third Title  [AI Executive Assistant  ]                               │
│  Chyron             [AI-POWERED · POWERCLUB GLOBAL]                         │
│  Ticker             [PCG.io · Join the future of work...]                    │
│                                                                              │
│  [Regenerate]         [Save Draft]         [✓ Approve & Create Job]          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 5. Jobs Tab

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  VIDEO JOBS                                              [Filter: Active ▼]   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  🎬  Nora VSL — Introducing Herself                                    │  │
│  │                                                                        │  │
│  │  [●──────────────────────────────────○──────────]                     │  │
│  │   TTS ✓  Avatar ✓  Post-Prod ⟳  Ready  Distribute                    │  │
│  │                                                                        │  │
│  │  HeyGen: completed  ·  Creatomate: rendering...                        │  │
│  │                                                                        │  │
│  │  [Preview Raw]  [Preview Final (when ready)]  [Distribute →]          │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  🎬  IATA Q1 Cargo Report                                              │  │
│  │                                                                        │  │
│  │  [●────●────●────●────○──────────]                                    │  │
│  │   TTS ✓  Avatar ✓  Post-Prod ✓  Ready  Distribute                    │  │
│  │                                                                        │  │
│  │  ┌──────────────────────────────────────────────────────────────┐     │  │
│  │  │  [▶ PLAY FINAL VIDEO - 2:03]                                  │     │  │
│  │  └──────────────────────────────────────────────────────────────┘     │  │
│  │                                                                        │  │
│  │  Distribute to:                                                        │  │
│  │  [✓ LinkedIn]  [✓ Instagram]  [✓ YouTube]  [○ TikTok]  [○ X]         │  │
│  │  [Schedule: 2026-03-30 09:00 ▼]                 [Distribute Now]      │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 6. Sidebar Navigation Addition

**File:** `frontend/src/components/layout/sidebar/constants.ts`

Add to the **My Workspace** section (between Intelligence and My Projects):

```typescript
{
  label: 'Video Studio',
  icon: Clapperboard,        // from lucide-react
  to: '/video-studio',
  roles: ['admin', 'org_editor'],
  badge: pendingJobsCount,   // optional: count of in-progress jobs
}
```

Also add to **Admin Platforms** section for cross-org admin view:
```typescript
{
  label: 'Video Studio',
  icon: Clapperboard,
  to: '/video-studio',
  roles: ['admin'],
}
```

---

### 7. App.tsx Route Addition

```tsx
// Video Studio
<Route
  path="/video-studio"
  element={
    <ProtectedRoute requiredRole="org_editor">
      <VideoStudioPage />
    </ProtectedRoute>
  }
/>
```

---

### 8. API Module Addition

**File:** `frontend/src/lib/api/video-gen.ts`

```typescript
export const videoGenApi = {
  // Avatars
  listAvatars(orgId?: string): Promise<AvatarProfile[]>
  createAvatar(data: CreateAvatarProfile): Promise<AvatarProfile>
  getAvatar(id: string): Promise<AvatarProfile>
  updateAvatar(id: string, data: Partial<AvatarProfile>): Promise<AvatarProfile>
  deleteAvatar(id: string): Promise<void>
  generateHeygenAvatar(id: string): Promise<AvatarProfile>

  // Pulse
  listPulse(orgId?: string, params?: PulseListParams): Promise<VideoPulseItem[]>
  gatherPulse(avatarProfileId: string): Promise<{ count: number }>
  updatePulseItem(id: string, data: { status: string }): Promise<VideoPulseItem>

  // Scripts
  listScripts(params?: ScriptListParams): Promise<VideoScript[]>
  createScript(data: CreateVideoScript): Promise<VideoScript>
  getScript(id: string): Promise<VideoScript>
  updateScript(id: string, data: Partial<VideoScript>): Promise<VideoScript>
  approveScript(id: string): Promise<VideoJob>    // returns the created job

  // Jobs
  listJobs(params?: JobListParams): Promise<VideoJob[]>
  getJob(id: string): Promise<VideoJob>
  produceJob(id: string): Promise<VideoJob>
  distributeJob(id: string, data: DistributeJobRequest): Promise<VideoJob>
  deleteJob(id: string): Promise<void>
}
```

---

## NORA VSL — FIRST VIDEO SPEC

This is the immediate deliverable: **Nora introduces herself and sells video generation services.**

**Avatar Setup:**
- Name: `Nora`
- Reference image: PCG's Nora visual (to be provided)
- Voice: existing ElevenLabs voice (Mia Moore or cloned Nora voice)
- Background: professional studio setting (dark/gold matching PCG brand)
- HeyGen avatar type: `instant` (from reference photo)

**Script Type:** `vsl`

**Script Structure:**
```
COLD OPEN (0:00-0:08)
"Hello — I'm Nora. Built by PowerClub Global."

HEADLINE (0:08-0:20)
"I handle your executive workflow, run your research, manage your team —
and today, I'm here to show you something brand new."

BODY (0:20-1:20)
"Every brand needs a voice. A consistent presence. Someone who shows up
every single day with news, insights, and stories your audience actually
cares about. Until now, that took a full production team.

With PCG's AI Video Studio, you get a photorealistic anchor — trained on
your brand, your voice, your story — delivering broadcast-quality content
on autopilot. News reports. Product launches. Educational content.
Generated in minutes, not weeks."

BRAND TIE-IN (1:20-1:40)
"Whether you're an enterprise brand, a media company, or a creator —
we build the avatar, write the scripts, and produce the video.
You just approve and publish."

CTA + SIGNOFF (1:40-2:00)
"This video? It was made with the exact system I'm describing.
Talk to us at PowerClubGlobal.com. I'm Nora — and this is just the beginning."
```

**Overlays:**
- Lower Third: `Nora | AI Executive Assistant`
- Chyron: `AI-POWERED VIDEO · POWERCLUBGLOBAL.COM`
- Brand Logo: PCG logo (top left)

---

## IMPLEMENTATION PHASES

```
╔═════════════════════════════════════════════════════════════════╗
║  PHASE 1 — Foundation  (DB + API clients + basic routes)        ║
║  Est: 4-5 hours                                                 ║
╠═════════════════════════════════════════════════════════════════╣
║  P1.1  SQL migration (4 tables)                                 ║
║  P1.2  DB models (avatar_profile, video_pulse_item,             ║
║         video_script, video_job)                                ║
║  P1.3  crates/video_gen/src/config.rs + lib.rs                 ║
║  P1.4  crates/video_gen/src/heygen.rs                          ║
║  P1.5  crates/video_gen/src/creatomate.rs                      ║
║  P1.6  crates/video_gen/src/intelligence.rs (Perplexity+Tavily)║
║  P1.7  crates/video_gen/src/broll.rs (Pexels)                  ║
║  P1.8  video_gen.rs routes (CRUD only, no pipeline yet)         ║
║  P1.9  Register in mod.rs + add Cargo.toml dep                  ║
╠═════════════════════════════════════════════════════════════════╣
║  PHASE 2 — Script Engine + Avatar Creation                      ║
║  Est: 3-4 hours                                                 ║
╠═════════════════════════════════════════════════════════════════╣
║  P2.1  crates/video_gen/src/script_gen.rs                      ║
║  P2.2  POST /video-gen/avatars/:id/generate-heygen              ║
║  P2.3  POST /video-gen/scripts → Claude script gen              ║
║  P2.4  POST /video-gen/scripts/:id/approve → creates job        ║
╠═════════════════════════════════════════════════════════════════╣
║  PHASE 3 — Full Pipeline (TTS → HeyGen → Creatomate)            ║
║  Est: 4-5 hours                                                 ║
╠═════════════════════════════════════════════════════════════════╣
║  P3.1  POST /video-gen/jobs/:id/produce → full async pipeline   ║
║         Step 1: ElevenLabs TTS → audio URL                     ║
║         Step 2: HeyGen generate → poll → download raw MP4      ║
║         Step 3: Pexels b-roll fetch                             ║
║         Step 4: Creatomate render → poll → final_video_url     ║
║         Status updates at each step (SSE or polling)            ║
║  P3.2  POST /video-gen/jobs/:id/distribute                      ║
║         → uses existing social post system                      ║
╠═════════════════════════════════════════════════════════════════╣
║  PHASE 4 — Frontend UI                                          ║
║  Est: 4-5 hours                                                 ║
╠═════════════════════════════════════════════════════════════════╣
║  P4.1  frontend/src/lib/api/video-gen.ts                        ║
║  P4.2  VideoStudioPage + tabs shell                             ║
║  P4.3  AvatarsTab + Create Avatar drawer                        ║
║  P4.4  PulseTab (news feed + generate script button)            ║
║  P4.5  ScriptsTab + Script Editor drawer                        ║
║  P4.6  JobsTab + stage progress + video player + distribute     ║
║  P4.7  Sidebar: add "Video Studio" nav item (Clapperboard icon) ║
║  P4.8  App.tsx: add /video-studio route                         ║
╠═════════════════════════════════════════════════════════════════╣
║  PHASE 5 — Nora VSL (first real video)                          ║
║  Est: 1-2 hours                                                 ║
╠═════════════════════════════════════════════════════════════════╣
║  P5.1  Provision Nora avatar in HeyGen (reference image needed) ║
║  P5.2  Clone/configure Nora ElevenLabs voice                    ║
║  P5.3  Build Creatomate PCG template (brand overlay)            ║
║  P5.4  Run VSL through full pipeline                            ║
║  P5.5  Review + distribute                                      ║
╠═════════════════════════════════════════════════════════════════╣
║  PHASE 6 — TIACA News Reports                                   ║
║  Est: 2-3 hours                                                 ║
╠═════════════════════════════════════════════════════════════════╣
║  P6.1  TIACA avatar profile setup (reference image + voice)     ║
║  P6.2  Configure vertical keywords for air cargo intel          ║
║  P6.3  Set up cron (daily 6am) → pulse gather → auto-script     ║
║  P6.4  First TIACA news report through full pipeline            ║
╚═════════════════════════════════════════════════════════════════╝

TOTAL ESTIMATED: ~18-20 hours
```

---

## CRITICAL DEPENDENCIES (acquire before building)

| Dependency | Action | Priority |
|------------|--------|----------|
| HeyGen API Key | Sign up at heygen.com (Team plan $89/mo recommended) | P1 blocker |
| Creatomate API Key | Sign up at creatomate.com (Starter $49/mo) | P1 blocker |
| Perplexity API Key | Sign up at perplexity.ai (API access, pay-per-use) | P2 |
| Tavily API Key | Sign up at tavily.com (free tier available) | P2 |
| Pexels API Key | Sign up at pexels.com/api (free) | P3 |
| Nora reference image | High-res face photo of Nora's visual identity | P5 blocker |
| Creatomate news template | Build once in Creatomate dashboard, save template ID | P3 blocker |

---

## FILE MODIFICATION SUMMARY

| File | Action |
|------|--------|
| `crates/db/migrations/20260329000000_video_studio.sql` | CREATE |
| `crates/db/src/models/avatar_profile.rs` | CREATE |
| `crates/db/src/models/video_pulse_item.rs` | CREATE |
| `crates/db/src/models/video_script.rs` | CREATE |
| `crates/db/src/models/video_job.rs` | CREATE |
| `crates/db/src/models/mod.rs` | MODIFY — add 4 pub mods |
| `crates/video_gen/Cargo.toml` | CREATE |
| `crates/video_gen/src/lib.rs` | CREATE |
| `crates/video_gen/src/config.rs` | CREATE |
| `crates/video_gen/src/heygen.rs` | CREATE |
| `crates/video_gen/src/creatomate.rs` | CREATE |
| `crates/video_gen/src/script_gen.rs` | CREATE |
| `crates/video_gen/src/intelligence.rs` | CREATE |
| `crates/video_gen/src/voice.rs` | CREATE |
| `crates/video_gen/src/broll.rs` | CREATE |
| `crates/server/src/routes/video_gen.rs` | CREATE |
| `crates/server/src/routes/mod.rs` | MODIFY — register video_gen |
| `Cargo.toml` (workspace) | MODIFY — add video_gen crate |
| `crates/server/Cargo.toml` | MODIFY — add video_gen dep |
| `.env` / `.env.example` | MODIFY — add 5 new API keys |
| `frontend/src/lib/api/video-gen.ts` | CREATE |
| `frontend/src/lib/api/index.ts` | MODIFY — export videoGenApi |
| `frontend/src/pages/video-studio/index.tsx` | CREATE |
| `frontend/src/pages/video-studio/AvatarsTab.tsx` | CREATE |
| `frontend/src/pages/video-studio/PulseTab.tsx` | CREATE |
| `frontend/src/pages/video-studio/ScriptsTab.tsx` | CREATE |
| `frontend/src/pages/video-studio/JobsTab.tsx` | CREATE |
| `frontend/src/App.tsx` | MODIFY — add /video-studio route |
| `frontend/src/components/layout/sidebar/constants.ts` | MODIFY — add nav item |
