# Lux Creator Studio — Illustrator-Compatible Deck Authoring

**Date**: 2026-04-12
**Branch**: `feature/lux-creator-studio` (new branch for feature development)
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp` (root) on new branch `feature/lux-creator-studio`
**Base**: `main` (at d94eb4ea)
**Duration**: ~12–14 weeks, 10 phases delivered as vertical slices
**Depends on**: Current Lux registry entry, `OrgBrandProfile`, `MediaAsset`, existing `ComfyUIClient`
**Unblocks**: Operator-driven deck editing inside the dashboard; client-presentable PDFs; Illustrator round-trip; multi-format export (SVG, PDF/X-4, PPTX, PNG); client-facing review portal

---

## Resolved decisions (from planning Q&A, 2026-04-12)

| # | Question | Decision |
|---|----------|----------|
| 1 | Worktree / branching | New feature branch `feature/lux-creator-studio` off `main` |
| 2 | Default canvas | **1920×1080 @ 72dpi** (16:9, Keynote/PPT parity, 2x export = 4K) |
| 3 | ComfyUI endpoint | **No dev endpoint exists yet** — Phase 7 stands one up, but it's owned by **Maci** (Master Cinematographer), not Lux. Lux delegates asset generation to Maci. |
| 4 | PPTX export | **P2 behind flag** in Phase 8; SVG + PDF/X-4 + PNG are the P1 export targets |
| 5 | Default font whitelist | **Modern editorial set**: Inter, DM Sans, Fraunces, Space Grotesk, JetBrains Mono (all SIL OFL, embeddable) |
| 6 | Client share portal | **Add as Phase 9** — read-only client share links with per-slide comment threads, +~2 weeks |

---

## Goal

Turn Lux from a markdown-only deck outliner into a **full presentation design agent** whose output is a structured slide document that operators can edit inside an in-dashboard **Creator Studio**, export to **Illustrator-compatible SVG/PDF**, and iterate on collaboratively with Lux suggesting changes the operator accepts or rejects.

A successful rollout means:

1. Lux generates real slide layouts (positioned text, images, shapes, brand colors, typography) — not markdown outlines.
2. Operators open the Polish stage deck in a WYSIWYG canvas in the dashboard, edit any element, reorder slides, and save.
3. Operators export to SVG and open in Illustrator with layers, editable text, and brand colors intact; saves back as SVG and re-imports without fidelity loss on common element types.
4. Lux can propose follow-up edits as **suggestions** that the operator accepts/rejects — it never stomps manual edits.
5. The studio draws from the org's brand profile (colors, typography, voice, logo) as W3C design tokens.
6. Output passes a preflight check (WCAG contrast, grid alignment, bleed/trim for print) before stage advance.

---

## Non-Goals (explicit)

- **Native `.ai` file writing** — technically impossible outside Illustrator itself (private `AIPrivateData` PDF stream). We ship SVG + print-quality PDF; Illustrator opens both.
- **Real-time multi-user collaboration** — single-editor-at-a-time with last-write-wins is fine for v1. CRDT/operational-transform is deferred.
- **Figma parity** — we are not rebuilding Figma. Focus on the 20% of editor features that cover 80% of deck authoring: text, image, shape, group, align, reorder, brand-token apply.
- **Animation/transitions** — static slides only. Motion deferred to a later phase behind its own flag.
- **Video embedding** — out of scope for v1.
- **New AI agents** — this plan extends Lux, not a new `Editron` or co-designer agent.

---

## Architectural Decisions

### A1. Canonical format: typed `DeckDocument` JSON

The source of truth is a structured JSON document, not markdown and not SVG. SVG and PDF become render/export targets only.

```ts
// shared/types.ts (generated from Rust via ts-rs)
type DeckDocument = {
  id: DbUuid;
  deal_id: DbUuid;
  version: number;                     // monotonic revision counter
  brand_token_version: string;         // snapshot of design tokens used
  canvas: { width: number; height: number; unit: "px" | "pt"; dpi: number };
  slides: Slide[];
  created_at: string;
  updated_at: string;
  last_edited_by: "lux" | "operator" | "system";
};

type Slide = {
  id: DbUuid;
  index: number;                       // position in deck
  name: string;                        // "Cover", "Agenda", etc.
  layout_hint?: "title" | "content" | "split" | "cover" | "closer";
  background: Fill;
  elements: SlideElement[];
  notes?: string;                      // speaker notes
  origin: "lux" | "operator";
  locked: boolean;                     // whole-slide lock
};

type SlideElement =
  | TextElement
  | ImageElement
  | ShapeElement
  | GroupElement;

type ElementBase = {
  id: DbUuid;
  bbox: { x: number; y: number; w: number; h: number; rotation?: number };
  z: number;                           // z-index within slide
  origin: "lux" | "operator";          // who authored this element
  locked_by: "lux" | "operator" | null;
  token_refs?: Record<string, string>; // e.g., { fill: "color.brand.primary" }
  opacity?: number;
};

type TextElement = ElementBase & {
  type: "text";
  text: string;
  font_family: string;
  font_size: number;
  font_weight: number;
  line_height: number;
  letter_spacing: number;
  color: Color;
  align: "left" | "center" | "right" | "justify";
  // Rich text runs for mixed styling within a single box
  runs?: Array<{ start: number; end: number; overrides: Partial<TextElement> }>;
};

type ImageElement = ElementBase & {
  type: "image";
  asset_id: DbUuid;                    // FK to media_assets
  fit: "cover" | "contain" | "fill";
  crop?: { x: number; y: number; w: number; h: number };
};

type ShapeElement = ElementBase & {
  type: "shape";
  shape: "rect" | "ellipse" | "line" | "path";
  path?: string;                       // SVG path data for "path" shape
  fill: Fill;
  stroke?: Stroke;
  corner_radius?: number;
};

type GroupElement = ElementBase & {
  type: "group";
  children: SlideElement[];
};

type Fill =
  | { kind: "solid"; color: Color }
  | { kind: "linear-gradient"; stops: Array<{ offset: number; color: Color }>; angle: number }
  | { kind: "none" };

type Color = { r: number; g: number; b: number; a: number; token?: string };
```

**Why JSON not SVG directly**: we need to reason about elements (lock state, origin, token refs, rich text runs), diff revisions, and run preflight checks over semantic structure. SVG is too lossy as a primary format — rich text runs, lock metadata, and token references disappear.

**Why not Markdown**: current Lux output is pure prose. The studio needs coordinates, not prose.

### A2. Illustrator interop via SVG 1.1 + PDF/X-4

- **Write**: Export `DeckDocument` → SVG 1.1 with text-as-text (not outlined), one `<g>` group per slide element, named groups carry `id` and `inkscape:label` / `data-origin` attributes. Illustrator 2024+ opens SVG 1.1 natively with layers, text, and groups preserved.
- **Read**: Illustrator save-as SVG → import back into `DeckDocument`. Preserve element IDs to maintain lock/origin state across round-trip.
- **Print path**: Headless Chromium `page.pdf()` with print CSS → PDF/X-4 (via `pdfcpu` or Ghostscript post-process). Illustrator opens PDF/X-4 and treats it essentially like an AI file for edits.
- **Fidelity budget**: Round-trip fidelity is expected on text, rect/ellipse, paths, solid fills, gradients, and groups. Effects (drop shadows, blurs) are best-effort — v1 flags unsupported features on import.

### A3. Renderer: Satori via Node sidecar

A small Node service (`crates/services/src/services/agent_tools/deck_renderer/`) exposes `POST /render` that accepts a `DeckDocument` and returns:

- `svg` (string)
- `png` (base64, for thumbnails and VLM review)
- `pdf` (base64, for export)

Implementation: **Satori** (JSX/CSS → SVG) + **resvg-js** (SVG → PNG) + **puppeteer-core** (HTML → PDF) running in a long-lived Node process. The Rust backend calls it over HTTP on `127.0.0.1`. Flox manifest adds `nodejs_20` and `pnpm`.

**Alternative considered**: Rust-native SVG renderer (`usvg` + `resvg` + custom layout engine). Rejected for v1 — ~4x the effort, no meaningful win. Can be swapped later behind the same HTTP contract if we outgrow Satori.

### A4. Editor: tldraw v3

The dashboard studio is built on **tldraw v3** (React, MIT licensed, schema-driven). Why:

- Ships selection, drag, resize, rotate, undo/redo, keyboard shortcuts, copy/paste, zoom/pan, alignment guides, and export to SVG/PNG out of the box.
- Extensible `CustomShape` API lets us map `SlideElement` types to tldraw shapes 1:1.
- `Editor.store` is a reactive schema-validated store that's easy to sync with our backend `DeckDocument`.
- Works offline-first; persistence is explicit (we control when to save).

**Alternatives considered**: Konva.js (lower level, we'd rebuild tldraw's features), Fabric.js (weaker React integration), Excalidraw (too whiteboard-focused), in-house SVG editor (months of work).

### A4b. Agent separation of concerns: Lux composes, Maci generates imagery

Lux is the **layout / composition / typography** agent. Maci is the **Master Cinematographer** and already owns image generation through `CinematicsService` + ComfyUI (see `crates/services/src/services/agent_registry.rs:136` and `crates/nora/src/conference_workflow/graphics.rs`). Lux does **not** call ComfyUI directly and does **not** hold image-gen tools.

When Lux is authoring a deck and a slide needs imagery (hero photo, illustration, background texture), it places a **pending asset placeholder** on the canvas and emits an asset request to Maci via a new delegation tool:

```
request_asset_from_maci(
  brief: {
    prompt: string,               // Lux-authored creative brief
    purpose: "hero" | "background" | "illustration" | "texture" | "icon",
    aspect_ratio: "16:9" | "1:1" | "4:5" | "9:16" | "3:4",
    style_preset?: "cinematic" | "editorial" | "commercial" | "abstract" | "minimal" | "bold",
    mood?: string,
    color_palette?: string[],     // resolved from brand tokens before the call
    reference_asset_ids?: DbUuid[],
  },
  slot: {
    deck_id: DbUuid,
    slide_id: DbUuid,
    element_id: DbUuid,           // the pending ImageElement holding the slot
  },
  priority: "low" | "medium" | "high",
) → AssetRequest { id, status: "pending" }
```

**Request lifecycle** (async, never blocks Lux):

1. Lux calls `request_asset_from_maci(...)`. The backend creates an `AssetRequest` row, places a pending `ImageElement` with `asset_status: "pending"` and a placeholder token (brand-tinted rect + prompt caption), and enqueues a Maci task via the existing `delegate_task` pattern.
2. Lux immediately continues composing other slides — asset fulfillment is non-blocking.
3. Maci picks up the task, runs `generate_cinematic_image` (its existing function), persists result as a `MediaAsset` row, and resolves the `AssetRequest` with the `asset_id`.
4. Backend publishes an SSE event on `/api/events/decks/:id` → `asset_request.resolved { request_id, asset_id, element_id }`.
5. The studio (and any in-flight Lux run) swaps the placeholder `ImageElement` → real `ImageElement { asset_id }`, preserving bbox, crop, fit, and z-order.

**Why this matters:**
- Preserves the agent specialization the codebase is already built around — Maci is the cinematography expert, Lux is the deck designer.
- Keeps Lux's prompt small and focused on composition; it doesn't need to know about ComfyUI workflows, seeds, CFG scales, or VAE settings.
- Operators can also request assets from Maci directly in the studio ("Ask Maci to regenerate this image" button on any `ImageElement`), using the same pipeline.
- Maci's existing clients (Nora conference workflows, brand agents) keep working unchanged.
- Failed Maci runs surface in the studio as a retry/override UI; Lux doesn't need to handle ComfyUI error paths itself.

**Synchronous fast path**: for small iconography (<256px) Lux may fall back to a `request_icon_from_library` tool that searches an icon set (e.g., Lucide, Heroicons) rather than round-tripping Maci. Decided per-phase; v1 ships Maci-only.

### A5. Lux ↔ operator collaboration: origin + locks + suggestions

Three mechanisms, layered:

1. **`origin` field on every element** — `"lux"` or `"operator"`. Set at creation time. Lux-authored elements can be regenerated by Lux; operator-authored elements never are.
2. **`locked_by` field** — when an operator touches a Lux element, the element transitions to `locked_by: "operator"` and is frozen against Lux rewrites. Explicit unlock via UI button returns it to `locked_by: null`.
3. **Suggestions** — Lux proposes changes as a `DeckSuggestion { deck_id, target_element_id?, target_slide_id?, op, payload, rationale }` record. The operator sees suggestions as inline diff badges and accepts/rejects them. Accepted suggestions apply the op and bump `deck.version`. Rejected suggestions are archived with the reason.

**State machine** for an element:
```
created_by_lux       → locked_by=null, origin=lux
edited_by_operator   → locked_by=operator, origin=lux (origin never changes)
manually_unlocked    → locked_by=null
created_by_operator  → locked_by=null, origin=operator (immune to Lux rewrites)
```

### A6. Brand → W3C Design Tokens

New route `GET /api/organizations/:id/design-tokens` returns W3C DTCG JSON:

```json
{
  "color": {
    "brand": {
      "primary":   { "$value": "#0A1F3B", "$type": "color" },
      "secondary": { "$value": "#FFB400", "$type": "color" },
      "accent":    { "$value": "#FF4A1C", "$type": "color" }
    }
  },
  "typography": {
    "heading": { "$value": { "fontFamily": "Söhne", "fontWeight": 700 }, "$type": "typography" },
    "body":    { "$value": { "fontFamily": "Inter", "fontWeight": 400 }, "$type": "typography" }
  },
  "spacing":  { "grid": { "$value": "8px", "$type": "dimension" } }
}
```

Transformer lives in `crates/services/src/services/brand_tokens.rs`, pure function `OrgBrandProfile → DesignTokens`. `DeckDocument` elements reference tokens via `token_refs`; renderer resolves at export time using the snapshot stored in `deck.brand_token_version`.

### A7. Phasing

| Phase | Scope | Weeks | Visible result |
|-------|-------|-------|----------------|
| 0 | Slide schema + DB migration + ts-rs export | 1 | None (foundation) |
| 1 | Lux emits structured slides via new tools | 1 | Deck JSON in DB, still markdown in UI |
| 2 | Read-only slide canvas viewer in `DeckTab` | 1 | Operator sees rendered slides (big visual win) |
| 3 | Interactive editor (tldraw integration) | 2 | Operator edits text, moves elements, saves |
| 4 | SVG export + Illustrator round-trip + import | 1 | AI interop working |
| 5 | Suggestion/accept-reject collaboration model | 1–2 | Lux proposes edits without stomping operator |
| 6 | Design tokens + brand integration | 1 | Brand colors/typography flow into decks |
| 7 | **Stand up ComfyUI for Maci** + Lux→Maci delegation contract + studio asset pipeline | 2–3 | Lux composes, Maci generates imagery via async delegation |
| 8 | Preflight (contrast, grid, bleed) + export targets (SVG, PDF/X-4, PNG; PPTX behind flag) | 1 | Ready-to-present gate |
| 9 | Client-facing review portal (tokenized share links, per-slide comments) | 2 | Clients review decks without logins |

Phases 0→4 are the critical path and deliver the headline feature. Phases 5→8 are independent and can parallelize.

---

## Phase 0 — Slide Schema + DB Migration

**Deliverables**
- New Rust types in `crates/db/src/models/deck_document.rs` matching A1 schema.
- Migration `crates/db/migrations/20260413000000_deck_documents.sql`:

```sql
CREATE TABLE deck_documents (
  id TEXT PRIMARY KEY,
  deal_id TEXT NOT NULL REFERENCES crm_deals(id) ON DELETE CASCADE,
  version INTEGER NOT NULL DEFAULT 1,
  brand_token_version TEXT,
  canvas_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  last_edited_by TEXT NOT NULL DEFAULT 'system'
);

CREATE TABLE deck_slides (
  id TEXT PRIMARY KEY,
  deck_id TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
  slide_index INTEGER NOT NULL,
  name TEXT,
  layout_hint TEXT,
  background_json TEXT NOT NULL,
  elements_json TEXT NOT NULL,   -- full element tree; normalize later if hot
  notes TEXT,
  origin TEXT NOT NULL DEFAULT 'lux',
  locked INTEGER NOT NULL DEFAULT 0,
  UNIQUE (deck_id, slide_index)
);

CREATE TABLE deck_revisions (
  id TEXT PRIMARY KEY,
  deck_id TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
  version INTEGER NOT NULL,
  author TEXT NOT NULL,          -- 'lux' | 'operator' | user_id
  snapshot_json TEXT NOT NULL,   -- full deck
  diff_summary TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE deck_suggestions (
  id TEXT PRIMARY KEY,
  deck_id TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
  target_slide_id TEXT,
  target_element_id TEXT,
  op TEXT NOT NULL,              -- 'replace'|'insert'|'delete'|'style'
  payload_json TEXT NOT NULL,
  rationale TEXT,
  status TEXT NOT NULL DEFAULT 'pending',  -- pending|accepted|rejected
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  resolved_at TEXT,
  resolved_by TEXT
);

CREATE INDEX idx_deck_documents_deal ON deck_documents(deal_id);
CREATE INDEX idx_deck_slides_deck ON deck_slides(deck_id, slide_index);
CREATE INDEX idx_deck_suggestions_deck_status ON deck_suggestions(deck_id, status);
```

- Rust model + `CreateDeckDocument` / `UpdateDeckDocument` helpers.
- ts-rs exports; run `npm run generate-types`.
- Unit tests on (de)serialization round-trip.

**Risks / gotchas**
- `elements_json` as opaque TEXT is fast to ship but hurts querying. Acceptable for v1; normalize in a future phase if we need element-level queries.
- Existing `crm_deals.deck_url` stays untouched for backward compat; a `deck_document_id` nullable FK is added in phase 1.

---

## Phase 1 — Lux emits structured slides

**Deliverables**
- New Lux tools in `crates/server/src/agent_flow_executor.rs::build_agent_tools`:
  - `create_deck_document(deal_id, canvas?) → deck_id`
  - `append_slide(deck_id, slide: Slide) → slide_id`
  - `add_element(deck_id, slide_id, element: SlideElement) → element_id`
  - `update_element(deck_id, element_id, patch) → ok`
  - `fetch_brand_tokens(org_id) → DesignTokens`
  - `request_asset_from_maci(brief, slot, priority?) → asset_request_id` — delegates image generation to Maci (see A4b). Returns immediately; resolution is async via SSE.
  - `list_org_assets(org_id, query?) → MediaAsset[]` — lets Lux pick existing assets from the org library before requesting new ones from Maci.

**Lux explicitly does NOT get**: `generate_image`, direct ComfyUI access, or any image model tools. All imagery flows through Maci.
- Rewrite of Lux system prompt in `crates/server/src/routes/crm_deal_automations.rs:1105`:
  - Instructs Lux to call `fetch_brand_tokens` first, then `create_deck_document`, then iteratively append slides with positioned elements using token references.
  - Keeps the existing 10-slide structure (Cover → Close) but with layout hints and concrete element placement.
- New route `POST /api/crm/deals/:id/decks` → runs Lux with the new tools, returns `deck_document_id`.
- Backward compat: old markdown `deck_url` endpoint stays, now additionally exposes `deck_document_id`.
- `crm_deals.deck_document_id TEXT` added via migration.

**Acceptance**
- Polish stage auto-trigger creates a `DeckDocument` with ≥10 slides, each with ≥3 elements, all positioned within canvas bounds.
- Every slide has `origin=lux`.
- Lux never hallucinates brand colors — all color references are `token_refs` into `fetch_brand_tokens` result.

**Risks**
- Structured tool use is harder for the model than writing markdown. Mitigation: JSON-mode tool schemas with strict validation; prompt includes 2-3 few-shot slide examples.
- Tool-use loop cost/latency could balloon. Mitigation: cap at 60 tool calls per generation; timebox to 90s; fall back to markdown-only on failure.

---

## Phase 2 — Read-only slide canvas viewer

**Deliverables**
- New endpoint `GET /api/decks/:id` returning full `DeckDocument`.
- New endpoint `GET /api/decks/:id/render.svg?slide=N` delegating to the Node renderer.
- `frontend/src/components/deck/DeckViewer.tsx` — read-only thumbnail grid + large slide preview, powered by rendered SVGs.
- `DeckTab.tsx` updated: if `deal.deck_document_id` exists, render `<DeckViewer>`; otherwise fall back to current markdown box.
- Node renderer sidecar stood up (A3), with a thin Rust client in `crates/services/src/services/deck_renderer_client.rs`.
- Flox manifest: `nodejs_20`, `pnpm`; dev script to boot the renderer alongside the backend.

**Acceptance**
- Operator opens a deal in Polish stage, sees rendered slide thumbnails matching brand colors, can click any thumbnail to see a full-size preview.
- Zero edit affordances — strictly read-only.

**Risks**
- Satori's CSS subset may not cover everything we want (drop shadows, complex gradients). Mitigation: phase 2 accepts the subset limitations; document them; escalate only if they block a P1 slide type.
- Node sidecar adds a process to manage. Mitigation: supervised by the same orchestrator that runs the Rust server; health check + auto-restart.

---

## Phase 3 — Interactive editor (tldraw)

**Deliverables**
- `frontend/src/components/deck/DeckEditor.tsx` — tldraw canvas with custom shapes:
  - `LuxTextShape` ↔ `TextElement`
  - `LuxImageShape` ↔ `ImageElement`
  - `LuxShapeShape` ↔ `ShapeElement`
- Sync layer: `useDeckEditorStore` hook that mirrors tldraw store ↔ `DeckDocument`. Debounced autosave via `PATCH /api/decks/:id` every 1.5s.
- Slide navigator (left rail): drag-to-reorder, add blank slide, delete, duplicate.
- Toolbar: text style (via brand typography tokens), fill/stroke (via brand color tokens), alignment, z-order, lock toggle.
- Undo/redo using tldraw's built-in history.
- Keyboard shortcuts: familiar Figma/Keynote defaults.
- Conflict handling: optimistic UI + `version` field on `DeckDocument`; if server version has advanced, show "deck updated elsewhere — reload?" banner rather than merge.

**Acceptance**
- Operator can open a Lux-generated deck, edit any text element, move any element, save, reload, and see the change persisted.
- Edits to Lux elements automatically set `locked_by: "operator"`.
- New elements the operator draws have `origin: "operator"`.
- Playwright smoke test: open Polish stage deck → edit title → save → reload → assert new title.

**Risks**
- tldraw custom shapes API has a learning curve. Mitigation: build one shape type end-to-end first (text) before the others.
- Rich text runs inside a single text box are not tldraw-native. Mitigation: v1 ships single-style text boxes only; runs are a phase-3.5 upgrade.

---

## Phase 4 — SVG export + Illustrator round-trip

**Deliverables**
- `POST /api/decks/:id/export?format=svg|pdf|png&slide=N|all` endpoint.
- SVG export preserves:
  - One `<g id="slide-{i}">` per slide, labeled for Illustrator.
  - Text as `<text>` (not outlined paths).
  - Element IDs in `data-element-id` for round-trip identity.
  - `data-origin` and `data-locked-by` for metadata (AI strips on save but we attempt import recovery).
- `POST /api/decks/:id/import` endpoint accepting SVG upload. Parser maps SVG groups back to `SlideElement`s; unknown elements become `ShapeElement { shape: "path" }` with a warning badge.
- Studio UI: "Open in Illustrator" (downloads SVG) and "Import edited SVG" (uploads) buttons.
- Print path: `format=pdf` routes through Node sidecar Puppeteer → PDF/X-4 post-processed via `pdfcpu` (added to flox).

**Acceptance**
- Export a deck to SVG, open in Illustrator, verify layers panel shows one layer per slide with nested named groups, text is editable, brand colors are preserved.
- Edit text in Illustrator, save as SVG, import back into studio — edits visible, non-edited elements preserve origin/lock metadata.
- PDF export opens in Illustrator and Preview with embedded fonts and correct color.

**Risks**
- Illustrator's SVG import subtly rewrites attributes. Mitigation: import parser is permissive and falls back to bbox reconstruction if element IDs are missing.
- Font embedding across platforms is fragile. Mitigation: whitelist a curated set of web-safe + brand fonts with `@font-face` + font files bundled in the renderer.
- PDF/X-4 compliance is strict; `pdfcpu validate` may reject. Mitigation: ship a PDF/A-2 fallback if PDF/X-4 gate fails, labeled in the UI.

---

## Phase 5 — Suggestion/accept-reject collaboration

**Deliverables**
- `POST /api/decks/:id/suggestions` — Lux proposes one or more ops.
- `POST /api/decks/:id/suggestions/:sid/accept` / `/reject` — operator resolves.
- New Lux trigger path: "Ask Lux to improve slide N" button in studio → agent runs against the current `DeckDocument` state → emits suggestions against unlocked elements only.
- Frontend: suggestion badges next to affected elements; side panel listing all pending suggestions with diff preview and rationale; bulk accept/reject.
- Suggestion ops supported: `replace_text`, `restyle`, `reposition`, `insert_element`, `delete_element`, `reorder_slide`.

**Acceptance**
- Operator edits a slide title → asks Lux to improve remaining slides → Lux returns suggestions that do NOT touch the edited title.
- Rejected suggestions don't re-appear on next run unless the operator explicitly asks.

**Risks**
- Suggestion drift: Lux may propose conflicting suggestions across runs. Mitigation: run-scoped suggestion IDs; a new run invalidates pending suggestions from the previous run unless the operator pinned them.

---

## Phase 6 — Design tokens + brand integration

**Deliverables**
- `crates/services/src/services/brand_tokens.rs` — pure `OrgBrandProfile → DesignTokens` transformer.
- `GET /api/organizations/:id/design-tokens` route.
- Renderer resolves `token_refs` at render time; falls back to inline values if token missing (logs warning).
- Studio toolbar color/font pickers are token-first: operator sees "Brand Primary / Secondary / Accent" swatches before the full color wheel.
- `deck.brand_token_version` snapshot on every save so historical versions render with the tokens they were authored against.

**Acceptance**
- Changing the org's brand primary color and reopening the deck visibly updates all token-referenced fills (if `brand_token_version` is current).
- Historical deck revisions render with their original tokens, not the current ones.

**Risks**
- Typography tokens require font availability at render time. Mitigation: curated font whitelist per org; upload pipeline for custom fonts is a Phase 6.5.

---

## Phase 7 — Stand up ComfyUI for Maci + Lux→Maci delegation

**Note**: No ComfyUI endpoint exists today (neither dev nor prod), even though Maci is already coded to use one (`crates/nora/src/conference_workflow/graphics.rs:140+`). Phase 7 stands up the dev endpoint to unblock **Maci's existing capability**, then wires Lux to delegate to Maci rather than generating images itself.

This phase is jointly owned by Maci (infra + image gen) and Lux (delegation contract + studio UX). It grows from 1–2 weeks to 2–3 weeks to accommodate the infra work.

### 7a. ComfyUI dev endpoint for Maci (prerequisite)

**Deliverables**
- Docker Compose service `comfyui` added to `docker-compose.local.yml`:
  - Base image: `yanwk/comfyui-boot:latest` or a pinned GPU/CPU variant based on dev machine capability.
  - Volume mounts: `./comfyui/models`, `./comfyui/output`, `./comfyui/workflows`.
  - Exposed on `127.0.0.1:8188`.
  - Health check: `GET /system_stats`.
- Model bootstrap script `scripts/bootstrap-comfyui.sh`:
  - Downloads `flux1-dev.safetensors` (or `flux1-schnell` for lower VRAM), VAE, CLIP text encoders.
  - Idempotent; skips download if files exist.
  - Documents disk footprint (~20GB) in `docs/DEVELOPMENT.md`.
- Flox manifest: add ComfyUI startup to dev activation hook (optional, behind env var `ENABLE_COMFYUI=1` to avoid forcing the download on every dev).
- Runbook entry: `docs/runbooks/comfyui-local.md` — how to start, stop, debug, check logs, swap models.
- Health-check endpoint in Rust: `GET /api/health/comfyui` → proxies to `/system_stats`, used by the studio UI to show a status indicator and gracefully hide image-gen affordances when the service is down.

**Acceptance**
- `docker compose up comfyui` starts the service; `curl http://127.0.0.1:8188/system_stats` returns stats.
- `scripts/bootstrap-comfyui.sh` is idempotent and completes without errors on a fresh dev machine.
- Rust backend's health proxy correctly reports up/down.

**Risks**
- ComfyUI models are large (~20GB). Mitigation: bootstrap script is optional and gated; developers who don't need image gen skip it.
- GPU availability varies across dev machines. Mitigation: `flux1-schnell` + CPU fallback is documented; slower but functional.
- Image-gen is deferred to **prod** indefinitely — no prod ComfyUI infrastructure in this plan. A follow-up sprint will decide between self-hosted prod ComfyUI vs. a hosted API (Replicate, fal.ai, Ideogram).

### 7b. Lux → Maci delegation + studio asset pipeline

**Deliverables**
- New `asset_requests` table:
  ```sql
  CREATE TABLE asset_requests (
    id TEXT PRIMARY KEY,
    deck_id TEXT NOT NULL REFERENCES deck_documents(id) ON DELETE CASCADE,
    slide_id TEXT NOT NULL,
    element_id TEXT NOT NULL,
    requested_by TEXT NOT NULL,        -- 'lux' | user_id
    assignee TEXT NOT NULL DEFAULT 'maci',
    brief_json TEXT NOT NULL,          -- serialized brief (prompt, style, etc.)
    status TEXT NOT NULL DEFAULT 'pending',  -- pending|in_progress|resolved|failed|cancelled
    priority TEXT NOT NULL DEFAULT 'medium',
    result_asset_id TEXT,              -- FK to media_assets on success
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
  );
  CREATE INDEX idx_asset_requests_deck_status ON asset_requests(deck_id, status);
  ```
- New Lux tool `request_asset_from_maci(brief, slot, priority?)` in `crates/server/src/agent_flow_executor.rs`:
  - Creates `asset_request` row with status=pending.
  - Places a pending `ImageElement { asset_status: "pending", asset_request_id }` on the slide.
  - Enqueues a Maci task via the existing `delegate_task` / `CinematicsService` pathway used by Nora in `crates/nora/src/conference_workflow/graphics.rs:140+`.
  - Returns `asset_request_id` immediately. Does NOT block.
- Maci-side handler reads the brief, calls `generate_cinematic_image`, persists result as `MediaAsset`, updates `asset_request.status = resolved` with `result_asset_id`.
- SSE stream: `/api/events/decks/:id` emits `asset_request.status_changed` events so the studio UI and any in-flight Lux run can react without polling.
- Studio UX:
  - Pending `ImageElement` renders as a brand-tinted placeholder with a spinner, the Maci avatar, and the prompt caption. Operator can cancel the request.
  - On `asset_request.resolved`, the placeholder swaps to the real image, preserving bbox, crop, fit, z-order, and any operator edits to the slot's position.
  - On `asset_request.failed`, the placeholder shows the error with "Retry" and "Upload manually instead" actions.
  - Operator can request a new asset on any `ImageElement` via an "Ask Maci" button — same pipeline, `requested_by = user_id`.
- Direct upload path (independent of Maci): drag-and-drop image onto canvas → `POST /api/media-assets` → `ImageElement` with `asset_id`.
- Asset library panel in studio: browse org-scoped `MediaAsset`s with FTS search (already supported by `MediaAsset::search`).
- Thumbnail generation: on upload or on Maci-resolution, resvg-js renders a 256px thumbnail, cached.
- Access control: assets scoped to `organization_id`; `require_org_membership` enforced on all routes.

**Acceptance**
- Operator uploads a logo via drag-and-drop, drops it on the cover slide, saves, reloads — persists correctly.
- Lux runs a Polish-stage generation on a deal in dev, places 3 pending asset slots via `request_asset_from_maci`, continues composing other slides, and within ~60s the placeholders swap to Maci-generated images.
- Operator clicks "Ask Maci" on an existing image, writes a new brief, sees the placeholder, waits, sees the replacement.
- In prod (where ComfyUI is absent), `request_asset_from_maci` returns an error surfaced as a pending-with-failure state in the studio ("Maci unavailable — upload manually"). Lux does not abort the rest of the run.

**Risks**
- Maci / ComfyUI latency is high (Flux runs ~30–120s per image). Mitigation: async request model; operator can advance work while assets cook.
- Maci task queue backpressure if Lux requests many assets at once. Mitigation: cap Lux to ≤6 concurrent requests per deck run; additional requests queue and resolve in priority order.
- ComfyUI dependency: `127.0.0.1:8188` must be up in dev. Mitigation: feature-flag `MACI_IMAGE_GEN_ENABLED` defaults on in dev (via flox `ENABLE_COMFYUI=1`), off in prod until a prod image-gen path is chosen. When off, `request_asset_from_maci` returns a structured unavailability error immediately.
- Maci returns an image that doesn't match the slot aspect ratio. Mitigation: Maci's task includes the target aspect ratio; backend validates and forwards to resvg for center-crop fit as a safety net.
- Agent coupling: changes to Maci's `generate_cinematic_image` signature could break Lux. Mitigation: the `request_asset_from_maci` tool is a stable contract owned by the deck subsystem; it translates to whatever Maci's current function shape is.

---

## Phase 8 — Preflight quality gates + extended exports

**Deliverables**
- `POST /api/decks/:id/preflight` → returns `PreflightReport` with:
  - `contrast`: APCA contrast on every text-over-background pair; flag < 60 Lc for body, < 75 Lc for headlines.
  - `grid`: each element's x/y is a multiple of the brand token `spacing.grid` (default 8px); flag off-grid elements.
  - `bleed`: for print target, verify 3mm bleed outside trim box; flag elements inside safe margin.
  - `fonts`: all referenced fonts are embeddable.
  - `overflow`: text boxes that exceed their bbox height.
- Studio UI: preflight panel with jump-to-issue; "Ready to Present" button disabled until all P0 checks pass.
- Stage transition: Polish → Present gate refuses to advance if P0 preflight fails (with override flag for operator).
- PPTX export: via `pptxgenjs` in the Node sidecar. Text, images, shapes map to PPTX equivalents; gradients and paths are rasterized.
- PDF/X-4 export hardened with `pdfcpu validate` in the gate.

**Acceptance**
- A deck with an off-grid text element cannot advance past Polish without an explicit override.
- Exported PPTX opens in Keynote and PowerPoint with text editable.

**Risks**
- PPTX export is inherently lossy vs SVG/PDF. Mitigation: UI clearly labels PPTX as "collaborative export" vs PDF as "presentation output".

---

## Phase 9 — Client-facing review portal

**Deliverables**
- `POST /api/decks/:id/share-links` — creates a tokenized, expiring, read-only share link scoped to a single deck version. Tokens are single-use per email (or anonymous). Stored in new table `deck_share_links (id, deck_id, deck_version, token, expires_at, viewer_email?, created_by, revoked_at?)`.
- `GET /share/decks/:token` — unauthenticated public route; resolves token, returns the rendered deck (SVG thumbnails + full-slide views) in a minimal client-facing viewer that does NOT require a dashboard login.
- Per-slide comment threads: `deck_comments (id, deck_id, slide_id, author_name, author_email?, body, created_at, resolved_at?)`. Clients comment via the share link; comments surface back into the studio as pinned notes on each slide.
- Studio UI: "Share with client" button generates a link, copies to clipboard, optionally emails the client. Comment panel shows unresolved client comments with a "mark resolved" action.
- Revocation: operators can revoke links from the studio; revoked links return a friendly "this link has expired" page.
- Analytics: track link opens, slide views, time-per-slide for operator feedback.
- Frozen snapshot: share links always render the deck version that was current at link-creation time, even if the operator keeps editing. Operators can regenerate the link against the latest version.

**Acceptance**
- Operator creates a share link, sends to a test client, client opens in an incognito window, navigates all slides, leaves a comment on slide 3, operator sees the comment in-studio.
- Revoked link shows a 410 Gone page.
- Edited deck does NOT leak edits to an existing share link until the operator regenerates it.

**Risks**
- Public route exposed without auth is attack surface. Mitigation: rate-limit per token + per-IP; tokens are 32-byte random; no DB queries outside the scoped deck; content security policy on the public viewer.
- Comment spam. Mitigation: per-link, per-IP rate limits; operator can disable comments on the link.
- GDPR on viewer emails. Mitigation: email is optional and only stored if the viewer chooses to identify themselves; all share-link data deletable via a single "revoke + purge" operation.

---

## Cross-cutting concerns

### Access control
- All deck routes enforce `require_deal_org_access` (pattern per `.claude/rules/rust-standards.md`).
- Suggestions and revisions carry `created_by`; only org editors can accept/reject.
- `parse_db_uuid_param` for all ID params.

### Error handling
- New `DeckError` enum with `From<DeckError> for ApiError`.
- No `.unwrap()` outside tests per project standards.
- Node sidecar failures return structured errors the backend can surface in the UI.

### Type generation
- ts-rs on all new models (`DeckDocument`, `Slide`, `SlideElement` union, `DesignTokens`, `PreflightReport`, `DeckSuggestion`).
- Run `npm run generate-types` after each phase; commit updated `shared/types.ts`.

### Testing
- **Unit**: schema round-trip, brand-token transformer, SVG export/import parity.
- **Integration**: end-to-end Lux run → deck persisted → rendered SVG matches expected structure.
- **Playwright**: deck-studio.spec.ts covering phases 2–4 flows; deck-collaboration.spec.ts for phase 5.
- **Regression**: golden SVG snapshots per slide layout for Satori output; fail on pixel drift > threshold.

### Observability
- `tracing::info_span` on all deck operations with `deck_id` / `deal_id`.
- New telemetry events: `deck.created`, `deck.edited`, `deck.exported`, `deck.suggestion_accepted`, `deck.preflight_failed`.
- Latency SLOs: Lux full run < 90s; single-slide render < 500ms; save round-trip < 300ms.

### Feature flags
- `LUX_STRUCTURED_DECKS_ENABLED` — phase 1 cutover without breaking markdown consumers.
- `LUX_STUDIO_EDITOR_ENABLED` — phase 3 gate.
- `MACI_IMAGE_GEN_ENABLED` — phase 7 gate. Defaults on in dev (via flox `ENABLE_COMFYUI=1`), off in prod until a prod image-gen path is chosen. Controls whether `request_asset_from_maci` actually dispatches to Maci or returns an immediate unavailability error. Owned by Maci, not Lux — flag named accordingly.
- `LUX_PPTX_EXPORT_ENABLED` — phase 8 flag for PPTX (kept off by default; SVG/PDF/PNG are P1).
- `LUX_PREFLIGHT_STRICT` — phase 8 gate for stage advance enforcement.
- `LUX_CLIENT_SHARE_ENABLED` — phase 9 gate for the public share-link routes.

---

## Risks (top of mind)

1. **Satori CSS subset** may not cover all desired layouts. *Mitigation*: scope layouts to Satori's capabilities in phase 2 prompts; escalate only when blocking a P0 layout.
2. **Illustrator SVG fidelity** is the single biggest unknown. *Mitigation*: phase 4 starts with a one-day spike to manually round-trip a hand-built SVG through AI before committing to the full export pipeline.
3. **Model tool-use reliability** for structured slide authoring. *Mitigation*: strict JSON schemas, few-shot examples, retry with schema-repair, markdown fallback.
4. **Node sidecar process management** adds operational complexity. *Mitigation*: supervised child process with health checks; document restart in runbook.
5. **Font licensing** for embedded fonts in exported PDFs. *Mitigation*: start with a curated open-licensed font set (Inter, Söhne-alike, JetBrains Mono); document licensing constraints for brand font uploads.
6. **Scope creep toward Figma parity** — easy to blow the budget chasing editor features. *Mitigation*: strict non-goals list; anything beyond the 20/80 goes to a `deferred/` doc.

---

## Open questions — RESOLVED 2026-04-12

All 6 blocking questions were answered in the planning Q&A. See the "Resolved decisions" table near the top of this doc. No open blockers remain for Phase 0 kickoff.

Remaining follow-up question (non-blocking, can be decided during Phase 7):
- **Prod image generation path** — this plan only provisions a dev ComfyUI endpoint. A follow-up sprint must choose between (a) self-hosted prod ComfyUI on GPU infra, (b) a hosted API like Replicate Flux / fal.ai / Ideogram, or (c) leaving image gen dev-only and requiring operator upload in prod. Recommendation deferred until Phase 7 is in flight and we have operator feedback on how often Lux-generated imagery is actually used.

---

## Success metrics

- **Lux deck quality**: blind operator rating of Lux output (1–5) improves from baseline (markdown outline) to ≥ 4.0 after phase 1; ≥ 4.5 after phase 6.
- **Studio adoption**: ≥ 80% of Polish-stage decks are edited in-studio before advancing to Present.
- **Round-trip reliability**: ≥ 95% of elements survive SVG → Illustrator → SVG round-trip without manual cleanup.
- **Preflight**: ≥ 90% of decks pass preflight on first attempt after phase 8.
- **Time to present**: median time from Polish stage entry to "Ready to Present" drops by ≥ 40% vs current markdown flow.

---

## Dependencies added

- **Rust**: no new major crates; existing `sqlx`, `serde`, `tokio`, `ts-rs` cover it.
- **Node (sidecar)**: `satori`, `resvg-js`, `puppeteer-core`, `@playwright/browser-chromium`, `pptxgenjs`.
- **Frontend**: `tldraw` (v3.x), `@tldraw/tlschema`, no other majors.
- **System**: `pdfcpu` or `ghostscript` for PDF/X-4 post-processing; added to flox manifest.
- **Fonts**: curated open-licensed bundle in `assets/fonts/` — Inter, DM Sans, Fraunces, Space Grotesk, JetBrains Mono (all SIL OFL, each in 400/500/700 weights minimum, with italic variants where available).
- **Docker**: ComfyUI service in `docker-compose.local.yml` (Phase 7), ~20GB of Flux model weights via bootstrap script.

---

## Out of scope / deferred (explicit)

- Real-time multi-user editing (CRDT)
- Animation and transitions
- Video embedding
- Client-facing review/comment portal (revisit after phase 5)
- Native `.ai` writing (technically impossible)
- Custom brand font upload UI (phase 6.5)
- In-studio localization/translation workflow
- Deck template library with community sharing
- AI-driven layout variation / A-B testing of layouts
