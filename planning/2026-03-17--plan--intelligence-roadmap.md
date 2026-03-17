# Intelligence & Sovereign Data Platform — Summary & Roadmap

**Date**: 2026-03-17
**Author**: Sirak Studios Engineering
**Status**: Phase 1 Complete, Phase 2 Planning

---

## 1. What We Built (Phase 1 — Complete)

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    EXTERNAL SOURCES                                  │
│                                                                      │
│   Dropbox Cloud ──► Dropbox Desktop App (auto-sync)                 │
│                          │                                           │
│                          ▼                                           │
│   C:\Users\sirak\Sirak Studios Dropbox\                             │
│   ├── Sirak Studios (sirak)\     ← Personal account                 │
│   └── Sirak Studios Team\        ← Team account                     │
└──────────────────────┬──────────────────────────────────────────────┘
                       │
                       │  sovereign_stack.rs (scraper, every 5 min)
                       │  • Copies new/modified files
                       │  • Skips cache junk, smart-sync stubs
                       │  • SHA-256 content hashing
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│              APN CLOUD  (E:\topos\sovereign_stack\)                 │
│              ★ Windows Explorer sidebar entry ★                     │
│                                                                      │
│   Sirak Studios/                                                     │
│   ├── Dropbox/Personal/      ← sovereign_personal (1,657 files)    │
│   ├── Dropbox/Team/          ← sovereign_org      (38,688 files)   │
│   ├── Media Pipeline/                                                │
│   ├── Uploads/               ← Dashboard uploads land here          │
│   └── .sovereign/                                                    │
│       └── apn-cloud.ico      ← Blue cloud + α icon                 │
│                                                                      │
│   Also: E:\topos\sovereign_storage\  (sovereign volume, 4 files)   │
└──────────────────────┬──────────────────────────────────────────────┘
                       │
                       │  org_cloud_indexer.rs
                       │  • Walks volumes, indexes into DB
                       │  • Deduplicates by volume + path
                       │  • MIME type detection
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      SQLITE DATABASE                                 │
│                                                                      │
│   cloud_files (45,896 rows)                                         │
│   ├── sovereign_org:      38,688  ← Sirak Studios org data         │
│   ├── data_sources:        5,547  ← Legacy synced entries          │
│   ├── sovereign_personal:  1,657  ← Aaren Sirak personal           │
│   └── sovereign:               4  ← System files                    │
│                                                                      │
│   data_sources (5,545 active rows)                                  │
│   ├── Sirak Studios/*:    3,895  (18.2 GB) — Org intelligence      │
│   └── Personal/*:         1,650  (4.5 GB)  — User intelligence     │
└──────────────────────┬──────────────────────────────────────────────┘
                       │
                       │  Axum API routes
                       │  • org_cloud.rs  — browse, download, upload
                       │  • data_sources.rs — CRUD, download, metadata
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     PCG DASHBOARD                                    │
│                                                                      │
│   ┌─────────────────────┐   ┌──────────────────────────────┐       │
│   │   MY WORKSPACE      │   │   ORGANIZATION PROFILE       │       │
│   │                     │   │                              │       │
│   │ ★ Intelligence      │   │   Intelligence tab           │       │
│   │   My Projects       │   │   ├── Data Sources (3,895)   │       │
│   │   My Tasks          │   │   ├── Artifacts              │       │
│   │   My Workflows      │   │   ├── Workflows              │       │
│   │   Calendar          │   │   ├── Pulse Engine           │       │
│   │   VIBELAND          │   │   └── Topology               │       │
│   │   VIBE              │   │                              │       │
│   └─────────────────────┘   │   Cloud tab                  │       │
│                              │   ├── Browse volumes          │       │
│   Intelligence page shows    │   ├── Preview files           │       │
│   Aaren Sirak's 1,650       │   ├── Upload to sovereign     │       │
│   personal files from        │   └── Download/stream         │       │
│   sovereign_personal         │                              │       │
│                              │   Data Sources page           │       │
│                              │   └── Org-only folder tree    │       │
│                              │       (Personal excluded)     │       │
└─────────────────────────────────────────────────────────────────────┘
                       │
                       │  sovereign_storage.rs + NATS relay
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                 APN MESH NETWORK                                     │
│                                                                      │
│   apn_sirak_master ◄──NATS──► apn_pythia_master                    │
│   (this node)                  (remote peer)                         │
│                                                                      │
│   • cloud_files metadata replicated via v0.7.0 payload              │
│   • Peers can request files from master via API                     │
│   • Heartbeat every 30s                                              │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Inventory

| Scope | Files | Size | Volume | Dashboard Location |
|-------|------:|-----:|--------|-------------------|
| Sirak Studios (Org) | 3,895 | 18.2 GB | sovereign_org | Data Sources page, Org Intelligence |
| Aaren Sirak (Personal) | 1,650 | 4.5 GB | sovereign_personal | My Workspace → Intelligence |
| Cloud Files (indexed) | 45,896 | — | all volumes | Org Cloud tab |

### File Type Breakdown (data_sources)

| Type | Count | Category |
|------|------:|----------|
| Unclassified (octet-stream) | 3,259 | Needs classification |
| PDF | 947 | Documents |
| Word (docx) | 406 | Documents |
| JPEG images | 363 | Media |
| Excel (xlsx) | 185 | Spreadsheets |
| MP3 audio | 95 | Media |
| Text/Markdown | 73 | Documents |
| WAV audio | 71 | Media |
| PowerPoint | 30 | Presentations |
| Video (mp4) | 23 | Media |
| PNG images | 15 | Media |

### Org Folder Structure (Sirak Studios)

```
Sirak Studios/
├── PROJECTS/          2,010 files  ← Client work, deliverables
├── RESOURCES/         1,089 files  ← Templates, brand assets, tools
├── ARCHIVE/             643 files  ← Completed/retired projects
├── SALES/                99 files  ← Proposals, estimates, leads
├── PROPOSALS/            47 files  ← Formal proposals
└── SOCIAL MEDIA/          7 files  ← Content calendar, assets
```

### Personal Folder Structure (Aaren Sirak)

```
Personal/
├── Aaren Stuff/             1,068 files  ← Personal documents
├── Water Golf/                328 files  ← Side project
├── Quest Of Love Video/       122 files  ← Video project
├── Meeting Recordings/         79 files  ← Google Meet, Zoom
├── Apps/                       19 files  ← App configs
├── KEY DOCS/                   18 files  ← Important personal docs
├── Screenshots/                13 files  ← Screen captures
├── Root/                        2 files  ← Misc
└── Business reports/            1 file   ← Dropbox reports
```

### Key Deliverables (Phase 1)

1. **Sovereign Stack Scraper** — `sovereign_stack.rs` copies Dropbox → E: drive every 5 min
2. **APN Cloud in File Explorer** — Windows sidebar entry with blue cloud α icon
3. **Volume Migration** — All references point to sovereign stack, not Dropbox
4. **Data Separation** — Org data vs personal data cleanly split
5. **Intelligence Page** — Personal data browser at `/intelligence` in My Workspace
6. **Cloud Browser** — Full preview support (images, video, audio, PDF, text, code, SVG)
7. **Upload Pipeline** — Files upload directly to sovereign stack, indexed automatically

---

## 2. Short-Term Roadmap (Phase 2)

### 2A. Data Classification & Enrichment

The biggest immediate gap: **3,259 files (59%)** are `application/octet-stream` — unclassified. Many are likely creative assets (.psd, .ai, .prproj, .aep, etc.) that the MIME detector missed because they were indexed from cloud_files without re-reading extension.

```
┌──────────────────────────────────────────────────────────────────┐
│                  DATA ENRICHMENT PIPELINE                         │
│                                                                   │
│   cloud_files ──► Classification Job                              │
│                   │                                               │
│                   ├── Re-detect MIME from file extension           │
│                   ├── Extract metadata (EXIF, PDF info, ID3)      │
│                   ├── Generate thumbnails (images, video, PDF)    │
│                   ├── OCR text extraction (scanned PDFs)          │
│                   └── AI tagging (project, client, date, topic)   │
│                          │                                        │
│                          ▼                                        │
│                   data_sources.metadata (enriched JSON)           │
│                   cloud_files.tags (new column)                   │
│                          │                                        │
│                          ▼                                        │
│                   Dashboard: Smart filters, search, insights      │
└──────────────────────────────────────────────────────────────────┘
```

**Tasks:**
- [ ] Batch re-classify octet-stream files by scanning file extension from cloud_files
- [ ] Add `tags` and `thumbnail_path` columns to cloud_files
- [ ] Background job: generate thumbnails for images/video/PDF (store on E:)
- [ ] Background job: extract text content from PDFs and docs for full-text search
- [ ] NORA integration: AI auto-tag files by project, client, content type

### 2B. Intelligence Dashboard Widgets

Transform raw file listings into actionable intelligence views.

```
┌─────────────────────────────────────────────────────────────────┐
│  ORGANIZATION INTELLIGENCE (Org Profile → Intelligence tab)      │
│                                                                   │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────────┐ │
│  │ Storage       │ │ Activity     │ │ Data Health              │ │
│  │ 22.7 GB total │ │ 47 new today │ │ ██████████░░ 59% typed   │ │
│  │ 5,545 files   │ │ 312 this wk  │ │ 3,259 need classifying   │ │
│  └──────────────┘ └──────────────┘ └──────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ BY PROJECT                                                   │ │
│  │ Prime Hospitality    ████████████████░░ 847 files  4.2 GB   │ │
│  │ Resonance Website    ██████████░░░░░░░░ 423 files  2.1 GB   │ │
│  │ Sirak Studios Brand  ████████░░░░░░░░░░ 312 files  1.8 GB   │ │
│  │ Water Golf           ██████░░░░░░░░░░░░ 328 files  0.9 GB   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌──────────────────────┐ ┌────────────────────────────────────┐ │
│  │ BY TYPE              │ │ RECENT ACTIVITY                    │ │
│  │ 📄 Documents   1,461 │ │ • 3 PDFs added to PROPOSALS       │ │
│  │ 🖼 Images       378  │ │ • New folder: PROJECTS/AcmeCo     │ │
│  │ 🎵 Audio        166  │ │ • 12 files synced from Dropbox    │ │
│  │ 🎬 Video         23  │ │ • Invoice.xlsx updated            │ │
│  │ 📊 Spreadsheets 185  │ │ • Meeting recording transcribed   │ │
│  │ ❓ Unclassified 3,259│ └────────────────────────────────────┘ │
│  └──────────────────────┘                                        │
└─────────────────────────────────────────────────────────────────┘
```

**Tasks:**
- [ ] Build `IntelligenceDashboard` component with stat cards
- [ ] Add storage usage chart (by volume, by project, by type)
- [ ] Add recent activity feed (from Pulse Engine events)
- [ ] Add data health indicator (% classified, % with thumbnails)
- [ ] Link project folders to actual PCG projects in the DB

### 2C. Personal Intelligence (My Workspace → Intelligence)

Enhance the personal Intelligence page beyond a file browser.

```
┌─────────────────────────────────────────────────────────────────┐
│  PERSONAL INTELLIGENCE (/intelligence)                           │
│                                                                   │
│  ┌─ Tabs ──────────────────────────────────────────────────────┐ │
│  │  Files │ Meetings │ Key Docs │ Activity │ Insights          │ │
│  └────────────────────────────────────────────────────────────-┘ │
│                                                                   │
│  FILES tab (current):                                            │
│    Folder tree + file list with search/filter/sort               │
│                                                                   │
│  MEETINGS tab (new):                                             │
│    Meeting recordings with transcripts and action items          │
│    └── 79 recordings from Google Meet / Zoom                     │
│    └── Auto-extract attendees, duration, topics                  │
│    └── NORA: summarize + extract action items                    │
│                                                                   │
│  KEY DOCS tab (new):                                             │
│    Pinned important documents (tax, legal, contracts)            │
│    └── User can star/pin files from any folder                   │
│    └── Quick access to frequently referenced docs                │
│                                                                   │
│  ACTIVITY tab (new):                                             │
│    Personal Pulse feed — what changed in your files              │
│                                                                   │
│  INSIGHTS tab (future):                                          │
│    AI-generated summaries of personal data patterns              │
│    └── "You have 12 unsigned contracts"                          │
│    └── "3 meeting recordings from last week unreviewed"          │
└─────────────────────────────────────────────────────────────────┘
```

**Tasks:**
- [ ] Add tab navigation to Intelligence page (Files, Meetings, Key Docs)
- [ ] Meetings sub-view: filter audio files from Meeting Recordings folder
- [ ] Key Docs: add `pinned` flag to data_sources, show pinned files
- [ ] Activity feed: query Pulse Engine for personal file events
- [ ] NORA integration: "Summarize my recent meetings" command

### 2D. Pulse Engine Integration — Real-Time Sync

New uploads and file changes should flow through Pulse Engine for real-time dashboard updates.

```
┌──────────────────────────────────────────────────────────────────┐
│                    PULSE ENGINE FLOW                               │
│                                                                   │
│   File Upload (Dashboard)                                         │
│       │                                                           │
│       ├──► Save to E:\topos\sovereign_stack\...\Uploads\          │
│       ├──► INSERT into cloud_files + data_sources                 │
│       └──► Publish pulse event: "file.created"                    │
│                │                                                  │
│                ▼                                                  │
│   Pulse Consumer                                                  │
│       │                                                           │
│       ├──► Trigger classification job                             │
│       ├──► Generate thumbnail                                     │
│       ├──► Notify connected clients (SSE)                         │
│       └──► Replicate to APN peers (NATS)                          │
│                                                                   │
│   Sovereign Stack Scraper (Dropbox → E:)                          │
│       │                                                           │
│       ├──► Copy file to sovereign stack                           │
│       ├──► INSERT into cloud_files                                │
│       └──► Publish pulse event: "file.synced"                     │
│                │                                                  │
│                ▼                                                  │
│   Dashboard auto-refreshes via SSE                                │
└──────────────────────────────────────────────────────────────────┘
```

**Tasks:**
- [ ] Add pulse event publishing to sovereign_stack.rs scraper (on new file copy)
- [ ] Add pulse event publishing to upload handlers (org_cloud.rs, data_sources.rs)
- [ ] Create SSE endpoint for file change notifications
- [ ] Frontend: subscribe to file events, auto-refresh queries
- [ ] APN replication: sovereign_storage.rs publishes cloud_files changes over NATS

### 2E. Search & Discovery

Full-text search across all org data.

```
   Search: "prime hospitality proposal"
              │
              ▼
   ┌─────────────────────────────────────────────┐
   │  SEARCH RESULTS                              │
   │                                              │
   │  📄 Prime Hospitality - Proposal v3.docx    │
   │     Sirak Studios/PROPOSALS/ · 2.4 MB       │
   │     "...comprehensive hospitality brand..."  │
   │                                              │
   │  📊 Prime Hospitality Budget.xlsx            │
   │     Sirak Studios/PROJECTS/Prime/ · 156 KB   │
   │                                              │
   │  📄 PH Meeting Notes 2026-02.md             │
   │     Personal/Meeting Recordings/ · 12 KB     │
   │     "...discussed prime hospitality Q2..."   │
   └─────────────────────────────────────────────┘
```

**Tasks:**
- [ ] Add SQLite FTS5 virtual table over data_sources (title, content, folder, tags)
- [ ] Backend: `/api/search` endpoint with relevance ranking
- [ ] Frontend: global search (Cmd+K) that searches files across all intelligence
- [ ] Index extracted text content from PDFs and docs
- [ ] NORA: natural language queries ("find all proposals from January")

---

## 3. Priority Order

```
NOW (Sprint 1 — this week)
├── ✅ Sovereign Stack scraper running
├── ✅ APN Cloud in File Explorer
├── ✅ Data separation (org vs personal)
├── ✅ Intelligence page in My Workspace
├── ✅ Cloud Browser with previews
│
NEXT (Sprint 2 — next week)
├── 🔲 Re-classify 3,259 octet-stream files
├── 🔲 Thumbnail generation pipeline
├── 🔲 Intelligence dashboard stat cards
├── 🔲 Pulse events on file upload/sync
│
THEN (Sprint 3)
├── 🔲 Full-text search (FTS5)
├── 🔲 Meeting recordings sub-view
├── 🔲 Key Docs pinning
├── 🔲 SSE live updates for file changes
│
FUTURE (Sprint 4+)
├── 🔲 NORA AI tagging & summarization
├── 🔲 Personal insights tab
├── 🔲 Project ↔ folder linking
├── 🔲 Cross-node file requests via APN
└── 🔲 Natural language search
```

---

## 4. Technical Decisions

| Decision | Rationale |
|----------|-----------|
| Sovereign stack on E: drive, not C: | C: has <500MB free; E: has 21TB |
| One-way Dropbox → Sovereign copy | Dropbox remains source of truth; sovereign is org-owned mirror |
| Personal data excluded from org views | Privacy: personal files belong to user, not organization |
| cloud_files as index, not data_sources | cloud_files has 45K+ entries from volume scans; data_sources is curated subset |
| SQLite FTS5 for search (planned) | Already using SQLite; FTS5 is fast, zero-dependency |
| Pulse Engine for file events | Existing pub/sub infrastructure; avoids polling |
| PIL-generated ICO icon | No ImageMagick needed; Python already available on machine |

---

## 5. Files Reference

| File | Role |
|------|------|
| `crates/server/src/sovereign_stack.rs` | Dropbox → sovereign stack scraper |
| `crates/server/src/org_cloud_indexer.rs` | Volume scanner, DB indexer |
| `crates/server/src/routes/org_cloud.rs` | Cloud file API (browse, download, upload) |
| `crates/server/src/routes/data_sources.rs` | Data source CRUD + file serving |
| `crates/server/src/sovereign_storage.rs` | APN NATS replication |
| `crates/server/src/pulse_consumer.rs` | Pulse Engine event consumer |
| `frontend/src/pages/intelligence.tsx` | Personal Intelligence page |
| `frontend/src/pages/data-sources.tsx` | Org Data Sources page |
| `frontend/src/pages/organization-profile/tabs/cloud/CloudBrowser.tsx` | Cloud file browser |
| `frontend/src/pages/organization-profile/tabs/intelligence/` | Org Intelligence tab |
| `frontend/src/components/layout/sidebar/constants.ts` | Sidebar nav items |
| `scripts/register_sovereign_stack.ps1` | APN Cloud Explorer registration |
| `.env` | Sovereign stack config vars |
