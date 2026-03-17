# Quality Sprint: Dialog Standardization, Modularity, File Splits

**Branch**: `refactor/quality-sprint-dialogs-modularity`
**Started**: 2026-03-17
**Status**: In Progress (Days 1-6 complete)

## Sprint Progress

| Day | Workstream | Status | Commit | Notes |
|-----|-----------|--------|--------|-------|
| 1 | B: showConfirm helper + 5 window.confirm replacements | Done | `dd630e806` (in main via #45 squash) | Zero window.confirm remaining |
| 2 | C: Query key factories in hooks | Done | `5aeab0620` (in main via #45 squash) | ~12 hook files, ~40 keys |
| 3 | C: Query key factories in pages & components | Done | `d71d2905b` | 42 files, 152 insertions |
| 4 | C: useMutationWithToast conversions | Done | `29dbd88aa` + `3de498af1` | 19 mutations converted |
| 5 | D: MeetingMode.tsx split | Done | `bf84f5a48` | 1207→4 files (types 65, Setup 252, Notes 173, index 886) |
| 6 | D: NoraAssistant + project-tasks splits | Done | — | See Day 6 section below |
| 7 | C+B: More mutations + settings dialog extractions | Pending | — | |
| 8 | C+B: Post-sloperation316 org-profile + showConfirm adoption | Available | — | sloperation316 merged (#45) |

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
| Inline query keys (est.) | ~318 | **~165** | ~100 |
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

## Remaining Work

### Day 7: More mutations + settings dialog NiceModal extractions
- Additional mutation conversions in safe-zone components
- Extract 3 inline dialogs from settings pages to NiceModal

### Day 8: Post-sloperation316 cleanup (NOW AVAILABLE)
- Query keys in org-profile + sidebar (sloperation316 merged)
- showConfirm adoption for remaining NiceModal.show('confirm') calls
- Cloud tab query key integration
