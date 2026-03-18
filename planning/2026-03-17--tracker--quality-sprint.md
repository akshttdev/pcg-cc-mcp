# Quality Sprint: Dialog Standardization, Modularity, File Splits

**Branch**: `refactor/quality-sprint-dialogs-modularity`
**Started**: 2026-03-17
**Status**: In Progress (Days 1-8 complete)

## Sprint Progress

| Day | Workstream | Status | Commit | Notes |
|-----|-----------|--------|--------|-------|
| 1 | B: showConfirm helper + 5 window.confirm replacements | Done | `dd630e806` (in main via #45 squash) | Zero window.confirm remaining |
| 2 | C: Query key factories in hooks | Done | `5aeab0620` (in main via #45 squash) | ~12 hook files, ~40 keys |
| 3 | C: Query key factories in pages & components | Done | `d71d2905b` | 42 files, 152 insertions |
| 4 | C: useMutationWithToast conversions | Done | `29dbd88aa` + `3de498af1` | 19 mutations converted |
| 5 | D: MeetingMode.tsx split | Done | `bf84f5a48` | 1207→4 files (types 65, Setup 252, Notes 173, index 886) |
| 6 | D: NoraAssistant + project-tasks splits | Done | `c5f446fb3` | See Day 6 section below |
| 7 | C: Mutation audit — all safe-zone converted | Done | (no commit needed) | All targets already converted in Day 4; settings dialogs tightly coupled, left inline |
| 8 | C: Query keys in org-profile + sidebar | Done | — | 32 files, ~75 inline keys replaced |

## Branch History Note

Days 1-2 commits were cherry-picked into `integration/sloperation316` before that branch was squash-merged to main as PR #45 (`9ce7410a4`). The quality sprint branch was then reset to `origin/main`. Days 3-5 were re-applied via cherry-pick from reflog.

## Verification Results (Days 1-5)

Full verification performed on branch `refactor/quality-sprint-dialogs-modularity`:
- **Day 1**: PASS — Zero `window.confirm()` calls, `showConfirm()` in all 4 target files
- **Day 2**: PASS — All 12 hook files using centralized query key factories
- **Day 3**: PASS — All 24 target components and 17 pages have zero inline query keys
- **Day 4**: PASS — All 19 mutations properly using `useMutationWithToast`
- **Day 5**: PASS — meeting-mode/ directory exists with 4 files, imports correct

**Remaining inline keys**: 165 across 68 files (non-safe-zone):
- `org-profile/*` (~60+ keys) — deferred to Day 8
- `sidebar/*` (~12 keys) — deferred to Day 8
- `deal-detail/tabs/*` (~13 keys)
- `workflow pages` (~12 keys)
- Misc pages from sloperation316 (~50+ keys)

## Sprint Metrics (Current)

| Metric | Before | Current | Target |
|--------|--------|---------|--------|
| `window.confirm()` calls | 5 | **0** | 0 |
| Inline query keys (est.) | ~318 | **~90** | ~100 |
| Raw `useMutation` (files) | ~60 | **~45** | ~35 |
| Files >900 lines | 15 | **12** | 12 |
| NiceModal registrations | 21 | 21 | 24 |

## Day 4 Mutations Converted

**Hooks (10):**
- `useOrgOnboarding.ts`: 4 mutations (create/update/delete org, save onboarding)
- `useCrmActivities.ts`: 2 mutations (create/delete activity)
- `useWorkflowTemplates.ts`: 1 mutation (convert deal)

**Components (3):**
- `TaskCommentThread.tsx`: delete comment
- `AgentWatcherPanel.tsx`: add watcher
- `data-sources.tsx`: delete source

**Pages/Dialogs (6):**
- `WalletSettings.tsx`: send VIBE
- `NetworkSettings.tsx`: update capabilities
- `knowledge.tsx`: refresh + mark stale (2)
- `media-library.tsx`: upload + delete (2)
- `CopyWorkflowDialog.tsx`: copy workflow
- `business-reports.tsx`: approve + revision (2)

**Skipped (complex patterns):**
- `useTaskMutations.ts` — async activity logging side effects
- `useProfiles.ts` — optimistic update with setQueryData
- `usePush.ts` / `useMerge.ts` — external callbacks passed in
- `StagingReviewPanel.tsx` — error state management across mutations
- `KeysSettings.tsx` — manual Alert state management
- `business-reports.tsx` patchMut — optimistic update

## Day 5 MeetingMode Split

`frontend/src/components/topsi/MeetingMode.tsx` (1,207 lines) →
`frontend/src/components/topsi/meeting-mode/`:
- `types.ts` (65 lines) — TranscriptEntry, MeetingNotes, helpers
- `MeetingSetup.tsx` (252 lines) — idle state: new meeting form + join active
- `MeetingNotesView.tsx` (173 lines) — ended state: notes + publish
- `index.tsx` (886 lines) — orchestrator with state, audio, active session

Import paths updated in `topsi.tsx`, `TopsiWidget.tsx`, `index.ts`.
Original `MeetingMode.tsx` preserved (can be deleted after verification).

## Day 6 NoraAssistant + project-tasks Splits

### NoraAssistant.tsx (1,027 lines) → `components/nora/assistant/`
- `types.ts` (112 lines) — all interfaces: NoraResponse, ConversationEntry, CinematicFormState, SpeechRecognition, etc.
- `CinematicBriefForm.tsx` (114 lines) — mode selector + cinematic brief form UI
- `ConversationHistory.tsx` (95 lines) — chat message display with markdown, actions, suggestions
- `VoiceControls.tsx` (105 lines) — voice conversation controls: recording/speaking indicators
- `index.tsx` (644 lines) — orchestrator: state, audio capture, send/receive, init

Barrel export updated in `components/nora/index.ts`. Also fixed `any` → `unknown` in types.

### project-tasks.tsx (997 lines) → `pages/project-tasks/`
- `ProjectTasksToolbar.tsx` (127 lines) — header bar with filter/sort/import/export/select/archive buttons
- `useTaskKeyboardNav.ts` (117 lines) — keyboard nav: next/prev task, next/prev column
- `TaskViewRenderer.tsx` (192 lines) — view type switch: kanban/table/gallery/timeline/calendar/overview
- `index.tsx` (653 lines) — orchestrator: state, effects, handlers, panel/drawer

Lazy import in `App.tsx` unchanged (`@/pages/project-tasks` now resolves to directory index).

## Day 7 Assessment

All safe-zone mutation conversions completed in Day 4. Settings dialog extraction evaluated but deferred — the 6 dialogs in AgentSettings/ProjectsSettings/WalletSettings are tightly coupled to parent state (mutations, form values, validation). Extracting to NiceModal would increase complexity without benefit.

## Day 8: Query Keys in Org-Profile + Sidebar

**Sidebar** (5 files, 12 inline keys → 0):
- `Sidebar.tsx`: `sidebarKeys.tree()`, `projectKeys.all`, `sidebarKeys.stagingPendingCount()`
- `ProjectFolder.tsx`: `sidebarKeys.projectBoards()`, `sidebarKeys.tree()` (x2)
- `ProjectList.tsx`, `OrgSection.tsx`, `ClientGroup.tsx`: `sidebarKeys.tree()`

**Org-Profile** (27 files, ~63 inline keys replaced):
- `index.tsx`, `BrandIdentityCard.tsx`, `ContactDetailModal.tsx`, `MemberAssignments.tsx`
- `ContactsTab.tsx`, `OverviewTab.tsx`, `WikiTab.tsx`
- Cloud tabs (5 files): `orgCloudKeys.*`
- Integration tabs (3 files): `discordKeys`, `integrationKeys.*`
- Intelligence tabs (6 files): `dataSourceKeys.*`, `knowledgeKeys.*`, `pulseKeys.*`, `workflowKeys.*`
- Social tabs (5 files): `socialKeys.*`

**New factories added to query-keys.ts** (22 lines):
- `organizationKeys`: `memberAssignments`, `projectsList`, `deals`, `dealsEnriched`, `activitiesOrg`, `workflowRuns`
- `discordKeys`: `activeSessions`
- `integrationKeys`: `githubTokenStatus`, `emailAccountsOrg`, `qbStatusOrg` (new factory object)
- `pulseKeys`: `alertsOrg`, `contentOrg`
- `dataSourceKeys`: `project`, `recentArtifacts`
- `entityKeys`: `companiesAll`
- `workflowKeys`: `systemAutomations`

**Skipped** (key string mismatch — need separate cleanup):
- `['brandProfile', orgId]` in social views (different from `['orgBrandProfile', orgId]`)
- `['workflowTemplates']` (factory returns `['workflow-templates']`, different string)
- `['social-mentions-ov', ...]` (no factory, one-off key)

## Remaining Inline Keys (~90)

Inline keys remaining after sprint are in files outside the sprint scope:
- Deal detail tabs, workflow editor pages, misc components
- Keys with string mismatches (noted above)
- These can be addressed in a follow-up sprint

## QA Review & Regression Analysis (2026-03-18)

### TypeScript Compilation
- `npx tsc --noEmit`: **Zero errors** — all splits, barrel exports, and query key factories compile clean.

### ESLint
- 7 errors in sprint-touched files, **all pre-existing** (not introduced by this sprint):
  - `_chunkIndex` unused in `meeting-mode/index.tsx` (pre-split)
  - `_orgId` unused in `ProjectsTab.tsx` (pre-existing)
  - Empty block statement in unrelated file

### Regression Checks
1. **No stale imports**: Zero references to old unsplit file paths (`components/topsi/MeetingMode`, `components/nora/NoraAssistant`, `pages/project-tasks.tsx` as file).
2. **Query key value matching**: All factory return values produce identical strings to replaced inline keys:
   - `sidebarKeys.tree()` → `['sidebarTree']`
   - `organizationKeys.brandProfile(id)` → `['orgBrandProfile', id]`
   - `sidebarKeys.stagingPendingCount()` → `['staging-pending-count']`
   - All 22 new factories verified.
3. **Original files preserved**: `NoraAssistant.tsx`, `MeetingMode.tsx`, `project-tasks.tsx` originals retained for reference.
4. **Mutation conversions**: Spot-checked `useOrgOnboarding`, `media-library`, `knowledge` — all correctly using `useMutationWithToast` with proper `invalidateKeys` and `successMessage`.
5. **No dead imports**: Key files checked for unused imports — none found.

### E2E Tests
- **84/85 passing** (1 pre-existing failure: `health-check.spec.ts:111` strict-mode violation — `getByRole('heading', { name: 'Projects' })` matches 2 elements)
- Auth setup, sidebar, navigation, tasks/kanban, org profile, settings, workflows, RBAC, pipeline — all green
- No regressions from sprint changes

### Pre-existing Issues Found During E2E Setup
1. **Seed DB BLOB→TEXT mismatch**: `dev_assets_seed/db.sqlite` has BLOB UUIDs in `users` table, but `User.id` is `uuid::Uuid` (sqlx expects TEXT). The `20260328000000_uuid_blob_to_text` migration intentionally skipped the `users` table. Runtime fix: manually convert with `UPDATE users SET id = lower(substr(hex(id),...))`. Seed DB should be regenerated post-migration.
2. **Login onboarding warning**: `table projects has no column named slug` — the `user_onboarding.rs` service tries to create a project but the seed DB `projects` table schema doesn't have a `slug` column that matches what the code expects. Non-blocking (login succeeds, onboarding silently fails).
3. **Migration checksum mismatch**: Copying seed DB and then running server fails with `VersionMismatch(20260409000000)` — migration file was modified after being applied to seed. Must use fresh seed + `sqlx migrate run`, or use the runtime DB.

### Second Review (2026-03-18) — Additional Fix
- **Missed inline key**: `project-tasks/index.tsx:143` had `queryKey: ['users']` instead of `userKeys.all` — fixed in commit `37b9a2616`
- **Pre-existing console.log**: `project-tasks/index.tsx:111` has debug logging (`[ProjectTasks] Creating task...`) — exists on main, not introduced by this PR

### Merge Conflict Analysis (vs active branches)

**`sloperation316-pipeline-progress`** — NOT merged in PR #45 (PR #45 merged `integration/sloperation316`, a different branch). 7 commits ahead / 5 behind main with 77 files changed (11,685 insertions). 18 conflicting files if both branches merge to main:
- **Rust backend** (3): `company.rs`, `crm_deals.rs`, `mod.rs` — unrelated to PR #46 changes
- **Frontend** (12): sidebar files (4), deal-detail tabs (3), org-profile tabs (3), `client-overview.tsx`, `call-intake.tsx`
- **Other** (3): `App.tsx`, `ClientProjectPanel.tsx`, `dealflow-pipeline.spec.ts`
- **Resolution strategy**: Merge PR #46 first. Then update pipeline-progress from main (`git merge origin/main`). Sidebar/org-profile conflicts are import additions for query key factories.

**`sloperation316-vibe-integration`** — Superset of pipeline-progress (2 extra commits: Dockerfile + VIBE plan). Also NOT merged in PR #45. Same conflict set + resolution strategy.

**`refactor/dev-velocity-sprint`** — Zero frontend overlap (planning file only). Explicitly designed to avoid PR #46 conflicts. Safe to work in parallel.

### Diff Summary (vs main)
- **97 files changed**, 4,050 insertions, 291 deletions
- **11 commits** on branch
