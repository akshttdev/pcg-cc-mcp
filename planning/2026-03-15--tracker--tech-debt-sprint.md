# Tech Debt Sprint — 2026-03-15

**Branch:** `tech-debt/sprint-2026-03-15`
**Goal:** Reduce frontend + backend tech debt across error handling, code organization, CI, and observability.

---

## Day 1 — Foundation (DONE)

- [x] Worktree deadlock fix (shared HTTP client)
- [x] `PageErrorBoundary` component for per-section error isolation
- [x] Error boundaries added to all org-profile tabs

## Day 2-3 — Unwrap Elimination + CI (DONE)

- [x] 40 `.unwrap()` calls replaced with proper error handling across 8 files
- [x] CI pipeline: fmt, clippy, test, lint, types, audit (`.github/workflows/test.yml`)

## Day 4 — api.ts Split (DONE)

- [x] `frontend/src/lib/api.ts` (6,669 lines) split into 21 domain modules
- [x] Barrel re-exports for backward compatibility

## Day 5 — Org-Profile Split + Stretch (DONE)

### Part A: organization-profile.tsx Split (COMPLETE)

- [x] Created directory structure: `organization-profile/{components,tabs,tabs/{social,intelligence,integrations}}`
- [x] Extracted shared code: `types.ts`, `constants.ts`, `helpers.ts`
- [x] Extracted 7 shared components: `BrandIdentityCard`, `ContactCard`, `ContactDetailModal`, `IntegrationCard`, `MemberAssignments`, `ProjectRow`, `StatPill`
- [x] Extracted 5 simple tabs: `OverviewTab`, `PipelinesTab`, `ContactsTab`, `ProjectsTab`, `MembersTab`
- [x] Extracted social tab group (6 files): `SocialTab` shell + 5 sub-views
- [x] Extracted intelligence tab group (7 files): `IntelligenceTab` shell + 6 sub-views
- [x] Extracted integrations tab group (4 files): `IntegrationsTab` shell + 3 sections
- [x] Created `index.tsx` main shell with lazy-loaded tabs + Suspense + ErrorBoundary
- [x] App.tsx import resolves automatically (directory index)
- [x] Deleted original `organization-profile.tsx` (7,086 lines → 33 files)
- [x] `npx tsc --noEmit` passes (0 new errors)

### Part B: Stretch Goals (COMPLETE)

- [x] **B1.** Structured logging: 3x `eprintln!` → `tracing::debug!()` in `crates/nora/src/brain/mod.rs`, 1x → `tracing::warn!()` in `crates/services/src/services/editron/mod.rs`
- [x] **B2.** Lazy-load tabs: `React.lazy()` + `<Suspense fallback={<TabSkeleton />}>` + `<PageErrorBoundary>` per tab in `index.tsx`
- [ ] **B3.** Query monitoring: `sqlx=debug` in default `RUST_LOG` — **skipped** (requires flox manifest change, risk of log noise in dev)
- [x] **B4.** Type fix: `Record<string, any>` → `Record<string, unknown>` in `frontend/src/components/notifications/utils.ts:45` (+ type guard for `.join()` call)

---

## Final Results

| Metric | Before | After |
|--------|--------|-------|
| `organization-profile.tsx` | 7,086 lines, 1 file | 33 files across directory tree |
| Largest file in module | — | `index.tsx` (~500 lines) |
| Tab lazy-loading | No | Yes (React.lazy + Suspense) |
| Per-tab error isolation | Yes (PageErrorBoundary) | Yes (maintained) |
| `eprintln!` in non-CLI code | 4 calls | 0 calls |
| `Record<string, any>` in utils.ts | 1 occurrence | 0 (→ `unknown`) |

## File Structure

```
frontend/src/pages/organization-profile/
├── index.tsx                    # Main shell + tab routing + brand dialog
├── constants.ts                 # BRAND_*, PLATFORM_*, STATUS_*, SENTIMENT_*, PRIORITY_*
├── helpers.ts                   # formatDate, formatCurrency, parseJsonArray, calendar helpers
├── types.ts                     # OrgMember, OrganizationProfilePageProps
├── components/
│   ├── BrandIdentityCard.tsx
│   ├── ContactCard.tsx
│   ├── ContactDetailModal.tsx
│   ├── IntegrationCard.tsx
│   ├── MemberAssignments.tsx
│   ├── ProjectRow.tsx
│   └── StatPill.tsx
├── tabs/
│   ├── OverviewTab.tsx
│   ├── PipelinesTab.tsx
│   ├── ContactsTab.tsx
│   ├── ProjectsTab.tsx
│   ├── MembersTab.tsx
│   ├── social/
│   │   ├── index.tsx            # SocialTab shell
│   │   ├── SocialOverviewView.tsx
│   │   ├── SocialAccountsView.tsx
│   │   ├── SocialContentView.tsx
│   │   ├── SocialInboxView.tsx
│   │   └── SocialAnalyticsView.tsx
│   ├── intelligence/
│   │   ├── index.tsx            # IntelligenceTab shell
│   │   ├── KnowledgeTab.tsx
│   │   ├── DataSourcesView.tsx
│   │   ├── ArtifactsView.tsx
│   │   ├── WorkflowsView.tsx
│   │   ├── PulseSection.tsx
│   │   └── TopologyView.tsx
│   └── integrations/
│       ├── index.tsx            # IntegrationsTab shell
│       ├── SocialSection.tsx
│       ├── CommunicationSection.tsx
│       └── DevelopmentSection.tsx
```
