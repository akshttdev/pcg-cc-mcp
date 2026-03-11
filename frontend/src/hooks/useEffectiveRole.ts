import { useMemo } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useOrganization } from '@/contexts/organization-context';

/**
 * Effective navigation role — determines what UI sections a user should see.
 * Priority (highest to lowest):
 *   platform_admin → full admin nav
 *   operator       → operational nav (management, no admin platforms)
 *   org_admin      → org management for active org
 *   org_member     → workspace + org content
 *   org_viewer     → read-only org content
 *   client_user    → minimal client nav (their projects/deliverables only)
 *   authenticated  → basic workspace only
 */
export type EffectiveRole =
  | 'platform_admin'
  | 'operator'
  | 'org_admin'
  | 'org_member'
  | 'org_viewer'
  | 'client_user'
  | 'authenticated';

export interface EffectiveRoleInfo {
  role: EffectiveRole;
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
}

export function useEffectiveRole(): EffectiveRoleInfo {
  const { user, hasRole } = useAuth();
  const { effectiveOrgId } = useOrganization();

  return useMemo(() => {
    if (!user) {
      return {
        role: 'authenticated' as const,
        canSeeAdminPlatforms: false,
        canSeeManagement: false,
        canSeeGlobalViews: false,
        canSeeCreativeTools: false,
        canSeeOrgTree: false,
        canManageOrg: false,
        canCreate: false,
        orgRole: null,
      };
    }

    const isPlatformAdmin = user.is_admin || hasRole('platform_admin');
    const isOperator = hasRole('operator');
    const isClientUser = hasRole('client_user');

    // Find user's role in the active org
    const activeOrgMembership = effectiveOrgId
      ? user.organizations.find((o) => o.id === effectiveOrgId)
      : null;
    const orgRole = activeOrgMembership?.role ?? null;

    // Determine effective role (priority order)
    let role: EffectiveRole;
    if (isPlatformAdmin) {
      role = 'platform_admin';
    } else if (isOperator) {
      role = 'operator';
    } else if (orgRole === 'admin') {
      role = 'org_admin';
    } else if (orgRole === 'member') {
      role = 'org_member';
    } else if (orgRole === 'viewer') {
      role = 'org_viewer';
    } else if (isClientUser) {
      role = 'client_user';
    } else {
      role = 'authenticated';
    }

    return {
      role,
      canSeeAdminPlatforms: isPlatformAdmin,
      canSeeManagement: isPlatformAdmin || isOperator,
      canSeeGlobalViews: isPlatformAdmin,
      canSeeCreativeTools: isPlatformAdmin || isOperator || orgRole === 'admin' || orgRole === 'member',
      canSeeOrgTree: orgRole !== null || isPlatformAdmin || isOperator,
      canManageOrg: isPlatformAdmin || orgRole === 'admin',
      canCreate: isPlatformAdmin || isOperator || orgRole === 'admin' || orgRole === 'member',
      orgRole,
    };
  }, [user, hasRole, effectiveOrgId]);
}
