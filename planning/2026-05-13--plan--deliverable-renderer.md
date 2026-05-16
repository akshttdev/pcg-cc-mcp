# Branded Deliverable Renderer — Dashboard Feature Plan

**Date**: 2026-05-13
**Branch**: TBD (`feat/deliverable-renderer`)
**Worktree**: TBD — to be created when work begins
**Base**: `main`
**Status**: Proposal — not actively being developed (deferred until after current Sirak/TIACA Phase 1 delivery cycle)

## Why this exists

Sirak Studios currently ships final client deliverables as Word docs. The 2026-05-13 TIACA Phase 1 hand-off ran a one-off Python + headless-Chrome pipeline outside the repo at `/Users/sirakstudios/topos/_sirak/deliverables/` to convert seven docx sources into branded PDFs. That pipeline works and produced shippable output, but it is:

- Manual (operator runs a CLI on a laptop)
- Untemplated (no per-client theme system, no version control on deliverable styles)
- Disconnected from the dashboard (each client's docs sit in Dropbox, not in PCG CC MCP)
- One-shot (no preview, no live edit, no re-render on content change)

The goal is to move that capability into the dashboard so any tenant (Sirak Studios, future agency tenants) can drop markdown content into a deliverable and produce a branded PDF without leaving the product. Same input → consistent house style → trackable artifact.

## What we shipped today (reference, not in scope here)

For context — the proof point this plan codifies:

- 7 TIACA Phase 1 PDFs delivered to `Sirak Studios Dropbox/Sirak Studios Team/ACTIVE CLIENTS/TIACA/Phase 1 Deliverables (Final)/PDF/`
- CSS template at `~/topos/_sirak/deliverables/template/sirak-deliverable.css` (Inter + JetBrains Mono, A4, brand-token-driven)
- Python renderer at `~/topos/_sirak/deliverables/render.py` (text parser + headless Chrome → PDF)
- Source structure: cover, TOC, section pages with branded section number badges, branded tables, bullet lists, callouts, end slab. Brand color tokenized via CSS variable.

This is the reference implementation the dashboard feature should productize.

## Scope

### In scope
- New entity: `Deliverable` (similar shape to existing `Deck`)
- Per-tenant **brand theme** (color tokens, logo, wordmark, optional fonts)
- Markdown-content authoring with frontmatter (title, tagline, prepared-for, prepared-by, date, deliverable number/phase label)
- House template library (start with 1 template: "Strategy Document"; future: meeting deck, brief, contract, performance review)
- Server-side render to PDF via headless Chromium pool
- Storage: PDF artifacts in object storage (S3-compatible), metadata in DB
- Download + share-link surfaces in the dashboard
- Re-render when content or theme changes (with version stamps)

### Out of scope (deferred)
- WYSIWYG editor (markdown is enough for now)
- Multi-template composition (e.g., mixing strategy + deck layouts in one doc)
- Inline collaboration / comments
- Public-facing client portal for deliverable review (could plug in via existing share link infrastructure)
- DOCX export (only if a client explicitly needs round-trip editing)

## Data model

New tables in `crates/db/migrations/`:

```sql
-- Themes are owned by an organization; one default per org
CREATE TABLE deliverable_themes (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    brand_primary TEXT NOT NULL,        -- hex color, e.g. #005A9B
    brand_primary_dark TEXT,
    brand_primary_darker TEXT,
    body_font TEXT NOT NULL DEFAULT 'Inter',
    mono_font TEXT NOT NULL DEFAULT 'JetBrains Mono',
    logo_url TEXT,
    wordmark_url TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Templates are global (provided by the platform); add custom-org templates later
CREATE TABLE deliverable_templates (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,          -- 'strategy-document', 'meeting-deck', etc.
    name TEXT NOT NULL,
    description TEXT,
    html_template TEXT NOT NULL,        -- Tera/Askama template
    css_template TEXT NOT NULL,         -- CSS with theme-token placeholders
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- A Deliverable is a piece of branded content tied to a client
CREATE TABLE deliverables (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    client_id TEXT REFERENCES clients(id),   -- nullable for internal/team deliverables
    theme_id TEXT NOT NULL REFERENCES deliverable_themes(id),
    template_id TEXT NOT NULL REFERENCES deliverable_templates(id),
    title TEXT NOT NULL,
    tagline TEXT,
    eyebrow_label TEXT,                  -- "DELIVERABLE 01 OF 06 | PHASE 1 — STRATEGIC FOUNDATION"
    prepared_for TEXT NOT NULL,
    prepared_by TEXT NOT NULL,
    delivery_date DATE NOT NULL,
    content_md TEXT NOT NULL,            -- markdown body with section delimiters
    status TEXT NOT NULL DEFAULT 'draft',-- draft | rendered | shared
    current_render_id TEXT REFERENCES deliverable_renders(id),
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Each render is an artifact with provenance — content + theme + template hash
CREATE TABLE deliverable_renders (
    id TEXT PRIMARY KEY,
    deliverable_id TEXT NOT NULL REFERENCES deliverables(id) ON DELETE CASCADE,
    content_sha TEXT NOT NULL,
    theme_sha TEXT NOT NULL,
    template_sha TEXT NOT NULL,
    pdf_url TEXT NOT NULL,               -- object storage URL
    page_count INTEGER NOT NULL,
    byte_size INTEGER NOT NULL,
    rendered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    rendered_by TEXT NOT NULL REFERENCES users(id)
);
```

Wire into existing `client` and `organization` entities — sits alongside `decks` rather than replacing it.

## Backend architecture

New crate or module: `crates/services/src/services/deliverables/`

```
deliverables/
├── mod.rs
├── theme.rs           # CRUD + token resolution
├── template.rs        # Template loading, hashing
├── content.rs         # Markdown parsing → structured sections
├── render.rs          # HTML composition + Chromium invocation
├── storage.rs         # PDF upload to object storage
└── api.rs             # Axum routes
```

### Render pipeline

```
Deliverable (markdown + theme_id + template_id)
    ↓
content.rs — markdown::parse → SectionTree { cover, toc, sections[], end }
    ↓
template.rs — load (html_template, css_template); resolve {{theme.*}} tokens
    ↓
render.rs — compose HTML, write to tempfile, invoke headless Chromium → PDF
    ↓
storage.rs — upload PDF, record DeliverableRender row
    ↓
emit SSE event so the dashboard updates without polling
```

### Chromium runtime

Two options:
1. **Embedded** — spawn `chromium-headless` from inside the Axum process (matches today's Python prototype). Requires Chromium in the container image; ~150 MB.
2. **Sidecar service** — separate small Rust/Node service that owns Chromium. Cleaner separation; needs deployment work.

Recommendation: start with embedded (#1) for simplicity; split into sidecar if/when render volume warrants it.

### Markdown content format

Use existing markdown-it-style parsing with a small custom extension for section delimiters:

```markdown
---
title: TIACA Strategic Positioning & Priority Framework
tagline: The operating model behind every TIACA campaign, deliverable, and decision.
eyebrow: DELIVERABLE 01 OF 06 | PHASE 1 — STRATEGIC FOUNDATION
prepared_for: TIACA — The International Air Cargo Association
prepared_by: Sirak Studios
date: 2026-05-13
template: strategy-document
theme: tiaca
---

## Section 01 — Purpose & Executive Summary

This is the first of six strategic deliverables...

### The Three Core Messages

- TIACA is the industry convener...
- Events drive the system...
- Infrastructure must match revenue opportunity...

## Section 02 — The Strategic Hierarchy
**Eyebrow:** TIMELESS — TRUE REGARDLESS OF SEASON OR EVENT CYCLE

...

| Layer | Function |
|---|---|
| Layer 1 — Events | Primary revenue driver. The commercial engine. |
| Layer 2 — Membership | Long-term value engine. The compounding ecosystem base. |
| Layer 3 — Brand & Content | Authority and trust amplification. The megaphone. |
```

This is much cleaner than today's "parse text from docx and guess at structure" approach. Markdown tables, lists, headings all round-trip natively.

## Frontend surface

Add to existing client dashboard at `/organizations/:orgId/clients/:clientId`:

- New tab: **Deliverables** (alongside Overview, Deals, Decks)
- List view: title, status (draft/rendered/shared), last render date, download button
- Detail/edit view:
  - Left pane: markdown editor (CodeMirror with markdown syntax + live preview)
  - Right pane: rendered preview iframe (re-renders on save, debounced)
  - Top bar: title, theme selector, template selector, Render/Download/Share actions
- Settings: per-org theme editor (color pickers, logo upload, font selector)

Reuse existing patterns: `useEventSourceManager` for render-completion notifications, existing share-link infrastructure for client-facing URLs.

## Rollout phases

### Phase 1 — Foundation (~1 sprint)
- Migration: `deliverable_themes`, `deliverable_templates`, `deliverables`, `deliverable_renders`
- Seed: one global "Strategy Document" template (port the CSS template from `~/topos/_sirak/deliverables/`)
- Seed: one default theme per existing org (Sirak Studios gets TIACA blue as default)
- Backend service module with stubs
- Chromium embedded in container; basic render endpoint
- CRUD API for deliverables (no UI yet)

### Phase 2 — Authoring UI (~1 sprint)
- Deliverables tab on client dashboard
- Markdown editor + live preview iframe
- Render button + status indicator
- Download artifact

### Phase 3 — Theming (~1 sprint)
- Theme editor (color, logo, fonts)
- Theme preview against the strategy template
- Per-org default theme

### Phase 4 — Template library expansion (ongoing)
- Add "Meeting Deck" template (currently `decks` table — consider consolidation)
- Add "Performance Review" template (Phase 2 TIACA pattern)
- Add "Brief" template (1-pager)
- Add "Article" template (long-form authority content)

### Phase 5 — Polish
- Share links (client-portal-style)
- Version history per deliverable
- Render diff (visual highlight of what changed since last render)
- DOCX export if a client demands round-trip editing

## Open questions

- **Consolidate with `decks`?** Decks are essentially a sibling concept. Consider either folding decks into deliverables-with-deck-template, or keeping them separate with shared render infrastructure. Lean: separate tables, shared services.
- **Per-tenant custom templates?** Probably yes eventually (Sirak wants its own templates; a future agency tenant will want their own). Phase 4+.
- **Where does Chromium run in prod?** Vercel serverless can't run Chromium; this likely means the renderer runs on a long-lived host (already true for the rest of pcg-cc-mcp).
- **Asset pipeline for embedded images?** Today's prototype only ships text content. When clients want to embed event photography, logos, or brand graphics, we need an upload + CDN layer. Likely reuse whatever decks already use.
- **Multi-tenant isolation of object storage?** Standard prefix-by-org-id pattern.

## Reference materials

- Working prototype: `~/topos/_sirak/deliverables/` (template, renderer, build output)
- Reference PDFs: `Sirak Studios Dropbox/.../TIACA/Phase 1 Deliverables (Final)/PDF/` (7 files, 2026-05-13)
- TIACA June Execution Guide PDF (Desktop) — visual reference for the design system
- Existing decks entity: `crates/db/src/models/deck.rs` — pattern to mirror

## Why deferred

Sirak Studios needs to ship Phase 1 deliverables today. The dashboard build is real engineering work (4 sprints minimum) and the manual pipeline is sufficient for the next 1–2 client cycles. Re-evaluate when (a) second non-TIACA client needs branded deliverables, or (b) Sirak Studios hits multiple-renders-per-week cadence and the manual flow becomes the bottleneck.
