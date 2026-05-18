# Docs — Collaborative Document Editor (Google Docs–style + Topsi co-pilot)

**Date**: 2026-05-14
**Branch**: TBD (`feat/docs-editor`)
**Worktree**: TBD — to be created when work begins
**Base**: `main`
**Status**: Proposal — supersedes the markdown-renderer dimension of `2026-05-13--plan--deliverable-renderer.md`. That earlier plan covered branded PDF render-out; this plan covers the authoring substrate underneath. The two together describe the full lifecycle (author → render → ship).

## TL;DR

Build **Docs** — a Google-Docs-class collaborative editor inside the PCG CC MCP dashboard where:

- Multiple human members co-edit the same document in real time with live cursors, comments, suggestions, and notes.
- A persistent **Topsi chat sidebar** runs alongside the doc. Topsi reads the doc, listens to chat, and makes edits as a real collaborator (its own cursor, its own attributed edits, undoable).
- Documents are first-class nodes in the knowledge graph. Facts (people, events, programs) are entities. Doc content references entities by ID. Change an entity, every doc that references it updates.
- Editor feel matches Google Docs: keyboard shortcuts, slash commands, `@`-mentions, outline panel, comments, suggest-vs-edit mode, share dialog, version history.

The current pain (3 team members losing information between handoffs, AI generating overlapping drafts in V1/V2/V3/V4 folders, no canonical version, blended ideas) goes away because there is one canonical document with attribution per edit and a CRDT preventing conflict-style data loss.

## Why this is the right substrate

| Problem today | What Docs solves |
|---|---|
| 7 empty `.docx` placeholders in Dropbox while real content lives in `AI Slop/Deliverables V4/` | One canonical doc per deliverable. No "which version is the truth" question. |
| Three team members edit in parallel, ideas get blended in strange ways | CRDT (Yjs) makes simultaneous edits commute. No silent merges, no "person B overwrote person A". |
| Facts duplicated across docs ("ES2026 is June 1–3" appears in 7 places) | Facts are entity nodes. The string in the doc is a reference, not a duplicate. Update once, propagate everywhere. |
| AI drafts produced out-of-band by ChatGPT and re-imported as new files | Topsi is a participant inside the doc, not an external producer. Its edits are real edits with attribution and undo. |
| No diff visibility — "what did Alicia change in V3?" requires comparing two opaque docx files | Per-edit attribution. Version history with named restore points. Suggesting mode for review-before-merge. |
| Renders happen by running a Python script on someone's laptop | Render is a server action on the canonical doc state. Anyone can trigger; nobody can render a stale version. |

## Product vision — what the user sees

### The Docs tab in a client dashboard

```
┌────────────────────────────────────────────────────────────────┐
│ TIACA → Docs                                            [+ New]│
├────────────────────────────────────────────────────────────────┤
│ Phase 1 Deliverables (Final)                                   │
│ ├── 01. Strategic Positioning & Priority Framework  · v12 · 🔵 │
│ ├── 02. Audience Segmentation & Value Proposition    · v8  · 🔵│
│ ├── 03. Campaign Architecture & Prioritization      · v6  · 🔵 │
│ ...                                                            │
│                                                                │
│ Phase 2 Operating Templates                                    │
│ └── Monthly Strategic Performance Review            · v3  · 🟢 │
│                                                                │
│ Working Drafts                                                 │
│ ├── June 2026 LinkedIn Content Plan                 · v5  · 🟡 │
│ ...                                                            │
└────────────────────────────────────────────────────────────────┘
```

Click a doc → opens the **editor view**:

```
┌────┬─────────────────────────────────────────────────┬──────────┐
│ ≡  │  [B I U S]  [H1 H2 H3] [• 1.]  [⌘K]  Suggesting │ Topsi    │
│ Out├─────────────────────────────────────────────────┤          │
│ li │  TIACA Strategic Positioning &                  │ ▣ chat   │
│ ne │  Priority Framework                             │          │
│    │                                                 │ You:     │
│ ─  │  [SECTION 01] Purpose & Executive Summary       │ "summa-  │
│ S1 │                                                 │ rize the │
│ S2 │  This is the first of six strategic delivera-   │ three    │
│ S3 │  bles in TIACA's Phase 1 engagement with Sirak  │ core     │
│ S4 │  Studios|              ← Alicia's cursor (gold) │ messages"│
│ S5 │                                                 │          │
│ S6 │  The framework separates two planning levels:   │ Topsi:   │
│    │  a timeless Strategic Hierarchy, which defines  │ "Done —  │
│    │  how TIACA creates value...                     │ inserted │
│    │                                                 │ a callout│
│    │  💬 "Should we mention 'and Port Polska'?"     │ at the   │
│    │     — Aaren · 2h ago · 1 reply                 │ top of   │
│    │                                                 │ §1. Want │
│    │  Topsi is typing...      ← Topsi's cursor (cyan)│ me to    │
│    │                                                 │ also..." │
└────┴─────────────────────────────────────────────────┴──────────┘
```

### Native features (parity with Google Docs)

- **Keyboard shortcuts**: ⌘B/I/U, ⌘K for links, ⌘⌥1/2/3 for headings, ⌘/ for slash menu, ⌘⇧F for full-screen
- **Slash command menu**: `/` opens an inline picker for headings, lists, callouts, tables, dividers, entity references, image embed, page break
- **Toolbar**: persistent at top, surfaces formatting state of current selection
- **Outline panel**: collapsible left rail listing all H1/H2/H3; click to jump
- **Comments**: select text → comment icon appears → opens a thread. Anchored to text via Yjs relative position so comments survive edits. Resolved/unresolved states. `@`-mention to ping a teammate or Topsi.
- **Suggesting mode**: toggle from the toolbar. Edits become tracked changes with accept/reject. Useful for client review or junior-to-senior handoff.
- **Highlight**: select text → highlight color picker. Doesn't carry semantic weight, just a visual flag.
- **Side notes**: a margin-anchored note (like Notion's "callout" but lives in the margin, doesn't break the flow). Useful for "remember to revisit this" or "we still need data here".
- **Version history**: sidebar panel showing named restore points, per-user changes, "restore this version" action
- **Share dialog**: viewer/commenter/editor permissions; copy link; revoke
- **Live cursors with names + colors**: each present user gets a distinct color, their cursor is always visible with a name tag
- **Presence list**: "Aaren, Alicia, Topsi are here" pill in the top-right
- **Find/replace**: ⌘F find, ⌘⇧F replace, regex toggle
- **Export**: render to branded PDF (the existing pipeline), Markdown, plain-text. DOCX export is deferred — clients should be invited as commenters instead of receiving Word files.

### What Google Docs doesn't have (our advantages)

- **Entity references** — type `@Glyn` or `#ES2026` and an entity chip appears. The chip displays the live value (name, date, etc.). If the entity updates, every chip in every doc updates automatically.
- **Topsi chat sidebar** — a persistent right-side chat with Topsi where you can ask questions about the doc, ask for edits, ask for a summary, ask Topsi to "rewrite section 3 in a more direct tone". Topsi reads the live doc state and applies edits as a real collaborator. Conversation history persists per doc.
- **Branded PDF render** — one click → on-brand PDF rendered server-side from the canonical doc state. No round-tripping through Word.
- **Cross-doc graph** — references from one doc to another are first-class. A new doc can "inherit context from Deliverable 01" and link sections.

## Architecture

### Editor stack

| Layer | Tech | Why |
|---|---|---|
| Editor UI | **TipTap** v2 | React-native ProseMirror wrapper. Mature extension ecosystem. Headless rendering = full design control. Native Yjs integration. |
| Document model | **ProseMirror** | Industry-standard rich-text model. Schema-driven. Same engine behind Notion, Linear, Atlassian. |
| CRDT | **Yjs** v13 | Canonical CRDT for collaborative editing. Battle-tested at scale (Linear, Sketch, Anthropic Artifacts). Tiny wire format. Excellent ProseMirror binding (`y-prosemirror`). |
| Sync transport | **y-websocket** protocol | Server-mediated. Each editing client connects to the sync server with the doc ID; ops flow bidirectionally. |
| Presence | **y-protocols/awareness** | Cursor + selection + user identity broadcast separately from doc ops. Ephemeral (no persistence). |
| Persistence | Postgres `bytea` column | Doc state serialized as Yjs binary update. Snapshots written every N seconds or N ops. |

### Sync server (Rust)

`crates/services/src/services/docs/sync.rs` — a Rust implementation of the y-websocket protocol using the [`yrs`](https://crates.io/crates/yrs) crate (the official Yjs Rust port). Each WebSocket connection joins a document "room"; ops are broadcast to all other connected clients and persisted to Postgres on a debounce.

```
┌─ Client A ────────┐   ┌──────────────────────────┐   ┌─ Client B ─┐
│ TipTap + Yjs doc  │←─→│   pcg-cc-mcp sync server │←─→│  Yjs doc   │
│ y-websocket prov. │   │   (Rust + yrs + axum WS) │   │            │
└───────────────────┘   │           ↓              │   └────────────┘
                        │     Postgres bytea       │
                        │     (snapshot + log)     │
                        └──────────────────────────┘
```

Snapshotting strategy:
- Append-only Yjs update log (`document_updates` table) — every op is persisted as a small binary blob keyed by `(doc_id, sequence)`.
- Compacted snapshots (`document_snapshots` table) written on a debounce (every 30s of activity or 100 ops, whichever first). The snapshot is the merged state at that moment.
- On client connect: server replays from the latest snapshot + subsequent ops, then attaches the live channel.
- Compaction job purges old ops past the most recent snapshot.

### Storage schema

```sql
-- A document is a node in the graph.
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    client_id TEXT REFERENCES clients(id),       -- nullable for internal docs
    parent_doc_id TEXT REFERENCES documents(id), -- for "inherits context from" links
    title TEXT NOT NULL,
    kind TEXT NOT NULL,                          -- deliverable | brief | note | template | spec | research
    template_id TEXT REFERENCES deliverable_templates(id),  -- for branded render
    theme_id TEXT REFERENCES deliverable_themes(id),
    status TEXT NOT NULL DEFAULT 'draft',        -- draft | review | final | archived
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Compacted state. Latest row per doc is the rehydration source.
CREATE TABLE document_snapshots (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    state_bytes BYTEA NOT NULL,                  -- Yjs encoded state
    op_count INTEGER NOT NULL,                   -- ops merged into this snapshot
    label TEXT,                                  -- optional named restore point
    created_by TEXT REFERENCES users(id),        -- nullable for auto-snapshots
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX ON document_snapshots (document_id, created_at DESC);

-- Append-only op log between snapshots.
CREATE TABLE document_updates (
    id BIGSERIAL PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    update_bytes BYTEA NOT NULL,                 -- Yjs update message
    author_id TEXT NOT NULL,                     -- user_id OR agent_id
    author_kind TEXT NOT NULL,                   -- 'user' | 'agent'
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX ON document_updates (document_id, id);

-- Sharing & permissions.
CREATE TABLE document_collaborators (
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    actor_id TEXT NOT NULL,                      -- user_id OR agent_id
    actor_kind TEXT NOT NULL,                    -- 'user' | 'agent'
    role TEXT NOT NULL,                          -- owner | editor | suggester | commenter | viewer
    added_by TEXT REFERENCES users(id),
    added_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (document_id, actor_id)
);

-- Comments live outside Yjs because they need their own auth model + queries.
CREATE TABLE document_comments (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    thread_root_id TEXT REFERENCES document_comments(id),  -- self-ref for thread roots
    author_id TEXT NOT NULL,
    author_kind TEXT NOT NULL,
    body_md TEXT NOT NULL,
    range_anchor BYTEA NOT NULL,                 -- Yjs relative position (survives edits)
    resolved BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX ON document_comments (document_id, resolved, created_at DESC);

-- Topsi chat sidebar — one thread per (user, doc) pair.
CREATE TABLE document_chat_threads (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    agent_id TEXT NOT NULL,                      -- which Topsi instance
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (document_id, user_id, agent_id)
);
CREATE TABLE document_chat_messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES document_chat_threads(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL,
    author_kind TEXT NOT NULL,
    body_md TEXT NOT NULL,
    edit_action_id TEXT REFERENCES document_agent_actions(id),  -- if this message triggered a doc edit
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX ON document_chat_messages (thread_id, created_at);

-- Agent actions — every doc edit Topsi makes is logged for attribution + undo.
CREATE TABLE document_agent_actions (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    triggered_by_message_id TEXT REFERENCES document_chat_messages(id),
    summary TEXT NOT NULL,                       -- "Added a callout at top of §1"
    op_range_start BIGINT NOT NULL,              -- document_updates.id (inclusive)
    op_range_end BIGINT NOT NULL,                -- document_updates.id (inclusive)
    undone_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Entity references inside docs.
CREATE TABLE document_entity_refs (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    span_id TEXT NOT NULL,                       -- unique within the doc, also stored in Yjs as mark attribute
    entity_type TEXT NOT NULL,                   -- person | event | program | audience | company | deliverable
    entity_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (document_id, span_id)
);
CREATE INDEX ON document_entity_refs (entity_type, entity_id);  -- "what docs reference Glyn?"
```

### Entity references in the editor

Slash command `/entity` or `@` mention opens an entity picker:

```
/entity Glyn      → Person: Glyn Hughes (Director General, TIACA)
/entity ES2026    → Event: TIACA Executive Summit 2026 (June 1–3, Warsaw)
/entity BlueSky   → Program: BlueSky Sustainability Assessment
```

Picking inserts a ProseMirror `entityRef` mark with `entity_type` + `entity_id` attributes and a stable `span_id`. The rendered chip displays the entity's current value pulled from the graph at view time.

When an entity is edited (e.g. Glyn's title changes), no doc-content mutation is needed — the chip just renders the new value. At PDF render time, the renderer resolves `entityRef` marks against the current entity state.

This is the propagation guarantee that makes the knowledge-graph substrate worth the build: change once, every reference reflects the truth.

## Agent integration — Topsi as a first-class collaborator

### Model

Topsi is not a special-cased "AI feature" bolted onto the editor. Topsi is a real participant in the Yjs document:

- It has an agent identity (`agent_id`) with a name, avatar, and color.
- It appears in the presence list when active (`"Topsi is here"`).
- Its cursor and selection are visible to other users.
- Its edits are real Yjs ops attributed to it.
- Its edits are undoable like any user's — but grouped at the **action** level, not the keystroke level (so undoing one Topsi action reverts the whole edit, not just the last character).

### Chat sidebar flow

```
User opens doc → Topsi chat thread is loaded (or created if first visit).
User types: "Rewrite section 3 in a more direct tone."
[user message persisted; sent to agent runtime with current Yjs doc state attached]

Agent runtime (existing Topsi infrastructure):
  1. Reads doc state via the editor's read API
  2. Produces a chat reply  + (optionally) a list of edit operations
  3. Streams reply tokens back to the chat sidebar
  4. Applies edits as Yjs ops attributed to its agent identity
  5. Records the action in document_agent_actions for undo grouping

User sees in real time:
  - Topsi's cursor moves to §3
  - Text in §3 changes as Topsi "types" (the Yjs ops stream in)
  - Chat sidebar shows "Done — rewrote §3 with a more declarative cadence. 
                       Want me to apply the same tone pass to §4?"
  - Notification chip above the doc: "Topsi edited §3 · Undo" (5s ephemeral)

User clicks Undo → all of Topsi's ops from that action are reverted in one step.
User likes it → does nothing, edit sticks.
```

### Agent edit modes

The chat sidebar exposes a mode toggle:

- **Direct edit** (default): Topsi applies edits immediately. Best for trusted operations and small tweaks.
- **Suggest edit**: Topsi inserts a tracked-changes suggestion. User reviews and accepts/rejects. Best for substantive content changes.

### What Topsi can do in a doc

| Operation | Direct | Suggest |
|---|---|---|
| Rewrite a paragraph in a different tone | ✓ | ✓ |
| Add a new section from a brief | ✓ (with confirmation) | ✓ |
| Summarize a section into a callout | ✓ | ✓ |
| Insert entity references where appropriate | ✓ | — |
| Find inconsistencies across sections | (proposes only, doesn't auto-fix) | — |
| Apply a template change across all sections | (proposes only) | ✓ |
| Pull facts from another doc in the same client | ✓ | ✓ |
| Render the doc to PDF | ✓ (action, not edit) | — |

### Concurrency between Topsi and humans

Yjs handles this naturally — Topsi's ops merge with human ops without conflict. The only failure mode is **semantic collision** (Topsi rewrites a paragraph while Alicia is mid-sentence). Mitigation:

- Topsi's edit prompt includes the current presence map. If a human is actively editing a range, Topsi waits or scopes around it.
- When Topsi starts a multi-second edit action, it broadcasts an `awareness` flag (`busy: §3`) so collaborators see the indicator.
- Topsi's edits are atomic at the action level — a partial action mid-failure rolls back cleanly.

## Frontend surface

### New routes
- `/organizations/:orgId/clients/:clientId/docs` — doc list view
- `/organizations/:orgId/clients/:clientId/docs/:docId` — editor view
- `/organizations/:orgId/docs/:docId` — for org-level docs not tied to a client

### Component breakdown

```
DocsListPage
  DocsToolbar (search, filter, new doc)
  DocsTable
    DocsRow (title, last edited, contributors, status)

DocEditorPage
  DocHeader (title, status, share, presence list, more menu)
  EditorMain
    EditorToolbar (formatting, structure, mode toggle, find)
    OutlinePanel (left rail, collapsible)
    EditorCanvas (TipTap instance bound to Yjs doc)
      EntityChip (rendered for entityRef marks)
      InlineCommentMarker
      SuggestionMark
      MarginNoteAnchor
    CommentsSidebar (right rail, threads list + thread detail)
    TopsiChatSidebar (right rail, switchable with comments)
      ChatThread (message list)
      ChatComposer (input + mode toggle)
      AgentActionLog (audit trail of edits Topsi made)
  ShareDialog (modal — invite by email, copy link, permission tiers)
  VersionHistoryPanel (drawer — snapshots, named restore points)
  ExportMenu (PDF, Markdown, plain text)
```

### Editor extensions (TipTap)

| Extension | Purpose |
|---|---|
| `@tiptap/extension-document` + `paragraph` + `text` | Core schema |
| `@tiptap/extension-heading` | H1/H2/H3 |
| `@tiptap/extension-bold`/`italic`/`underline`/`strike` | Marks |
| `@tiptap/extension-bullet-list` + `ordered-list` + `list-item` | Lists |
| `@tiptap/extension-table` + `table-row` + `table-cell` + `table-header` | Tables (branded styling) |
| `@tiptap/extension-link` | Hyperlinks |
| `@tiptap/extension-highlight` | Highlight marks |
| `@tiptap/extension-placeholder` | "Type / for commands" |
| `@tiptap/extension-collaboration` + `y-prosemirror` | Yjs binding |
| `@tiptap/extension-collaboration-cursor` | Live cursors with awareness |
| `@tiptap/extension-history` (disabled — use Yjs undo manager instead) | — |
| Custom: `EntityRefExtension` | Entity chips with picker |
| Custom: `CommentExtension` | Inline comment marks anchored via relative position |
| Custom: `SuggestionExtension` | Track-changes-style suggestions |
| Custom: `MarginNoteExtension` | Margin-anchored notes |
| Custom: `SlashCommandExtension` | `/` menu |
| Custom: `CalloutExtension` + `PullQuoteExtension` | Branded structural blocks (match the PDF template) |

## Backend surface

### REST endpoints

```
GET    /api/docs                                      # list (filtered by client/org/status)
POST   /api/docs                                      # create from template
GET    /api/docs/:id                                  # metadata (not content — content via WS)
PATCH  /api/docs/:id                                  # rename, change status, set template
DELETE /api/docs/:id                                  # archive

GET    /api/docs/:id/collaborators
POST   /api/docs/:id/collaborators                    # invite
DELETE /api/docs/:id/collaborators/:actorId

GET    /api/docs/:id/comments                         # threads (anchored, with resolve state)
POST   /api/docs/:id/comments                         # new thread
POST   /api/docs/:id/comments/:threadId/replies
PATCH  /api/docs/:id/comments/:threadId               # resolve/unresolve

GET    /api/docs/:id/chat                             # Topsi thread for current user
POST   /api/docs/:id/chat                             # send message — triggers agent run
DELETE /api/docs/:id/chat                             # clear thread

GET    /api/docs/:id/history                          # snapshots list
POST   /api/docs/:id/history                          # named restore point
POST   /api/docs/:id/history/:snapshotId/restore      # revert to snapshot

POST   /api/docs/:id/render                           # branded PDF render → artifact URL
GET    /api/docs/:id/export?format=md|txt             # plain export

POST   /api/docs/:id/agent-actions/:actionId/undo     # undo a Topsi action
```

### WebSocket endpoints

```
ws://.../api/docs/:id/sync           # y-websocket protocol — doc ops + awareness
ws://.../api/docs/:id/chat-stream    # Topsi reply token stream (SSE-style over WS)
```

### Agent execution flow

```
POST /api/docs/:id/chat { body }
    ↓
Persist user message → document_chat_messages
    ↓
Enqueue agent task with: { doc_id, user_id, message_id, doc_state_snapshot }
    ↓
Topsi runtime (existing infrastructure):
  - Receives task with current doc state
  - Decides: reply-only OR reply + edits
  - Streams reply tokens to chat-stream WS
  - If edits: opens its own y-websocket connection as the agent identity
              applies Yjs ops, all attributed to agent_id
              records document_agent_actions row spanning the op range
              writes a chat message linking to that action
```

## Permissions model

| Role | Read doc | Comment | Suggest | Edit directly | Share | Delete |
|---|---|---|---|---|---|---|
| Viewer | ✓ | — | — | — | — | — |
| Commenter | ✓ | ✓ | — | — | — | — |
| Suggester | ✓ | ✓ | ✓ | — | — | — |
| Editor | ✓ | ✓ | ✓ | ✓ | — | — |
| Owner | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

Agents follow the same model. Topsi by default joins as `Editor` (it makes direct edits) but can be downgraded to `Suggester` per-doc if a team wants approval-before-merge.

Sharing: any Editor can invite collaborators below their own role. Only Owners can invite other Owners or change a collaborator to/from Owner.

## Version history + contribution tracking

This is treated as a first-class feature, not a bolted-on log. Match every signal Google Docs surfaces — plus the ones GDocs doesn't (agent attribution, entity-change ripple, render history).

### What the user sees

**1. Contributor strip in the doc header**

```
TIACA Strategic Positioning & Priority Framework
🟣 Aaren · 🟡 Alicia · 🔵 Bodhi · 🟢 Topsi              v12 · saved 4s ago
```

A horizontal cluster of avatars for every contributor (ranked by recency of contribution). Hover a chip → tooltip with last-edit timestamp and edit count. Click → filters the version-history sidebar and the in-doc highlight overlay to that contributor.

**2. "Last edited by" footer**

```
Last edit: Topsi · "Rewrote §3 in a more declarative cadence" · 2 minutes ago
```

Bottom-of-page tag, identical pattern to GDocs' "Last edit was Wednesday at 4:13 PM". One line, scannable, click → opens the relevant version-history entry.

**3. Version history sidebar (the GDocs `File → Version history → See version history` analog)**

```
┌──────────────────────────────────┐
│  Version history                 │
│  ────────────────                │
│                                  │
│  Today                           │
│   ● 14:22  Topsi edited §3       │
│   ● 14:08  Aaren · 3 edits       │
│   ● 11:30  Named: "Pre-Alicia   │
│            review"               │
│            by Bodhi              │
│                                  │
│  Yesterday                       │
│   ● 22:45  Alicia · 12 edits     │
│   ● 09:10  Bodhi · §1, §4        │
│                                  │
│  May 12                          │
│   ● 18:00  Auto-snapshot          │
│   ● Created · Bodhi              │
│                                  │
│  [+ Name this version]           │
└──────────────────────────────────┘
```

Each entry shows: timestamp, author (with avatar/color), summary of what changed (section IDs touched, op count or one-line description for agent actions), and a "Restore" action. Named versions appear with a label and the user who named them.

Clicking a version opens a **two-pane diff view**:
- Left: doc as it was at that version
- Right: doc now
- Inline color overlays show what changed by whom (each contributor gets their own color, same as their cursor)
- Toolbar: "Filter by author", "Show only agent edits", "Show only resolved comments"

**4. Activity feed (a separate panel — full audit, not just snapshots)**

```
14:22  Topsi  rewrote §3 (Direct edit · triggered by Aaren's chat: "more direct tone")
       └ Undo this action
14:08  Aaren  added 3 bullets to §1 → The Three Core Messages
13:55  Alicia  resolved 2 comment threads
13:30  Aaren  invited alicia@sirakstudios.com as Editor
11:30  Bodhi  named version "Pre-Alicia review"
```

Activity feed includes:
- Doc edits (grouped by author per session)
- Topsi actions (with the triggering chat message inline so you can see the prompt that produced the edit)
- Comments added / replied / resolved
- Permission changes (invites, role changes, revocations)
- Renders (who exported a PDF and when)
- Named versions

Same query powers the Activity tab on the doc and the per-contributor filter view.

**5. Per-paragraph attribution (the GDocs "see edit history of this text" analog)**

Select any block or paragraph → right-click → **"Show contributors"**. Surfaces:
- Original author (whoever first inserted this block)
- Last editor (whoever most recently changed it)
- Full contributor list for this block (everyone who edited)
- Inline diff: highlight characters by author color

Implementation: at view time, walk the Yjs op log for ops whose range intersects the selected block, group by `author_id`. Cached per snapshot for speed. v1 is per-block ("who last touched §3?"); per-character blame is a v2 nice-to-have.

**6. Topsi-action surfacing**

Topsi's actions get distinct visual treatment because users want to know what was AI-edited vs human-edited:

- Topsi's cursor color is reserved (cyan by default; theme-able per org)
- The contributor strip puts a small ⚡ icon next to Topsi's avatar
- Version history entries for Topsi actions show the triggering chat message inline ("triggered by Aaren's chat: '…'")
- Filter: "Hide agent edits" toggle in version history + activity feed
- An org-level setting: "Require approval before Topsi direct-edits" (forces Suggest mode globally)

### Backend support

The schema already captures everything needed:

- `document_updates` — every op with `author_id` + `author_kind`. This is the source of truth for who-did-what.
- `document_snapshots` — periodic compacted state with `created_by` (the user/agent whose op triggered the snapshot) + optional `label`.
- `document_agent_actions` — Topsi edits grouped at the action level with `triggered_by_message_id` linking back to the chat that prompted them.
- `document_collaborators` — sharing/role audit trail (already includes `added_by` + `added_at`).

New endpoints to support the UX:

```
GET    /api/docs/:id/activity?cursor=...&filter=...    # paginated activity feed
GET    /api/docs/:id/contributors                      # ranked list with last-seen + edit count
GET    /api/docs/:id/blame?block_id=...                # per-block attribution
GET    /api/docs/:id/diff?from=:snapshotId&to=current  # rendered HTML diff with author marks
POST   /api/docs/:id/history/named                     # explicit "Name this version"
```

The activity feed is a union query over `document_updates` (grouped into sessions), `document_comments`, `document_collaborators` changes, `document_agent_actions`, and `deliverable_renders` (from the renderer plan), all keyed by `document_id` and sorted by `created_at`.

### Retention

- Activity feed: kept forever. Cheap (rows, not blobs).
- Op log (`document_updates`): kept forever for blame queries. Compaction is structural (merge into snapshots) but doesn't drop attribution.
- Snapshots: every 30s of activity + every named version + on-render. Permanent.
- Yjs binary state in snapshots is ~5–50KB per doc per snapshot; manageable.

### What this looks like in the editor UI (Phase 2 deliverable)

```
┌── Doc header ────────────────────────────────────────────────────┐
│ TIACA Strategic Positioning & Priority Framework                 │
│ 🟣🟡🔵🟢 + 2 more   ·  v12 · saved 4s ago   [Share] [History] [⋯]│
├──────────────────────────────────────────────────────────────────┤
│ ... editor content ...                                           │
│                                                                  │
│ Last edit: Topsi · "Rewrote §3" · 2 minutes ago                 │
└──────────────────────────────────────────────────────────────────┘
```

Hover any contributor avatar → mini-card:
```
┌── Alicia ───────────────────────┐
│ Last edited 11 minutes ago      │
│ 47 edits this week              │
│ Currently viewing §3            │
│ [Open profile]                  │
└─────────────────────────────────┘
```

## Rollout phases

### Phase 1 — Editor foundation (1 sprint)
- Schema migrations: `documents`, `document_snapshots`, `document_updates`, `document_collaborators`
- Rust sync server (yrs + axum WS handler)
- TipTap editor mounted at `/docs/:id` with the standard extensions
- Yjs binding + WebSocket provider
- Live cursors with presence
- Branded toolbar + slash menu (basic commands)
- Create doc from template (using existing renderer's templates as starting content)
- Tested: 2 simultaneous editors don't lose work

### Phase 2 — Comments + sharing + history + contributor tracking (1.5 sprints)
- `document_comments` schema + REST
- Inline comment marks anchored via Yjs relative position
- Comments sidebar UI with `@`-mention pings
- Share dialog + permissions enforcement
- **Contributor strip** in doc header (avatar cluster, hover details)
- **"Last edited by" footer** with link into version history
- **Version history sidebar** — chronological session-grouped entries with author attribution + diff view
- **Activity feed panel** — union view of edits, comments, sharing changes, renders
- **Per-block blame** ("Show contributors" right-click action)
- **Named restore points** + restore action
- **Author filter** in history + activity panels
- Resolved/unresolved comment state

### Phase 3 — Topsi chat sidebar + agent edits (2 sprints)
- `document_chat_threads`, `document_chat_messages`, `document_agent_actions` schemas
- Chat sidebar UI (collapsible, switchable with comments)
- Agent task runtime integration (extending existing Topsi infrastructure)
- Streaming reply tokens via WebSocket
- Topsi as Yjs collaborator (its own awareness identity + cursor)
- Action-grouped undo
- Direct vs Suggest mode toggle
- Action log audit trail

### Phase 4 — Entity references + propagation (1–2 sprints)
- `document_entity_refs` schema + entity model surfaces
- `entityRef` ProseMirror mark + EntityChip component
- Entity picker (slash command + `@` mention)
- Live chip rendering pulling from entity state
- "What docs reference this entity?" query for the entity detail page
- Render-time entity resolution for branded PDF

### Phase 5 — Suggestions + margin notes + polish (1 sprint)
- Suggesting mode toggle (track changes)
- Accept/reject UI
- Margin notes
- Find/replace
- Export to MD/TXT
- Keyboard shortcut reference card

### Phase 6 — Templates as docs + cross-doc references (1 sprint)
- Templates become docs themselves (editable in the same UI)
- "Inherit context from another doc" — new doc starts with reference chips to another doc's sections
- Cross-doc fact propagation (e.g. Deliverable 04's phase table referenced by the LinkedIn Content Plan)

## Open questions

1. **SQLite vs Postgres** — pcg-cc-mcp is SQLite today. SQLite handles BLOB and indexes well enough for v1, but for production with many concurrent editors and large doc histories, Postgres is the right home. Decision needed before Phase 1.
2. **Agent action attribution display** — show Topsi's edits in the version history? Inline in the doc as a different cursor color? Both? Lean: both, with a "filter by author" view.
3. **Comment notifications** — email digest, in-app only, or both? Probably in-app for v1, email for v2.
4. **Mobile** — TipTap supports mobile but the layout (left outline + center editor + right sidebar) doesn't fit phones. v1 is desktop only; "view-only with comment-add" mobile in a later phase.
5. **Offline editing** — Yjs handles offline edits natively (queues ops, merges on reconnect). Worth shipping in Phase 1 or deferring? Recommend ship in Phase 1 — it's almost free with the y-websocket reconnect logic.
6. **Doc-to-PDF round-trip** — the current render pipeline takes the Yjs doc state, walks the ProseMirror tree, emits HTML to the existing CSS template, renders via Chromium. This means the editor's ProseMirror schema must support every block the PDF needs (callouts, pull quotes, tables, eyebrows). Worth scoping the schema carefully in Phase 1 against the existing template.
7. **Entity model commitment** — what are the first-class entities? Initial proposal: Person, Event, Program, Audience, Company, Deliverable, Section. Anything else? (Topic? Theme? Stakeholder?) Recommend confirming the schema before Phase 4 starts.
8. **Topsi context window** — does Topsi see the whole doc on every prompt or just the current section + selection? Whole doc for v1 (it's small — strategy docs are 10–20K chars), section-only for very large docs as a v2 optimization.

## Risks + mitigations

| Risk | Mitigation |
|---|---|
| **Yrs (Rust port) lag behind Yjs (JS)** — protocol drift, edge cases | Pin both to specific versions, integration test on every dep bump. yrs is actively maintained by the Yjs author. |
| **Sync server scaling** — many concurrent docs/users | Stateless WS workers, room state in Postgres. Scale horizontally. |
| **Agent edits surprise users** — "what just happened to my doc" | Strong attribution UX: cursor color, action toast, undo button, version history showing every agent action. |
| **CRDT bugs lose data** | Snapshot frequently (every 30s). Keep the op log forever (cheap). Worst case: rebuild from log + replay. |
| **Schema lock-in for entities** | Phase 4 starts AFTER team agrees on first-class entity types. Migrations later are expensive. |
| **Mobile gap** for clients-on-the-go | Communicate desktop-first explicitly. Mobile view-only ships later. |

## References

- TipTap v2 docs: https://tiptap.dev/docs/editor/introduction
- Yjs: https://yjs.dev
- yrs (Rust): https://github.com/y-crdt/y-crdt
- y-websocket protocol: https://github.com/yjs/y-websocket
- ProseMirror collaborative editing model: https://prosemirror.net/docs/guide/#collab
- Anthropic Artifacts (reference impl using Yjs for AI-collaborative docs): https://www.anthropic.com/news/artifacts
- Linear's editor architecture talk: (industry reference for production-grade TipTap + Yjs)

## Relationship to other plans

- **2026-05-13--plan--deliverable-renderer.md** — that plan covered the *render-out* side (markdown source → branded PDF). This plan replaces "markdown source" with "Yjs document". The render pipeline is unchanged: ProseMirror tree → HTML → CSS template → Chromium → PDF. The renderer becomes a server action triggered from the editor instead of a CLI tool.
- **reference--sirak-admin-pattern.md** (memory) — Docs uses the same auth + brand-themed UI pattern as other Sirak admin surfaces. Inter + JetBrains Mono + brand-blue accent. The editor is brand-themed end to end (toolbar, chips, comments).

## Recommendation

Ship Phase 1 first (editor + CRDT + sharing) so the team's immediate pain — losing edits between handoffs — disappears. Then Phase 2 (comments + history) so review cycles get traceable. Phase 3 (Topsi sidebar) is the big differentiator and the reason this exists rather than "just install Notion", but it's only useful once the editor is solid.

Estimated total: 7–9 sprints to a complete v1 (Phases 1–5). Phase 6 is post-v1 polish.

Begin work after the entity-schema check (Open Question 7) is resolved.
