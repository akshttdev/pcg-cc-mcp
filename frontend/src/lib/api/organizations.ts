import { makeRequest, handleApiResponse } from './client';
import type { PersonOrgContact, OrgBrandProfile, OrgKnowledgeSource } from './communication';

// ============================================================================
// Sidebar Tree Types
// ============================================================================

export interface SidebarProject {
  id: string;
  name: string;
  is_container: boolean;
  children: SidebarProject[];
  health_status?: string;
  active_issues_count?: number;
  knowledge_completeness?: number;
  last_activity_at?: string;
}

export interface SidebarClient {
  id: string;
  name: string;
  slug: string;
  health_status?: string;
  active_issues_count?: number;
  knowledge_completeness?: number;
  last_activity_at?: string;
  crm_person_id?: string;
  crm_confidence?: number;
  projects: SidebarProject[];
}

export interface SidebarSharedBoard {
  board_id: string;
  board_name: string;
  project_id: string;
  project_name: string;
  permission: string;
  share_type: string;
}

export interface SidebarSharedBoardGroup {
  source_org_id: string;
  source_org_name: string;
  share_type: string;
  boards: SidebarSharedBoard[];
}

export interface SidebarOrg {
  id: string;
  name: string;
  slug: string;
  role: string;
  health_status?: string;
  active_issues_count?: number;
  knowledge_completeness?: number;
  last_activity_at?: string;
  internal_projects: SidebarProject[];
  clients: SidebarClient[];
  shared_boards: SidebarSharedBoardGroup[];
}

export interface SidebarTree {
  owned_orgs: SidebarOrg[];
  member_orgs: SidebarOrg[];
}

export interface OrganizationData {
  id: string;
  name: string;
  slug: string;
  description?: string;
  avatar_url?: string;
  owner_id: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  address?: string;
}

export interface ClientData {
  id: string;
  organization_id: string;
  name: string;
  slug: string;
  description?: string;
  logo_url?: string;
  website?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Organizations & Clients API
// ============================================================================

export const organizationsApi = {
  // Sidebar tree
  getSidebarTree: async (): Promise<SidebarTree> => {
    const response = await makeRequest('/api/sidebar/tree');
    return handleApiResponse<SidebarTree>(response);
  },

  // Organizations
  getAll: async (): Promise<OrganizationData[]> => {
    const response = await makeRequest('/api/organizations');
    return handleApiResponse<OrganizationData[]>(response);
  },

  getById: async (id: string): Promise<OrganizationData> => {
    const response = await makeRequest(`/api/organizations/${id}`);
    return handleApiResponse<OrganizationData>(response);
  },

  create: async (data: { name: string; slug: string; description?: string }): Promise<OrganizationData> => {
    const response = await makeRequest('/api/organizations', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrganizationData>(response);
  },

  update: async (id: string, data: { name?: string; slug?: string; description?: string }): Promise<OrganizationData> => {
    const response = await makeRequest(`/api/organizations/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrganizationData>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  activate: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${id}/activate`, {
      method: 'PATCH',
    });
    return handleApiResponse<void>(response);
  },

  deactivate: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${id}/deactivate`, {
      method: 'PATCH',
    });
    return handleApiResponse<void>(response);
  },

  // Members
  getMembers: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members`);
    return handleApiResponse<any[]>(response);
  },

  addMember: async (orgId: string, userId: string, role?: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members`, {
      method: 'POST',
      body: JSON.stringify({ user_id: userId, role }),
    });
    return handleApiResponse<any>(response);
  },

  removeMember: async (orgId: string, userId: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  changeMemberRole: async (orgId: string, userId: string, role: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/role`, {
      method: 'PUT',
      body: JSON.stringify({ role }),
    });
    return handleApiResponse<any>(response);
  },

  // Member assignments
  getMemberAssignments: async (orgId: string, userId: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assignments`);
    return handleApiResponse<any>(response);
  },

  assignMember: async (orgId: string, userId: string, type: string, targetId: string, role?: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assign`, {
      method: 'POST',
      body: JSON.stringify({ type, target_id: targetId, role }),
    });
    return handleApiResponse<any>(response);
  },

  watchTaskForMember: async (orgId: string, userId: string, taskId: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/watch`, {
      method: 'POST',
      body: JSON.stringify({ task_id: taskId }),
    });
    return handleApiResponse<any>(response);
  },

  unassignProject: async (orgId: string, userId: string, projectId: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assignments/project/${projectId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  unassignClient: async (orgId: string, userId: string, clientId: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assignments/client/${clientId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Org invitations
  createInvitation: async (orgId: string, role?: string, maxUses?: number, expiresInHours?: number): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/invitations`, {
      method: 'POST',
      body: JSON.stringify({ role, max_uses: maxUses, expires_in_hours: expiresInHours }),
    });
    return handleApiResponse<any>(response);
  },

  listInvitations: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/invitations`);
    return handleApiResponse<any[]>(response);
  },

  // Clients
  getClients: async (orgId: string): Promise<ClientData[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/clients`);
    return handleApiResponse<ClientData[]>(response);
  },

  createClient: async (orgId: string, data: { name: string; slug: string; description?: string; website?: string }): Promise<ClientData> => {
    const response = await makeRequest(`/api/organizations/${orgId}/clients`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ClientData>(response);
  },

  updateClient: async (clientId: string, data: { name?: string; slug?: string; description?: string; website?: string }): Promise<ClientData> => {
    const response = await makeRequest(`/api/clients/${clientId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ClientData>(response);
  },

  deleteClient: async (clientId: string): Promise<void> => {
    const response = await makeRequest(`/api/clients/${clientId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Board Shares
  getBoardShares: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/board-shares`);
    return handleApiResponse<any[]>(response);
  },

  getSharedBoards: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/shared-boards`);
    return handleApiResponse<any[]>(response);
  },

  createBoardShare: async (orgId: string, data: { board_id: string; target_organization_id: string; permission?: string; share_type?: string }): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/board-shares`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },

  updateBoardShare: async (shareId: string, data: { permission?: string; share_type?: string; is_active?: boolean }): Promise<any> => {
    const response = await makeRequest(`/api/board-shares/${shareId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },

  deleteBoardShare: async (shareId: string): Promise<void> => {
    const response = await makeRequest(`/api/board-shares/${shareId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Person-org junction (for context badges)
  listPersonContacts: async (orgId: string): Promise<PersonOrgContact[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/person-contacts`);
    return handleApiResponse<PersonOrgContact[]>(response);
  },
  addPersonContact: async (orgId: string, data: { person_id: string; context?: string }): Promise<PersonOrgContact> => {
    const response = await makeRequest(`/api/organizations/${orgId}/person-contacts`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonOrgContact>(response);
  },

  // Brand profile
  getBrandProfile: async (orgId: string): Promise<OrgBrandProfile | null> => {
    const response = await makeRequest(`/api/organizations/${orgId}/brand-profile`);
    return handleApiResponse<OrgBrandProfile | null>(response);
  },
  upsertBrandProfile: async (orgId: string, data: Partial<OrgBrandProfile>): Promise<OrgBrandProfile> => {
    const response = await makeRequest(`/api/organizations/${orgId}/brand-profile`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrgBrandProfile>(response);
  },
  triggerBrandResearch: async (orgId: string): Promise<{ orgId: string; status: string; message: string }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/brand-research`, { method: 'POST' });
    return handleApiResponse(r);
  },
  seedBrandProject: async (orgId: string) => {
    const r = await fetch(`/api/organizations/${orgId}/seed-brand-project`, {
      method: 'POST',
      credentials: 'include',
    });
    return r.json();
  },
  getBrandResearchStatus: async (orgId: string): Promise<{ orgId: string; status: string; summary?: string; ranAt?: string }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/brand-research/status`);
    return handleApiResponse(r);
  },
  generateIntakeToken: async (orgId: string): Promise<{ token: string; url: string; expiresAt: string }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/intake-token`, { method: 'POST' });
    return handleApiResponse(r);
  },
  getKnowledge: async (orgId: string): Promise<{ knowledge_entries: OrgKnowledgeSource[]; stats: { knowledge_entry_count: number; data_source_count: number; avg_coverage: number } }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/knowledge`);
    return handleApiResponse(r);
  },
};

// ============================================================================
// Intake API
// ============================================================================

export const intakeApi = {
  getContext: async (token: string): Promise<{ orgName: string; orgId: string; existing: Record<string, string | null> }> => {
    const r = await makeRequest(`/api/intake/${token}`);
    return handleApiResponse(r);
  },
  submit: async (token: string, data: Record<string, string>): Promise<{ message: string }> => {
    const r = await makeRequest(`/api/intake/${token}`, { method: 'POST', body: JSON.stringify(data) });
    return handleApiResponse(r);
  },
};

// ============================================================================
// Project Folders API
// ============================================================================

export interface ProjectFolderData {
  id: string;
  organization_id: string;
  client_id?: string;
  name: string;
  sort_order: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Knowledge API
// ============================================================================

export interface ProjectKnowledgeSource {
  id: string;
  project_id: string;
  source_type: string;
  source_id: string;
  source_title: string;
  source_summary?: string;
  coverage_score: number;
  is_active: boolean;
  is_stale: boolean;
  auto_registered: boolean;
  last_refreshed_at: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectKnowledgeCompleteness {
  project_id: string;
  total_sources: number;
  fresh_sources: number;
  avg_coverage: number;
  type_count: number;
  knowledge_completeness: number;
}

export interface ProjectKnowledgeResponse {
  project_id: string;
  completeness?: ProjectKnowledgeCompleteness;
  total_sources: number;
  stale_count: number;
  sources_by_type: Record<string, ProjectKnowledgeSource[]>;
}

export interface CreateKnowledgeSourceRequest {
  source_type: string;
  source_title: string;
  source_summary?: string;
  coverage_score?: number;
}

export const knowledgeApi = {
  getProjectKnowledge: async (projectId: string): Promise<ProjectKnowledgeResponse> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge`);
    return handleApiResponse<ProjectKnowledgeResponse>(response);
  },

  createSource: async (projectId: string, data: CreateKnowledgeSourceRequest): Promise<ProjectKnowledgeSource> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProjectKnowledgeSource>(response);
  },

  refreshSource: async (projectId: string, sourceId: string): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge/${sourceId}/refresh`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  markStale: async (projectId: string, sourceId: string): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge/${sourceId}/stale`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },
};

// projectFoldersApi removed — projects now use parent_project_id nesting via projectsApi.setParent()

// ============================================================================
// Entity Conversion API
// ============================================================================

export type EntityType = 'organization' | 'client' | 'project';

export interface ConvertEntityRequest {
  source_type: EntityType;
  source_id: string;
  target_type: EntityType;
  target_parent_id?: string;
}

export interface ConvertEntityResponse {
  new_id: string;
  new_type: EntityType;
}

export const entityConversionApi = {
  convert: async (data: ConvertEntityRequest): Promise<ConvertEntityResponse> => {
    const response = await makeRequest('/api/entities/convert', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ConvertEntityResponse>(response);
  },
};

// ============================================================================
// AGENT EXECUTION CONFIG
// ============================================================================

export type ExecutionMode = 'standard' | 'ralph' | 'parallel' | 'pipeline';
export type RalphLoopStatus = 'initializing' | 'running' | 'validating' | 'complete' | 'maxreached' | 'failed' | 'cancelled';

export interface AgentExecutionProfile {
  id: string;
  name: string;
  description: string | null;
  execution_mode: ExecutionMode;
  max_iterations: number | null;
  completion_promise: string | null;
  exit_signal_key: string | null;
  backpressure_commands: string | null;
  iteration_delay_ms: number | null;
  iteration_timeout_ms: number | null;
  total_timeout_ms: number | null;
  preserve_session: boolean | null;
}

export interface AgentExecutionConfig {
  id: string;
  agent_id: string;
  execution_profile_id: string | null;
  execution_mode_override: ExecutionMode | null;
  max_iterations_override: number | null;
  backpressure_commands_override: string | null;
  system_prompt_prefix: string | null;
  system_prompt_suffix: string | null;
  auto_commit_on_success: boolean | null;
  auto_create_pr_on_complete: boolean | null;
  require_tests_pass: boolean | null;
  is_active: boolean | null;
  created_at: string;
  updated_at: string;
  type?: 'Found';
}

export interface CreateAgentExecutionConfig {
  execution_profile_id?: string | null;
  execution_mode_override?: ExecutionMode | null;
  max_iterations_override?: number | null;
  backpressure_commands_override?: string;
  system_prompt_prefix?: string;
  system_prompt_suffix?: string;
  auto_commit_on_success?: boolean;
  auto_create_pr_on_complete?: boolean;
  require_tests_pass?: boolean;
}

export interface UpdateAgentExecutionConfig extends CreateAgentExecutionConfig {}

export interface RalphLoopState {
  id: string;
  task_attempt_id: string;
  agent_id: string | null;
  current_iteration: number;
  max_iterations: number;
  session_id: string | null;
  status: RalphLoopStatus;
  completion_promise: string | null;
  completion_detected_at: string | null;
  final_validation_passed: boolean | null;
  total_tokens_used: number | null;
  total_cost_cents: number | null;
  started_at: string;
  completed_at: string | null;
  last_iteration_at: string | null;
  last_error: string | null;
  consecutive_failures: number | null;
  type?: 'Found';
}

export interface RalphIteration {
  id: string;
  ralph_loop_id: string;
  execution_process_id: string | null;
  iteration_number: number;
  status: string;
  completion_signal_found: boolean | null;
  exit_signal_found: boolean | null;
  all_backpressure_passed: boolean | null;
  tokens_used: number | null;
  cost_cents: number | null;
  duration_ms: number | null;
  started_at: string;
  completed_at: string | null;
  output_summary: string | null;
  files_modified: number | null;
  commits_made: number | null;
}

export const agentExecutionConfigApi = {
  listProfiles: async (): Promise<AgentExecutionProfile[]> => {
    const response = await makeRequest('/api/execution-profiles');
    return handleApiResponse<AgentExecutionProfile[]>(response);
  },
  getAgentConfig: async (agentId: string): Promise<AgentExecutionConfig | { type: 'NotFound' }> => {
    const response = await makeRequest(`/api/agents/${agentId}/execution-config`);
    return handleApiResponse<AgentExecutionConfig | { type: 'NotFound' }>(response);
  },
  createAgentConfig: async (agentId: string, data: CreateAgentExecutionConfig): Promise<AgentExecutionConfig> => {
    const response = await makeRequest(`/api/agents/${agentId}/execution-config`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentExecutionConfig>(response);
  },
  updateAgentConfig: async (agentId: string, data: UpdateAgentExecutionConfig): Promise<AgentExecutionConfig> => {
    const response = await makeRequest(`/api/agents/${agentId}/execution-config`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentExecutionConfig>(response);
  },
};
