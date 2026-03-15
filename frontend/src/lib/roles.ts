/**
 * Shared role hierarchy, types, and permission definitions.
 *
 * Role levels represent editorial scope at each tier:
 *   admin  = invite + edit
 *   editor = edit (org "member" in DB maps here)
 *   viewer = read-only
 */

// ─── Effective Role ─────────────────────────────────────────────────────────

export type EffectiveRole =
  | 'platform_admin'
  | 'platform_member'
  | 'org_admin'
  | 'org_editor'
  | 'org_viewer'
  | 'client_admin'
  | 'client_editor'
  | 'client_viewer'
  | 'project_member'
  | 'authenticated';

export const ROLE_LEVEL: Record<EffectiveRole, number> = {
  platform_admin: 11,
  platform_member: 10,
  org_admin: 9,
  org_editor: 8,
  org_viewer: 7,
  client_admin: 6,
  client_editor: 5,
  client_viewer: 4,
  project_member: 3,
  authenticated: 1,
};

export const ROLE_LABELS: Record<EffectiveRole, string> = {
  platform_admin: 'Platform Admin',
  platform_member: 'Platform Member',
  org_admin: 'Org Admin',
  org_editor: 'Org Editor',
  org_viewer: 'Org Viewer',
  client_admin: 'Client Admin',
  client_editor: 'Client Editor',
  client_viewer: 'Client Viewer',
  project_member: 'Project Member',
  authenticated: 'Authenticated',
};

/** True when `current` role is at least as high as `required`. */
export function hasMinRole(current: EffectiveRole, required: EffectiveRole): boolean {
  return ROLE_LEVEL[current] >= ROLE_LEVEL[required];
}

/** Returns all roles at or below the given role (for view-as selector). */
export function rolesAtOrBelow(role: EffectiveRole): EffectiveRole[] {
  const level = ROLE_LEVEL[role];
  return (Object.keys(ROLE_LEVEL) as EffectiveRole[])
    .filter((r) => ROLE_LEVEL[r] <= level)
    .sort((a, b) => ROLE_LEVEL[b] - ROLE_LEVEL[a]);
}

// ─── Role grouping for view-as switcher ─────────────────────────────────────

export type RoleGroup = 'platform' | 'organization' | 'client' | 'other';

export const ROLE_GROUP: Record<EffectiveRole, RoleGroup> = {
  platform_admin: 'platform',
  platform_member: 'platform',
  org_admin: 'organization',
  org_editor: 'organization',
  org_viewer: 'organization',
  client_admin: 'client',
  client_editor: 'client',
  client_viewer: 'client',
  project_member: 'other',
  authenticated: 'other',
};

export const ROLE_GROUP_LABELS: Record<RoleGroup, string> = {
  platform: 'Platform',
  organization: 'Organization',
  client: 'Client',
  other: 'Other',
};

/** Group roles by their tier for the view-as switcher UI. */
export function groupRoles(roles: EffectiveRole[]): { group: RoleGroup; label: string; roles: EffectiveRole[] }[] {
  const groups: RoleGroup[] = ['platform', 'organization', 'client', 'other'];
  return groups
    .map((group) => ({
      group,
      label: ROLE_GROUP_LABELS[group],
      roles: roles.filter((r) => ROLE_GROUP[r] === group),
    }))
    .filter((g) => g.roles.length > 0);
}

// ─── DB value mapping ───────────────────────────────────────────────────────

/** Map platform_roles DB values to EffectiveRole. */
export function mapPlatformRole(dbRole: string): 'platform_admin' | 'platform_member' {
  switch (dbRole) {
    case 'platform_admin':
      return 'platform_admin';
    case 'operator':
      return 'platform_member';
    default:
      return 'platform_member';
  }
}

/** Map organization_members.role DB values to EffectiveRole. */
export function mapOrgRole(dbRole: string): 'org_admin' | 'org_editor' | 'org_viewer' {
  switch (dbRole) {
    case 'admin':
      return 'org_admin';
    case 'member':
      return 'org_editor';
    case 'viewer':
      return 'org_viewer';
    default:
      return 'org_viewer';
  }
}

/** Map client_members.role DB values to EffectiveRole. */
export function mapClientRole(dbRole: string): 'client_admin' | 'client_editor' | 'client_viewer' {
  switch (dbRole) {
    case 'admin':
      return 'client_admin';
    case 'editor':
      return 'client_editor';
    case 'viewer':
      return 'client_viewer';
    default:
      return 'client_viewer';
  }
}

// ─── Settings scope ─────────────────────────────────────────────────────────

export type SettingsScope = 'user' | 'system' | 'org' | 'client';

export const SETTINGS_SCOPE_LABELS: Record<SettingsScope, string> = {
  user: 'User',
  system: 'Admin',
  org: 'Org',
  client: 'Client',
};

// ─── Granular permissions (types only — no enforcement yet) ─────────────────

export type Permission =
  // Platform-level
  | 'platform.users.manage'
  | 'platform.orgs.manage'
  | 'platform.projects.manage'
  | 'platform.settings.view'
  | 'platform.developer.access'
  // Org-level
  | 'org.members.invite'
  | 'org.members.manage'
  | 'org.settings.edit'
  | 'org.agents.manage'
  | 'org.models.manage'
  | 'org.mcp.manage'
  | 'org.keys.manage'
  | 'org.network.manage'
  | 'org.content.edit'
  | 'org.content.view'
  // Client-level
  | 'client.members.invite'
  | 'client.members.manage'
  | 'client.settings.edit'
  | 'client.content.edit'
  | 'client.content.view'
  | 'client.pulse.view'
  // Project-level
  | 'project.members.manage'
  | 'project.content.edit'
  | 'project.content.view';

export type RolePermissionMap = Record<EffectiveRole, readonly Permission[]>;

export const DEFAULT_ROLE_PERMISSIONS: RolePermissionMap = {
  platform_admin: [
    'platform.users.manage', 'platform.orgs.manage', 'platform.projects.manage',
    'platform.settings.view', 'platform.developer.access',
    'org.members.invite', 'org.members.manage', 'org.settings.edit',
    'org.agents.manage', 'org.models.manage', 'org.mcp.manage', 'org.keys.manage', 'org.network.manage',
    'org.content.edit', 'org.content.view',
    'client.members.invite', 'client.members.manage', 'client.settings.edit',
    'client.content.edit', 'client.content.view', 'client.pulse.view',
    'project.members.manage', 'project.content.edit', 'project.content.view',
  ],
  platform_member: [
    'platform.settings.view',
    'org.members.invite', 'org.settings.edit',
    'org.agents.manage', 'org.models.manage', 'org.mcp.manage', 'org.keys.manage', 'org.network.manage',
    'org.content.edit', 'org.content.view',
    'client.content.edit', 'client.content.view', 'client.pulse.view',
    'project.members.manage', 'project.content.edit', 'project.content.view',
  ],
  org_admin: [
    'org.members.invite', 'org.members.manage', 'org.settings.edit',
    'org.agents.manage', 'org.models.manage', 'org.mcp.manage', 'org.keys.manage', 'org.network.manage',
    'org.content.edit', 'org.content.view',
    'client.members.invite', 'client.members.manage', 'client.settings.edit',
    'client.content.edit', 'client.content.view', 'client.pulse.view',
    'project.members.manage', 'project.content.edit', 'project.content.view',
  ],
  org_editor: [
    'org.content.edit', 'org.content.view',
    'org.agents.manage', 'org.models.manage', 'org.keys.manage',
    'client.content.edit', 'client.content.view', 'client.pulse.view',
    'project.content.edit', 'project.content.view',
  ],
  org_viewer: [
    'org.content.view',
    'client.content.view', 'client.pulse.view',
    'project.content.view',
  ],
  client_admin: [
    'client.members.invite', 'client.members.manage', 'client.settings.edit',
    'client.content.edit', 'client.content.view', 'client.pulse.view',
    'project.content.edit', 'project.content.view',
  ],
  client_editor: [
    'client.content.edit', 'client.content.view', 'client.pulse.view',
    'project.content.edit', 'project.content.view',
  ],
  client_viewer: [
    'client.content.view', 'client.pulse.view',
    'project.content.view',
  ],
  project_member: [
    'project.content.edit', 'project.content.view',
  ],
  authenticated: [
    'project.content.view',
  ],
} as const;
