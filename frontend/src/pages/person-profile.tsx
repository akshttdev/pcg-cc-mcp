import { useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { personsApi, intelligenceApi, reportsApi, type PersonRecord, type PersonNote, type PersonSocialProfile, type PersonCompanyRole } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import { toast } from 'sonner';
import {
  ArrowLeft, Building2, Mail, Phone, Globe, Layers, TrendingUp,
  FileText, ChevronRight, Loader2, Zap, Linkedin, Twitter, Instagram,
  Youtube, Github, Facebook, MessageCircle, Star, BookOpen, StickyNote,
  Plus, Pencil, Trash2, CheckCircle2, Clock, AlertCircle, Hash, ExternalLink,
  Briefcase, Target, DollarSign, User,
} from 'lucide-react';

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtDate(dt: string | null | undefined) {
  if (!dt) return '—';
  return new Date(dt).toLocaleString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

function fmtDateTime(dt: string | null | undefined) {
  if (!dt) return '—';
  return new Date(dt).toLocaleString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function depthColor(depth: string | undefined) {
  if (depth === 'deep')     return 'text-emerald-400 border-emerald-700';
  if (depth === 'moderate') return 'text-amber-400 border-amber-700';
  return 'text-slate-500 border-slate-700';
}

function statusBadge(status: string | undefined) {
  const map: Record<string, string> = {
    done:    'border-emerald-700 text-emerald-400',
    running: 'border-blue-700 text-blue-400',
    queued:  'border-amber-700 text-amber-400',
    failed:  'border-red-700 text-red-400',
    idle:    'border-slate-700 text-slate-500',
  };
  return map[status ?? 'idle'] ?? 'border-slate-700 text-slate-500';
}

function lifecycleColor(stage: string) {
  const m: Record<string, string> = {
    lead: 'border-blue-700 text-blue-400',
    mql: 'border-purple-700 text-purple-400',
    sql: 'border-amber-700 text-amber-400',
    opportunity: 'border-orange-700 text-orange-400',
    customer: 'border-emerald-700 text-emerald-400',
    churned: 'border-slate-700 text-slate-500',
  };
  return m[stage] ?? 'border-slate-700 text-slate-400';
}

function parseJson<T>(s: string | undefined | null, fallback: T): T {
  if (!s) return fallback;
  try { return JSON.parse(s) as T; } catch { return fallback; }
}

function PlatformIcon({ platform }: { platform: string }) {
  const size = 'w-4 h-4';
  if (platform === 'linkedin')  return <Linkedin className={size} />;
  if (platform === 'twitter')   return <Twitter className={size} />;
  if (platform === 'instagram') return <Instagram className={size} />;
  if (platform === 'youtube')   return <Youtube className={size} />;
  if (platform === 'github')    return <Github className={size} />;
  if (platform === 'facebook')  return <Facebook className={size} />;
  if (platform === 'tiktok')    return <MessageCircle className={size} />;
  return <Globe className={size} />;
}

function TabBtn({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      className={`px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
        active ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
      }`}
    >
      {children}
    </button>
  );
}

// ── Overview Tab ──────────────────────────────────────────────────────────────

function OverviewTab({ person, companyRoles }: { person: PersonRecord; companyRoles: PersonCompanyRole[] }) {
  const emails = parseJson<{value: string; label: string}[]>(person.emails, []);
  const phones = parseJson<{value: string; label: string}[]>(person.phones, []);
  const tags = parseJson<string[]>(person.tags, []);

  // Parse intelligence raw for key findings
  const intelRaw = parseJson<Record<string, unknown>>(person.intelligence_raw, {});
  const keyFacts = [
    intelRaw.positioning,
    intelRaw.specialization,
    intelRaw.expertise,
    intelRaw.background,
  ].filter(Boolean) as string[];

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
      {/* Contact Card */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5 space-y-3">
        <p className="text-xs font-semibold uppercase tracking-widest text-slate-500 mb-3">Contact</p>

        {/* Multi-emails */}
        {emails.length > 0 ? emails.map((e, i) => (
          <div key={i} className="flex items-center gap-3 text-sm">
            <Mail className="w-4 h-4 text-slate-500 shrink-0" />
            <div className="min-w-0">
              <a href={`mailto:${e.value}`} className="text-indigo-400 hover:text-indigo-300 truncate block">{e.value}</a>
              {e.label && <span className="text-xs text-slate-600">{e.label}</span>}
            </div>
          </div>
        )) : person.email && (
          <div className="flex items-center gap-3 text-sm">
            <Mail className="w-4 h-4 text-slate-500 shrink-0" />
            <a href={`mailto:${person.email}`} className="text-indigo-400 hover:text-indigo-300">{person.email}</a>
          </div>
        )}

        {/* Multi-phones */}
        {phones.length > 0 ? phones.map((p, i) => (
          <div key={i} className="flex items-center gap-3 text-sm">
            <Phone className="w-4 h-4 text-slate-500 shrink-0" />
            <div className="min-w-0">
              <a href={`tel:${p.value}`} className="text-slate-300 hover:text-white truncate block">{p.value}</a>
              {p.label && <span className="text-xs text-slate-600">{p.label}</span>}
            </div>
          </div>
        )) : person.phone && (
          <div className="flex items-center gap-3 text-sm">
            <Phone className="w-4 h-4 text-slate-500 shrink-0" />
            <span className="text-slate-300">{person.phone}</span>
          </div>
        )}

        {person.website && (
          <div className="flex items-center gap-3 text-sm">
            <Globe className="w-4 h-4 text-slate-500 shrink-0" />
            <a href={person.website} target="_blank" rel="noopener noreferrer" className="text-indigo-400 hover:text-indigo-300 truncate">{person.website}</a>
          </div>
        )}

        {person.onboarding_channel && (
          <div className="flex items-center gap-3 text-sm">
            <MessageCircle className="w-4 h-4 text-slate-500 shrink-0" />
            <span className="text-slate-400">via <span className="capitalize text-slate-300">{person.onboarding_channel.replace(/_/g, ' ')}</span></span>
          </div>
        )}
        {person.preferred_contact && (
          <div className="flex items-center gap-3 text-sm">
            <Star className="w-4 h-4 text-amber-500 shrink-0" />
            <span className="text-slate-400">Preferred: <span className="capitalize text-slate-300">{person.preferred_contact.replace(/_/g, ' ')}</span></span>
          </div>
        )}
      </div>

      {/* Profile & Companies */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5 space-y-3">
        <p className="text-xs font-semibold uppercase tracking-widest text-slate-500 mb-3">Profile</p>

        {person.lifecycle_stage && (
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-400">Lifecycle</span>
            <Badge variant="outline" className={`text-xs ${lifecycleColor(person.lifecycle_stage)}`}>
              {person.lifecycle_stage.toUpperCase()}
            </Badge>
          </div>
        )}
        {person.lead_score > 0 && (
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-400">Lead Score</span>
            <div className="flex items-center gap-1.5">
              <div className="w-24 h-1.5 bg-slate-800 rounded-full overflow-hidden">
                <div className="h-full bg-indigo-500 rounded-full" style={{ width: `${Math.min(person.lead_score, 100)}%` }} />
              </div>
              <span className="text-xs text-slate-300 w-7 text-right">{person.lead_score}</span>
            </div>
          </div>
        )}
        {person.financial_role && person.financial_role !== 'neutral' && (
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-400">Financial Role</span>
            <Badge variant="outline" className="text-xs border-slate-700 text-slate-400">
              <DollarSign className="w-3 h-3 mr-0.5" />{person.financial_role}
            </Badge>
          </div>
        )}
        {person.business_stage && (
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-400">Business Stage</span>
            <span className="text-xs text-slate-300 capitalize">{person.business_stage.replace(/_/g, ' ')}</span>
          </div>
        )}
        {person.client_profile && (
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-400">Client Type</span>
            <span className="text-xs text-slate-300 capitalize">{person.client_profile}</span>
          </div>
        )}

        {/* Company associations */}
        {companyRoles.length > 0 && (
          <div className="pt-3 border-t border-slate-800 space-y-2">
            <p className="text-xs font-semibold uppercase tracking-widest text-slate-600 mb-2">Companies</p>
            {companyRoles.map(cr => (
              <div key={cr.id} className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1.5 min-w-0">
                  <Building2 className="w-3.5 h-3.5 text-slate-500 shrink-0" />
                  {cr.company_slug ? (
                    <Link to={`/companies/${cr.company_id}`} className="text-sm text-indigo-400 hover:text-indigo-300 truncate">
                      {cr.company_name ?? cr.company_id}
                    </Link>
                  ) : (
                    <span className="text-sm text-slate-300 truncate">{cr.company_name ?? cr.company_id}</span>
                  )}
                </div>
                {cr.title && <span className="text-xs text-slate-500 shrink-0">{cr.title}</span>}
              </div>
            ))}
          </div>
        )}

        {/* Tags */}
        {tags.length > 0 && (
          <div className="pt-3 border-t border-slate-800">
            <div className="flex flex-wrap gap-1.5">
              {tags.map(t => (
                <span key={t} className="inline-flex items-center gap-0.5 px-2 py-0.5 rounded-full bg-slate-800 text-xs text-slate-400">
                  <Hash className="w-2.5 h-2.5" />{t}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Intel snapshot */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
        <div className="flex items-center justify-between mb-3">
          <p className="text-xs font-semibold uppercase tracking-widest text-slate-500">Intelligence</p>
          <div className="flex items-center gap-2">
            <Badge variant="outline" className={`text-xs ${depthColor(person.research_depth)}`}>
              <Layers className="w-3 h-3 mr-1" />{person.research_pass_count ?? 0} passes
            </Badge>
            <Badge variant="outline" className={`text-xs ${statusBadge(person.intelligence_status)}`}>
              {person.intelligence_status ?? 'idle'}
            </Badge>
          </div>
        </div>

        {person.intelligence_confidence > 0 && (
          <div className="mb-3">
            <div className="flex justify-between text-xs text-slate-500 mb-1">
              <span>Confidence</span>
              <span>{Math.round(person.intelligence_confidence * 100)}%</span>
            </div>
            <div className="h-1.5 bg-slate-800 rounded-full overflow-hidden">
              <div className="h-full bg-indigo-500 rounded-full" style={{ width: `${person.intelligence_confidence * 100}%` }} />
            </div>
          </div>
        )}

        {person.intelligence_summary ? (
          <p className="text-sm text-slate-400 leading-relaxed line-clamp-6">{person.intelligence_summary}</p>
        ) : (
          <p className="text-sm text-slate-600 italic">No intelligence yet.</p>
        )}

        {keyFacts.length > 0 && (
          <div className="mt-3 pt-3 border-t border-slate-800 space-y-1.5">
            {keyFacts.slice(0, 3).map((f, i) => (
              <p key={i} className="text-xs text-slate-500 leading-snug">· {String(f)}</p>
            ))}
          </div>
        )}

        {person.intelligence_last_run_at && (
          <p className="text-xs text-slate-600 mt-3">Last run {fmtDate(person.intelligence_last_run_at)}</p>
        )}
      </div>
    </div>
  );
}

// ── Social Tab ────────────────────────────────────────────────────────────────

function SocialTab({ profiles }: { profiles: PersonSocialProfile[] }) {
  const platformColors: Record<string, string> = {
    linkedin:  'bg-blue-900/30 border-blue-800 text-blue-400',
    twitter:   'bg-slate-900/50 border-slate-700 text-slate-300',
    instagram: 'bg-pink-900/20 border-pink-800 text-pink-400',
    youtube:   'bg-red-900/20 border-red-800 text-red-400',
    github:    'bg-slate-900/50 border-slate-700 text-slate-300',
    tiktok:    'bg-purple-900/20 border-purple-800 text-purple-400',
    facebook:  'bg-blue-900/20 border-blue-800 text-blue-400',
    website:   'bg-slate-900/50 border-slate-700 text-slate-400',
  };

  if (profiles.length === 0) {
    return (
      <div className="text-center py-16 text-slate-500">
        <Globe className="w-10 h-10 mx-auto mb-3 opacity-20" />
        <p className="text-sm">No social profiles linked yet.</p>
        <p className="text-xs mt-1 text-slate-600">Social data is populated automatically during intelligence research.</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      {profiles.map(p => (
        <div key={p.id} className={`rounded-xl border p-5 ${platformColors[p.platform] ?? 'bg-slate-900/50 border-slate-800'}`}>
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-2.5">
              <PlatformIcon platform={p.platform} />
              <div>
                <p className="text-sm font-semibold capitalize">{p.platform}</p>
                {p.handle && <p className="text-xs opacity-70">@{p.handle}</p>}
              </div>
            </div>
            <div className="flex items-center gap-1.5">
              {p.verified === 1 && <CheckCircle2 className="w-4 h-4 text-blue-400" />}
              {p.profile_url && (
                <a href={p.profile_url} target="_blank" rel="noopener noreferrer">
                  <ExternalLink className="w-3.5 h-3.5 opacity-50 hover:opacity-100" />
                </a>
              )}
            </div>
          </div>

          <div className="mt-3 flex items-center gap-4 text-xs opacity-70">
            {p.follower_count != null && (
              <span><span className="font-semibold text-sm opacity-100">{p.follower_count.toLocaleString()}</span> followers</span>
            )}
            {p.following_count != null && (
              <span><span className="font-semibold text-sm opacity-100">{p.following_count.toLocaleString()}</span> following</span>
            )}
          </div>

          {p.bio && <p className="text-xs opacity-60 leading-relaxed mt-2 line-clamp-3">{p.bio}</p>}

          {p.last_synced_at && (
            <p className="text-xs opacity-40 mt-2">Synced {fmtDate(p.last_synced_at)}</p>
          )}
        </div>
      ))}
    </div>
  );
}

// ── Notes Tab ─────────────────────────────────────────────────────────────────

function NotesTab({ personId }: { personId: string }) {
  const qc = useQueryClient();
  const [newText, setNewText] = useState('');
  const [editId, setEditId] = useState<string | null>(null);
  const [editText, setEditText] = useState('');

  const { data: notes = [], isLoading } = useQuery({
    queryKey: ['person-notes', personId],
    queryFn: () => personsApi.listNotes(personId),
  });

  const createMut = useMutation({
    mutationFn: () => personsApi.createNote(personId, newText),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['person-notes', personId] });
      setNewText('');
      toast.success('Note added');
    },
    onError: () => toast.error('Failed to add note'),
  });

  const updateMut = useMutation({
    mutationFn: ({ id, text }: { id: string; text: string }) =>
      personsApi.updateNote(id, { text }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['person-notes', personId] });
      setEditId(null);
      toast.success('Note updated');
    },
    onError: () => toast.error('Failed to update note'),
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => personsApi.deleteNote(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['person-notes', personId] });
      toast.success('Note deleted');
    },
    onError: () => toast.error('Failed to delete note'),
  });

  const noteStatusIcon = (status: string) => {
    if (status === 'resolved') return <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />;
    if (status === 'follow_up') return <Clock className="w-3.5 h-3.5 text-amber-400" />;
    if (status === 'pinned') return <Star className="w-3.5 h-3.5 text-yellow-400" />;
    return <AlertCircle className="w-3.5 h-3.5 text-slate-500" />;
  };

  if (isLoading) return <div className="flex justify-center py-12"><Loader2 className="w-5 h-5 animate-spin text-indigo-400" /></div>;

  return (
    <div className="space-y-4">
      {/* Add note */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-4 space-y-3">
        <p className="text-xs font-semibold uppercase tracking-widest text-slate-500">Add Note</p>
        <Textarea
          value={newText}
          onChange={e => setNewText(e.target.value)}
          placeholder="Add a note about this person..."
          className="bg-transparent border-slate-700 text-slate-200 resize-none text-sm"
          rows={3}
        />
        <Button
          size="sm"
          onClick={() => createMut.mutate()}
          disabled={!newText.trim() || createMut.isPending}
          className="gap-1.5 bg-indigo-600 hover:bg-indigo-500"
        >
          {createMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Plus className="w-4 h-4" />}
          Add Note
        </Button>
      </div>

      {/* Notes list */}
      {notes.length === 0 ? (
        <div className="text-center py-12 text-slate-500">
          <StickyNote className="w-8 h-8 mx-auto mb-2 opacity-20" />
          <p className="text-sm">No notes yet.</p>
        </div>
      ) : (
        notes.map(note => (
          <div key={note.id} className="rounded-xl border border-slate-800 bg-slate-900/50 p-4">
            <div className="flex items-start justify-between gap-3">
              <div className="flex items-center gap-2 min-w-0">
                {noteStatusIcon(note.status)}
                <span className="text-xs text-slate-500 capitalize">{note.status.replace(/_/g, ' ')}</span>
                <span className="text-xs text-slate-600">· {fmtDateTime(note.created_at)}</span>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                <button
                  onClick={() => { setEditId(note.id); setEditText(note.text); }}
                  className="p-1 text-slate-600 hover:text-slate-400 transition-colors"
                >
                  <Pencil className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => deleteMut.mutate(note.id)}
                  className="p-1 text-slate-600 hover:text-red-400 transition-colors"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            {editId === note.id ? (
              <div className="mt-3 space-y-2">
                <Textarea
                  value={editText}
                  onChange={e => setEditText(e.target.value)}
                  className="bg-transparent border-slate-700 text-slate-200 resize-none text-sm"
                  rows={3}
                />
                <div className="flex gap-2">
                  <Button size="sm" onClick={() => updateMut.mutate({ id: note.id, text: editText })}
                    disabled={updateMut.isPending} className="h-7 text-xs bg-indigo-600 hover:bg-indigo-500">Save</Button>
                  <Button size="sm" variant="ghost" onClick={() => setEditId(null)} className="h-7 text-xs text-slate-400">Cancel</Button>
                </div>
              </div>
            ) : (
              <p className="mt-2 text-sm text-slate-300 leading-relaxed whitespace-pre-wrap">{note.text}</p>
            )}
          </div>
        ))
      )}
    </div>
  );
}

// ── Research Tab ──────────────────────────────────────────────────────────────

function ResearchTab({ personId }: { personId: string }) {
  const queryClient = useQueryClient();
  const { data: passes = [], isLoading } = useQuery({
    queryKey: ['research-passes', personId],
    queryFn: () => intelligenceApi.listResearchPasses(personId),
  });

  const triggerMut = useMutation({
    mutationFn: () => intelligenceApi.triggerNextPass(personId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['research-passes', personId] });
      queryClient.invalidateQueries({ queryKey: ['person', personId] });
      toast.success('Research pass queued');
    },
    onError: () => toast.error('Failed to trigger research'),
  });

  const focusColors: Record<string, string> = {
    identity:        'text-blue-400',
    market_position: 'text-emerald-400',
    competitors:     'text-amber-400',
    target_clients:  'text-purple-400',
    deep_strategy:   'text-red-400',
  };

  if (isLoading) return <div className="flex justify-center py-12"><Loader2 className="w-5 h-5 animate-spin text-indigo-400" /></div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-slate-400">{passes.length} research pass{passes.length !== 1 ? 'es' : ''} completed</p>
        <Button
          size="sm"
          onClick={() => triggerMut.mutate()}
          disabled={triggerMut.isPending}
          className="gap-2 bg-indigo-600 hover:bg-indigo-500"
        >
          {triggerMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Layers className="w-4 h-4" />}
          Run Next Pass
        </Button>
      </div>

      {passes.length === 0 ? (
        <div className="text-center py-12 text-slate-500">
          <Layers className="w-8 h-8 mx-auto mb-2 opacity-30" />
          <p className="text-sm">No research passes yet. Run the first pass to start building intelligence.</p>
        </div>
      ) : (
        passes.map((pass) => {
          const findings = parseJson<string[]>(typeof pass.key_findings === 'string' ? pass.key_findings : JSON.stringify(pass.key_findings), []);
          return (
            <div key={pass.id} className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="w-7 h-7 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center">
                    <span className="text-xs font-bold text-slate-300">{pass.pass_number}</span>
                  </div>
                  <span className={`text-sm font-medium capitalize ${focusColors[pass.research_focus ?? ''] ?? 'text-slate-300'}`}>
                    {(pass.research_focus ?? '').replace(/_/g, ' ')}
                  </span>
                  {pass.confidence_delta != null && pass.confidence_delta !== 0 && (
                    <span className={`text-xs ${pass.confidence_delta > 0 ? 'text-emerald-400' : 'text-red-400'}`}>
                      {pass.confidence_delta > 0 ? '+' : ''}{Math.round(pass.confidence_delta * 100)}%
                    </span>
                  )}
                </div>
                <Badge variant="outline" className={statusBadge(pass.status)}>
                  {pass.status}
                </Badge>
              </div>
              {pass.summary && (
                <p className="text-sm text-slate-400 leading-relaxed mb-3">{pass.summary}</p>
              )}
              {findings.length > 0 && (
                <div className="space-y-1">
                  {findings.map((f, i) => (
                    <div key={i} className="flex items-start gap-2 text-xs text-slate-500">
                      <span className="text-indigo-500 shrink-0 mt-0.5">·</span>
                      <span className="leading-relaxed">{f}</span>
                    </div>
                  ))}
                </div>
              )}
              <p className="text-xs text-slate-600 mt-3">{fmtDate(pass.created_at)}</p>
            </div>
          );
        })
      )}
    </div>
  );
}

// ── Reports Tab ───────────────────────────────────────────────────────────────

function ReportsTab({ personId }: { personId: string }) {
  const { data: reports = [], isLoading } = useQuery({
    queryKey: ['person-reports', personId],
    queryFn: () => intelligenceApi.listPersonReports(personId),
  });

  const generateMut = useMutation({
    mutationFn: () => reportsApi.generate(personId, 'business_audit'),
    onSuccess: () => toast.success('Report generation started'),
    onError: () => toast.error('Failed to generate report'),
  });

  if (isLoading) return <div className="flex justify-center py-12"><Loader2 className="w-5 h-5 animate-spin text-indigo-400" /></div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-slate-400">{reports.length} report{reports.length !== 1 ? 's' : ''}</p>
        <Button
          size="sm"
          onClick={() => generateMut.mutate()}
          disabled={generateMut.isPending}
          className="gap-2 bg-indigo-600 hover:bg-indigo-500"
        >
          {generateMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Zap className="w-4 h-4" />}
          Generate Report
        </Button>
      </div>

      {reports.length === 0 ? (
        <div className="text-center py-12 text-slate-500">
          <FileText className="w-8 h-8 mx-auto mb-2 opacity-30" />
          <p className="text-sm">No reports yet. Generate a business analytics report for this lead.</p>
        </div>
      ) : (
        reports.map((r) => (
          <Link key={r.id} to={`/business-reports/${r.id}`} className="block group">
            <div className="rounded-xl border border-slate-800 bg-slate-900/50 hover:border-indigo-700/50 hover:bg-slate-900 transition-all p-5">
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <p className="font-semibold text-slate-200 group-hover:text-white transition-colors">{r.title}</p>
                    <Badge variant="outline" className={`shrink-0 text-[10px] ${
                      r.status === 'ready' ? 'border-emerald-700 text-emerald-400' : 'border-amber-700 text-amber-400'
                    }`}>{r.status}</Badge>
                  </div>
                  {r.executive_summary && (
                    <p className="text-sm text-slate-500 line-clamp-2">{r.executive_summary}</p>
                  )}
                  <p className="text-xs text-slate-600 mt-2">{fmtDate(r.created_at)}</p>
                </div>
                <ChevronRight className="w-4 h-4 text-slate-600 group-hover:text-slate-400 shrink-0 mt-1" />
              </div>
            </div>
          </Link>
        ))
      )}
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function PersonProfilePage() {
  const { personId } = useParams<{ personId: string }>();
  const navigate = useNavigate();
  const [tab, setTab] = useState<'overview' | 'social' | 'notes' | 'research' | 'reports'>('overview');

  const { data: person, isLoading } = useQuery({
    queryKey: ['person', personId],
    queryFn: () => personsApi.get(personId!),
    enabled: !!personId,
  });

  const triggerResearchMut = useMutation({
    mutationFn: () => intelligenceApi.triggerResearch(personId!),
    onSuccess: () => toast.success('Research triggered'),
    onError: () => toast.error('Failed to trigger research'),
  });

  if (isLoading) return (
    <div className="flex items-center justify-center h-64">
      <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
    </div>
  );

  if (!person) return <div className="p-8 text-slate-500">Person not found.</div>;

  const initials = person.full_name.split(' ').slice(0, 2).map((w: string) => w[0]?.toUpperCase() ?? '').join('');
  const primaryCompany = person.company_roles?.find(r => r.is_primary === 1) ?? person.company_roles?.[0];

  return (
    <div className="max-w-4xl mx-auto px-4 py-8 space-y-6">
      {/* Back */}
      <Button variant="ghost" size="sm" className="gap-2 text-slate-400" onClick={() => window.history.back()}>
        <ArrowLeft className="w-4 h-4" /> Back
      </Button>

      {/* Header card */}
      <div className="rounded-2xl border border-slate-800 bg-slate-900/60 p-6">
        <div className="flex items-start gap-5">
          {/* Avatar */}
          <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white text-xl font-bold shrink-0 shadow-lg">
            {person.avatar_url
              ? <img src={person.avatar_url} alt={person.full_name} className="w-full h-full object-cover rounded-2xl" />
              : initials}
          </div>

          <div className="min-w-0 flex-1">
            <h1 className="text-2xl font-bold text-white mb-0.5">{person.full_name}</h1>
            <div className="flex items-center gap-2 flex-wrap text-sm text-slate-400">
              {person.job_title && <span>{person.job_title}</span>}
              {person.job_title && (person.company_name || primaryCompany) && <span className="text-slate-600">@</span>}
              {primaryCompany ? (
                <Link to={`/companies/${primaryCompany.company_id}`} className="flex items-center gap-1 text-indigo-400 hover:text-indigo-300">
                  <Building2 className="w-3.5 h-3.5" /> {primaryCompany.company_name ?? person.company_name}
                </Link>
              ) : person.company_name ? (
                <span className="flex items-center gap-1"><Building2 className="w-3.5 h-3.5" /> {person.company_name}</span>
              ) : null}
            </div>

            <div className="flex items-center gap-2 mt-3 flex-wrap">
              <Badge variant="outline" className={`text-xs ${
                person.person_type === 'lead'   ? 'border-amber-700 text-amber-400' :
                person.person_type === 'client' ? 'border-emerald-700 text-emerald-400' :
                'border-slate-700 text-slate-400'
              }`}>
                <User className="w-3 h-3 mr-1" />{person.person_type}
              </Badge>
              <Badge variant="outline" className={`text-xs ${lifecycleColor(person.lifecycle_stage)}`}>
                <Target className="w-3 h-3 mr-1" />{person.lifecycle_stage}
              </Badge>
              <Badge variant="outline" className={`text-xs ${depthColor(person.research_depth)}`}>
                <Layers className="w-3 h-3 mr-1" />
                {person.research_pass_count ?? 0} passes · {person.research_depth ?? 'shallow'}
              </Badge>
              {person.company_id && (
                <Link to={`/companies/${person.company_id}`}>
                  <Badge variant="outline" className="text-xs border-slate-700 text-slate-400 hover:border-indigo-700 hover:text-indigo-400 cursor-pointer">
                    <BookOpen className="w-3 h-3 mr-1" />Company Wiki
                  </Badge>
                </Link>
              )}
            </div>
          </div>

          {/* Action buttons */}
          <div className="flex flex-col gap-2 shrink-0">
            <Button
              size="sm"
              variant="outline"
              className="gap-2 text-slate-400 border-slate-700"
              onClick={() => triggerResearchMut.mutate()}
              disabled={triggerResearchMut.isPending || person.intelligence_status === 'running' || person.intelligence_status === 'queued'}
            >
              {triggerResearchMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <TrendingUp className="w-4 h-4" />}
              Research
            </Button>
            <Button size="sm" variant="outline" className="gap-2 text-slate-400 border-slate-700"
              onClick={() => navigate(`/people/${personId}/intel`)}>
              <FileText className="w-4 h-4" /> Intel
            </Button>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 flex-wrap">
        <TabBtn active={tab === 'overview'} onClick={() => setTab('overview')}>Overview</TabBtn>
        <TabBtn active={tab === 'social'} onClick={() => setTab('social')}>
          Social {(person.social_profiles?.length ?? 0) > 0 && `(${person.social_profiles!.length})`}
        </TabBtn>
        <TabBtn active={tab === 'notes'} onClick={() => setTab('notes')}>Notes</TabBtn>
        <TabBtn active={tab === 'research'} onClick={() => setTab('research')}>
          Research {(person.research_pass_count ?? 0) > 0 && `(${person.research_pass_count})`}
        </TabBtn>
        <TabBtn active={tab === 'reports'} onClick={() => setTab('reports')}>Intel</TabBtn>
      </div>

      {/* Tab content */}
      {tab === 'overview'  && <OverviewTab person={person} companyRoles={person.company_roles ?? []} />}
      {tab === 'social'    && <SocialTab profiles={person.social_profiles ?? []} />}
      {tab === 'notes'     && <NotesTab personId={personId!} />}
      {tab === 'research'  && <ResearchTab personId={personId!} />}
      {tab === 'reports'   && <ReportsTab personId={personId!} />}
    </div>
  );
}
