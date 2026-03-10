import { Link } from 'react-router-dom';
import {
  FolderKanban,
  Users,
  Brain,
  Megaphone,
  Crown,
  Settings,
  ExternalLink,
  Info,
  Building2,
  Layers,
  Clapperboard,
  BarChart3,
} from 'lucide-react';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';

interface DirectoryLink {
  label: string;
  to?: string;
  note?: string;
  admin?: boolean;
}

interface DirectoryCategory {
  title: string;
  icon: React.ReactNode;
  links: DirectoryLink[];
}

const categories: DirectoryCategory[] = [
  {
    title: 'Workspace',
    icon: <FolderKanban className="h-5 w-5" />,
    links: [
      { label: 'Projects', to: '/projects' },
      { label: 'My Tasks', to: '/my-tasks' },
      { label: 'My Workflows', to: '/workflows' },
      { label: 'Global Tasks', to: '/global-tasks', admin: true },
    ],
  },
  {
    title: 'Organization',
    icon: <Building2 className="h-5 w-5" />,
    links: [
      { label: 'Organization Overview', note: '/organizations/:orgId' },
      { label: 'Members', note: '/organizations/:orgId/members' },
      { label: 'Projects', note: '/organizations/:orgId/projects' },
      { label: 'Integrations', note: '/organizations/:orgId/integrations' },
      { label: 'Client Detail', note: '/organizations/:orgId/clients/:clientId' },
    ],
  },
  {
    title: 'CRM',
    icon: <Users className="h-5 w-5" />,
    links: [
      { label: 'CRM Admin', to: '/crm' },
      { label: 'People', to: '/people' },
      { label: 'Companies', to: '/companies' },
      { label: 'Proposals', to: '/proposals' },
      { label: 'Invoices', to: '/invoices' },
      { label: 'Org CRM Overview', note: '/organizations/:orgId/crm' },
      { label: 'Org Contacts', note: '/organizations/:orgId/crm/contacts' },
      { label: 'Org Companies', note: '/organizations/:orgId/crm/companies' },
      { label: 'Org Pipeline', note: '/organizations/:orgId/crm/pipeline' },
      { label: 'Org Deliverables', note: '/organizations/:orgId/crm/deliverables' },
    ],
  },
  {
    title: 'Intelligence',
    icon: <Brain className="h-5 w-5" />,
    links: [
      { label: 'Intelligence Overview', note: '/organizations/:orgId/intelligence' },
      { label: 'Data Sources', note: '/organizations/:orgId/intelligence/data-sources' },
      { label: 'Data Library (Full)', note: '/organizations/:orgId/data-sources' },
      { label: 'Data Source Detail', note: '/organizations/:orgId/data-sources/:id' },
      { label: 'Artifacts', note: '/organizations/:orgId/intelligence/artifacts' },
      { label: 'Workflows', note: '/organizations/:orgId/intelligence/workflows' },
      { label: 'Pulse', note: '/organizations/:orgId/intelligence/pulse' },
      { label: 'Topology', note: '/organizations/:orgId/intelligence/topology' },
      { label: 'Project Knowledge', note: '/projects/:projectId/knowledge' },
    ],
  },
  {
    title: 'Project Views',
    icon: <Layers className="h-5 w-5" />,
    links: [
      { label: 'Project Detail', note: '/projects/:projectId (brand profile, boards, assets)' },
      { label: 'Project Tasks', note: '/projects/:projectId/tasks' },
      { label: 'Project Controller', note: '/projects/:projectId/control' },
      { label: 'Project Deliverables', note: '/projects/:projectId/deliverables' },
      { label: 'Project Media Library', note: '/projects/:projectId/media' },
      { label: 'Project Pulse', note: '/projects/:projectId/pulse' },
      { label: 'Project CRM', note: '/projects/:projectId/crm' },
      { label: 'Project CRM Sales', note: '/projects/:projectId/crm/sales' },
      { label: 'Project CRM Delivery', note: '/projects/:projectId/crm/delivery' },
      { label: 'Project CRM Clients', note: '/projects/:projectId/crm/clients' },
      { label: 'Project CRM Conferences', note: '/projects/:projectId/crm/conferences' },
      { label: 'Project CRM Overview', note: '/projects/:projectId/crm/overview' },
    ],
  },
  {
    title: 'Social & Communications',
    icon: <Megaphone className="h-5 w-5" />,
    links: [
      { label: 'Social Command', to: '/social-command' },
      { label: 'Discord Voice', to: '/discord' },
      { label: 'Org Social / Experiences', note: '/organizations/:orgId/social' },
    ],
  },
  {
    title: 'Creative & Media',
    icon: <Clapperboard className="h-5 w-5" />,
    links: [
      { label: 'Vibe', to: '/vibe' },
      { label: 'VIBELAND', to: '/virtual-environment' },
      { label: 'OSS Library Listener', to: '/oss-library-listener' },
    ],
  },
  {
    title: 'Platform Tools',
    icon: <Crown className="h-5 w-5" />,
    links: [
      { label: 'Nora Command', to: '/nora', admin: true },
      { label: 'Topsi Platform', to: '/topsi' },
      { label: 'Mission Control', to: '/mission-control' },
      { label: 'Command Center', to: '/command-center' },
      { label: 'Pulse Engine', to: '/pulse' },
      { label: 'AI Usage', to: '/ai-usage', admin: true },
    ],
  },
  {
    title: 'Analytics & Monitoring',
    icon: <BarChart3 className="h-5 w-5" />,
    links: [
      { label: 'Site Directory', to: '/site-directory', admin: true },
      { label: 'Review Portal', note: '/review/:token (public, token-based)' },
    ],
  },
  {
    title: 'Settings',
    icon: <Settings className="h-5 w-5" />,
    links: [
      { label: 'General', to: '/settings/general' },
      { label: 'Profile', to: '/settings/profile' },
      { label: 'Wallet', to: '/settings/wallet' },
      { label: 'Users', to: '/settings/users', admin: true },
      { label: 'Organizations', to: '/settings/organizations', admin: true },
      { label: 'Projects', to: '/settings/projects', admin: true },
      { label: 'Privacy & Security', to: '/settings/privacy' },
      { label: 'Activity Log', to: '/settings/activity' },
      { label: 'Agents', to: '/settings/agents' },
      { label: 'API Keys', to: '/settings/keys' },
      { label: 'Models', to: '/settings/models' },
      { label: 'MCP Servers', to: '/settings/mcp' },
      { label: 'Network & Mesh', to: '/settings/network' },
    ],
  },
];

export function SiteDirectoryPage() {
  return (
    <div className="container mx-auto max-w-7xl px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight">Site Directory</h1>
        <p className="mt-1 text-muted-foreground">
          Complete map of every section in the platform. Links with paths shown as notes are scoped to a specific organization or project.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
        {categories.map((category) => (
          <Card key={category.title} className="flex flex-col">
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2 text-lg">
                {category.icon}
                {category.title}
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-1 pt-0">
              {category.links.map((link) =>
                link.to ? (
                  <Link
                    key={link.label}
                    to={link.to}
                    className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-accent hover:text-accent-foreground"
                  >
                    <ExternalLink className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    <span>{link.label}</span>
                    {link.admin && (
                      <Badge variant="secondary" className="ml-auto text-[10px] px-1.5 py-0">
                        ADMIN
                      </Badge>
                    )}
                  </Link>
                ) : (
                  <div
                    key={link.label}
                    className="flex items-start gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground"
                  >
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                    <div>
                      <span>{link.label}</span>
                      {link.note && (
                        <p className="text-xs text-muted-foreground/70">{link.note}</p>
                      )}
                    </div>
                  </div>
                )
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
