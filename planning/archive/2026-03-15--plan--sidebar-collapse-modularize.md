# Sidebar: Fix Collapse on Navigation + Modularize

**Branch:** `dev/2026-03-14`
**PR:** [#30](https://github.com/KingBodhi/pcg-cc-mcp/pull/30)
**Status:** COMPLETE — merged 2026-03-15 (PR #30)

---

## Problem

1. **Collapse bug**: Sidebar collapses the organization section on every page navigation — users lose their place in the sidebar tree
2. **Maintainability**: `sidebar.tsx` is 2173 lines with ~18 components in a single file

## Root Causes

All in `frontend/src/components/layout/sidebar.tsx`:

| Cause | Lines | Impact |
|-------|-------|--------|
| `isWorkspacePage` effect forces `orgContentExpanded = false` on workspace page navigation | 1221-1223 | Org collapses when visiting `/my-tasks`, `/projects`, `/workflows`, etc. |
| Section toggles use `useState` | 1583-1586 | Admin/Workspace/Management/Global Views sections reset on re-render |
| Org subsection toggles use `useState` | 1216-1217 | Internal Projects / Clients subsections reset |
| Project expansion forces single-select | 1563-1567 | Navigating to project B collapses project A |

## Part A: Fix Collapse Behavior

**Approach:** Replace all `useState` toggle state with `useExpandable` from `frontend/src/stores/useExpandableStore.ts` — a Zustand-backed keyed boolean store that survives route changes within the session. API-compatible with `useState<boolean>` and Radix `Collapsible.onOpenChange`.

### Changes

1. **Remove `isWorkspacePage` auto-collapse effect** — delete the `useEffect` at lines 1221-1223. Replace `useState(!isWorkspacePage)` with `useExpandable('sidebar:org:${org.id}', true)`. Keep `isWorkspacePage` prop only for active-highlight styling.

2. **Org subsections** — Replace `useState(true)` for `internalExpanded`/`clientsExpanded` with `useExpandable('sidebar:org:${org.id}:internal', true)` etc.

3. **Main section toggles** — Replace 4 `useState` calls with `useExpandable('sidebar:globalViews', false)` etc.

4. **Soften project auto-expand** — Change the `useEffect` to ADD the active project to the expanded set instead of replacing the entire set. Users can still manually collapse.

## Part B: Modularize

### Current components in sidebar.tsx (2173 lines)

| Line | Component | ~Lines |
|------|-----------|--------|
| 109 | `countProjects` / `isProjectInTree` | 20 |
| 133 | Nav constants (6 arrays + external links) | 62 |
| 198 | `HealthDot` | 18 |
| 218 | `OrgCrmSection` | 56 |
| 276 | `OrgSocialSection` | 64 |
| 342 | `OrgIntelligenceSection` | 55 |
| 399 | `CrmSidebarLinks` | 65 |
| 466 | `ProjectFolder` | 168 |
| 635 | `SortableSidebarProjectFolder` | 318 |
| 955 | `SortableProjectList` | 88 |
| 1045 | `ClientGroup` | 148 |
| 1195 | `OrgSection` | 219 |
| 1416 | `SidebarOrgGroups` | 99 |
| 1517 | `Sidebar` (export) | 656 |

### Target structure

```
frontend/src/components/layout/sidebar/
├── index.tsx              — Re-export Sidebar (backward compat)
├── Sidebar.tsx            — Main component (~300 lines)
├── constants.ts           — Nav arrays + external links (~90 lines)
├── helpers.ts             — countProjects, isProjectInTree, HealthDot (~40 lines)
├── OrgSection.tsx         — OrgSection + SidebarOrgGroups (~320 lines)
├── OrgWorkspaceLinks.tsx  — CRM/Social/Intel sections + CrmSidebarLinks (~240 lines)
├── ProjectFolder.tsx      — ProjectFolder + SortableSidebarProjectFolder (~490 lines)
├── ProjectList.tsx        — SortableProjectList (~90 lines)
└── ClientGroup.tsx        — ClientGroup (~150 lines)
```

### Extraction order (each step compiles independently)

1. `constants.ts` + `helpers.ts` — zero dependencies
2. `OrgWorkspaceLinks.tsx` — self-contained routing/icon components
3. `ProjectFolder.tsx` + `ProjectList.tsx` — largest components
4. `ClientGroup.tsx` — depends on ProjectList sibling import
5. `OrgSection.tsx` — imports from all siblings
6. `Sidebar.tsx` + `index.tsx` — remainder + re-export

### Backward compatibility

`import { Sidebar } from '@/components/layout/sidebar'` resolves to `sidebar/index.tsx` — no import changes needed in `AppShell.tsx` or any other consumer.

## Implementation Order

```
1. Apply Part A fixes in sidebar.tsx             — verify collapse behavior
2. Extract to sidebar/ directory (Part B)        — compile check per step
3. Final verification                            — full test pass
```

## Verification

- [x] `cd frontend && npx tsc --noEmit` — compiles
- [x] `cd frontend && npm run lint` — no new errors
- [ ] Navigate project → My Tasks → project — org stays expanded
- [ ] Manually collapse sections → navigate → they stay collapsed
- [ ] Page refresh → sections reset to defaults (session-only, expected)
- [x] Existing sidebar imports still resolve via index.tsx
- [x] Vercel preview deployed successfully

## PR Review Feedback (2026-03-15)

Owner review identified 3 should-fix + 2 nice-to-fix items. All addressed in `a74e820`:

| # | Issue | Resolution |
|---|-------|------------|
| 1 | `allOrgs` broke `useMemo` memoization in `SidebarOrgGroups` | Wrapped in its own `useMemo` keyed on stable refs |
| 2 | Duplicate `findInTree()` in `OrgSection` | Replaced with imported `isProjectInTree` from helpers |
| 3 | `ClientGroup` used `localStorage+useState` inconsistently | Migrated to `useExpandable` (Zustand) |
| 4 | `any` type in staging filter (`Sidebar.tsx`) | Removed — `WorkflowStagingRecord.status` already typed |
| 5 | DnD sensors recreated every render | No change — `useSensors` is a dnd-kit hook that memoizes internally |
