import { useState, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { Building2, Plus, ExternalLink, Globe } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Skeleton } from '@/components/ui/skeleton';
import { toast } from 'sonner';
import { companiesApi, type CompanyRecord } from '@/lib/api';

// ── Intelligence status badge ─────────────────────────────────────────────────

function IntelBadge({ status }: { status: string }) {
  const cfg: Record<string, { label: string; cls: string }> = {
    idle:    { label: 'No Intel',  cls: 'bg-gray-100 text-gray-500' },
    queued:  { label: 'Queued',    cls: 'bg-yellow-100 text-yellow-700' },
    running: { label: 'Running',   cls: 'bg-blue-100 text-blue-700' },
    done:    { label: 'Intel ✓',   cls: 'bg-green-100 text-green-700' },
    failed:  { label: 'Failed',    cls: 'bg-red-100 text-red-700' },
  };
  const { label, cls } = cfg[status] ?? cfg['idle'];
  return <Badge className={`text-xs border-0 px-1.5 py-0 ${cls}`}>{label}</Badge>;
}

// ── Create company dialog ─────────────────────────────────────────────────────

function CreateCompanyDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [formData, setFormData] = useState({
    name: '',
    website: '',
    industry: '',
    description: '',
    headquarters: '',
    phone: '',
    email: '',
    linkedin_url: '',
    twitter_handle: '',
    instagram_handle: '',
  });

  const resetForm = useCallback(() => {
    setFormData({
      name: '',
      website: '',
      industry: '',
      description: '',
      headquarters: '',
      phone: '',
      email: '',
      linkedin_url: '',
      twitter_handle: '',
      instagram_handle: '',
    });
  }, []);

  const create = useMutation({
    mutationFn: () =>
      companiesApi.create({
        name: formData.name,
        website: formData.website || undefined,
        industry: formData.industry || undefined,
        description: formData.description || undefined,
        headquarters: formData.headquarters || undefined,
      }),
    onSuccess: (company) => {
      // If extra fields were provided, update them via PATCH
      const extraFields: Record<string, string> = {};
      if (formData.phone) extraFields.phone = formData.phone;
      if (formData.email) extraFields.email = formData.email;
      if (formData.linkedin_url) extraFields.linkedin_url = formData.linkedin_url;
      if (formData.twitter_handle) extraFields.twitter_handle = formData.twitter_handle;
      if (formData.instagram_handle) extraFields.instagram_handle = formData.instagram_handle;

      if (Object.keys(extraFields).length > 0) {
        companiesApi.update(company.id, extraFields).catch(() => {
          // Non-critical — company was already created
        });
      }

      queryClient.invalidateQueries({ queryKey: ['companies'] });
      toast.success('Company created');
      resetForm();
      onClose();
    },
    onError: () => toast.error('Failed to create company'),
  });

  const handleFieldChange = useCallback(
    (field: string) => (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      setFormData((prev) => ({ ...prev, [field]: e.target.value }));
    },
    [],
  );

  const handleSubmit = useCallback(() => {
    create.mutate();
  }, [create]);

  const handleOpenChange = useCallback(
    (v: boolean) => {
      if (!v) onClose();
    },
    [onClose],
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[550px]">
        <DialogHeader>
          <DialogTitle>New Company</DialogTitle>
        </DialogHeader>
        <div className="grid gap-3 py-2 max-h-[60vh] overflow-y-auto pr-1">
          <div className="grid gap-1.5">
            <Label htmlFor="co-name">Name *</Label>
            <Input
              id="co-name"
              value={formData.name}
              onChange={handleFieldChange('name')}
              placeholder="Sandals Resorts"
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="grid gap-1.5">
              <Label htmlFor="co-web">Website</Label>
              <Input
                id="co-web"
                value={formData.website}
                onChange={handleFieldChange('website')}
                placeholder="https://sandals.com"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-ind">Industry</Label>
              <Input
                id="co-ind"
                value={formData.industry}
                onChange={handleFieldChange('industry')}
                placeholder="Hospitality"
              />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="co-desc">Description</Label>
            <Textarea
              id="co-desc"
              value={formData.description}
              onChange={handleFieldChange('description')}
              placeholder="Brief description of the company..."
              rows={2}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="co-hq">Headquarters</Label>
            <Input
              id="co-hq"
              value={formData.headquarters}
              onChange={handleFieldChange('headquarters')}
              placeholder="Miami, FL"
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="grid gap-1.5">
              <Label htmlFor="co-phone">Phone</Label>
              <Input
                id="co-phone"
                value={formData.phone}
                onChange={handleFieldChange('phone')}
                placeholder="+1 555-0100"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-email">Email</Label>
              <Input
                id="co-email"
                type="email"
                value={formData.email}
                onChange={handleFieldChange('email')}
                placeholder="info@company.com"
              />
            </div>
          </div>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="grid gap-1.5">
              <Label htmlFor="co-li">LinkedIn</Label>
              <Input
                id="co-li"
                value={formData.linkedin_url}
                onChange={handleFieldChange('linkedin_url')}
                placeholder="linkedin.com/company/..."
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-tw">Twitter</Label>
              <Input
                id="co-tw"
                value={formData.twitter_handle}
                onChange={handleFieldChange('twitter_handle')}
                placeholder="@handle"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-ig">Instagram</Label>
              <Input
                id="co-ig"
                value={formData.instagram_handle}
                onChange={handleFieldChange('instagram_handle')}
                placeholder="@handle"
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            disabled={!formData.name.trim() || create.isPending}
            onClick={handleSubmit}
          >
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function CompaniesPage() {
  const navigate = useNavigate();
  const [showCreate, setShowCreate] = useState(false);
  const [search, setSearch] = useState('');

  const { data: companies = [], isLoading } = useQuery({
    queryKey: ['companies'],
    queryFn: () => companiesApi.list({ limit: 200 }),
  });

  const filtered = companies.filter(
    (c) =>
      !search ||
      c.name.toLowerCase().includes(search.toLowerCase()) ||
      (c.industry ?? '').toLowerCase().includes(search.toLowerCase()) ||
      (c.headquarters ?? '').toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b shrink-0">
        <div className="flex items-center gap-2">
          <Building2 className="h-5 w-5 text-muted-foreground" />
          <h1 className="text-xl font-semibold">Companies</h1>
          <Badge variant="outline" className="text-xs">
            {companies.length}
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <Input
            className="h-8 w-52 text-sm"
            placeholder="Search…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <Button size="sm" onClick={() => setShowCreate(true)}>
            <Plus className="h-4 w-4 mr-1" />
            New Company
          </Button>
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {isLoading ? (
          <div className="space-y-2">
            {[...Array(6)].map((_, i) => (
              <Skeleton key={i} className="h-12 w-full" />
            ))}
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
            <Building2 className="h-12 w-12 mb-3 opacity-30" />
            <p className="text-lg font-semibold mb-1 text-foreground">
              {search ? 'No companies match your search' : 'No companies yet'}
            </p>
            <p className="text-sm max-w-sm text-center">
              {search
                ? 'Try adjusting your search terms.'
                : 'Add companies to track organizations, run intelligence, and link them to your CRM.'}
            </p>
            {!search && (
              <Button size="sm" className="mt-4" onClick={() => setShowCreate(true)}>
                <Plus className="h-4 w-4 mr-1" />
                New Company
              </Button>
            )}
          </div>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b text-left text-muted-foreground">
                <th className="pb-2 font-medium">Name</th>
                <th className="pb-2 font-medium">Industry</th>
                <th className="pb-2 font-medium">Headquarters</th>
                <th className="pb-2 font-medium">Intel</th>
                <th className="pb-2 font-medium">Platform Org</th>
                <th className="pb-2 font-medium w-8" />
              </tr>
            </thead>
            <tbody>
              {filtered.map((c) => (
                <CompanyRow
                  key={c.id}
                  company={c}
                  onClick={() => navigate(`/companies/${c.id}`)}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>

      <CreateCompanyDialog open={showCreate} onClose={() => setShowCreate(false)} />
    </div>
  );
}

function CompanyRow({
  company,
  onClick,
}: {
  company: CompanyRecord;
  onClick: () => void;
}) {
  return (
    <tr
      className="border-b last:border-0 hover:bg-muted/40 cursor-pointer transition-colors"
      onClick={onClick}
    >
      <td className="py-3 pr-4">
        <div className="flex items-center gap-2">
          <Building2 className="h-4 w-4 text-muted-foreground shrink-0" />
          <span className="font-medium">{company.name}</span>
        </div>
        {company.website && (
          <a
            href={company.website}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(e) => e.stopPropagation()}
            className="flex items-center gap-1 text-xs text-blue-500 hover:underline mt-0.5 ml-6"
          >
            <Globe className="h-3 w-3" />
            {company.website.replace(/^https?:\/\//, '').replace(/\/$/, '')}
          </a>
        )}
      </td>
      <td className="py-3 pr-4 text-muted-foreground">{company.industry ?? '—'}</td>
      <td className="py-3 pr-4 text-muted-foreground">{company.headquarters ?? '—'}</td>
      <td className="py-3 pr-4">
        <IntelBadge status={company.intelligence_status} />
      </td>
      <td className="py-3 pr-4">
        {company.organization_id ? (
          <Badge className="text-xs border-0 bg-indigo-100 text-indigo-700 px-1.5 py-0">
            Platform
          </Badge>
        ) : (
          <span className="text-xs text-muted-foreground">—</span>
        )}
      </td>
      <td className="py-3">
        <ExternalLink className="h-3.5 w-3.5 text-muted-foreground opacity-0 group-hover:opacity-100" />
      </td>
    </tr>
  );
}
