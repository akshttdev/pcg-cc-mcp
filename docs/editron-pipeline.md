# Editron Media Pipeline

Editron is the Post-Production Architect agent that Nora activates whenever event footage arrives. The agent operates a full post-production lane that accepts batch links (e.g., Dropbox), normalizes media, iterates on cuts, and renders final deliverables. When stylized or AI-generated plates are needed, Editron can consult the distinct **Master Cinematographer (Spectra)** agent, but Editron still owns the ingest → render lane end-to-end.

## Responsibilities
- Validate incoming media batches, ensure checksums, and fingerprint files for future training.
- Run iterative highlight discovery loops (hero shots, sponsor beats, VO cues).
- Assemble recap / highlight timelines using reusable iMovie templates and AppleScript automations.
- Render multi-aspect deliverables (16:9, 9:16, 1:1) and tee up upload metadata.
- Learn from creative-director feedback to tighten pacing, color grades, and overlays.

## Workflow Architecture
Editron runs as a multi-tier workflow so every deliverable is traceable, tunable, and resilient when event constraints change mid-stream.

### Tier 0 — Intake & Compliance
- Nora (or the operator) calls `ingest_media_batch` with batch metadata; Editron fingerprints each file, validates checksums, and writes a `MediaBatch` record before downstream tools can access it.
- Storage routing: cold assets stay in Dropbox/S3; hot proxies are hydrated inside the proxy farm with `storage_tier` + `retention_policy` hints so fast iteration never blocks on full-resolution pulls.

### Tier 1 — Batch Intelligence
- `analyze_media_batch` fans out across the clip list to run transcript+caption passes, shot detection, and sponsor cue tagging.
- Results are written into structured briefs (JSON + Markdown) that can be consumed by recap, highlight, and social hook planners. The briefs include energy curves, crowd response timestamps, and must-use directives from the client.

### Tier 2 — Story Forge
- A `story_blueprint` object merges creative brief data, hero shots, and sponsor beats into timeline payloads for each deliverable.
- Editron can spawn multiple blueprint variants per deliverable target (recap vs. hype vs. social hook) before committing render hours. Variant metadata lets humans choose quickly if turnaround is extremely tight.

### Tier 3 — Motion Systems & Transitions
- `generate_video_edits` materializes the blueprints into working timelines. AppleScript automations create compound clips, apply LUT/motion-template packs, and annotate where manual polish might still be required.
- Transition packs, lower-thirds, and stinger animations are parameterized so Editron can swap typography palettes or animation speed without rewriting scripts.

### Tier 4 — Render & Delivery
- `render_video_deliverables` reads the edit session manifest, assigns render queue priorities, and exports the requested aspect ratios with per-destination profiles (Dropbox delivery, Frame.io review, S3 cold archive).
- Upload adapters push status events back into Nora so command center operators see ETA, bitrate, and QC flags in real time.

### Tier 5 — Feedback & Memory
- Approved deliverables, inline comments, and rejection reasons loop into Editron memory. The workflow stores pace adjustments, grade tweaks, and typography overrides alongside the originating project/brand pod.
- The memory layer feeds future Tier 1 + Tier 2 passes so Editron automatically leans toward historically accepted looks and transitions for that client.

## Toolchain
| Stage | Nora Tool & Contract | Implementation Hook | Observability / Notes |
| --- | --- | --- | --- |
| Intake | `ingest_media_batch` payload defines storage tier, compliance flags, retention | Tool definition lives with other MCP tools in `crates/server/src/mcp/nora_tools.rs:1`; execution lands in a new `media::ingest` module under `crates/executors`. | Emits SSE events through the existing process stream so `frontend/src/contexts/ProcessSelectionContext.tsx:11` can highlight ingest health, retries, and checksum status, and it drops an “Ingest media batch” Kanban card (tags `editron`, `ingest`) in the selected project. |
| Analysis | `analyze_media_batch` accepts `scope` (recap, highlight, social), `model_hint`, and `max_iterations`. | Batch-intelligence jobs run inside `crates/executors` with optional GPU dispatch metadata stored in `crates/server/src/routes/execution_processes.rs`. Results are persisted via `crates/db`. | Attaches briefs + shot lists to tasks so operators can open them inside command drawer dialogs, and every pass spawns an “Analyze media batch” card referencing the brief. |
| Assembly | `generate_video_edits` references `story_blueprint_id`, template pack, and automation profile. | AppleScript bridge + LUT catalogs are bundled in `assets/` and `dev_assets/`; invocation orchestrated by a background worker defined in `crates/nora`. | Task attempt logs stream to the MCP event bus, and an “Assemble edits” card links to the active edit session for collaboration. |
| Render | `render_video_deliverables` takes multi-destination payloads, quality profiles, and rush priority. | Render orchestrator packaged as `render_queue` service under `crates/executors`; output artifacts uploaded via adapters declared in `shared`. | Delivery adapters push final URLs + QC hashes back into task attachments so downstream automations (social scheduler) can auto-trigger, and the “Render deliverables” card closes automatically once exports pass QC. |
| Feedback/Memory | `record_editron_feedback` (new) stores approvals, declines, and creative notes keyed by brand pod. | Memory service implemented as `feedback_registry` in `crates/services`; surfacing through MCP enables Nora to cite historical context. | Quality telemetry is summarized in `frontend/src/pages` analytics views so leads can monitor throughput and acceptance rate trends. |

### External Services
- **Dropbox Content Webhook** → triggers Nora with the batch link + metadata JSON.
- **Proxy Farm** → ffmpeg workers bake lightweight proxies + waveform thumbnails; uses `storage_tier` hint from the tool request.
- **AppleScript Runner** → automates iMovie timeline creation, compound clips, audio ducking, and export presets. Returns an `edit_session_id` consumed by `render_video_deliverables`.
- **Render Queue** → wraps Compressor or ffmpeg to export the requested formats/destinations with priority controls (`low`, `standard`, `rush`).
- **Review Layer** (optional) → Frame.io webhook posts ready-to-review links back into Nora for QA or stakeholder approval.

## Engineering Experience Inside PCG Dashboard MCP
Designing Editron within this repo means leaning on the existing MCP, process tracking, and asset vault primitives rather than inventing new rails:

- **Backend MCP surface** — Define Editron tools beside the other Nora utilities in `crates/server/src/mcp/nora_tools.rs:1`, then back them with typed handlers inside `crates/server/src/routes`. Each handler should enqueue long running jobs through `crates/executors`, giving us retry policies, structured logging, and consistent process IDs for telemetry.
- **Executor lanes** — Place ingest, analysis, assembly, and render workers under purpose-built modules inside `crates/executors`. When GPU or media-specific dependencies are required, ship container images referenced by `deploy/` manifests so command center operators can elastically scale render pods.
- **Shared contracts** — Model payloads (e.g., `MediaBatch`, `StoryBlueprint`, `DeliverableManifest`) in `crates/db` and re-export TypeScript bindings with `npm run generate-types`, so both `frontend/src/lib/api` and other agents read identical schemas stored in `shared/types.ts`.
- **Command center UX** — Surface Editron state through existing drawers/dialogs (`frontend/src/components/dialogs/tasks` and `frontend/src/components/dialogs/global`) plus timeline analytics views under `frontend/src/pages`. The process context provider at `frontend/src/contexts/ProcessSelectionContext.tsx:11` already drives SSE updates; simply ensure Editron processes publish granular milestones (ingest complete, analysis pass 2 done, renders on queue, review link ready).
- **Asset + brand pods** — Reuse the Brand Pod + Asset Vault logic documented in `README.md` to pin sponsor overlays, color profiles, and typography packs to each client. Editron tools should pull these from the asset vault first before falling back to defaults, ensuring consistent creative output per engagement.
- **Observability & governance** — Embrace existing `health-check.sh` + `workflows/` automation for smoke tests. Add targeted `cargo test` suites for Editron modules and ensure Frame.io/Dropbox credentials live in `.env` templates, keeping secrets out of git. Finally, tie accept/reject telemetry into Nora metrics (`crates/server/src/nora_metrics.rs`) so leadership can see throughput, SLA adherence, and revision counts.

With this architecture we keep the agent adaptable—operators can swap creative packs or raise automation levels per project—while still enforcing a meticulous, logged, and testable pipeline that the command center can trust every night.

## User Flow Example
1. User: “Nora, we just got last night’s event footage. Here’s the Dropbox link, need a recap + three highlight reels.”
2. Nora maps the request to Editron and calls `ingest_media_batch` with the Dropbox URL and reference label.
3. After ingest completes, Editron runs `analyze_media_batch` twice—first for recap story beats, second for high-energy clips. Outputs shot lists and transcript moments.
4. Editron triggers `generate_video_edits` for target deliverables (recap + highlight set). The AppleScript runner bootstraps iMovie timelines, applies presets, and reports an `edit_session_id`.
5. Once the creative director (or automated QA) approves, Nora fires `render_video_deliverables` with the destination list (Dropbox delivery folder + Frame.io review link). Render queue pushes progress back to Nora events.
6. Future training data: notes, approved exports, and telemetry feed back into Editron’s memory so subsequent events cut faster.

## Future Enhancements
- Plug in Frame.io review callbacks to auto-close tasks when reviewers approve.
- Add audio mastering microservice so Editron can normalize stems before export.
- Expand tool parameters with LUT selection, typography packs, and motion-template overrides.
- Track per-event model fine-tunes (lighting presets, sponsor overlays) for compounding quality gains.
