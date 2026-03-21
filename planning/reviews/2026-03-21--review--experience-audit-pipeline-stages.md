# Experience Audit Report — CRM Pipeline Stage Transitions

**Date**: 2026-03-21
**Scope**: Move a deal through all 8 CRM pipeline stages via the UI
**Branch**: `feature/2026-03-19--tier1-phase0`
**Viewport**: Desktop (1440×900)

## Executive Summary

**Users cannot move deals between pipeline stages.** The kanban board's drag-and-drop fires the DnD event (dnd-kit registers the drop) but the deal doesn't actually move — it stays in the original column. The deal detail view has a stage progress bar that is display-only (not clickable). The context menu only offers Edit and Delete. This is a **BLOCKER** for the core CRM pipeline workflow. The sprint's Item #9 added FSM validation for stage transitions (`move_to_stage`, `valid_stage_transitions`), but the frontend kanban drag handler appears to not call the backend stage transition API.

## Findings by Severity

### BLOCKERS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| B1 | CRM Pipeline kanban | **Drag-and-drop does not move deals between stages.** DnD event fires (status element shows "Draggable item ... was dropped over droppable area ...") but the deal card remains in the original column. Lead count stays at "1", Business Analysis stays at "0". No toast, no error, no feedback — the action silently fails. | `ux-audit-deal-stuck-in-lead.png` | Wire the dnd-kit `onDragEnd` handler to call `PATCH /api/crm/deals/{id}/move-stage` (or the equivalent `move_to_stage` backend method). The backend FSM validation exists (sprint Item #9) but the frontend never calls it on drop. | INVESTMENT |

### PAIN POINTS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| P1 | Deal detail dialog — stage bar | **Stage progress bar is display-only.** Shows current stage ("Lead") with 5 circle indicators for remaining stages, but clicking them does nothing. User expects clicking a stage to advance the deal (common pattern in CRM tools like HubSpot, Pipedrive). | — | Make the stage indicators clickable. On click, call the stage transition API. Show a confirmation dialog if skipping stages: "Move from Lead directly to Discovery?" | INVESTMENT |
| P2 | Deal card context menu | **Context menu only has Edit and Delete.** No "Move to stage", "Advance to next stage", or stage selection option. This is the most discoverable place for stage transitions but it's missing the key action. | `ux-audit-deal-stuck-in-lead.png` | Add "Move to..." submenu in the context menu showing all available stages. Highlight the next stage as the default/recommended option. | QUICK WIN |
| P3 | Deal detail dialog — no "Advance" button | **No explicit "Advance to next stage" CTA anywhere in the deal detail.** The detail view shows stage, value, probability, description, but no action to progress the deal. The only actions are an edit icon and a close button. | — | Add an "Advance to [next stage]" button in the deal detail header, next to the stage progress bar. Make it the most prominent action. | QUICK WIN |

### FRICTION

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| F1 | Drag-and-drop | **No visual feedback during drag.** When dragging the deal card, there's no visual indication of valid drop targets (columns don't highlight), no ghost card showing where the deal will land, and no "drop here" indicator. The drag just silently fails with no clue why. | — | Add column highlight on dragover, ghost card preview at drop position, and a visual indicator for invalid drops (e.g., red border if FSM blocks the transition). | INVESTMENT |
| F2 | Stage agent labels | **Stage columns show agent names (Scout, Astra, Cash, Lux) with no explanation.** A user seeing "Scout" under "Lead" doesn't know this is an AI agent that handles research at this stage. No tooltip or description. | — | Add tooltips explaining what each agent does at that stage: "Scout — AI agent that researches the prospect and gathers initial intelligence." | QUICK WIN |

### POLISH

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| PO1 | Deal card | **"Research needed" badge on Lead stage deal.** Good contextual status — indicates what action is needed at this stage. This pattern should be consistent across all stages. | — | Ensure each stage has a contextual status badge ("Research needed", "Analysis in progress", "Proposal ready", etc.). | POLISH |

## Journey Scorecard

| Journey | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|------------|----------|------|----------|-------|
| Move deal through stages | Attempted 3 methods: drag, stage bar click, context menu | **FAILED** — deal cannot be moved | 1 (B1) | 3 (P1-P3) | 2 (F1-F2) | **F** — impossible |

## Quick Wins (do first)

1. **P2** — Add "Move to..." submenu in deal card context menu — 2-4 hours
2. **P3** — Add "Advance to [next stage]" button in deal detail dialog — 2-4 hours
3. **F2** — Add tooltips to agent names on stage columns — 1 hour

## Investments (plan for)

1. **B1** — Wire dnd-kit `onDragEnd` to backend stage transition API — 1-2 days — **This is the #1 blocker.** The backend FSM exists, the frontend DnD library is set up, but the two aren't connected. Until this works, the pipeline kanban is a read-only view.
2. **P1** — Make stage progress bar clickable in deal detail — 1 day — Standard CRM pattern, users expect it
3. **F1** — Add visual DnD feedback (column highlights, ghost cards, invalid drop indicators) — 1-2 days — Makes drag-and-drop intuitive even when FSM blocks certain transitions

## Structural Recommendations

1. **Stage transition should have 3 complementary methods** — (a) drag on kanban, (b) click stage in detail view, (c) "Move to" in context menu. All three should call the same backend API (`move_to_stage` with FSM validation). Currently zero of three work. CRM competitors (HubSpot, Pipedrive, Close) offer all three. — Scope: 2-3 days total for all three methods sharing one handler.

## Note on Agent vs User Actions

The planning doc mentions that stage transitions can be triggered by both **agents** (strict FSM validation via `transition_validated`) and **dashboard users** (permissive via `transition_to_phase`). The current UI has no stage transition mechanism for either. The agent flow engine (sprint Item #9) would move deals via backend code, but users have no UI path to do the same. This audit tests the **user** path only.
