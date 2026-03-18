import { createContext, useContext, ReactNode, useMemo } from 'react';
import { useLocation, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';
import type { OrganizationData } from '@/lib/api';
import { useOrganizationById } from '@/hooks/queries';
import { useAuth } from './AuthContext';

interface OrganizationContextValue {
  /** orgId extracted from the current URL (undefined if not on an org route) */
  orgId: string | undefined;
  /** The effective org: URL orgId → user home org → first org. Mirrors sidebar logic. */
  effectiveOrgId: string | undefined;
  organization: OrganizationData | undefined;
  organizations: OrganizationData[];
  isLoading: boolean;
}

const OrganizationContext = createContext<OrganizationContextValue | null>(null);

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

  // Effective org mirrors sidebar logic: URL org → home org → first org
  const effectiveOrgId = useMemo(() => {
    if (orgId) return orgId;
    return user?.home_organization_id ?? user?.organizations?.[0]?.id;
  }, [orgId, user]);

  const { data: organizations = [], isLoading: isOrgsLoading } = useQuery({
    queryKey: organizationKeys.all,
    queryFn: () => organizationsApi.getAll(),
    staleTime: 5 * 60 * 1000,
  });

  const { data: organization, isLoading: isOrgLoading } = useOrganizationById(orgId);

  const value = useMemo(
    () => ({
      orgId,
      effectiveOrgId,
      organization,
      organizations,
      isLoading: isOrgsLoading || isOrgLoading,
    }),
    [orgId, effectiveOrgId, organization, organizations, isOrgsLoading, isOrgLoading]
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
    throw new Error('useOrganization must be used within an OrganizationProvider');
  }
  return context;
}
