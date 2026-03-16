/**
 * Centralized query key factory.
 *
 * Every React-Query cache key should be produced through this factory so that
 * consumers and invalidation sites reference the same constants.
 *
 * Pattern: each domain exposes an `all` root key and scoped builders that
 * extend it.  Prefix-based invalidation works automatically —
 * `queryKeys.tasks.all` invalidates every tasks-related query.
 */

import type { PipelineType } from '@/types/crm';

// ── Tasks ──────────────────────────────────────────────────────────────────

export const taskKeys = {
  all: ['tasks'] as const,
  list: (projectId: string) => ['tasks', projectId] as const,
  detail: (taskId: string) => ['task', taskId] as const,
  my: () => ['my-tasks'] as const,
  myCreated: () => ['my-created-tasks'] as const,
  myWatched: () => ['my-watched-tasks'] as const,
  global: (projectIds: string[]) => ['global-tasks', projectIds] as const,
  comments: (taskId: string) => ['taskComments', taskId] as const,
  activity: (taskId: string) => ['taskActivity', taskId] as const,
  attempts: (taskId: string) => ['taskAttempts', taskId] as const,
  attemptsAll: () => ['task-attempts-all'] as const,
  projectTasks: (projectId: string) => ['project-tasks', projectId] as const,
};

// ── CRM ────────────────────────────────────────────────────────────────────

export const crmKeys = {
  all: ['crm'] as const,

  // Contacts
  contactsAll: () => ['crm-contacts'] as const,
  contacts: (orgId?: string) => ['crm-contacts', orgId] as const,
  contactsFiltered: (projectId?: string | null, stage?: string | null) =>
    ['crm-contacts', projectId, stage] as const,
  contactsSearch: (projectId?: string | null, query?: string | null) =>
    ['crm-contacts-search', projectId, query] as const,
  contact: (contactId: string) => ['crm', 'contact', contactId] as const,
  contactLegacy: (contactId: string) => ['crm-contact', contactId] as const,
  contactDeals: (contactId: string) => ['contact-deals', contactId] as const,
  contactActivities: (contactId: string) => ['contact-activities', contactId] as const,

  // Pipelines (project-scoped)
  pipelines: (organizationId: string) => ['crm', 'pipelines', organizationId] as const,
  pipelinesByType: (organizationId: string, type: PipelineType) =>
    ['crm', 'pipelines', organizationId, type] as const,
  pipeline: (id: string) => ['crm', 'pipeline', id] as const,
  pipelinesLegacy: () => ['crmPipelines'] as const,
  pipelineStages: (pipelineId: string) => ['crmPipelineStages', pipelineId] as const,

  // Pipelines (org-scoped)
  orgPipelines: (orgId: string) => ['crm', 'org-pipelines', orgId] as const,
  orgPipelinesByType: (orgId: string, type: PipelineType) =>
    ['crm', 'org-pipelines', orgId, type] as const,
  orgKanban: (orgId: string, pipelineId: string) =>
    ['crm', 'org-kanban', orgId, pipelineId] as const,

  // Kanban & Deals
  kanban: (pipelineId: string) => ['crm', 'kanban', pipelineId] as const,
  kanbanAll: () => ['crm', 'kanban'] as const,
  orgKanbanAll: () => ['crm', 'org-kanban'] as const,
  deals: (organizationId: string) => ['crm', 'deals', organizationId] as const,
  dealsAll: () => ['crm', 'deals'] as const,
  deal: (id: string) => ['crm', 'deal', id] as const,
  dealsByContact: (contactId: string) => ['crm', 'deals', 'contact', contactId] as const,

  // Activities
  activities: (projectId: string) => ['crm', 'activities', projectId] as const,
  activitiesByContact: (contactId: string) =>
    ['crm', 'activities', 'contact', contactId] as const,
  activitiesByDeal: (dealId: string) => ['crm', 'activities', 'deal', dealId] as const,

  // Metrics & Stats
  metrics: (projectId: string, pipelineId?: string) =>
    ['crm', 'metrics', projectId, pipelineId] as const,
  statsAll: () => ['crm-stats'] as const,
  stats: (projectId?: string | null) => ['crm-stats', projectId] as const,
};

// ── Projects ───────────────────────────────────────────────────────────────

export const projectKeys = {
  all: ['projects'] as const,
  list: (filter?: string) => ['projects', filter] as const,
  detail: (projectId: string) => ['project', projectId] as const,
  crm: () => ['projects', 'crm'] as const,
};

// ── Organizations ──────────────────────────────────────────────────────────

export const organizationKeys = {
  all: ['organizations'] as const,
  detail: (orgId?: string) => ['organization', orgId] as const,
  clients: (orgId?: string) => ['org-clients', orgId] as const,
  clientsSettings: (orgId?: string | null) => ['clients', orgId] as const,
  members: (orgId: string) => ['org-members', orgId] as const,
  orgData: (orgId: string) => ['org', orgId] as const,
  brandProfile: (orgId: string) => ['orgBrandProfile', orgId] as const,
  knowledge: (orgId: string) => ['orgKnowledge', orgId] as const,
  projects: (orgId: string) => ['orgProjects', orgId] as const,
  deliverables: (orgId: string, projectIds: string[]) =>
    ['orgDeliverables', orgId, projectIds] as const,
  onboarding: (orgId: string) => ['org-onboarding', orgId] as const,
};

// ── Sidebar & Navigation ───────────────────────────────────────────────────

export const sidebarKeys = {
  tree: () => ['sidebarTree'] as const,
  treeLegacy: () => ['sidebar-tree'] as const,
  projectBoards: (projectId: string) => ['projectBoardsSidebar', projectId] as const,
  stagingPendingCount: (orgId: string) => ['stagingPendingCount', orgId] as const,
};

// ── Agents ─────────────────────────────────────────────────────────────────

export const agentKeys = {
  all: ['agents'] as const,
  active: () => ['agents', 'active'] as const,
  detail: (agentId: string) => ['agents', agentId] as const,
  byName: (name: string) => ['agents', 'by-name', name] as const,
  list: () => ['agents-list'] as const,
  executionConfig: (agentId: string) => ['agent-execution-config', agentId] as const,
  executionProfiles: () => ['execution-profiles'] as const,
  wallets: () => ['agent-wallets'] as const,
};

// ── Workflows & Execution ──────────────────────────────────────────────────

export const workflowKeys = {
  all: ['workflows'] as const,
  definitions: () => ['workflowDefinitions'] as const,
  triggers: (workflowId: string) => ['workflowTriggers', workflowId] as const,
  models: () => ['workflowModels'] as const,
  runs: (workflowId: string, organizationId?: string) =>
    ['workflow-runs', workflowId, organizationId] as const,
  runsBuilder: () => ['workflow-runs-builder'] as const,
  status: (workflowId: string) => ['workflow-status', workflowId] as const,
  statusPoll: (workflowId: string) => ['workflow-status-poll', workflowId] as const,
  artifacts: (workflowId: string) => ['workflow-artifacts', workflowId] as const,
  staging: (workflowRunId?: string) => ['staging', workflowRunId] as const,
  stagingPending: () => ['stagingPending'] as const,
  schemas: (targetTypes: string[]) => ['schemas', targetTypes] as const,
};

// ── Data Sources ───────────────────────────────────────────────────────────

export const dataSourceKeys = {
  all: () => ['dataSources'] as const,
  detail: (id: string) => ['dataSource', id] as const,
  workflows: (id: string) => ['dataSourceWorkflows', id] as const,
  artifacts: (id: string) => ['dataSourceArtifacts', id] as const,
  names: (ids: string[]) => ['data-source-names', ids] as const,
};

// ── Users ──────────────────────────────────────────────────────────────────

export const userKeys = {
  all: ['users'] as const,
  list: (filters?: unknown) => ['users', filters] as const,
  search: (query: string) => ['users-search', query] as const,
  profiles: () => ['profiles'] as const,
  projectMembers: (projectId: string) => ['project-members', projectId] as const,
  clientMembers: (clientId: string) => ['client-members', clientId] as const,
};

// ── Notifications ──────────────────────────────────────────────────────────

export const notificationKeys = {
  inbox: () => ['notifications-inbox'] as const,
};

// ── Execution ──────────────────────────────────────────────────────────────

export const executionKeys = {
  summary: (attemptId: string) => ['execution-summary', attemptId] as const,
  ralphLoop: (id: string) => ['ralph-loop', id] as const,
  ralphIterations: (id: string) => ['ralph-iterations', id] as const,
};

// ── Business ───────────────────────────────────────────────────────────────

export const businessKeys = {
  proposals: () => ['proposals'] as const,
  companyProposals: (companyId: string) => ['company-proposals', companyId] as const,
  invoices: (filter?: string) => ['invoices', filter] as const,
  invoicesAll: () => ['invoices'] as const,
  deliverables: (projectId: string) => ['deliverables', projectId] as const,
  reports: () => ['business-reports'] as const,
  report: (id: string) => ['report', id] as const,
  meetings: (proposalId: string) => ['scheduled-meetings', proposalId] as const,
};

// ── Companies & People ─────────────────────────────────────────────────────

export const entityKeys = {
  companies: () => ['companies'] as const,
  company: (id: string) => ['company', id] as const,
  companyContacts: (id: string) => ['company-contacts', id] as const,
  companyIntel: (id: string) => ['company-intel', id] as const,
  companyContactMethods: (id: string) => ['company-contact-methods', id] as const,
  person: (id: string) => ['person', id] as const,
  researchPasses: (personId: string) => ['research-passes', personId] as const,
  leads: (orgFilter?: string, typeFilter?: string, search?: string) =>
    ['persons', 'leads', orgFilter, typeFilter, search] as const,
  allDirectory: () => ['allProjectsDirectory'] as const,
};

// ── Pulse ──────────────────────────────────────────────────────────────────

export const pulseKeys = {
  all: () => ['pulse'] as const,
  stats: (projectId: string) => ['pulse', 'stats', projectId] as const,
  content: (projectId: string) => ['pulse', 'content', projectId] as const,
  contentAll: () => ['pulse', 'content'] as const,
  statsAll: () => ['pulse', 'stats'] as const,
  sources: (projectId: string) => ['pulse', 'sources', projectId] as const,
  alerts: (projectId: string) => ['pulse', 'alerts', projectId] as const,
  alertRules: (projectId: string) => ['pulse', 'alert-rules', projectId] as const,
  tracking: (projectId: string) => ['pulse', 'tracking', projectId] as const,
};

// ── Communications ─────────────────────────────────────────────────────────

export const commsKeys = {
  callStats: (projectId: string) => ['call-stats', projectId] as const,
  calls: (projectId: string) => ['calls', projectId] as const,
  smsStats: (projectId: string) => ['sms-stats', projectId] as const,
  sms: (projectId: string) => ['sms', projectId] as const,
  smsAll: () => ['sms'] as const,
  smsStatsAll: () => ['sms-stats'] as const,
  emailAccounts: (ownerType?: string, ownerId?: string) =>
    ['email-accounts', ownerType, ownerId] as const,
  emailAccountsAll: () => ['email-accounts'] as const,
  emailInboxStats: (projectId: string) => ['email-inbox-stats', projectId] as const,
  emailMessages: (projectId?: string, accountId?: string, filter?: string) =>
    ['email-messages', projectId, accountId, filter] as const,
  emailMessagesAll: () => ['email-messages'] as const,
};

// ── Token Usage ────────────────────────────────────────────────────────────

export const tokenUsageKeys = {
  today: () => ['token-usage', 'today'] as const,
  daily: (days: number) => ['token-usage', 'daily', days] as const,
  byProvider: (days: number) => ['token-usage', 'by-provider', days] as const,
  byModel: (days: number) => ['token-usage', 'by-model', days] as const,
  byProject: (days: number) => ['token-usage', 'by-project', days] as const,
  byAgent: (days: number) => ['token-usage', 'by-agent', days] as const,
};

// ── Settings ───────────────────────────────────────────────────────────────

export const settingsKeys = {
  providerKeys: () => ['provider-keys'] as const,
  apnCapabilities: () => ['apn-capabilities'] as const,
};

// ── Project Controller ─────────────────────────────────────────────────────

export const controllerKeys = {
  config: (projectId: string) => ['project-controller-config', projectId] as const,
  conversations: (projectId: string) =>
    ['project-controller-conversations', projectId] as const,
  conversation: (projectId: string, conversationId: string) =>
    ['project-controller-conversation', projectId, conversationId] as const,
};

// ── OSS Libraries ──────────────────────────────────────────────────────────

export const ossKeys = {
  libraries: () => ['oss-libraries'] as const,
  updates: (libraryId: string) => ['oss-updates', libraryId] as const,
  recent: () => ['oss-recent'] as const,
};

// ── Review / Intake ────────────────────────────────────────────────────────

export const reviewKeys = {
  detail: (token: string) => ['review', token] as const,
  intake: (token: string) => ['intake', token] as const,
};

// ── Discord ────────────────────────────────────────────────────────────────

export const discordKeys = {
  active: () => ['discord-active'] as const,
  archive: () => ['discord-archive'] as const,
};

// ── Media ──────────────────────────────────────────────────────────────────

export const mediaKeys = {
  library: (projectId: string) => ['media', projectId] as const,
  search: (projectId: string, query: string) => ['media', projectId, query] as const,
};

// ── Knowledge ──────────────────────────────────────────────────────────────

export const knowledgeKeys = {
  project: (projectId: string) => ['projectKnowledge', projectId] as const,
};

// ── Unified export ─────────────────────────────────────────────────────────

export const queryKeys = {
  tasks: taskKeys,
  crm: crmKeys,
  projects: projectKeys,
  organizations: organizationKeys,
  sidebar: sidebarKeys,
  agents: agentKeys,
  workflows: workflowKeys,
  dataSources: dataSourceKeys,
  users: userKeys,
  notifications: notificationKeys,
  executions: executionKeys,
  business: businessKeys,
  entities: entityKeys,
  pulse: pulseKeys,
  comms: commsKeys,
  tokenUsage: tokenUsageKeys,
  settings: settingsKeys,
  controller: controllerKeys,
  oss: ossKeys,
  review: reviewKeys,
  discord: discordKeys,
  media: mediaKeys,
  knowledge: knowledgeKeys,
} as const;
