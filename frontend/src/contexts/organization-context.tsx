import { createContext, useContext, ReactNode, useMemo } from 'react';
import { useLocation, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api';
import type { OrganizationData } from '@/lib/api';

interface OrganizationContextValue {
  orgId: string | undefined;
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

  // Extract orgId from URL if on an org route
  const orgId = useMemo(() => {
    if (routeOrgId) return routeOrgId;
    const match = location.pathname.match(/^\/organizations\/([^/]+)/);
    return match ? match[1] : undefined;
  }, [routeOrgId, location.pathname]);

  const { data: organizations = [], isLoading: isOrgsLoading } = useQuery({
    queryKey: ['organizations'],
    queryFn: () => organizationsApi.getAll(),
    staleTime: 5 * 60 * 1000,
  });

  const { data: organization, isLoading: isOrgLoading } = useQuery({
    queryKey: ['organization', orgId],
    queryFn: () => organizationsApi.getById(orgId!),
    enabled: !!orgId,
    staleTime: 5 * 60 * 1000,
  });

  const value = useMemo(
    () => ({
      orgId,
      organization,
      organizations,
      isLoading: isOrgsLoading || isOrgLoading,
    }),
    [orgId, organization, organizations, isOrgsLoading, isOrgLoading]
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
