# Research: Offline-First, Local-First, and Edge Computing Patterns for ORCHA

**Date**: 2026-03-19
**Type**: Research
**Scope**: Technology evaluation for offline/local-first capabilities across ORCHA roadmap stages 0-5
**Status**: Complete

---

## Executive Summary

ORCHA's architecture (Rust/Axum + React/Vite + SQLite) is naturally positioned for local-first patterns. SQLite is already the gold standard for local storage, and Rust compiles to both native binaries and WebAssembly. This research evaluates 30+ tools and patterns across 7 domains, mapping each to the appropriate roadmap stage with concrete recommendations.

**Key strategic recommendations:**
1. **Stage 0-1**: PWA with Workbox + optimistic updates (low effort, immediate UX win)
2. **Stage 2**: Turso/libSQL as the bridge between local SQLite and cloud PostgreSQL
3. **Stage 3**: Tauri 2.0 desktop app with embedded SQLite + Ollama for local LLM
4. **Stage 4**: Edge deployment via Cloudflare Workers + Turso embedded replicas
5. **Stage 5**: Full sovereignty mode with air-gapped deployment, CRDT sync, and customer-managed encryption

**Total estimated cost to implement all stages**: $0 in licensing (all MIT/Apache 2.0), ~18-24 engineer-months spread across stages 1-5.

---

## Table of Contents

1. [Local-First Architecture](#1-local-first-architecture)
2. [Offline Capabilities](#2-offline-capabilities)
3. [Local LLM Integration](#3-local-llm-integration)
4. [Edge Computing](#4-edge-computing)
5. [Sync Architecture](#5-sync-architecture)
6. [Desktop/Mobile Packaging](#6-desktopmobile-packaging)
7. [Sovereignty Mode](#7-sovereignty-mode)
8. [Recommended Implementation Order](#8-recommended-implementation-order)
9. [Risk Register](#9-risk-register)

---

## 1. Local-First Architecture

### 1.1 CRDTs for Conflict Resolution

#### Automerge (MIT)

**What it is**: A JSON-like CRDT data structure library with a Rust core compiled to WASM for JavaScript. Supports nested objects, arrays, text, and counters. Automerge 3 achieved ~10x reduction in memory usage over v2.

**Pros**:
- Rust core = same logic across all platforms (native, WASM, mobile FFI)
- Rich data types (JSON-like structures, not just text)
- Mature sync protocol with efficient binary encoding
- Active development (Automerge 3 in 2025-2026)
- Designed for exactly the local-first pattern ORCHA needs

**Cons**:
- Document size grows with edit history (compaction helps but doesn't eliminate)
- Complex merge semantics for structured data (list reordering, map conflicts)
- Learning curve for CRDT mental model vs. traditional DB
- No built-in server persistence layer (you build your own)

**License**: MIT
**Roadmap fit**: Stage 3-4 (for collaborative editing features, workflow co-authoring)
**Budget impact**: 2-3 engineer-weeks to integrate for specific collaborative features
**Rationale**: Overkill for Stage 0-2 where single-user or turn-based collaboration suffices. Becomes valuable when multiple users edit workflows/documents simultaneously.

#### Yjs (MIT)

**What it is**: High-performance JavaScript CRDT library for collaborative applications. Exposes shared data types (Map, Array, Text, XML) that sync automatically. Lightweight (~30KB) with extensive editor integrations (ProseMirror, Monaco, CodeMirror, Lexical).

**Pros**:
- Largest ecosystem of integrations (editors, frameworks, sync providers)
- Decentralized by design (no central server required)
- React integration via SyncedStore or custom hooks using `useSyncExternalStore()`
- Offline-first with IndexedDB persistence built in
- WebRTC, WebSocket, and custom network providers
- Battle-tested in production (used by major collaborative apps)

**Cons**:
- JavaScript-only core (no Rust native, though WASM bridges exist)
- Garbage collection of deleted content is complex
- Document state grows over time (tombstones for deleted items)
- Less suitable for structured database-like data (better for documents/rich text)

**License**: MIT
**Roadmap fit**: Stage 3 (collaborative rich text editing, workflow annotations)
**Budget impact**: 1-2 engineer-weeks for basic integration
**Rationale**: Best choice if ORCHA needs collaborative text editing (specs, notes, documentation within the platform). Yjs + Lexical/ProseMirror is the standard stack.

#### diamond-types (Apache 2.0)

**What it is**: The world's fastest CRDT implementation, written in Rust. Claims 5000x faster than competing CRDTs (and further 10-80x improvements since). Focuses on text editing with sophisticated B-Tree and run-length encoding optimizations.

**Pros**:
- Pure Rust = perfect fit for ORCHA backend
- Extraordinary performance (O(log n) position mapping)
- Minimal memory footprint via run-length encoding
- Apache 2.0 license (compatible with any licensing strategy)

**Cons**:
- **Text-only** (no support for JSON/structured data yet — work in progress)
- Smaller community than Automerge/Yjs
- Still WIP for non-text data types
- No ecosystem of sync providers or editor integrations

**License**: Apache 2.0
**Roadmap fit**: Stage 5 (if ORCHA needs ultra-high-performance text sync for code editing features)
**Budget impact**: 3-4 engineer-weeks (building integration layer from scratch)
**Rationale**: Monitor for when structured data support ships. Currently too narrow for ORCHA's needs.

#### CRDT Recommendation

| Use Case | Recommended | Stage |
|----------|------------|-------|
| Collaborative rich text (specs, notes) | **Yjs** | 3 |
| Collaborative structured data (workflows, CRM) | **Automerge** | 3-4 |
| Ultra-performance text sync | **diamond-types** | 5 (monitor) |

---

### 1.2 Sync Protocols

#### Replicache Pattern / Zero (MIT)

**What it is**: Replicache pioneered the "client-side mutator + server-side push/pull" pattern for local-first web apps. Now in **maintenance mode** — the team has shifted to **Zero**, a reactive sync engine that brings the same pattern with less boilerplate.

**Pros**:
- Proven pattern: optimistic mutations, server reconciliation, conflict resolution
- Zero (successor) provides reactive queries with automatic sync
- Now open-source and free (was previously paid)
- Great developer experience for React apps

**Cons**:
- Replicache itself is maintenance-mode (migrate to Zero)
- Zero is still young (2024 launch)
- Requires specific server-side implementation (push/pull endpoints)
- JavaScript-only (no Rust server SDK)

**License**: MIT (open-sourced)
**Roadmap fit**: Stage 2-3 (evaluate Zero as sync engine for web client)
**Budget impact**: 2-3 engineer-weeks
**Rationale**: The *pattern* is more important than the library. ORCHA can implement the mutator + push/pull pattern using existing Axum endpoints without depending on Replicache/Zero directly.

#### PowerSync (Apache 2.0)

**What it is**: A Postgres-to-SQLite sync layer for local-first apps. Backend-database-agnostic (Postgres, MongoDB, MySQL). Keeps backend DB in sync with on-device SQLite databases via client SDK.

**Pros**:
- **Direct Postgres-to-SQLite sync** — exactly ORCHA's migration path
- Self-hostable (Open Edition) or managed cloud
- Sync Streams (Beta, 2025) for on-demand selective sync
- Works with existing PostgreSQL schema (no CRDT migration)
- Client SDKs for React, React Native, Flutter, Swift, Kotlin
- Handles conflict resolution at the sync layer

**Cons**:
- Adds infrastructure dependency (PowerSync service between Postgres and clients)
- Sync rules configuration has a learning curve
- React SDK maturity is behind Flutter SDK
- Write path goes through cloud (not pure local-first for writes)

**License**: Apache 2.0 (Open Edition), proprietary (Enterprise features)
**Roadmap fit**: **Stage 2** (when PostgreSQL migration happens)
**Budget impact**: 2-3 engineer-weeks integration + $0-99/mo hosting
**Rationale**: Strong candidate for bridging ORCHA's SQLite local ↔ PostgreSQL cloud story. Evaluate alongside ElectricSQL at Stage 2.

#### ElectricSQL (Apache 2.0)

**What it is**: Postgres sync engine handling partial replication, data delivery, and fan-out. Originally built for Postgres-to-SQLite sync, now repositioned as a "data platform for multi-agent" systems. Includes PGlite (Postgres in WASM) and TanStack DB.

**Pros**:
- **Native Postgres integration** (reads the WAL directly)
- Partial replication (sync only relevant tenant data)
- Electric Cloud in public beta (managed service)
- PGlite companion: full Postgres in the browser via WASM (<3MB gzipped)
- Active development with AI/agent use case focus
- Companies like Trigger.dev using in production (millions of updates/day)

**Cons**:
- Relatively young (public beta as of 2025)
- Architecture has pivoted multiple times (originally active-active sync, now read-path focused)
- Write path still goes through server (not bidirectional CRDT sync)
- PGlite is promising but browser Postgres has overhead vs. SQLite

**License**: Apache 2.0
**Roadmap fit**: **Stage 2-3** (evaluate as Postgres sync engine)
**Budget impact**: 2-3 engineer-weeks + $0-49/mo cloud
**Rationale**: The PGlite + Electric combo is compelling if ORCHA goes full-Postgres (even in the browser). If ORCHA keeps SQLite for local, PowerSync is a better fit. If ORCHA wants Postgres everywhere, ElectricSQL wins.

---

### 1.3 Local-First Web Storage

#### OPFS (Origin Private File System)

**What it is**: Browser API for high-performance file storage. Unlike IndexedDB, supports synchronous I/O in Web Workers and binary file access. Enables SQLite WASM to run with proper file-based persistence.

**Pros**:
- Native file-system-like performance in the browser
- Enables SQLite WASM with proper persistence (not IndexedDB shim)
- WAL-mode-like concurrency via OPFSPermutedVFS
- No size limits (beyond device storage)

**Cons**:
- Synchronous path only available in Web Workers (architectural constraint)
- Browser support: Chrome/Edge full, Firefox progressing, Safari lagging
- Incognito mode limits: Chrome ~100MB, Firefox/Safari disable entirely
- Relatively new API, patterns still evolving

**License**: Web standard (free)
**Roadmap fit**: Stage 2-3 (for browser-based offline storage)
**Budget impact**: 1 engineer-week (OPFS + SQLite WASM setup)

#### PGlite (Apache 2.0)

**What it is**: Full PostgreSQL compiled to WASM, under 3MB gzipped. Runs in browser, Node.js, Bun, Deno. Supports pgvector, live queries, and sync with ElectricSQL.

**Pros**:
- Full Postgres SQL compatibility in the browser
- Extensions (pgvector for embeddings)
- Persistence via OPFS or IndexedDB
- Used by Prisma and Google Firebase Data Connect
- Natural fit if cloud DB is PostgreSQL (same SQL dialect everywhere)

**Cons**:
- 3MB download overhead (vs. ~500KB for SQLite WASM)
- Higher memory usage than SQLite WASM
- Extension support is limited compared to server Postgres
- Still maturing (active development)

**License**: Apache 2.0
**Roadmap fit**: Stage 3 (evaluate as browser DB if full Postgres compatibility is needed)
**Budget impact**: 1-2 engineer-weeks
**Rationale**: Only choose PGlite over SQLite WASM if you need Postgres-specific features in the browser (e.g., pgvector for local embeddings). SQLite WASM is lighter and more battle-tested.

#### IndexedDB

**What it is**: Established browser storage API. Transactional, supports structured data, blobs, and indexes. Available in all modern browsers including Web Workers and Service Workers.

**Pros**:
- Universal browser support (including Safari, Firefox, all mobile)
- Mature ecosystem of wrappers (Dexie.js, idb)
- Works in Service Workers (critical for offline queuing)
- No size limits in practice (browser-managed quotas, typically 50%+ of disk)

**Cons**:
- Asynchronous-only API (callback/promise-based)
- Complex query patterns compared to SQL
- Performance inferior to OPFS for large datasets
- No relational queries (need to build joins manually)

**License**: Web standard (free)
**Roadmap fit**: Stage 1 (for offline queue, cached data, service worker storage)
**Budget impact**: 0.5 engineer-weeks (well-understood technology)

#### Storage Recommendation

| Stage | Approach |
|-------|----------|
| Stage 1 | IndexedDB for offline queue + cached API responses |
| Stage 2 | SQLite WASM + OPFS for structured offline data |
| Stage 3+ | Evaluate PGlite if Postgres-everywhere strategy is adopted |

---

## 2. Offline Capabilities

### 2.1 Service Worker Patterns for Offline React Apps

**What it is**: Service Workers intercept network requests, enabling caching strategies, offline fallbacks, and background operations. For React/Vite apps, Vite PWA plugin (`vite-plugin-pwa`) provides zero-config service worker generation.

**Key patterns for ORCHA:**

1. **Shell caching** (precache): Cache the React app shell (HTML, JS, CSS) so the app loads instantly even offline. Use Workbox `precacheAndRoute()` with the Vite build manifest.

2. **API response caching** (runtime): Cache API responses using strategies:
   - `NetworkFirst` for frequently updated data (tasks, CRM records)
   - `StaleWhileRevalidate` for semi-static data (user profiles, org settings)
   - `CacheFirst` for static data (workflow templates, schema definitions)

3. **Offline fallback**: When network and cache both miss, show an offline-specific UI rather than browser error. Workbox `offlineFallback()` recipe handles this.

4. **Navigation preload**: For `NetworkFirst` navigation requests, use navigation preload to start the network fetch in parallel with service worker startup, eliminating the SW boot delay.

**Pros**:
- Immediate perceived performance improvement (app loads from cache)
- Works with existing React/Vite stack (no architecture changes)
- Graceful degradation (online features just work, offline features degrade)
- Progressive enhancement (add offline capabilities incrementally)

**Cons**:
- Service Worker update lifecycle is complex (stale caches, version management)
- Cache invalidation is hard (stale data can confuse users)
- Storage quota management needed (browsers can evict caches)
- Testing offline scenarios requires discipline

**Roadmap fit**: **Stage 1** (low effort, high UX impact)
**Budget impact**: 1-2 engineer-weeks
**Rationale**: This is the lowest-hanging fruit. Adding a service worker with Workbox to the existing Vite build gives ORCHA offline app loading and basic data caching with minimal code changes.

### 2.2 Workbox (MIT)

**What it is**: Google's service worker toolkit. Most-used SW library (54% of mobile sites). Provides precaching, runtime caching strategies, background sync, and offline recipes.

**Key modules for ORCHA:**

| Module | Purpose | Priority |
|--------|---------|----------|
| `workbox-precaching` | Cache app shell at install time | P0 |
| `workbox-routing` | Match requests to caching strategies | P0 |
| `workbox-strategies` | NetworkFirst, StaleWhileRevalidate, CacheFirst | P0 |
| `workbox-background-sync` | Queue failed API writes, replay on reconnect | P1 |
| `workbox-expiration` | Auto-expire cached responses (by age or count) | P1 |
| `workbox-recipes` | One-line offline fallback, warm cache | P2 |

**Integration with Vite**: `vite-plugin-pwa` uses Workbox under the hood. Configuration in `vite.config.ts`:
```typescript
// Conceptual — not a code change, just illustrative
VitePWA({
  strategies: 'generateSW',
  workbox: {
    runtimeCaching: [
      { urlPattern: /\/api\//, handler: 'NetworkFirst' },
      { urlPattern: /\/assets\//, handler: 'CacheFirst' },
    ]
  }
})
```

**Pros**:
- Zero-config with `vite-plugin-pwa`
- Declarative caching strategies
- Built-in background sync queue
- Well-documented, large community
- TypeScript support

**Cons**:
- `generateSW` mode has limits (custom SW logic requires `injectManifest`)
- Background sync only works in Chromium browsers (Firefox/Safari fallback to poll)
- Cache storage counts toward browser quota

**License**: MIT
**Roadmap fit**: **Stage 1**
**Budget impact**: 1 engineer-week (basic), 2 weeks (full offline story)

### 2.3 Background Sync API

**What it is**: Web API that defers tasks to a service worker to execute when stable network connectivity is restored. Combined with Workbox `BackgroundSyncPlugin`, failed API requests are stored in IndexedDB and replayed automatically.

**Browser support (2026)**:
- Chrome/Edge: Full support
- Firefox: Partial (no periodic sync, basic sync supported)
- Safari: Not supported (falls back to manual retry on next SW activation)

**Pattern for ORCHA**:
1. User creates a task while offline
2. API call fails (network error)
3. Workbox `BackgroundSyncPlugin` stores the request in IndexedDB queue
4. When connectivity restores, browser fires `sync` event
5. Service worker replays queued requests
6. If replay fails, exponential backoff retry

**Pros**:
- Transparent to the user (they don't see the network failure)
- Works even if the user closes the tab (SW runs independently)
- Workbox handles all the queue management

**Cons**:
- Safari non-support means ~25% of users need fallback
- Queue replay order matters (create project before create task)
- Conflict resolution needed if server state changed during offline period
- Max queue lifetime is browser-managed (typically 24h for Chrome)

**Roadmap fit**: **Stage 1-2**
**Budget impact**: 1 engineer-week (with Workbox, most is configuration)

### 2.4 Conflict Resolution UI Patterns

**What it is**: When offline changes conflict with server state, the user needs to understand and resolve conflicts. Patterns range from "last write wins" (simple, lossy) to full merge UIs.

**Recommended patterns for ORCHA by data type:**

| Data Type | Conflict Strategy | UI Pattern |
|-----------|------------------|------------|
| Task status changes | Last-write-wins with timestamp | Toast notification: "Task was updated while offline, your change applied" |
| Task descriptions/notes | Three-way merge | Side-by-side diff view (like Git) |
| CRM field updates | Field-level merge | Highlight conflicting fields, let user choose |
| Workflow definitions | Lock-based (one editor at a time) | "Someone is editing this workflow" banner |
| File uploads | Keep both | "A newer version exists. Keep both?" dialog |

**Roadmap fit**: Stage 2-3 (basic last-write-wins in Stage 1, rich merge UI in Stage 3)
**Budget impact**: 1 week (Stage 1 simple), 3-4 weeks (Stage 3 rich merge)

### 2.5 Offline Queue Management

**Key design decisions for ORCHA:**

1. **Operation ordering**: Maintain a causal dependency graph. Task creation must precede task update. Use lamport timestamps or vector clocks.
2. **Idempotency**: All mutating API endpoints should be idempotent (use client-generated UUIDs, which ORCHA already does with `DbUuid::new()`).
3. **Queue persistence**: IndexedDB via Workbox (survives tab close, browser restart).
4. **Queue visibility**: Show users a "pending changes" indicator when items are queued.
5. **Queue limits**: Cap at ~1000 operations to prevent storage issues. Warn user at 80%.
6. **Error handling**: If a queued operation fails on replay (e.g., referenced entity was deleted), surface to user with actionable context.

**Roadmap fit**: Stage 1-2
**Budget impact**: 2 engineer-weeks (queue infrastructure + UI indicators)

---

## 3. Local LLM Integration

### 3.1 llama.cpp (MIT)

**What it is**: The de facto standard for local LLM inference in C/C++. 1200+ contributors, 4000+ releases. Supports GGUF model format with quantization from 1.5-bit to 8-bit. Hardware-optimized for Apple Silicon (Metal), NVIDIA (CUDA), AMD (ROCm), and x86 (AVX/AVX2/AVX512/AMX).

**Performance benchmarks (2025-2026):**

| Hardware | Model | Speed |
|----------|-------|-------|
| Apple M3/M4 | 7-8B params | 60-120 tok/sec |
| NVIDIA H100 | 70B params | 40-80 tok/sec |
| Consumer GPU (RTX 4090) | 13B params | 50-90 tok/sec |
| CPU-only (modern x86) | 7B params | 10-20 tok/sec |

**Pros**:
- Broadest hardware support of any inference engine
- Active community with rapid model format adoption
- GGUF quantization reduces memory by 50-75% with minimal quality loss
- Can be embedded via FFI into Rust backend
- Supports speculative decoding, context caching, grammar-constrained output

**Cons**:
- C/C++ codebase (FFI overhead, memory safety concerns)
- Model management is manual (download, quantize, configure)
- No built-in API server (use llama-server or wrap with Ollama)
- Quantized models have measurable quality degradation for complex tasks

**License**: MIT
**Roadmap fit**: **Stage 3** (embedded in Tauri desktop app for offline agent capabilities)
**Budget impact**: 2-3 engineer-weeks (FFI integration + model management)
**Rationale**: Use via Ollama for development/desktop, directly embed for Tauri distribution.

### 3.2 Candle (MIT/Apache 2.0)

**What it is**: Hugging Face's minimalist ML framework for Rust. Inference-first (training is experimental). Compiles to single megabyte-sized binaries. First-class WASM support for browser inference.

**Supported models**: LLaMA, Falcon, Mistral, Phi, StableLM, Gemma, Whisper, Stable Diffusion.

**Pros**:
- **Pure Rust** = native integration with ORCHA backend (no FFI)
- Compiles to WASM (browser inference without llama.cpp)
- Millisecond startup (no Python runtime)
- LoRA fine-tuning support
- Hugging Face model hub integration
- Multi-backend: CPU, CUDA, Metal

**Cons**:
- Smaller model zoo than llama.cpp/Ollama
- Inference performance behind llama.cpp for LLMs (optimized for different workload)
- Training support is experimental
- Less community adoption than llama.cpp
- Distributed inference on 2025 roadmap (may not be stable)

**License**: MIT / Apache 2.0 (dual-licensed)
**Roadmap fit**: **Stage 3-4** (for embedding-generation, small model inference in Rust backend; WASM browser inference)
**Budget impact**: 2-3 engineer-weeks
**Rationale**: Best choice for Rust-native ML tasks like generating embeddings, running small classification models, or Whisper transcription (Nora voice). For large LLM inference, Ollama/llama.cpp is more mature.

### 3.3 Ollama (MIT)

**What it is**: The de facto standard CLI for local LLM management. Wraps llama.cpp behind a REST API + CLI. OpenAI-compatible API at `localhost:11434`. Supports 100+ models (Llama, Mistral, Gemma, Phi, Qwen, DeepSeek).

**2026 features**: Anthropic API compatibility (v0.14.0), multimodal vision models, Windows ARM64 native build.

**Pros**:
- **Zero-config model management** (one command to download + run)
- OpenAI-compatible API = drop-in replacement for cloud APIs
- Automatic GPU detection and memory management
- Model library with community-maintained quantizations
- Cross-platform (macOS, Linux, Windows)
- $0/token cost
- Privacy: no data leaves the machine

**Cons**:
- Runtime dependency (Ollama must be installed separately)
- Resource-intensive (7B model needs ~4-8GB RAM, 70B needs ~40GB)
- Quality of local models is below frontier cloud models (Claude, GPT-4)
- No built-in fine-tuning (Modelfile customization only)
- Adds latency compared to cloud APIs on fast networks

**License**: MIT
**Roadmap fit**: **Stage 3** (Tauri desktop app ships with Ollama integration for offline agent capabilities)
**Budget impact**: 1-2 engineer-weeks (API integration, model selection UI)
**Rationale**: ORCHA's executor pattern already supports multiple backends. Adding an Ollama executor alongside Claude/Gemini executors is straightforward. Key use cases: offline task classification, summarization, and basic agent operations when cloud APIs are unavailable.

**Integration pattern for ORCHA:**
```
Cloud available → Use Claude/Gemini (frontier quality)
Cloud unavailable → Fall back to Ollama (local model)
User preference → "Always local" toggle per task type
```

### 3.4 WebLLM (Apache 2.0)

**What it is**: Browser-based LLM inference using WebGPU. Achieves 80% of native performance. Supports INT-4 quantization for models like Llama-3.1-8B running in-browser.

**2026 status**: WebGPU achieved full cross-browser support (January 2026 — Firefox 147, Safari/iOS 26).

**Pros**:
- No server needed (runs entirely in browser)
- WebGPU provides 3-10x faster inference than WASM-only
- PagedAttention and FlashAttention in WGSL shaders
- INT-4 quantization reduces memory by 75%
- Privacy by default (data never leaves device)

**Cons**:
- 80% of native performance (20% gap)
- Model download required per-session (or cached in OPFS)
- 35% of users still affected by WebGPU compatibility issues
- Limited to models that fit in GPU VRAM (typically 7-8B max)
- Battery drain on mobile devices

**License**: Apache 2.0
**Roadmap fit**: **Stage 4-5** (browser-only offline inference for lightweight tasks)
**Budget impact**: 2-3 engineer-weeks
**Rationale**: Interesting for sovereignty mode (no server dependency at all) but impractical for most users before Stage 4. The Tauri + Ollama path provides better local inference for Stage 3.

### 3.5 GGUF Model Format and Quantization

**What it is**: The standard model format for llama.cpp. GGUF (GGML Universal Format) is a compact, portable binary format that includes model weights, tokenizer, and metadata in a single file.

**Quantization levels and trade-offs:**

| Quant | Bits | Size (7B model) | Quality Loss | Use Case |
|-------|------|-----------------|-------------|----------|
| F16 | 16 | ~14GB | None | Server inference |
| Q8_0 | 8 | ~7GB | Negligible | Desktop with 16GB+ RAM |
| Q5_K_M | 5 | ~5GB | Very small | Recommended balance |
| Q4_K_M | 4 | ~4GB | Small | Desktop with 8GB RAM |
| Q3_K_M | 3 | ~3GB | Moderate | Resource-constrained |
| Q2_K | 2 | ~2.5GB | Significant | Emergency fallback |
| IQ1_M | 1.5 | ~2GB | Heavy | Experimental |

**Recommendation for ORCHA**: Ship Q4_K_M quantizations by default. Offer Q5_K_M for users with 16GB+ RAM. This covers the 8B model class (Llama 3.1 8B, Mistral 7B, Phi-3 Medium) at ~4-5GB memory footprint.

### 3.6 Hardware Requirements Summary

| Tier | RAM | GPU | Models | Use Case |
|------|-----|-----|--------|----------|
| Minimum | 8GB | Integrated | 3B (Q4) | Task classification, simple Q&A |
| Recommended | 16GB | 6GB VRAM | 8B (Q4) | Summarization, basic agent ops |
| Advanced | 32GB | 12GB VRAM | 13B (Q5) | Code generation, complex reasoning |
| Power | 64GB+ | 24GB+ VRAM | 70B (Q4) | Near-cloud quality |

---

## 4. Edge Computing

### 4.1 WASM Compilation of Rust Backend Components

**What it is**: Compile selected Rust crates to WebAssembly for execution in browsers, edge runtimes (Cloudflare Workers), or embedded contexts.

**ORCHA components suitable for WASM:**

| Component | WASM Feasibility | Value | Priority |
|-----------|-----------------|-------|----------|
| Workflow validation/execution | High | Run workflow logic client-side | Stage 3 |
| Data transformation nodes | High | Process data at edge | Stage 4 |
| Auth token validation | High | Edge auth without round-trip | Stage 4 |
| SQLite query layer (`crates/db`) | Medium | Client-side data access | Stage 3 |
| Full Axum server | Low | Too many system dependencies | Not recommended |

**Pros**:
- Share business logic between server and client (single source of truth)
- Near-native performance (within 10-20% of native Rust)
- Sandboxed execution (WASM memory safety)
- Portable across all edge platforms

**Cons**:
- Not all Rust crates compile to WASM (anything using `std::fs`, `std::net`, `tokio` needs adaptation)
- WASM binary size can be large (need `wasm-opt` and tree shaking)
- Debugging WASM is harder than native
- SQLx won't compile to WASM (need alternative like `rusqlite` with WASM target)

**Roadmap fit**: Stage 3-4
**Budget impact**: 3-4 engineer-weeks (refactor crates for WASM compatibility)

### 4.2 Cloudflare Workers with Rust/WASM

**What it is**: Serverless edge runtime on Cloudflare's 300+ global locations. Compile Rust to WASM, deploy as Workers. Sub-millisecond cold starts. Workers AI provides on-edge LLM inference (Llama, Stable Diffusion).

**Performance**: Teams report 120ms → 15ms latency improvements by moving Rust crypto/auth to Workers.

**Pros**:
- 300+ edge locations globally (sub-50ms to most users)
- `workers-rs` crate for Rust development
- Workers AI for edge LLM inference (free tier available)
- D1 (SQLite at edge) + Durable Objects for state
- Generous free tier (100K requests/day)
- Async Rust via `spawn_local` / JavaScript event loop bridge

**Cons**:
- 128MB memory limit per Worker (hard constraint)
- 30-second CPU time limit (50ms for free tier)
- Vendor lock-in to Cloudflare ecosystem
- `wasm-bindgen` interop adds complexity
- D1 (edge SQLite) is still evolving

**License**: Proprietary (Cloudflare service), `workers-rs` MIT
**Roadmap fit**: **Stage 4** (edge API layer, auth, caching)
**Budget impact**: 1-2 engineer-weeks + $5-25/mo (Workers Paid plan)
**Rationale**: Useful for Stage 4 when ORCHA has 100+ orgs and needs global edge performance. Not worth the complexity before that scale.

### 4.3 Turso / libSQL (MIT)

**What it is**: MIT fork of SQLite that adds native replication, embedded replicas, encryption at rest, and distributed write capabilities. 26+ global edge replicas. Embedded replicas sync locally for microsecond reads.

**Key features for ORCHA:**

1. **Embedded replicas**: SQLite database replicated inside your application process. Reads are local (microsecond latency), writes go to primary and sync back.
2. **Encryption at rest**: Built-in via SQLCipher-compatible encryption.
3. **BEGIN CONCURRENT**: MVCC for improved write throughput (in development).
4. **Full SQLite compatibility**: Existing queries work unmodified.

**Pros**:
- **Natural evolution of ORCHA's existing SQLite** (same SQL, same patterns)
- Embedded replicas = local-first by default
- Self-hostable or managed cloud
- Encryption at rest (sovereignty requirement)
- 26+ global edge locations
- MIT license (no licensing risk)
- Rust client SDK (`libsql` crate)

**Cons**:
- Not PostgreSQL (if ORCHA commits to Postgres for cloud, Turso is a parallel path)
- Write throughput still limited vs. PostgreSQL
- Smaller ecosystem than PostgreSQL
- Advanced features (e.g., JSON functions) may lag SQLite upstream
- Pricing for managed service can scale ($29/mo starter, enterprise custom)

**License**: MIT (libSQL fork), proprietary (Turso Cloud features)
**Roadmap fit**: **Stage 2** (evaluate as alternative to full PostgreSQL migration)
**Budget impact**: 1-2 engineer-weeks migration + $0-29/mo
**Rationale**: Turso/libSQL is the strongest bridge between ORCHA's current SQLite and a distributed future. If ORCHA adopts Turso, the existing SQLite codebase works with minimal changes, embedded replicas provide local-first, and the managed service provides multi-tenant cloud. The trade-off: you don't get PostgreSQL's ecosystem (RLS, advanced extensions, pgvector). Decision point at Stage 2: Turso (evolutionary) vs. PostgreSQL (revolutionary).

### 4.4 Edge AI Inference

**Current landscape (2026):**

| Provider | Runtime | Models | Pricing |
|----------|---------|--------|---------|
| Cloudflare Workers AI | WASM + GPU | Llama, Mistral, Stable Diffusion | Free tier + pay-per-use |
| Fly.io GPU Machines | Docker + GPU | Any (bring your own) | $0.5-3/hr GPU |
| Vercel AI SDK | Edge Functions | Proxy to providers | Edge runtime free |
| AWS Lambda@Edge | Container | Small models only | Pay-per-invocation |

**Recommendation for ORCHA**: Edge AI inference is most valuable for:
- Task classification/routing (small model, fast)
- Embedding generation for search (< 1B param model)
- Intent detection for Nora voice (latency-sensitive)

Use cloud APIs (Claude, Gemini) for complex reasoning tasks. Edge inference for latency-sensitive, simple tasks.

**Roadmap fit**: Stage 4
**Budget impact**: 1-2 engineer-weeks + variable compute costs

---

## 5. Sync Architecture

### 5.1 Bidirectional Sync: Local SQLite ↔ Cloud PostgreSQL

**Architecture options:**

#### Option A: PowerSync (Recommended for Stage 2)
```
[Client SQLite] ←→ [PowerSync Service] ←→ [Cloud PostgreSQL]
```
- PowerSync handles sync rules, conflict resolution, partial replication
- Client writes to local SQLite, PowerSync uploads to Postgres
- Postgres changes streamed back to client via sync rules
- Selective sync via PowerSync "buckets" (e.g., per-tenant, per-project)

#### Option B: ElectricSQL + PGlite
```
[Client PGlite/SQLite] ← [Electric Sync] ← [Cloud PostgreSQL]
```
- Electric provides read-path sync (Postgres WAL → client)
- Write path goes directly to Postgres (not bidirectional CRDT)
- Best if write-offline is not a hard requirement

#### Option C: Custom Sync (Stage 3+)
```
[Client SQLite] ←→ [ORCHA Sync Service (Rust)] ←→ [Cloud PostgreSQL]
```
- Build custom sync using ORCHA's existing Axum infrastructure
- Operation log table in both databases
- Client sends operation log entries, server applies + broadcasts
- More control, more maintenance burden

#### Option D: Turso Embedded Replicas
```
[Client Embedded Replica] ←→ [Turso Primary] (no PostgreSQL)
```
- Skip PostgreSQL entirely
- Turso primary is the cloud database
- Embedded replicas in client apps
- Simplest architecture, but no PostgreSQL ecosystem

**Recommendation**: Option A (PowerSync) for Stage 2, with Option C as a Stage 3+ evolution if PowerSync becomes a bottleneck.

### 5.2 Optimistic Updates with Server Reconciliation

**Pattern already partially implemented in ORCHA** (React Query's `onMutate` → optimistic update → rollback on error). To extend for offline:

1. **Client generates UUID** for every mutation (ORCHA already uses `DbUuid::new()`)
2. **Mutation stored in local queue** with UUID + timestamp + payload
3. **UI updates immediately** from local state
4. **Background sync** replays mutations to server
5. **Server response** confirms or rejects each mutation by UUID
6. **Reconciliation**: On rejection, revert local state and notify user

**Key invariant**: Every mutation is idempotent. Replay of the same UUID is a no-op on the server.

**Roadmap fit**: Stage 1-2 (extend existing React Query patterns)
**Budget impact**: 2 engineer-weeks

### 5.3 Selective Sync

**What to sync per client:**

| Data | Sync Strategy | Rationale |
|------|--------------|-----------|
| User's org(s) + settings | Always sync | Small, critical |
| User's assigned tasks | Always sync | Core workflow |
| User's CRM pipeline stages | Always sync | Core workflow |
| Project files/artifacts | On-demand | Large, selective |
| Workflow definitions | Always sync (org-level) | Needed for offline execution |
| Other orgs' data | Never | Tenant isolation |
| Audit logs | Never (server-only) | Large, not needed offline |
| Agent execution history | On-demand (recent only) | Large, mostly archival |

**Implementation**: PowerSync "sync rules" or custom bucket definitions that filter by `organization_id` + `user_id`.

### 5.4 Bandwidth-Efficient Delta Sync

**Strategies:**

1. **Row-level change tracking**: Use `updated_at` timestamps (ORCHA already has these). Sync only rows changed since last sync.
2. **Column-level deltas**: For large text fields (task descriptions, workflow definitions), compute and transmit diffs (RFC 6902 JSON Patch).
3. **Compression**: gzip/brotli on sync payloads (standard HTTP compression).
4. **Batch sync**: Bundle multiple changes into single sync messages (reduce HTTP overhead).
5. **Merkle tree verification**: Hash-based sync verification to detect drift without transmitting full data.

**Estimated bandwidth savings**: Row-level tracking + compression = 80-90% reduction vs. full-table sync.

**Roadmap fit**: Stage 2-3
**Budget impact**: 1-2 engineer-weeks

---

## 6. Desktop/Mobile Packaging

### 6.1 Tauri 2.0 (MIT/Apache 2.0)

**What it is**: Rust-based framework for desktop and mobile apps using web frontends. Uses OS native webview (not bundled Chromium). Tauri 2.0 supports Windows, macOS, Linux, Android, iOS from a single codebase.

**Performance vs. Electron:**

| Metric | Tauri | Electron |
|--------|-------|----------|
| Bundle size | 3-5 MB | 100-150 MB |
| Memory usage | 50% less | Baseline |
| Startup time | < 500ms | 1-2 seconds |
| Min disk footprint | ~600KB | ~150MB |

**ORCHA-specific advantages:**
- Rust backend = shared code between server and desktop app
- Sidecar pattern: bundle Ollama or llama.cpp as a sidecar process
- SQLite embedded naturally (already ORCHA's DB)
- System tray for Nora voice activation
- File system integration (drag-and-drop to ORCHA)
- Auto-update via Tauri's built-in updater
- Deep OS integration (notifications, file watchers, keychain)

**Cons**:
- WebView rendering differences across platforms (Safari on macOS, WebView2 on Windows, WebKitGTK on Linux)
- Mobile support still maturing (Android/iOS in beta)
- Plugin ecosystem smaller than Electron's
- Custom native plugins require Rust knowledge
- WebView limitations (no DevTools in production, WebView bugs are platform bugs)

**License**: MIT / Apache 2.0
**Roadmap fit**: **Stage 3** (Priority 32 in roadmap)
**Budget impact**: 3-4 engineer-weeks (initial desktop app) + 1 week/month maintenance
**Rationale**: Perfect fit for ORCHA. Rust backend shared code, embedded SQLite for offline, system tray for Nora. The existing React frontend works inside Tauri with minimal changes. Bundle Ollama as a sidecar for local LLM inference.

### 6.2 Capacitor (MIT)

**What it is**: Ionic's mobile wrapper for web apps. Packages React apps as native iOS/Android apps with access to native APIs (camera, push notifications, biometrics). Strong PWA compatibility — same codebase for web, iOS, Android.

**Pros**:
- Existing React/Vite frontend works with zero changes
- Mature plugin ecosystem (100+ official + community plugins)
- Strong PWA baseline (app works as PWA + native wrapper)
- Easier learning curve than Tauri (no Rust required)
- Custom plugins in Swift/Kotlin (not Rust)
- Live reload during development

**Cons**:
- Mobile-focused (no desktop support — need Tauri separately)
- WebView performance overhead vs. native
- Not as performant as Tauri for resource-intensive tasks
- iOS WebView restrictions (no background JS execution)
- Larger footprint than pure PWA

**License**: MIT
**Roadmap fit**: **Stage 3-4** (mobile companion app)
**Budget impact**: 2-3 engineer-weeks
**Rationale**: Use Capacitor for mobile (iOS/Android) and Tauri for desktop. The React frontend is shared across all three targets (web, desktop, mobile) with platform-specific code only for native integrations.

### 6.3 Packaging Strategy Recommendation

| Platform | Tool | Stage | Notes |
|----------|------|-------|-------|
| Web (PWA) | Vite + Workbox | Stage 1 | Installable, offline-capable |
| Desktop | Tauri 2.0 | Stage 3 | Embedded SQLite + Ollama sidecar |
| Mobile (MVP) | PWA (no wrapper) | Stage 1-2 | Mobile-responsive web app |
| Mobile (Native) | Capacitor | Stage 3-4 | Push notifications, biometrics |

### 6.4 Auto-Update and Version Management

**Tauri auto-update**: Built-in updater plugin checks a hosted JSON endpoint for new versions. Downloads differential updates (not full re-download). Supports multiple update channels (stable, beta, nightly).

**Capacitor**: App store distribution (Apple App Store, Google Play) handles updates. For enterprise distribution, use MDM or direct APK/IPA sideloading.

**Version strategy**: Semantic versioning. Desktop app version pinned to minimum compatible API version. Server maintains backward compatibility for N-1 desktop version.

---

## 7. Sovereignty Mode (Roadmap Stage 5)

### 7.1 On-Premises Deployment Patterns

**Recommended architecture:**

```
[Customer Network]
  ├── Docker Compose / K8s
  │   ├── ORCHA Server (Rust binary)
  │   ├── PostgreSQL / Turso
  │   ├── NATS (event bus)
  │   ├── Ollama (local LLM)
  │   └── MinIO (S3-compatible storage)
  └── Reverse Proxy (nginx/Caddy)
```

**Deployment options by customer sophistication:**

| Option | Complexity | Target Customer |
|--------|-----------|----------------|
| Docker Compose (one-command) | Low | Small teams, developers |
| Helm Chart (Kubernetes) | Medium | Mid-market with K8s |
| Terraform + Ansible | High | Enterprise with IaC |
| Managed appliance (VM image) | Low | Non-technical orgs |

**Pros**:
- Customer owns all data (strongest sovereignty claim)
- No internet dependency (air-gap capable)
- Compliance with data residency laws (GDPR, CCPA, industry-specific)

**Cons**:
- Support burden (customer infra issues become your issues)
- Update distribution is complex (no forced updates)
- Monitoring/observability requires customer cooperation
- Per-customer divergence risk (customizations, version drift)

**Roadmap fit**: **Stage 5** (Priority 42)
**Budget impact**: 4-6 engineer-weeks for initial packaging + 1 engineer ongoing

### 7.2 Air-Gapped Installation

**What it is**: Deployment in networks with zero internet connectivity (government, defense, regulated industries).

**Requirements:**
1. **Offline installer**: All dependencies bundled (Docker images, model weights, frontend assets)
2. **No license phone-home**: License validation must work offline (signed JWT or similar)
3. **Offline model serving**: Ollama with pre-downloaded GGUF models
4. **No CDN dependencies**: All assets self-hosted (no Google Fonts, no CDN links)
5. **Offline documentation**: Bundled docs site

**Implementation:**
- **Installer**: Tarball or USB-bootable image containing all Docker images (`docker save/load`)
- **License**: Ed25519 signed license file with expiry, org ID, and feature flags. Validated locally.
- **Updates**: "Sneakernet" — customer downloads update bundle, transfers via approved media

**Pros**:
- Addresses highest-security market segment (premium pricing: 3-5x cloud)
- Strong regulatory compliance story
- True data sovereignty

**Cons**:
- Extremely high support cost per customer
- Cannot remotely diagnose issues
- Update cycle measured in months, not days
- Model quality limited to what was bundled at install time

**Roadmap fit**: **Stage 5**
**Budget impact**: 3-4 engineer-weeks (air-gap packaging)
**Rationale**: Only pursue when enterprise demand validates the support cost. Price at 3-5x cloud to cover the support burden.

### 7.3 Customer-Managed Encryption Keys (CMEK)

**Architecture:**

```
Customer provides KMS endpoint or key file
  → ORCHA encrypts data-at-rest with customer key
  → Customer can revoke key to "crypto-shred" their data
  → ORCHA cannot access data without customer key
```

**Implementation approaches:**

| Approach | Complexity | Sovereignty Level |
|----------|-----------|------------------|
| Database-level encryption (Turso/SQLCipher) | Low | Medium (ORCHA manages DB) |
| Application-level encryption (envelope encryption) | Medium | High (per-field encryption) |
| Customer KMS integration (AWS KMS, HashiCorp Vault) | High | Highest (customer manages keys) |
| Hardware Security Module (HSM) | Very High | Maximum (tamper-proof) |

**Recommendation**: Start with database-level encryption (Turso's built-in encryption or SQLCipher) in Stage 4. Add application-level envelope encryption + KMS integration in Stage 5.

**Roadmap fit**: Stage 4 (basic), Stage 5 (full CMEK)
**Budget impact**: 2-3 engineer-weeks (basic), 4-6 weeks (full KMS)

### 7.4 Data Residency Compliance

**Requirements by region:**

| Region | Regulation | Key Requirement |
|--------|-----------|----------------|
| EU | GDPR | Data stored in EU, right to erasure, DPA required |
| US (California) | CCPA/CPRA | Right to delete, opt-out of sale |
| US (Healthcare) | HIPAA | BAA required, encryption at rest + transit, audit logs |
| Canada | PIPEDA | Data stored in Canada (some provinces) |
| Australia | Privacy Act | APPs compliance, cross-border transfer rules |

**ORCHA implementation:**
1. **Multi-region deployment**: Deploy ORCHA instances per region (EU, US-East, US-West, APAC)
2. **Data routing**: Org settings include `data_region` field; API gateway routes to correct region
3. **Cross-region prohibition**: Org data never leaves its designated region
4. **Audit trail**: All data access logged with timestamp, user, action, data category

**Roadmap fit**: Stage 4-5
**Budget impact**: 4-6 engineer-weeks + multi-region infrastructure costs

### 7.5 Hybrid Cloud/On-Prem Architecture

**Pattern**: Control plane in cloud, data plane on-premises.

```
[ORCHA Cloud]                    [Customer Premises]
  ├── Auth/Identity              ├── ORCHA Data Plane
  ├── Billing/Metering           │   ├── Database (all customer data)
  ├── Update Distribution        │   ├── Agent Execution
  ├── Aggregated Analytics       │   └── File Storage
  └── Marketplace                └── Sync Agent (outbound only)
```

**Key principle**: Customer data never touches cloud servers. Only aggregated, anonymized telemetry flows up. Control plane handles auth, billing, and update distribution.

**Pros**:
- Satisfies most compliance requirements
- Easier to support than full air-gap
- Customer gets updates automatically
- Platform can still meter usage for billing

**Cons**:
- Complex networking (outbound-only sync agent)
- Split-brain risk if cloud control plane is unreachable
- More infrastructure than pure cloud

**Roadmap fit**: **Stage 5** (but plan the data plane/control plane split in Stage 3-4 architecture)
**Budget impact**: 6-8 engineer-weeks

---

## 8. Recommended Implementation Order

### Stage 0-1 (Q2-Q4 2026): Foundation — ~4-6 engineer-weeks

| Item | Effort | Impact |
|------|--------|--------|
| PWA with Workbox (app shell caching) | 1 week | Instant loading, installable |
| Offline queue with Background Sync | 1 week | Create/update tasks offline |
| Optimistic updates (extend React Query) | 1 week | Perceived zero-latency UI |
| IndexedDB for cached API responses | 0.5 weeks | Offline data browsing |
| Mobile-responsive audit | 0.5 weeks | PWA works on mobile |
| Offline indicator UI | 0.5 weeks | User knows when offline |

### Stage 2 (Q1-Q3 2027): Sync Layer — ~6-8 engineer-weeks

| Item | Effort | Impact |
|------|--------|--------|
| Evaluate PowerSync vs. ElectricSQL vs. Turso | 2 weeks | Choose sync strategy |
| Implement chosen sync layer | 3 weeks | Bidirectional local↔cloud sync |
| Selective sync (per-org data) | 1 week | Bandwidth efficiency |
| Conflict resolution UI (basic) | 1 week | User-facing merge conflicts |
| SQLite WASM + OPFS for browser | 1 week | Full offline data in browser |

### Stage 3 (Q4 2027-Q2 2028): Desktop + Local AI — ~10-14 engineer-weeks

| Item | Effort | Impact |
|------|--------|--------|
| Tauri 2.0 desktop app | 3-4 weeks | Desktop distribution |
| Ollama integration (executor) | 2 weeks | Offline agent capabilities |
| Embedded SQLite in Tauri | 1 week | Offline data (desktop) |
| System tray + Nora voice hook | 1 week | Always-on assistant |
| Auto-update mechanism | 1 week | Desktop lifecycle |
| Candle for embeddings/small models | 2 weeks | Rust-native ML |
| CRDT evaluation (Yjs or Automerge) | 2 weeks | Collaborative features |

### Stage 4 (Q3 2028-Q4 2029): Edge + Mobile — ~8-12 engineer-weeks

| Item | Effort | Impact |
|------|--------|--------|
| Capacitor mobile wrapper | 2-3 weeks | iOS/Android native app |
| Cloudflare Workers edge layer | 2 weeks | Global low-latency |
| Edge AI inference (small models) | 2 weeks | Latency-sensitive AI |
| WASM compilation of business logic | 2-3 weeks | Shared logic client/edge/server |
| Database encryption (basic CMEK) | 2 weeks | Enterprise readiness |

### Stage 5 (2030-2031): Full Sovereignty — ~16-20 engineer-weeks

| Item | Effort | Impact |
|------|--------|--------|
| Docker Compose on-prem packaging | 3 weeks | Self-hosted deployment |
| Air-gapped installer | 3 weeks | Highest-security market |
| Full CMEK with KMS integration | 4 weeks | Customer key management |
| Hybrid cloud/on-prem architecture | 4 weeks | Control plane/data plane split |
| Multi-region data residency | 3 weeks | Compliance |
| P2P sync via APN protocol | 3 weeks | Decentralized sync |

---

## 9. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| PowerSync/ElectricSQL pivot or shutdown | Medium | High | Abstract sync layer; evaluate 2+ options before committing |
| WebView inconsistencies in Tauri across platforms | Medium | Medium | Test on all platforms in CI; maintain browser fallback |
| Local LLM quality insufficient for agent tasks | High | Medium | Use as fallback only; cloud APIs remain primary |
| OPFS browser support gaps (Safari) | Low | Medium | IndexedDB fallback for unsupported browsers |
| Sync conflict resolution UX confuses users | Medium | Medium | Default to last-write-wins; advanced merge UI opt-in |
| Air-gapped support costs exceed revenue | Medium | High | Price at 3-5x cloud; minimum contract size |
| WASM binary size too large for edge deployment | Low | Low | Tree shaking, feature flags, split into micro-modules |
| Turso vs. PostgreSQL decision paralysis | Medium | High | Decide at Stage 2 entry; both are viable, pick one |

---

## License Summary

| Tool | License | Commercial Use | Modification | Distribution |
|------|---------|---------------|-------------|-------------|
| Automerge | MIT | Yes | Yes | Yes |
| Yjs | MIT | Yes | Yes | Yes |
| diamond-types | Apache 2.0 | Yes | Yes | Yes (with attribution) |
| PowerSync | Apache 2.0 (Open) | Yes | Yes | Yes |
| ElectricSQL | Apache 2.0 | Yes | Yes | Yes |
| PGlite | Apache 2.0 | Yes | Yes | Yes |
| Turso/libSQL | MIT | Yes | Yes | Yes |
| Workbox | MIT | Yes | Yes | Yes |
| llama.cpp | MIT | Yes | Yes | Yes |
| Candle | MIT/Apache 2.0 | Yes | Yes | Yes |
| Ollama | MIT | Yes | Yes | Yes |
| WebLLM | Apache 2.0 | Yes | Yes | Yes |
| Tauri 2.0 | MIT/Apache 2.0 | Yes | Yes | Yes |
| Capacitor | MIT | Yes | Yes | Yes |
| Cloudflare Workers | Proprietary (service) | Via service agreement | N/A | N/A |
| workers-rs | MIT/Apache 2.0 | Yes | Yes | Yes |

**All recommended tools are MIT or Apache 2.0 licensed.** No GPL dependencies. No licensing costs. The only paid services are optional managed offerings (Turso Cloud, PowerSync Cloud, Cloudflare Workers) which have self-hosted alternatives.

---

## Key Decision Points

### Decision 1: Turso/libSQL vs. PostgreSQL (Stage 2 entry)

| Factor | Turso/libSQL | PostgreSQL |
|--------|-------------|------------|
| Migration effort from current SQLite | **Low** (same SQL dialect) | **High** (202 migrations to port) |
| Local-first story | **Native** (embedded replicas) | Requires PowerSync/ElectricSQL |
| Extension ecosystem | Limited | **Rich** (pgvector, PostGIS, RLS) |
| Multi-tenant patterns | Database-per-tenant | **Shared schema + RLS** |
| Cross-tenant analytics | Harder (separate DBs) | **Easy** (single DB) |
| Enterprise readiness | Growing | **Mature** |
| Edge deployment | **Native** (26+ locations) | Requires Turso or custom replication |

**Recommendation**: Adopt PostgreSQL for cloud multi-tenant (aligns with roadmap Priority 21) but use Turso embedded replicas for client-side offline storage. This gives PostgreSQL's power on the server and SQLite-compatible local storage on the client.

### Decision 2: Sync Engine (Stage 2)

Evaluate PowerSync and ElectricSQL with a 2-week spike. Selection criteria:
1. Postgres WAL compatibility
2. Selective sync granularity
3. React SDK maturity
4. Self-hosting option quality
5. Write-path offline support

### Decision 3: Desktop Framework (Stage 3)

**Tauri 2.0** is the clear choice. Rust backend code sharing, smallest bundle size, embedded SQLite, sidecar support for Ollama. The only risk is WebView inconsistencies, mitigated by maintaining the web app as primary and desktop as enhanced wrapper.

---

*Research compiled 2026-03-19. All tool versions and capabilities reflect status as of this date.*
