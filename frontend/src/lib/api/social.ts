import { makeRequest, handleApiResponse } from './client';

// ============================================
// Social Command APIs
// ============================================

export interface SocialAccountRecord {
  id: string;
  project_id: string;
  platform: string;
  account_type: string;
  platform_account_id: string;
  username?: string | null;
  display_name?: string | null;
  profile_url?: string | null;
  avatar_url?: string | null;
  follower_count?: number | null;
  following_count?: number | null;
  post_count?: number | null;
  metadata?: string | null;
  status: string;
  last_sync_at?: string | null;
  last_error?: string | null;
  created_at: string;
  updated_at: string;
}

export interface SocialPostRecord {
  id: string;
  project_id: string;
  social_account_id?: string | null;
  task_id?: string | null;
  content_type: string;
  caption?: string | null;
  content_blocks?: string | null;
  media_urls?: string | null;
  hashtags?: string | null;
  mentions?: string | null;
  platforms: string;
  platform_specific?: string | null;
  status: string;
  scheduled_for?: string | null;
  published_at?: string | null;
  category?: string | null;
  queue_position?: number | null;
  is_evergreen: boolean;
  recycle_after_days?: number | null;
  last_recycled_at?: string | null;
  created_by_agent_id?: string | null;
  approved_by?: string | null;
  approved_at?: string | null;
  platform_post_id?: string | null;
  platform_url?: string | null;
  publish_error?: string | null;
  impressions: number;
  reach: number;
  likes: number;
  comments: number;
  shares: number;
  saves: number;
  clicks: number;
  engagement_rate: number;
  created_at: string;
  updated_at: string;
}

export interface SocialMentionRecord {
  id: string;
  social_account_id: string;
  project_id: string;
  mention_type: string;
  platform: string;
  platform_mention_id: string;
  author_username?: string | null;
  author_display_name?: string | null;
  author_avatar_url?: string | null;
  author_follower_count?: number | null;
  author_is_verified: boolean;
  content?: string | null;
  media_urls?: string | null;
  parent_post_id?: string | null;
  parent_platform_id?: string | null;
  status: string;
  sentiment?: string | null;
  priority: string;
  replied_at?: string | null;
  replied_by?: string | null;
  reply_content?: string | null;
  assigned_agent_id?: string | null;
  auto_response_sent: boolean;
  received_at: string;
  created_at: string;
  updated_at: string;
}

export interface SocialInboxStats {
  total_unread: number;
  high_priority: number;
}

export const socialApi = {
  listAccounts: async (projectId?: string): Promise<SocialAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/social/accounts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<SocialAccountRecord[]>(response);
  },

  listPosts: async (projectId?: string): Promise<SocialPostRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/social/posts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<SocialPostRecord[]>(response);
  },

  listMentions: async (
    projectId: string,
    options?: { unreadOnly?: boolean; limit?: number; accountId?: string }
  ): Promise<SocialMentionRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('project_id', projectId);
    if (options?.unreadOnly) searchParams.set('unread_only', 'true');
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    if (options?.accountId)
      searchParams.set('social_account_id', options.accountId);
    const query = searchParams.toString();
    const response = await makeRequest(`/api/social/inbox?${query}`);
    return handleApiResponse<SocialMentionRecord[]>(response);
  },

  inboxStats: async (projectId: string): Promise<SocialInboxStats> => {
    const response = await makeRequest(`/api/social/inbox/stats/${projectId}`);
    return handleApiResponse<SocialInboxStats>(response);
  },

  listPostsFiltered: async (params: {
    projectId?: string;
    status?: string;
    category?: string;
    platform?: string;
    limit?: number;
  }): Promise<SocialPostRecord[]> => {
    const sp = new URLSearchParams();
    if (params.projectId) sp.set('project_id', params.projectId);
    if (params.status) sp.set('status', params.status);
    if (params.category) sp.set('category', params.category);
    if (params.platform) sp.set('platform', params.platform);
    if (params.limit) sp.set('limit', params.limit.toString());
    const response = await makeRequest(`/api/social/posts?${sp.toString()}`);
    return handleApiResponse<SocialPostRecord[]>(response);
  },

  createPost: async (data: {
    project_id: string;
    caption: string;
    platforms: string;
    content_type?: string;
    status?: string;
    scheduled_for?: string;
    category?: string;
    hashtags?: string;
  }): Promise<SocialPostRecord> => {
    const response = await makeRequest('/api/social/posts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  updatePost: async (id: string, data: Partial<{
    caption: string;
    status: string;
    scheduled_for: string | null;
    category: string;
    platforms: string;
  }>): Promise<SocialPostRecord> => {
    const response = await makeRequest(`/api/social/posts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  deletePost: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/social/posts/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  updateMention: async (id: string, data: Partial<{
    status: string;
    priority: string;
    sentiment: string;
    reply_content: string;
    replied_by: string;
    replied_at: string;
  }>): Promise<SocialMentionRecord> => {
    const response = await makeRequest(`/api/social/inbox/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialMentionRecord>(response);
  },

  updateAccount: async (id: string, data: Partial<{
    status: string;
    username: string;
    display_name: string;
    follower_count: number;
  }>): Promise<SocialAccountRecord> => {
    const response = await makeRequest(`/api/social/accounts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialAccountRecord>(response);
  },

  deleteAccount: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/social/accounts/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// =============================================================================
// Email Account Records
// =============================================================================

export interface EmailAccountRecord {
  id: string;
  project_id: string;
  provider: string;
  account_type: string;
  email_address: string;
  display_name: string | null;
  avatar_url: string | null;
  granted_scopes: string | null;
  storage_used_bytes: number | null;
  storage_total_bytes: number | null;
  unread_count: number | null;
  status: string;
  last_sync_at: string | null;
  last_error: string | null;
  sync_enabled: number | null;
  sync_frequency_minutes: number | null;
  auto_reply_enabled: number | null;
  signature: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateEmailAccountRequest {
  project_id: string;
  provider: string;
  account_type?: string;
  email_address: string;
  display_name?: string;
  avatar_url?: string;
  access_token?: string;
  refresh_token?: string;
  token_expires_at?: string;
  imap_host?: string;
  imap_port?: number;
  smtp_host?: string;
  smtp_port?: number;
  use_ssl?: boolean;
  granted_scopes?: string[];
  metadata?: Record<string, unknown>;
}

export interface UpdateEmailAccountRequest {
  display_name?: string;
  avatar_url?: string;
  status?: string;
  sync_enabled?: boolean;
  sync_frequency_minutes?: number;
  auto_reply_enabled?: boolean;
  signature?: string;
}

export interface OAuthUrlResponse {
  auth_url: string;
  state: string;
}

export const emailApi = {
  listAccounts: async (projectId?: string, provider?: string, ownerType?: string, ownerId?: string): Promise<EmailAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    if (provider) searchParams.set('provider', provider);
    if (ownerType) searchParams.set('owner_type', ownerType);
    if (ownerId) searchParams.set('owner_id', ownerId);
    const query = searchParams.toString();
    const response = await makeRequest(`/api/email/accounts${query ? `?${query}` : ''}`);
    return handleApiResponse<EmailAccountRecord[]>(response);
  },

  getAccount: async (id: string): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}`);
    return handleApiResponse<EmailAccountRecord>(response);
  },

  createAccount: async (data: CreateEmailAccountRequest): Promise<EmailAccountRecord> => {
    const response = await makeRequest('/api/email/accounts', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  updateAccount: async (id: string, data: UpdateEmailAccountRequest): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  deleteAccount: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/accounts/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  triggerSync: async (id: string): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}/sync`, {
      method: 'POST',
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  initiateOAuth: async (
    projectId: string | null,
    provider: string,
    redirectUri: string,
    ownerType?: string,
    ownerId?: string
  ): Promise<OAuthUrlResponse> => {
    const response = await makeRequest('/api/email/oauth/initiate', {
      method: 'POST',
      body: JSON.stringify({
        ...(projectId ? { project_id: projectId } : {}),
        ...(ownerType ? { owner_type: ownerType } : {}),
        ...(ownerId ? { owner_id: ownerId } : {}),
        provider,
        redirect_uri: redirectUri,
      }),
    });
    return handleApiResponse<OAuthUrlResponse>(response);
  },
};
