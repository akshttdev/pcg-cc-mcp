import { useState, useMemo } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import {
  Search,
  Users,
  UserCheck,
  Briefcase,
  TrendingUp,
  ArrowRight,
  Building2,
  Mail,
  Phone,
  UserCircle,
  ChevronDown,
  Globe,
} from 'lucide-react';
import { crmApi, type CrmContactRecord } from '@/lib/api';
import { useAuth } from '@/contexts/AuthContext';

// ── Helpers ────────────────────────────────────────────────────────────────

const LIFECYCLE_COLORS: Record<string, string> = {
  subscriber: 'bg-gray-100 text-gray-600',
  lead:       'bg-blue-100 text-blue-700',
  mql:        'bg-indigo-100 text-indigo-700',
  sql:        'bg-violet-100 text-violet-700',
  opportunity:'bg-amber-100 text-amber-700',
  customer:   'bg-green-100 text-green-700',
  evangelist: 'bg-pink-100 text-pink-700',
  churned:    'bg-red-100 text-red-700',
};

const STAGE_FILTERS = [
  { key: undefined,      label: 'All',         icon: Users },
  { key: 'lead',         label: 'Leads',       icon: TrendingUp },
  { key: 'customer',     label: 'Clients',     icon: Briefcase },
  { key: 'subscriber',   label: 'Subscribers', icon: UserCheck },
  { key: 'opportunity',  label: 'Opportunities', icon: Building2 },
];

function ContactCard({ contact }: { contact: CrmContactRecord }) {
  const navigate = useNavigate();
  const fullName = contact.full_name
    || [contact.first_name, contact.last_name].filter(Boolean).join(' ')
    || 'Unnamed';

  const initials = fullName
    .split(' ')
    .map((n) => n[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();

  const stageColor = LIFECYCLE_COLORS[contact.lifecycle_stage] ?? LIFECYCLE_COLORS.lead;

  return (
    <div
      className="flex items-center gap-4 p-4 border rounded-lg hover:bg-muted/40 cursor-pointer transition-colors group"
      onClick={() => navigate(`/people/${contact.id}`)}
    >
      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-sm font-semibold shrink-0">
        {contact.avatar_url ? (
          <img src={contact.avatar_url} alt={fullName} className="w-full h-full rounded-full object-cover" />
        ) : (
          initials
        )}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium truncate">{fullName}</span>
          <Badge className={`text-xs px-1.5 py-0 ${stageColor} border-0`}>
            {contact.lifecycle_stage.replace(/_/g, ' ')}
          </Badge>
        </div>
        <div className="flex items-center gap-3 mt-0.5 text-xs text-muted-foreground">
          {contact.company_name && (
            <span className="flex items-center gap-1 truncate">
              <Building2 className="h-3 w-3" />
              {contact.company_name}
            </span>
          )}
          {contact.email && (
            <span className="flex items-center gap-1 truncate">
              <Mail className="h-3 w-3" />
              {contact.email}
            </span>
          )}
          {contact.phone && (
            <span className="flex items-center gap-1">
              <Phone className="h-3 w-3" />
              {contact.phone}
            </span>
          )}
        </div>
        {contact.job_title && (
          <p className="text-xs text-muted-foreground mt-0.5">{contact.job_title}</p>
        )}
      </div>

      <div className="hidden sm:block text-xs text-muted-foreground capitalize shrink-0">
        {contact.lifecycle_stage.replace(/_/g, ' ')}
      </div>

      <ArrowRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
    </div>
  );
}

// ── Page ────────────────────────────────────────────────────────────────────

export function PeoplePage() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const [search, setSearch] = useState('');
  const [stageFilter, setStageFilter] = useState<string | undefined>(undefined);
  const [myContacts, setMyContacts] = useState(false);

  // Org scoping: default to the user's first org, allow switching to "All"
  const userOrgs: { id: string; name: string; slug: string }[] = (user as any)?.organizations ?? [];
  const orgId = user?.home_organization_id ?? userOrgs[0]?.id;
  const [selectedOrgId, setSelectedOrgId] = useState<string | undefined>(orgId);
  const selectedOrg = userOrgs.find(o => o.id === selectedOrgId);

  const { data: contacts = [], isLoading } = useQuery<CrmContactRecord[]>({
    queryKey: ['crm-contacts-all', selectedOrgId, stageFilter],
    queryFn: () =>
      crmApi.listContacts(selectedOrgId!, {
        lifecycleStage: stageFilter,
        limit: 200,
      }),
    enabled: !!selectedOrgId,
  });

  // Client-side search filtering
  const filtered = useMemo(() => {
    if (!search) return contacts;
    const q = search.toLowerCase();
    return contacts.filter((c) => {
      const name = (c.full_name || `${c.first_name ?? ''} ${c.last_name ?? ''}`).toLowerCase();
      return (
        name.includes(q) ||
        c.email?.toLowerCase().includes(q) ||
        c.company_name?.toLowerCase().includes(q)
      );
    });
  }, [contacts, search]);

  const stageCounts = contacts.reduce<Record<string, number>>((acc, c) => {
    acc[c.lifecycle_stage] = (acc[c.lifecycle_stage] ?? 0) + 1;
    return acc;
  }, {});

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">All People</h1>
          <p className="text-sm text-muted-foreground mt-1">
            CRM contacts across {selectedOrg?.name ?? 'your organization'}.
          </p>
        </div>
        {selectedOrgId && (
          <Button onClick={() => navigate(`/organizations/${selectedOrgId}/crm/contacts`)}>
            CRM Contacts
          </Button>
        )}
      </div>

      {/* Org scope switcher + Stage filter pills */}
      <div className="flex items-center gap-2 flex-wrap">
        {/* Org selector */}
        {userOrgs.length > 0 && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm border border-border hover:bg-muted/60 transition-colors font-medium">
                <Building2 className="h-3.5 w-3.5" />
                {selectedOrg?.name ?? 'All Orgs'}
                <ChevronDown className="h-3 w-3 opacity-60" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onClick={() => setSelectedOrgId(undefined)}>
                <Globe className="h-4 w-4 mr-2" />
                All Organizations
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              {userOrgs.map(org => (
                <DropdownMenuItem key={org.id} onClick={() => setSelectedOrgId(org.id)}>
                  <Building2 className="h-4 w-4 mr-2" />
                  {org.name}
                  {selectedOrgId === org.id && <span className="ml-auto text-primary">✓</span>}
                </DropdownMenuItem>
              ))}
              {selectedOrgId && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem asChild>
                    <Link to={`/organizations/${selectedOrgId}/crm`} className="flex items-center gap-2">
                      <UserCheck className="h-4 w-4" />
                      Open {selectedOrg?.name} CRM
                    </Link>
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        )}

        {/* Stage filter pills */}
        {STAGE_FILTERS.map((f) => {
          const Icon = f.icon;
          const count = f.key ? (stageCounts[f.key] ?? 0) : contacts.length;
          const active = stageFilter === f.key;
          return (
            <button
              key={f.key ?? 'all'}
              onClick={() => setStageFilter(f.key)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm border transition-colors ${
                active
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'border-border hover:bg-muted/60'
              }`}
            >
              <Icon className="h-3.5 w-3.5" />
              {f.label}
              <span className={`text-xs ${active ? 'opacity-80' : 'text-muted-foreground'}`}>
                {count}
              </span>
            </button>
          );
        })}

        <button
          onClick={() => setMyContacts(!myContacts)}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm border transition-colors ${
            myContacts
              ? 'bg-violet-600 text-white border-violet-600'
              : 'border-border hover:bg-muted/60'
          }`}
        >
          <UserCircle className="h-3.5 w-3.5" />
          My Contacts
        </button>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="Search by name, email, or company..."
          className="pl-9"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      {/* List */}
      {isLoading ? (
        <div className="space-y-3">
          {[...Array(6)].map((_, i) => (
            <Skeleton key={i} className="h-16 w-full rounded-lg" />
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <Users className="h-12 w-12 mb-3 opacity-30" />
          <p className="text-lg font-semibold mb-1 text-foreground">
            {search || stageFilter ? 'No contacts match your filters' : 'No contacts yet'}
          </p>
          <p className="text-sm max-w-sm text-center">
            {search || stageFilter
              ? 'Try adjusting your search or filter criteria.'
              : 'Contacts will appear here as they are added via CRM or workflow imports.'}
          </p>
          {selectedOrg && (
            <p className="text-xs mt-1">
              Showing contacts for <strong>{selectedOrg.name}</strong>
            </p>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((c) => (
            <ContactCard key={c.id} contact={c} />
          ))}
        </div>
      )}
    </div>
  );
}
