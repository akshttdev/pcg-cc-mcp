import { Network } from 'lucide-react';
import { Link } from 'react-router-dom';

import { Button } from '@/components/ui/button';

export interface ViewInTopologyButtonProps {
  orgId: string;
  focusType: 'company' | 'person' | 'proposal' | 'deal' | 'project' | 'client';
  focusId: string;
  depth?: number;
  /** Override button label. Defaults to "View in Topology". */
  label?: string;
  variant?: 'default' | 'outline' | 'ghost' | 'secondary';
  size?: 'sm' | 'default';
}

/**
 * Navigates to the org's Intelligence › Topology tab, zoomed on the given
 * entity. Designed to live on any entity's profile page as a quick
 * "show me how this connects" affordance.
 */
export function ViewInTopologyButton({
  orgId,
  focusType,
  focusId,
  depth = 2,
  label = 'View in Topology',
  variant = 'outline',
  size = 'sm',
}: ViewInTopologyButtonProps) {
  const params = new URLSearchParams({
    focus_type: focusType,
    focus_id: focusId,
    depth: String(depth),
  });
  const href = `/organizations/${orgId}/intelligence/topology?${params}`;

  return (
    <Button
      asChild
      variant={variant}
      size={size}
      data-testid={`view-in-topology-${focusType}`}
      className="gap-1.5"
    >
      <Link to={href}>
        <Network className="h-3.5 w-3.5" />
        {label}
      </Link>
    </Button>
  );
}
