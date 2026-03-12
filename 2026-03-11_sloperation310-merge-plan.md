# Merge Plan: sloperation310 + feature/fraze-2026-03-11 → main

**Created:** 2026-03-11
**Status:** In Progress — Deep Review Complete, Ready for Phase 1 Merge

---

## Merge Order

1. **sloperation310 → main** (clean merge, no git conflicts)
2. **feature/fraze-2026-03-11 → main** (2 file conflicts to resolve)

---

## Phase 1: sloperation310 → main

### Git Conflicts: NONE
The merge is clean — no file-level conflicts detected via `git merge-tree`.

### Migration Timestamp Conflicts (6) — ALREADY RESOLVED ON SLOPERATION

Sloperation310 has already renamed main's conflicting migrations to new timestamps. Dependency ordering is correct:

| Original Timestamp | main's migration | slop's migration | Slop's rename of main's |
|---|---|---|---|
| 20260304000000 | `seed_bugreports_project` | `meeting_person_links` | → `20260304300000` |
| 20260305200000 | `create_business_entities` | `org_invite_and_company_org` | → `20260305199000` |
| 20260313000000 | `workflow_output_staging` | `review_annotations_and_decisions` (no-op: `SELECT 1`) | → `20260313200000` |
| 20260314000000 | `member_management_overhaul` | `organization_brand_profile` | → `20260314050000` |
| 20260315000000 | `crm_org_scope_phase1` | `brand_guide_enhancements` | → `20260315050000` |
| 20260317000000 | `meeting_person_links` | `business_analytics_enhanced` | → `20260317050000` (no-op, column already exists) |

**Key dependency verified:** `org_invite_and_company_org` (ALTERs `persons`) runs AFTER `create_business_entities` (CREATEs `persons`). All other orderings are safe.

**Post-merge action:** Rebuild DB from scratch since migration checksums and ordering changed.

### Sidebar Width: w-72 Recommended
- sloperation310 keeps `w-64`; our branch also has `w-64`
- UI analysis recommends **w-72**: +32px reduces text truncation for project/client/org names by ~20%
- On 1920px: content area goes from 86.7% → 84.9% — imperceptible
- On 1440px: content goes from 84% → 80% — still very workable
- Sidebar collapses at <1024px so small screens unaffected
- **Decision:** Adopt w-72 in phase 2 when merging our branch

---

## Deep Review Findings: sloperation310

### 1. `intake.rs` (2,100 lines) — EXPERIMENTAL, Needs Hardening

**Purpose:** Multi-stage pipeline: ingest call transcripts/emails → Claude extraction → CRM matching → company research → business audit reports → auto-create deals/proposals.

**Critical Issues:**
- **NO AUTH on 8 of 10 endpoints.** Only `approve_business_report` and `request_revision` extract `AccessContext`. Anyone can POST to `/call-intake/email` or `/call-intake/upload` and trigger Claude API calls.
- **No webhook signature verification** on `email_webhook` — vulnerable to spam/abuse.
- **No transactions** anywhere. `approve_business_report` does report update + CRM deal stage move + proposal creation without transactional guarantees — partial failure = inconsistent state.
- **Race condition:** `org_id` from `resolve_org_and_assignee` is shadowed by `body.organization_id` in the spawned task. If payload has no `organization_id` but resolver computed one, spawned task gets `None`.
- **Hardcoded values:** org IDs (`SIRAK_STUDIOS_ORG`, `PCG_ORG`), email domain, Claude models (`claude-sonnet-4-20250514`, `claude-opus-4-6`), content limit (15k chars), max_tokens.
- **Org-scoping violations:** `list_intake` and `list_reports` return ALL items with no org filtering. `get_intake_item` / `get_report` don't verify caller access.
- **Panic risk:** `BusinessReport::find_by_id(...).await?.unwrap()` — crashes if report deleted between update and re-fetch.
- **No rate limiting** on Claude API calls — single intake item can trigger 5+ API calls.
- **Unused field:** `ExtractedIntake.opportunity_signals` extracted but never stored.

**Assessment:** Pipeline logic is well-structured but not production-ready. Merge as experimental, track hardening in backlog.

### 2. `marketplace.rs` (520 lines) — EXPERIMENTAL, Needs Hardening

**Purpose:** APN service marketplace — providers register listings, consumers subscribe with API keys, gateway routes requests via NATS/HTTP with VIBE billing.

**Critical Issues:**
- **SSRF risk:** `route_via_http` uses provider-supplied `endpoint_url` without validation. Malicious provider could point to internal services.
- **Auth extracted but never checked:** All protected endpoints bind `_access` (unused). Any authenticated user can CRUD any listing, manage any subscription.
- **No transactions** on billing: gateway marks request fulfilled → debits consumer → credits provider. If debit fails after fulfillment, inconsistent state.
- **f64 for financial calculations:** `vibe_cost * 0.85` and `(amount * 100_000_000.0) as i64` — floating-point is inappropriate for money.
- **NATS connection per request** (`async_nats::connect` on every gateway call) — expensive, should pool.
- **HTTP client per request** (`reqwest::Client::builder()` per call) — wastes TCP connections.
- **`project_id` scoping** in `SubQuery` and `list_subscriptions` — contradicts org-scoping architecture.
- **Hardcoded 85/15 fee split**, 30s timeout, 50-item default limit.

**Assessment:** Architecture is sound conceptually but needs security hardening, connection pooling, and financial precision fixes before production.

### 3. `App.tsx` (+402 lines) — STRUCTURAL CHANGES, Conflict Risk for Phase 2

**New routes added:**
- `/organizations/:orgId/data-sources` → `DataSourcesPage`
- `/organizations/:orgId/crm/acquisition` → org profile with acquisition pipeline
- `/organizations/:orgId/crm/lifecycle` → org profile with lifecycle pipeline
- `/discord` → `DiscordPage` (admin)
- `/organizations/:orgId/brand-guide` → `BrandGuidePage`
- `/review/:token` → public `ReviewPage`
- `/intake/:token` → public `BrandIntakePage`
- `/mcp-servers` → redirect to `/settings/mcp`

**Routes/imports removed:**
- `ClientOverview` lazy import (replaced by `DataSourcesPage`)
- `CompanyProfilePage` lazy import
- `OrganizationsSettings` lazy import
- `/companies/{id}/research` route

**Structural breaking changes:**
- **`OrganizationProvider` removed** from provider tree — if our branch uses `useOrganization()` context, it will crash
- **`AuthLayout` and `AppShell` removed** — replaced by inline `AppContent` component
- **`HomeRedirect` removed** — `/` now renders `<Projects />` directly

**Bug:** Duplicate `PeoplePage` lazy declaration (lines ~61 and ~63) — will cause build error or shadowing.

### 4. `intelligence.rs` (1,063 lines refactored) — Significant Simplification

**Changes:**
- Nora timeout reduced 90s → 60s
- `write_intelligence_results` simplified drastically: now only updates `persons.intelligence_*` fields + optional knowledge graph. **Removed:** social profile upsert, company find-or-create, CRM contact/deal creation, proposal creation, research task creation, business analysis generation.
- Direct research prompt rewritten for structured JSON output
- Company contact method handling simplified (removed social media keys)
- `POST /companies/{id}/research` endpoint **removed** from router

**New endpoints (dead code — defined but NOT registered in router):**
- `GET /api/persons/:id/research-passes`
- `POST /api/persons/:id/research-passes/next`
- `GET /api/persons/:id/reports`

**Org-scoping gap:** `ResearchRequest` and `NextPassRequest` use `project_id: Option<Uuid>`, no `organization_id` field.

### 5. CRM Deal Enhancements — COMPLIANT with Org-Scoping

**CrmDealCard.tsx** (235 lines): Rich Kanban card with intelligence status indicators, report review status chips, review task badges, AI summary preview, board asset progress bar, drag-and-drop. Well-built.

**CrmDealDetailPanel.tsx** (232 lines): Side-sheet with Details/Intel/Projects/Activity tabs. Intel tab shows research passes, AI summary, business report with approve/revision workflow. `projectId` prop used only as fallback for activity timeline, not for creating records.

**crm_deal.rs:** `CrmDealWithContact` gains 15 new fields (person_id, report_id, intelligence fields, review status). New helper methods. Kanban data now correctly scoped per-pipeline.

**`CreateCrmDeal` and `CreateCrmContact` both use `organization_id` (required), no `project_id`.** ✅ Compliant.

### 6. `api.ts` — Additive Changes Only

New types: `OrgBrandProfile`, `ReviewComment`, `ReviewData`, `BusinessReportRecord` (5 new fields), `DataSourceRecord.folder`.
New APIs: `reportsApi.approve/requestRevision`, `authApi.getInviteInfo/register`, `mediaApi.*`, `intakeApi.*`, `reviewApi.*`, `apiClient` wrapper.
**No existing signatures changed.** Safe to merge.

### 7. Organization Profile — Minimal Change on sloperation310

**sloperation310 vs main:** Only 12 lines different — a **duplicate `pulseApi` import** (bug). Everything else identical.
**Our branch (HEAD) vs sloperation310:** +886 lines — complete CRM contact CRUD, ContactDetailModal, member management redesign, pathname routing, workflow runs on overview.
**HEAD is a strict functional superset** of sloperation310 for this file. No features lost by using our version.

### 8. Brand Guide System — Complete Feature

Lives inside org profile as `BrandIdentityCard` component. CRUD via `organizationsApi.getBrandProfile/upsertBrandProfile`. Stored in `organization_brand_profiles` table. Includes brand values, voice, archetype, competitors, photography notes. Company research triggers AI (Scout/Nora) for brand intelligence.

### 9. Business Reports — Complete Feature (853 lines)

Full reporting system: list view with status badges + detail view with person/company enrichment, threat assessment, brand positioning, markdown rendering, approve/revision workflow. Auto-refreshes every 30s. Backed by `intake.rs` pipeline.

---

## Phase 2: feature/fraze-2026-03-11 → main (after sloperation merged)

### Git Conflicts: 2 files

**1. `frontend/src/components/layout/sidebar.tsx`**
- **sloperation310 changes (6 items):**
  - Social nav: query params → dedicated `/organizations/:id/social` path
  - Client group header split: chevron for toggle, name as clickable `<Link>`
  - "Deliverables" → "Pipeline" with `?client=` scoping
  - Staging badge TODO comment
  - Sidebar width stays w-64
- **Our branch changes:**
  - Social nav refactor (similar direction — query params → pathname)
  - Client group header restructure
  - "Deliverables" → "Pipeline" link
  - Staging TODO comment
- **Resolution:** Changes are convergent. Take our branch as base, adopt w-72, verify social routing paths match sloperation's pattern, keep client link navigation from sloperation (clickable name separate from collapse toggle — better UX).

**2. `frontend/src/pages/organization-profile.tsx`**
- **sloperation310:** Only adds duplicate `pulseApi` import (bug)
- **Our branch:** Major refactor (CRM CRUD, ContactDetailModal, members, pathname routing)
- **Resolution:** Use our branch version entirely. Do NOT port the duplicate import. No functionality lost.

### App.tsx Conflict Risk
- sloperation310 removes `AppShell`, `AuthLayout`, `OrganizationProvider`, `HomeRedirect`
- If our branch imports any of these, we'll need to adapt to the new `AppContent` inline pattern
- **Action needed:** Check our branch's App.tsx against sloperation310's version before phase 2

### Migration Conflicts: NONE
Our branch adds no new migrations beyond what's on main. The `20260320000000_add_org_scoping_indexes.sql` is already on main.

### Architecture Alignment: VERIFIED
- CRM org-scoping: ✅ sloperation310 compliant
- `pulseApi`: Bug (duplicate import), not a real feature to preserve
- Pathname routing: Both branches moving same direction
- Member management: Our UI refactor is a superset of sloperation's DB migration

---

## Backlog: Issues to Track Post-Merge

### Critical (fix before any production use)
- [ ] `intake.rs`: Add auth middleware to all endpoints
- [ ] `intake.rs`: Add webhook signature verification
- [ ] `marketplace.rs`: SSRF protection on provider endpoint URLs
- [ ] `marketplace.rs`: Add authorization checks (ownership verification) to protected endpoints

### High Priority
- [ ] `intake.rs`: Wrap multi-write operations in transactions
- [ ] `marketplace.rs`: Wrap billing operations in transactions
- [ ] `marketplace.rs`: Replace f64 with integer base units for VIBE calculations
- [ ] `marketplace.rs`: Use shared NATS connection pool (not per-request)
- [ ] `marketplace.rs`: Use shared `reqwest::Client` (not per-request)
- [ ] `intake.rs`: Fix org_id race condition (shadowed variable in spawned task)
- [ ] `App.tsx`: Fix duplicate `PeoplePage` lazy import
- [ ] `intelligence.rs`: Wire up dead-code endpoints or remove them
- [ ] `marketplace.rs`: Change `project_id` scoping to `organization_id`

### Medium Priority
- [ ] `intake.rs`: Make org IDs, Claude models, content limits configurable
- [ ] `intake.rs`: Org-scope `list_intake` and `list_reports` endpoints
- [ ] `intake.rs`: Replace `.unwrap()` with proper error handling
- [ ] `intake.rs`: Add rate limiting for Claude API calls
- [ ] `intelligence.rs`: Add `organization_id` to `ResearchRequest`
- [ ] Consider restoring intelligence status indicators on contact cards (commented out in our branch)
- [ ] Add "View full profile" link in ContactDetailModal for linked person records

### Low Priority
- [ ] `intake.rs`: Remove unused `opportunity_signals` field
- [ ] `marketplace.rs`: Make fee split, timeouts, limits configurable
- [ ] `marketplace.rs`: Use shared `reqwest::Client` for HTTP routing

---

## Phase 2 Resolution Log

### App.tsx — Restored Our Branch Version
**Problem:** Merge created a Frankenstein file with TWO complete routing systems (sloperation's inline `AppContent` + our `App` with `AppShell`/`AuthLayout`/`OrganizationProvider`/`HomeRedirect`). JSX mismatch (`<Routes>` opened, `</SentryRoutes>` closed), duplicate lazy declarations (`PeoplePage` x2, `DataSourcesPage` x2, `DiscordPage` x2), missing imports.

**Resolution:** Restored our original App.tsx. Added 2 new routes from sloperation:
- `/organizations/:orgId/crm/acquisition` → OrganizationProfilePage with acquisition pipeline
- `/organizations/:orgId/crm/lifecycle` → OrganizationProfilePage with lifecycle pipeline

**Dead code from sloperation (preserved in files, not imported):**
- `AppShell.tsx`, `AuthLayout.tsx` — still exist, still used by our App.tsx
- Sloperation's `AppContent` inline layout pattern — NOT adopted. Contained useful features that should be ported later:
  - Onboarding flow (disclaimer → onboarding → GitHub login → telemetry → release notes)
  - Responsive mobile sidebar with backdrop
  - `BreadcrumbNav` in app shell
  - `DevBanner`, `TopsiWidget`, `CommandPalette`, `KeyboardShortcutsOverlay`
  - `ErrorDisplay` component
  - These may already exist in our `AppShell` — verify before porting

### sidebar.tsx — Auto-merged (w-64 → w-72)
Sloperation's only change was sidebar width. Auto-merge applied it cleanly to our refactored version.

### organization-profile.tsx — Auto-merged (pulseApi import added)
Single `pulseApi` import added. Verified it's actively used (4 references). Our refactored version preserved intact.

---

## Post-Merge Checklist

### After Phase 1 (sloperation310 → main)
- [ ] Rebuild DB from scratch (delete `dev_assets/db.sqlite`, let flox re-seed, run migrations)
- [ ] `cargo sqlx prepare --workspace`
- [ ] `npm run generate-types`
- [ ] `pnpm build` — verify frontend compiles (watch for duplicate PeoplePage)
- [ ] `cargo build` — verify backend compiles
- [ ] Manual QA: login, sidebar, org profile, CRM pipeline, intelligence, business reports, intake

### After Phase 2 (feature/fraze → main)
- [ ] Rebuild DB again
- [ ] `cargo sqlx prepare --workspace`
- [ ] `npm run generate-types`
- [ ] Full build check (frontend + backend)
- [ ] Manual QA: sidebar (w-72), org profile contacts/members, CRM, workflows, pathname routing
- [ ] Verify login (admin/admin123)
