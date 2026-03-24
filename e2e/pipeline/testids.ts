/**
 * Pipeline E2E Test IDs
 *
 * Centralized testid constants matching data-testid attributes in components.
 * If naming conventions change, update here — not in every spec file.
 *
 * Convention: {feature}-{element}-{qualifier?}
 * Components: frontend/src/components/crm/...
 */

// ── Pipeline Board (CrmPipelineBoard.tsx) ────────────────────────────────────

export const pipeline = {
  addDeal: "pipeline-add-deal",
  addDealEmpty: "pipeline-add-deal-empty",
  addDealStage: (stage: string) => `pipeline-add-deal-${stage}`,
  settings: "pipeline-settings",
  stageColumn: (name: string) => `stage-column-${name.toLowerCase().replace(/\s+/g, "-")}`,
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

// ── Call Scheduling (OverviewTab.tsx) ─────────────────────────────────────────

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
  markWon: "deck-mark-won",
  generateInvite: "deck-generate-invite",
} as const;

// ── Pipeline Settings (CrmPipelineSettings.tsx) ──────────────────────────────

export const pipelineSettings = {
  stageRow: (name: string) => `stage-row-${name.toLowerCase().replace(/\s+/g, "-")}`,
  stageEdit: (name: string) => `stage-edit-${name.toLowerCase().replace(/\s+/g, "-")}`,
} as const;

// ── Tabs (shared tabs.tsx component) ─────────────────────────────────────────
// TabsTrigger auto-generates: data-testid="tab-{value}"
// TabsContent auto-generates: data-testid="tab-content-{value}"

export const tabs = {
  trigger: (value: string) => `tab-${value}`,
  content: (value: string) => `tab-content-${value}`,
} as const;
