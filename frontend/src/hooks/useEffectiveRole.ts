import { useMemo } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useOrganization } from '@/contexts/organization-context';
import { useViewContext } from '@/contexts/view-context';
import {
  type EffectiveRole,
  ROLE_LEVEL,
  mapOrgRole,
} from '@/lib/roles';

// Re-export for consumers that import from here
export type { EffectiveRole } from '@/lib/roles';

export interface EffectiveRoleInfo {
  /** The effective role (may be overridden by view-as). */
  role: EffectiveRole;
  /** The user's actual role before any view-as override. */
  actualRole: EffectiveRole;
  /** Whether the role is currently overridden by view-as. */
  isOverridden: boolean;
  /** User can see admin platform tools (Site Directory, Nora, Mission Control, etc.) */
  canSeeAdminPlatforms: boolean;
  /** User can see management section (People, Companies, Proposals, Invoices, etc.) */
  canSeeManagement: boolean;
  /** User can see global cross-org views (All Tasks, CRM Admin, All Social) */
  canSeeGlobalViews: boolean;
  /** User can see creative tools (VIBELAND, VIBE) */
  canSeeCreativeTools: boolean;
  /** User can see the org tree with CRM/Social/Intelligence sub-nav */
  canSeeOrgTree: boolean;
  /** User can manage org settings (members, integrations) */
  canManageOrg: boolean;
  /** User can create projects and orgs */
  canCreate: boolean;
  /** The user's role in the active org (if any) */
  orgRole: 'admin' | 'member' | 'viewer' | null;
  /** The user's role in the active client (if any) — future: resolve from client context */
  clientRole: 'admin' | 'editor' | 'viewer' | null;
}

/** Compute the natural (non-overridden) effective role from auth state. */
export function computeNaturalRole(
  user: { is_admin: boolean; platform_roles?: string[] } | null,
  hasRole: (role: any) => boolean,
  orgRole: 'admin' | 'member' | 'viewer' | null,
): EffectiveRole {
  if (!user) return 'authenticated';

  const isPlatformAdmin = user.is_admin || hasRole('platform_admin');
  const isOperator = hasRole('operator');
  const isClientUser = hasRole('client_user');

  if (isPlatformAdmin) return 'platform_admin';
  if (isOperator) return 'platform_member';

  if (orgRole) {
    return mapOrgRole(orgRole);
  }

  if (isClientUser) return 'client_editor';
  return 'authenticated';
}

/** Compute capability flags from an effective role. */
function computeCapabilities(role: EffectiveRole, orgRole: 'admin' | 'member' | 'viewer' | null) {
  const level = ROLE_LEVEL[role];
  return {
    canSeeAdminPlatforms: level >= ROLE_LEVEL.platform_admin,
    canSeeManagement: level >= ROLE_LEVEL.platform_member,
    canSeeGlobalViews: level >= ROLE_LEVEL.platform_admin,
    canSeeCreativeTools: level >= ROLE_LEVEL.org_editor,
    canSeeOrgTree: level >= ROLE_LEVEL.org_viewer || orgRole !== null,
    canManageOrg: level >= ROLE_LEVEL.org_admin,
    canCreate: level >= ROLE_LEVEL.org_editor,
  };
}

export function useEffectiveRole(): EffectiveRoleInfo {
  const { user, hasRole } = useAuth();
  const { effectiveOrgId } = useOrganization();
  const { viewAsRole } = useViewContext();

  return useMemo(() => {
    const activeOrgMembership = effectiveOrgId
      ? user?.organizations.find((o) => o.id === effectiveOrgId)
      : null;
    const orgRole = activeOrgMembership?.role ?? null;

    // TODO: resolve clientRole from client context when available
    const clientRole: 'admin' | 'editor' | 'viewer' | null = null;

    const actualRole = computeNaturalRole(user, hasRole, orgRole);

    // Apply view-as override only if it's lower than actual role
    const isOverridden = viewAsRole !== null && ROLE_LEVEL[viewAsRole] < ROLE_LEVEL[actualRole];
    const role = isOverridden ? viewAsRole! : actualRole;

    const capabilities = computeCapabilities(role, isOverridden ? null : orgRole);

    return {
      role,
      actualRole,
      isOverridden,
      ...capabilities,
      orgRole,
      clientRole,
    };
  }, [user, hasRole, effectiveOrgId, viewAsRole]);
}
