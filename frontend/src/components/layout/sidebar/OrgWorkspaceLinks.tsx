import { useMemo, useState } from 'react';
import { Link, type useLocation } from 'react-router-dom';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
  ChevronDown,
  ChevronRight,
  Users,
  Building2,
  TrendingUp,
  Package,
  LayoutGrid,
  Brain,
  Share2,
  Calendar,
  Database,
  FileText,
  BarChart2,
  Inbox,
  Radio,
  Network,
  GitBranch,
  BarChart3,
} from 'lucide-react';
import { cn } from '@/lib/utils';

// ============================================================================
// OrgCrmSection — collapsible CRM with Overview/Contacts/Pipeline/Deliverables/Social
// ============================================================================

export function OrgCrmSection({
  orgId,
  location,
}: {
  orgId: string;
  location: ReturnType<typeof useLocation>;
}) {
  const orgBase = `/organizations/${orgId}`;
  const isCrmActive =
    location.pathname === orgBase ||
    location.pathname.startsWith(`${orgBase}/crm`);
  const [open, setOpen] = useState(isCrmActive);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1.5 w-full px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            isCrmActive && 'text-accent-foreground font-medium'
          )}
        >
          <Users className="h-3 w-3 shrink-0 text-primary" />
          <span className="flex-1 text-left">CRM</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {[
            { label: 'Overview',     to: `${orgBase}/crm`,              icon: LayoutGrid, color: 'text-muted-foreground',        match: location.pathname === `${orgBase}/crm` },
            { label: 'Contacts',     to: `${orgBase}/crm/contacts`,     icon: Users,      color: 'text-primary',                 match: location.pathname === `${orgBase}/crm/contacts` },
            { label: 'Companies',    to: `${orgBase}/crm/companies`,    icon: Building2,  color: 'text-purple-500',              match: location.pathname === `${orgBase}/crm/companies` },
            { label: 'Pipeline',     to: `${orgBase}/crm/pipeline`,     icon: TrendingUp, color: 'text-[hsl(var(--warning))]',   match: location.pathname === `${orgBase}/crm/pipeline` },
            { label: 'Deliverables', to: `${orgBase}/crm/deliverables`, icon: Package,    color: 'text-[hsl(var(--success))]',   match: location.pathname === `${orgBase}/crm/deliverables` },
          ].map(({ label, to, icon: Icon, color, match }) => (
            <Link
              key={label}
              to={to}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                match && 'bg-primary/10 text-foreground font-medium'
              )}
            >
              <Icon className={cn('h-3 w-3 shrink-0', color)} />
              <span>{label}</span>
            </Link>
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// OrgSocialSection — collapsible Social sub-navigation for an org
// ============================================================================

export function OrgSocialSection({
  orgId,
  location,
}: {
  orgId: string;
  location: ReturnType<typeof useLocation>;
}) {
  const orgBase = `/organizations/${orgId}`;
  const sp = new URLSearchParams(location.search);
  const socialBase = `${orgBase}/social`;
  const isOnSocial = location.pathname === socialBase || (location.pathname === orgBase && sp.get('tab') === 'social');
  const [open, setOpen] = useState(isOnSocial);

  const socialViews = [
    { label: 'Overview',  sv: '',          icon: LayoutGrid,  color: 'text-muted-foreground' },
    { label: 'Accounts',  sv: 'accounts',  icon: Share2,      color: 'text-pink-500' },
    { label: 'Content',   sv: 'content',   icon: FileText,    color: 'text-purple-500' },
    { label: 'Inbox',     sv: 'inbox',     icon: Inbox,       color: 'text-amber-500' },
    { label: 'Analytics', sv: 'analytics', icon: BarChart2,   color: 'text-blue-500' },
  ];

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1.5 w-full px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            isOnSocial && 'text-accent-foreground font-medium'
          )}
        >
          <Share2 className="h-3 w-3 shrink-0 text-pink-500" />
          <span className="flex-1 text-left">Social</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {socialViews.map(({ label, sv, icon: Icon, color }) => {
            const to = sv
              ? `${socialBase}?sv=${sv}`
              : socialBase;
            const isActive = isOnSocial && (sp.get('sv') || '') === sv;
            return (
              <Link
                key={label}
                to={to}
                className={cn(
                  'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                  isActive && 'bg-primary/10 text-foreground font-medium'
                )}
              >
                <Icon className={cn('h-3 w-3 shrink-0', color)} />
                <span>{label}</span>
              </Link>
            );
          })}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// OrgIntelligenceSection — collapsible Intelligence sub-navigation for an org
// ============================================================================

export function OrgIntelligenceSection({
  orgId,
  location,
}: {
  orgId: string;
  location: ReturnType<typeof useLocation>;
}) {
  const orgBase = `/organizations/${orgId}`;
  const isOnOrgIntel = location.pathname.startsWith(`${orgBase}/intelligence`);
  const [open, setOpen] = useState(isOnOrgIntel);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1.5 w-full px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            isOnOrgIntel && 'text-accent-foreground font-medium'
          )}
        >
          <Brain className="h-3 w-3 shrink-0 text-[hsl(var(--success))]" />
          <span className="flex-1 text-left">Intelligence</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {[
            { label: 'Overview',      icon: Brain,         color: 'text-[hsl(var(--success))]', to: `${orgBase}/intelligence`,              match: location.pathname === `${orgBase}/intelligence` },
            { label: 'Data Sources',  icon: Database,       color: 'text-primary',                to: `${orgBase}/intelligence/data-sources`, match: location.pathname === `${orgBase}/intelligence/data-sources` },
            { label: 'Artifacts',     icon: FileText,      color: 'text-[hsl(var(--brand))]',    to: `${orgBase}/intelligence/artifacts`,    match: location.pathname === `${orgBase}/intelligence/artifacts` },
            { label: 'Workflows',     icon: GitBranch,     color: 'text-purple-500',             to: `${orgBase}/intelligence/workflows`,    match: location.pathname === `${orgBase}/intelligence/workflows` },
            { label: 'Pulse',         icon: Radio,         color: 'text-[hsl(var(--warning))]',  to: `${orgBase}/intelligence/pulse`,        match: location.pathname === `${orgBase}/intelligence/pulse` },
            { label: 'Topology',      icon: Network,       color: 'text-[hsl(var(--info))]',     to: `${orgBase}/intelligence/topology`,     match: location.pathname === `${orgBase}/intelligence/topology` },
          ].map(({ label, to, icon: Icon, color, match }) => (
            <Link
              key={label}
              to={to}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                match && 'bg-primary/10 text-foreground font-medium'
              )}
            >
              <Icon className={cn('h-3 w-3 shrink-0', color)} />
              <span>{label}</span>
            </Link>
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// CrmSidebarLinks — project-level CRM links (collapsible)
// ============================================================================

export function CrmSidebarLinks({
  projectId,
  location,
  indent = 'pl-5',
}: {
  projectId: string;
  location: ReturnType<typeof useLocation>;
  indent?: string;
}) {
  const crmLinks = useMemo(
    () => [
      { label: 'Overview',        to: `/projects/${projectId}/crm/overview`,     icon: BarChart3  },
      { label: 'Sales Pipeline',  to: `/projects/${projectId}/crm/sales`,        icon: TrendingUp },
      { label: 'Client Delivery', to: `/projects/${projectId}/crm/delivery`,     icon: Package    },
      { label: 'Contacts',        to: `/projects/${projectId}/crm`,              icon: Users      },
      { label: 'Conferences',     to: `/projects/${projectId}/crm/conferences`,  icon: Calendar   },
    ],
    [projectId]
  );

  const hasCrmActive = crmLinks.some((link) => location.pathname === link.to);
  const [open, setOpen] = useState(hasCrmActive);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-2 w-full pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            indent,
            hasCrmActive && 'text-foreground font-medium'
          )}
        >
          <Users className="h-3 w-3 text-primary shrink-0" />
          <span className="flex-1 text-left">CRM</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {crmLinks.map((link) => {
            const isActive = location.pathname === link.to;
            const Icon = link.icon;
            return (
              <Link
                key={link.to}
                to={link.to}
                className={cn(
                  'flex items-center gap-2 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                  isActive && 'bg-primary/10 text-foreground font-medium'
                )}
              >
                <Icon className="h-3 w-3 text-muted-foreground" />
                <span>{link.label}</span>
              </Link>
            );
          })}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
