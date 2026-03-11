import { Building2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useOrganization } from '@/contexts/organization-context';

/**
 * Empty state shown when a user navigates to an org-scoped page
 * without having an active organization selected.
 */
export function NoOrgSelected({ feature }: { feature?: string }) {
  const { organizations } = useOrganization();
  const hasOrgs = organizations.length > 0;

  return (
    <div className="flex flex-col items-center justify-center gap-4 py-24 text-center">
      <div className="rounded-full bg-muted p-4">
        <Building2 className="h-8 w-8 text-muted-foreground" />
      </div>
      <div>
        <h2 className="text-lg font-semibold">No organization selected</h2>
        <p className="mt-1 max-w-md text-sm text-muted-foreground">
          {feature ? `${feature} is` : 'This section is'} scoped to an organization.
          {hasOrgs
            ? ' Select an organization from the sidebar to continue.'
            : ' You are not a member of any organization yet.'}
        </p>
      </div>
      {hasOrgs && (
        <div className="flex flex-wrap justify-center gap-2">
          {organizations.slice(0, 3).map((org) => (
            <Link
              key={org.id}
              to={`/organizations/${org.id}`}
              className="inline-flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors hover:bg-accent"
            >
              <Building2 className="h-3.5 w-3.5" />
              {org.name}
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
