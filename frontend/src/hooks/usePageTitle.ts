import { useEffect } from 'react';
import { useLocation } from 'react-router-dom';

const APP_NAME = 'Powerclub Global';

const ROUTE_TITLES: Record<string, string> = {
  '/projects': 'Projects',
  '/my-tasks': 'My Tasks',
  '/workflows': 'My Workflows',
  '/social-command': 'Social Command',
  '/virtual-environment': 'VIBELAND',
  '/vibe': 'Vibe',
  '/settings': 'Settings',
  '/nora': 'Nora',
  '/topsi': 'Topsi',
  '/global-tasks': 'Global Tasks',
  '/mission-control': 'Mission Control',
  '/crm': 'CRM',
  '/people': 'People',
  '/proposals': 'Proposals',
  '/companies': 'Companies',
  '/command-center': 'Command Center',
  '/invoices': 'Invoices',
  '/discord': 'Discord',
  '/pulse': 'Pulse',
  '/site-directory': 'Site Directory',
  '/oss-library-listener': 'OSS Library Listener',
};

const PATTERN_TITLES: Array<{ pattern: RegExp; title: string }> = [
  { pattern: /^\/organizations\/[^/]+\/crm\/contacts/, title: 'CRM Contacts' },
  { pattern: /^\/organizations\/[^/]+\/crm\/companies/, title: 'CRM Companies' },
  { pattern: /^\/organizations\/[^/]+\/crm\/pipeline/, title: 'CRM Pipeline' },
  { pattern: /^\/organizations\/[^/]+\/crm\/deliverables/, title: 'Deliverables' },
  { pattern: /^\/organizations\/[^/]+\/crm/, title: 'CRM Overview' },
  { pattern: /^\/organizations\/[^/]+\/intelligence/, title: 'Intelligence' },
  { pattern: /^\/organizations\/[^/]+\/members/, title: 'Members' },
  { pattern: /^\/organizations\/[^/]+\/integrations/, title: 'Integrations' },
  { pattern: /^\/organizations\/[^/]+\/projects/, title: 'Projects' },
  { pattern: /^\/organizations\/[^/]+/, title: 'Organization' },
  { pattern: /^\/projects\/[^/]+\/tasks/, title: 'Tasks' },
  { pattern: /^\/projects\/[^/]+\/control/, title: 'Project Control' },
  { pattern: /^\/projects\/[^/]+\/knowledge/, title: 'Knowledge' },
  { pattern: /^\/projects\/[^/]+\/deliverables/, title: 'Deliverables' },
  { pattern: /^\/projects\/[^/]+\/media/, title: 'Media Library' },
  { pattern: /^\/projects\/[^/]+\/pulse/, title: 'Pulse' },
  { pattern: /^\/projects\/[^/]+/, title: 'Project' },
  { pattern: /^\/people\/[^/]+/, title: 'Person' },
  { pattern: /^\/companies\/[^/]+/, title: 'Company' },
  { pattern: /^\/settings\/(.+)/, title: 'Settings' },
  { pattern: /^\/review\//, title: 'Review' },
];

function getTitleForPath(pathname: string): string {
  // Check exact matches first
  const exact = ROUTE_TITLES[pathname];
  if (exact) return exact;

  // Check pattern matches
  for (const { pattern, title } of PATTERN_TITLES) {
    if (pattern.test(pathname)) return title;
  }

  return '';
}

/** Sets document.title based on the current route */
export function usePageTitle(customTitle?: string) {
  const location = useLocation();

  useEffect(() => {
    const title = customTitle || getTitleForPath(location.pathname);
    document.title = title ? `${title} — ${APP_NAME}` : APP_NAME;
  }, [location.pathname, customTitle]);
}
