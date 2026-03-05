import { useQueries, useQuery } from '@tanstack/react-query';
import { organizationsApi, crmApi, type CrmContactRecord } from '@/lib/api';

export interface OrgContact extends CrmContactRecord {
  _sourceProjectId: string;
  _sourceProjectName: string;
}

export function useOrgContacts(orgId: string | undefined) {
  const { data: sidebarTree } = useQuery({
    queryKey: ['sidebarTree'],
    queryFn: () => organizationsApi.getSidebarTree(),
    staleTime: 60_000,
  });

  const sidebarOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find(o => o.id === orgId)
    : undefined;

  // Extract all project IDs + names from sidebar tree for this org
  const projectEntries: { id: string; name: string }[] = [];
  if (sidebarOrg) {
    for (const p of sidebarOrg.internal_projects || []) {
      projectEntries.push({ id: p.id, name: p.name });
    }
    for (const f of sidebarOrg.internal_folders || []) {
      for (const p of f.projects) {
        projectEntries.push({ id: p.id, name: p.name });
      }
    }
    for (const c of sidebarOrg.clients || []) {
      for (const p of c.projects || []) {
        projectEntries.push({ id: p.id, name: p.name });
      }
      for (const f of c.folders || []) {
        for (const p of f.projects) {
          projectEntries.push({ id: p.id, name: p.name });
        }
      }
    }
  }

  const contactQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['crm-contacts', entry.id],
      queryFn: () => crmApi.listContacts(entry.id),
      enabled: !!orgId && projectEntries.length > 0,
      staleTime: 60_000,
    })),
  });

  const isLoading = contactQueries.some(q => q.isLoading);
  const loadedCount = contactQueries.filter(q => q.isSuccess).length;

  // Merge and deduplicate by email (keep highest lead_score)
  const contactMap = new Map<string, OrgContact>();
  contactQueries.forEach((q, i) => {
    if (!q.data) return;
    const entry = projectEntries[i];
    for (const contact of q.data) {
      const key = contact.email || contact.id;
      const existing = contactMap.get(key);
      const tagged: OrgContact = {
        ...contact,
        _sourceProjectId: entry.id,
        _sourceProjectName: entry.name,
      };
      if (!existing || tagged.lead_score > existing.lead_score) {
        contactMap.set(key, tagged);
      }
    }
  });

  const contacts = Array.from(contactMap.values());

  return {
    contacts,
    isLoading,
    projectCount: projectEntries.length,
    loadedCount,
  };
}
