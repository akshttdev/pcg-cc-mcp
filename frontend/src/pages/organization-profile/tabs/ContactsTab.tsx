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
} from 'lucide-react';
import {
  crmApi,
  companiesApi,
  type CreateCrmContactRequest,
  type CrmContactRecord,
  type CompanyRecord,
} from '@/lib/api';
import { personsApi, type PersonRecord } from '@/lib/api/communication';
import { useCrmContacts, crmContactsQueryKey } from '@/hooks/queries';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';
import type { CrmDealWithContact } from '@/types/crm';
import { ContactCard } from '../components/ContactCard';
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

  const { data: contacts = [], isLoading: contactsLoading } = useCrmContacts(orgId);

  const { data: persons = [], isLoading: personsLoading } = useQuery<PersonRecord[]>({
    queryKey: ['persons-all'],
    queryFn: () => personsApi.list({ limit: 500 }),
    staleTime: 60_000,
  });

  const { data: companies = [], isLoading: companiesLoading } = useQuery<CompanyRecord[]>({
    queryKey: ['companies-all'],
    queryFn: () => companiesApi.list({ limit: 500 }),
    staleTime: 60_000,
  });

  // Pipeline contacts: CRM deals for this org that have both a linked person and a company
  const { data: pipelineDeals = [], isLoading: pipelineLoading } = useQuery<CrmDealWithContact[]>({
    queryKey: ['crm-deals-org', orgId],
    queryFn: async () => {
      const res = await fetch(`/api/crm/deals?organization_id=${orgId}`, {
        headers: { Authorization: `Bearer ${localStorage.getItem('token') ?? ''}` },
      });
      const json = await res.json();
      return (json.data ?? []) as CrmDealWithContact[];
    },
    staleTime: 30_000,
  });

  // Deduplicate pipeline contacts by person_id (keep most recent deal per person)
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
      setContactForm({
        first_name: '',
        last_name: '',
        email: '',
        phone: '',
        company_name: '',
        job_title: '',
        lifecycle_stage: 'lead',
      });
      toast.success('Contact created');
    },
    onError: () => toast.error('Failed to create contact'),
  });

  const createCompanyMutation = useMutation({
    mutationFn: (data: {
      name: string;
      website?: string;
      industry?: string;
      created_by_org_id?: string;
    }) => companiesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['companies-all'] });
      setShowAddCompany(false);
      setCompanyForm({ name: '', website: '', industry: '' });
      toast.success('Company created');
    },
    onError: () => toast.error('Failed to create company'),
  });

  const handleCreateContact = useCallback(
    (e: React.FormEvent) => {
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
    },
    [contactForm, orgId, createContactMutation]
  );

  const handleCreateCompany = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (!companyForm.name.trim()) return;
      createCompanyMutation.mutate({
        name: companyForm.name,
        website: companyForm.website || undefined,
        industry: companyForm.industry || undefined,
        created_by_org_id: orgId,
      });
    },
    [companyForm, orgId, createCompanyMutation]
  );

  // ── Filtering ─────────────────────────────────────────────────────────────

  const selectedContact = useMemo(
    () => contacts.find((c: CrmContactRecord) => c.id === selectedContactId),
    [contacts, selectedContactId]
  );

  const filteredPersons = useMemo(() => {
    if (!searchQuery) return persons;
    const q = searchQuery.toLowerCase();
    return persons.filter(
      (p) =>
        (p.full_name && p.full_name.toLowerCase().includes(q)) ||
        (p.email && p.email.toLowerCase().includes(q)) ||
        (p.company_name && p.company_name.toLowerCase().includes(q))
    );
  }, [persons, searchQuery]);

  const filteredCompanies = useMemo(() => {
    if (!searchQuery) return companies;
    const q = searchQuery.toLowerCase();
    return companies.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        (c.industry && c.industry.toLowerCase().includes(q)) ||
        (c.headquarters && c.headquarters.toLowerCase().includes(q))
    );
  }, [companies, searchQuery]);

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
    view === 'people'
      ? personsLoading
      : view === 'companies'
        ? companiesLoading
        : pipelineLoading;

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="space-y-4">
      {/* Three-view toggle */}
      <div className="flex items-center gap-1 p-1 bg-muted rounded-lg w-fit">
        <button
          onClick={() => setView('people')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            view === 'people'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Users className="h-3.5 w-3.5" />
          People
          <span className="text-xs text-muted-foreground">({persons.length})</span>
        </button>
        <button
          onClick={() => setView('companies')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            view === 'companies'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Building2 className="h-3.5 w-3.5" />
          Companies
          <span className="text-xs text-muted-foreground">({companies.length})</span>
        </button>
        <button
          onClick={() => setView('pipeline')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            view === 'pipeline'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
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
              view === 'people'
                ? 'Search people...'
                : view === 'companies'
                  ? 'Search companies...'
                  : 'Search pipeline contacts...'
            }
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
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

      {/* ── People view ───────────────────────────────────────────────────── */}
      {view === 'people' && (
        filteredPersons.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Users className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{personsLoading ? 'Loading people…' : 'No people found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredPersons.map((person) => (
              <Link
                key={person.id}
                to={`/persons/${person.id}`}
                className="block p-4 rounded-lg border bg-card hover:border-primary/40 hover:shadow-sm transition-all group"
              >
                <div className="flex items-start gap-3">
                  {person.avatar_url ? (
                    <img
                      src={person.avatar_url}
                      alt=""
                      className="w-9 h-9 rounded-full object-cover shrink-0"
                    />
                  ) : (
                    <div className="w-9 h-9 rounded-full bg-muted flex items-center justify-center shrink-0 text-sm font-semibold text-muted-foreground">
                      {person.full_name?.charAt(0) ?? '?'}
                    </div>
                  )}
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-semibold truncate group-hover:text-primary transition-colors">
                      {person.full_name ?? 'Unknown'}
                    </p>
                    {person.title && (
                      <p className="text-xs text-muted-foreground truncate">{person.title}</p>
                    )}
                    {person.company_name && (
                      <p className="text-xs text-muted-foreground truncate flex items-center gap-1 mt-0.5">
                        <Building2 className="h-3 w-3 shrink-0" />
                        {person.company_name}
                      </p>
                    )}
                  </div>
                  {person.intelligence_status === 'done' && (
                    <CheckCircle2 className="h-3.5 w-3.5 text-green-500 shrink-0 mt-0.5" />
                  )}
                  {(person.intelligence_status === 'running' ||
                    person.intelligence_status === 'queued') && (
                    <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin shrink-0 mt-0.5" />
                  )}
                </div>
                {person.intelligence_summary && (
                  <p className="text-xs text-muted-foreground mt-2 line-clamp-2">
                    {person.intelligence_summary}
                  </p>
                )}
              </Link>
            ))}
          </div>
        )
      )}

      {/* ── Companies view ────────────────────────────────────────────────── */}
      {view === 'companies' && (
        filteredCompanies.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Building2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{companiesLoading ? 'Loading companies…' : 'No companies found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredCompanies.map((company) => (
              <Link
                key={company.id}
                to={`/companies/${company.id}`}
                className="block p-4 rounded-lg border bg-card hover:border-primary/40 hover:shadow-sm transition-all group"
              >
                <div className="flex items-start gap-3">
                  {company.logo_url ? (
                    <img
                      src={company.logo_url}
                      alt=""
                      className="w-9 h-9 rounded object-cover shrink-0"
                    />
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
                        <MapPin className="h-3 w-3 shrink-0" />
                        {company.headquarters}
                      </p>
                    )}
                  </div>
                  {company.intelligence_status && company.intelligence_status !== 'idle' && (
                    <span
                      className={`w-2 h-2 rounded-full shrink-0 mt-1.5 ${
                        company.intelligence_status === 'done'
                          ? 'bg-green-500'
                          : company.intelligence_status === 'running'
                            ? 'bg-blue-500 animate-pulse'
                            : 'bg-yellow-500'
                      }`}
                    />
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
            ))}
          </div>
        )
      )}

      {/* ── Pipeline view ─────────────────────────────────────────────────── */}
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
                Pipeline contacts appear here once we know both the person and their company.
              </p>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredPipeline.map((deal) => (
              <div
                key={deal.person_id}
                className="p-4 rounded-lg border bg-card hover:border-primary/40 hover:shadow-sm transition-all"
              >
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
                    <Link to={`/persons/${deal.person_id}`}>
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

                {/* Deal stage badge */}
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

      {/* ── Add Contact Dialog ────────────────────────────────────────────── */}
      <Dialog open={showAddContact} onOpenChange={setShowAddContact}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Contact</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleCreateContact} className="space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>First Name</Label>
                <Input
                  value={contactForm.first_name}
                  onChange={(e) => setContactForm((f) => ({ ...f, first_name: e.target.value }))}
                  placeholder="Jane"
                />
              </div>
              <div>
                <Label>Last Name</Label>
                <Input
                  value={contactForm.last_name}
                  onChange={(e) => setContactForm((f) => ({ ...f, last_name: e.target.value }))}
                  placeholder="Doe"
                />
              </div>
            </div>
            <div>
              <Label>Email</Label>
              <Input
                type="email"
                value={contactForm.email}
                onChange={(e) => setContactForm((f) => ({ ...f, email: e.target.value }))}
                placeholder="jane@example.com"
              />
            </div>
            <div>
              <Label>Phone</Label>
              <Input
                value={contactForm.phone}
                onChange={(e) => setContactForm((f) => ({ ...f, phone: e.target.value }))}
                placeholder="+1 555-0123"
              />
            </div>
            <div>
              <Label>Company</Label>
              <Input
                value={contactForm.company_name}
                onChange={(e) => setContactForm((f) => ({ ...f, company_name: e.target.value }))}
                placeholder="Acme Corp"
              />
            </div>
            <div>
              <Label>Job Title</Label>
              <Input
                value={contactForm.job_title}
                onChange={(e) => setContactForm((f) => ({ ...f, job_title: e.target.value }))}
                placeholder="CTO"
              />
            </div>
            <div>
              <Label>Lifecycle Stage</Label>
              <Select
                value={contactForm.lifecycle_stage}
                onValueChange={(v) => setContactForm((f) => ({ ...f, lifecycle_stage: v }))}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>
                      {info.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowAddContact(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={createContactMutation.isPending}>
                {createContactMutation.isPending ? 'Creating…' : 'Create Contact'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* ── Add Company Dialog ────────────────────────────────────────────── */}
      <Dialog open={showAddCompany} onOpenChange={setShowAddCompany}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Company</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleCreateCompany} className="space-y-3">
            <div>
              <Label>Company Name *</Label>
              <Input
                value={companyForm.name}
                onChange={(e) => setCompanyForm((f) => ({ ...f, name: e.target.value }))}
                placeholder="Acme Corp"
              />
            </div>
            <div>
              <Label>Website</Label>
              <Input
                value={companyForm.website}
                onChange={(e) => setCompanyForm((f) => ({ ...f, website: e.target.value }))}
                placeholder="https://acme.com"
              />
            </div>
            <div>
              <Label>Industry</Label>
              <Input
                value={companyForm.industry}
                onChange={(e) => setCompanyForm((f) => ({ ...f, industry: e.target.value }))}
                placeholder="Technology"
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowAddCompany(false)}>
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={createCompanyMutation.isPending || !companyForm.name.trim()}
              >
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
        />
      )}
    </div>
  );
}
