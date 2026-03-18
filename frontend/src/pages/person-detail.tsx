import { useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Skeleton } from '@/components/ui/skeleton';
import { crmApi, type CrmContactRecord } from '@/lib/api';
import { crmKeys } from '@/lib/query-keys';
import { useAuth } from '@/contexts/AuthContext';

/**
 * Person detail page — now redirects to the org-scoped CRM contact view.
 * The persons table has been unified with crm_contacts; this page resolves
 * the contact's organization and redirects to the org CRM contacts page.
 */
export function PersonDetailPage() {
  const { personId } = useParams<{ personId: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();

  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;

  // Try to fetch the contact to confirm it exists
  const { data: contact, isLoading, isError } = useQuery<CrmContactRecord>({
    queryKey: crmKeys.contactLegacy(personId!),
    queryFn: () => crmApi.getContact(personId!),
    enabled: !!personId,
  });

  useEffect(() => {
    if (contact && orgId) {
      // Redirect to the org contacts page — the contact detail modal will open via URL state
      navigate(
        `/organizations/${contact.organization_id || orgId}/crm/contacts?contact=${contact.id}`,
        { replace: true }
      );
    } else if (isError) {
      // Contact not found — go back to people list
      navigate('/people', { replace: true });
    }
  }, [contact, orgId, isError, navigate]);

  return (
    <div className="p-6 max-w-2xl mx-auto">
      {isLoading ? (
        <div className="space-y-4">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-64 w-full rounded-lg" />
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">Redirecting to contact details...</p>
      )}
    </div>
  );
}
