import type { ComponentType } from 'react';

import type { VizNode } from '@/lib/graph/adapter';

import { EntityPreview, type EntityPreviewProps } from './EntityPreview';

/**
 * Registry of node_type → preview component.
 *
 * Phase 2 ships the generic `EntityPreview` as the fallback for every node
 * type. Bespoke previews for company / contact / deal / task / etc. land
 * incrementally — each becomes a lazy-imported entry here. The goal is
 * that the canvas never knows about specific node types.
 *
 * Adding a real preview:
 *   1. Create `CompanyPreview.tsx` in this directory.
 *   2. `PREVIEWS.company = lazy(() => import('./CompanyPreview'))`
 *   3. That's it — canvas auto-uses it on click.
 */

type PreviewComponent = ComponentType<EntityPreviewProps>;

const PREVIEWS: Partial<Record<string, PreviewComponent>> = {
  // All node types use the fallback for now; bespoke previews land in follow-up PRs.
};

export function resolvePreview(nodeType: string): PreviewComponent {
  return PREVIEWS[nodeType] ?? EntityPreview;
}

/**
 * Render the correct preview for a node. Convenience wrapper so callers
 * don't have to thread the registry lookup themselves.
 */
export function NodePreview(props: EntityPreviewProps) {
  const Cmp = resolvePreview(props.node.node_type);
  return <Cmp {...props} />;
}

export type { EntityPreviewProps } from './EntityPreview';
export { EntityPreview } from './EntityPreview';

/** Default hrefs per node type. Returns `null` if we don't know where the
 *  detail page for this type lives yet. */
export function openProfileHref(node: VizNode, orgId?: string): string | null {
  switch (node.node_type) {
    case 'company':
      return orgId ? `/organizations/${orgId}/companies/${node.ref_id}` : null;
    case 'organization':
      return `/organizations/${node.ref_id}`;
    case 'person':
      return `/people/${node.ref_id}`;
    case 'client':
      return orgId ? `/organizations/${orgId}/clients/${node.ref_id}` : null;
    case 'project':
      return `/projects/${node.ref_id}`;
    case 'proposal':
      return `/proposals/${node.ref_id}`;
    case 'deal':
      return orgId ? `/organizations/${orgId}/crm/deals/${node.ref_id}` : null;
    case 'pipeline':
      return orgId
        ? `/organizations/${orgId}/crm/pipelines/${node.ref_id}`
        : null;
    default:
      return null;
  }
}
