import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { Building2, Plus, ExternalLink, Globe } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
  const [name, setName] = useState('');
  const [website, setWebsite] = useState('');
  const [industry, setIndustry] = useState('');

  const create = useMutation({
    mutationFn: () =>
      companiesApi.create({
        name,
        website: website || undefined,
        industry: industry || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['companies'] });
      toast.success('Company created');
      setName('');
      setWebsite('');
      setIndustry('');
      onClose();
    },
    onError: () => toast.error('Failed to create company'),
  });

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New Company</DialogTitle>
        </DialogHeader>
        <div className="grid gap-3 py-2">
          <div className="grid gap-1.5">
            <Label htmlFor="co-name">Name *</Label>
            <Input
              id="co-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Sandals Resorts"
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="co-web">Website</Label>
            <Input
              id="co-web"
              value={website}
              onChange={(e) => setWebsite(e.target.value)}
              placeholder="https://sandals.com"
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="co-ind">Industry</Label>
            <Input
              id="co-ind"
              value={industry}
              onChange={(e) => setIndustry(e.target.value)}
              placeholder="Hospitality"
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            disabled={!name.trim() || create.isPending}
            onClick={() => create.mutate()}
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
          <div className="flex flex-col items-center justify-center h-48 text-muted-foreground gap-2">
            <Building2 className="h-10 w-10 opacity-20" />
            <p className="text-sm">{search ? 'No companies match your search' : 'No companies yet'}</p>
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
