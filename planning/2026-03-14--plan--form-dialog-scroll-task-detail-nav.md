# Scrollable Form Dialogs + Task Detail Panel Navigation

**Date:** 2026-03-14
**Branch:** `qa/dogfood-pipeline-e2e-2026-03-13` (or new feature branch)
**Status:** Planning
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

---

## Part 1: FormDialogBody Helper Component

### Problem

The codebase has two scroll/footer patterns for dialogs:
- **Bad (TaskFormDialog, FeedbackDialog, ProjectFormDialog):** `overflow-y-auto` on DialogContent, footer scrolls with content — buttons disappear off-screen
- **Good (TaskTemplateEditDialog):** `flex flex-col` on DialogContent, `ScrollArea` as `flex-1`, footer outside scroll — buttons always visible

### Solution

Create a `FormDialogBody` helper that enforces the good pattern and standardizes it across all form dialogs.

### Step 1: Create `frontend/src/components/ui/form-dialog-body.tsx`

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

**Consumer requirement:** Must set `flex flex-col overflow-hidden` on `DialogContent` (overrides default `grid gap-4`).

### Step 2: Apply to TaskFormDialog (worst offender)

**File:** `frontend/src/components/dialogs/tasks/TaskFormDialog.tsx`

1. **Line 787:** Change className from `"sm:max-w-[650px] max-h-[90vh] overflow-y-auto"` to `"sm:max-w-[650px] max-h-[90vh] flex flex-col overflow-hidden"`
2. Wrap form fields (lines 808-1159) as `FormDialogBody` children
3. Move button row (lines 1161-1200) into `footer` prop
4. No changes to form field content

### Step 3: Apply to FeedbackDialog

**File:** `frontend/src/components/dialogs/feedback/FeedbackDialog.tsx` (line 187)

Same treatment: `overflow-y-auto` → `flex flex-col overflow-hidden`, wrap body in `FormDialogBody`, move Submit/Cancel to `footer`.

### Step 4: Apply to ProjectFormDialog

**File:** `frontend/src/components/dialogs/projects/ProjectFormDialog.tsx` (line 371)

Currently has `overflow-x-hidden` with no max-height. Add `max-h-[90vh] flex flex-col overflow-hidden`, wrap body in `FormDialogBody`.

### Step 5: Refactor TaskTemplateEditDialog for consistency

**File:** `frontend/src/components/dialogs/tasks/TaskTemplateEditDialog.tsx` (line 151)

Already works correctly (has manual ScrollArea + DialogFooter outside scroll). Refactor to use `FormDialogBody` for consistency across all form dialogs.

### Dialogs that need NO changes (already correct):
- `ProjectMembersDialog` — footer outside scroll area ✓
- `ClientMembersDialog` — footer outside scroll area ✓
- `ControllerSettingsDialog` — compact, no scroll needed ✓
- `FolderPickerDialog` — custom flex layout with fixed footer ✓
- `DealConvertDialog` — only template preview scrolls ✓

---

## Part 2: Task Detail Fullscreen Breadcrumb Navigation

### Key Finding

`BreadcrumbNav` already exists at `frontend/src/components/breadcrumb/BreadcrumbNav.tsx` and builds the complete navigation path: Home > Org > Project > Tasks > Task Title (truncated to 50 chars). It includes org switcher, clickable links, and loading skeleton. But it's **hidden in fullscreen** because `AppShell.tsx:66` sets `showNavbar = !isFullscreen`.

### Step 6: Render BreadcrumbNav inside fullscreen task detail

**File:** `frontend/src/components/tasks/TaskDetails/EnhancedTaskDetailsPanel.tsx`

When `isFullscreen` is true, render `<BreadcrumbNav />` at the top of the panel (before `EnhancedTaskHeader`). This reuses the existing component with all its features.

```tsx
{isFullscreen && <BreadcrumbNav />}
<EnhancedTaskHeader ... />
```

### Step 7: Make fullscreen toggle more visible

**File:** `frontend/src/components/tasks/TaskDetails/EnhancedTaskHeader.tsx`

When `isFullscreen`:
- Change fullscreen toggle from icon-only to labeled button: `[Minimize2] Exit Fullscreen`
- Add keyboard shortcut hints to tooltips: `"Exit fullscreen (f)"`, `"Close (Esc)"`

When NOT fullscreen (side panel):
- Keep existing icon-only buttons (no change — space is limited)

### Step 8: Same for non-enhanced header

**File:** `frontend/src/components/tasks/TaskDetailsHeader.tsx`

Same treatment: render `BreadcrumbNav` when fullscreen, make fullscreen toggle labeled.

---

## Files Modified

| # | File | Change |
|---|------|--------|
| 1 | `frontend/src/components/ui/form-dialog-body.tsx` | **NEW** — FormDialogBody helper (~30 lines) |
| 2 | `frontend/src/components/dialogs/tasks/TaskFormDialog.tsx` | Restructure layout to use FormDialogBody |
| 3 | `frontend/src/components/dialogs/feedback/FeedbackDialog.tsx` | Restructure layout to use FormDialogBody |
| 4 | `frontend/src/components/dialogs/projects/ProjectFormDialog.tsx` | Restructure layout to use FormDialogBody |
| 5 | `frontend/src/components/dialogs/tasks/TaskTemplateEditDialog.tsx` | Refactor to use FormDialogBody |
| 6 | `frontend/src/components/tasks/TaskDetails/EnhancedTaskDetailsPanel.tsx` | Add BreadcrumbNav when fullscreen |
| 7 | `frontend/src/components/tasks/TaskDetails/EnhancedTaskHeader.tsx` | Labeled fullscreen toggle + shortcut hints |
| 8 | `frontend/src/components/tasks/TaskDetailsHeader.tsx` | BreadcrumbNav + labeled toggle |

### Existing Components Reused
- `BreadcrumbNav` — `frontend/src/components/breadcrumb/BreadcrumbNav.tsx` (full path builder with org switcher)
- `ScrollArea` — `frontend/src/components/ui/scroll-area.tsx` (Radix-based)
- `DialogFooter` pattern — `frontend/src/components/ui/dialog.tsx` (flex layout)

---

## Verification

1. `cd frontend && npx tsc --noEmit` — no TS errors
2. Open Edit Task dialog → scroll form → verify footer stays visible with tinted bg
3. Open Feedback dialog → same check
4. Open Project Form dialog → same check
5. Click task → fullscreen → verify breadcrumb visible at top with correct path
6. Click breadcrumb org/project/Tasks links → verify navigation back works
7. Press `f` → toggles fullscreen with breadcrumb appearing/disappearing; `Esc` → returns
8. Verify via Playwright MCP in browser
