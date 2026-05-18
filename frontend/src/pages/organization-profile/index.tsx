import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeft,
  BookOpen,
  Brain,
  Briefcase,
  Building2,
  CheckCircle2,
  Contact2,
  Crosshair,
  Database,
  DollarSign,
  FolderOpen,
  Layers,
  LayoutGrid,
  Lightbulb,
  Link as LinkIcon,
  Loader2,
  MapPin,
  Megaphone,
  Palette,
  Pencil,
  Plug,
  RefreshCw,
  Share2,
  Target,
  Users,
  Zap,
} from 'lucide-react';
import { lazy, Suspense, useMemo, useState } from 'react';
import {
  Link,
  useNavigate,
  useParams,
  useSearchParams,
} from 'react-router-dom';

import { PageErrorBoundary } from '@/components/PageErrorBoundary';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { useCrmContacts, useOrganizationById } from '@/hooks/queries';
import {
  type ClientData,
  crmDealsApi,
  organizationsApi,
  type OrgBrandProfile,
  resolveApiUrl,
  type SidebarClient,
  type SidebarProject,
} from '@/lib/api';
import { organizationKeys, sidebarKeys } from '@/lib/query-keys';

import { StatPill } from './components/StatPill';
import {
  BRAND_ARCHETYPE_OPTIONS,
  BRAND_VOICE_OPTIONS,
  ICP_COMPANY_SIZE_OPTIONS,
  MARKET_POSITION_OPTIONS,
} from './constants';
import { formatCurrency, parseJsonArray } from './helpers';
import type { OrganizationProfilePageProps, OrgMember } from './types';

// Re-export BrandIdentityCard for external consumers
export { BrandIdentityCard } from './components/BrandIdentityCard';

// ── Lazy-loaded tabs ─────────────────────────────────────────────────────────

const OverviewTab = lazy(() =>
  import('./tabs/OverviewTab').then((m) => ({ default: m.OverviewTab }))
);
const PipelinesTab = lazy(() =>
  import('./tabs/PipelinesTab').then((m) => ({ default: m.PipelinesTab }))
);
const ContactsTab = lazy(() =>
  import('./tabs/ContactsTab').then((m) => ({ default: m.ContactsTab }))
);
const ProjectsTab = lazy(() =>
  import('./tabs/ProjectsTab').then((m) => ({ default: m.ProjectsTab }))
);
const MembersTab = lazy(() =>
  import('./tabs/MembersTab').then((m) => ({ default: m.MembersTab }))
);
const SocialTab = lazy(() => import('./tabs/social'));
const IntelligenceTab = lazy(() => import('./tabs/intelligence'));
const IntegrationsTab = lazy(() => import('./tabs/integrations'));
const OrgWikiTab = lazy(() =>
  import('./tabs/WikiTab').then((m) => ({ default: m.OrgWikiTab }))
);
const CloudTab = lazy(() => import('./tabs/cloud'));
const KnowledgeTab = lazy(() =>
  import('./tabs/KnowledgeTab').then((m) => ({ default: m.KnowledgeTab }))
);

function TabSkeleton() {
  return (
    <div className="flex items-center justify-center py-16 text-muted-foreground">
      <Loader2 className="h-5 w-5 animate-spin mr-2" />
      Loading…
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export function OrganizationProfilePage({
  defaultTab,
  defaultPipeline,
}: OrganizationProfilePageProps = {}) {
  const { orgId } = useParams<{ orgId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();

  const tabFromUrl = searchParams.get('tab') || defaultTab || 'overview';
  const pipelineFromUrl = searchParams.get('pipeline') || defaultPipeline;
  const clientFilter = searchParams.get('client');
  const _viewFromUrl = searchParams.get('view');
  void _viewFromUrl;

  const TAB_PATHS: Record<string, string> = {
    overview: '',
    pipelines: '/crm/pipeline',
    contacts: '/crm/contacts',
    projects: '/projects',
    social: '/social',
    intelligence: '/intelligence',
    knowledge: '/knowledge',
    wiki: '/wiki',
    members: '/members',
    integrations: '/integrations',
    cloud: '/cloud',
  };

  const setTab = (tab: string) => {
    const basePath = `/organizations/${orgId}`;
    const tabPath = TAB_PATHS[tab] ?? '';
    const params = new URLSearchParams();
    if (tab === 'pipelines' && pipelineFromUrl)
      params.set('pipeline', pipelineFromUrl);
    if (tab === 'projects' && clientFilter) params.set('client', clientFilter);
    const qs = params.toString();
    navigate(`${basePath}${tabPath}${qs ? `?${qs}` : ''}`, { replace: true });
  };

  const clearClientFilter = () => {
    const params = new URLSearchParams(searchParams);
    params.delete('client');
    setSearchParams(params, { replace: true });
  };

  const { data: org, isLoading: orgLoading } = useOrganizationById(orgId);

  const { data: members = [] } = useQuery<OrgMember[]>({
    queryKey: organizationKeys.members(orgId!),
    queryFn: () => organizationsApi.getMembers(orgId!),
    enabled: !!orgId,
  });

  const { data: clients = [] } = useQuery<ClientData[]>({
    queryKey: organizationKeys.clients(orgId),
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const { data: sidebarTree } = useQuery({
    queryKey: sidebarKeys.tree(),
    queryFn: () => organizationsApi.getSidebarTree(),
    staleTime: 60_000,
  });

  const { data: orgDeals = [] } = useQuery({
    queryKey: organizationKeys.deals(orgId!),
    queryFn: () => crmDealsApi.listOrgDeals(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const { data: crmContacts = [] } = useCrmContacts(orgId);

  const qc = useQueryClient();
  const { data: brandProfile } = useQuery<OrgBrandProfile | null>({
    queryKey: organizationKeys.brandProfile(orgId!),
    queryFn: () => organizationsApi.getBrandProfile(orgId!),
    enabled: !!orgId,
    staleTime: 5 * 60_000,
  });
  const [brandEditing, setBrandEditing] = useState(false);
  const [brandForm, setBrandForm] = useState<Partial<OrgBrandProfile>>({});
  const [brandSaving, setBrandSaving] = useState(false);

  const openBrandEdit = () => {
    setBrandForm({
      tagline: brandProfile?.tagline ?? '',
      primaryColor: brandProfile?.primaryColor ?? '#2563EB',
      secondaryColor: brandProfile?.secondaryColor ?? '#EC4899',
      accentColor: brandProfile?.accentColor ?? '',
      typographyHeading: brandProfile?.typographyHeading ?? '',
      typographyBody: brandProfile?.typographyBody ?? '',
      logoUrl: brandProfile?.logoUrl ?? '',
      industry: brandProfile?.industry ?? '',
      marketPosition: brandProfile?.marketPosition ?? '',
      uniqueValueProposition: brandProfile?.uniqueValueProposition ?? '',
      missionStatement: brandProfile?.missionStatement ?? '',
      visionStatement: brandProfile?.visionStatement ?? '',
      brandValues: brandProfile?.brandValues ?? '[]',
      brandVoice: brandProfile?.brandVoice ?? '',
      brandArchetype: brandProfile?.brandArchetype ?? '',
      targetAudience: brandProfile?.targetAudience ?? '',
      icpDescription: brandProfile?.icpDescription ?? '',
      icpCompanySize: brandProfile?.icpCompanySize ?? '',
      icpIndustries: brandProfile?.icpIndustries ?? '[]',
      competitorBrands: brandProfile?.competitorBrands ?? '[]',
      differentiators: brandProfile?.differentiators ?? '[]',
      contentPillars: brandProfile?.contentPillars ?? '[]',
      contentTone: brandProfile?.contentTone ?? '',
      websiteUrl: brandProfile?.websiteUrl ?? '',
      socialInstagram: brandProfile?.socialInstagram ?? '',
      socialTwitter: brandProfile?.socialTwitter ?? '',
      socialLinkedin: brandProfile?.socialLinkedin ?? '',
      socialFacebook: brandProfile?.socialFacebook ?? '',
      socialYoutube: brandProfile?.socialYoutube ?? '',
      socialTiktok: brandProfile?.socialTiktok ?? '',
    });
    setBrandEditing(true);
  };
  const saveBrand = async () => {
    setBrandSaving(true);
    try {
      await organizationsApi.upsertBrandProfile(orgId!, brandForm);
      qc.invalidateQueries({ queryKey: organizationKeys.brandProfile(orgId!) });
      setBrandEditing(false);
    } finally {
      setBrandSaving(false);
    }
  };
  const bset = (k: keyof OrgBrandProfile, v: string) =>
    setBrandForm((f) => ({ ...f, [k]: v }));
  const bgetArr = (k: keyof OrgBrandProfile) =>
    parseJsonArray(brandForm[k] as string).join(', ');
  const bsetArr = (k: keyof OrgBrandProfile, v: string) =>
    bset(
      k,
      JSON.stringify(
        v
          .split(',')
          .map((s: string) => s.trim())
          .filter(Boolean)
      )
    );

  // ── Brand research ──────────────────────────────────────────────────────────
  const [brandResearching, setBrandResearching] = useState(false);
  const [intakeUrl, setIntakeUrl] = useState<string | null>(null);
  const [intakeCopied, setIntakeCopied] = useState(false);

  const triggerResearch = async () => {
    if (!orgId) return;
    setBrandResearching(true);
    try {
      await organizationsApi.triggerBrandResearch(orgId);
      const poll = setInterval(async () => {
        const status = await organizationsApi.getBrandResearchStatus(orgId);
        if (status.status === 'done' || status.status === 'failed') {
          clearInterval(poll);
          setBrandResearching(false);
          qc.invalidateQueries({
            queryKey: organizationKeys.brandProfile(orgId!),
          });
        }
      }, 3000);
      setTimeout(() => {
        clearInterval(poll);
        setBrandResearching(false);
      }, 120_000);
    } catch {
      setBrandResearching(false);
    }
  };

  const generateIntake = async () => {
    if (!orgId) return;
    const result = await organizationsApi.generateIntakeToken(orgId);
    setIntakeUrl(result.url);
  };

  const copyIntake = () => {
    if (!intakeUrl) return;
    navigator.clipboard.writeText(intakeUrl);
    setIntakeCopied(true);
    setTimeout(() => setIntakeCopied(false), 2000);
  };

  const sidebarOrg = sidebarTree
    ? [
        ...(sidebarTree.owned_orgs || []),
        ...(sidebarTree.member_orgs || []),
      ].find((o) => o.id === orgId)
    : undefined;

  const allProjects = useMemo(() => {
    if (!sidebarOrg) return [];
    const collectProjects = (projects: SidebarProject[]): SidebarProject[] =>
      projects.flatMap((p) => [p, ...collectProjects(p.children || [])]);
    return [
      ...collectProjects(sidebarOrg.internal_projects || []),
      ...(sidebarOrg.clients || []).flatMap((c: SidebarClient) =>
        collectProjects(c.projects || [])
      ),
    ];
  }, [sidebarOrg]);

  const totalDealValue = useMemo(
    () => orgDeals.reduce((sum, d) => sum + (d.amount || 0), 0),
    [orgDeals]
  );

  if (orgLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Loading organisation...</div>
      </div>
    );
  }

  if (!org || !orgId) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 text-center">
        <Building2 className="w-12 h-12 text-muted-foreground/50" />
        <div>
          <h2 className="text-lg font-semibold">Organization not found</h2>
          <p className="text-sm text-muted-foreground mt-1">
            This organization doesn't exist or you don't have access to it.
          </p>
        </div>
        <Link to="/projects" className="text-sm text-primary hover:underline">
          ← Back to your projects
        </Link>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="border-b shadow-sm overflow-hidden">
        {brandProfile && (
          <div
            className="h-[3px]"
            style={{
              background: `linear-gradient(90deg, ${brandProfile.primaryColor} 0%, ${brandProfile.accentColor ?? brandProfile.secondaryColor} 100%)`,
            }}
          />
        )}
        <div className="bg-card/50 backdrop-blur-sm">
          <div className="max-w-[1600px] mx-auto px-6 py-4">
            <div className="flex items-center gap-4 flex-wrap">
              <Link
                to="/projects"
                className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
              >
                <ArrowLeft className="h-5 w-5" />
              </Link>

              {brandProfile?.logoUrl ? (
                <div
                  className="h-11 w-11 rounded-xl shrink-0 shadow-sm overflow-hidden flex items-center justify-center"
                  style={{ backgroundColor: brandProfile.primaryColor }}
                >
                  <img
                    src={resolveApiUrl(brandProfile.logoUrl)}
                    alt={org.name}
                    className="h-full w-full object-contain p-1"
                  />
                </div>
              ) : brandProfile ? (
                <div
                  className="h-11 w-11 rounded-xl flex items-center justify-center text-white text-sm font-semibold shrink-0 shadow-sm select-none"
                  style={{
                    background: `linear-gradient(135deg, ${brandProfile.primaryColor}, ${brandProfile.accentColor ?? brandProfile.secondaryColor})`,
                  }}
                >
                  {org.name
                    .split(' ')
                    .slice(0, 2)
                    .map((w: string) => w[0])
                    .join('')
                    .toUpperCase()}
                </div>
              ) : (
                <div className="p-2.5 bg-blue-100 dark:bg-blue-950 rounded-xl shrink-0">
                  <Building2 className="w-6 h-6 text-blue-600" />
                </div>
              )}

              <div className="min-w-0 flex-1">
                <div className="flex items-baseline gap-2.5 flex-wrap">
                  <h1 className="text-2xl font-bold text-foreground leading-none">
                    {org.name}
                  </h1>
                  {brandProfile?.typographyHeading && (
                    <span className="text-xs text-muted-foreground tracking-widest uppercase font-medium">
                      {brandProfile.typographyHeading}
                    </span>
                  )}
                </div>
                {brandProfile?.tagline ? (
                  <p className="text-sm text-muted-foreground mt-0.5 italic truncate">
                    "{brandProfile.tagline}"
                  </p>
                ) : org.description ? (
                  <p className="text-sm text-muted-foreground mt-0.5 truncate">
                    {org.description}
                  </p>
                ) : null}
                {org.address && (
                  <p className="flex items-center gap-1 text-xs text-muted-foreground mt-0.5">
                    <MapPin className="h-3 w-3 shrink-0" />
                    {org.address}
                  </p>
                )}
                {brandProfile && (
                  <div className="flex items-center gap-2.5 mt-1.5 flex-wrap">
                    <div className="flex items-center gap-1">
                      <div
                        className="h-3.5 w-3.5 rounded-full border border-border/60 shadow-sm"
                        style={{ backgroundColor: brandProfile.primaryColor }}
                        title={`Primary: ${brandProfile.primaryColor}`}
                      />
                      <div
                        className="h-3.5 w-3.5 rounded-full border border-border/60 shadow-sm"
                        style={{ backgroundColor: brandProfile.secondaryColor }}
                        title={`Secondary: ${brandProfile.secondaryColor}`}
                      />
                      {brandProfile.accentColor && (
                        <div
                          className="h-3.5 w-3.5 rounded-full border border-border/60 shadow-sm"
                          style={{ backgroundColor: brandProfile.accentColor }}
                          title={`Accent: ${brandProfile.accentColor}`}
                        />
                      )}
                    </div>
                    {brandProfile.industry && (
                      <Badge variant="secondary" className="text-xs h-4 py-0">
                        {brandProfile.industry}
                      </Badge>
                    )}
                    {brandProfile.marketPosition && (
                      <Badge
                        variant="outline"
                        className="text-xs h-4 py-0 capitalize"
                      >
                        {brandProfile.marketPosition}
                      </Badge>
                    )}
                    {brandProfile.brandArchetype && (
                      <Badge className="text-xs h-4 py-0 bg-amber-500/10 text-amber-700 dark:text-amber-400 border-amber-500/20">
                        {brandProfile.brandArchetype}
                      </Badge>
                    )}
                  </div>
                )}
              </div>

              <div className="ml-auto flex items-center gap-2 shrink-0 flex-wrap">
                <Badge
                  variant={org.is_active ? 'default' : 'secondary'}
                  className="text-xs"
                >
                  {org.is_active ? 'Active' : 'Inactive'}
                </Badge>

                {brandProfile &&
                  (() => {
                    const scores = [
                      brandProfile.logoUrl ? 3 : 0,
                      brandProfile.accentColor
                        ? 3
                        : brandProfile.primaryColor &&
                            brandProfile.secondaryColor
                          ? 2
                          : 1,
                      brandProfile.typographyBody
                        ? 3
                        : brandProfile.typographyHeading
                          ? 2
                          : 0,
                      brandProfile.brandVoice &&
                      brandProfile.brandArchetype &&
                      brandProfile.contentTone
                        ? 3
                        : brandProfile.brandVoice
                          ? 1
                          : 0,
                      brandProfile.missionStatement &&
                      brandProfile.uniqueValueProposition &&
                      brandProfile.brandValues
                        ? 3
                        : brandProfile.missionStatement
                          ? 1
                          : 0,
                      brandProfile.targetAudience && brandProfile.icpDescription
                        ? 3
                        : brandProfile.targetAudience
                          ? 1
                          : 0,
                      brandProfile.websiteUrl &&
                      brandProfile.socialLinkedin &&
                      brandProfile.socialInstagram
                        ? 3
                        : brandProfile.websiteUrl
                          ? 1
                          : 0,
                    ];
                    const pct = Math.round(
                      (scores.reduce((a, b) => a + b, 0) /
                        (scores.length * 3)) *
                        100
                    );
                    const color =
                      pct >= 80 ? '#22c55e' : pct >= 50 ? '#f59e0b' : '#ef4444';
                    const r = 10;
                    const circ = 2 * Math.PI * r;
                    return (
                      <button
                        onClick={openBrandEdit}
                        title={`Brand completeness: ${pct}%`}
                        className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
                      >
                        <svg width="28" height="28" viewBox="0 0 28 28">
                          <circle
                            cx="14"
                            cy="14"
                            r={r}
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="3"
                            className="text-border"
                          />
                          <circle
                            cx="14"
                            cy="14"
                            r={r}
                            fill="none"
                            stroke={color}
                            strokeWidth="3"
                            strokeDasharray={`${(circ * pct) / 100} ${circ}`}
                            strokeLinecap="round"
                            transform="rotate(-90 14 14)"
                          />
                          <text
                            x="14"
                            y="14"
                            textAnchor="middle"
                            dominantBaseline="central"
                            fontSize="7"
                            fontWeight="600"
                            fill={color}
                          >
                            {pct}%
                          </text>
                        </svg>
                      </button>
                    );
                  })()}

                <Button
                  variant="outline"
                  size="sm"
                  onClick={brandResearching ? undefined : triggerResearch}
                  disabled={brandResearching}
                  className="text-xs gap-1.5"
                  title="Research brand presence with Exa"
                >
                  {brandResearching ? (
                    <>
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      Researching…
                    </>
                  ) : (
                    <>
                      <RefreshCw className="h-3.5 w-3.5" />
                      Research
                    </>
                  )}
                </Button>

                {brandProfile ? (
                  <>
                    <Link to={`/organizations/${orgId}/brand-guide`}>
                      <Button
                        variant="outline"
                        size="sm"
                        className="text-xs gap-1.5"
                      >
                        <BookOpen className="h-3.5 w-3.5" />
                        View Guide
                      </Button>
                    </Link>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={openBrandEdit}
                      className="text-xs gap-1.5 text-muted-foreground"
                    >
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                  </>
                ) : (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={openBrandEdit}
                    className="text-xs gap-1.5"
                  >
                    <Palette className="h-3.5 w-3.5" />
                    Set Up Brand
                  </Button>
                )}
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs gap-1.5 border-indigo-700/60 text-indigo-400 hover:bg-indigo-950/40"
                  onClick={() => setTab('wiki')}
                >
                  <Brain className="h-3.5 w-3.5" />
                  Intel
                </Button>
              </div>
            </div>

            <div className="hidden sm:flex items-center gap-3 mt-4 flex-wrap">
              <StatPill
                icon={FolderOpen}
                label="Projects"
                value={allProjects.length}
              />
              <StatPill
                icon={Briefcase}
                label="Clients"
                value={clients.length}
              />
              <StatPill icon={Users} label="Members" value={members.length} />
              <StatPill
                icon={DollarSign}
                label="Pipeline"
                value={formatCurrency(totalDealValue)}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-[1600px] mx-auto px-6 py-5">
          <Tabs value={tabFromUrl} onValueChange={setTab}>
            <div className="flex items-center justify-between mb-6 gap-2">
              <TabsList className="overflow-x-auto max-w-full">
                <TabsTrigger value="overview">
                  <LayoutGrid className="h-4 w-4 mr-2" />
                  Overview
                </TabsTrigger>
                <TabsTrigger value="pipelines">
                  <Target className="h-4 w-4 mr-2" />
                  Pipelines
                </TabsTrigger>
                <TabsTrigger value="contacts">
                  <Contact2 className="h-4 w-4 mr-2" />
                  Contacts
                </TabsTrigger>
                <TabsTrigger value="projects">
                  <FolderOpen className="h-4 w-4 mr-2" />
                  Projects
                </TabsTrigger>
                <TabsTrigger value="social">
                  <Share2 className="h-4 w-4 mr-2" />
                  Social
                </TabsTrigger>
                <TabsTrigger value="intelligence">
                  <Brain className="h-4 w-4 mr-2" />
                  Intelligence
                </TabsTrigger>
                <TabsTrigger value="knowledge">
                  <Layers className="h-4 w-4 mr-2" />
                  Knowledge
                </TabsTrigger>
                <TabsTrigger value="wiki">
                  <BookOpen className="h-4 w-4 mr-2" />
                  Wiki
                </TabsTrigger>
                <TabsTrigger value="members">
                  <Users className="h-4 w-4 mr-2" />
                  Members
                </TabsTrigger>
                <TabsTrigger value="integrations">
                  <Plug className="h-4 w-4 mr-2" />
                  Integrations
                </TabsTrigger>
                <TabsTrigger value="cloud">
                  <Database className="h-4 w-4 mr-2" />
                  Cloud
                </TabsTrigger>
              </TabsList>
              <Link
                to={`/organizations/${orgId}/data-sources`}
                className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground border rounded-md px-3 py-1.5 transition-colors"
              >
                <Database className="h-3.5 w-3.5" />
                Data Library
              </Link>
            </div>

            <TabsContent value="overview">
              <PageErrorBoundary label="Overview">
                <Suspense fallback={<TabSkeleton />}>
                  <OverviewTab
                    orgId={orgId}
                    projectEntries={allProjects.map((p) => ({
                      id: p.id,
                      name: p.name,
                    }))}
                    projectCount={allProjects.length}
                    totalDealValue={totalDealValue}
                    totalDeals={orgDeals.length}
                    contactCount={crmContacts.length}
                  />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="pipelines">
              <PageErrorBoundary label="Pipelines">
                <Suspense fallback={<TabSkeleton />}>
                  <PipelinesTab
                    orgId={orgId}
                    defaultPipeline={pipelineFromUrl || undefined}
                  />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="contacts">
              <PageErrorBoundary label="Contacts">
                <Suspense fallback={<TabSkeleton />}>
                  <ContactsTab orgId={orgId} />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="projects">
              <PageErrorBoundary label="Projects">
                <Suspense fallback={<TabSkeleton />}>
                  <ProjectsTab
                    orgId={orgId}
                    sidebarOrg={sidebarOrg ?? null}
                    clientFilter={clientFilter}
                    onClearClientFilter={clearClientFilter}
                  />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="social">
              <PageErrorBoundary label="Social">
                <Suspense fallback={<TabSkeleton />}>
                  <SocialTab
                    projectEntries={allProjects.map((p) => ({
                      id: p.id,
                      name: p.name,
                    }))}
                    orgId={orgId}
                  />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="intelligence">
              <PageErrorBoundary label="Intelligence">
                <Suspense fallback={<TabSkeleton />}>
                  <IntelligenceTab
                    projectEntries={allProjects.map((p) => ({
                      id: p.id,
                      name: p.name,
                    }))}
                    orgId={orgId}
                  />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="knowledge">
              <PageErrorBoundary label="Knowledge">
                <Suspense fallback={<TabSkeleton />}>
                  <KnowledgeTab orgId={orgId} />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="wiki">
              <PageErrorBoundary label="Wiki">
                <Suspense fallback={<TabSkeleton />}>
                  <OrgWikiTab orgId={orgId} orgName={org.name} />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="members">
              <PageErrorBoundary label="Members">
                <Suspense fallback={<TabSkeleton />}>
                  <MembersTab orgId={orgId} orgName={org.name} />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="integrations">
              <PageErrorBoundary label="Integrations">
                <Suspense fallback={<TabSkeleton />}>
                  <IntegrationsTab orgId={orgId} />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="cloud">
              <PageErrorBoundary label="Cloud">
                <Suspense fallback={<TabSkeleton />}>
                  <CloudTab orgId={orgId} />
                </Suspense>
              </PageErrorBoundary>
            </TabsContent>
          </Tabs>
        </div>
      </div>

      {/* Brand Guide Edit Dialog */}
      <Dialog open={brandEditing} onOpenChange={setBrandEditing}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Palette className="h-5 w-5 text-amber-500" />
              Brand Guide — {org.name}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-6 py-2">
            {/* Visual */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5">
                <Palette className="h-3 w-3" /> Visual Identity
              </p>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label className="text-xs">Tagline</Label>
                  <Input
                    value={brandForm.tagline ?? ''}
                    onChange={(e) => bset('tagline', e.target.value)}
                    placeholder="One-liner that captures the brand"
                  />
                </div>
                <div>
                  <Label className="text-xs">Industry</Label>
                  <Input
                    value={brandForm.industry ?? ''}
                    onChange={(e) => bset('industry', e.target.value)}
                    placeholder="e.g. Luxury Agency, SaaS"
                  />
                </div>
                <div>
                  <Label className="text-xs">Primary Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input
                      type="color"
                      value={brandForm.primaryColor ?? '#2563EB'}
                      onChange={(e) => bset('primaryColor', e.target.value)}
                      className="h-9 w-12 rounded border cursor-pointer"
                    />
                    <Input
                      value={brandForm.primaryColor ?? ''}
                      onChange={(e) => bset('primaryColor', e.target.value)}
                      placeholder="#2563EB"
                      className="font-mono"
                    />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Secondary Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input
                      type="color"
                      value={brandForm.secondaryColor ?? '#EC4899'}
                      onChange={(e) => bset('secondaryColor', e.target.value)}
                      className="h-9 w-12 rounded border cursor-pointer"
                    />
                    <Input
                      value={brandForm.secondaryColor ?? ''}
                      onChange={(e) => bset('secondaryColor', e.target.value)}
                      placeholder="#EC4899"
                      className="font-mono"
                    />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Accent Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input
                      type="color"
                      value={brandForm.accentColor ?? '#000000'}
                      onChange={(e) => bset('accentColor', e.target.value)}
                      className="h-9 w-12 rounded border cursor-pointer"
                    />
                    <Input
                      value={brandForm.accentColor ?? ''}
                      onChange={(e) => bset('accentColor', e.target.value)}
                      placeholder="#000000"
                      className="font-mono"
                    />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Heading Font</Label>
                  <Input
                    value={brandForm.typographyHeading ?? ''}
                    onChange={(e) => bset('typographyHeading', e.target.value)}
                    placeholder="e.g. Cinzel, Playfair Display"
                  />
                </div>
                <div>
                  <Label className="text-xs">Body Font</Label>
                  <Input
                    value={brandForm.typographyBody ?? ''}
                    onChange={(e) => bset('typographyBody', e.target.value)}
                    placeholder="e.g. Inter, DM Sans"
                  />
                </div>
                <div>
                  <Label className="text-xs">Logo URL</Label>
                  <Input
                    value={brandForm.logoUrl ?? ''}
                    onChange={(e) => bset('logoUrl', e.target.value)}
                    placeholder="https://..."
                  />
                </div>
              </div>
            </div>
            {/* Positioning */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5">
                <Lightbulb className="h-3 w-3" /> Positioning
              </p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2">
                  <Label className="text-xs">Unique Value Proposition</Label>
                  <Input
                    value={brandForm.uniqueValueProposition ?? ''}
                    onChange={(e) =>
                      bset('uniqueValueProposition', e.target.value)
                    }
                    placeholder="What makes this brand irreplaceable?"
                  />
                </div>
                <div className="col-span-2">
                  <Label className="text-xs">Mission Statement</Label>
                  <Textarea
                    value={brandForm.missionStatement ?? ''}
                    onChange={(e) => bset('missionStatement', e.target.value)}
                    placeholder="Why does this brand exist?"
                    rows={2}
                  />
                </div>
                <div className="col-span-2">
                  <Label className="text-xs">Vision Statement</Label>
                  <Textarea
                    value={brandForm.visionStatement ?? ''}
                    onChange={(e) => bset('visionStatement', e.target.value)}
                    placeholder="Where is this brand going?"
                    rows={2}
                  />
                </div>
                <div>
                  <Label className="text-xs">Brand Voice</Label>
                  <Select
                    value={brandForm.brandVoice ?? ''}
                    onValueChange={(v) => bset('brandVoice', v)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select voice" />
                    </SelectTrigger>
                    <SelectContent>
                      {BRAND_VOICE_OPTIONS.map((o) => (
                        <SelectItem key={o} value={o} className="capitalize">
                          {o}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Brand Archetype</Label>
                  <Select
                    value={brandForm.brandArchetype ?? ''}
                    onValueChange={(v) => bset('brandArchetype', v)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select archetype" />
                    </SelectTrigger>
                    <SelectContent>
                      {BRAND_ARCHETYPE_OPTIONS.map((o) => (
                        <SelectItem key={o} value={o}>
                          {o}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Market Position</Label>
                  <Select
                    value={brandForm.marketPosition ?? ''}
                    onValueChange={(v) => bset('marketPosition', v)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select position" />
                    </SelectTrigger>
                    <SelectContent>
                      {MARKET_POSITION_OPTIONS.map((o) => (
                        <SelectItem key={o} value={o} className="capitalize">
                          {o}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">
                    Brand Values{' '}
                    <span className="text-muted-foreground font-normal">
                      (comma-separated)
                    </span>
                  </Label>
                  <Input
                    value={bgetArr('brandValues')}
                    onChange={(e) => bsetArr('brandValues', e.target.value)}
                    placeholder="Integrity, Innovation, Excellence"
                  />
                </div>
              </div>
            </div>
            {/* Audience */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5">
                <Crosshair className="h-3 w-3" /> Audience & ICP
              </p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2">
                  <Label className="text-xs">Target Audience</Label>
                  <Textarea
                    value={brandForm.targetAudience ?? ''}
                    onChange={(e) => bset('targetAudience', e.target.value)}
                    placeholder="Who is this brand speaking to?"
                    rows={2}
                  />
                </div>
                <div className="col-span-2">
                  <Label className="text-xs">
                    Ideal Customer Profile (ICP)
                  </Label>
                  <Textarea
                    value={brandForm.icpDescription ?? ''}
                    onChange={(e) => bset('icpDescription', e.target.value)}
                    placeholder="Describe the perfect client in detail"
                    rows={3}
                  />
                </div>
                <div>
                  <Label className="text-xs">ICP Company Size</Label>
                  <Select
                    value={brandForm.icpCompanySize ?? ''}
                    onValueChange={(v) => bset('icpCompanySize', v)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select size" />
                    </SelectTrigger>
                    <SelectContent>
                      {ICP_COMPANY_SIZE_OPTIONS.map((o) => (
                        <SelectItem key={o} value={o} className="capitalize">
                          {o}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">
                    ICP Industries{' '}
                    <span className="text-muted-foreground font-normal">
                      (comma-separated)
                    </span>
                  </Label>
                  <Input
                    value={bgetArr('icpIndustries')}
                    onChange={(e) => bsetArr('icpIndustries', e.target.value)}
                    placeholder="Hospitality, Real Estate, Fashion"
                  />
                </div>
              </div>
            </div>
            {/* Competitive */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5">
                <Zap className="h-3 w-3" /> Competitive Intelligence
              </p>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label className="text-xs">
                    Competitors{' '}
                    <span className="text-muted-foreground font-normal">
                      (comma-separated)
                    </span>
                  </Label>
                  <Input
                    value={bgetArr('competitorBrands')}
                    onChange={(e) =>
                      bsetArr('competitorBrands', e.target.value)
                    }
                    placeholder="Competitor A, Competitor B"
                  />
                </div>
                <div>
                  <Label className="text-xs">
                    Differentiators{' '}
                    <span className="text-muted-foreground font-normal">
                      (comma-separated)
                    </span>
                  </Label>
                  <Input
                    value={bgetArr('differentiators')}
                    onChange={(e) => bsetArr('differentiators', e.target.value)}
                    placeholder="End-to-end, Luxury positioning, Speed"
                  />
                </div>
              </div>
            </div>
            {/* Content & Social */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5">
                <Megaphone className="h-3 w-3" /> Content & Social
              </p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2">
                  <Label className="text-xs">
                    Content Pillars{' '}
                    <span className="text-muted-foreground font-normal">
                      (comma-separated)
                    </span>
                  </Label>
                  <Input
                    value={bgetArr('contentPillars')}
                    onChange={(e) => bsetArr('contentPillars', e.target.value)}
                    placeholder="Education, Behind the scenes, Client results"
                  />
                </div>
                <div className="col-span-2">
                  <Label className="text-xs">Tone Notes</Label>
                  <Textarea
                    value={brandForm.contentTone ?? ''}
                    onChange={(e) => bset('contentTone', e.target.value)}
                    placeholder="Nuances about how this brand communicates"
                    rows={2}
                  />
                </div>
                <div className="col-span-2">
                  <Label className="text-xs">Website URL</Label>
                  <Input
                    value={brandForm.websiteUrl ?? ''}
                    onChange={(e) => bset('websiteUrl', e.target.value)}
                    placeholder="https://brand.com"
                  />
                </div>
                <div>
                  <Label className="text-xs">Instagram</Label>
                  <Input
                    value={brandForm.socialInstagram ?? ''}
                    onChange={(e) => bset('socialInstagram', e.target.value)}
                    placeholder="@handle"
                  />
                </div>
                <div>
                  <Label className="text-xs">LinkedIn</Label>
                  <Input
                    value={brandForm.socialLinkedin ?? ''}
                    onChange={(e) => bset('socialLinkedin', e.target.value)}
                    placeholder="@handle or company slug"
                  />
                </div>
                <div>
                  <Label className="text-xs">X (Twitter)</Label>
                  <Input
                    value={brandForm.socialTwitter ?? ''}
                    onChange={(e) => bset('socialTwitter', e.target.value)}
                    placeholder="@handle"
                  />
                </div>
                <div>
                  <Label className="text-xs">TikTok</Label>
                  <Input
                    value={brandForm.socialTiktok ?? ''}
                    onChange={(e) => bset('socialTiktok', e.target.value)}
                    placeholder="@handle"
                  />
                </div>
                <div>
                  <Label className="text-xs">YouTube</Label>
                  <Input
                    value={brandForm.socialYoutube ?? ''}
                    onChange={(e) => bset('socialYoutube', e.target.value)}
                    placeholder="@channel"
                  />
                </div>
                <div>
                  <Label className="text-xs">Facebook</Label>
                  <Input
                    value={brandForm.socialFacebook ?? ''}
                    onChange={(e) => bset('socialFacebook', e.target.value)}
                    placeholder="page name or handle"
                  />
                </div>
              </div>
            </div>
          </div>
          {/* Intake link generator */}
          <div className="border-t pt-4 mt-2">
            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 flex items-center gap-1.5">
              <LinkIcon className="h-3 w-3" /> Client Intake Link
            </p>
            <p className="text-xs text-muted-foreground mb-3">
              Generate a shareable link so your client can fill in this
              questionnaire directly.
            </p>
            {intakeUrl ? (
              <div className="flex items-center gap-2">
                <Input
                  value={intakeUrl}
                  readOnly
                  className="text-xs font-mono h-8 flex-1"
                />
                <Button
                  size="sm"
                  variant="outline"
                  onClick={copyIntake}
                  className="text-xs shrink-0 gap-1"
                >
                  <CheckCircle2
                    className={`h-3 w-3 ${intakeCopied ? 'text-green-500' : ''}`}
                  />
                  {intakeCopied ? 'Copied!' : 'Copy'}
                </Button>
              </div>
            ) : (
              <Button
                size="sm"
                variant="outline"
                onClick={generateIntake}
                className="text-xs gap-1.5"
              >
                <LinkIcon className="h-3.5 w-3.5" /> Generate Intake Link
              </Button>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setBrandEditing(false)}>
              Cancel
            </Button>
            <Button onClick={saveBrand} disabled={brandSaving}>
              {brandSaving ? (
                <>
                  <Loader2 className="h-3 w-3 mr-1.5 animate-spin" />
                  Saving…
                </>
              ) : (
                'Save Brand Guide'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
