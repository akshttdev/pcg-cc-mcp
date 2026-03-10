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
    title: 'Project Management',
    icon: <FolderKanban className="h-5 w-5" />,
    links: [
      { label: 'Projects', to: '/projects' },
      { label: 'My Tasks', to: '/my-tasks' },
      { label: 'Global Tasks', to: '/global-tasks', admin: true },
    ],
  },
  {
    title: 'CRM & Sales',
    icon: <Users className="h-5 w-5" />,
    links: [
      { label: 'CRM Admin', to: '/crm' },
      { label: 'All People', to: '/people', admin: true },
      { label: 'All Companies', to: '/companies', admin: true },
      { label: 'Proposals', to: '/proposals' },
      { label: 'Invoices', to: '/invoices' },
    ],
  },
  {
    title: 'Intelligence & Knowledge',
    icon: <Brain className="h-5 w-5" />,
    links: [
      { label: 'Workflow Builder', to: '/workflows' },
      { label: 'Knowledge', note: 'Accessed per-project: /projects/:id/knowledge' },
      { label: 'Data Sources', note: 'Accessed per-org via Intelligence tab' },
    ],
  },
  {
    title: 'Social & Communications',
    icon: <Megaphone className="h-5 w-5" />,
    links: [
      { label: 'Social Command', to: '/social-command' },
      { label: 'Discord Voice', to: '/discord' },
    ],
  },
  {
    title: 'Admin Tools',
    icon: <Crown className="h-5 w-5" />,
    links: [
      { label: 'Nora Command', to: '/nora' },
      { label: 'Topsi Platform', to: '/topsi' },
      { label: 'Mission Control', to: '/mission-control' },
      { label: 'Command Center', to: '/command-center' },
      { label: 'Pulse Engine', to: '/pulse' },
      { label: 'Mesh Network', to: '/settings/network' },
      { label: 'Vibe', to: '/vibe' },
      { label: 'VIBELAND', to: '/virtual-environment' },
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
          Quick access to every section of the platform
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
