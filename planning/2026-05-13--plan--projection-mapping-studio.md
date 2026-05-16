# Projection Mapping Studio — Dashboard Feature Plan

**Date**: 2026-05-13
**Branch**: TBD (`feat/projection-studio`)
**Worktree**: TBD — to be created when work begins
**Base**: `main`
**Status**: Proposal — initial spec for review

## Why this exists

Sirak Studios is exploring projection mapping as a product line for **immersive wellness events** (60–90 minute breath/sound journeys, gallery installations, retreat experiences). The aesthetic — slow, organic, generative, breath-paced — maps directly onto where AI content generation is strongest right now, and the long-form duration makes a pre-rendered asset bank impractical. The product needs to be:

- **Brief-driven**: a producer describes the intent ("11-minute parasympathetic descent, forest-dawn palette, paced to 6 breaths/min") and the system generates content.
- **Agentically composed**: Claude orchestrates generation jobs (ComfyUI, Runway, Luma) against a scene grammar, not one-off prompts.
- **Multi-projector aware**: from day one the data model assumes multi-output venues with calibration data, not single-screen playback.
- **Operator-controllable live**: during the event a facilitator can shift palette, energy, and pacing in real time without touching the render rig.

The dashboard's role is **brief, generate, curate, approve, program, and operate**. It is **not** the renderer — TouchDesigner (and optionally Resolume / Notch / disguise at larger tiers) handles the actual pixel output and projector blending. The dashboard tells the render rig what to play, when, and at what parameter values.

This follows the established Studio pattern in PCG CC MCP — same shape as the AI Video Studio (`/video-studio`, `crates/video_gen/`, job-based pipeline). New surface: `/projection-studio`, `crates/projection_gen/`, `/api/projection-gen/*`.

---

## Vision & Use Cases

### Primary use case — Immersive wellness event
A facilitator at a Sirak Studios wellness retreat brings up Projection Studio, selects a venue ("Krog St Zone D — 4 projectors, dome ceiling"), picks a session template ("Breath Descent · 75min"), reviews the auto-generated scene timeline, approves it, and runs the show. During the session they nudge a "Calm ↔ Intensify" slider that drives palette, motion speed, and density across all four projectors simultaneously.

### Secondary use cases
- **TIACA gala backdrop** — 20-minute looping atmosphere across a single hero surface.
- **Inessa Art opening** — gallery walls react to ambient sound; assets generated to match the artist's palette.
- **Fusion Swim Week runway** — pre-programmed sequences synced to music; one operator at a laptop.
- **Tenant resale** — Powerclub-Global agency tenants book the rig + operator for their own client events.

### Aesthetic constraints (wellness-specific)
- Slow motion, long fades, no hard cuts.
- Loop-safe content (no temporal seams).
- Calming color science — LUT-tuned, not raw model output.
- Sub-30s "land" time when scenes transition (no startling participants).
- Light spill discipline — short-throw lasers with true blacks.

---

## System Architecture

Five layers. Each layer has a clear responsibility and a defined interface to the next.

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║              PROJECTION MAPPING STUDIO — SYSTEM ARCHITECTURE                    ║
╚══════════════════════════════════════════════════════════════════════════════════╝

┌─ LAYER 1 ─ DASHBOARD (PCG CC MCP) ─────────────────────────────────────────────┐
│                                                                                │
│   React + Rust/Axum  •  Tenant-scoped  •  /projection-studio                   │
│                                                                                │
│   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐             │
│   │  Sessions   │ │   Library   │ │    Shows    │ │   Venues    │             │
│   │  (briefs)   │ │  (assets)   │ │ (timelines) │ │  (rigs)     │             │
│   └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘             │
│   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐                             │
│   │   Console   │ │    Jobs     │ │  Analytics  │                             │
│   │   (live)    │ │  (gen queue)│ │  (post-show)│                             │
│   └─────────────┘ └─────────────┘ └─────────────┘                             │
│                                                                                │
└────────────────────────────────────┬───────────────────────────────────────────┘
                                     │ REST + WebSocket
                                     ▼
┌─ LAYER 2 ─ AGENT ORCHESTRATOR (crates/projection_gen) ─────────────────────────┐
│                                                                                │
│   • Brief → scene grammar JSON                                                 │
│   • Scene grammar → ComfyUI workflow / Runway prompt / Luma prompt             │
│   • Job queue mgmt  • Output tagging (palette, motion, mood)                   │
│   • Asset library writes  • Show timeline assembly                             │
│   • Live cue dispatch to render rig                                            │
│                                                                                │
└────────┬──────────────────────────────────────────────────────┬────────────────┘
         │ HTTP/WS                                              │ OSC / WebSocket
         ▼                                                      ▼
┌─ LAYER 3 ─ GENERATION ───────────────────┐  ┌─ LAYER 4 ─ RUNTIME ─────────────┐
│                                          │  │                                  │
│   ComfyUI (RunPod / Lambda / local GPU)  │  │  TouchDesigner show machine     │
│   Runway Gen-4   (API)                   │  │   • surface mapping             │
│   Luma Dream Machine (API)               │  │   • edge blending               │
│   Kling (API)                            │  │   • parameter drive             │
│   StreamDiffusion (optional realtime)    │  │   • BLE sensor input            │
│                                          │  │   • Ableton Link sync           │
│   ──────────────────────────────────     │  │                                  │
│   Output → S3/R2 object storage          │  │  ┌─ Optional larger tier ──┐    │
│                                          │  │  │ Resolume / Notch /     │    │
│                                          │  │  │ disguise / UE5+nDisplay│    │
│                                          │  │  └────────────────────────┘    │
└──────────────────────────────────────────┘  └────────────────┬─────────────────┘
                                                               │ HDMI/DP/SDI
                                                               ▼
┌─ LAYER 5 ─ I/O ────────────────────────────────────────────────────────────────┐
│                                                                                │
│   PROJECTORS         SENSORS              CONTROL              SYNC            │
│   Epson LS12000-class  Polar H10 (BLE)    PJLink / Art-Net    Ableton Link    │
│   Christie / Barco     Whoop / Garmin     DMX (lighting)      LTC timecode    │
│   Short-throw laser    Audio interface                                         │
│                        (live music in)                                         │
│                                                                                │
└────────────────────────────────────────────────────────────────────────────────┘
```

### Responsibility split

| Layer | Owns | Does NOT own |
|---|---|---|
| Dashboard | Brief authoring, library curation, show programming, live operator UX, multi-tenant access | Rendering, projector calibration math, BLE pairing |
| Agent Orchestrator | Brief → workflow translation, job lifecycle, asset metadata, cue dispatch | The pixels themselves |
| Generation | Producing video/image/3D output to spec | When/where it plays |
| Runtime (TD) | Mapping, blending, real-time parameter drive, sensor input fusion | Content origination, library management |
| I/O | Light/sound/sensor signal | Anything software |

---

## Dashboard Information Architecture

Mirrors the AI Video Studio shape. Top-level route: `/projection-studio`, scoped per tenant.

```
/projection-studio
├── /sessions                  ← Session Briefs (the "what we want to create")
│   ├── /sessions/new          ← Brief Builder (template + parameter form)
│   ├── /sessions/:id          ← Brief detail + generated scene list + approve
│   └── /sessions/:id/scenes/:sceneId   ← Single scene detail + regenerate
│
├── /library                   ← Asset Library (all generated content)
│   ├── filter by: palette, motion-class, mood, duration, tenant, tags
│   └── /library/:assetId      ← Asset detail (preview, metadata, usage count)
│
├── /shows                     ← Compiled Shows (ready-to-run timelines)
│   ├── /shows/new             ← Show Builder (drag scenes to timeline + cues)
│   └── /shows/:id             ← Show detail + cue list + venue binding
│
├── /venues                    ← Physical Rigs (projector configs, calibration)
│   ├── /venues/new            ← Venue setup wizard
│   └── /venues/:id            ← Surfaces, projector list, calibration data
│
├── /console                   ← LIVE Operator Console (during event)
│   ├── master sliders (calm↔intensify, palette, density)
│   ├── cue list with manual fire
│   ├── biometric overlay (lead participant HR/breath)
│   └── projector health strip
│
├── /jobs                      ← Generation Queue (mirror of /video-studio jobs UX)
│   └── /jobs/:id              ← Job detail + logs + retry
│
└── /analytics                 ← Post-show review
    ├── biometric heatmaps (group HR/breath over time)
    ├── operator action log
    └── facilitator notes
```

### Why these screens, in this order

The IA reflects the **workflow**: you author a Session (brief), the system generates scenes, you curate them in the Library, you assemble a Show, you bind it to a Venue, you run it from Console, you review in Analytics. Each screen is a stop on that pipeline.

---

## Data Model

New entities (all tenant-scoped via existing `organization_id` pattern from the CRM).

| Entity | Purpose | Key fields |
|---|---|---|
| `pm_session` | A creative brief — "what we want to make" | `id`, `org_id`, `client_id?`, `title`, `template_id`, `intent_text`, `duration_sec`, `breath_target_bpm?`, `palette_seed`, `status` (draft/generating/curating/approved) |
| `pm_scene` | A timed segment of a session | `id`, `session_id`, `order_index`, `name`, `start_sec`, `duration_sec`, `energy_curve` (json), `palette_lut`, `motion_class`, `breath_phase?` |
| `pm_asset` | A renderable artifact (video, image, 3D loop) | `id`, `org_id`, `scene_id?`, `kind` (video_loop/still/3d/shader), `storage_url`, `duration_sec?`, `palette_extracted` (json), `motion_descriptor` (json), `tags[]`, `source_job_id` |
| `pm_generation_job` | A unit of work to an external gen service | `id`, `org_id`, `scene_id`, `provider` (comfyui/runway/luma/kling), `workflow_id?`, `prompt_payload` (json), `status`, `output_asset_ids[]`, `cost_cents`, `started_at`, `completed_at` |
| `pm_show` | A compiled, ready-to-run timeline | `id`, `org_id`, `session_id?`, `title`, `venue_id`, `total_duration_sec`, `cue_list` (json), `status` (draft/approved/archived) |
| `pm_cue` | A single timeline event | `id`, `show_id`, `time_sec`, `kind` (scene_change/param_ramp/trigger), `payload` (json) |
| `pm_venue` | A physical rig configuration | `id`, `org_id`, `name`, `address?`, `projector_count`, `surfaces` (json — geometry), `calibration_blob` (TouchDesigner JSON), `device_ids[]` |
| `pm_device` | A controllable hardware unit | `id`, `venue_id`, `kind` (projector/sensor/render_node), `model`, `network_addr`, `health_status`, `last_seen_at` |
| `pm_session_log` | Live event telemetry | `id`, `show_id`, `event_ts`, `kind` (operator_action/biometric_sample/cue_fired/error), `payload` (json) |
| `pm_template` | A reusable session shape | `id`, `org_id?` (null = global), `name`, `default_duration`, `scene_grammar` (json) |

### ERD (simplified)

```
pm_template ──┐
              │ (instance of)
              ▼
pm_session ──── pm_scene ──── pm_generation_job ──── pm_asset
                   │                                     ▲
                   └─────────────────────────────────────┘
                                                          (curated into)
pm_show ──── pm_cue                                       │
   │                                                      │
   ├── (binds to) ── pm_venue ── pm_device                │
   │                                                      │
   └── (during run) ── pm_session_log                     │
                                                          │
            pm_asset ◄────────────────────────────────────┘
                  (referenced by cues at runtime)
```

### Multi-tenant notes
- Every `pm_*` row carries `organization_id` like all CRM tables. Sirak Studios's TIACA event assets are invisible to Powerclub-Global.
- Assets are tenant-isolated by default but can be marked `shared_with_org_ids[]` for the future case where Powerclub agencies license a Sirak-built scene library.
- Venues are tenant-owned; on rental jobs the rig provider tenant grants temporary access to the operating tenant.

---

## External Tools & Integration Matrix

Three buckets: **generation** (cloud APIs, mostly), **runtime** (on-site show machines), **I/O** (hardware).

### Generation (Layer 3)

| Tool | Role | Integration | Cost shape | When to use |
|---|---|---|---|---|
| **ComfyUI** | Open-source diffusion workflow runtime — the workhorse | HTTP + WebSocket API; dashboard POSTs a workflow JSON, polls for status. Self-host on RunPod/Modal pod. | GPU rental ($1–3/hr per pod); free software | 80% of wellness content. Loop-safe, palette-controlled, cheapest. |
| **Runway Gen-4** | Premium video generation, best motion coherence | REST API. Synchronous job → poll → download. | ~$0.05–0.50 per clip | Hero scenes, opening/closing moments where motion quality matters. |
| **Luma Dream Machine** | Organic motion specialist | REST API | ~$0.30 per 5s clip | Particularly good for "breath" feel — slow, natural. |
| **Kling** | Long-form video, action capable | REST API | ~$0.50/clip | Less relevant for wellness; useful for runway/fashion sister use cases. |
| **StreamDiffusion** | Real-time img2img diffusion | Self-host alongside TD; Spout/Syphon I/O | GPU only | Optional Phase 6+: live diffusion over a base TD scene during the event. |
| **Stable Video Diffusion / AnimateDiff** | Local fine-grained motion control | ComfyUI nodes | GPU only | Inside ComfyUI workflows where Runway/Luma don't give the parametric control we need. |
| **Suno / Udio** | (Future) generated soundscapes | REST API | ~$0.10/track | Phase 7+ — currently expect live music or curated playlists. |

### Runtime (Layer 4)

| Tool | Role | Integration with dashboard | When to use |
|---|---|---|---|
| **TouchDesigner** | Primary show runtime. Surface mapping, blending, parametric drive, sensor input. | Receives OSC + WebSocket from dashboard. Pulls assets from S3 over network (local cache for show day). Reports back state via WebSocket. | Default for all installations. |
| **MadMapper** | Pure surface-mapping app, lighter than TD | TD outputs Spout/NDI → MadMapper handles mapping if needed. Usually subsumed by TD. | Small single-operator gigs where TD is overkill. |
| **Resolume Arena** | VJ-grade playback + effects, simple cue triggers | Dashboard publishes a setlist JSON; Resolume composition imports clips from S3 cache. OSC for live triggers. | Music-driven shows (Fusion Swim Week) where the operator wants the Resolume cue grid UX. |
| **Notch** | Real-time VFX blocks | Notch Block (.dfxdll) loaded inside TD or Resolume; parameters exposed via OSC. | Phase 4+ when we want effects beyond what TD's native palette gives. |
| **disguise (d3)** | Pro media server for large shows | Dashboard exports show as disguise timeline; UE5+RenderStream for content. Out-of-scope for wellness scale. | Phase 8 or rental partnership only — $10K+/wk rentals, used for large events. |
| **Unreal Engine 5 + nDisplay** | 3D content + cluster rendering | Headless UE5 node receives cues from dashboard via UE Remote Control HTTP API. | Phase 5+, when content needs real 3D depth (dome, large architectural). |

### I/O (Layer 5) — Hardware integration

| Hardware | Role | Integration |
|---|---|---|
| **Epson LS12000 / Christie / Barco laser projectors** | Output. Short-throw + true black is mandatory for wellness. | **PJLink** protocol (TCP) for control — dashboard polls power/temp/lamp hours, can issue shutdown. |
| **Polar H10 chest strap** | Lead participant breath/HR | BLE → TD's BLE CHOP → OSC back to dashboard's `/console` for visualization and as agent input. |
| **Whoop / Garmin** | Group biometric (post-event) | REST APIs, polled post-session for analytics. |
| **Ableton Link** | Music sync across software | TD native support. If a DJ is on Ableton, all scenes phase-lock to the bar grid. |
| **Audio interface (RME / MOTU)** | Live mic/instrument capture for audio-reactive scenes | Class-compliant USB → TD's Audio Device In CHOP. |
| **DMX / Art-Net** | Room lighting | TD has DMX Out CHOP. Optional Phase 6+ — keeps room lights phase-locked to projection. |
| **LTC timecode** | Sync to external production timecode | TD reads LTC; relevant if the wellness show is part of a larger production. |

### Cloud / infra (cross-cutting)

| Service | Role |
|---|---|
| **Cloudflare R2** | Asset storage. Signed URLs for TD pulls. Tenant-scoped buckets. |
| **RunPod / Modal / Lambda Labs** | GPU rental for ComfyUI pods. Orchestrator spins up on job, idles down after 10min. |
| **Tailscale** | Secure tunnel from dashboard backend to on-site render rig for cue dispatch. |
| **Sentry / PostHog** | Existing — error tracking + analytics. |

---

## Workflows

### W1 — Brief → Generate → Curate → Approve (offline, pre-event)

```
Producer                Dashboard              Orchestrator         ComfyUI/Runway          R2
   │                       │                       │                     │                   │
   ├─ POST /sessions ──────►                       │                     │                   │
   │   (brief JSON)        │                       │                     │                   │
   │                       ├─ enqueue scenes ──────►                     │                   │
   │                       │                       ├─ scene1: comfy job ─►                   │
   │                       │                       ├─ scene2: runway job ►                   │
   │                       │                       ├─ scene3: comfy job ─►                   │
   │                       │                       │   ...                │                   │
   │                       │                       │                     ├─ asset upload ───►│
   │                       │                       │◄──── webhook ───────│                   │
   │                       │                       ├─ extract palette/   │                   │
   │                       │                       │  motion descriptor  │                   │
   │                       │                       ├─ insert pm_asset    │                   │
   │                       │◄── WS: scene ready ───│                     │                   │
   ◄── live UI update ─────│                       │                     │                   │
   │                       │                       │                     │                   │
   ├─ review /sessions/:id │                       │                     │                   │
   ├─ regen scene-2 ───────►                       │                     │                   │
   │   (refined prompt)    ├─ re-enqueue ──────────►─────────────────────►                   │
   │                       │                       │                     │                   │
   ├─ APPROVE session ─────►                       │                     │                   │
   │                       ├─ session.status=approved                    │                   │
```

**"Show before approve" gate**: the producer cannot mark `pm_session.status = approved` until every scene has at least one curated asset and the producer has played each one through. UI enforces this — no shortcut.

### W2 — Show Build (assemble timeline)

```
Producer                Dashboard            DB
   │                       │                  │
   ├─ POST /shows/new ─────►                  │
   │   (from session or    │                  │
   │    blank canvas)      │                  │
   │                       ├─ create pm_show ─►
   │                       │                  │
   ├─ drag scene → timeline ►                 │
   │                       ├─ create pm_cue ──►
   ├─ add param ramp ──────►                  │
   │                       ├─ create pm_cue (kind=param_ramp) ►
   ├─ bind venue ──────────►                  │
   │                       ├─ pm_show.venue_id ►
   ├─ preview (render      │                  │
   │   compositor mockup)  │                  │
   │                       │                  │
   ├─ APPROVE show ────────►                  │
   │                       ├─ status=approved │
```

### W3 — Venue Calibration (one-time per rig)

```
Operator (on-site)         Dashboard           TD show machine
   │                          │                   │
   ├─ open /venues/:id        │                   │
   ├─ "Begin calibration" ────►                   │
   │                          ├─ open TD remote ─►
   │                          │                   ├─ project test grid
   │                          │                   ├─ operator nudges corners
   │                          │                   ├─ saves calibration JSON
   │                          │◄── WS: cal done ──│
   ├─ review surfaces ────────│                   │
   ├─ save ───────────────────►                   │
   │                          ├─ pm_venue.calibration_blob = JSON
   │                          │                   │
```

### W4 — Live Show Run

```
Operator         /console          Orchestrator       TD show machine        Sensors
   │                │                  │                   │                    │
   ├─ pick show ────►                  │                   │                    │
   ├─ "PRELOAD" ────►                  │                   │                    │
   │                ├─ send asset list ►                   │                    │
   │                │                  ├─ push to TD ──────►                    │
   │                │                  │                   ├─ download from R2  │
   │                │                  │                   ├─ cache to local SSD│
   │                │                  │◄── ready ─────────│                    │
   │                │◄── PRELOADED ────│                   │                    │
   │                │                  │                   │                    │
   ├─ "GO" ─────────►                  │                   │                    │
   │                ├─ start clock ────►                   │                    │
   │                │                  ├─ dispatch cue 1 ──►                    │
   │                │                  │                   ├─ scene change      │
   │                │                  │                   │                    │
   │                │                  │                   │◄── breath: 6bpm ───│
   │                │◄── biometric ────│◄──────────────────│                    │
   │                │                  │                   │                    │
   ├─ slider:       │                  │                   │                    │
   │   intensify ───►                  │                   │                    │
   │                ├─ /api/console/   ►                   │                    │
   │                │   set-param      ├─ OSC: density=0.7 ►                    │
   │                │                  │                   ├─ update params     │
   │                │                  │                   │                    │
   ├─ "END" ────────►                  │                   │                    │
   │                ├─ stop, log all ──►                   │                    │
```

### W5 — Post-show Analytics

After the show ends, the orchestrator:
1. Closes the `pm_session_log` stream.
2. Pulls biometric samples from connected sensors (Whoop/Garmin REST if used).
3. Computes engagement metrics (HR variance, breath rate trend, energy curve vs intended curve).
4. Renders heatmaps and operator-action timeline.
5. Surfaces in `/projection-studio/analytics/:showId`.

---

## Phased Buildout

Eight phases. Each phase has a "demoable" milestone — something a producer can actually use end-to-end at the end of that phase, even if it's not yet the full vision.

### Phase 0 — Foundation (1 sprint)
- Migrations for all `pm_*` tables.
- Rust crate scaffold: `crates/projection_gen/` with stubs.
- React route stubs for the seven main screens.
- Object storage bucket per tenant (R2).
- **Demo**: empty UI you can navigate; CRUD on venues works.

### Phase 1 — Generation (ComfyUI only) (2 sprints)
- Brief Builder UI (template picker + parameter form).
- Orchestrator: brief → ComfyUI workflow translator (Claude API call).
- ComfyUI integration: pod spin-up via RunPod API, workflow submission, polling, asset upload to R2.
- Asset Library basic view (grid with previews, filter by tag).
- **Demo**: type a brief → 8 scenes generate → see them in library.

### Phase 2 — Curation & Approval (1 sprint)
- Per-scene regenerate with refined prompt.
- Auto-tag pipeline (palette extract via image stats, motion descriptor via optical flow).
- "Show before approve" gate — session approval blocked until all scenes have curated assets.
- **Demo**: full brief → generate → regen one scene → approve session.

### Phase 3 — Show Builder (1 sprint)
- Show Builder UI — drag scenes to timeline, set cue points, param ramps.
- Venue binding.
- Preview compositor (single-output mockup at this stage — no actual projection yet).
- **Demo**: assemble a 30min show from approved scenes, see it play in browser preview.

### Phase 4 — TouchDesigner Bridge (2 sprints)
- TD show project template (Sirak Studios house template).
- WebSocket protocol: dashboard ↔ TD.
- Asset preload mechanism (TD pulls signed R2 URLs to local cache).
- OSC schema for parameter drive.
- Single-projector end-to-end: dashboard "GO" → TD plays.
- **Demo**: full Brief→Generate→Show→Run on one projector at the studio.

### Phase 5 — Live Operator Console (1 sprint)
- `/console` screen with master sliders, cue list, manual fire.
- Real-time state sync (WS).
- Operator action logging.
- **Demo**: run a show and live-modulate from the console.

### Phase 6 — Multi-projector & Venue Calibration (2 sprints)
- Venue setup wizard for n-projector rigs.
- TD calibration remote (test grid projection + corner nudging via dashboard).
- Edge-blend zone definition UI.
- Per-projector health monitor (PJLink polling).
- **Demo**: 4-projector calibrated rig at a real venue.

### Phase 7 — Biometric Integration (1 sprint)
- BLE pairing flow (Polar H10) on the show machine.
- TD BLE CHOP → OSC → dashboard `/console` overlay.
- Biometric data as agent input — orchestrator can re-pace cues based on lead participant breath.
- **Demo**: run a wellness session where palette responds to live HR.

### Phase 8 — Premium tier (Runway/Luma) + Analytics (2 sprints)
- Add Runway Gen-4 and Luma to provider pool.
- Orchestrator picks provider based on scene importance + budget.
- Post-show analytics dashboard.
- **Demo**: hybrid ComfyUI/Runway show with cost-tracked job log + post-show review.

### Future / out of initial scope
- **Notch blocks** in the runtime (Phase 9+).
- **UE5 + nDisplay** for dome / architectural (Phase 10+).
- **disguise** integration for large rental shows (when revenue justifies — likely never in-house).
- **Suno/Udio** generative soundscapes (Phase 11+).
- **Operator marketplace** — Powerclub agencies book Sirak-trained operators (product feature, not technical).

---

## Hardware Stack — Three Tiers

### Tier 1 — Studio / Small Wellness Room (1–2 projectors)
| Item | Spec | ~Cost |
|---|---|---|
| Projector | Epson LS12000 4K laser short-throw | $5K |
| Show machine | M2 Mac Studio (Ultra) or Linux box w/ RTX 4090 | $5K |
| BLE adapter | Built-in or USB BLE 5.0 | $30 |
| Audio interface | MOTU M2 | $200 |
| Cabling | HDMI 2.1, fiber for long runs | $500 |
| **Total** | | **~$11K** |

### Tier 2 — Mid-size Event / Gallery (3–4 projectors, blended)
| Item | Spec | ~Cost |
|---|---|---|
| Projectors x4 | Epson LS12000 or Christie 4K7-HS | $20–60K |
| Show machine | Workstation w/ 2× RTX 6000 Ada | $15K |
| Network switch | 10GbE managed | $1K |
| Sensors | Polar H10 + spares | $400 |
| Lighting bridge | Art-Net node | $300 |
| **Total** | | **~$37–77K** |

### Tier 3 — Large / Architectural / Dome
- 8+ projectors, often rented per-event.
- disguise rx II + 4U render nodes — rental partnership, not owned.
- UE5 + nDisplay cluster.
- Estimated rental: $15–40K/event in equipment alone.

**Recommendation**: build the dashboard against Tier 2 capability from Phase 4 on. Tier 1 is the development rig. Tier 3 is partnership territory.

---

## Risks & Open Questions

### Technical risks
- **Latency**: dashboard → orchestrator → TD round-trip on cue dispatch must be sub-100ms. Mitigation: orchestrator dispatches cues over WS with timestamp ahead; TD schedules locally.
- **Asset preload size**: a 75min show could be 20GB. Mitigation: local SSD cache, dashboard sends a "preload" command 30min before show.
- **Generation quality drift**: ComfyUI workflows degrade as models update. Mitigation: pin workflow versions, snapshot models in our pod template.
- **BLE reliability**: chest strap drops mid-session. Mitigation: graceful degrade — orchestrator falls back to scripted timing if sensor goes silent for >30s.
- **Multi-tenant noise**: a Powerclub tenant misuses Sirak's premium ComfyUI workflow. Mitigation: workflow ownership at template level, not asset level.

### Open product questions
1. **Who's the first paying user?** TIACA gala? Sirak retreat? Need a concrete first event to anchor Phase 4 demo.
2. **Operator skill model** — facilitator-operator or dedicated tech? Drives Console UX complexity.
3. **Show approval gating** — does the client approve the show in the dashboard, or does the producer? (Affects whether `/projection-studio` needs a client-share view, similar to Decks.)
4. **Pricing** — flat per-event, per-minute, per-projector-hour? Affects analytics + billing wiring.
5. **Asset reuse across tenants** — when Powerclub's Inessa Art opening reuses a Sirak-built scene, how is that licensed/billed? (Probably out of scope for v1.)

### Decisions needed before Phase 1
- [ ] Confirm RunPod vs Modal vs Lambda for GPU hosting.
- [ ] Pick ComfyUI base model (SDXL? Flux? SVD?).
- [ ] Decide whether scene generation is one-shot or step-by-step refinable.
- [ ] Lock the TD project template scope (what features ship in Phase 4 template vs added later).

---

## Out of Scope (this plan)

- The TouchDesigner project itself (separate deliverable, lives in `/Users/sirakstudios/topos/_sirak/projection/td-templates/`).
- Hardware procurement processes.
- On-site networking / cabling specifications per venue.
- Client-facing booking/sales workflow (handled by existing CRM `/clients/:id` deal pipeline).
- Music/audio production (assume facilitator brings their own).
- Onboarding/training material for operators (separate deliverable).

---

## Next Steps

1. **Review this plan with Aaren and the team** — calibrate scope and timeline.
2. **Pick a first event** — likely TIACA gala or a Sirak retreat — to anchor the Phase 4 demo target.
3. **Spike Phase 0 + Phase 1** — get the brief→ComfyUI→library loop working end-to-end at lowest fidelity. Validate the agent translator quality before building UI polish.
4. **Acquire Tier 1 hardware** — 1 projector + 1 show machine, in-studio, for development.
5. **Build TD reference template** in parallel with Phase 0/1 — the dashboard work is blocked on this from Phase 4 onward.
