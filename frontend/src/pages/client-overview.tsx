import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import { lazy, Suspense, useState } from 'react';
import {
  Link,
  useNavigate,
  useParams,
  useSearchParams,
} from 'react-router-dom';
import type { Project } from 'shared/types';

import {
  type ClientWithIntel,
  type CrmContactRecord,
  organizationsApi,
  type OrgBrandProfile,
  projectsApi,
  type SocialAccountRecord,
  socialApi,
} from '@/lib/api';
import {
  organizationKeys,
  projectKeys,
  socialKeys,
  userKeys,
} from '@/lib/query-keys';

/** Extended client data as returned by the API */
interface ClientOverviewData extends ClientWithIntel {
  crm_person_id?: string;
  crm_confidence?: number;
}

interface MemberRecord {
  id?: string;
  user_id?: string;
  full_name?: string;
  username?: string;
  email?: string;
  role?: string;
}
import '@/components/dialogs/shared/ConvertEntityDialog';

import NiceModal from '@ebay/nice-modal-react';
import {
  Activity,
  ArrowLeft,
  BarChart3,
  BookOpen,
  Brain,
  CheckCircle,
  Contact2,
  ExternalLink,
  FolderKanban,
  Globe,
  Layers,
  Loader2,
  Mail,
  Package,
  Phone,
  Plug,
  Plus,
  Share2,
  Trash2,
  Users,
} from 'lucide-react';

import { ClientMembersDialog } from '@/components/dialogs/client-members-dialog';
import { ClientProjectPanel } from '@/components/projects/ClientProjectPanel';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { CardGrid } from '@/components/ui/card-grid';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Loader } from '@/components/ui/loader';
import { useCrmContacts } from '@/hooks/queries';
import { cn } from '@/lib/utils';

import { PersonIntelPage } from './person-intel';

// ── Lazy-load scoped tab components ──────────────────────────────────────────
const SocialTab = lazy(() =>
  import('./organization-profile/tabs/social').then((m) => ({
    default:
      m.default ??
      ((m as Record<string, unknown>).SocialTab as React.ComponentType<{
        projectEntries: Array<{ id: string; name: string }>;
        orgId?: string;
        platformFilter?: string[];
      }>),
  }))
);

type Tab =
  | 'overview'
  | 'projects'
  | 'contacts'
  | 'members'
  | 'pipeline'
  | 'social'
  | 'intel'
  | 'integrations';

function TabSkeleton() {
  return (
    <div className="flex items-center justify-center py-16">
      <Loader size={20} />
    </div>
  );
}

function TabBtn({
  active,
  onClick,
  icon: Icon,
  children,
}: {
  active: boolean;
  onClick: () => void;
  icon?: React.ElementType;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'flex items-center gap-1.5 px-3 py-2 text-sm font-medium rounded-md transition-colors whitespace-nowrap',
        active
          ? 'bg-primary text-primary-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground hover:bg-accent/60'
      )}
    >
      {Icon && <Icon className="h-3.5 w-3.5" />}
      {children}
    </button>
  );
}

// ── Brand header strip ────────────────────────────────────────────────────────
const CLIENT_PALETTES = [
  { from: '#6366f1', to: '#8b5cf6' },
  { from: '#0ea5e9', to: '#6366f1' },
  { from: '#10b981', to: '#0ea5e9' },
  { from: '#f59e0b', to: '#ef4444' },
  { from: '#ec4899', to: '#8b5cf6' },
  { from: '#14b8a6', to: '#6366f1' },
];

function hashId(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) | 0;
  return Math.abs(h);
}

// ── Projects tab ──────────────────────────────────────────────────────────────
function ProjectsTab({
  projects,
  isProjectsLoading,
  orgId,
  clientId,
}: {
  projects: Project[];
  isProjectsLoading: boolean;
  orgId: string;
  clientId: string;
}) {
  if (isProjectsLoading)
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground py-8">
        <Loader size={16} /> Loading projects…
      </div>
    );
  if (projects.length === 0)
    return (
      <div className="text-center py-12 text-muted-foreground">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 opacity-30" />
        <p className="text-sm">No projects yet.</p>
      </div>
    );
  return (
    <div className="space-y-8">
      {projects.map((project) => (
        <ClientProjectPanel
          key={project.id}
          projectId={project.id}
          clientPageUrl={`/organizations/${orgId}/clients/${clientId}`}
        />
      ))}
    </div>
  );
}

// ── KG Intel panel (inline on Overview) ───────────────────────────────────────
function KgIntelPanel({ client }: { client: ClientOverviewData }) {
  if (!client.company_id && !client.primary_person_id) {
    return (
      <Card className="p-4 border-dashed border-indigo-500/30 bg-indigo-950/10">
        <div className="flex items-center gap-2 mb-1">
          <Brain className="h-4 w-4 text-indigo-400" />
          <span className="text-xs font-semibold text-indigo-300 uppercase tracking-wide">
            Knowledge Graph
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Not linked to the Knowledge Graph.{' '}
          <span className="text-indigo-400">
            Search companies to link intel.
          </span>
        </p>
      </Card>
    );
  }

  const raw = client.company_intel_raw
    ? (() => {
        try {
          return JSON.parse(client.company_intel_raw!);
        } catch {
          return null;
        }
      })()
    : null;

  return (
    <Card className="p-4 border-indigo-700/30 bg-indigo-950/10 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-indigo-400" />
          <span className="text-xs font-semibold text-indigo-300 uppercase tracking-wide">
            Knowledge Graph
          </span>
          {client.company_intel_status && (
            <span
              className={`text-xs px-1.5 py-0.5 rounded-full font-medium ${
                client.company_intel_status === 'done'
                  ? 'bg-green-500/20 text-green-400'
                  : client.company_intel_status === 'running'
                    ? 'bg-amber-500/20 text-amber-400'
                    : 'bg-muted text-muted-foreground'
              }`}
            >
              {client.company_intel_status}
            </span>
          )}
          {client.company_intel_confidence != null && (
            <span className="text-xs text-muted-foreground">
              {Math.round(client.company_intel_confidence * 100)}% confidence
            </span>
          )}
        </div>
        {client.company_id && (
          <Link
            to={`/companies/${client.company_id}/intel`}
            className="text-xs text-indigo-400 hover:text-indigo-300 flex items-center gap-1"
          >
            Full Intel <ExternalLink className="h-3 w-3" />
          </Link>
        )}
      </div>

      {/* Company summary */}
      {client.company_intel_summary && (
        <p className="text-xs text-foreground/80 leading-relaxed border-l-2 border-indigo-500/30 pl-3">
          {client.company_intel_summary}
        </p>
      )}

      {/* Company meta */}
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {(client.company_industry ?? raw?.industry) && (
          <span>{client.company_industry ?? raw?.industry}</span>
        )}
        {(client.company_employee_count ?? raw?.employee_count) && (
          <span>
            {client.company_employee_count ?? raw?.employee_count} employees
          </span>
        )}
        {(client.company_website ?? raw?.website) && (
          <a
            href={client.company_website ?? raw?.website}
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary hover:underline flex items-center gap-1"
          >
            <Globe className="h-3 w-3" />
            {(client.company_website ?? raw?.website ?? '').replace(
              /^https?:\/\//,
              ''
            )}
          </a>
        )}
      </div>

      {/* Primary person */}
      {(client.person_full_name || client.person_intel_summary) && (
        <div className="border-t border-border/40 pt-3 flex items-start gap-3">
          <div className="w-7 h-7 rounded-full bg-indigo-800/40 flex items-center justify-center text-xs font-semibold text-indigo-300 shrink-0">
            {client.person_full_name?.charAt(0) ?? '?'}
          </div>
          <div className="min-w-0">
            <p className="text-xs font-semibold">{client.person_full_name}</p>
            {client.person_title && (
              <p className="text-xs text-muted-foreground">
                {client.person_title}
              </p>
            )}
            {client.person_intel_summary && (
              <p className="text-xs text-foreground/70 mt-1 line-clamp-2">
                {client.person_intel_summary}
              </p>
            )}
            {client.person_linkedin_url && (
              <a
                href={client.person_linkedin_url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-indigo-400 hover:underline"
              >
                LinkedIn
              </a>
            )}
          </div>
          {client.primary_person_id && (
            <Link
              to={`/people/${client.primary_person_id}/intel`}
              className="ml-auto text-xs text-indigo-400 hover:text-indigo-300 shrink-0"
            >
              Intel →
            </Link>
          )}
        </div>
      )}
    </Card>
  );
}

// ── Overview tab ──────────────────────────────────────────────────────────────
function OverviewTab({
  client,
  projects,
  orgId,
}: {
  client: ClientOverviewData;
  projects: Project[];
  orgId: string;
}) {
  const activeProjects = projects.filter(
    (p) =>
      (p as Project & { project_status?: string }).project_status !== 'archived'
  );
  const completedProjects = projects.filter(
    (p) =>
      (p as Project & { project_status?: string }).project_status === 'archived'
  );
  return (
    <div className="space-y-6">
      {/* KG Intel — always shown at top of overview */}
      <KgIntelPanel client={client} />

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <Card className="p-4">
          <div className="flex items-center gap-2 mb-1">
            <FolderKanban className="h-4 w-4 text-primary" />
            <span className="text-xs text-muted-foreground font-medium">
              Projects
            </span>
          </div>
          <p className="text-2xl font-semibold">{projects.length}</p>
          <p className="text-xs text-muted-foreground">
            {activeProjects.length} active
          </p>
        </Card>
        <Card className="p-4">
          <div className="flex items-center gap-2 mb-1">
            <CheckCircle className="h-4 w-4 text-green-500" />
            <span className="text-xs text-muted-foreground font-medium">
              Completed
            </span>
          </div>
          <p className="text-2xl font-semibold">{completedProjects.length}</p>
          <p className="text-xs text-muted-foreground">archived</p>
        </Card>
        <Card className="p-4">
          <div className="flex items-center gap-2 mb-1">
            <Activity className="h-4 w-4 text-amber-500" />
            <span className="text-xs text-muted-foreground font-medium">
              Status
            </span>
          </div>
          <Badge
            variant={client.is_active ? 'default' : 'secondary'}
            className="mt-1"
          >
            {client.client_since
              ? 'Client'
              : client.prospect_at
                ? 'Prospect'
                : client.is_active
                  ? 'Active'
                  : 'Inactive'}
          </Badge>
        </Card>
        {client.crm_confidence != null && (
          <Card className="p-4">
            <div className="flex items-center gap-2 mb-1">
              <BarChart3 className="h-4 w-4 text-indigo-500" />
              <span className="text-xs text-muted-foreground font-medium">
                CRM Confidence
              </span>
            </div>
            <p className="text-2xl font-semibold">
              {Math.round((client.crm_confidence ?? 0) * 100)}%
            </p>
          </Card>
        )}
      </div>

      {client.description && (
        <Card className="p-4">
          <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            About
          </p>
          <p className="text-sm text-foreground/80 leading-relaxed">
            {client.description}
          </p>
        </Card>
      )}

      <div className="flex flex-wrap gap-3">
        {client.website && (
          <a
            href={client.website}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 text-sm text-primary hover:underline"
          >
            <Globe className="h-4 w-4" />
            {client.website.replace(/^https?:\/\//, '')}
            <ExternalLink className="h-3 w-3" />
          </a>
        )}
        <Link
          to={`/organizations/${orgId}/crm/pipeline?client=${client.id}`}
          className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          <Package className="h-4 w-4" /> View in Pipeline
        </Link>
        {client.crm_person_id && (
          <Link
            to={`/people/${client.crm_person_id}`}
            className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            <Users className="h-4 w-4" /> CRM Profile
          </Link>
        )}
      </div>

      {activeProjects.length > 0 && (
        <div>
          <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Active Projects
          </p>
          <div className="space-y-1.5">
            {activeProjects.slice(0, 4).map((p) => (
              <Link
                key={p.id}
                to={`/projects/${p.id}/tasks`}
                className="flex items-center gap-2 px-3 py-2 rounded-md border border-border/60 hover:bg-accent/60 transition-colors text-sm"
              >
                <Layers className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="truncate">{p.name}</span>
                <ExternalLink className="h-3 w-3 text-muted-foreground ml-auto shrink-0" />
              </Link>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Contacts tab (scoped to client company) ───────────────────────────────────
function ClientContactsTab({
  orgId,
  clientName,
}: {
  orgId: string;
  clientName: string;
}) {
  const { data: allContacts = [], isLoading } = useCrmContacts(orgId);
  const contacts = (allContacts as CrmContactRecord[]).filter(
    (c) => c.company_name?.toLowerCase() === clientName.toLowerCase()
  );
  if (isLoading)
    return (
      <div className="flex items-center justify-center py-12">
        <Loader size={20} />
      </div>
    );
  if (contacts.length === 0)
    return (
      <div className="text-center py-12 text-muted-foreground">
        <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
        <p className="text-sm">No contacts found for {clientName}</p>
        <p className="text-xs mt-1 opacity-60">
          Contacts appear here when their company name matches this client.
        </p>
      </div>
    );
  return (
    <CardGrid columns={{ md: 2, lg: 3 }} gap={3}>
      {contacts.map((contact) => (
        <div
          key={contact.id}
          className="rounded-lg border bg-card p-4 space-y-2"
        >
          <div className="flex items-start gap-3">
            <div className="w-9 h-9 rounded-full bg-muted flex items-center justify-center shrink-0 text-sm font-semibold text-muted-foreground">
              {contact.full_name?.charAt(0) ?? '?'}
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-sm font-semibold truncate">
                {contact.full_name ?? 'Unknown'}
              </p>
              {contact.job_title && (
                <p className="text-xs text-muted-foreground truncate">
                  {contact.job_title}
                </p>
              )}
              {contact.email && (
                <p className="text-xs text-muted-foreground truncate flex items-center gap-1 mt-0.5">
                  <Mail className="h-3 w-3 shrink-0" />
                  {contact.email}
                </p>
              )}
              {contact.phone && (
                <p className="text-xs text-muted-foreground truncate flex items-center gap-1 mt-0.5">
                  <Phone className="h-3 w-3 shrink-0" />
                  {contact.phone}
                </p>
              )}
            </div>
          </div>
          {contact.lifecycle_stage && (
            <Badge variant="secondary" className="text-xs">
              {contact.lifecycle_stage}
            </Badge>
          )}
        </div>
      ))}
    </CardGrid>
  );
}

// ── Members tab (client-specific, deduplicated) ───────────────────────────────
function ClientMembersTab({
  clientId,
  orgId,
}: {
  clientId: string;
  orgId: string;
}) {
  const { data: members = [], isLoading } = useQuery<MemberRecord[]>({
    queryKey: userKeys.clientMembers(clientId),
    queryFn: async () => {
      const res = await fetch(`/api/clients/${clientId}/members`, {
        credentials: 'include',
      });
      if (!res.ok) throw new Error('Failed');
      return (await res.json()).data ?? [];
    },
    staleTime: 30_000,
  });

  const { data: orgMembers = [] } = useQuery<MemberRecord[]>({
    queryKey: organizationKeys.members(orgId),
    queryFn: () => organizationsApi.getMembers(orgId),
    staleTime: 60_000,
  });

  const orgMemberMap = new Map(orgMembers.map((m) => [m.user_id ?? m.id, m]));

  // Deduplicate by user_id
  const seen = new Set<string>();
  const uniqueMembers = members.filter((m) => {
    const id = m.user_id ?? m.id;
    if (!id || seen.has(id)) return false;
    seen.add(id);
    return true;
  });

  if (isLoading)
    return (
      <div className="flex items-center justify-center py-12">
        <Loader size={20} />
      </div>
    );
  if (uniqueMembers.length === 0)
    return (
      <div className="text-center py-12 text-muted-foreground">
        <Users className="h-8 w-8 mx-auto mb-2 opacity-40" />
        <p className="text-sm">No members assigned to this client yet.</p>
      </div>
    );
  return (
    <div className="space-y-2">
      {uniqueMembers.map((m) => {
        const uid = m.user_id ?? m.id;
        const orgM = orgMemberMap.get(uid);
        const name = orgM?.full_name ?? m.full_name ?? m.username ?? uid ?? '';
        const email = orgM?.email ?? m.email;
        return (
          <div
            key={uid}
            className="flex items-center gap-3 p-3 rounded-lg border bg-card"
          >
            <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-xs font-semibold text-muted-foreground shrink-0">
              {name.charAt(0).toUpperCase()}
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium truncate">{name}</p>
              {email && (
                <p className="text-xs text-muted-foreground truncate">
                  {email}
                </p>
              )}
            </div>
            <Badge variant="outline" className="text-xs capitalize">
              {m.role ?? 'member'}
            </Badge>
          </div>
        );
      })}
    </div>
  );
}

// ── Pipeline tab ──────────────────────────────────────────────────────────────
function PipelineTab({ orgId, clientId }: { orgId: string; clientId: string }) {
  return (
    <div className="space-y-4">
      <div className="rounded-lg border bg-card p-6 text-center">
        <Package className="h-10 w-10 mx-auto mb-3 text-primary opacity-70" />
        <h3 className="font-semibold mb-1">Client Pipeline</h3>
        <p className="text-sm text-muted-foreground mb-4">
          View all active deals, acquisition stages, and lifecycle pipeline for
          this client.
        </p>
        <Link to={`/organizations/${orgId}/crm/pipeline?client=${clientId}`}>
          <Button className="gap-2">
            <Package className="h-4 w-4" /> Open Pipeline
          </Button>
        </Link>
      </div>
    </div>
  );
}

// ── Platform list for filter bar ─────────────────────────────────────────────
const ALL_PLATFORMS = [
  'linkedin',
  'instagram',
  'twitter',
  'tiktok',
  'threads',
  'facebook',
  'youtube',
  'bluesky',
  'pinterest',
] as const;

// ── Social placeholder ────────────────────────────────────────────────────────
function SocialPlaceholderTab({ projects }: { projects: Project[] }) {
  const [selectedProject, setSelectedProject] = useState('');
  const [selectedPlatforms, setSelectedPlatforms] = useState<string[]>([]);

  const togglePlatform = (pl: string) => {
    setSelectedPlatforms((prev) =>
      prev.includes(pl) ? prev.filter((x) => x !== pl) : [...prev, pl]
    );
  };

  if (projects.length === 0)
    return (
      <div className="text-center py-12 text-muted-foreground">
        <Share2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
        <p className="text-sm">No projects to show social accounts for.</p>
      </div>
    );

  const filteredEntries = selectedProject
    ? projects
        .filter((p) => p.id === selectedProject)
        .map((p) => ({ id: p.id, name: p.name }))
    : projects.map((p) => ({ id: p.id, name: p.name }));

  return (
    <div className="space-y-4">
      {/* Filter bar */}
      <div className="flex items-center gap-3 flex-wrap p-3 rounded-lg border border-border/60 bg-muted/30">
        {/* Project filter */}
        {projects.length > 1 && (
          <select
            value={selectedProject}
            onChange={(e) => setSelectedProject(e.target.value)}
            className="text-xs border border-border rounded-md px-2.5 py-1.5 bg-background text-foreground h-8"
          >
            <option value="">All projects</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        )}

        {/* Platform filter pills */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-xs text-muted-foreground">Platform:</span>
          {ALL_PLATFORMS.map((pl) => {
            const active = selectedPlatforms.includes(pl);
            return (
              <button
                key={pl}
                onClick={() => togglePlatform(pl)}
                className={cn(
                  'px-2.5 py-1 text-xs rounded-md border transition-colors capitalize',
                  active
                    ? 'bg-primary/10 border-primary/30 text-primary'
                    : 'border-border text-muted-foreground hover:text-foreground hover:border-border/80'
                )}
              >
                {pl}
              </button>
            );
          })}
          {selectedPlatforms.length > 0 && (
            <button
              onClick={() => setSelectedPlatforms([])}
              className="px-2 py-1 text-xs rounded-md border border-border/60 text-muted-foreground hover:text-foreground transition-colors"
            >
              Clear
            </button>
          )}
        </div>
      </div>

      <Suspense fallback={<TabSkeleton />}>
        <SocialTab
          projectEntries={filteredEntries}
          platformFilter={
            selectedPlatforms.length > 0 ? selectedPlatforms : undefined
          }
        />
      </Suspense>
    </div>
  );
}

// ── Client integrations — social accounts ─────────────────────────────────────
const PLATFORM_ICONS_LOCAL: Record<
  string,
  React.ComponentType<{ className?: string }>
> = {
  linkedin: ({ className }) => (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433a2.062 2.062 0 01-2.063-2.065 2.064 2.064 0 112.063 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z" />
    </svg>
  ),
  instagram: ({ className }) => (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838a6.162 6.162 0 100 12.324 6.162 6.162 0 000-12.324zM12 16a4 4 0 110-8 4 4 0 010 8zm6.406-11.845a1.44 1.44 0 100 2.881 1.44 1.44 0 000-2.881z" />
    </svg>
  ),
  twitter: ({ className }) => (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  ),
  facebook: ({ className }) => (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M24 12.073c0-6.627-5.373-12-12-12s-12 5.373-12 12c0 5.99 4.388 10.954 10.125 11.854v-8.385H7.078v-3.47h3.047V9.43c0-3.007 1.792-4.669 4.533-4.669 1.312 0 2.686.235 2.686.235v2.953H15.83c-1.491 0-1.956.925-1.956 1.874v2.25h3.328l-.532 3.47h-2.796v8.385C19.612 23.027 24 18.062 24 12.073z" />
    </svg>
  ),
  youtube: ({ className }) => (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M23.495 6.205a3.007 3.007 0 00-2.088-2.088c-1.87-.501-9.396-.501-9.396-.501s-7.507-.01-9.396.501A3.007 3.007 0 00.527 6.205a31.247 31.247 0 00-.522 5.805 31.247 31.247 0 00.522 5.783 3.007 3.007 0 002.088 2.088c1.868.502 9.396.502 9.396.502s7.506 0 9.396-.502a3.007 3.007 0 002.088-2.088 31.247 31.247 0 00.5-5.783 31.247 31.247 0 00-.5-5.805zM9.609 15.601V8.408l6.264 3.602z" />
    </svg>
  ),
  tiktok: ({ className }) => <Globe className={className} />,
};

const PLATFORM_BG_LOCAL: Record<string, string> = {
  linkedin: 'bg-[#0A66C2]/10',
  instagram: 'bg-[#E4405F]/10',
  twitter: 'bg-foreground/10',
  facebook: 'bg-[#1877F2]/10',
  youtube: 'bg-[#FF0000]/10',
  tiktok: 'bg-muted',
};
const PLATFORM_COLOR_LOCAL: Record<string, string> = {
  linkedin: 'text-[#0A66C2]',
  instagram: 'text-[#E4405F]',
  twitter: 'text-foreground',
  facebook: 'text-[#1877F2]',
  youtube: 'text-[#FF0000]',
  tiktok: 'text-foreground',
};

const PLATFORMS_LIST = [
  { key: 'linkedin', label: 'LinkedIn' },
  { key: 'instagram', label: 'Instagram' },
  { key: 'twitter', label: 'X / Twitter' },
  { key: 'tiktok', label: 'TikTok' },
  { key: 'youtube', label: 'YouTube' },
  { key: 'facebook', label: 'Facebook' },
];

function ClientConnectDialog({
  open,
  onClose,
  orgId,
  projects,
}: {
  open: boolean;
  onClose: () => void;
  orgId?: string;
  projects: Project[];
}) {
  const [scope, setScope] = useState<'org' | string>('org');

  const { data: platformStatus = [] } = useQuery({
    queryKey: ['social-platform-status'],
    queryFn: () => socialApi.getPlatformStatus(),
    staleTime: 300_000,
    enabled: open,
  });

  const configuredSet = new Set(
    platformStatus.filter((p) => p.configured).map((p) => p.platform)
  );

  const handleConnect = (platform: string) => {
    if (scope === 'org') {
      socialApi.connectAccount(platform, { orgId });
    } else {
      socialApi.connectAccount(platform, { projectId: scope });
    }
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Connect Social Account</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground font-medium">
            Connect to
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => setScope('org')}
              className={`px-3 py-1.5 text-xs rounded-md border transition-colors ${scope === 'org' ? 'bg-primary text-primary-foreground border-primary' : 'border-border hover:bg-muted'}`}
            >
              Organization
            </button>
            {projects.map((p) => (
              <button
                key={p.id}
                onClick={() => setScope(p.id)}
                className={`px-3 py-1.5 text-xs rounded-md border transition-colors ${scope === p.id ? 'bg-primary text-primary-foreground border-primary' : 'border-border hover:bg-muted'}`}
              >
                {p.name}
              </button>
            ))}
          </div>
        </div>
        <div className="grid grid-cols-2 gap-2 mt-2">
          {PLATFORMS_LIST.map(({ key, label }) => {
            const Icon = PLATFORM_ICONS_LOCAL[key] || Globe;
            const configured =
              configuredSet.size === 0 || configuredSet.has(key);
            return (
              <button
                key={key}
                disabled={!configured}
                onClick={() => configured && handleConnect(key)}
                className={`flex items-center gap-3 p-3 rounded-lg border text-left transition-colors ${configured ? 'border-border hover:bg-muted/60 cursor-pointer' : 'border-border/40 opacity-40 cursor-not-allowed'}`}
              >
                <div
                  className={`h-8 w-8 rounded-full flex items-center justify-center shrink-0 ${PLATFORM_BG_LOCAL[key] || 'bg-muted'}`}
                >
                  <Icon
                    className={`h-4 w-4 ${PLATFORM_COLOR_LOCAL[key] || 'text-muted-foreground'}`}
                  />
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium">{label}</p>
                  <p className="text-[10px] text-muted-foreground">
                    {configured ? 'Click to connect' : 'Not configured'}
                  </p>
                </div>
              </button>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ClientIntegrationsTab({
  orgId,
  projects,
}: {
  orgId?: string;
  projects: Project[];
}) {
  const [connectOpen, setConnectOpen] = useState(false);
  const queryClient = useQueryClient();

  const projectQueries = useQueries({
    queries: projects.map((p) => ({
      queryKey: socialKeys.accounts(p.id),
      queryFn: () => socialApi.listAccounts({ projectId: p.id }),
      staleTime: 60_000,
    })),
  });

  const { data: orgAccounts = [] } = useQuery({
    queryKey: socialKeys.accounts(orgId ?? ''),
    queryFn: () => socialApi.listAccounts({ organizationId: orgId }),
    staleTime: 60_000,
    enabled: !!orgId,
  });

  const allAccounts: (SocialAccountRecord & { _scope: string })[] = [
    ...orgAccounts.map((a) => ({ ...a, _scope: 'Organization' })),
    ...projectQueries.flatMap((q, i) =>
      (q.data ?? []).map((a) => ({
        ...a,
        _scope: projects[i]?.name ?? 'Project',
      }))
    ),
  ].sort((a, b) => a.platform.localeCompare(b.platform));

  const deleteMut = useMutation({
    mutationFn: (id: string) => socialApi.deleteAccount(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: socialKeys.accounts(orgId ?? ''),
      });
      projects.forEach((p) =>
        queryClient.invalidateQueries({ queryKey: socialKeys.accounts(p.id) })
      );
    },
  });

  return (
    <div className="space-y-4">
      <ClientConnectDialog
        open={connectOpen}
        onClose={() => setConnectOpen(false)}
        orgId={orgId}
        projects={projects}
      />

      <Card className="p-4 space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Share2 className="h-4 w-4 text-pink-500" />
            <span className="text-sm font-medium">Social Accounts</span>
            <Badge variant="secondary" className="text-xs">
              {allAccounts.length}
            </Badge>
          </div>
          <Button
            size="sm"
            variant="outline"
            className="h-7 text-xs gap-1.5"
            onClick={() => setConnectOpen(true)}
          >
            <Plus className="h-3.5 w-3.5" />
            Connect Account
          </Button>
        </div>

        {allAccounts.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Share2 className="h-8 w-8 mx-auto mb-3 opacity-30" />
            <p className="text-sm font-medium mb-1">No accounts connected</p>
            <p className="text-xs mb-4">
              Connect social accounts for this client's projects or the
              organization.
            </p>
            <Button
              size="sm"
              variant="outline"
              className="gap-1.5"
              onClick={() => setConnectOpen(true)}
            >
              <Plus className="h-3.5 w-3.5" />
              Connect Account
            </Button>
          </div>
        ) : (
          <div className="space-y-2">
            {allAccounts.map((account) => {
              const Icon = PLATFORM_ICONS_LOCAL[account.platform] || Globe;
              return (
                <div
                  key={account.id}
                  className="flex items-center gap-3 p-3 rounded-lg border border-border/50 hover:bg-muted/30 transition-colors"
                >
                  <div
                    className={`h-9 w-9 rounded-full flex items-center justify-center shrink-0 ${PLATFORM_BG_LOCAL[account.platform] || 'bg-muted'}`}
                  >
                    {account.avatar_url ? (
                      <img
                        src={account.avatar_url}
                        className="h-9 w-9 rounded-full object-cover"
                        alt=""
                      />
                    ) : (
                      <Icon
                        className={`h-4 w-4 ${PLATFORM_COLOR_LOCAL[account.platform] || 'text-muted-foreground'}`}
                      />
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-medium truncate">
                        {account.display_name ||
                          account.username ||
                          account.platform}
                      </p>
                      <Badge
                        variant={
                          account.status === 'active'
                            ? 'default'
                            : account.status === 'error'
                              ? 'destructive'
                              : 'secondary'
                        }
                        className="text-xs"
                      >
                        {account.status}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground capitalize">
                      {account.platform} · {account._scope}
                    </p>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    {account.profile_url && (
                      <a
                        href={account.profile_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="p-1.5 rounded hover:bg-muted transition-colors"
                      >
                        <ExternalLink className="h-3.5 w-3.5 text-muted-foreground" />
                      </a>
                    )}
                    <button
                      onClick={() =>
                        confirm('Disconnect this account?') &&
                        deleteMut.mutate(account.id)
                      }
                      className="p-1.5 rounded hover:bg-destructive/10 transition-colors"
                    >
                      {deleteMut.isPending ? (
                        <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                      ) : (
                        <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                      )}
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Card>
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────
export function ClientOverview() {
  const { orgId, clientId } = useParams<{ orgId: string; clientId: string }>();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const svParam = searchParams.get('sv');
  const validTabs: Tab[] = [
    'overview',
    'projects',
    'contacts',
    'members',
    'pipeline',
    'social',
    'intel',
    'integrations',
  ];
  const [tab, setTab] = useState<Tab>(() =>
    validTabs.includes(svParam as Tab) ? (svParam as Tab) : 'overview'
  );

  const switchTab = (t: Tab) => {
    setTab(t);
    const params = new URLSearchParams(searchParams);
    if (t === 'overview') params.delete('sv');
    else params.set('sv', t);
    setSearchParams(params, { replace: true });
  };
  const [membersOpen, setMembersOpen] = useState(false);

  const { data: client, isLoading } = useQuery<ClientOverviewData>({
    queryKey: ['client', clientId],
    queryFn: () =>
      organizationsApi.getClientWithIntel(
        clientId!
      ) as Promise<ClientOverviewData>,
    enabled: !!clientId,
  });

  const { data: projects = [], isLoading: isProjectsLoading } = useQuery({
    queryKey: projectKeys.clientProjects(clientId!),
    queryFn: () => projectsApi.getByClientId(clientId!),
    enabled: !!clientId,
  });

  const { data: brandProfile } = useQuery<OrgBrandProfile | null>({
    queryKey: organizationKeys.brandProfile(orgId!),
    queryFn: () => organizationsApi.getBrandProfile(orgId!),
    enabled: !!orgId,
    staleTime: 5 * 60_000,
  });

  const allProjects = projects;

  // Brand colors: use org brand profile if available, otherwise hash-derived palette
  const palette =
    CLIENT_PALETTES[hashId(clientId ?? '') % CLIENT_PALETTES.length];
  const brandFrom = brandProfile?.primaryColor ?? palette.from;
  const brandTo = brandProfile?.secondaryColor ?? palette.to;

  if (isLoading)
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader message="Loading client..." size={28} />
      </div>
    );

  if (!client)
    return (
      <div className="flex items-center justify-center min-h-[400px] text-muted-foreground">
        Client not found
      </div>
    );

  return (
    <div className="max-w-6xl mx-auto space-y-0">
      {/* Brand header strip */}
      <div
        className="relative px-6 pt-5 pb-6 rounded-t-xl overflow-hidden"
        style={{
          background: `linear-gradient(135deg, ${brandFrom}22 0%, ${brandTo}15 100%)`,
        }}
      >
        {/* Decorative glow */}
        <div
          className="absolute inset-0 pointer-events-none"
          style={{
            background: `radial-gradient(ellipse at 10% 50%, ${brandFrom}18 0%, transparent 60%)`,
          }}
        />

        {/* Back nav */}
        <button
          onClick={() => navigate(`/organizations/${orgId}`)}
          className="relative flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors mb-4"
        >
          <ArrowLeft className="h-4 w-4" /> Back to Organization
        </button>

        {/* Header row */}
        <div className="relative flex items-start justify-between gap-4">
          <div className="flex items-center gap-4">
            {/* Brand avatar — use KG logo if available */}
            <div
              className="w-14 h-14 rounded-xl flex items-center justify-center text-xl font-semibold text-white shrink-0 shadow-lg overflow-hidden"
              style={{
                background: `linear-gradient(135deg, ${brandFrom}, ${brandTo})`,
              }}
            >
              {(client.company_logo_url ?? client.logo_url) ? (
                <img
                  src={client.company_logo_url ?? client.logo_url}
                  alt={client.name}
                  className="w-full h-full object-contain p-1"
                />
              ) : (
                client.name.charAt(0)
              )}
            </div>
            <div>
              <h1 className="text-2xl font-bold">{client.name}</h1>
              <div className="flex items-center gap-2 mt-1 flex-wrap">
                <Badge variant={client.client_since ? 'default' : 'secondary'}>
                  {client.client_since
                    ? 'Client'
                    : client.prospect_at
                      ? 'Prospect'
                      : 'Active'}
                </Badge>
                {client.company_industry && (
                  <span className="text-xs text-muted-foreground">
                    {client.company_industry}
                  </span>
                )}
                <span className="text-xs text-muted-foreground font-mono">
                  {client.slug}
                </span>
                <span className="text-xs text-muted-foreground">
                  {allProjects.length} project
                  {allProjects.length !== 1 ? 's' : ''}
                </span>
              </div>
            </div>
          </div>

          {/* Header action buttons */}
          <div className="flex gap-2 shrink-0 flex-wrap justify-end">
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5 border-indigo-700/60 text-indigo-400 hover:bg-indigo-950/40"
              onClick={() => switchTab('intel')}
            >
              <Brain className="h-4 w-4" /> Intel
            </Button>
            <Link
              to={
                client.company_id
                  ? `/companies/${client.company_id}/brand-guide`
                  : `/organizations/${orgId}/brand-guide`
              }
            >
              <Button variant="outline" size="sm" className="gap-1.5">
                <BookOpen className="h-4 w-4" /> Brand Guide
              </Button>
            </Link>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setMembersOpen(true)}
            >
              <Users className="h-4 w-4 mr-1" /> Members
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                NiceModal.show('convert-entity', {
                  sourceType: 'client',
                  sourceId: client.id,
                  sourceName: client.name,
                })
              }
            >
              Convert to…
            </Button>
          </div>
        </div>
      </div>

      {/* Tabs + content */}
      <div className="px-6 pb-6 space-y-6 border border-t-0 border-border/60 rounded-b-xl">
        <div className="flex gap-1 border-b border-border pb-1 pt-4 flex-wrap -mx-0">
          <TabBtn
            active={tab === 'overview'}
            onClick={() => switchTab('overview')}
            icon={BarChart3}
          >
            Overview
          </TabBtn>
          <TabBtn
            active={tab === 'projects'}
            onClick={() => switchTab('projects')}
            icon={FolderKanban}
          >
            Projects{allProjects.length > 0 && ` (${allProjects.length})`}
          </TabBtn>
          <TabBtn
            active={tab === 'contacts'}
            onClick={() => switchTab('contacts')}
            icon={Users}
          >
            People
          </TabBtn>
          <TabBtn
            active={tab === 'members'}
            onClick={() => switchTab('members')}
            icon={Users}
          >
            Members
          </TabBtn>
          <TabBtn
            active={tab === 'pipeline'}
            onClick={() => switchTab('pipeline')}
            icon={Package}
          >
            Pipeline
          </TabBtn>
          <TabBtn
            active={tab === 'social'}
            onClick={() => switchTab('social')}
            icon={Share2}
          >
            Social
          </TabBtn>
          <TabBtn
            active={tab === 'intel'}
            onClick={() => switchTab('intel')}
            icon={Brain}
          >
            Intel
          </TabBtn>
          <TabBtn
            active={tab === 'integrations'}
            onClick={() => switchTab('integrations')}
            icon={Plug}
          >
            Integrations
          </TabBtn>
        </div>

        <div>
          {tab === 'overview' && (
            <OverviewTab
              client={client}
              projects={allProjects}
              orgId={orgId!}
            />
          )}
          {tab === 'projects' && (
            <ProjectsTab
              projects={allProjects}
              isProjectsLoading={isProjectsLoading}
              orgId={orgId!}
              clientId={clientId!}
            />
          )}
          {tab === 'contacts' && (
            <ClientContactsTab orgId={orgId!} clientName={client.name} />
          )}
          {tab === 'members' && (
            <ClientMembersTab clientId={clientId!} orgId={orgId!} />
          )}
          {tab === 'pipeline' && (
            <PipelineTab orgId={orgId!} clientId={clientId!} />
          )}
          {tab === 'social' && <SocialPlaceholderTab projects={allProjects} />}
          {tab === 'intel' &&
            (client.company_id ? (
              <div className="space-y-4">
                <div className="flex gap-2 flex-wrap">
                  <Link to={`/companies/${client.company_id}/intel`}>
                    <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-indigo-700/60 text-indigo-400 hover:bg-indigo-950/40 text-xs font-medium transition-colors">
                      <Brain className="h-3.5 w-3.5" /> Company Intel →
                    </button>
                  </Link>
                  {client.primary_person_id && (
                    <Link to={`/people/${client.primary_person_id}/intel`}>
                      <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-border text-muted-foreground hover:text-foreground text-xs font-medium transition-colors">
                        <Users className="h-3.5 w-3.5" /> Person Intel →
                      </button>
                    </Link>
                  )}
                </div>
                {client.crm_person_id && (
                  <PersonIntelPage
                    personId={client.crm_person_id}
                    embedded={true}
                  />
                )}
              </div>
            ) : client.crm_person_id ? (
              <PersonIntelPage
                personId={client.crm_person_id}
                embedded={true}
              />
            ) : (
              <div className="text-center py-16 text-muted-foreground">
                <Brain className="h-10 w-10 mx-auto mb-3 opacity-20" />
                <p className="text-sm font-medium mb-1">
                  No Knowledge Graph Link
                </p>
                <p className="text-xs opacity-70 max-w-xs mx-auto">
                  Link this client to a company in the Knowledge Graph to access
                  intel.
                </p>
              </div>
            ))}
          {tab === 'integrations' && (
            <ClientIntegrationsTab orgId={orgId} projects={allProjects} />
          )}
        </div>
      </div>

      {clientId && (
        <ClientMembersDialog
          open={membersOpen}
          onOpenChange={setMembersOpen}
          clientId={clientId}
          clientName={client.name}
        />
      )}
    </div>
  );
}

export default ClientOverview;
