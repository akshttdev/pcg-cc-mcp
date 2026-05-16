import { useQuery } from '@tanstack/react-query';
import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
} from 'react';
import { useLocation, useParams } from 'react-router-dom';

import { useOrganizationById } from '@/hooks/queries';
import type { OrganizationData } from '@/lib/api';
import { organizationsApi } from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';

import { useAuth } from './AuthContext';

const LAST_ORG_STORAGE_KEY = 'orcha:last_org_id';

interface OrganizationContextValue {
  /** orgId extracted from the current URL (undefined if not on an org route) */
  orgId: string | undefined;
  /** The effective org: URL orgId → user home org → first org. Mirrors sidebar logic. */
  effectiveOrgId: string | undefined;
  organization: OrganizationData | undefined;
  organizations: OrganizationData[];
  isLoading: boolean;
}

const OrganizationContext = createContext<OrganizationContextValue | null>(
  null
);

interface OrganizationProviderProps {
  children: ReactNode;
}

export function OrganizationProvider({ children }: OrganizationProviderProps) {
  const { orgId: routeOrgId } = useParams<{ orgId?: string }>();
  const location = useLocation();
  const { user } = useAuth();

  // Extract orgId from URL if on an org route
  const orgId = useMemo(() => {
    if (routeOrgId) return routeOrgId;
    const match = location.pathname.match(/^\/organizations\/([^/]+)/);
    return match ? match[1] : undefined;
  }, [routeOrgId, location.pathname]);

  // Persist the org from the URL so non-org routes (like /settings) can
  // restore the user's last-selected org instead of resetting to the home org.
  useEffect(() => {
    if (orgId) {
      try {
        localStorage.setItem(LAST_ORG_STORAGE_KEY, orgId);
      } catch {
        // localStorage may be unavailable (Safari private mode, etc.) — ignore
      }
    }
  }, [orgId]);

  // Effective org: URL org → last-selected (localStorage) → home org → first org.
  // We trust the localStorage value if it's a syntactically valid UUID; the
  // backend will reject any API call where the user isn't actually a member.
  const effectiveOrgId = useMemo(() => {
    if (orgId) return orgId;
    let lastOrgId: string | null = null;
    try {
      lastOrgId = localStorage.getItem(LAST_ORG_STORAGE_KEY);
    } catch {
      lastOrgId = null;
    }
    const UUID_RE =
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    if (lastOrgId && UUID_RE.test(lastOrgId)) {
      return lastOrgId;
    }
    return user?.home_organization_id ?? user?.organizations?.[0]?.id;
  }, [orgId, user]);

  const { data: organizations = [], isLoading: isOrgsLoading } = useQuery({
    queryKey: organizationKeys.all,
    queryFn: () => organizationsApi.getAll(),
    staleTime: 5 * 60 * 1000,
  });

  const { data: organization, isLoading: isOrgLoading } =
    useOrganizationById(orgId);

  const value = useMemo(
    () => ({
      orgId,
      effectiveOrgId,
      organization,
      organizations,
      isLoading: isOrgsLoading || isOrgLoading,
    }),
    [
      orgId,
      effectiveOrgId,
      organization,
      organizations,
      isOrgsLoading,
      isOrgLoading,
    ]
  );

  return (
    <OrganizationContext.Provider value={value}>
      {children}
    </OrganizationContext.Provider>
  );
}

export function useOrganization(): OrganizationContextValue {
  const context = useContext(OrganizationContext);
  if (!context) {
    throw new Error(
      'useOrganization must be used within an OrganizationProvider'
    );
  }
  return context;
}
