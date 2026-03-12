# Branch Merge Analysis: sloperation311 vs feature/fraze-2026-03-11

**Date:** 2026-03-11
**Status:** COMPLETED — merged as PR #15, squash-merged to main 2026-03-12
**Base branch:** main (8bee7e9)
**Common ancestor between branches:** d626ce17 (PR #14 merge — feature/2026-03-11-cleanup)

---

## Branch Summary

| | sloperation311 | feature/fraze-2026-03-11 |
|---|---|---|
| **Commits since main** | 50 | 42 |
| **Unique commits** (not on other branch) | 9 | 13 |
| **Files changed** | 193 | 141 |
| **Lines** | +10,804 / -1,690 | +5,798 / -628 |
| **Shared commits** | 29 (via PR #14 merge + earlier history) | 29 |

Both branches share a large common base through PR #14. The divergence is relatively contained.

---

## sloperation311 — Key Changes

### 1. New Backend Routes/APIs

#### Intake Pipeline (`crates/server/src/routes/intake.rs` — NEW, 2,102 lines)
- Full call transcript & email intake pipeline with 5 stages: Ingest, Extract, Associate, Research, Report
- Claude-powered extraction of participants, pain points, topics, action items, sentiment
- Fuzzy-matches extracted names/emails to `persons` table (creates new persons if no match)
- Auto-triggers intelligence research on matched persons
- Generates `business_audit` reports with executive summary, competitor analysis, recommendations
- Report approval workflow: `pending_review` → `approved`/`rejected`

#### APN Marketplace (`crates/server/src/routes/marketplace.rs` — NEW, 520 lines)
- Service marketplace for APN nodes with public and protected endpoints
- Gateway routing via NATS broadcast and direct HTTP proxy
- VIBE settlement: 85% provider / 15% platform fee
- API key authentication for consumer requests

#### Company Intelligence Research (`crates/server/src/routes/companies.rs` — +220 lines)
- `POST /companies/:id/research` triggers Scout agent via Nora
- Falls back to direct Anthropic API call if Nora is unavailable
- Stores intelligence summary, confidence, status on company record

#### Twilio SMS Thread Buffering (`crates/server/src/routes/twilio.rs` — major rework)
- Replaced synchronous SMS handling with debounced 8-second thread buffering
- Multiple rapid SMS messages collected into single thread before processing
- Nora processes combined thread; replies via outbound Twilio REST API

#### Topsi Meeting Enhancements (`crates/server/src/routes/topsi.rs` — +213 lines)
- `POST /topsi/meeting/notes/:session_id` — regenerate notes via Anthropic API
- Meeting list includes `segment_count` and project name lookups
- Project-access scoping helpers

#### Intelligence Routes (`crates/server/src/routes/intelligence.rs` — significant rework)
- Multi-pass research system: shallow → moderate → deep with focused areas
- Auto-generates business reports after research completes
- Creates CRM deals and proposals linked to reports
- New endpoints: `GET /persons/:id/research-passes`, `POST /persons/:id/research-passes/next`, `GET /persons/:id/reports`

### 2. New DB Models (6)
- `business_report` — analytics reports with approval workflow
- `call_intake_item` — intake pipeline records
- `person_research_pass` — iterative research tracking
- `marketplace_listing` — APN service listings
- `marketplace_subscription` — consumer subscriptions with API keys
- `gateway_request` — marketplace gateway request logging

### 3. Modified DB Models
- `crm_deal` (+207 lines) — intelligence fields: status, summary, confidence, research passes, report linkage, review task
- `proposal` — added `organization_id`
- `data_source` — added `folder` field
- `review_token` — source files support

### 4. Migrations (30 files, +394 / -335)
Key additions: intake pipeline tables, iterative research, business analytics, lead workflow review, tasks-CRM deal link, brand profile, brand guide enhancements

### 5. Frontend — CRM Enhancements
- **CrmDealCard.tsx** — major rework: intelligence status indicators, report review chips, task status badges. Shifted from "project management" card to "intelligence/lead pipeline" card
- **CrmDealDetailPanel.tsx** (+209 lines) — new Intel tab with research passes, AI profile summary, report approval workflow
- **crm.ts** (+14 lines) — `CrmDealWithContact` extended with all intelligence fields

### 6. Frontend — New Pages
- `/call-intake` (433 lines) — call intake dashboard
- `/business-reports` and `/business-reports/:id` (853 lines) — report listing + detail with approval
- `/persons/:personId` (373 lines) — person profile with research/reports tabs
- `/leads` (264 lines) — lead management cards

### 7. Frontend — App.tsx (massive rework)
- Added `ThemeProvider`, `SearchProvider`, `ShortcutsHelp`, `CommandPalette`, `KeyboardShortcutsOverlay`, `BreadcrumbNav`, `DevBanner`, `Toaster`, `WebviewContextMenu` to app shell
- Inlined `AppContent` component with onboarding flow
- `RoleRoute` guards on protected routes
- Removed `ClientOverview`, `OrganizationsSettings` lazy imports

### 8. Frontend — Sidebar
- Expanded sidebar widened from `w-64` to `w-72`
- New nav items: "Call Intake", "Reports" in Management section
- Role-based gating via `useEffectiveRole()`
- Collapsed org indicator (active org initial as avatar)
- Unified single ScrollArea

### 9. Frontend — Organization Profile
- "Knowledge" → "Intelligence" rename throughout
- Contacts tab: Companies sub-section toggle, intelligence status dots, research depth badges
- Overview tab: removed TODO comment about unified activity sources

### 10. Agent Enhancements
- **Nora** (+200 lines in tools.rs): real web search via Exa API, browser rendering via Playwright/Chromium, new `render_page` tool
- **Topsi** (+64 lines in agent.rs, +45 in tools/mod.rs): web search + page fetch via Exa API, system prompt updated to declare web access

### 11. API Client (`api.ts` — +247 lines)
- New namespaces: `reportsApi`, `intakeApi`, `authApi`
- Extended: `intelligenceApi` (research passes, person reports), `companiesApi` (org filter), `proposalsApi` (org filter)
- New types: `ResearchPass`, `BusinessReportRecord`, `OrgBrandProfile`
- **NOTE:** Duplicate `mediaApi`/`reviewApi` declarations at end of file (potential issue)

### 12. Infrastructure
- Fly.io deployment config (Dockerfile.fly, fly.toml)
- Tauri desktop app icons
- 3 commits of warnings cleanup across workspace
- APN zombie process prevention

---

## feature/fraze-2026-03-11 — Key Changes (Unique to Branch)

### 1. Workflow System Enhancements
- **WorkflowEditor.tsx** (+60 lines) — new `data_source` node type with `DataSourceNodeConfig` component for restricting data source types
- **WorkflowRunsPanel.tsx** (+36 lines) — resolves `data_source_id` to human-readable names via batch fetch
- **StagingReviewPanel.tsx** — null display fix: shows `""` instead of literal `"null"` for null field values
- **workflows.tsx** (+64 lines) — data source name resolution in detail panel, doc comment explaining page separation
- **data_source_workflows.rs** (+162 lines backend) — improved company name extraction regex (only "at" not "of/from/with"), false-positive filter for business terms, multi-type extraction mock generator for combined schemas

### 2. Organization Profile Overhaul (+1,135 net lines)
- **Tab navigation:** replaced `?tab=X` query params with path-based routing via `navigate()` and `TAB_PATHS` mapping
- **Overview tab:** quick links as `Link` components with summary stats; workflow runs in activity feed via `workflowsApi.listRecentRuns`
- **Contacts tab rewritten:** direct `crmApi.listContacts()` instead of `useOrgContacts` hook; sub-tab toggle between Contacts and Companies; inline Add Contact/Company forms
- **Members tab enhanced:** add member dropdown, role picker, inline role change, remove member, expandable member assignments, invite link dialog with copy-to-clipboard

### 3. Sidebar & Navigation
- **sidebar.tsx** (+75 unique lines) — social route fix (path-based `/organizations/:orgId/social` instead of `?tab=social`); client nav fix (chevron toggles collapse, name is a `Link` to client page)
- **BreadcrumbNav.tsx** (+125 lines) — comprehensive breadcrumb coverage for nearly all routes including CRM sub-pages, settings sub-pages, person/company detail, business reports
- **site-directory.tsx** — added Social under org pages, Business Reports to CRM section

### 4. Data Sources Page (+145 lines)
- "Run Workflow" action on data source rows for sources with `status === 'ready'`
- `RunWorkflowFromSourceDialog` — lists workflow definitions prioritizing those with `data_source` node type
- On success with staged records, navigates to `/workflows?tab=staging&run=<runId>`

### 5. Frontend Routing (App.tsx — +50 lines)
- Intelligence routes now pass `defaultTab="intelligence"` instead of `"knowledge"`
- `RoleRoute` guards applied to specific routes (shared with sloperation)
- 5 new route entries (shared pages)

### 6. Planning Documentation (unique)
- `planning/notes/APN-README.md` (186 lines) — APN architecture reference
- `planning/pr12-regressions.md` (109 lines) — PR #12 regression tracker
- `planning/workflow-e2e-test-findings.md` (194 lines) — E2E test findings

### 7. .gitignore
- Added `.claude/` and `test-screenshots/`

---

## Conflict Analysis

### Files Changed by Both Branches (Divergent)

Only **3 files** have actual divergent changes between the branches. Most shared files (120+) have identical changes from the common ancestor.

#### 1. `frontend/src/pages/organization-profile.tsx` — HIGH CONFLICT (1,352 divergent lines)
**sloperation311 version:**
- "Knowledge" → "Intelligence" rename
- Contacts tab has intelligence status dots, research depth badges
- Overview keeps TODO comment about unified activity

**fraze version (superset):**
- All of sloperation's renames PLUS path-based tab navigation
- Overview: quick links as Links with stats, workflow runs in activity feed, TODO removed
- Contacts: fully rewritten with direct CRM API, add contact/company forms
- Members: full CRUD with role management, invite links

**Verdict:** Fraze is a strict superset. Take fraze version entirely.

#### 2. `frontend/src/components/layout/sidebar.tsx` — LOW CONFLICT (130 divergent lines)
**sloperation311 version:**
- `OrgSocialSection` uses `?tab=social` query params
- `ClientGroup` entire header row is collapse trigger

**fraze version:**
- `OrgSocialSection` uses path-based `/social` routes (backwards-compatible with `?tab=social` fallback)
- `ClientGroup` split into chevron (collapse) + name (Link navigation)

**Verdict:** Fraze is a strict superset with better UX. Take fraze version.

#### 3. `crates/server/src/routes/data_source_workflows.rs` — MODERATE CONFLICT (214 divergent lines)
**sloperation311 version:**
- Base extraction logic (unchanged from common ancestor)

**fraze version:**
- Improved company name extraction regex (only "at" not "of/from/with")
- False-positive filter for business terms
- Multi-type extraction mock generator

**Verdict:** Fraze has all improvements. Take fraze version.

### Files Only on sloperation311 (64 files) — NO CONFLICTS
These are entirely new files with no counterpart on fraze:
- `crates/server/src/routes/intake.rs` (2,102 lines)
- `crates/server/src/routes/marketplace.rs` (520 lines)
- `crates/server/src/routes/intelligence.rs` (refactored)
- `crates/server/src/routes/twilio.rs` (reworked)
- `crates/server/src/routes/companies.rs` (extended)
- `crates/server/src/routes/topsi.rs` (extended)
- `crates/server/src/routes/crm_deals.rs`, `crm_pipelines.rs`
- 6 new DB models + 28 migration files
- `frontend/src/components/crm/CrmDealDetailPanel.tsx`
- `frontend/src/pages/topsi.tsx`, `people.tsx`
- `crates/nora/src/tools.rs` (Exa + Playwright)
- `crates/topsi/src/agent.rs`, `tools/mod.rs`
- Discord bot changes

### Files Only on fraze (11 files) — NO CONFLICTS
- Workflow components: `WorkflowEditor.tsx`, `WorkflowRunsPanel.tsx`, `StagingReviewPanel.tsx`
- `BreadcrumbNav.tsx`
- Pages: `data-sources.tsx`, `workflows.tsx`, `site-directory.tsx`
- Planning docs (3 files)
- `.gitignore`

---

## Merge Strategy Recommendations

### Recommended Approach: Merge sloperation311 into fraze

**Rationale:** Fraze is a strict superset of sloperation on all 3 conflicting files. Merging sloperation INTO fraze means:
- The 3 conflict files auto-resolve by keeping fraze's version
- All 64 sloperation-only files merge cleanly (no counterparts)
- All 11 fraze-only files are already present

### Step-by-Step Plan

```bash
# 1. Create a safety branch
git checkout feature/fraze-2026-03-11
git checkout -b merge/sloperation-into-fraze

# 2. Merge sloperation311
git merge origin/sloperation311

# 3. Resolve the 3 conflicts (all favor fraze/ours):
#    - organization-profile.tsx → keep ours (fraze)
#    - sidebar.tsx → keep ours (fraze)
#    - data_source_workflows.rs → keep ours (fraze)
# Can use: git checkout --ours <file> for each

# 4. Verify build
flox activate -- cargo check --workspace
cd frontend && pnpm tsc --noEmit

# 5. Verify no duplicate API declarations (sloperation has duped mediaApi/reviewApi in api.ts)
# Since api.ts changes are identical on both branches, this is already present — needs manual dedup

# 6. Test key flows:
#    - Workflow run from data source (fraze feature)
#    - CRM deal intelligence pipeline (sloperation feature)
#    - Organization profile tabs (fraze overhaul)
#    - Breadcrumb navigation (fraze feature)
```

### Post-Merge Cleanup Items — Resolution Status

1. **~~Deduplicate `api.ts`~~**: RESOLVED — no duplicates found; `mediaApi`/`reviewApi` already deduplicated.

2. **~~Migration ordering~~**: RESOLVED — all shared migrations are identical; sloperation-only migrations have no fraze counterpart. 5 duplicate migration pairs (20260314-15 vs 20260318-19) are safe — later ones are no-ops (`IF NOT EXISTS` or commented-out ALTERs), and all are tracked in `_sqlx_migrations`.

3. **~~`intake.rs` size~~**: RESOLVED — Split 2,102-line monolith into 4 sub-modules: `intake/mod.rs` (types, router, org routing), `intake/handlers.rs` (HTTP handlers), `intake/pipeline.rs` (pipeline orchestration, CRM association, knowledge graph), `intake/report.rs` (company research, report generation).

4. **~~SMS thread buffer~~**: RESOLVED — Added `# Single-Instance Constraint` doc section to `twilio.rs` header documenting the 3 `Lazy<Arc<Mutex<HashMap>>>` statics, their failure modes under multi-instance, and the migration path (Redis/NATS KV).

5. **~~`RoleRoute` guards~~**: RESOLVED — 14 routes properly guarded with correct `minRole` levels.

6. **~~`home_organization_id` column~~**: RESOLVED — added formal migration `20260322000000_add_home_organization_id.sql`. Previously required manual ALTER TABLE on fresh DBs.

7. **~~DOM nesting warning~~**: RESOLVED — fixed `<p>` inside `<p>` in `DisclaimerDialog.tsx` by replacing `DialogDescription` with styled `<div>`.

### Risk Assessment

| Risk | Level | Mitigation |
|---|---|---|
| Merge conflicts | **Low** | Only 3 files, all resolvable by taking fraze |
| Runtime regressions | **Medium** | org-profile is heavily reworked on fraze; sloperation adds CRM intelligence features that render in org-profile's Contacts tab. Manual test needed. |
| Migration conflicts | **Low** | Shared migrations are identical; sloperation-only migrations have no fraze counterpart |
| API type mismatches | **Low** | `api.ts` is identical on both branches; `crm.ts` differs only by 2 lines (fraze subset of sloperation) |
| Build failures | **Low** | New Rust modules (intake, marketplace) are self-contained; fraze's workflow backend changes are in a different file |
