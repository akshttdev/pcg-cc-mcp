# Virtual Environment Sister Platform

A “Tron-inspired” virtual environment that sits beside the PCG CC MCP: the MCP remains the authoritative ledger/logs/API surface, while the virtual world is a synchronized, explorable slice of the same state. Users can fluidly swap between the browser dashboard and an immersive 3D space, issuing voice or gesture commands that compile down to MCP actions.

---

## Initial Immersive Layout

- **Space**: spawn users inside a seemingly infinite dusk-lit grid that conveys the Tron aesthetic; horizon lines and volumetric fog reinforce scale while keeping focus on actionable structures.  
- **PCG Command Center Spawn**: the entry portal is the PCG Command Center, modeled as an office pavilion floating above the grid. NORA’s avatar resides here as the resident agent, greeting arrivals, narrating recent MCP activity, and bridging voice commands into the orchestration stack.  
- **Directory Cubes**: every top-level project under `/home/spaceterminal/topos/` (e.g., `pcg-cc-mcp`, `duck-rs`, `cable-com`, `jungleverse`, `ducknet`, `hourglass-extracts`, `ComfyUI`, `duck-linux-distribution`) is rendered as a monumental cube resting on the grid. The cube’s label and glow intensity reflect live MCP metrics (tasks, alerts) when available.  
- **Project Alcoves**: stepping through a cube face dives into that project’s dedicated island/pod layout, letting us iterate on finer spatial metaphors later without rebuilding the whole hub.  
- **Future Expansion Hooks**: reserve anchor points for additional structures (data towers, pipelines) so subsequent iterations can attach specialized pods without moving the core spawn or cube grid.

This layout ensures the very first build demonstrates: (1) MCP integration via the Command Center office, (2) presence of NORA as an interactive agent, and (3) a tangible overview of every repo/project already in the workspace.

---

## Guiding Principles

1. **Mirrored state** – every entity in the virtual world is backed by MCP data; no drift.
2. **Voice-first orchestration** – high-level intent enters through the Voice Orchestrator, not manual menus.
3. **Agent legibility** – Orchestration + Sub Agents appear as actors with visible status, context, and affordances.
4. **Composable structures** – islands, districts, nodes, and conduits form a spatial language for pods, tasks, dependencies, and data flows.
5. **Dual modality** – everything you can see or do in 3D is reflected in the browser experience and vice versa.

---

## Core User Flow (Tron Loop)

1. **Voice Orchestrator speaks intent** – the user (or a narrator persona) issues natural-language goals (“Spin up a marketing sprint for Jupiter Foods and route motion design to the glyph studio”).
2. **Orchestration Agent plans** – parses the intent, consults MCP state, assembles a command graph, and reports back with a visual pulse in-world plus a textual summary in the browser log.
3. **Sub Agents execute** – specialized agents (research, build, compliance, comms, etc.) pick up execution nodes. In-world, they animate along data conduits; in MCP, they append structured updates.
4. **World + MCP stay in sync** – every action updates MCP first; the world listens to the same event bus and refreshes geometry, effects, and overlays instantaneously.
5. **User nudges in either venue** – dragging an object, tagging an agent, or editing the task card in the dashboard emits the same orchestration events, keeping parity.

ASCII interaction loop:
```
Voice Input → Orchestration Agent → Command Graph → Sub Agents
     ↑               ↓                   ↓               ↘
Virtual World UX ← Event Stream ← MCP Ledger/API ← Browser UI
```

---

## Entity + Structure Vocabulary

| MCP Concept | Virtual Analog | Notes |
| --- | --- | --- |
| Workspaces / Clients | Floating “Systems” platforms | Color-coded shells that encase every artifact tied to that client.
| Pods / Brand Pods | Faceted towers inside each system | Each facet = function (dev, design, ops) with docking pads for agents.
| Tasks / Tickets | Light disks or data glyphs | Hover near pods, enlargeable to reveal logs, attachments, and owners.
| Dependencies | Energy conduits | Pulsing intensity illustrates throughput or blockage.
| Agents | Avatars (humans) or signature drones (AI) | Halos show role, trust level, and current objective.
| Voice Orchestrator | Central beam / dais | Always-on presence, transcribes intents, glows when listening.
| Orchestration Agent | Control spire | Projects the current command graph; users step onto balcony to inspect.
| MCP Logs | Time ribbon hovering above world | Scroll through a timestamped strip; selecting a log replays state.

---

## Agent Fabric

- **Voice Orchestrator (VO)**  
  - Multimodal listener (mic + text).  
  - Performs intent classification + semantic grounding.  
  - Emits Orchestration Requests to the MCP via gRPC/WebSocket.

- **Orchestration Agent (OA)**  
  - Lives as both an MCP service and a rendered avatar.  
  - Builds and maintains the command DAG, hands nodes to Sub Agents, and keeps status heatmaps updated in-world.

- **Sub Agents (SA)**  
  - Domain specialists (Builder, Researcher, Liaison, Reviewer).  
  - Each runs either on the MCP agent framework or proxies to external tools; progress is visualized as traversal along conduits.

- **World Runtime**  
  - Receives MCP event envelopes, reconstitutes them into ECS (entity component system) entries, and animates assets.  
  - For XR-ready builds, sits on Unity/Unreal; for browser 3D, start with Three.js + WebGL + WebXR fallback.

---

## Technical Architecture

### Data + Sync Layer

- **Event Source**: MCP emits structured events (task_created, agent_status_changed, plan_step_completed).  
- **Scene Compiler**: Rust or Node microservice subscribes to events, maps them into scene diffs (entities, transforms, effects).  
- **State Cache**: Redis/SQLite snapshots for quick reconnect; version each diff for rewind/time-travel.  
- **Log Bridge**: Same events feed the MCP browser UI, enabling “click log entry → highlight object in world”.

### Interaction Layer

- **Voice Stack**: WebRTC mic capture → speech-to-text (local or cloud) → intent service.  
- **Gesture/Pointer Stack**: Raycasting inside engine; emits actions to the Interaction Gateway, which translates them into MCP API calls.

### Rendering Layer

- **Prototype**: Browser-first WebGL (react-three-fiber) to minimize install friction.  
- **Immersive**: Unity/Unreal scene consuming identical scene diff stream; optional XR controllers.  
- **UI Overlay**: In-world HUDs for metrics, real-time subtitles for VO, and log ribbons referencing MCP timestamps.

---

## Security + Permissions

- Reuse MCP auth tokens; the 3D client exchanges them for WebSocket capabilities.  
- Presence broadcasts omit sensitive payloads; details fetched on-demand per user scopes.  
- Voice commands respect user ACLs before instantiating orchestration changes.

---

## Delivery Roadmap

1. **Phase 0 – Scene Diff Adapter (2 weeks)**  
   - Extend MCP to publish the canonical event stream.  
   - Build Scene Compiler that mirrors DuckSpace schema but adds VO/OA/SA entities.

2. **Phase 1 – Observer Mode (4 weeks)**  
   - WebGL client that visualizes systems, pods, tasks, and agents; read-only log ribbon.  
   - Simulated agent avatars driven by historical MCP data to validate pacing.

3. **Phase 2 – Voice + Command Graph (5 weeks)**  
   - Plug in Voice Orchestrator UI + transcription.  
   - Show Orchestration Agent spire with live command DAG; still limited write-back.

4. **Phase 3 – Bidirectional Control (6 weeks)**  
   - Enable drag-and-drop + gestures to trigger MCP mutations (assign, reprioritize, spawn Sub Agent).  
   - Browser dashboard gains “Send to World” buttons for parity.

5. **Phase 4 – XR + Multi-presence (ongoing)**  
   - Unity/Unreal thin client, shared presence, spatial voice chat, cinematic effects.

---

## Immediate Next Actions

1. Define the MVP entity schema (VO, OA, SA, structures) in TypeScript/Rust shared types.  
2. Stand up the Scene Compiler crate/service using existing MCP event bus.  
3. Build a minimal WebGL proof-of-concept that displays one client system, one pod, and animates a single Sub Agent route based on live MCP data.  
4. Prototype the Voice Orchestrator UI (mic widget + transcript) inside the MCP frontend to reuse auth + session plumbing.
