import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { organizationsApi, type ClientData } from '@/lib/api';
import { Briefcase, Brain, ExternalLink, Building2, Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';

export function OrgClientsPage() {
  const { orgId } = useParams<{ orgId: string }>();
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<'all' | 'clients' | 'prospects'>('all');

  const { data: clients = [], isLoading } = useQuery({
    queryKey: ['org-clients', orgId],
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const filtered = (clients as ClientData[]).filter((c) => {
    const matchesSearch = !search || c.name.toLowerCase().includes(search.toLowerCase());
    const matchesFilter =
      filter === 'all' ? true :
      filter === 'clients' ? !!c.client_since :
      filter === 'prospects' ? (!c.client_since && !!c.prospect_at) : true;
    return matchesSearch && matchesFilter && c.is_active;
  });

  const clientCount = (clients as ClientData[]).filter((c) => !!c.client_since).length;
  const prospectCount = (clients as ClientData[]).filter((c) => !c.client_since && !!c.prospect_at).length;

  return (
    <div className="max-w-5xl mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Briefcase className="h-6 w-6 text-indigo-400" />
            Clients
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            {clientCount} active client{clientCount !== 1 ? 's' : ''} · {prospectCount} prospect{prospectCount !== 1 ? 's' : ''}
          </p>
        </div>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-3 flex-wrap">
        <div className="relative flex-1 min-w-48 max-w-xs">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            className="pl-8 h-8 text-sm"
            placeholder="Search clients…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <div className="flex gap-1">
          {(['all', 'clients', 'prospects'] as const).map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-3 py-1 text-xs rounded-md font-medium transition-colors ${
                filter === f
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-accent/60'
              }`}
            >
              {f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {isLoading && (
        <div className="flex items-center justify-center py-16">
          <Loader size={24} message="Loading clients…" />
        </div>
      )}

      {!isLoading && filtered.length === 0 && (
        <div className="text-center py-16 text-muted-foreground">
          <Briefcase className="h-10 w-10 mx-auto mb-3 opacity-20" />
          <p className="text-sm font-medium">No clients found</p>
          {filter !== 'all' && (
            <p className="text-xs mt-1 opacity-60">Try switching to "All"</p>
          )}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
        {filtered.map((client) => {
          const isClient = !!client.client_since;
          const isProspect = !isClient && !!client.prospect_at;
          return (
            <Link
              key={client.id}
              to={`/organizations/${orgId}/clients/${client.id}`}
              className="block rounded-lg border border-border/60 hover:border-indigo-500/40 hover:bg-accent/40 transition-colors p-4 space-y-3 group"
            >
              <div className="flex items-start gap-3">
                <div className="w-10 h-10 rounded-lg bg-muted flex items-center justify-center shrink-0 overflow-hidden">
                  {(client as ClientData & { company_logo_url?: string }).company_logo_url
                    ? <img src={(client as ClientData & { company_logo_url?: string }).company_logo_url} className="w-full h-full object-contain p-1" alt="" />
                    : <Building2 className="h-5 w-5 text-muted-foreground" />
                  }
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-semibold truncate group-hover:text-indigo-400 transition-colors">
                    {client.name}
                  </p>
                  {(client as ClientData & { company_industry?: string }).company_industry && (
                    <p className="text-xs text-muted-foreground truncate">
                      {(client as ClientData & { company_industry?: string }).company_industry}
                    </p>
                  )}
                </div>
                <ExternalLink className="h-3.5 w-3.5 text-muted-foreground shrink-0 opacity-0 group-hover:opacity-100 transition-opacity" />
              </div>

              <div className="flex items-center gap-2 flex-wrap">
                <Badge
                  variant={isClient ? 'default' : 'secondary'}
                  className={`text-xs ${isClient ? '' : isProspect ? 'border-amber-500/40 text-amber-400 bg-amber-500/10' : ''}`}
                >
                  {isClient ? 'Client' : isProspect ? 'Prospect' : 'Active'}
                </Badge>
                {(client as ClientData & { company_intel_status?: string }).company_intel_status === 'done' && (
                  <span className="flex items-center gap-1 text-xs text-indigo-400">
                    <Brain className="h-3 w-3" /> Intel
                  </span>
                )}
              </div>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
