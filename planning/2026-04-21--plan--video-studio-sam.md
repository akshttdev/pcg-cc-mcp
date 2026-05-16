# PCG Video Studio — Full Integration Plan
**Owner:** Sam  
**Branch:** `feature/video-studio`  
**Goal:** A complete in-dashboard video production environment — operators manage every stage of video creation without leaving `dashboard.powerclubglobal.com`

---

## Vision

```
DISCOVER  →  SCRIPT  →  PRODUCE  →  ASSEMBLE  →  POLISH  →  PUBLISH
(Pulse)      (AI)       (Hedra       (B-Roll +    (Overlay   (Social
                        + TTS)        FFmpeg)       Editor)  Connectors)

All stages accessible from /video-studio inside the ORCHA dashboard.
Triggered either from Pulse discovery or by manual operator prompt.
```

---

## Current State (P0 — Baseline, DONE)

```
crates/video_gen/src/
  tts.rs        ✓  ElevenLabs TTS + word-level timestamps
  heygen.rs     ✓  HeyGen upload + generate + poll

crates/server/src/routes/video_gen.rs
  produce_job() ✓  TTS → HeyGen → download → DB

crates/db/migrations/20260426000000_video_studio.sql
  avatar_profiles   ✓
  video_jobs        ✓

frontend/src/pages/video-studio.tsx  (759 lines)
  Anchors tab       ✓
  Jobs tab          ✓  (10s polling)
  Library tab       ✓

crates/nora/src/tools/execute_impl.rs
  GenerateImage     ✓  FLUX Pro Ultra Redux (fal.ai)
  ApplyVideoEffect  ✓  FFmpeg presets
  PostProcessVideoJob ✓ chain effects

gen_ep01_overlays.py  ✓  Pillow overlay generator (Ep.01 proven)
```

---

## Full Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        VIDEO STUDIO SYSTEM                              │
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐  │
│  │    PULSE     │    │    SCRIPT    │    │      PRODUCTION          │  │
│  │  INTELLIGENCE│───►│    ENGINE   │───►│      PIPELINE            │  │
│  │              │    │              │    │                          │  │
│  │  Topics →    │    │  Claude →    │    │  TTS → Hedra → B-Roll →  │  │
│  │  KG Sources  │    │  Structured  │    │  Overlay → Final Cut     │  │
│  └──────────────┘    │  VideoScript │    └──────────────────────────┘  │
│         ▲            └──────────────┘                  │               │
│         │                                              ▼               │
│  ┌──────────────┐                          ┌──────────────────────────┐ │
│  │    TOPIC     │                          │      DISTRIBUTION        │ │
│  │  REGISTRY   │                          │                          │ │
│  │  (operator  │                          │  SocialPost records →    │ │
│  │   config)   │                          │  Platform connectors     │ │
│  └──────────────┘                          └──────────────────────────┘ │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                   DASHBOARD EDITOR  (/video-studio)              │  │
│  │   Pulse Feed │ Script Editor │ B-Roll │ Overlays │ Distribution  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Database Schema — Full Target

```sql
-- Topic registry (operator-configured)
CREATE TABLE video_topics (
    id              BLOB PRIMARY KEY,
    organization_id BLOB NOT NULL REFERENCES organizations(id),
    name            TEXT NOT NULL,          -- "AI & Inference"
    keywords        TEXT NOT NULL,          -- JSON array of search terms
    is_active       INTEGER DEFAULT 1,
    auto_produce    INTEGER DEFAULT 0,      -- fully automatic daily video
    created_at      TEXT NOT NULL
);

-- Pulse intelligence feed (from Perplexity + Tavily)
CREATE TABLE video_pulse_items (
    id              BLOB PRIMARY KEY,
    organization_id BLOB NOT NULL REFERENCES organizations(id),
    topic_id        BLOB REFERENCES video_topics(id),
    headline        TEXT NOT NULL,
    summary         TEXT,
    source_url      TEXT NOT NULL,          -- original article/event URL
    source_type     TEXT NOT NULL,          -- article|video|event|product
    relevance_score REAL DEFAULT 0.0,
    vertical        TEXT,
    used_in_job_id  BLOB REFERENCES video_jobs(id),
    discovered_at   TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

-- Structured scripts (from Claude, operator-editable)
CREATE TABLE video_scripts (
    id              BLOB PRIMARY KEY,
    organization_id BLOB NOT NULL,
    pulse_item_id   BLOB REFERENCES video_pulse_items(id),
    status          TEXT DEFAULT 'draft',   -- draft|approved|in_production
    -- Script body sections
    cold_open       TEXT,
    headline        TEXT,
    body            TEXT,
    brand_tiein     TEXT,
    cta_signoff     TEXT,
    -- Structured metadata (JSON)
    lower_third_name  TEXT,
    lower_third_title TEXT,
    chyron_text       TEXT,
    ticker_items    TEXT DEFAULT '[]',      -- JSON: ["BTC +2.4%", "ETH +1.8%"]
    broll_cues      TEXT DEFAULT '[]',      -- JSON: [{timestamp_ms, duration_ms, query, source_url}]
    scene_prompts   TEXT DEFAULT '[]',      -- JSON: [{timestamp_ms, prompt}] for Kling
    overlay_cues    TEXT DEFAULT '[]',      -- JSON: [{timestamp_ms, template_id, vars{}}]
    -- Meta
    estimated_duration_secs INTEGER,
    word_count      INTEGER,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- B-Roll candidates per script segment
CREATE TABLE video_broll_candidates (
    id              BLOB PRIMARY KEY,
    script_id       BLOB NOT NULL REFERENCES video_scripts(id),
    broll_cue_index INTEGER NOT NULL,       -- index into script.broll_cues[]
    source_url      TEXT NOT NULL,
    source_type     TEXT NOT NULL,          -- article|youtube|image|tweet|product
    local_path      TEXT,                   -- extracted clip path
    in_point_ms     INTEGER DEFAULT 0,
    out_point_ms    INTEGER,
    extraction_status TEXT DEFAULT 'pending', -- pending|extracting|ready|failed
    relevance_score REAL DEFAULT 0.0,
    is_selected     INTEGER DEFAULT 0,      -- operator chose this one
    thumbnail_url   TEXT,
    created_at      TEXT NOT NULL
);

-- Overlay templates (global + project-specific)
CREATE TABLE overlay_templates (
    id              BLOB PRIMARY KEY,
    organization_id BLOB,                   -- NULL = global/system template
    project_id      BLOB,                   -- NULL = org-wide
    name            TEXT NOT NULL,
    category        TEXT NOT NULL,          -- lower_third|ticker|bug|title|cta|stat|quote|tag|custom
    template_json   TEXT NOT NULL,          -- serialized OverlayTemplate struct
    thumbnail_url   TEXT,
    is_global       INTEGER DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Overlay instances on a specific job (rendered with vars)
CREATE TABLE video_job_overlays (
    id              BLOB PRIMARY KEY,
    job_id          BLOB NOT NULL REFERENCES video_jobs(id),
    template_id     BLOB NOT NULL REFERENCES overlay_templates(id),
    timestamp_ms    INTEGER NOT NULL,
    duration_ms     INTEGER NOT NULL,
    vars_json       TEXT DEFAULT '{}',      -- {"name":"Sami","title":"Strategist"}
    rendered_path   TEXT,                   -- path to rendered PNG
    created_at      TEXT NOT NULL
);

-- Extend avatar_profiles (migration)
ALTER TABLE avatar_profiles ADD COLUMN hedra_character_id TEXT;
ALTER TABLE avatar_profiles ADD COLUMN preferred_provider TEXT DEFAULT 'hedra';

-- Extend video_jobs (migration)
ALTER TABLE video_jobs ADD COLUMN script_id BLOB REFERENCES video_scripts(id);
ALTER TABLE video_jobs ADD COLUMN use_synclabs INTEGER DEFAULT 0;
ALTER TABLE video_jobs ADD COLUMN synclabs_video_url TEXT;
ALTER TABLE video_jobs ADD COLUMN broll_status TEXT DEFAULT 'pending';
ALTER TABLE video_jobs ADD COLUMN assembly_status TEXT DEFAULT 'pending';
ALTER TABLE video_jobs ADD COLUMN final_video_url TEXT;
ALTER TABLE video_jobs ADD COLUMN cost_cents_actual INTEGER;
ALTER TABLE video_jobs ADD COLUMN provider TEXT DEFAULT 'hedra';
```

---

## P1 — Video Provider Router

**Goal:** Hedra as primary, HeyGen as fallback. Trait-based abstraction so future providers (Runway, Pika, Luma) drop in cleanly.

```
crates/video_gen/src/providers/
├── mod.rs          VideoGenerationProvider trait + get_provider()
├── hedra.rs        Hedra Character-3 implementation
├── heygen.rs       HeyGen (move existing heygen.rs here)
└── router.rs       Provider selection logic
```

```
PROVIDER SELECTION:

  fn get_provider() -> Box<dyn VideoGenerationProvider>

  HEDRA_API_KEY set?
       │
      YES ──► HedraProvider::new()  ← primary
       │
      NO
       │
  HEYGEN_API_KEY set?
       │
      YES ──► HeyGenProvider::new() ← fallback
       │
      NO ──► panic!("No video provider configured")


  trait VideoGenerationProvider: Send + Sync {
      async fn generate(
          &self,
          character_id: &str,   // hedra_character_id OR heygen_avatar_id
          audio_url:    &str,
          config:       &VideoGenConfig,
      ) -> Result<String>;      // returns job_id

      async fn poll_status(
          &self,
          job_id: &str,
      ) -> Result<VideoGenStatus>; // Pending|Processing|Complete(url)|Failed

      fn provider_name(&self) -> &'static str;
      fn is_configured(&self) -> bool;
  }


  AvatarProfile resolution:
  ┌───────────────────────────────────────────────────────┐
  │  preferred_provider = 'hedra'                        │
  │    → use hedra_character_id                          │
  │    → if missing, fall back to HeyGen                 │
  │                                                       │
  │  preferred_provider = 'heygen'                       │
  │    → use heygen_avatar_id (legacy)                   │
  └───────────────────────────────────────────────────────┘
```

**Tasks:**
- `crates/video_gen/src/providers/mod.rs` — define trait + `get_provider()`
- `crates/video_gen/src/providers/heygen.rs` — move existing `heygen.rs` here, implement trait
- `crates/video_gen/src/providers/hedra.rs` — new, implement trait against Hedra Character-3 API
- `crates/video_gen/src/lib.rs` — update `pub mod` declarations
- `crates/server/src/routes/video_gen.rs` — replace direct `heygen::*` calls with `get_provider()`
- DB migration — add `hedra_character_id`, `preferred_provider` to `avatar_profiles`
- Env: add `HEDRA_API_KEY` to `.env` and Docker secrets

---

## P2 — Pulse Intelligence Engine

**Goal:** Continuously collect topic intelligence into `video_pulse_items` and the knowledge graph. Videos can be triggered on-demand from any discovered item.

```
FLOW:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  video_topics (operator-configured)
  ["AI & Inference", "Blockchain", "Quantum", "TIACA", ...]
         │
         ▼  Daily cron 06:00 UTC  (or POST /api/video-pulse/gather)
         │
  gather_intelligence(org_id, topics[])
    │
    ├─ for each topic:
    │    Perplexity sonar-pro   ─┐
    │    Tavily search          ─┴─► raw_results[]
    │
    └─ Claude (Haiku, cheap):
         • deduplicate across topics
         • score relevance (0.0–1.0)
         • tag: source_type, vertical
         • extract: source_url (original, not aggregator)
         │
         ▼
  upsert video_pulse_items
  (UNIQUE on source_url — no duplicate stories)
         │
         ▼  also register in knowledge graph
  project_knowledge_sources {
    owner_type: 'organization',
    owner_id:   org_id,
    source_type: 'PulseItem',
    source_id:   pulse_item.id,
    source_title: headline,
    coverage_score: relevance_score
  }


  TWO TRIGGER PATHS FROM PULSE:
  ┌─────────────────────────────────────────────────────┐
  │                                                     │
  │  A) On-demand                                       │
  │     Operator clicks "Make Video" on pulse card      │
  │     → POST /api/video-studio/scripts/generate       │
  │       { pulse_item_id }                             │
  │                                                     │
  │  B) Manual prompt                                   │
  │     Operator types brief directly                   │
  │     → POST /api/video-studio/scripts/generate       │
  │       { prompt, topic_id? }                         │
  │                                                     │
  │  Both paths → Script Engine (P3)                    │
  └─────────────────────────────────────────────────────┘
```

**Tasks:**
- `crates/video_gen/src/intelligence.rs` — `gather_intelligence()`, Perplexity + Tavily parallel calls, Claude dedup/score
- DB migration — `video_topics`, `video_pulse_items` tables
- DB model — `VideoTopic`, `VideoPulseItem` CRUD
- Routes — `GET/POST /video-studio/topics`, `GET /video-studio/pulse`, `POST /video-studio/pulse/gather`
- Cron registration — wire `gather_intelligence` into hourly/daily cron in `main.rs`
- KG upsert — register each pulse item in `project_knowledge_sources`

---

## P3 — Script Engine

**Goal:** Claude generates a fully structured `VideoScript` from either a Pulse item or free prompt. Operator reviews and edits before production begins.

```
SCRIPT GENERATION FLOW:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Input:
    pulse_item { headline, summary, source_url, vertical }
    OR
    operator_prompt { text, topic_id? }
    +
    org brand profile (tone, voice, colors)
    +
    existing scripts for this org (style reference)

         │
         ▼
  Claude (Sonnet) — structured output:

  VideoScript {
    cold_open:        "In the next 90 seconds..."
    headline:         "NVIDIA's H100 production hits..."
    body:             "Three key developments..."
    brand_tiein:      "At PowerClub Global, we see..."
    cta_signoff:      "Subscribe for daily briefings..."

    lower_third_name:  "Sami Satoshi"
    lower_third_title: "Digital Asset Strategist"
    chyron_text:       "PCG TECH BRIEFING"

    ticker_items: [
      "BTC +2.4% · ETH +1.8%",
      "NVIDIA H100 demand surge",
      "Fed holds rates steady"
    ]

    broll_cues: [
      { timestamp_ms: 12000, duration_ms: 4000,
        query: "NVIDIA H100 chip datacenter",
        source_url: "https://techcrunch.com/..." },  ← from pulse_item
      { timestamp_ms: 30000, duration_ms: 6000,
        query: "server rack cooling AI infrastructure",
        source_url: null }  ← generic search
    ]

    overlay_cues: [
      { timestamp_ms: 0,     template_id: "pcg_bug",        vars: {} },
      { timestamp_ms: 0,     template_id: "ticker_strip",   vars: { text: "..." } },
      { timestamp_ms: 4000,  template_id: "lower_third",    vars: { name: "Sami Satoshi", title: "..." } },
      { timestamp_ms: 12000, template_id: "broll_tag",      vars: { label: "AI / Inference" } },
      { timestamp_ms: 30000, template_id: "stat_callout",   vars: { number: "$40B", label: "H100 Revenue" } }
    ]

    estimated_duration_secs: 95
  }

         │
         ▼
  INSERT video_scripts (status='draft')

         │
         ▼
  Operator reviews in Script Editor tab
  Can edit any section inline
  status → 'approved' → triggers B-Roll discovery (P4)
```

**Tasks:**
- `crates/video_gen/src/script_gen.rs` — `generate_script(pulse_item | prompt, brand_profile)` → `VideoScript`
- DB migration — `video_scripts` table with all JSON columns
- DB model — `VideoScript` CRUD + `update_section()`
- Routes:
  - `POST /video-studio/scripts/generate` — generate from pulse item or prompt
  - `GET /video-studio/scripts` — list scripts
  - `GET /video-studio/scripts/:id` — get full script
  - `PATCH /video-studio/scripts/:id` — update section(s)
  - `POST /video-studio/scripts/:id/approve` — set status=approved, trigger B-Roll

---

## P4 — B-Roll Pipeline

**Goal:** For each `broll_cue` in the script, find and extract real source material. Fan out from the original pulse source URL to discover as much usable content as possible.

```
B-ROLL DISCOVERY + EXTRACTION:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  broll_cue {
    query: "NVIDIA H100 chip datacenter",
    source_url: "https://techcrunch.com/nvidia-h100..."
  }
         │
         ▼
  STEP 1: FAN-OUT SEARCH from source_url

  broll_discovery(query, source_url):
    ├─ Scrape source_url (Scout scraper already in stack)
    │    extract: embedded images, video iframes, related links
    │
    ├─ Exa similarity search on source_url
    │    returns: related articles, event pages, product pages
    │
    ├─ Tavily image search on query
    │    returns: image URLs relevant to topic
    │
    └─ YouTube API search on query (if YOUTUBE_API_KEY set)
         returns: video IDs + timestamps

  → returns BrollCandidate[] ranked by relevance


  STEP 2: EXTRACTION by source type

  ┌──────────────────────────────────────────────────────┐
  │  type = 'article'                                    │
  │    Playwright headless → load URL                    │
  │    Scroll to content landmarks (headings, images)    │
  │    Strategic pause points from script timestamp      │
  │    → screen_record() → clip .mp4                     │
  │                                                      │
  │  type = 'youtube' / 'video'                          │
  │    yt-dlp download best quality mp4                  │
  │    FFmpeg clip at in_point → out_point               │
  │    → clip .mp4                                       │
  │                                                      │
  │  type = 'image'                                      │
  │    Download image                                    │
  │    Ken Burns effect (FFmpeg zoompan filter)          │
  │    duration_ms from broll_cue                        │
  │    → animated .mp4                                   │
  │                                                      │
  │  type = 'tweet' / 'social post'                      │
  │    Playwright screenshot at 2x resolution            │
  │    Ken Burns + subtle zoom                           │
  │    → animated .mp4                                   │
  └──────────────────────────────────────────────────────┘

  → local_path stored in video_broll_candidates


  STEP 3: OPERATOR REVIEW in B-Roll Editor tab

  Per broll_cue segment, operator sees:
    [Article screen rec ▶]  [YouTube clip ▶]  [Product image ▶]
    Score: 0.94              Score: 0.87        Score: 0.71
    TechCrunch               YouTube            NVIDIA.com

    In: [00:00] Out: [00:04]
    [Use This]  (sets is_selected = 1)

  Multiple clips CAN be selected per cue — assembled in order.


  MULTIPLE CLIPS PER SEGMENT (high-fidelity edits):
  ┌──────────────────────────────────────────────────────┐
  │  segment at 0:12 → 0:18 (6 seconds):                │
  │    0:12→0:14  article scroll (2s)                   │
  │    0:14→0:16  product image Ken Burns (2s)           │
  │    0:16→0:18  YouTube event clip (2s)                │
  │                                                      │
  │  FFmpeg concat with 0.3s crossfade between each     │
  └──────────────────────────────────────────────────────┘
```

**Crate structure:**
```
crates/video_gen/src/broll/
├── mod.rs          BrollCandidate, discovery orchestrator
├── discovery.rs    fan-out search (Exa + Tavily + YouTube)
├── extractor.rs    article screen recording (Playwright subprocess)
├── downloader.rs   yt-dlp wrapper for video clips
└── ken_burns.rs    FFmpeg zoompan for images + tweets
```

**Tasks:**
- DB migration — `video_broll_candidates` table
- DB model — `VideoBrollCandidate` CRUD
- `broll/discovery.rs` — fan-out search from source_url + query
- `broll/extractor.rs` — Playwright screen recorder (subprocess, outputs mp4)
- `broll/downloader.rs` — yt-dlp wrapper
- `broll/ken_burns.rs` — FFmpeg zoompan animated clip generator
- Routes:
  - `POST /video-studio/scripts/:id/discover-broll` — trigger discovery for all cues
  - `GET /video-studio/scripts/:id/broll` — list candidates grouped by cue
  - `PATCH /video-studio/broll/:id/select` — mark as selected
  - `PATCH /video-studio/broll/:id/trim` — update in/out points
- Auto-trigger on `POST /video-studio/scripts/:id/approve`

---

## P5 — Overlay Engine (Rust)

**Goal:** Dynamic, project-specific overlays with text injection. Every project can have custom templates. Final output is PNG sheets composited via FFmpeg.

```
OVERLAY ARCHITECTURE:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  crates/video_gen/src/overlay/
  ├── mod.rs          OverlayEngine — load, render, composite
  ├── template.rs     OverlayTemplate, Layer variants
  ├── renderer.rs     Rasterize template → PNG bytes
  ├── compositor.rs   Build FFmpeg filter_complex string
  └── fonts.rs        Font loading + text measurement


  TEMPLATE STRUCTURE:
  ┌──────────────────────────────────────────────────────┐
  │  OverlayTemplate {                                   │
  │    id:         String,                               │
  │    name:       String,      "Lower Third Standard"   │
  │    category:   Category,    LowerThird               │
  │    canvas_w:   u32,         1080 (or 720 for 9:16)  │
  │    canvas_h:   u32,         1920 (or 1280 for 9:16) │
  │    layers:     Vec<Layer>,                           │
  │    vars:       Vec<VarDef>, [{ key:"name", label,    │
  │                               default }]             │
  │  }                                                   │
  │                                                      │
  │  Layer variants:                                     │
  │    TextLayer   { text, font, size, color,            │
  │                  x, y, max_width, alignment }        │
  │    RectLayer   { x, y, w, h, fill, stroke,           │
  │                  radius, opacity }                   │
  │    ImageLayer  { path, x, y, w, h, opacity }         │
  │    GradientLayer { x, y, w, h, stops[] }             │
  │    AnimLayer   { target_layer, keyframes[] }         │
  └──────────────────────────────────────────────────────┘


  DYNAMIC TEXT INJECTION:
    template.render(vars: HashMap<&str, &str>) → PNG bytes

    "{{name}}"    → "Sami Satoshi"
    "{{title}}"   → "Digital Asset Strategist"
    "{{ticker}}"  → "BTC +2.4% · ETH +1.8%"
    "{{label}}"   → "AI / Inference"
    "{{number}}"  → "$40B"
    "{{date}}"    → "April 21, 2026"
    "{{episode}}" → "EP. 01"


  GLOBAL SYSTEM TEMPLATES (seed in migration):
  ┌──────────────────────────┬──────────────────────────┐
  │  lower_third_standard    │  name + title bar        │
  │  lower_third_minimal     │  name only               │
  │  ticker_strip            │  scrolling headlines     │
  │  pcg_bug                 │  logo + show name corner │
  │  episode_badge           │  EP.## gold pill         │
  │  title_card              │  full-screen segment     │
  │  stat_callout            │  number + label          │
  │  pull_quote              │  quote + attribution     │
  │  cta_frame               │  subscribe / CTA         │
  │  broll_tag               │  topic label over b-roll │
  │  alert_banner            │  breaking news bar       │
  └──────────────────────────┴──────────────────────────┘

  PROJECT-SPECIFIC OVERLAYS:
    Operator builds new template in Overlay Editor
    Saved to overlay_templates { organization_id, project_id }
    Available in template picker for all videos in that org/project


  COMPOSITOR — FFmpeg filter_complex:
  ┌──────────────────────────────────────────────────────┐
  │  assemble_final_video(job_id):                       │
  │                                                      │
  │  Inputs:                                             │
  │    [0:v] anchor talking head (Hedra output)          │
  │    [1:v] b-roll clip at 0:12                         │
  │    [2:v] b-roll clip at 0:30                         │
  │    [3:v] lower_third.png  (rendered overlay)         │
  │    [4:v] ticker_strip.png (rendered overlay)         │
  │    [5:v] pcg_bug.png      (rendered overlay)         │
  │    [0:a] original TTS audio                          │
  │                                                      │
  │  filter_complex:                                     │
  │    swap b-roll at timestamps (with 0.3s xfade)       │
  │    overlay each PNG at its timestamp with fade       │
  │    ticker: use FFmpeg scroll filter on text          │
  │    output: [final_v][final_a]                        │
  │                                                      │
  │  → final_video.mp4                                   │
  └──────────────────────────────────────────────────────┘
```

**Tasks:**
- DB migration — `overlay_templates`, `video_job_overlays` tables
- DB models — `OverlayTemplate`, `VideoJobOverlay` CRUD
- `overlay/template.rs` — OverlayTemplate struct, Layer enum, serialization
- `overlay/renderer.rs` — rasterize using `image` crate + `ab_glyph` for text
- `overlay/compositor.rs` — build FFmpeg filter_complex from job overlays + b-roll cuts
- `overlay/fonts.rs` — font loading (Cinzel, Archivo Black, Fira Sans Condensed from assets)
- Seed migration — insert global system templates as JSON
- Routes:
  - `GET /video-studio/overlay-templates` — list (global + org)
  - `POST /video-studio/overlay-templates` — create new project template
  - `PATCH /video-studio/overlay-templates/:id` — update
  - `GET /video-studio/overlay-templates/:id/preview` — render PNG preview

---

## P6 — Full Assembly Pipeline

**Goal:** Wire all stages into a single async job pipeline. SSE streams progress to the frontend in real time.

```
FULL JOB PIPELINE (video_jobs state machine):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Job created (script_id, avatar_id)
       │
       ▼ status: 'tts_processing'
  ElevenLabs TTS
  (eleven_turbo_v2_5, word timestamps)
       │
       ▼ status: 'anchor_generating'
  get_provider().generate(character_id, audio_url)
  ← Hedra Character-3 (or HeyGen fallback)
       │
       │ [if use_synclabs=true]
       ├──► status: 'synclabs_processing'
       │    Sync.so lip-sync enhancement
       │
       ▼ status: 'broll_assembling'
  assemble b-roll segments
  (selected candidates → concat + xfade)
       │
       ▼ status: 'overlays_rendering'
  render all overlay PNGs
  (OverlayRenderer for each video_job_overlays entry)
       │
       ▼ status: 'final_assembly'
  FFmpeg filter_complex
  (anchor + b-roll + overlays → final_video.mp4)
       │
       ▼ status: 'complete'
  final_video_url stored
  SSE event emitted → frontend player loads


  COST TRACKING per job:
  ┌────────────────────────────────────────────────────┐
  │  TTS:        duration_secs × $0.002/s              │
  │  Hedra:      duration_secs × $0.006/s (estimate)   │
  │  Sync.so:    duration_secs × $0.05/s (if enabled)  │
  │  Kling:      clips_count × $0.28 (if used)         │
  │  Perplexity: ~$0.05 flat per gather                │
  │  Total → cost_cents_actual on job completion       │
  └────────────────────────────────────────────────────┘


  SSE ENDPOINT:
    GET /video-studio/jobs/:id/stream
    → emits: { stage, progress_pct, message, cost_so_far }
```

**Tasks:**
- Refactor `video_gen.rs` `produce_job()` to use new provider router
- Add all new pipeline stages to job state machine
- Wire b-roll assembly into pipeline after anchor generation
- Wire overlay rendering into pipeline after b-roll
- Wire FFmpeg final assembly (compositor.rs)
- Add `cost_cents_actual` tracking throughout
- SSE endpoint for job progress
- Routes:
  - `POST /video-studio/jobs` — create job from script_id + avatar_id
  - `GET /video-studio/jobs/:id/stream` — SSE progress

---

## P7 — Dashboard Editor UI

**Goal:** Operators never leave the dashboard. Full production environment at `/video-studio`.

```
LAYOUT:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌──────────┬───────────────────────────────────┬────────────┐
│ SIDEBAR  │         CENTER WORKSPACE          │ PROPERTIES │
│          │                                   │            │
│ Pulse    │  ┌─────────────────────────────┐  │ (context-  │
│ Feed  ●3 │  │     VIDEO PREVIEW           │  │  sensitive │
│          │  │     1080×1920 or 16:9       │  │  panel)    │
│ Scripts  │  │     scrubable player        │  │            │
│          │  └─────────────────────────────┘  │ Overlay    │
│ Anchors  │                                   │ selected:  │
│          │  ┌─────────────────────────────┐  │  • text    │
│ B-Roll   │  │         TIMELINE            │  │  • font    │
│ Library  │  │  0s──────────────────►95s   │  │  • color   │
│          │  │  [anchor track        ████] │  │  • pos     │
│ Overlays │  │  [b-roll  ░░██░░░███░░░░░░] │  │  • timing  │
│          │  │  [lower_third ██░░░░░░░░░░] │  │            │
│ Jobs     │  │  [ticker  ████████████████] │  │ B-Roll     │
│          │  │  [bug     ████████████████] │  │ selected:  │
│ Publish  │  └─────────────────────────────┘  │  • in/out  │
│          │                                   │  • swap    │
│          │  ┌──────┬───────┬───────┬───────┐ │  • effects │
│          │  │Script│B-Roll │Overlay│Publish│ │            │
│          │  └──────┴───────┴───────┴───────┘ │            │
└──────────┴───────────────────────────────────┴────────────┘


SIDEBAR: Pulse Feed panel
  Story cards ranked by relevance_score
  Vertical filter tabs (All | AI | Blockchain | Quantum | ...)
  Each card:
    headline (truncated 2 lines)
    source (techcrunch.com)
    relevance badge (0.94)
    discovered: "2 hours ago"
    [Make Video] button → POST /scripts/generate { pulse_item_id }
    [View Source] → opens source_url in new tab


EDITOR TAB: Script
  ┌────────────────────────────────────────────────────┐
  │  [Cold Open ▾]                                     │
  │  "In the next 90 seconds, here's what you need    │
  │   to know about NVIDIA's latest move..."           │
  │                                    [Regenerate ↺] │
  │                                                    │
  │  [Headline ▾]                                      │
  │  "NVIDIA H100 production hits record high..."      │
  │                                    [Regenerate ↺] │
  │                                                    │
  │  [Body ▾]    [Brand Tie-In ▾]    [CTA ▾]           │
  │                                                    │
  │  ─── Lower Thirds ───────────────────────────────  │
  │  Name:  [Sami Satoshi          ]                   │
  │  Title: [Digital Asset Strategist]                 │
  │  Ticker: [BTC +2.4% · ETH +1.8% · SOL -0.3%    ] │
  │                                                    │
  │  Est. duration: 95s   Word count: 312              │
  │  [Save Draft]          [Approve → Start B-Roll]    │
  └────────────────────────────────────────────────────┘


EDITOR TAB: B-Roll
  Per broll_cue segment:
  ┌────────────────────────────────────────────────────┐
  │  ● Segment at 0:12 → 0:18  "NVIDIA H100 chip"     │
  │                                                    │
  │  [TechCrunch article ▶]  Score: 0.94  Article     │
  │  [NVIDIA product page ▶] Score: 0.87  Image       │
  │  [YouTube event clip  ▶] Score: 0.82  Video       │
  │  [+ Search more]                                   │
  │                                                    │
  │  Selected: [article 0:00→0:02] [image 0:02→0:04]  │
  │  [image 0:04→0:06]  ← drag to reorder             │
  │                                                    │
  │  In: [00:12.000]  Out: [00:18.000]                 │
  └────────────────────────────────────────────────────┘


EDITOR TAB: Overlays
  Timeline view:
  ┌────────────────────────────────────────────────────┐
  │  0s─────────────────────────────────────────►95s  │
  │  [pcg_bug    ══════════════════════════════════]   │
  │  [ticker     ══════════════════════════════════]   │
  │  [lower_third  ══════════]                         │
  │  [broll_tag           ══════]    [broll_tag   ══]  │
  │  [stat_callout              ═══════]               │
  │  [cta_frame                              ════════] │
  │                                                    │
  │  Click overlay → Properties panel (edit vars)     │
  │  Drag → reposition on timeline                     │
  │  [+ Add Overlay]  [New Template]  [Org Templates ▾]│
  └────────────────────────────────────────────────────┘


EDITOR TAB: Distribution
  ┌────────────────────────────────────────────────────┐
  │  ☑ LinkedIn   [Auto caption...]              Edit  │
  │  ☑ Instagram  [Reel caption...]              Edit  │
  │  ☑ YouTube    [Title + description...]       Edit  │
  │  ☐ TikTok     [Connect TikTok to enable]          │
  │  ☑ X/Twitter  [Thread version...]            Edit  │
  │  ☐ Threads    [Connect Threads to enable]         │
  │                                                    │
  │  Format:  ● 9:16 Vertical  ○ 16:9 Landscape       │
  │                                                    │
  │  [Publish Now]  or  [Schedule: Apr 22 @ 9:00 AM ▾]│
  │                    [Approve & Queue]               │
  └────────────────────────────────────────────────────┘
```

**Tasks:**
- Redesign `video-studio.tsx` with sidebar navigation
- Video preview player component (reuse + enhance existing)
- Timeline component (drag-to-scrub, overlay tracks)
- Pulse Feed panel with story cards + relevance badges
- Script Editor tab (editable sections, regenerate per-block)
- B-Roll Editor tab (per-cue candidate browser, in/out editor, multi-clip selection)
- Overlay Editor tab (timeline editor, template picker, vars editor)
- Distribution tab (per-platform captions, schedule picker)
- Properties panel (context-sensitive right panel)
- SSE connection for live job progress

---

## P8 — Distribution

**Goal:** One-click publish to social platforms from the Distribution tab. Uses existing social connector infrastructure.

```
DISTRIBUTION FLOW:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Job status = 'complete'
       │
       ▼
  generate_distribution_copy(job_id, script_id):
    Claude: generate caption + hashtags per platform
    format rules:
      LinkedIn:  professional tone, 1200 chars max
      Instagram: 5 hashtags, hooks-first
      YouTube:   title + description + tags
      X:         280 chars OR thread format
      TikTok:    trending sounds suggestion + caption
       │
       ▼
  INSERT social_posts [] (one per platform)
  { platform, content, video_url, status='draft',
    scheduled_at=NULL }

       │ operator reviews in Distribution tab
       │ edits captions if needed
       │ sets schedule or publishes immediately
       ▼
  social_posts.status = 'scheduled' / 'published'
       │
       ▼
  existing automation_publish_scheduled_posts()
  picks up and fires via social connectors
  (connectors already wired: Twitter, TikTok, Threads +
   LinkedIn, Instagram via integrations dev work)
```

**Tasks:**
- `crates/video_gen/src/distribution.rs` — `generate_distribution_copy()`
- Route: `POST /video-studio/jobs/:id/generate-distribution` — triggers caption generation
- Route: `POST /video-studio/jobs/:id/publish` — sets schedule + creates SocialPost records
- Wire into existing `social_posts` table + automation loop

---

## P9 — TIACA Automation (Fully Automatic)

**Goal:** Daily news report produced and queued for operator approval with zero manual steps.

```
TIACA DAILY FLOW:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  06:00 UTC daily cron
       │
       ▼
  gather_intelligence(TIACA_VERTICALS)
  ["aviation", "cargo logistics", "trade", "sustainability",
   "air freight", "IATA", "supply chain"]
       │
       ▼ top-scored pulse_item selected automatically
       │
       ▼ generate_script(pulse_item, TIACA_brand_profile)
       │
       ▼ discover_broll(all cues in parallel)
       │
       ▼ auto-select highest-relevance candidate per cue
       │
       ▼ create video_job { auto_produced: true }
       │
       ▼ pipeline runs (TTS → Hedra → B-Roll → Overlay → Final Cut)
       │
       ▼ generate_distribution_copy()
       │
       ▼ notify operator via Discord:
         "📹 TIACA daily brief ready for review
          Headline: [headline]
          Duration: 95s
          [Review in Dashboard →]"
       │
       ▼ operator clicks through, approves, publishes
```

**Tasks:**
- `video_topics` seed row for TIACA with verticals
- Cron entry in `main.rs` — daily at 06:00 UTC
- Auto-select logic (highest relevance_score not yet used)
- Discord notification via existing Discord connector
- Env: `TIACA_ORG_ID`, `TIACA_AVATAR_ID`, `TIACA_AUTO_PRODUCE=true`

---

## Priority Order Summary

```
P0  DONE  ─── Baseline: TTS → HeyGen → frontend ✓

P1  FIRST ─── Provider Router (Hedra primary, HeyGen fallback)
               BLOCKS everything — all video gen goes through this

P2  SECOND ── Pulse Intelligence Engine
               BLOCKS P3 (script needs pulse items to read)
               BLOCKS P9 (TIACA auto-flow needs pulse)

P3  THIRD ─── Script Engine
               BLOCKS P4 (b-roll cues come from script)
               BLOCKS P6 (job needs script_id)

P4  FOURTH ── B-Roll Pipeline
               BLOCKS P6 (assembly needs b-roll clips)
               Can build P5 in parallel with P4

P5  PARALLEL  Overlay Engine (Rust)
               Can build alongside P4
               BLOCKS P6 (assembly needs overlay renders)

P6  FIFTH ─── Full Assembly Pipeline
               Requires P1 + P4 + P5 complete

P7  SIXTH ─── Dashboard Editor UI
               Can build incrementally as each backend stage lands
               Full editor requires P3 + P4 + P5 + P6

P8  SEVENTH ─ Distribution
               Requires P6 (needs complete job + video_url)
               Uses existing social_posts infrastructure

P9  LAST ──── TIACA Automation
               Requires all prior phases
               First real end-to-end automated production
```

```
DEPENDENCY GRAPH:

  P0 ──► P1 ──► P2 ──► P3 ──► P4 ──┐
                                    ├──► P6 ──► P8 ──► P9
                         P5 ────────┘
                              │
                         P7 (builds incrementally alongside P3-P6)
```

---

## Environment Variables Required

```
# Video providers
HEDRA_API_KEY=...          # P1 — primary avatar video
HEYGEN_API_KEY=...         # P1 — fallback (existing)
ELEVENLABS_API_KEY=...     # existing

# B-Roll extraction
YOUTUBE_API_KEY=...        # P4 — YouTube search
# yt-dlp must be installed on host: apt install yt-dlp
# Playwright: node_modules/.bin/playwright (already in stack?)

# Intelligence
PERPLEXITY_API_KEY=...     # P2
TAVILY_API_KEY=...         # P2 (free tier available)

# Optional quality enhancement
SYNCLABS_API_KEY=...       # P6 — opt-in lip-sync polish

# TIACA automation
TIACA_ORG_ID=...           # P9
TIACA_AVATAR_ID=...        # P9
TIACA_AUTO_PRODUCE=true    # P9
```

---

## Key Files Reference

```
crates/video_gen/src/
├── lib.rs
├── tts.rs                   ✓ existing
├── providers/               NEW P1
│   ├── mod.rs
│   ├── hedra.rs
│   ├── heygen.rs            (moved)
│   └── router.rs
├── intelligence.rs          NEW P2
├── script_gen.rs            NEW P3
├── broll/                   NEW P4
│   ├── mod.rs
│   ├── discovery.rs
│   ├── extractor.rs
│   ├── downloader.rs
│   └── ken_burns.rs
├── overlay/                 NEW P5
│   ├── mod.rs
│   ├── template.rs
│   ├── renderer.rs
│   ├── compositor.rs
│   └── fonts.rs
└── distribution.rs          NEW P8

crates/server/src/routes/
└── video_gen.rs             UPDATE throughout

frontend/src/pages/
└── video-studio.tsx         MAJOR REWRITE P7

crates/db/migrations/
└── 2026XXXX_video_studio_v2.sql
```

---

## Confirmed Infrastructure

- **Playwright** — ✓ available in stack. P4 article screen recording unblocked, no Node sidecar needed.
- **yt-dlp** — confirm on host before P4: `which yt-dlp`
- **HEDRA_API_KEY** — pending provisioning (unblocks P1)

---

## Sami Satoshi — Anchor Character Brief

- **Name:** Sami Satoshi | **Title:** Digital Asset Strategist
- **Ethnicity:** Indian
- **Set design:** Tropical Indian Oasis — lush tropical plants, warm Indian-inspired decor, natural textures, visible tech gadgets on desk/shelves, PCG Neon sign prominent in background
- **Tone:** Warm, sophisticated, modern — not a generic corporate news desk
- **Status:** Starting point, to be refined over time

Use this set description for all Hedra Character-3 avatar initialization and FLUX portrait batch generation (Era3D multi-angle set). Overlay safe zones should account for neon sign placement in background right third.

Initial topic seeds for `video_topics`:
- AI & Inference
- Blockchain & DeFi
- Quantum Computing
- Aviation & Cargo (TIACA)
- Sovereign Technology
