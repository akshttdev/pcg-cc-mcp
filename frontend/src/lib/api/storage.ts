import { handleApiResponse, makeRequest } from './client';

// Mirrors `crates/server/src/routes/storage.rs::AccountView` — keep in sync.
export interface StorageAccountView {
  id: string;
  provider: 'onedrive' | 'dropbox' | 'gdrive';
  account_email: string | null;
  display_name: string | null;
  status: 'active' | 'syncing' | 'expired' | 'error' | 'disconnected';
  auto_sync: boolean;
  sync_interval_secs: number;
  sync_root_path: string | null;
  last_sync_at: string | null;
  last_error: string | null;
  total_files_synced: number;
  created_at: string;
}

export interface StorageSyncResult {
  files_added: number;
  files_updated: number;
  files_deleted: number;
  folders_seen: number;
}

export interface UpdateStorageAccountInput {
  auto_sync?: boolean;
  sync_interval_secs?: number;
  sync_root_path?: string;
}

export interface ConnectStorageParams {
  provider: 'onedrive' | 'dropbox' | 'gdrive';
  organization_id: string;
  project_id?: string;
  redirect_after?: string;
  sync_root_path?: string;
}

export const storageApi = {
  /**
   * Build the URL the browser should navigate to in order to start an OAuth
   * flow with a provider. The backend persists a CSRF state row, then 302s
   * to the provider's authorize endpoint.
   */
  connectUrl: ({
    provider,
    organization_id,
    project_id,
    redirect_after,
    sync_root_path,
  }: ConnectStorageParams): string => {
    const qs = new URLSearchParams({ organization_id });
    if (project_id) qs.set('project_id', project_id);
    if (redirect_after) qs.set('redirect_after', redirect_after);
    if (sync_root_path) qs.set('sync_root_path', sync_root_path);
    return `/api/storage/connect/${provider}?${qs}`;
  },

  list: async (organization_id: string): Promise<StorageAccountView[]> => {
    const qs = new URLSearchParams({ organization_id });
    const res = await makeRequest(`/api/storage/accounts?${qs}`);
    return handleApiResponse<StorageAccountView[]>(res);
  },

  disconnect: async (id: string): Promise<void> => {
    const res = await makeRequest(`/api/storage/accounts/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(res);
  },

  syncNow: async (id: string): Promise<StorageSyncResult> => {
    const res = await makeRequest(`/api/storage/accounts/${id}/sync`, {
      method: 'POST',
    });
    return handleApiResponse<StorageSyncResult>(res);
  },

  update: async (
    id: string,
    payload: UpdateStorageAccountInput
  ): Promise<StorageAccountView> => {
    const res = await makeRequest(`/api/storage/accounts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    return handleApiResponse<StorageAccountView>(res);
  },
};
