import { useState, useCallback, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import {
  Building2,
  MapPin,
  Globe,
  Contact2,
  Search,
  Plus,
  Users,
  Briefcase,
  CheckCircle2,
  Loader2,
  ExternalLink,
  Brain,
  User,
} from 'lucide-react';
import {
  crmApi,
  companiesApi,
  // organizationsApi,
  type CreateCrmContactRequest,
  type CrmContactRecord,
  type CompanyRecord,
} from '@/lib/api';
import { useCrmContacts, crmContactsQueryKey } from '@/hooks/queries';
import { entityKeys, organizationKeys } from '@/lib/query-keys';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';
import type { CrmDealWithContact } from '@/types/crm';
// import { ContactCard } from '../components/ContactCard';
import { ContactDetailModal } from '../components/ContactDetailModal';

type PeopleView = 'people' | 'companies' | 'pipeline';

export function ContactsTab({ orgId }: { orgId: string }) {
  const [view, setView] = useState<PeopleView>('people');
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');
  const [showAddContact, setShowAddContact] = useState(false);
  const [showAddCompany, setShowAddCompany] = useState(false);
  const [selectedContactId, setSelectedContactId] = useState<string | null>(null);
  const [contactForm, setContactForm] = useState({
    first_name: '',
    last_name: '',
    email: '',
    phone: '',
    company_name: '',
    job_title: '',
    lifecycle_stage: 'lead',
  });
  const [companyForm, setCompanyForm] = useState({ name: '', website: '', industry: '' });
  const queryClient = useQueryClient();

  // ── Data fetching ─────────────────────────────────────────────────────────

  // Org-scoped CRM contacts — the source of truth for this org's people
  const { data: contacts = [], isLoading: contactsLoading } = useCrmContacts(orgId);

  // All companies (knowledge graph) — used to look up company profiles by name
  const { data: allCompanies = [], isLoading: companiesLoading } = useQuery<CompanyRecord[]>({
    queryKey: entityKeys.companiesAll(),
    queryFn: () => companiesApi.list({ limit: 500 }),
    staleTime: 60_000,
  });

  // Pipeline deals — enriched with person_id, contact_company, company_id for each contact
  const { data: pipelineDeals = [], isLoading: pipelineLoading } = useQuery<CrmDealWithContact[]>({
    queryKey: organizationKeys.dealsEnriched(orgId),
    queryFn: async () => {
      const res = await fetch(`/api/crm/deals/enriched?organization_id=${orgId}`, {
        headers: { Authorization: `Bearer ${localStorage.getItem('session_id') ?? ''}` },
        credentials: 'include',
      });
      const json = await res.json();
      return (json.data ?? []) as CrmDealWithContact[];
    },
    staleTime: 30_000,
  });

  // Build contact_id → person_id + company_id map from deals
  const dealPersonMap = useMemo(() => {
    const m = new Map<string, { person_id?: string; company_id?: string; intelligence_status?: string; company_intelligence_status?: string }>();
    for (const d of pipelineDeals) {
      if (d.crm_contact_id) {
        m.set(d.crm_contact_id, {
          person_id: d.person_id,
          company_id: d.company_id,
          intelligence_status: d.intelligence_status,
          company_intelligence_status: d.company_intelligence_status,
        });
      }
    }
    return m;
  }, [pipelineDeals]);

  // Build company_id → person_id map from deals (for company card Profile/Intel buttons)
  // const companyPersonMap = useMemo(() => {
  //   const m = new Map<string, string>();
  //   for (const d of pipelineDeals) {
  //     if (d.company_id && d.person_id && !m.has(d.company_id)) {
  //       m.set(d.company_id, d.person_id);
  //     }
  //   }
  //   return m;
  // }, [pipelineDeals]);

  // Org clients — used to link company cards to client pages
  // const { data: orgClients = [] } = useQuery<{ id: string; name: string }[]>({
  //   queryKey: ['org-clients', orgId],
  //   queryFn: () => organizationsApi.getClients(orgId),
  //   staleTime: 60_000,
  // });

  // Company name → company record lookup
  const companyByName = useMemo(() => {
    const m = new Map<string, CompanyRecord>();
    for (const c of allCompanies) {
      m.set(c.name.toLowerCase(), c);
    }
    return m;
  }, [allCompanies]);

  // Unique companies from org's contacts
  const orgCompanies = useMemo(() => {
    const seen = new Set<string>();
    const result: CompanyRecord[] = [];
    for (const contact of contacts) {
      const name = contact.company_name?.trim();
      if (!name || seen.has(name.toLowerCase())) continue;
      seen.add(name.toLowerCase());
      const company = companyByName.get(name.toLowerCase());
      if (company) result.push(company);
    }
    return result;
  }, [contacts, companyByName]);

  // Pipeline contacts: deals with both person and company
  const pipelineContacts = useMemo(() => {
    const seen = new Set<string>();
    return pipelineDeals
      .filter((d) => d.person_id && d.contact_company)
      .filter((d) => {
        if (seen.has(d.person_id!)) return false;
        seen.add(d.person_id!);
        return true;
      });
  }, [pipelineDeals]);

  // ── Mutations ─────────────────────────────────────────────────────────────

  const createContactMutation = useMutation({
    mutationFn: (data: CreateCrmContactRequest) => crmApi.createContact(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: crmContactsQueryKey(orgId) });
      setShowAddContact(false);
      setContactForm({ first_name: '', last_name: '', email: '', phone: '', company_name: '', job_title: '', lifecycle_stage: 'lead' });
      toast.success('Contact created');
    },
    onError: () => toast.error('Failed to create contact'),
  });

  const createCompanyMutation = useMutation({
    mutationFn: (data: { name: string; website?: string; industry?: string; created_by_org_id?: string }) =>
      companiesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['companies-all'] });
      setShowAddCompany(false);
      setCompanyForm({ name: '', website: '', industry: '' });
      toast.success('Company created');
    },
    onError: () => toast.error('Failed to create company'),
  });

  const handleCreateContact = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    createContactMutation.mutate({
      organization_id: orgId,
      ...contactForm,
      first_name: contactForm.first_name || undefined,
      last_name: contactForm.last_name || undefined,
      email: contactForm.email || undefined,
      phone: contactForm.phone || undefined,
      company_name: contactForm.company_name || undefined,
      job_title: contactForm.job_title || undefined,
    });
  }, [contactForm, orgId, createContactMutation]);

  const handleCreateCompany = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (!companyForm.name.trim()) return;
    createCompanyMutation.mutate({
      name: companyForm.name,
      website: companyForm.website || undefined,
      industry: companyForm.industry || undefined,
      created_by_org_id: orgId,
    });
  }, [companyForm, orgId, createCompanyMutation]);

  // ── Filtering ─────────────────────────────────────────────────────────────

  const selectedContact = useMemo(
    () => contacts.find((c: CrmContactRecord) => c.id === selectedContactId),
    [contacts, selectedContactId]
  );

  const filteredContacts = useMemo(() => {
    let result = contacts;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (c) =>
          (c.full_name && c.full_name.toLowerCase().includes(q)) ||
          (c.email && c.email.toLowerCase().includes(q)) ||
          (c.company_name && c.company_name.toLowerCase().includes(q))
      );
    }
    if (stageFilter !== 'all') {
      result = result.filter((c) => c.lifecycle_stage === stageFilter);
    }
    return result;
  }, [contacts, searchQuery, stageFilter]);

  const filteredCompanies = useMemo(() => {
    if (!searchQuery) return orgCompanies;
    const q = searchQuery.toLowerCase();
    return orgCompanies.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        (c.industry && c.industry.toLowerCase().includes(q)) ||
        (c.headquarters && c.headquarters.toLowerCase().includes(q))
    );
  }, [orgCompanies, searchQuery]);

  const filteredPipeline = useMemo(() => {
    if (!searchQuery) return pipelineContacts;
    const q = searchQuery.toLowerCase();
    return pipelineContacts.filter(
      (d) =>
        (d.contact_name && d.contact_name.toLowerCase().includes(q)) ||
        (d.contact_company && d.contact_company.toLowerCase().includes(q)) ||
        (d.contact_email && d.contact_email.toLowerCase().includes(q))
    );
  }, [pipelineContacts, searchQuery]);

  const isLoading =
    view === 'people' ? contactsLoading
    : view === 'companies' ? companiesLoading
    : pipelineLoading;

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="space-y-4">
      {/* Three-view toggle */}
      <div className="flex items-center gap-1 p-1 bg-muted rounded-lg w-fit">
        <button
          onClick={() => setView('people')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            view === 'people' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Users className="h-3.5 w-3.5" />
          People
          <span className="text-xs text-muted-foreground">({contacts.length})</span>
        </button>
        <button
          onClick={() => setView('companies')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            view === 'companies' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Building2 className="h-3.5 w-3.5" />
          Companies
          <span className="text-xs text-muted-foreground">({orgCompanies.length})</span>
        </button>
        <button
          onClick={() => setView('pipeline')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            view === 'pipeline' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Briefcase className="h-3.5 w-3.5" />
          Pipeline
          <span className="text-xs text-muted-foreground">({pipelineContacts.length})</span>
        </button>
      </div>

      {/* Search + action bar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={
              view === 'people' ? 'Search contacts…'
              : view === 'companies' ? 'Search companies…'
              : 'Search pipeline contacts…'
            }
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        {view === 'people' && (
          <select
            value={stageFilter}
            onChange={(e) => setStageFilter(e.target.value)}
            className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          >
            <option value="all">All Stages</option>
            {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
              <option key={key} value={key}>{info.label}</option>
            ))}
          </select>
        )}
        {isLoading && <span className="text-xs text-muted-foreground">Loading…</span>}
        {view === 'people' && (
          <Button size="sm" onClick={() => setShowAddContact(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Contact
          </Button>
        )}
        {view === 'companies' && (
          <Button size="sm" onClick={() => setShowAddCompany(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Company
          </Button>
        )}
      </div>

      {/* ── People (org contacts with person profile links) ─────────────── */}
      {view === 'people' && (
        filteredContacts.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{contactsLoading ? 'Loading contacts…' : 'No contacts found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredContacts.map((contact) => {
              const personInfo = dealPersonMap.get(contact.id);
              const personId = personInfo?.person_id;
              const contactCompanyId = personInfo?.company_id;
              const intelStatus = personInfo?.intelligence_status;
              return (
                <div key={contact.id} className="rounded-lg border bg-card p-4 space-y-2">
                  <div
                    className="flex items-start gap-3 cursor-pointer"
                    onClick={() => setSelectedContactId(contact.id)}
                  >
                    {contact.avatar_url ? (
                      <img src={contact.avatar_url} alt="" className="w-9 h-9 rounded-full object-cover shrink-0" />
                    ) : (
                      <div className="w-9 h-9 rounded-full bg-muted flex items-center justify-center shrink-0 text-sm font-semibold text-muted-foreground">
                        {contact.full_name?.charAt(0) ?? '?'}
                      </div>
                    )}
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-semibold truncate">{contact.full_name ?? 'Unknown'}</p>
                      {contact.job_title && (
                        <p className="text-xs text-muted-foreground truncate">{contact.job_title}</p>
                      )}
                      {contact.company_name && (
                        <p className="text-xs text-muted-foreground truncate flex items-center gap-1 mt-0.5">
                          <Building2 className="h-3 w-3 shrink-0" />
                          {contactCompanyId ? (
                            <Link to={`/companies/${contactCompanyId}`} onClick={(e) => e.stopPropagation()} className="hover:text-primary transition-colors truncate">
                              {contact.company_name}
                            </Link>
                          ) : (
                            contact.company_name
                          )}
                        </p>
                      )}
                    </div>
                    {intelStatus === 'done' && <CheckCircle2 className="h-3.5 w-3.5 text-green-500 shrink-0 mt-0.5" />}
                    {(intelStatus === 'running' || intelStatus === 'queued') && (
                      <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin shrink-0 mt-0.5" />
                    )}
                  </div>
                  {personId && (
                    <div className="flex items-center gap-1.5 pt-0.5">
                      <Link
                        to={`/people/${personId}`}
                        onClick={(e) => e.stopPropagation()}
                        className="flex-1 flex items-center justify-center gap-1 text-xs py-1 px-2 rounded border border-border hover:bg-accent transition-colors"
                      >
                        <User className="h-3 w-3" /> Profile
                      </Link>
                      <Link
                        to={`/people/${personId}/intel`}
                        onClick={(e) => e.stopPropagation()}
                        className="flex-1 flex items-center justify-center gap-1 text-xs py-1 px-2 rounded border border-indigo-700/60 text-indigo-400 hover:bg-indigo-950/40 transition-colors"
                      >
                        <Brain className="h-3 w-3" /> Intel
                      </Link>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )
      )}

      {/* ── Companies (from org's contact company names) ───────────────── */}
      {view === 'companies' && (
        filteredCompanies.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Building2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>
              {companiesLoading
                ? 'Loading companies…'
                : contacts.length === 0
                  ? 'No contacts yet — add contacts with company names to see company profiles here'
                  : 'No company profiles found for your contacts yet'}
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredCompanies.map((company) => {
              // const personId = companyPersonMap.get(company.id);
              return (
                <div key={company.id} className="rounded-lg border bg-card p-4 space-y-2 hover:border-primary/40 hover:shadow-sm transition-all">
                  <Link to={`/companies/${company.id}`} className="block group">
                    <div className="flex items-start gap-3">
                      {company.logo_url ? (
                        <img src={company.logo_url} alt="" className="w-9 h-9 rounded object-cover shrink-0" />
                      ) : (
                        <div className="w-9 h-9 rounded bg-muted flex items-center justify-center shrink-0">
                          <Building2 className="h-4 w-4 text-muted-foreground" />
                        </div>
                      )}
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-semibold truncate group-hover:text-primary transition-colors">
                          {company.name}
                        </p>
                        {company.industry && (
                          <p className="text-xs text-muted-foreground truncate">{company.industry}</p>
                        )}
                        {company.headquarters && (
                          <p className="text-xs text-muted-foreground flex items-center gap-1 mt-0.5">
                            <MapPin className="h-3 w-3 shrink-0" />{company.headquarters}
                          </p>
                        )}
                      </div>
                      {company.intelligence_status && company.intelligence_status !== 'idle' && (
                        <span className={`w-2 h-2 rounded-full shrink-0 mt-1.5 ${
                          company.intelligence_status === 'done' ? 'bg-green-500'
                          : company.intelligence_status === 'running' ? 'bg-blue-500 animate-pulse'
                          : 'bg-yellow-500'
                        }`} />
                      )}
                    </div>
                    {company.intelligence_summary && (
                      <p className="text-xs text-muted-foreground mt-2 line-clamp-2">
                        {company.intelligence_summary}
                      </p>
                    )}
                    {company.website && (
                      <p className="text-xs text-primary/70 mt-1 truncate flex items-center gap-1">
                        <Globe className="h-3 w-3 shrink-0" />
                        {company.website.replace(/^https?:\/\//, '')}
                      </p>
                    )}
                  </Link>
                  <div className="flex items-center gap-1.5 pt-0.5">
                    <Link
                      to={`/companies/${company.id}`}
                      className="flex-1 flex items-center justify-center gap-1 text-xs py-1 px-2 rounded border border-border hover:bg-accent transition-colors"
                    >
                      <Building2 className="h-3 w-3" /> Profile
                    </Link>
                    <Link
                      to={`/companies/${company.id}?tab=intelligence`}
                      className="flex-1 flex items-center justify-center gap-1 text-xs py-1 px-2 rounded border border-indigo-700/60 text-indigo-400 hover:bg-indigo-950/40 transition-colors"
                    >
                      <Brain className="h-3 w-3" /> Intel
                    </Link>
                  </div>
                </div>
              );
            })}
          </div>
        )
      )}

      {/* ── Pipeline (deals with full person + company profiles) ───────── */}
      {view === 'pipeline' && (
        filteredPipeline.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Briefcase className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>
              {pipelineLoading
                ? 'Loading pipeline contacts…'
                : 'No pipeline contacts with full profiles yet'}
            </p>
            {!pipelineLoading && (
              <p className="text-xs mt-1">
                Contacts appear here once they have a linked person entity and company in the pipeline.
              </p>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredPipeline.map((deal) => (
              <div key={deal.person_id} className="p-4 rounded-lg border bg-card hover:border-primary/40 transition-all">
                {/* Person row */}
                <div className="flex items-start gap-3">
                  <div className="w-9 h-9 rounded-full bg-muted flex items-center justify-center shrink-0 text-sm font-semibold text-muted-foreground">
                    {deal.contact_name?.charAt(0) ?? '?'}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <p className="text-sm font-semibold truncate">{deal.contact_name}</p>
                      {deal.intelligence_status === 'done' && (
                        <CheckCircle2 className="h-3 w-3 text-green-500 shrink-0" />
                      )}
                    </div>
                    {deal.contact_email && (
                      <p className="text-xs text-muted-foreground truncate">{deal.contact_email}</p>
                    )}
                  </div>
                  {deal.person_id && (
                    <Link to={`/people/${deal.person_id}`}>
                      <ExternalLink className="h-3.5 w-3.5 text-muted-foreground hover:text-primary transition-colors" />
                    </Link>
                  )}
                </div>

                {/* Company row */}
                {deal.contact_company && (
                  <div className="mt-2 flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Building2 className="h-3 w-3 shrink-0" />
                    <span className="truncate">{deal.contact_company}</span>
                    {deal.company_intelligence_status === 'done' && (
                      <CheckCircle2 className="h-3 w-3 text-green-500 shrink-0" />
                    )}
                    {deal.company_id && (
                      <Link
                        to={`/companies/${deal.company_id}`}
                        className="ml-auto shrink-0 hover:text-primary transition-colors"
                      >
                        <ExternalLink className="h-3 w-3" />
                      </Link>
                    )}
                  </div>
                )}

                {/* Stage + analysis badges */}
                <div className="mt-2 flex items-center gap-1.5">
                  <Badge variant="outline" className="text-[10px] h-4 px-1.5">
                    {deal.stage || 'Pipeline'}
                  </Badge>
                  {deal.report_id && (
                    <Badge variant="secondary" className="text-[10px] h-4 px-1.5 gap-1">
                      <Contact2 className="h-2.5 w-2.5" />
                      Analysis
                    </Badge>
                  )}
                </div>
              </div>
            ))}
          </div>
        )
      )}

      {/* ── Add Contact Dialog ─────────────────────────────────────────── */}
      <Dialog open={showAddContact} onOpenChange={setShowAddContact}>
        <DialogContent className="max-w-md">
          <DialogHeader><DialogTitle>Add Contact</DialogTitle></DialogHeader>
          <form onSubmit={handleCreateContact} className="space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>First Name</Label>
                <Input value={contactForm.first_name} onChange={e => setContactForm(f => ({ ...f, first_name: e.target.value }))} placeholder="Jane" />
              </div>
              <div>
                <Label>Last Name</Label>
                <Input value={contactForm.last_name} onChange={e => setContactForm(f => ({ ...f, last_name: e.target.value }))} placeholder="Doe" />
              </div>
            </div>
            <div>
              <Label>Email</Label>
              <Input type="email" value={contactForm.email} onChange={e => setContactForm(f => ({ ...f, email: e.target.value }))} placeholder="jane@example.com" />
            </div>
            <div>
              <Label>Phone</Label>
              <Input value={contactForm.phone} onChange={e => setContactForm(f => ({ ...f, phone: e.target.value }))} placeholder="+1 555-0123" />
            </div>
            <div>
              <Label>Company</Label>
              <Input value={contactForm.company_name} onChange={e => setContactForm(f => ({ ...f, company_name: e.target.value }))} placeholder="Acme Corp" />
            </div>
            <div>
              <Label>Job Title</Label>
              <Input value={contactForm.job_title} onChange={e => setContactForm(f => ({ ...f, job_title: e.target.value }))} placeholder="CTO" />
            </div>
            <div>
              <Label>Lifecycle Stage</Label>
              <Select value={contactForm.lifecycle_stage} onValueChange={v => setContactForm(f => ({ ...f, lifecycle_stage: v }))}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>{info.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowAddContact(false)}>Cancel</Button>
              <Button type="submit" disabled={createContactMutation.isPending}>
                {createContactMutation.isPending ? 'Creating…' : 'Create Contact'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* ── Add Company Dialog ─────────────────────────────────────────── */}
      <Dialog open={showAddCompany} onOpenChange={setShowAddCompany}>
        <DialogContent className="max-w-md">
          <DialogHeader><DialogTitle>Add Company</DialogTitle></DialogHeader>
          <form onSubmit={handleCreateCompany} className="space-y-3">
            <div>
              <Label>Company Name *</Label>
              <Input value={companyForm.name} onChange={e => setCompanyForm(f => ({ ...f, name: e.target.value }))} placeholder="Acme Corp" />
            </div>
            <div>
              <Label>Website</Label>
              <Input value={companyForm.website} onChange={e => setCompanyForm(f => ({ ...f, website: e.target.value }))} placeholder="https://acme.com" />
            </div>
            <div>
              <Label>Industry</Label>
              <Input value={companyForm.industry} onChange={e => setCompanyForm(f => ({ ...f, industry: e.target.value }))} placeholder="Technology" />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowAddCompany(false)}>Cancel</Button>
              <Button type="submit" disabled={createCompanyMutation.isPending || !companyForm.name.trim()}>
                {createCompanyMutation.isPending ? 'Creating…' : 'Add Company'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Contact Detail Modal */}
      {selectedContact && (
        <ContactDetailModal
          contact={selectedContact}
          orgId={orgId}
          open={!!selectedContactId}
          onClose={() => setSelectedContactId(null)}
          personId={dealPersonMap.get(selectedContact.id)?.person_id}
        />
      )}
    </div>
  );
}
