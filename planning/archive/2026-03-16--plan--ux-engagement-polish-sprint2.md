# Sprint Plan: UX Engagement + Visual Polish (Sprint 2)

## Context

PR #38 shipped 11 UX polish items. This sprint targets the next tier: **org-scoped onboarding system**, **notification quick actions**, and **dark mode verification** — balanced with visual polish items (KPI trends, project card progress, workflow status, pipeline empty state).

**Branch:** `feature/ux-engagement-polish-sprint2`
**PR:** [#39](https://github.com/KingBodhi/pcg-cc-mcp/pull/39)
**Status:** COMPLETE (2026-03-16)
**Duration:** 5 days (1 week)
**Scope:** 12 items across ~20 files, ~400 lines of changes
**E2E Results:** 85/85 passing (including fix for Task.collaborators missing field bug)

## Key Design Decision: Org-Scoped Onboarding

**Original**: Onboarding was project-scoped (`project_onboarding` + `onboarding_segments` tables).
**New**: Onboarding is **organization-scoped** — one onboarding per org, shown on the org overview page.

**Rationale**: The segments (CRM, Intelligence, Integrations, etc.) are org-level concerns, not project-level. An org only needs to onboard once.

**Approach**: Option (b) — new `org_onboarding` + `org_onboarding_segments` tables alongside existing project tables. Existing project routes get deprecation comments. New org routes added in parallel.

**Two onboarding types** (user account onboarding deferred to future sprint):
- **Org Onboarding** (this sprint): 9-segment carousel on org overview — CRM, Intelligence, Integrations, etc.
- **User Account Onboarding** (future): First-login walkthrough — profile, preferences, Topsi intro.

## Sprint Schedule

| Day | Items | Focus |
|-----|-------|-------|
| 1 | 1A, 1B, 1C | Onboarding: org-scoped DB + model, API module, hook |
| 2 | 2A, 2B, 2C | Onboarding: org overview integration, dark mode; Notifications |
| 3 | 3A, 3B | Dark mode: known fixes + Playwright walkthrough |
| 4 | 4A, 4B, 4C | KPI trends, project card progress, workflow status |
| 5 | 5A, 5B, 5C, 5D | Pipeline empty state, settings cleanup, testing |

---

## Day 1: Org Onboarding — DB + Model + API + Hook

### 1A. New Org Onboarding DB Tables + Rust Model

**Migration** (`20260316000000_add_org_onboarding.sql`):
- `org_onboarding`: mirrors `project_onboarding` but with `organization_id` FK instead of `project_id`
- `org_onboarding_segments`: mirrors `onboarding_segments` but with `organization_id` FK

**Model** (`crates/db/src/models/org_onboarding.rs`):
- `OrgOnboarding` struct (mirrors `ProjectOnboarding` with `organization_id`)
- `OrgOnboardingSegment` struct (mirrors `OnboardingSegment` with `organization_id`)
- Reuses existing `OnboardingStatus`, `SegmentStatus`, `SegmentType` enums from `project_onboarding.rs`
- `create_with_segments()` creates 9 default segments (Research, Brand, Website, CRM, Email, Legal, Intelligence, Integrations, Social)

**Segment types already updated** in `project_onboarding.rs`:
- Added: `Crm`, `Intelligence`, `Integrations`
- Updated descriptions: Website, Email

**Backend routes** (`crates/server/src/routes/org_onboarding.rs`):
- `GET /api/onboarding/organization/{org_id}` — get org onboarding
- `POST /api/onboarding/organization/{org_id}/start` — start org onboarding
- `PUT /api/onboarding/org/{id}` — update org onboarding
- `GET /api/onboarding/org/{id}/segments` — list segments
- `PUT /api/onboarding/org-segment/{id}` — update segment
- `POST /api/onboarding/org-segment/{id}/start` — start segment
- `POST /api/onboarding/org-segment/{id}/complete` — complete segment

**Deprecation**: Add `// DEPRECATED: Use org_onboarding routes. Scheduled for removal.` comments to existing project onboarding routes.

### 1B. Frontend Onboarding API Module

**File**: `frontend/src/lib/api/onboarding.ts`
- `onboardingApi` object with both project (deprecated) and org endpoints
- Org methods: `getByOrg`, `startOrg`, `updateOrg`, `getOrgSegments`, etc.

### 1C. `useOrgOnboarding` Hook

**File**: `frontend/src/hooks/useOrgOnboarding.ts`
- `useQuery(['org-onboarding', orgId])` → `onboardingApi.getByOrg`
- Mutations: `startOnboarding`, `startSegment`, `completeSegment`, `skipSegment`

---

## Day 2: Org Overview Integration + Carousel Dark Mode + Notifications

### 2A. Wire Carousel into Org Overview Page

**File**: `frontend/src/pages/organization-profile/tabs/OverviewTab.tsx`
- Import `OnboardingCarousel` and `useOrgOnboarding`
- Between stats cards and quick links, render carousel when `status === 'active'`
- "Start Setup" button when no onboarding exists
- Completed state: compact "Setup Complete" badge with checkmark
- `onSegmentClick` navigates to relevant org feature area (CRM → pipeline, Intelligence → workflows, etc.)

### 2B. Onboarding Carousel Dark Mode Fixes (already done in 1A)

Colors updated with `dark:` variants in SEGMENT_CONFIG and STATUS_CONFIG.

### 2C. Notification Quick Actions

**InboxNotificationItem**: Source-aware action buttons (View Task, View Run, View)
**ActivityNotificationItem**: Status chip on status_change, View button
**NotificationCenter**: Improved deep-linking, dismiss handler

---

## Day 3: Dark Mode Verification

### 3A. Fix Known Hardcoded Colors

- `--brand-gold` CSS variable (light: `#b8952a`, dark: `#d4b44a`)
- LoginPage + Navbar: replace inline `color: '#b8952a'` with `var(--brand-gold)`

### 3B. Playwright Dark Mode Walkthrough

Screenshot each major page in dark mode, fix issues found.

---

## Day 4: Visual Polish

### 4A. Org Overview KPI Trend Indicators

Active/No data indicator below each KPI card number.

### 4B. Project Card Progress Bar

Thin progress bar based on completed/total tasks.

### 4C. Workflow Card Last-Run Status

"Last run: X ago" with status icon on workflow definition cards.

---

## Day 5: Pipeline Empty State + Settings + Testing

### 5A. Pipeline Empty State Onboarding Card

Replace "No deals" in first column with onboarding CTA when entire pipeline empty.

### 5B. Settings "Coming Soon" De-emphasis

Group planned items with divider, reduce opacity.

### 5C. Testing

Playwright walkthrough of all features.

### 5D. Code Quality

`tsc --noEmit`, lint, `generate-types`, `cargo check`.

---

## Deprecation Plan: Project Onboarding Removal (Future Sprint)

1. Remove frontend `onboardingApi` project methods
2. Remove `useProjectOnboarding` hook
3. Remove `/api/onboarding/project/*` routes from `crates/server/src/routes/onboarding.rs`
4. Keep DB tables (may repurpose for project-specific setup wizards)
5. Remove from `routes/mod.rs` router merge

---

## User Account Onboarding — Future Sprint Design Notes

**Scope**: First-login experience for new users.
**Potential steps**:
1. Set display name + avatar
2. Choose theme (light/dark)
3. Configure API keys (or skip)
4. Topsi introduction + first prompt
5. Join/create organization

**Implementation approach**: Separate `user_onboarding` table with step-based progress (not segment carousel). Render as full-page wizard on first login, gated by `user.onboarding_completed_at IS NULL`.

---

## Files Summary (~20 files)

| File | Change | Effort |
|------|--------|--------|
| `crates/db/migrations/20260316000000_add_org_onboarding.sql` | NEW — org onboarding tables | S |
| `crates/db/src/models/org_onboarding.rs` | NEW — org onboarding model | M |
| `crates/db/src/models/mod.rs` | Add org_onboarding module | XS |
| `crates/db/src/models/project_onboarding.rs` | Add Crm/Intelligence/Integrations segments | S (done) |
| `crates/server/src/routes/org_onboarding.rs` | NEW — org onboarding routes | M |
| `crates/server/src/routes/onboarding.rs` | Add deprecation comments | XS |
| `crates/server/src/routes/mod.rs` | Add org_onboarding module + merge | XS |
| `frontend/src/lib/api/onboarding.ts` | NEW — API client (project + org) | S (done) |
| `frontend/src/lib/api/index.ts` | Add onboarding export | XS (done) |
| `frontend/src/hooks/useOrgOnboarding.ts` | NEW — React Query hook for org | S |
| `frontend/src/components/onboarding/OnboardingCarousel.tsx` | New segments + dark mode | S (done) |
| `frontend/src/pages/organization-profile/tabs/OverviewTab.tsx` | Carousel + KPI trends | M |
| `frontend/src/components/notifications/InboxNotificationItem.tsx` | Quick actions | S |
| `frontend/src/components/notifications/ActivityNotificationItem.tsx` | Status chip | S |
| `frontend/src/components/notifications/NotificationCenter.tsx` | Deep-linking + dismiss | S |
| `frontend/src/styles/index.css` | Brand gold CSS variable | XS |
| `frontend/src/components/auth/LoginPage.tsx` | Use CSS variable | XS |
| `frontend/src/components/layout/navbar.tsx` | Use CSS variable | XS |
| `frontend/src/components/projects/ProjectCard.tsx` | Progress bar | S |
| `frontend/src/pages/workflows/tabs/BuilderTab.tsx` | Last-run status | S |
| `frontend/src/components/crm/CrmPipelineBoard.tsx` | Empty state card | S |
| `frontend/src/pages/settings/SettingsLayout.tsx` | De-emphasize planned | XS |
