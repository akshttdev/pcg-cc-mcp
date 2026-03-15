# Scrollable Form Dialogs + Task Detail Panel Navigation

**Date:** 2026-03-14
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13`
**Status:** Part 1 DONE, Part 2 DONE (evolved scope)
**Triggered by:** E2E QA testing findings (bugs #14-16 in `2026-03-13--review--dogfood-e2e-qa.md`)

---

## Context

During E2E QA testing of the dogfooding pipeline, several UX issues were identified:

1. **Edit Task modal** — long form where Cancel/Update buttons scroll off-screen, hard to find CTA
2. **No consistent pattern** — 33+ dialog files each reinvent scroll/footer strategy independently
3. **Task detail fullscreen** — breadcrumb nav hidden when fullscreen (`showNavbar = !isFullscreen` in AppShell.tsx:66), user loses kanban context with no obvious back navigation
4. User wants to keep fullscreen ability but with **breadcrumb nav visible** for orientation and return

### Decisions (from user input)
- **Back navigation:** Reuse existing `BreadcrumbNav` component (already builds Home > Org > Project > Tasks > Task Title)
- **Footer style:** Tinted background (`bg-muted/50` + `border-t`) for visual separation
- **Scope:** All broken dialogs — TaskFormDialog, FeedbackDialog, ProjectFormDialog + refactor TaskTemplateEditDialog for consistency
- **Task detail:** Evolved from fullscreen-only nav to a ResizableDrawer panel with expand/collapse

---

## Part 1: FormDialogBody Helper Component — DONE

**Commit:** `e72d80187`

### Step 1: Create `frontend/src/components/ui/form-dialog-body.tsx` — DONE

~30 line component. Enforces: fixed header, scrollable body, sticky footer.

```tsx
interface FormDialogBodyProps {
  children: React.ReactNode;       // Scrollable form fields
  footer: React.ReactNode;         // Action buttons (Cancel/Submit)
  className?: string;              // Extra classes on ScrollArea wrapper
  footerClassName?: string;        // Extra classes on footer div
}
```

Internal layout:
- `ScrollArea` with `flex-1 -mx-6 px-6` (body scrolls, scrollbar extends to dialog edge)
- Footer div with `border-t bg-muted/50 pt-4 -mx-6 px-6 pb-1` + standard `DialogFooter` flex layout

### Step 2: Apply to TaskFormDialog — DONE

Changed `DialogContent` from `overflow-y-auto` to `flex flex-col overflow-hidden`. Wrapped form fields in `FormDialogBody` children, moved buttons to `footer` prop.

### Step 3: Apply to FeedbackDialog — DONE

Same treatment: `overflow-y-auto` → `flex flex-col overflow-hidden`, wrapped in `FormDialogBody`.

### Step 4: Apply to ProjectFormDialog — DONE

Added `max-h-[90vh] flex flex-col overflow-hidden`, content div made scrollable with `overflow-y-auto flex-1`.

### Step 5: Refactor TaskTemplateEditDialog — DONE

Replaced manual `ScrollArea` + `DialogFooter` with `FormDialogBody` for consistency.

---

## Part 2: Task Detail Panel Navigation — DONE (scope evolved)

The original plan called for breadcrumb nav in fullscreen + labeled toggle buttons. Through user feedback, the scope evolved significantly:

### What was implemented (commits `761bdb609` through `fad6c9fd7`):

1. **Fullscreen toggle in BreadcrumbNav** (`761bdb609`)
   - Added Maximize2/Minimize2 toggle button to `BreadcrumbNav` on all pages
   - Task pages: URL-based fullscreen (`/full` suffix) via `useTaskViewManager`
   - All other pages: zustand store-based (`contentFullscreen`) via `useViewStore`
   - `AppShell.tsx`: `showNavbar = !isFullscreen && !contentFullscreen`, BreadcrumbNav always rendered

2. **ResizableDrawer component** (`72e527532`)
   - **NEW:** `frontend/src/components/ui/resizable-drawer.tsx`
   - Right-side drawer with drag-to-resize handle
   - Width persisted to localStorage, clamped to viewport minus sidebar minus 10px gap
   - Render children pattern exposing `{ isExpanded, toggleExpand }` via `DrawerRenderProps`
   - Used by `project-tasks.tsx` for task detail panel (replaces inline panel)

3. **Expand/collapse toggle** (`a080115e1`)
   - Replaced fullscreen toggle in `EnhancedTaskHeader` with expand/collapse
   - Expand fills screen to left edge (max width), collapse restores original size
   - `preExpandWidthRef` stores width before expand for restoration
   - Icons: `ArrowLeftFromLine` (expand) / `ArrowRightFromLine` (collapse)

4. **Click-outside to close** (`fad6c9fd7`)
   - Removed backdrop overlay so task cards behind drawer remain clickable
   - Added document `mousedown` listener with ref containment check
   - Skips close when dragging resize handle (`isDraggingRef`)

### Files modified in Part 2:

| File | Change |
|------|--------|
| `frontend/src/components/breadcrumb/BreadcrumbNav.tsx` | Added fullscreen toggle button (Maximize2/Minimize2), uses URL-based or store-based toggle |
| `frontend/src/stores/useViewStore.ts` | Added `contentFullscreen`, `toggleContentFullscreen`, `setContentFullscreen` |
| `frontend/src/layouts/AppShell.tsx` | `showNavbar` includes `contentFullscreen`, BreadcrumbNav always rendered |
| `frontend/src/components/ui/resizable-drawer.tsx` | **NEW** — Reusable resizable drawer with grip handle |
| `frontend/src/pages/project-tasks.tsx` | Uses `ResizableDrawer` for task detail panel |
| `frontend/src/components/tasks/TaskDetails/EnhancedTaskDetailsPanel.tsx` | Added `onToggleExpand`/`isExpanded` props, renders BreadcrumbNav when fullscreen |
| `frontend/src/components/tasks/TaskDetails/EnhancedTaskHeader.tsx` | Replaced fullscreen toggle with expand/collapse |
| `frontend/src/components/tasks/TaskDetailsHeader.tsx` | Added BreadcrumbNav when fullscreen, keyboard shortcut hints |
| `frontend/src/components/tasks/TaskDetailsPanel.tsx` | Added BreadcrumbNav when fullscreen |

---

## Verification

- [x] `cd frontend && npx tsc --noEmit` — no TS errors
- [ ] Open Edit Task dialog → scroll form → verify footer stays visible with tinted bg
- [ ] Open Feedback dialog → same check
- [ ] Open Project Form dialog → same check
- [ ] Click task → drawer opens on right with resize handle
- [ ] Drag resize handle → width changes, persists on reopen
- [ ] Click expand → drawer fills to sidebar edge; click collapse → restores original width
- [ ] Click task card behind drawer → drawer closes, card selectable
- [ ] Click outside drawer → closes
- [ ] Fullscreen via BreadcrumbNav → breadcrumb visible at top of fullscreen panel
- [ ] Press `Esc` → closes panel/exits fullscreen
