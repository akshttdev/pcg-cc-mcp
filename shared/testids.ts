/**
 * Shared Test ID Helpers
 *
 * Single source of truth for data-testid attributes.
 * Used by frontend components (to set testids) and E2E tests (to query them).
 *
 * Convention: {feature}-{element}-{qualifier?}
 *
 * Frontend: import { pipeline } from 'shared/testids';
 * E2E:      import { pipeline } from '../../shared/testids';
 */

const slugify = (name: string) => name.toLowerCase().replace(/\s+/g, "-");

// ── Pipeline Board (CrmPipelineBoard.tsx) ────────────────────────────────────

export const pipeline = {
  addDeal: "pipeline-add-deal",
  addDealEmpty: "pipeline-add-deal-empty",
  addDealStage: (stage: string) => `pipeline-add-deal-${slugify(stage)}`,
  settings: "pipeline-settings",
  stageColumn: (name: string) => `stage-column-${slugify(name)}`,
} as const;

// ── Deal Cards (CrmDealCard.tsx, KanbanCard) ─────────────────────────────────

export const dealCard = {
  card: (id: string) => `deal-card-${id}`,
  menu: (id: string) => `deal-menu-${id}`,
} as const;

// ── Deal Detail Panel (deal-detail/) ─────────────────────────────────────────

export const dealDetail = {
  sheet: "deal-detail-sheet",
  dialog: "deal-detail-dialog",
  expand: "deal-detail-expand",
  edit: "deal-detail-edit",
  delete: "deal-detail-delete",
  tab: (name: string) => `deal-detail-tabs-${name}`,
} as const;

// ── Call Scheduling (OverviewCallSchedulingSection.tsx) ───────────────────────

export const callScheduling = {
  row: (type: string) => `call-${type}-row`,
  date: (type: string) => `call-${type}-date`,
  method: (type: string) => `call-${type}-method`,
  status: (type: string) => `call-${type}-status`,
  save: (type: string) => `call-${type}-save`,
} as const;

// ── Deck & Close Tab (DeckTab.tsx) ───────────────────────────────────────────

export const deck = {
  generate: "deck-generate",
  sendInvoice: "deck-send-invoice",
  confirmSendInvoice: "deck-confirm-send-invoice",
  markWon: "deck-mark-won",
  confirmWon: "deck-confirm-won",
  generateInvite: "deck-generate-invite",
} as const;

// ── Pipeline Settings (CrmPipelineSettings.tsx) ──────────────────────────────

export const pipelineSettings = {
  stageRow: (name: string) => `stage-row-${slugify(name)}`,
  stageEdit: (name: string) => `stage-edit-${slugify(name)}`,
} as const;

// ── Tabs (shared tabs.tsx component) ─────────────────────────────────────────
// TabsTrigger auto-generates: data-testid="tab-{value}"
// TabsContent auto-generates: data-testid="tab-content-{value}"

export const tabs = {
  trigger: (value: string) => `tab-${value}`,
  content: (value: string) => `tab-content-${value}`,
} as const;
