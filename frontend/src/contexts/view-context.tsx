/**
 * ViewContext — manages the "view-as" role override.
 *
 * This context does NOT depend on useEffectiveRole (which depends on us).
 * It computes available roles directly from auth state to avoid circular deps.
 */
import { createContext, useContext, useState, useEffect, useRef, useMemo, type ReactNode } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useOrganization } from '@/contexts/organization-context';
import {
  type EffectiveRole,
  ROLE_LEVEL,
  rolesAtOrBelow,
  groupRoles,
  mapOrgRole,
  type RoleGroup,
} from '@/lib/roles';

const STORAGE_KEY = 'pcg:view-as-role';

interface ViewContextValue {
  /** The currently selected view-as role, or null for natural role. */
  viewAsRole: EffectiveRole | null;
  /** Set a view-as override. */
  setViewAsRole: (role: EffectiveRole | null) => void;
  /** Clear the override back to natural role. */
  clearViewAs: () => void;
  /** Whether a view-as override is currently active. */
  isOverridden: boolean;
  /** Roles available for view-as (at or below user's natural role), grouped by tier. */
  availableRoleGroups: { group: RoleGroup; label: string; roles: EffectiveRole[] }[];
  /** Flat list of available roles. */
  availableRoles: EffectiveRole[];
  /** The user's natural (non-overridden) role. */
  naturalRole: EffectiveRole;
}

const ViewContext = createContext<ViewContextValue | undefined>(undefined);

/** Compute the natural role from auth state (mirrors useEffectiveRole logic but without the override). */
function computeNaturalRoleFromAuth(
  user: { is_admin: boolean } | null,
  hasRole: (role: any) => boolean,
  orgRole: 'admin' | 'member' | 'viewer' | null,
): EffectiveRole {
  if (!user) return 'authenticated';

  const isPlatformAdmin = user.is_admin || hasRole('platform_admin');
  const isOperator = hasRole('operator');
  const isClientUser = hasRole('client_user');

  if (isPlatformAdmin) return 'platform_admin';
  if (isOperator) return 'platform_member';
  if (orgRole) return mapOrgRole(orgRole);
  if (isClientUser) return 'client_editor';
  return 'authenticated';
}

export function ViewContextProvider({ children }: { children: ReactNode }) {
  const { user, hasRole } = useAuth();
  const { effectiveOrgId } = useOrganization();

  // Compute natural role
  const orgRole = useMemo(() => {
    if (!effectiveOrgId || !user) return null;
    return user.organizations.find((o) => o.id === effectiveOrgId)?.role ?? null;
  }, [user, effectiveOrgId]);

  const naturalRole = useMemo(
    () => computeNaturalRoleFromAuth(user, hasRole, orgRole),
    [user, hasRole, orgRole],
  );

  // Initialize from localStorage
  const [viewAsRole, setViewAsRoleState] = useState<EffectiveRole | null>(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored && stored in ROLE_LEVEL) {
        const parsed = stored as EffectiveRole;
        // Only restore if it's lower than natural (will be validated in effect)
        return parsed;
      }
    } catch { /* ignore */ }
    return null;
  });

  // Clear override if user changes or if stored role is >= natural role
  // Skip check while user is still loading (naturalRole would be 'authenticated')
  useEffect(() => {
    if (!user) return;
    if (viewAsRole && ROLE_LEVEL[viewAsRole] >= ROLE_LEVEL[naturalRole]) {
      setViewAsRoleState(null);
      localStorage.removeItem(STORAGE_KEY);
    }
  }, [user, naturalRole, viewAsRole]);

  // Clear on logout (user transitions from non-null to null)
  const prevUserRef = useRef(user);
  useEffect(() => {
    if (prevUserRef.current && !user) {
      // User was logged in and is now logged out
      setViewAsRoleState(null);
      localStorage.removeItem(STORAGE_KEY);
    }
    prevUserRef.current = user;
  }, [user]);

  const setViewAsRole = (role: EffectiveRole | null) => {
    setViewAsRoleState(role);
    if (role) {
      localStorage.setItem(STORAGE_KEY, role);
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  };

  const clearViewAs = () => setViewAsRole(null);

  const isOverridden = viewAsRole !== null && ROLE_LEVEL[viewAsRole] < ROLE_LEVEL[naturalRole];

  const availableRoles = useMemo(
    () => rolesAtOrBelow(naturalRole).filter((r) => r !== naturalRole),
    [naturalRole],
  );

  const availableRoleGroups = useMemo(
    () => groupRoles(availableRoles),
    [availableRoles],
  );

  const value: ViewContextValue = {
    viewAsRole: isOverridden ? viewAsRole : null,
    setViewAsRole,
    clearViewAs,
    isOverridden,
    availableRoleGroups,
    availableRoles,
    naturalRole,
  };

  return <ViewContext.Provider value={value}>{children}</ViewContext.Provider>;
}

export function useViewContext(): ViewContextValue {
  const context = useContext(ViewContext);
  if (context === undefined) {
    throw new Error('useViewContext must be used within a ViewContextProvider');
  }
  return context;
}
