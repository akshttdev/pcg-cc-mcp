import type {
  AirtableBase,
  CreateAirtableBase,
  UpdateAirtableBase,
  AirtableRecordLink,
  AirtableBaseInfo,
  AirtableTable,
  AirtableRecord,
  AirtableConnectionWithBase,
  AirtableVerifyRequest,
  AirtableVerifyResponse,
  AirtableImportRequest,
  AirtableImportResult,
  AirtablePushTaskRequest,
} from 'shared/types';
import { makeRequest, handleApiResponse } from './client';

export const airtableApi = {
  // Verify Airtable Personal Access Token
  verifyCredentials: async (
    credentials: AirtableVerifyRequest
  ): Promise<AirtableVerifyResponse> => {
    const response = await makeRequest('/api/airtable/verify', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
    return handleApiResponse<AirtableVerifyResponse>(response);
  },

  // List user's Airtable bases (from Airtable API)
  listUserBases: async (): Promise<AirtableBaseInfo[]> => {
    const response = await makeRequest('/api/airtable/bases');
    return handleApiResponse<AirtableBaseInfo[]>(response);
  },

  // List base connections (from our DB)
  listConnections: async (
    projectId?: string
  ): Promise<AirtableBase[]> => {
    const url = projectId
      ? `/api/airtable/connections?project_id=${projectId}`
      : '/api/airtable/connections';
    const response = await makeRequest(url);
    return handleApiResponse<AirtableBase[]>(response);
  },

  // Get a single connection with base info
  getConnection: async (
    connectionId: string
  ): Promise<AirtableConnectionWithBase> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}`
    );
    return handleApiResponse<AirtableConnectionWithBase>(response);
  },

  // Create a base connection
  createConnection: async (
    connection: CreateAirtableBase
  ): Promise<AirtableBase> => {
    const response = await makeRequest('/api/airtable/connections', {
      method: 'POST',
      body: JSON.stringify(connection),
    });
    return handleApiResponse<AirtableBase>(response);
  },

  // Update a base connection
  updateConnection: async (
    connectionId: string,
    update: UpdateAirtableBase
  ): Promise<AirtableBase> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}`,
      {
        method: 'PATCH',
        body: JSON.stringify(update),
      }
    );
    return handleApiResponse<AirtableBase>(response);
  },

  // Delete a base connection
  deleteConnection: async (connectionId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}`,
      {
        method: 'DELETE',
      }
    );
    return handleApiResponse<void>(response);
  },

  // Get tables in a connected base
  getBaseTables: async (connectionId: string): Promise<AirtableTable[]> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}/tables`
    );
    return handleApiResponse<AirtableTable[]>(response);
  },

  // Get records from a table in a connected base
  getTableRecords: async (
    connectionId: string,
    tableId: string
  ): Promise<AirtableRecord[]> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}/records?table_id=${tableId}`
    );
    return handleApiResponse<AirtableRecord[]>(response);
  },

  // Import records from an Airtable table as PCG tasks
  importRecords: async (
    connectionId: string,
    request: AirtableImportRequest
  ): Promise<AirtableImportResult> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}/import`,
      {
        method: 'POST',
        body: JSON.stringify(request),
      }
    );
    return handleApiResponse<AirtableImportResult>(response);
  },

  // Get Airtable link for a task
  getTaskLink: async (taskId: string): Promise<AirtableRecordLink | null> => {
    const response = await makeRequest(`/api/airtable/tasks/${taskId}/link`);
    return handleApiResponse<AirtableRecordLink | null>(response);
  },

  // Push a PCG task to Airtable
  pushTaskToAirtable: async (
    taskId: string,
    request: AirtablePushTaskRequest
  ): Promise<AirtableRecordLink> => {
    const response = await makeRequest(`/api/airtable/tasks/${taskId}/push`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return handleApiResponse<AirtableRecordLink>(response);
  },

  // Sync task deliverables to Airtable as a comment
  syncDeliverables: async (taskId: string): Promise<AirtableRecordLink> => {
    const response = await makeRequest(
      `/api/airtable/tasks/${taskId}/sync-deliverables`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<AirtableRecordLink>(response);
  },
};
