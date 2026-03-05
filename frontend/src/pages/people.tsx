import { useState } from 'react';
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
import { personsApi, type PersonRecord } from '@/lib/api';
import { useAuth } from '@/contexts/AuthContext';

// ── Helpers ────────────────────────────────────────────────────────────────

const PERSON_TYPE_INFO: Record<string, { label: string; color: string }> = {
  team:       { label: 'Team',       color: 'bg-violet-100 text-violet-700' },
  client:     { label: 'Client',     color: 'bg-green-100 text-green-700' },
  contractor: { label: 'Contractor', color: 'bg-amber-100 text-amber-700' },
  lead:       { label: 'Lead',       color: 'bg-blue-100 text-blue-700' },
  partner:    { label: 'Partner',    color: 'bg-pink-100 text-pink-700' },
  contact:    { label: 'Contact',    color: 'bg-gray-100 text-gray-600' },
};

const TYPE_FILTERS = [
  { key: undefined,    label: 'All',         icon: Users },
  { key: 'team',       label: 'Team',        icon: UserCheck },
  { key: 'client',     label: 'Clients',     icon: Briefcase },
  { key: 'lead',       label: 'Leads',       icon: TrendingUp },
  { key: 'contractor', label: 'Contractors', icon: Building2 },
];

function PersonCard({ person }: { person: PersonRecord }) {
  const navigate = useNavigate();
  const typeInfo = PERSON_TYPE_INFO[person.person_type] ?? PERSON_TYPE_INFO.contact;

  const initials = person.full_name
    .split(' ')
    .map((n) => n[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();

  return (
    <div
      className="flex items-center gap-4 p-4 border rounded-lg hover:bg-muted/40 cursor-pointer transition-colors group"
      onClick={() => navigate(`/people/${person.id}`)}
    >
      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-sm font-semibold shrink-0">
        {person.avatar_url ? (
          <img src={person.avatar_url} alt={person.full_name} className="w-full h-full rounded-full object-cover" />
        ) : initials}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium truncate">{person.full_name}</span>
          <Badge className={`text-xs px-1.5 py-0 ${typeInfo.color} border-0`}>
            {typeInfo.label}
          </Badge>
          {person.intelligence_confidence > 0.6 && (
            <span className="text-xs text-green-600 font-medium">
              {Math.round(person.intelligence_confidence * 100)}%
            </span>
          )}
        </div>
        <div className="flex items-center gap-3 mt-0.5 text-xs text-muted-foreground">
          {person.company_name && (
            <span className="flex items-center gap-1 truncate">
              <Building2 className="h-3 w-3" />
              {person.company_name}
            </span>
          )}
          {(() => {
            const emailList = (() => { try { return JSON.parse(person.emails || '[]'); } catch { return []; } })();
            const primaryEmail = emailList[0]?.value ?? person.email;
            return primaryEmail ? (
              <span className="flex items-center gap-1 truncate">
                <Mail className="h-3 w-3" />
                {primaryEmail}
              </span>
            ) : null;
          })()}
          {(() => {
            const phoneList = (() => { try { return JSON.parse(person.phones || '[]'); } catch { return []; } })();
            const primaryPhone = phoneList[0]?.value ?? person.phone;
            return primaryPhone ? (
              <span className="flex items-center gap-1">
                <Phone className="h-3 w-3" />
                {primaryPhone}
              </span>
            ) : null;
          })()}
        </div>
        {person.intelligence_summary && (
          <p className="text-xs text-muted-foreground mt-1 line-clamp-1 italic">
            {person.intelligence_summary}
          </p>
        )}
      </div>

      <div className="hidden sm:block text-xs text-muted-foreground capitalize shrink-0">
        {person.lifecycle_stage.replace(/_/g, ' ')}
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
  const [activeType, setActiveType] = useState<string | undefined>(undefined);
  const [myContacts, setMyContacts] = useState(false);

  // Org scoping: default to the user's first org, allow switching to "All"
  const userOrgs: { id: string; name: string; slug: string }[] = (user as any)?.organizations ?? [];
  const [selectedOrgId, setSelectedOrgId] = useState<string | undefined>(
    userOrgs.length > 0 ? userOrgs[0].id : undefined
  );
  const selectedOrg = userOrgs.find(o => o.id === selectedOrgId);

  const { data: persons = [], isLoading } = useQuery<PersonRecord[]>({
    queryKey: ['persons', activeType, search, myContacts, user?.id, selectedOrgId],
    queryFn: () =>
      personsApi.list({
        person_type: activeType,
        assigned_to: myContacts && user?.id ? user.id : undefined,
        organization_id: selectedOrgId,
        q: search || undefined,
        limit: 200,
      }),
  });

  const counts = persons.reduce<Record<string, number>>((acc, p) => {
    acc[p.person_type] = (acc[p.person_type] ?? 0) + 1;
    return acc;
  }, {});

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">People</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Contacts, leads, and clients for {selectedOrg?.name ?? 'all organisations'}.
          </p>
        </div>
        <Button onClick={() => navigate('/people/new')}>
          + New Person
        </Button>
      </div>

      {/* Org scope switcher + My Contacts */}
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
                All Organisations
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
                    <Link to={`/organizations/${selectedOrgId}?tab=crm`} className="flex items-center gap-2">
                      <UserCheck className="h-4 w-4" />
                      Open {selectedOrg?.name} CRM
                    </Link>
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        )}

        {/* Type filter pills */}
        {TYPE_FILTERS.map((f) => {
          const Icon = f.icon;
          const count = f.key ? (counts[f.key] ?? 0) : persons.length;
          const active = activeType === f.key;
          return (
            <button
              key={f.key ?? 'all'}
              onClick={() => setActiveType(f.key)}
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
      ) : persons.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <Users className="h-12 w-12 mb-3 opacity-30" />
          <p className="text-sm">No people found</p>
          {selectedOrg && (
            <p className="text-xs mt-1">
              Showing contacts for <strong>{selectedOrg.name}</strong>
            </p>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          {persons.map((p) => (
            <PersonCard key={p.id} person={p} />
          ))}
        </div>
      )}
    </div>
  );
}
