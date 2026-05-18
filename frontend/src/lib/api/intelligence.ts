import type { ExecutionArtifact } from './artifacts';
import {
  ApiError,
  handleApiResponse,
  makeRequest,
  resolveApiUrl,
} from './client';

// ── Intelligence ──────────────────────────────────────────────────────────────

export interface IntelligenceStatus {
  contact_id: string;
  status: 'idle' | 'queued' | 'running' | 'done' | 'failed';
  summary?: string;
  confidence: number;
  agent?: string;
  last_run_at?: string;
}

// Automations API
export const automationsApi = {
  list: async (): Promise<unknown[]> => {
    const response = await makeRequest('/api/automations');
    return handleApiResponse<unknown[]>(response);
  },
};

export const intelligenceApi = {
  triggerResearch: async (
    contactId: string,
    opts?: { project_id?: string; agent_preference?: string }
  ): Promise<{ contact_id: string; status: string; message: string }> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/research`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(opts ?? {}),
      }
    );
    return handleApiResponse(response);
  },

  /** Trigger research for a CRM contact (canonical — no person bridge needed) */
  triggerContactResearch: async (
    contactId: string,
    opts?: { project_id?: string; agent_preference?: string }
  ): Promise<{ contact_id: string; status: string; message: string }> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/research`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(opts ?? {}),
      }
    );
    return handleApiResponse(response);
  },

  getStatus: async (contactId: string): Promise<IntelligenceStatus> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/intelligence-status`
    );
    return handleApiResponse<IntelligenceStatus>(response);
  },

  /** Get intelligence status from CRM contact (canonical) */
  getContactStatus: async (contactId: string): Promise<IntelligenceStatus> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/intelligence-status`
    );
    return handleApiResponse<IntelligenceStatus>(response);
  },

  listResearchPasses: async (contactId: string): Promise<ResearchPass[]> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/research-passes`
    );
    return handleApiResponse<ResearchPass[]>(response);
  },

  triggerNextPass: async (
    contactId: string,
    opts?: { focus?: string; project_id?: string }
  ): Promise<{
    pass_id: string;
    pass_number: number;
    focus: string;
    status: string;
  }> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/research-passes/next`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(opts ?? {}),
      }
    );
    return handleApiResponse(response);
  },

  listContactReports: async (
    contactId: string
  ): Promise<BusinessReportRecord[]> => {
    const response = await makeRequest(
      `/api/crm/contacts/${contactId}/reports`
    );
    return handleApiResponse<BusinessReportRecord[]>(response);
  },
};

export const reportsApi = {
  list: async (): Promise<BusinessReportRecord[]> => {
    const response = await makeRequest('/api/business-reports');
    return handleApiResponse<BusinessReportRecord[]>(response);
  },

  get: async (id: string): Promise<BusinessReportRecord> => {
    const response = await makeRequest(`/api/business-reports/${id}`);
    return handleApiResponse<BusinessReportRecord>(response);
  },

  patch: async (
    id: string,
    data: Partial<BusinessReportRecord>
  ): Promise<BusinessReportRecord> => {
    const response = await makeRequest(`/api/business-reports/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<BusinessReportRecord>(response);
  },

  generate: async (
    personId: string,
    reportType?: string
  ): Promise<{ status: string; person_id: string }> => {
    const response = await makeRequest('/api/business-reports/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ person_id: personId, report_type: reportType }),
    });
    return handleApiResponse(response);
  },

  approve: async (
    id: string
  ): Promise<{
    report: BusinessReportRecord;
    deal: unknown;
    proposal: unknown;
  }> => {
    const response = await makeRequest(`/api/business-reports/${id}/approve`, {
      method: 'POST',
    });
    return handleApiResponse(response);
  },

  requestRevision: async (
    id: string,
    notes: string
  ): Promise<BusinessReportRecord> => {
    const response = await makeRequest(
      `/api/business-reports/${id}/request-revision`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ notes }),
      }
    );
    return handleApiResponse<BusinessReportRecord>(response);
  },
};

export interface ResearchPass {
  id: string;
  contact_id: string;
  pass_number: number;
  research_focus: string;
  status: string;
  summary?: string;
  key_findings: string; // JSON
  confidence_delta: number;
  agent_used?: string;
  created_at: string;
  completed_at?: string;
  error?: string;
}

export interface BusinessReportRecord {
  id: string;
  person_id?: string;
  company_id?: string;
  report_type: string;
  title: string;
  status: string;
  executive_summary?: string;
  company_overview?: string;
  pain_points: string; // JSON [{point, severity}]
  opportunities: string; // JSON [{title, description, priority, estimated_value}]
  recommended_services: string; // JSON [{name, rationale, timeline}]
  next_steps: string; // JSON [{action, owner, deadline}]
  // Enhanced analytics sections
  individual_profiles: string; // JSON [{name, role, company, linkedin, summary, key_insights}]
  market_analysis?: string;
  competitor_analysis: string; // JSON [{name, website, strengths, weaknesses, threat_level}]
  target_clients?: string;
  brand_positioning?: string;
  digital_presence?: string;
  sources: string; // JSON [{title, url, excerpt}]
  full_report_md?: string;
  // CRM deal linkage + human review checkpoint
  crm_deal_id?: string;
  review_status: string; // 'pending_review' | 'approved' | 'rejected'
  reviewed_by?: string;
  reviewed_at?: string;
  review_notes?: string;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Data Sources API
// ============================================================================

export interface DataSourceRecord {
  id: string;
  organization_id?: string;
  project_id?: string;
  created_by?: string;
  title: string;
  description?: string;
  data_type: string;
  /** "file", "text", or "integration" */
  source_type: string;
  file_type?: string;
  /** Raw text content (for source_type = "text") */
  content?: string;
  file_name?: string;
  file_path?: string;
  file_size_bytes?: number;
  file_hash?: string;
  metadata: string; // JSON string
  status: string;
  processing_error?: string;
  /** Slash-delimited folder path, e.g. "Meetings/Google Meet" */
  folder: string;
  created_at: string;
  updated_at: string;
  archived_at?: string;
}

export interface CreateDataSourceRequest {
  organization_id?: string;
  project_id?: string;
  title: string;
  description?: string;
  data_type: string;
  /** "file", "text", or "integration" */
  source_type?: string;
  file_type?: string;
  /** Raw text content (for source_type = "text") */
  content?: string;
  metadata?: Record<string, unknown>;
  folder?: string;
}

export interface UpdateDataSourceRequest {
  title?: string;
  description?: string;
  data_type?: string;
  source_type?: string;
  content?: string;
  metadata?: string;
  status?: string;
  processing_error?: string;
  folder?: string;
}

export const SOURCE_TYPE_OPTIONS = [
  { value: 'text', label: 'Text (copy/paste)' },
  { value: 'file', label: 'File Upload' },
  { value: 'integration', label: 'Integration' },
] as const;

export const DATA_TYPE_OPTIONS = [
  { value: 'conversation', label: 'Conversation' },
  { value: 'document', label: 'Document' },
  { value: 'transcript', label: 'Transcript' },
  { value: 'report', label: 'Report' },
  { value: 'dataset', label: 'Dataset' },
  { value: 'media', label: 'Media' },
  { value: 'other', label: 'Other' },
] as const;

export const dataSourcesApi = {
  listAll: async (): Promise<DataSourceRecord[]> => {
    const response = await makeRequest('/api/data-sources/all');
    return handleApiResponse<DataSourceRecord[]>(response);
  },

  listByOrganization: async (orgId: string): Promise<DataSourceRecord[]> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/data-sources`
    );
    return handleApiResponse<DataSourceRecord[]>(response);
  },

  listByProject: async (projectId: string): Promise<DataSourceRecord[]> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/data-sources`
    );
    return handleApiResponse<DataSourceRecord[]>(response);
  },

  get: async (id: string): Promise<DataSourceRecord> => {
    const response = await makeRequest(`/api/data-sources/${id}`);
    return handleApiResponse<DataSourceRecord>(response);
  },

  create: async (data: CreateDataSourceRequest): Promise<DataSourceRecord> => {
    const response = await makeRequest('/api/data-sources', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<DataSourceRecord>(response);
  },

  upload: async (formData: FormData): Promise<DataSourceRecord> => {
    const response = await fetch(resolveApiUrl('/api/data-sources/upload'), {
      method: 'POST',
      body: formData,
      credentials: 'include',
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new ApiError(
        `Failed to upload data source: ${errorText}`,
        response.status,
        response
      );
    }
    const result = await response.json();
    return result.data as DataSourceRecord;
  },

  update: async (
    id: string,
    data: UpdateDataSourceRequest
  ): Promise<DataSourceRecord> => {
    const response = await makeRequest(`/api/data-sources/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<DataSourceRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/data-sources/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  getMetadataTemplate: async (
    dataType: string
  ): Promise<Record<string, unknown>> => {
    const response = await makeRequest(
      `/api/data-sources/metadata-template/${dataType}`
    );
    return handleApiResponse<Record<string, unknown>>(response);
  },

  getWorkflows: async (dataSourceId: string) => {
    const response = await makeRequest(
      `/api/data-sources/${dataSourceId}/workflows`
    );
    return handleApiResponse<WorkflowDefinition[]>(response);
  },

  runWorkflow: async (
    dataSourceId: string,
    workflowId: string,
    model?: string,
    force?: boolean
  ) => {
    const response = await makeRequest(
      `/api/data-sources/${dataSourceId}/workflows/${workflowId}/run`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model, force }),
      }
    );
    return handleApiResponse<{
      workflow_run_id: string;
      staged_records: number;
    }>(response);
  },

  getArtifacts: async (dataSourceId: string) => {
    const response = await makeRequest(
      `/api/data-sources/${dataSourceId}/artifacts`
    );
    return handleApiResponse<ExecutionArtifact[]>(response);
  },

  download: async (id: string): Promise<Blob> => {
    const response = await fetch(
      resolveApiUrl(`/api/data-sources/${id}/download`),
      {
        credentials: 'include',
      }
    );
    if (!response.ok) {
      throw new ApiError(
        `Download failed: ${response.statusText}`,
        response.status,
        response
      );
    }
    return response.blob();
  },

  downloadUrl: (id: string): string =>
    resolveApiUrl(`/api/data-sources/${id}/download`),
};

// ── Workflow types ──────────────────────────────────────────────────────────

export interface WorkflowNodePosition {
  x: number;
  y: number;
}

export interface WorkflowNode {
  id: string;
  name: string;
  type: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- dynamic workflow node params accessed by string keys
  parameters: Record<string, any>;
  position: WorkflowNodePosition;
}

export interface WorkflowConnection {
  source: string;
  target: string;
  source_output?: number;
  target_input?: number;
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  description?: string;
  nodes: WorkflowNode[];
  connections: WorkflowConnection[];
  is_system: boolean;
  owner_type: string; // "system", "organization", "user"
  owner_id?: string;
  default_model?: string;
}

export interface CreateWorkflowRequest {
  id: string;
  name: string;
  description?: string;
  nodes: WorkflowNode[];
  connections: WorkflowConnection[];
  owner_type?: string;
  owner_id?: string;
  default_model?: string;
}

export interface PreviewNodeResult {
  node_id: string;
  node_name: string;
  node_type: string;
  output: string;
  usage?: {
    model_used?: string;
    provider?: string;
    input_tokens?: number;
    output_tokens?: number;
    estimated_cost_micros?: number;
  };
}

export interface UpdateWorkflowRequest {
  name?: string;
  description?: string;
  nodes?: WorkflowNode[];
  connections?: WorkflowConnection[];
  default_model?: string;
}

export interface AvailableModel {
  id: string;
  label: string;
  is_default: boolean;
  provider: string;
  cost_per_million_input: number;
  cost_per_million_output: number;
}

export const workflowsApi = {
  listDefinitions: async (): Promise<WorkflowDefinition[]> => {
    const response = await makeRequest('/api/workflows/definitions');
    return handleApiResponse<WorkflowDefinition[]>(response);
  },

  createDefinition: async (
    data: CreateWorkflowRequest
  ): Promise<WorkflowDefinition> => {
    const response = await makeRequest('/api/workflows/definitions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowDefinition>(response);
  },

  updateDefinition: async (
    id: string,
    data: UpdateWorkflowRequest
  ): Promise<WorkflowDefinition> => {
    const response = await makeRequest(`/api/workflows/definitions/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowDefinition>(response);
  },

  deleteDefinition: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/workflows/definitions/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  previewWorkflow: async (data: {
    nodes: WorkflowNode[];
    connections: WorkflowConnection[];
    content?: string;
  }): Promise<PreviewNodeResult[]> => {
    const response = await makeRequest('/api/workflows/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PreviewNodeResult[]>(response);
  },

  listRecentArtifacts: async (organizationId?: string) => {
    const params = organizationId ? `?organization_id=${organizationId}` : '';
    const response = await makeRequest(`/api/artifacts/recent${params}`);
    return handleApiResponse<ExecutionArtifact[]>(response);
  },

  listAvailableModels: async (): Promise<AvailableModel[]> => {
    const response = await makeRequest('/api/workflows/models');
    return handleApiResponse<AvailableModel[]>(response);
  },

  listRecentRuns: async (params?: {
    workflow_id?: string;
    organization_id?: string;
    limit?: number;
  }): Promise<WorkflowRun[]> => {
    const searchParams = new URLSearchParams();
    if (params?.workflow_id)
      searchParams.set('workflow_id', params.workflow_id);
    if (params?.organization_id)
      searchParams.set('organization_id', params.organization_id);
    if (params?.limit) searchParams.set('limit', String(params.limit));
    const qs = searchParams.toString();
    const response = await makeRequest(
      `/api/workflows/runs/recent${qs ? `?${qs}` : ''}`
    );
    return handleApiResponse<WorkflowRun[]>(response);
  },

  getRunById: async (id: string): Promise<WorkflowRun> => {
    const response = await makeRequest(`/api/workflows/runs/${id}`);
    return handleApiResponse<WorkflowRun>(response);
  },

  getRunStats: async (id: string): Promise<WorkflowRunStats> => {
    const response = await makeRequest(`/api/workflows/runs/${id}/stats`);
    return handleApiResponse<WorkflowRunStats>(response);
  },
};

// ── Workflow Run types ──────────────────────────────────────────────────────

export interface WorkflowRun {
  id: string;
  workflow_id: string;
  workflow_name: string;
  data_source_id?: string;
  organization_id?: string;
  project_id?: string;
  model_used?: string;
  status: 'running' | 'completed' | 'failed';
  total_input_tokens?: number;
  total_output_tokens?: number;
  total_estimated_cost_micros?: number;
  total_records_staged?: number;
  total_records_approved?: number;
  total_records_rejected?: number;
  total_records_committed?: number;
  total_duplicates_found?: number;
  total_validation_errors?: number;
  node_count?: number;
  llm_node_count?: number;
  duration_ms?: number;
  content_hash?: string;
  started_at: string;
  completed_at?: string;
  created_at: string;
}

export interface WorkflowRunStats {
  run: WorkflowRun;
  live_counts: {
    total: number;
    approved: number;
    rejected: number;
    committed: number;
    duplicates: number;
    pending: number;
  };
  rates: {
    approval_rate: number;
    duplicate_rate: number;
  };
  cost_dollars: number;
}

// ── Workflow Trigger types ──────────────────────────────────────────────────

export interface WorkflowTrigger {
  id: string;
  workflow_id: string;
  name: string;
  enabled: boolean;
  trigger_type:
    | 'data_source_created'
    | 'data_source_updated'
    | 'schedule'
    | 'webhook';
  filter_data_source_types: string | null; // JSON array
  filter_organization_id: string | null;
  filter_project_id: string | null;
  filter_tags: string | null; // JSON array
  model_override: string | null;
  auto_approve: boolean;
  // Webhook-specific
  webhook_secret: string | null;
  webhook_url: string | null;
  // Rate limiting & retry
  cooldown_seconds: number;
  max_retries: number;
  last_error: string | null;
  retry_count: number;
  next_retry_at: string | null;
  // Metadata
  last_triggered_at: string | null;
  trigger_count: number;
  created_at: string;
  updated_at: string;
}

export interface TriggerExecution {
  id: string;
  trigger_id: string;
  workflow_run_id: string | null;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'retrying';
  started_at: string;
  completed_at: string | null;
  duration_ms: number | null;
  error: string | null;
  records_staged: number;
  source_type: string | null;
  source_id: string | null;
  metadata: string | null;
  created_at: string;
}

export interface CreateWorkflowTrigger {
  workflow_id: string;
  name: string;
  trigger_type?: string;
  filter_data_source_types?: string[];
  filter_organization_id?: string;
  filter_project_id?: string;
  filter_tags?: string[];
  model_override?: string;
  auto_approve?: boolean;
  cooldown_seconds?: number;
  max_retries?: number;
}

export interface UpdateWorkflowTrigger {
  name?: string;
  enabled?: boolean;
  trigger_type?: string;
  filter_data_source_types?: string[];
  filter_organization_id?: string;
  filter_project_id?: string;
  filter_tags?: string[];
  model_override?: string;
  auto_approve?: boolean;
  cooldown_seconds?: number;
  max_retries?: number;
}

export const triggersApi = {
  list: async (workflowId?: string): Promise<WorkflowTrigger[]> => {
    const params = workflowId
      ? `?workflow_id=${encodeURIComponent(workflowId)}`
      : '';
    const response = await makeRequest(`/api/workflows/triggers${params}`);
    return handleApiResponse<WorkflowTrigger[]>(response);
  },

  get: async (id: string): Promise<WorkflowTrigger> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}`);
    return handleApiResponse<WorkflowTrigger>(response);
  },

  create: async (data: CreateWorkflowTrigger): Promise<WorkflowTrigger> => {
    const response = await makeRequest('/api/workflows/triggers', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowTrigger>(response);
  },

  update: async (
    id: string,
    data: UpdateWorkflowTrigger
  ): Promise<WorkflowTrigger> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowTrigger>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  toggle: async (id: string): Promise<WorkflowTrigger> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}/toggle`, {
      method: 'POST',
    });
    return handleApiResponse<WorkflowTrigger>(response);
  },

  check: async (dataSourceId: string): Promise<WorkflowTrigger[]> => {
    const response = await makeRequest('/api/workflows/triggers/check', {
      method: 'POST',
      body: JSON.stringify({ data_source_id: dataSourceId }),
    });
    return handleApiResponse<WorkflowTrigger[]>(response);
  },

  listExecutions: async (triggerId: string): Promise<TriggerExecution[]> => {
    const response = await makeRequest(
      `/api/workflows/triggers/${triggerId}/executions`
    );
    return handleApiResponse<TriggerExecution[]>(response);
  },
};

// ── Schema types and API ──────────────────────────────────────────────────────

export interface FieldDef {
  type: string;
  required: boolean;
  description: string;
  enum_values?: string[];
  format?: string;
}

export interface TargetSchema {
  target_type: string;
  description: string;
  fields: Record<string, FieldDef>;
}

export const schemasApi = {
  list: async (): Promise<
    { target_type: string; description: string; icon: string }[]
  > => {
    const response = await makeRequest('/api/schemas');
    return response.json();
  },

  get: async (targetType: string): Promise<TargetSchema> => {
    const response = await makeRequest(`/api/schemas/${targetType}`);
    return response.json();
  },
};

export interface WorkflowStagingRecord {
  id: string;
  workflow_run_id: string;
  workflow_id: string;
  node_id: string;
  data_source_id: string | null;
  organization_id: string | null;
  project_id: string | null;
  target_type: 'crm_contact' | 'company' | 'crm_deal' | 'task';
  record_data: string; // JSON string
  status: 'pending_review' | 'approved' | 'rejected' | 'committed' | 'error';
  duplicate_of_id: string | null;
  duplicate_of_type: string | null;
  confidence: number | null;
  error_message: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  committed_at: string | null;
  validation_errors: string | null; // JSON array string of validation error messages, or null
  created_at: string;
  updated_at: string;
}

export interface CommitResult {
  id: string;
  target_type: string;
  created_id: string | null;
  error: string | null;
}

export interface BatchCommitResult {
  committed: number;
  errors: number;
  results: CommitResult[];
}

export const stagingApi = {
  listByRun: async (
    workflowRunId: string
  ): Promise<WorkflowStagingRecord[]> => {
    const response = await makeRequest(
      `/api/workflow-staging?workflow_run_id=${workflowRunId}`
    );
    return handleApiResponse<WorkflowStagingRecord[]>(response);
  },

  listPending: async (
    organizationId: string
  ): Promise<WorkflowStagingRecord[]> => {
    const response = await makeRequest(
      `/api/workflow-staging/pending?organization_id=${organizationId}`
    );
    return handleApiResponse<WorkflowStagingRecord[]>(response);
  },

  get: async (id: string): Promise<WorkflowStagingRecord> => {
    const response = await makeRequest(`/api/workflow-staging/${id}`);
    return handleApiResponse<WorkflowStagingRecord>(response);
  },

  update: async (
    id: string,
    data: { status?: string; record_data?: unknown }
  ): Promise<WorkflowStagingRecord> => {
    const response = await makeRequest(`/api/workflow-staging/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowStagingRecord>(response);
  },

  batchAction: async (
    ids: string[],
    action: 'approve' | 'reject'
  ): Promise<void> => {
    const response = await makeRequest('/api/workflow-staging/batch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ids, action }),
    });
    await handleApiResponse<void>(response);
  },

  commit: async (id: string): Promise<CommitResult> => {
    const response = await makeRequest(`/api/workflow-staging/${id}/commit`, {
      method: 'POST',
    });
    return handleApiResponse<CommitResult>(response);
  },

  batchCommit: async (workflowRunId: string): Promise<BatchCommitResult> => {
    const response = await makeRequest('/api/workflow-staging/batch-commit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow_run_id: workflowRunId }),
    });
    return handleApiResponse<BatchCommitResult>(response);
  },

  autoApprove: async (workflowRunId: string): Promise<{ affected: number }> => {
    const response = await makeRequest('/api/workflow-staging/auto-approve', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow_run_id: workflowRunId }),
    });
    return handleApiResponse<{ affected: number }>(response);
  },

  rejectDuplicates: async (
    workflowRunId: string
  ): Promise<{ affected: number }> => {
    const response = await makeRequest(
      '/api/workflow-staging/reject-duplicates',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ workflow_run_id: workflowRunId }),
      }
    );
    return handleApiResponse<{ affected: number }>(response);
  },
};
