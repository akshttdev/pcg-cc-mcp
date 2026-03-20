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
  dealTranscripts: (dealId: string) => ['deal-transcripts', dealId] as const,
  dealLegacy: (dealId: string) => ['crm-deal', dealId] as const,
  kanbanLegacy: () => ['crm-kanban'] as const,
  kanbanLegacyFlat: () => ['kanban'] as const,
  callLogsDeal: (dealId: string) => ['call-logs-deal', dealId] as const,

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
  activitiesAll: () => ['crm', 'activities'] as const,
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
  byClient: (clientId?: string) => ['projects', 'byClient', clientId] as const,
  clientProjects: (clientId: string) => ['clientProjects', clientId] as const,
};

// ── Organizations ──────────────────────────────────────────────────────────

export const organizationKeys = {
  all: ['organizations'] as const,
  admin: () => ['organizations-admin'] as const,
  detail: (orgId?: string) => ['organization', orgId] as const,
  clients: (orgId?: string) => ['org-clients', orgId] as const,
  orgClients: (orgId?: string) => ['organizations', orgId, 'clients'] as const,
  clientsSettings: (orgId?: string | null) => ['clients', orgId] as const,
  members: (orgId: string) => ['org-members', orgId] as const,
  orgData: (orgId: string) => ['org', orgId] as const,
  brandProfile: (orgId: string) => ['orgBrandProfile', orgId] as const,
  knowledge: (orgId: string) => ['orgKnowledge', orgId] as const,
  projects: (orgId: string) => ['orgProjects', orgId] as const,
  deliverables: (orgId: string, projectIds: string[]) =>
    ['orgDeliverables', orgId, projectIds] as const,
  onboarding: (orgId: string) => ['org-onboarding', orgId] as const,
  memberAssignments: (orgId: string, userId: string) => ['member-assignments', orgId, userId] as const,
  projectsList: (orgId: string) => ['org-projects-list', orgId] as const,
  deals: (orgId: string) => ['org-deals', orgId] as const,
  dealsEnriched: (orgId: string) => ['crm-deals-org-enriched', orgId] as const,
  activitiesOrg: (orgId: string) => ['crm-activities-org', orgId] as const,
  workflowRuns: (orgId: string) => ['workflow-runs', orgId] as const,
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
  recentRuns: () => ['workflowRuns'] as const,
  runs: (workflowId: string, organizationId?: string) =>
    ['workflow-runs', workflowId, organizationId] as const,
  runsBuilder: () => ['workflow-runs-builder'] as const,
  systemAutomations: () => ['system-automations'] as const,
  status: (workflowId: string) => ['workflow-status', workflowId] as const,
  statusPoll: (workflowId: string) => ['workflow-status-poll', workflowId] as const,
  artifacts: (workflowId: string) => ['workflow-artifacts', workflowId] as const,
  runsByWorkflow: (workflowId: string) => ['workflowRunsByWf', workflowId] as const,
  dataSourceNamesForRuns: (dsIds: string[]) => ['ds-names-for-runs', dsIds] as const,
  dataSourceNamesForRunsTab: (dsIds: string[]) => ['ds-names-for-runs-tab', dsIds] as const,
  stagingPendingByWorkflow: (workflowId: string) => ['stagingPendingByWf', workflowId] as const,
  orgDataSources: (orgId: string) => ['orgDataSources', orgId] as const,
  staging: (workflowRunId?: string) => ['staging', workflowRunId] as const,
  stagingPending: () => ['stagingPending'] as const,
  stagingPendingOrg: (orgId?: string) => ['stagingPending', orgId] as const,
  schemas: (targetTypes: string[]) => ['schemas', targetTypes] as const,
};

// ── Data Sources ───────────────────────────────────────────────────────────

export const dataSourceKeys = {
  all: ['dataSources'] as const,
  personal: () => ['dataSources', 'personal'] as const,
  list: (orgId?: string, projectId?: string) => ['dataSources', orgId, projectId] as const,
  detail: (id: string) => ['dataSource', id] as const,
  workflows: (id: string) => ['dataSourceWorkflows', id] as const,
  artifacts: (id: string) => ['dataSourceArtifacts', id] as const,
  names: (ids: string[]) => ['data-source-names', ids] as const,
  project: (projectId: string) => ['dataSourcesProject', projectId] as const,
  recentArtifacts: () => ['recentArtifacts'] as const,
};

// ── Users ──────────────────────────────────────────────────────────────────

export const userKeys = {
  all: ['users'] as const,
  allUsers: () => ['all-users'] as const,
  list: (filters?: unknown) => ['users', filters] as const,
  search: (query: string) => ['users-search', query] as const,
  profiles: () => ['profiles'] as const,
  projectMembers: (projectId: string) => ['project-members', projectId] as const,
  clientMembers: (clientId: string) => ['client-members', clientId] as const,
};

// ── Notifications ──────────────────────────────────────────────────────────

export const notificationKeys = {
  activity: () => ['notifications'] as const,
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
  companyBrandProfile: (id: string) => ['companyBrandProfile', id] as const,
  person: (id: string) => ['person', id] as const,
  researchPasses: (personId: string) => ['research-passes', personId] as const,
  personReports: (personId: string) => ['person-reports', personId] as const,
  personNotes: (personId: string) => ['person-notes', personId] as const,
  leads: (orgFilter?: string, typeFilter?: string, search?: string) =>
    ['persons', 'leads', orgFilter, typeFilter, search] as const,
  allDirectory: () => ['allProjectsDirectory'] as const,
  companiesAll: () => ['companies-all'] as const,
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
  projects: () => ['pulse', 'projects'] as const,
  contentLatest: (projectId: string) => ['pulse', 'content', 'latest', projectId] as const,
  contentLegacy: () => ['pulse', 'content', 'legacy'] as const,
  alertsOrg: (orgId: string) => ['pulse-alerts-org', orgId] as const,
  contentOrg: (orgId: string) => ['pulse-content-org', orgId] as const,
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
  todaySummary: () => ['token-usage-today'] as const,
  daily: (days: number) => ['token-usage', 'daily', days] as const,
  byProvider: (days: number) => ['token-usage', 'by-provider', days] as const,
  byModel: (days: number) => ['token-usage', 'by-model', days] as const,
  byProject: (days: number) => ['token-usage', 'by-project', days] as const,
  byProjectSummary: () => ['token-usage-by-project'] as const,
  byAgent: (days: number) => ['token-usage', 'by-agent', days] as const,
};

// ── Settings ───────────────────────────────────────────────────────────────

export const settingsKeys = {
  providerKeys: () => ['provider-keys'] as const,
  apnCapabilities: () => ['apn-capabilities'] as const,
  modelPricing: () => ['model-pricing'] as const,
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
  activeSessions: () => ['discord-active-sessions'] as const,
};

// ── Integrations ────────────────────────────────────────────────────────────

export const integrationKeys = {
  githubTokenStatus: () => ['github-token-status'] as const,
  emailAccountsOrg: (orgId: string) => ['email-accounts-org', orgId] as const,
  qbStatusOrg: (orgId: string) => ['qb-status-org', orgId] as const,
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

// ── Autonomy ────────────────────────────────────────────────────────────────

export const autonomyKeys = {
  all: ['autonomy'] as const,
  taskMode: (taskId: string) => ['autonomy', 'task', taskId, 'mode'] as const,
  checkpointDefinitions: (projectId: string) => ['autonomy', 'checkpoints', 'definitions', projectId] as const,
  executionCheckpoints: (executionId: string) => ['autonomy', 'checkpoints', executionId] as const,
  pendingCheckpoints: (executionId: string) => ['autonomy', 'checkpoints', executionId, 'pending'] as const,
  projectGates: (projectId: string) => ['autonomy', 'gates', projectId] as const,
  pendingGates: (executionId: string) => ['autonomy', 'gates', executionId, 'pending'] as const,
  pendingSummary: () => ['autonomy', 'pending-summary'] as const,
  canProceed: (executionId: string) => ['autonomy', 'can-proceed', executionId] as const,
};

// ── Bowser (Browser Automation) ─────────────────────────────────────────────

export const bowserKeys = {
  all: ['bowser'] as const,
  summary: () => ['bowser', 'summary'] as const,
  sessionsActive: () => ['bowser', 'sessions', 'active'] as const,
  session: (sessionId: string) => ['bowser', 'session', sessionId] as const,
  sessionDetails: (sessionId: string) => ['bowser', 'session', sessionId, 'details'] as const,
  screenshots: (sessionId: string) => ['bowser', 'session', sessionId, 'screenshots'] as const,
  screenshotsDiffs: (sessionId: string) => ['bowser', 'session', sessionId, 'screenshots', 'diffs'] as const,
  actions: (sessionId: string) => ['bowser', 'session', sessionId, 'actions'] as const,
  allowlist: (projectId: string) => ['bowser', 'allowlist', projectId] as const,
};

// ── TopiClips ─────────────────────────────────────────────────────────────

export const topiclipsKeys = {
  all: ['topiclips'] as const,
  gallery: (projectId: string) => ['topiclips', 'gallery', projectId] as const,
  symbols: () => ['topiclips', 'symbols'] as const,
  timeline: (sessionId: string) => ['topiclips', 'timeline', sessionId] as const,
};

// ── Topsi ──────────────────────────────────────────────────────────────────

export const topsiKeys = {
  all: ['topsi'] as const,
  status: () => ['topsi', 'status'] as const,
  topology: () => ['topsi', 'topology'] as const,
  issues: () => ['topsi', 'issues'] as const,
  projects: () => ['topsi', 'projects'] as const,
  recommendations: (projectId?: string) => ['topsi', 'recommendations', projectId ?? 'all'] as const,
};

// ── Collaboration ───────────────────────────────────────────────────────────

export const collaborationKeys = {
  all: ['collaboration'] as const,
  state: (executionId: string) => ['collaboration', executionId] as const,
  pauseHistory: (executionId: string) => ['collaboration', executionId, 'pause-history'] as const,
  handoffs: (executionId: string) => ['collaboration', executionId, 'handoffs'] as const,
  injections: (executionId: string) => ['collaboration', executionId, 'injections'] as const,
  pendingInjections: (executionId: string) => ['collaboration', executionId, 'pending-injections'] as const,
};

// ── Social ──────────────────────────────────────────────────────────────────

export const socialKeys = {
  accounts: (projectId?: string | null) => ['social-accounts', projectId] as const,
  postsAll: () => ['social-posts'] as const,
  posts: (projectId?: string | null) => ['social-posts', projectId] as const,
  mentions: (projectId?: string | null) => ['social-mentions', projectId] as const,
  mentionsOverview: (projectId: string) => ['social-mentions-ov', projectId] as const,
  inboxStats: (projectId?: string | null) => ['social-inbox-stats', projectId] as const,
};

// ── Network ─────────────────────────────────────────────────────────────────

export const networkKeys = {
  apnIdentity: () => ['apn-identity'] as const,
  pythiaHealth: () => ['pythia-health'] as const,
  pythiaEconomics: () => ['pythia-economics'] as const,
};

// ── Org Cloud ─────────────────────────────────────────────────────────────

export const orgCloudKeys = {
  all: (orgId: string) => ['org-cloud', orgId] as const,
  browse: (orgId: string, params?: Record<string, unknown>) => ['org-cloud', orgId, params] as const,
  stats: (orgId: string) => ['org-cloud-stats', orgId] as const,
  settings: (orgId: string) => ['org-cloud-settings', orgId] as const,
  contributions: (orgId: string) => ['org-cloud-contributions', orgId] as const,
};

// ── Agent Flows ───────────────────────────────────────────────────────────

export const agentFlowKeys = {
  all: ['agentFlows'] as const,
  detailAll: ['agentFlow'] as const,
  list: (taskId?: string, status?: string) => ['agentFlows', taskId, status] as const,
  detail: (flowId?: string) => ['agentFlow', flowId] as const,
  awaitingApproval: () => ['agentFlows', 'awaiting-approval'] as const,
  events: (flowId?: string, since?: string, eventType?: string) =>
    ['agentFlowEvents', flowId, since, eventType] as const,
  byTasks: (taskIds: string[]) => ['agentFlows', 'byTasks', [...taskIds].sort().join(',')] as const,
};

// ── Task Card Data ────────────────────────────────────────────────────────

export const taskCardKeys = {
  artifacts: (taskId?: string) => ['taskArtifacts', taskId] as const,
  workflowEvents: (taskId?: string) => ['taskWorkflowEvents', taskId] as const,
  batchArtifacts: (taskIds: string[]) => ['tasksArtifacts', [...taskIds].sort().join(',')] as const,
  batchWorkflowEvents: (taskIds: string[]) => ['tasksWorkflowEvents', [...taskIds].sort().join(',')] as const,
};

// ── Project Capacity ──────────────────────────────────────────────────────

export const capacityKeys = {
  project: (projectId?: string) => ['projectCapacity', projectId] as const,
  activeSlots: (projectId?: string) => ['activeSlots', projectId] as const,
  activeExecutions: () => ['activeExecutions'] as const,
};

// ── Project Access ────────────────────────────────────────────────────────

export const projectAccessKeys = {
  access: (projectId?: string) => ['project-access', projectId] as const,
};

// ── Execution Processes ───────────────────────────────────────────────────

export const executionProcessKeys = {
  list: (attemptId?: string) => ['executionProcesses', attemptId] as const,
  details: (processId: string) => ['processDetails', processId] as const,
};

// ── Branch & Git ──────────────────────────────────────────────────────────

export const branchKeys = {
  status: (attemptId?: string) => ['branchStatus', attemptId] as const,
  attemptBranch: (attemptId?: string) => ['attemptBranch', attemptId] as const,
  projectBranches: (projectId?: string) => ['projectBranches', projectId] as const,
};

// ── Mission Control ───────────────────────────────────────────────────────

export const missionControlKeys = {
  dashboard: () => ['missionControl', 'dashboard'] as const,
  artifacts: (executionId?: string) => ['missionControl', 'artifacts', executionId] as const,
  plan: (executionId?: string) => ['missionControl', 'plan', executionId] as const,
  activePlans: () => ['missionControl', 'activePlans'] as const,
};

// ── System Metrics ────────────────────────────────────────────────────────

export const systemMetricsKeys = {
  basic: () => ['system-metrics'] as const,
  detailed: () => ['system-metrics-detailed'] as const,
};

// ── Project Boards ────────────────────────────────────────────────────────

export const projectBoardKeys = {
  boards: (projectId?: string) => ['projectBoards', projectId] as const,
  tasks: (projectId?: string) => ['projectTasks', projectId] as const,
};

// ── Org Persons ───────────────────────────────────────────────────────────

export const orgPersonKeys = {
  list: (orgId?: string) => ['org-persons', orgId] as const,
};

// ── Workflow Templates ────────────────────────────────────────────────────

export const workflowTemplateKeys = {
  all: () => ['workflow-templates'] as const,
};

// ── Orcha ─────────────────────────────────────────────────────────────────

export const orchaKeys = {
  status: () => ['orcha-status'] as const,
};

// ── Agent Watchers ───────────────────────────────────────────────────────

export const agentWatcherKeys = {
  watchers: (taskId: string) => ['agent-watchers', taskId] as const,
  availableAgents: () => ['available-agents-for-watchers'] as const,
};

// ── Command Center ─────────────────────────────────────────────────────────

export const commandCenterKeys = {
  dashboard: () => ['command-center'] as const,
};

// ── Costs ─────────────────────────────────────────────────────────────────

export const costKeys = {
  orgSummary: (orgId: string, days?: number) => ['costs', 'summary', orgId, days] as const,
  orgByModel: (orgId: string, days?: number) => ['costs', 'by-model', orgId, days] as const,
  orgByProject: (orgId: string, days?: number) => ['costs', 'by-project', orgId, days] as const,
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
  integrations: integrationKeys,
  media: mediaKeys,
  knowledge: knowledgeKeys,
  autonomy: autonomyKeys,
  bowser: bowserKeys,
  topiclips: topiclipsKeys,
  topsi: topsiKeys,
  collaboration: collaborationKeys,
  social: socialKeys,
  network: networkKeys,
  orgCloud: orgCloudKeys,
  agentFlows: agentFlowKeys,
  taskCard: taskCardKeys,
  capacity: capacityKeys,
  projectAccess: projectAccessKeys,
  executionProcesses: executionProcessKeys,
  branches: branchKeys,
  missionControl: missionControlKeys,
  systemMetrics: systemMetricsKeys,
  projectBoards: projectBoardKeys,
  orgPersons: orgPersonKeys,
  workflowTemplates: workflowTemplateKeys,
  orcha: orchaKeys,
  agentWatchers: agentWatcherKeys,
  commandCenter: commandCenterKeys,
  costs: costKeys,
} as const;
