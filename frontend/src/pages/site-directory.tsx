import { useMemo } from 'react';
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
import { useOrganization } from '@/contexts/organization-context';

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

function buildCategories(orgId: string | undefined): DirectoryCategory[] {
  const o = orgId ? `/organizations/${orgId}` : undefined;

  return [
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
        ...(o
          ? [
              { label: 'Organization Overview', to: o },
              { label: 'Members', to: `${o}/members` },
              { label: 'Projects', to: `${o}/projects` },
              { label: 'Integrations', to: `${o}/integrations` },
            ]
          : [
              { label: 'Organization Overview', note: 'Select an organization first' },
              { label: 'Members', note: 'Select an organization first' },
              { label: 'Projects', note: 'Select an organization first' },
              { label: 'Integrations', note: 'Select an organization first' },
            ]),
        { label: 'Client Detail', note: 'Navigate from organization clients list' },
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
        ...(o
          ? [
              { label: 'Org CRM Overview', to: `${o}/crm` },
              { label: 'Org Contacts', to: `${o}/crm/contacts` },
              { label: 'Org Companies', to: `${o}/crm/companies` },
              { label: 'Org Pipeline', to: `${o}/crm/pipeline` },
              { label: 'Org Deliverables', to: `${o}/crm/deliverables` },
            ]
          : [
              { label: 'Org CRM Overview', note: 'Select an organization first' },
              { label: 'Org Contacts', note: 'Select an organization first' },
              { label: 'Org Companies', note: 'Select an organization first' },
              { label: 'Org Pipeline', note: 'Select an organization first' },
              { label: 'Org Deliverables', note: 'Select an organization first' },
            ]),
      ],
    },
    {
      title: 'Intelligence',
      icon: <Brain className="h-5 w-5" />,
      links: [
        ...(o
          ? [
              { label: 'Intelligence Overview', to: `${o}/intelligence` },
              { label: 'Data Sources', to: `${o}/intelligence/data-sources` },
              { label: 'Data Library (Full)', to: `${o}/data-sources` },
              { label: 'Artifacts', to: `${o}/intelligence/artifacts` },
              { label: 'Workflows', to: `${o}/intelligence/workflows` },
              { label: 'Pulse', to: `${o}/intelligence/pulse` },
              { label: 'Topology', to: `${o}/intelligence/topology` },
            ]
          : [
              { label: 'Intelligence Overview', note: 'Select an organization first' },
              { label: 'Data Sources', note: 'Select an organization first' },
              { label: 'Data Library (Full)', note: 'Select an organization first' },
              { label: 'Artifacts', note: 'Select an organization first' },
              { label: 'Workflows', note: 'Select an organization first' },
              { label: 'Pulse', note: 'Select an organization first' },
              { label: 'Topology', note: 'Select an organization first' },
            ]),
        { label: 'Data Source Detail', note: 'Navigate from data sources list' },
        { label: 'Project Knowledge', note: 'Navigate from project detail' },
      ],
    },
    {
      title: 'Project Views',
      icon: <Layers className="h-5 w-5" />,
      links: [
        { label: 'Project Detail', note: 'Brand profile, boards, assets — open from Projects' },
        { label: 'Project Tasks', note: 'Kanban board — open from project sidebar' },
        { label: 'Project Controller', note: 'Agent execution — open from project sidebar' },
        { label: 'Project Deliverables', note: 'Delivery tracking — open from project sidebar' },
        { label: 'Project Media Library', note: 'Media assets — open from project sidebar' },
        { label: 'Project Pulse', note: 'Monitoring — open from project sidebar' },
        { label: 'Project CRM', note: 'Project-scoped CRM — open from project sidebar' },
      ],
    },
    {
      title: 'Social & Communications',
      icon: <Megaphone className="h-5 w-5" />,
      links: [
        { label: 'Social Command', to: '/social-command' },
        { label: 'Discord Voice', to: '/discord' },
        ...(o
          ? [{ label: 'Org Social / Experiences', to: `${o}/social` }]
          : [{ label: 'Org Social / Experiences', note: 'Select an organization first' }]),
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
        { label: 'Review Portal', note: 'Public, token-based access' },
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
}

export function SiteDirectoryPage() {
  const { orgId, organizations } = useOrganization();
  const effectiveOrgId = orgId || organizations?.[0]?.id;
  const categories = useMemo(() => buildCategories(effectiveOrgId), [effectiveOrgId]);

  return (
    <div className="container mx-auto max-w-7xl px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight">Site Directory</h1>
        <p className="mt-1 text-muted-foreground">
          Complete map of every section in the platform.
          {effectiveOrgId && organizations?.length ? (
            <> Organization links point to <strong>{organizations.find(o => o.id === effectiveOrgId)?.name || effectiveOrgId}</strong>.</>
          ) : (
            <> Select an organization to enable org-scoped links.</>
          )}
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
