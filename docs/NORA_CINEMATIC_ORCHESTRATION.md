# Nora → Cinematographer → Stable Diffusion Orchestration

## Current Baseline
- **Executive interface** – `frontend/src/components/nora/NoraAssistant.tsx` already wires the chat UI into `POST /api/nora/chat`, but the payload only supports free-form text/voice; there is no notion of attaching `ProjectAsset` references yet.
- **Backend entrypoint** – `crates/server/src/routes/nora.rs` exposes the Nora REST surface. All coordination hooks ultimately call into `crates/nora/src/agent.rs` and its `CoordinationManager` (`crates/nora/src/coordination.rs`). No additional agents are registered, so the event bus never emits task handoffs.
- **Asset vault** – `crates/db/src/models/project_asset.rs` plus the `/api/projects/:id/assets` routes (defined in `crates/server/src/routes/projects.rs`) already let us store source footage, storyboards, voiceovers, etc. The React `ProjectDetail` screen surfaces uploads, but Nora cannot currently fetch or reference those assets.
- **Stable Diffusion stack** – `/home/spaceterminal/topos/ComfyUI` ships with AnimateDiff (`custom_nodes/ComfyUI-AnimateDiff-Evolved`) and VideoHelperSuite (`custom_nodes/ComfyUI-VideoHelperSuite`). The HTTP API (`server.py`, `/prompt` route) is callable exactly as shown in `script_examples/basic_api_example.py`. No orchestration layer consumes this API yet.

## Target Outcome
Allow a user to brief Nora with concept text plus references. Nora packages that brief and hands it to a dedicated **Master Cinematographer Agent**. That agent researches, plans shots, assembles ComfyUI workflows (including AnimateDiff + VideoHelper nodes), triggers renders, and publishes the resulting clips back into the MCP (as `ProjectAsset`s + Nora status updates).

## Proposed Flow
1. **Brief Collection** – User opens Nora, selects "Cinematic Brief" mode, and either uploads files or selects existing `ProjectAsset`s. (`frontend/src/components/nora/NoraAssistant.tsx` gains an attachment picker that calls `/api/projects/:id/assets`).
2. **Brief Serialization** – `POST /api/nora/cinematics/briefs` stores a `CinematicBrief` record (new table) containing: summary, project_id, asset_ids, desired duration, tone, deliverables, and any textual script.
3. **Coordination Event** – Nora's `CoordinationManager` registers the Master Cinematographer agent on startup and, when a new brief arrives, emits a `TaskHandoff` event with the brief ID + context. (`crates/nora/src/agent.rs` + `coordination.rs`).
4. **Master Cinematographer Agent** – Implemented as a new Rust crate (e.g., `crates/cinematics`). Responsibilities:
   - Pull the `CinematicBrief` record + linked `ProjectAsset` metadata via the DB service.
   - Call the existing `LLMClient` with a specialized system prompt to produce a `ShotPlan` (scene breakdown, prompt scaffolds, camera moves, lenses, reference art direction).
   - Resolve model resources (checkpoint, VAE, LoRA, AnimateDiff module) against the ComfyUI filesystem (e.g., `ComfyUI/models/checkpoints`, `custom_nodes/ComfyUI-AnimateDiff-Evolved/models`).
   - Fill a workflow template stored under `pcg-cc-mcp/assets/cinematics/workflows/*.json` (exported via "File → Export (API)" in ComfyUI) by injecting per-shot prompts, seeds, and VideoHelper settings.
   - Queue the workflow via `POST http://127.0.0.1:8188/prompt` (see `ComfyUI/script_examples/basic_api_example.py`), poll `/history/{prompt_id}`, and copy the resulting video(s) from `ComfyUI/output` into the project's asset vault.
   - Emit progress back to Nora through `CoordinationEvent::AgentStatusUpdate` + `TaskHandoff` completion payloads.
5. **Result Surfacing** – Once the render completes, the agent:
   - Registers the clip as a `ProjectAsset` (category `"ai_short"`, scope `"cinematics"`).
   - Adds a Nora `ExecutiveAction` with type `"CinematicDelivery"` so the chat UI can show download links.
   - Optionally opens a `Task` with deliverable metadata via `TaskExecutor` if human review is required.

## Component Additions

### Data Model Extensions
- `CinematicBriefs` table
  - Fields: `id`, `project_id`, `requester_id`, `nora_session_id`, `title`, `summary`, `script`, `asset_ids[]`, `duration_seconds`, `fps`, `style_tags[]`, `status`, `llm_notes`, `render_payload` (json), `output_assets[]`, timestamps.
  - Implemented in a new SQLx migration plus `crates/db/src/models/cinematic_brief.rs`.
- `ShotPlans` table (optional but recommended) keyed by `brief_id` for fine-grained tracking of each shot's metadata.

### Backend Services
1. **New crate `crates/cinematics`**
   - Modules: `briefs` (DB helpers), `planner` (LLM prompt templates), `workflow` (JSON templating + ComfyUI API client), `renderer` (status polling + asset ingestion).
   - Uses `reqwest` to hit the ComfyUI server and `tokio::fs` to copy artifacts.
   - Expose a `CinematicsService` trait so `crates/server` can drive it.
2. **Server routes** (`crates/server/src/routes/cinematics.rs`)
   - `POST /api/nora/cinematics/briefs` – create brief (used by Nora + UI).
   - `GET /api/nora/cinematics/briefs/:id` – status, including shot plan + outputs.
   - `POST /api/nora/cinematics/briefs/:id/render` – manual trigger / re-render.
   - `GET /api/nora/cinematics/briefs` – list per project.
3. **Coordination glue**
   - Register agent once: `coordination_manager.register_agent(AgentCoordinationState { agent_id: "master_cinematographer", agent_type: "Master Cinematographer", capabilities: vec!["cinematic_planning", "animatediff_workflow", "video_post"], .. })` in `NoraAgent::new` (after coordination manager init).
   - Add helper in `crates/nora/src/agent.rs` to convert a chat request tagged as `CinematicBrief` into the API call mentioned above, then emit `TaskHandoff` with `context = {"briefId": ...}`.

### Frontend Updates
- Nora UI: add a mode toggle + attachments drawer referencing `ProjectAsset`s.
  - Extend `NoraRequest` payload to include `context.assetIds` and `requestType: "cinematicBrief"`.
  - Display cinematic plan progress by tapping `NoraResponse.actions` (new type `"CinematicDelivery"`).
- Project detail page: show `CinematicBriefs` timeline (status chips, preview thumbnails) and allow launching new briefs outside the chat UI.

### Stable Diffusion / ComfyUI Integration
- Standardize launch command in docs:
  ```bash
  cd /home/spaceterminal/topos/ComfyUI
  python main.py --listen 0.0.0.0 --port 8188 --enable-cors-header
  ```
- Store base workflows inside `assets/cinematics/workflows/` (export via ComfyUI "File → Export (API)" and keep JSON under git). Provide at least:
  1. **AnimateDiff Short Loop** – text2video using `AnimateDiff-Evolved` + `VideoHelperSuite::VideoCombine`.
  2. **IP-Adapter Driven Storyboard** – uses reference frames.
  3. **Img2Img Motion Remix** – accepts uploaded footage.
- Include README snippets referencing `ComfyUI/custom_nodes/ComfyUI-AnimateDiff-Evolved/README.md` for model placement, ensuring team members know where to drop motion modules.

## Implementation Phases
1. **Scaffolding**
   - SQL migration for cinematic tables.
   - New `cinematics` crate with service skeleton + config struct (`StableDiffusionConfig { base_url, default_workflow, output_dir }`) stored in `Config` (extend `crates/services/src/services/config/versions/v8.rs`).
2. **API + Nora Hooks**
   - Backend routes + Nora request/response schema updates.
   - Frontend UI to create briefs and select assets.
3. **Cinematographer Agent Core**
   - Planner: LLM prompt templates referencing provided assets + research hints.
   - Workflow builder: load JSON template, patch nodes (prompt text, negative prompt, seeds, fps, context window).
   - Renderer: queue prompt via ComfyUI HTTP client, poll `/history`, parse outputs, and register `ProjectAsset`s.
4. **Delivery + Review Loop**
   - Add `ExecutiveAction` mapping for cinematic deliveries, enabling Nora to share download links and optionally schedule reviews (create `Task` in `TaskExecutor`).
   - Provide SSE or WebSocket updates so the Nora UI can stream progress.
5. **Quality + Toolchain**
   - Seed example briefs and template shot plans for regression testing.
   - Add integration tests that mock the ComfyUI API (use `wiremock` or `httpmock`) to confirm JSON payloads.

## Open Questions & Dependencies
- **LLM Provider** – reuse Nora's configured provider (`LLMClient`) or provision a separate key/model better tuned for creative prompting (e.g., `gpt-4.1` or `o1`). Decide per project via config.
- **Asset Storage** – large renders may overwhelm SQLite path references. Confirm whether to mirror outputs into an object store or keep within `ComfyUI/output` + symlinks.
- **Scheduling / Retry** – define how to retry failed renders, log GPU availability, and throttle queue submissions if ComfyUI is already busy.
- **Research Hooks** – Cinematographer agent may need external mood board data (Pinterest, Shutterstock). Determine if this should be a separate MCP tool for compliance reasons.

Locking this architecture in gives us a clear path: once the brief schema, ComfyUI templates, and Cinematographer agent skeleton land, we can iterate on shot-planning prompts and rendering fidelity without touching Nora's executive core.
