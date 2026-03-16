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
import { Building2, MapPin, Globe, Contact2, Search, Plus } from 'lucide-react';
import {
  crmApi,
  companiesApi,
  type CreateCrmContactRequest,
  type CrmContactRecord,
  type CompanyRecord,
} from '@/lib/api';
import { useCrmContacts, crmContactsQueryKey } from '@/hooks/queries';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';
import { ContactCard } from '../components/ContactCard';
import { ContactDetailModal } from '../components/ContactDetailModal';

export function ContactsTab({ orgId }: { orgId: string }) {
  const [crmView, setCrmView] = useState<'contacts' | 'companies'>('contacts');
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');
  const [showAddContact, setShowAddContact] = useState(false);
  const [showAddCompany, setShowAddCompany] = useState(false);
  const [selectedContactId, setSelectedContactId] = useState<string | null>(null);
  const [contactForm, setContactForm] = useState({ first_name: '', last_name: '', email: '', phone: '', company_name: '', job_title: '', lifecycle_stage: 'lead' });
  const [companyForm, setCompanyForm] = useState({ name: '', website: '', industry: '' });
  const queryClient = useQueryClient();

  const { data: contacts = [], isLoading } = useCrmContacts(orgId);

  const selectedContact = useMemo(
    () => contacts.find((c: CrmContactRecord) => c.id === selectedContactId),
    [contacts, selectedContactId]
  );

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

  const createCompanyMutation = useMutation({
    mutationFn: (data: { name: string; website?: string; industry?: string; created_by_org_id?: string }) => companiesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-companies', orgId] });
      setShowAddCompany(false);
      setCompanyForm({ name: '', website: '', industry: '' });
      toast.success('Company created');
    },
    onError: () => toast.error('Failed to create company'),
  });

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

  const { data: companies = [], isLoading: companiesLoading } = useQuery<CompanyRecord[]>({
    queryKey: ['org-companies', orgId],
    queryFn: () => companiesApi.list({ created_by_org_id: orgId, limit: 500 }),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const filteredContacts = useMemo(() => {
    let result = contacts;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        c =>
          (c.full_name && c.full_name.toLowerCase().includes(q)) ||
          (c.email && c.email.toLowerCase().includes(q)) ||
          (c.company_name && c.company_name.toLowerCase().includes(q))
      );
    }
    if (stageFilter !== 'all') {
      result = result.filter(c => c.lifecycle_stage === stageFilter);
    }
    return result;
  }, [contacts, searchQuery, stageFilter]);

  const filteredCompanies = useMemo(() => {
    if (!searchQuery) return companies;
    const q = searchQuery.toLowerCase();
    return companies.filter(c =>
      c.name.toLowerCase().includes(q) ||
      (c.industry && c.industry.toLowerCase().includes(q)) ||
      (c.headquarters && c.headquarters.toLowerCase().includes(q))
    );
  }, [companies, searchQuery]);

  const stageInfo = LIFECYCLE_STAGE_INFO;

  return (
    <div className="space-y-4">
      {/* Sub-tab toggle */}
      <div className="flex items-center gap-1 p-1 bg-muted rounded-lg w-fit">
        <button
          onClick={() => setCrmView('contacts')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            crmView === 'contacts' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Contact2 className="h-3.5 w-3.5" />
          Contacts
          <span className="text-xs text-muted-foreground">({contacts.length})</span>
        </button>
        <button
          onClick={() => setCrmView('companies')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            crmView === 'companies' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Building2 className="h-3.5 w-3.5" />
          Companies
          <span className="text-xs text-muted-foreground">({companies.length})</span>
        </button>
      </div>

      {/* Search + filter bar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={crmView === 'contacts' ? 'Search contacts...' : 'Search companies...'}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        {crmView === 'contacts' && (
          <select
            value={stageFilter}
            onChange={(e) => setStageFilter(e.target.value)}
            className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          >
            <option value="all">All Stages</option>
            {Object.entries(stageInfo).map(([key, info]) => (
              <option key={key} value={key}>{info.label}</option>
            ))}
          </select>
        )}
        {(isLoading || companiesLoading) && (
          <span className="text-xs text-muted-foreground">Loading...</span>
        )}
        {crmView === 'contacts' ? (
          <Button size="sm" onClick={() => setShowAddContact(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Contact
          </Button>
        ) : (
          <Button size="sm" onClick={() => setShowAddCompany(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Company
          </Button>
        )}
      </div>

      {/* Add Contact Dialog */}
      <Dialog open={showAddContact} onOpenChange={setShowAddContact}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Contact</DialogTitle>
          </DialogHeader>
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
                {createContactMutation.isPending ? 'Creating...' : 'Create Contact'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Add Company Dialog */}
      <Dialog open={showAddCompany} onOpenChange={setShowAddCompany}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Company</DialogTitle>
          </DialogHeader>
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
                {createCompanyMutation.isPending ? 'Creating...' : 'Add Company'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Contacts view */}
      {crmView === 'contacts' && (
        filteredContacts.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{isLoading ? 'Loading contacts...' : 'No contacts found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredContacts.map((contact) => (
              <ContactCard key={contact.id} contact={contact} onClick={() => setSelectedContactId(contact.id)} />
            ))}
          </div>
        )
      )}

      {/* Companies view */}
      {crmView === 'companies' && (
        filteredCompanies.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Building2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{companiesLoading ? 'Loading companies...' : 'No companies found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredCompanies.map(company => (
              <Link
                key={company.id}
                to={`/companies/${company.id}`}
                className="block p-4 rounded-lg border bg-card hover:border-primary/40 hover:shadow-sm transition-all group"
              >
                <div className="flex items-start gap-3">
                  {company.logo_url ? (
                    <img src={company.logo_url} alt="" className="w-9 h-9 rounded object-cover shrink-0" />
                  ) : (
                    <div className="w-9 h-9 rounded bg-muted flex items-center justify-center shrink-0">
                      <Building2 className="h-4 w-4 text-muted-foreground" />
                    </div>
                  )}
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-semibold truncate group-hover:text-primary transition-colors">{company.name}</p>
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
                    <span className={`w-2 h-2 rounded-full shrink-0 mt-1 ${
                      company.intelligence_status === 'done' ? 'bg-green-500' :
                      company.intelligence_status === 'running' ? 'bg-blue-500 animate-pulse' :
                      'bg-yellow-500'
                    }`} />
                  )}
                </div>
                {company.intelligence_summary && (
                  <p className="text-xs text-muted-foreground mt-2 line-clamp-2">{company.intelligence_summary}</p>
                )}
                {company.website && (
                  <p className="text-xs text-primary/70 mt-1 truncate flex items-center gap-1">
                    <Globe className="h-3 w-3 shrink-0" />{company.website.replace(/^https?:\/\//, '')}
                  </p>
                )}
              </Link>
            ))}
          </div>
        )
      )}

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
