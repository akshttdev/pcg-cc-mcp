import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
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
} from 'lucide-react';
import { personsApi, type PersonRecord } from '@/lib/api';

// ── Helpers ────────────────────────────────────────────────────────────────

const PERSON_TYPE_INFO: Record<string, { label: string; color: string }> = {
  team:       { label: 'Team',       color: 'bg-violet-100 text-violet-700' },
  client:     { label: 'Client',     color: 'bg-green-100 text-green-700' },
  contractor: { label: 'Contractor', color: 'bg-amber-100 text-amber-700' },
  lead:       { label: 'Lead',       color: 'bg-blue-100 text-blue-700' },
  partner:    { label: 'Partner',    color: 'bg-pink-100 text-pink-700' },
  contact:    { label: 'Contact',    color: 'bg-gray-100 text-gray-600' },
};

const FINANCIAL_ROLE_INFO: Record<string, { label: string; color: string }> = {
  taker:   { label: 'Taker',   color: 'text-red-600' },
  giver:   { label: 'Giver',   color: 'text-green-600' },
  both:    { label: 'Taker + Giver', color: 'text-purple-600' },
  neutral: { label: '',        color: '' },
};

const FILTERS = [
  { key: undefined, label: 'All', icon: Users },
  { key: 'team',       label: 'Team',       icon: UserCheck },
  { key: 'client',     label: 'Clients',    icon: Briefcase },
  { key: 'lead',       label: 'Leads',      icon: TrendingUp },
  { key: 'contractor', label: 'Contractors', icon: Building2 },
];

function PersonCard({ person }: { person: PersonRecord }) {
  const navigate = useNavigate();
  const typeInfo = PERSON_TYPE_INFO[person.person_type] ?? PERSON_TYPE_INFO.contact;
  const roleInfo = FINANCIAL_ROLE_INFO[person.financial_role] ?? FINANCIAL_ROLE_INFO.neutral;

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
      {/* Avatar */}
      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-sm font-semibold shrink-0">
        {person.avatar_url ? (
          <img src={person.avatar_url} alt={person.full_name} className="w-full h-full rounded-full object-cover" />
        ) : (
          initials
        )}
      </div>

      {/* Main info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium truncate">{person.full_name}</span>
          <Badge className={`text-xs px-1.5 py-0 ${typeInfo.color} border-0`}>
            {typeInfo.label}
          </Badge>
          {roleInfo.label && (
            <span className={`text-xs font-medium ${roleInfo.color}`}>{roleInfo.label}</span>
          )}
        </div>
        <div className="flex items-center gap-3 mt-0.5 text-xs text-muted-foreground">
          {person.company_name && (
            <span className="flex items-center gap-1 truncate">
              <Building2 className="h-3 w-3" />
              {person.company_name}
            </span>
          )}
          {person.email && (
            <span className="flex items-center gap-1 truncate">
              <Mail className="h-3 w-3" />
              {person.email}
            </span>
          )}
          {person.phone && (
            <span className="flex items-center gap-1">
              <Phone className="h-3 w-3" />
              {person.phone}
            </span>
          )}
        </div>
        {person.intelligence_summary && (
          <p className="text-xs text-muted-foreground mt-1 line-clamp-1 italic">
            {person.intelligence_summary}
          </p>
        )}
      </div>

      {/* Lifecycle */}
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
  const [search, setSearch] = useState('');
  const [activeFilter, setActiveFilter] = useState<string | undefined>(undefined);

  const { data: persons = [], isLoading } = useQuery<PersonRecord[]>({
    queryKey: ['persons', activeFilter, search],
    queryFn: () =>
      personsApi.list({
        person_type: activeFilter,
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
            Universal contact intelligence — every team member, client, lead, and contractor in one place.
          </p>
        </div>
        <Button onClick={() => navigate('/people/new')}>
          + New Person
        </Button>
      </div>

      {/* Filter pills */}
      <div className="flex items-center gap-2 flex-wrap">
        {FILTERS.map((f) => {
          const Icon = f.icon;
          const count = f.key ? (counts[f.key] ?? 0) : persons.length;
          const active = activeFilter === f.key;
          return (
            <button
              key={f.key ?? 'all'}
              onClick={() => setActiveFilter(f.key)}
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
