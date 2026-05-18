import { handleApiResponse, makeRequest } from './client';

// ============================================
// Social Command APIs
// ============================================

export interface SocialAccountRecord {
  id: string;
  project_id?: string | null;
  organization_id?: string | null;
  user_id?: string | null;
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
  project_id?: string | null;
  organization_id?: string | null;
  user_id?: string | null;
  social_account_id?: string | null;
  task_id?: string | null;
  deliverable_id?: string | null;
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
  review_token?: string | null;
  review_note?: string | null;
  reviewed_by?: string | null;
  publish_attempt: number;
  assignee_id?: string | null;
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
  listAccounts: async (params?: {
    projectId?: string;
    organizationId?: string;
  }): Promise<SocialAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (params?.organizationId)
      searchParams.set('organization_id', params.organizationId);
    else if (params?.projectId)
      searchParams.set('project_id', params.projectId);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/social/accounts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<SocialAccountRecord[]>(response);
  },

  listPosts: async (params?: {
    projectId?: string;
    organizationId?: string;
  }): Promise<SocialPostRecord[]> => {
    const searchParams = new URLSearchParams();
    if (params?.organizationId)
      searchParams.set('organization_id', params.organizationId);
    else if (params?.projectId)
      searchParams.set('project_id', params.projectId);
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
    organizationId?: string;
    status?: string;
    category?: string;
    platform?: string;
    limit?: number;
  }): Promise<SocialPostRecord[]> => {
    const sp = new URLSearchParams();
    if (params.organizationId) sp.set('organization_id', params.organizationId);
    else if (params.projectId) sp.set('project_id', params.projectId);
    if (params.status) sp.set('status', params.status);
    if (params.category) sp.set('category', params.category);
    if (params.platform) sp.set('platform', params.platform);
    if (params.limit) sp.set('limit', params.limit.toString());
    const response = await makeRequest(`/api/social/posts?${sp.toString()}`);
    return handleApiResponse<SocialPostRecord[]>(response);
  },

  createPost: async (data: {
    project_id?: string;
    organization_id?: string;
    caption: string;
    platforms: string[];
    content_type?: string;
    status?: string;
    scheduled_for?: string;
    category?: string;
    hashtags?: string[];
    assignee_id?: string;
  }): Promise<SocialPostRecord> => {
    const response = await makeRequest('/api/social/posts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  updatePost: async (
    id: string,
    data: Partial<{
      caption: string;
      media_urls: string[];
      hashtags: string[];
      mentions: string[];
      platforms: string[];
      platform_specific: Record<string, unknown>;
      status: string;
      scheduled_for: string | null;
      category: string;
      is_evergreen: boolean;
      recycle_after_days: number | null;
      assignee_id: string | null;
    }>
  ): Promise<SocialPostRecord> => {
    const response = await makeRequest(`/api/social/posts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  getPost: async (id: string): Promise<SocialPostRecord> => {
    const response = await makeRequest(`/api/social/posts/${id}`);
    return handleApiResponse<SocialPostRecord>(response);
  },

  listQueue: async (
    projectId: string,
    category?: string
  ): Promise<SocialPostRecord[]> => {
    const sp = new URLSearchParams({ project_id: projectId });
    if (category) sp.set('category', category);
    const response = await makeRequest(`/api/social/queue?${sp}`);
    return handleApiResponse<SocialPostRecord[]>(response);
  },

  reorderQueue: async (orderedIds: string[]): Promise<void> => {
    const response = await makeRequest('/api/social/queue/reorder', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ordered_ids: orderedIds }),
    });
    return handleApiResponse<void>(response);
  },

  getAnalytics: async (
    projectId: string,
    days?: number
  ): Promise<{
    total_impressions: number;
    total_reach: number;
    total_likes: number;
    total_comments: number;
    total_shares: number;
    total_saves: number;
    total_clicks: number;
    posts_published: number;
    avg_engagement_rate: number;
    daily: Array<{
      date: string;
      impressions: number;
      likes: number;
      comments: number;
      shares: number;
      posts: number;
    }>;
    by_platform: Array<{
      platform: string;
      posts_published: number;
      total_impressions: number;
      total_likes: number;
      avg_engagement_rate: number;
    }>;
  }> => {
    const sp = new URLSearchParams({ project_id: projectId });
    if (days) sp.set('days', String(days));
    const response = await makeRequest(`/api/social/analytics?${sp}`);
    return handleApiResponse(response);
  },

  getPostGrowth: async (
    postId: string
  ): Promise<
    Array<{
      captured_at: string;
      impressions: number;
      likes: number;
      comments: number;
      shares: number;
      engagement_rate: number;
    }>
  > => {
    const response = await makeRequest(
      `/api/social/analytics/growth?post_id=${postId}`
    );
    return handleApiResponse(response);
  },

  getTopPosts: async (params: {
    projectId?: string;
    orgId?: string;
    limit?: number;
    metric?: 'impressions' | 'likes' | 'engagement_rate' | 'comments';
    days?: number;
  }): Promise<SocialPostRecord[]> => {
    const sp = new URLSearchParams();
    if (params.projectId) sp.set('project_id', params.projectId);
    if (params.orgId) sp.set('org_id', params.orgId);
    if (params.limit) sp.set('limit', String(params.limit));
    if (params.metric) sp.set('metric', params.metric);
    if (params.days) sp.set('days', String(params.days));
    const response = await makeRequest(`/api/social/analytics/top-posts?${sp}`);
    return handleApiResponse(response);
  },

  getBestTimes: async (params: {
    projectId?: string;
    orgId?: string;
  }): Promise<
    Array<{
      day_of_week: number; // 0=Sun, 6=Sat
      hour_of_day: number; // 0-23
      post_count: number;
      avg_engagement: number;
    }>
  > => {
    const sp = new URLSearchParams();
    if (params.projectId) sp.set('project_id', params.projectId);
    if (params.orgId) sp.set('org_id', params.orgId);
    const response = await makeRequest(
      `/api/social/analytics/best-times?${sp}`
    );
    return handleApiResponse(response);
  },

  getInsights: async (params: {
    projectId?: string;
    orgId?: string;
    days?: number;
  }): Promise<{
    summary: string;
    top_finding: string;
    recommendations: string[];
    generated_at: string;
  }> => {
    const response = await makeRequest('/api/social/analytics/insights', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        project_id: params.projectId,
        org_id: params.orgId,
        days: params.days ?? 30,
      }),
    });
    return handleApiResponse(response);
  },

  connectAccount: (
    platform: string,
    params: { projectId?: string; orgId?: string }
  ) => {
    const sp = new URLSearchParams();
    if (params.orgId) sp.set('organization_id', params.orgId);
    if (params.projectId) sp.set('project_id', params.projectId);
    window.location.href = `/api/social/connect/${platform}?${sp}`;
  },

  generateCaptions: async (params: {
    platform: string;
    topic: string;
    tone?: string;
    context?: string;
    hashtags_count?: number;
  }): Promise<string[]> => {
    const response = await makeRequest('/api/social/captions/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(params),
    });
    const result = await handleApiResponse<{ captions: string[] }>(response);
    return result.captions;
  },

  deletePost: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/social/posts/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  updateMention: async (
    id: string,
    data: Partial<{
      status: string;
      priority: string;
      sentiment: string;
      reply_content: string;
      replied_by: string;
      replied_at: string;
    }>
  ): Promise<SocialMentionRecord> => {
    const response = await makeRequest(`/api/social/inbox/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialMentionRecord>(response);
  },

  updateAccount: async (
    id: string,
    data: Partial<{
      status: string;
      username: string;
      display_name: string;
      follower_count: number;
    }>
  ): Promise<SocialAccountRecord> => {
    const response = await makeRequest(`/api/social/accounts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialAccountRecord>(response);
  },

  deleteAccount: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/social/accounts/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  transitionStatus: async (
    id: string,
    status: string,
    by?: string,
    note?: string
  ): Promise<SocialPostRecord> => {
    const response = await makeRequest(`/api/social/posts/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status, by, note }),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  uploadMedia: async (params: {
    projectId: string;
    file: File;
    deliverableId?: string;
    socialPostId?: string;
    uploadedBy?: string;
  }): Promise<{
    id: string;
    public_url: string;
    filename: string;
    mime_type: string;
  }> => {
    const sp = new URLSearchParams({ project_id: params.projectId });
    if (params.deliverableId) sp.set('deliverable_id', params.deliverableId);
    if (params.socialPostId) sp.set('social_post_id', params.socialPostId);
    if (params.uploadedBy) sp.set('uploaded_by', params.uploadedBy);
    const form = new FormData();
    form.append('file', params.file);
    const response = await makeRequest(
      `/api/media/social-upload?${sp.toString()}`,
      {
        method: 'POST',
        body: form,
      }
    );
    return handleApiResponse(response);
  },
};

// =============================================================================
// Social Intelligence API
// =============================================================================

export interface ContentOpportunity {
  id: string;
  type: string;
  title: string;
  brief: string | null;
  suggested_format: string | null;
  suggested_slot: string | null;
  status: string;
  relevance_score: number;
  expires_at: string | null;
  has_draft: boolean;
  draft_post_id: string | null;
  created_at: string;
}

export interface AudienceInsight {
  type: string;
  topic: string;
  mention_count: number;
  sentiment: string | null;
  content_angle: string | null;
}

export interface ShareOfVoicePeriod {
  period_start: string;
  period_end: string;
  own_mentions: number;
  total_niche_mentions: number;
  share_of_voice_pct: string | null;
  top_keywords: unknown;
  competitors: unknown;
}

export interface TrackedEntity {
  id: string;
  type: string;
  name: string;
  instagram: string | null;
  twitter: string | null;
  linkedin: string | null;
  relevance: number;
  notes: string | null;
}

export interface AddTrackedEntityRequest {
  org_id: string;
  name: string;
  entity_type:
    | 'competitor'
    | 'kol_influencer'
    | 'thought_leader'
    | 'trade_press';
  instagram_handle?: string;
  twitter_handle?: string;
  linkedin_url?: string;
  niche_relevance?: number;
  notes?: string;
}

export const socialIntelligenceApi = {
  listOpportunities: async (params: {
    orgId: string;
    status?: string;
    opportunityType?: string;
    limit?: number;
  }): Promise<{ count: number; opportunities: ContentOpportunity[] }> => {
    const sp = new URLSearchParams();
    if (params.status) sp.set('status', params.status);
    if (params.opportunityType)
      sp.set('opportunity_type', params.opportunityType);
    if (params.limit) sp.set('limit', String(params.limit));
    const response = await makeRequest(
      `/api/organizations/${params.orgId}/intelligence/opportunities${sp.toString() ? `?${sp}` : ''}`
    );
    const raw = await handleApiResponse<Record<string, unknown>[]>(response);
    const items = Array.isArray(raw) ? raw : [];
    const opportunities: ContentOpportunity[] = items.map((r) => ({
      id: r.id as string,
      type: (r.opportunity_type ?? r.type) as string,
      title: r.title as string,
      brief: (r.brief ?? null) as string | null,
      suggested_format: (r.suggested_format ?? null) as string | null,
      suggested_slot: (r.suggested_slot ?? null) as string | null,
      status: r.status as string,
      relevance_score: (r.relevance_score ?? 0) as number,
      expires_at: (r.expires_at ?? null) as string | null,
      has_draft: !!r.draft_post_id,
      draft_post_id: (r.draft_post_id ?? null) as string | null,
      created_at: r.created_at as string,
    }));
    return { count: opportunities.length, opportunities };
  },

  approveOpportunity: async (
    _orgId: string,
    opportunityId: string,
    notes?: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/social/opportunities/${opportunityId}`,
      {
        method: 'PATCH',
        body: JSON.stringify({ status: 'approved', review_notes: notes }),
      }
    );
    return handleApiResponse(response);
  },

  rejectOpportunity: async (
    _orgId: string,
    opportunityId: string,
    reason?: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/social/opportunities/${opportunityId}`,
      {
        method: 'PATCH',
        body: JSON.stringify({ status: 'rejected', review_notes: reason }),
      }
    );
    return handleApiResponse(response);
  },

  getAudienceInsights: async (
    orgId: string,
    days?: number
  ): Promise<{ insights: AudienceInsight[] }> => {
    const sp = days ? `?period_days=${days}` : '';
    const response = await makeRequest(
      `/api/organizations/${orgId}/intelligence/audience-insights${sp}`
    );
    const raw = await handleApiResponse<Record<string, unknown>[]>(response);
    const items = Array.isArray(raw) ? raw : [];
    const insights: AudienceInsight[] = items.map((r) => ({
      type: (r.insight_type ?? r.type ?? '') as string,
      topic: (r.topic ?? '') as string,
      mention_count: (r.mention_count ?? 0) as number,
      sentiment: (r.sentiment ?? null) as string | null,
      content_angle: (r.suggested_content_angle ?? r.content_angle ?? null) as
        | string
        | null,
    }));
    return { insights };
  },

  getShareOfVoice: async (
    orgId: string,
    days?: number,
    platform?: string
  ): Promise<{ periods: ShareOfVoicePeriod[] }> => {
    const sp = new URLSearchParams();
    if (days) sp.set('period_days', String(days));
    if (platform) sp.set('platform', platform);
    const response = await makeRequest(
      `/api/organizations/${orgId}/intelligence/share-of-voice${sp.toString() ? `?${sp}` : ''}`
    );
    const raw = await handleApiResponse<Record<string, unknown>[]>(response);
    const items = Array.isArray(raw) ? raw : [];
    const periods: ShareOfVoicePeriod[] = items.map((r) => ({
      period_start: (r.period_start ?? '') as string,
      period_end: (r.period_end ?? '') as string,
      own_mentions: (r.own_mention_count ?? r.own_mentions ?? 0) as number,
      total_niche_mentions: (r.total_niche_mention_count ??
        r.total_niche_mentions ??
        0) as number,
      share_of_voice_pct: (r.share_of_voice_pct ?? null) as string | null,
      top_keywords: r.top_keywords ?? null,
      competitors: r.competitor_summary ?? r.competitors ?? null,
    }));
    return { periods };
  },

  listTrackedEntities: async (
    orgId: string,
    entityType?: string
  ): Promise<{ count: number; entities: TrackedEntity[] }> => {
    const sp = entityType ? `?entity_type=${entityType}` : '';
    const response = await makeRequest(
      `/api/organizations/${orgId}/intelligence/tracked-entities${sp}`
    );
    const raw = await handleApiResponse<Record<string, unknown>[]>(response);
    const items = Array.isArray(raw) ? raw : [];
    const entities: TrackedEntity[] = items.map((r) => ({
      id: r.id as string,
      type: (r.entity_type ?? r.type) as string,
      name: r.name as string,
      instagram: (r.instagram_handle ?? r.instagram ?? null) as string | null,
      twitter: (r.twitter_handle ?? r.twitter ?? null) as string | null,
      linkedin: (r.linkedin_url ?? r.linkedin ?? null) as string | null,
      relevance: (r.niche_relevance ?? r.relevance ?? 0.5) as number,
      notes: (r.notes ?? null) as string | null,
    }));
    return { count: entities.length, entities };
  },

  addTrackedEntity: async (
    data: AddTrackedEntityRequest
  ): Promise<{ id: string }> => {
    const response = await makeRequest(
      `/api/organizations/${data.org_id}/intelligence/tracked-entities`,
      { method: 'POST', body: JSON.stringify(data) }
    );
    return handleApiResponse(response);
  },

  deleteTrackedEntity: async (
    _orgId: string,
    entityId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/social/tracked-entities/${entityId}`,
      { method: 'DELETE' }
    );
    return handleApiResponse(response);
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
  listAccounts: async (
    projectId?: string,
    provider?: string,
    ownerType?: string,
    ownerId?: string
  ): Promise<EmailAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    if (provider) searchParams.set('provider', provider);
    if (ownerType) searchParams.set('owner_type', ownerType);
    if (ownerId) searchParams.set('owner_id', ownerId);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/email/accounts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<EmailAccountRecord[]>(response);
  },

  getAccount: async (id: string): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}`);
    return handleApiResponse<EmailAccountRecord>(response);
  },

  createAccount: async (
    data: CreateEmailAccountRequest
  ): Promise<EmailAccountRecord> => {
    const response = await makeRequest('/api/email/accounts', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  updateAccount: async (
    id: string,
    data: UpdateEmailAccountRequest
  ): Promise<EmailAccountRecord> => {
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
