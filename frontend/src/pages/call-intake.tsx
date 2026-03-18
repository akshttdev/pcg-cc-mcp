import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { formatDateTime } from '@/lib/formatters';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  PhoneIncoming, RefreshCw, Play, FileText, Briefcase,
  CheckCircle, XCircle, Clock, Loader2, Upload,
  ChevronDown, ChevronRight, Plus, User, Brain,
} from 'lucide-react';
import { toast } from 'sonner';
import { callIntakeApi, reportsApi } from '@/lib/api';
import { useAuth } from '@/contexts/AuthContext';

interface CallIntakeItem {
  id: string;
  source_type: string;
  subject: string | null;
  from_name: string | null;
  from_email: string | null;
  call_date: string | null;
  status: string;
  call_summary: string | null;
  extracted_participants: string; // JSON
  extracted_topics: string; // JSON
  extracted_pain_points: string; // JSON
  extracted_action_items: string; // JSON
  extracted_sentiment: string | null;
  person_id: string | null;
  company_id: string | null;
  company_name: string | null;
  report_id: string | null;
  crm_deal_id: string | null;
  created_at: string;
  processed_at: string | null;
  error: string | null;
}



const STATUS_COLOR: Record<string, string> = {
  pending: 'bg-yellow-500/10 text-yellow-400 border-yellow-500/20',
  processing: 'bg-blue-500/10 text-blue-400 border-blue-500/20',
  processed: 'bg-green-500/10 text-green-400 border-green-500/20',
  failed: 'bg-red-500/10 text-red-400 border-red-500/20',
};

const STATUS_ICON: Record<string, React.ComponentType<{ className?: string }>> = {
  pending: Clock,
  processing: Loader2,
  processed: CheckCircle,
  failed: XCircle,
};

function parseJson<T>(str: string, fallback: T): T {
  try { return JSON.parse(str); } catch { return fallback; }
}


export default function CallIntakePage() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const [items, setItems] = useState<CallIntakeItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [processing, setProcessing] = useState<Set<string>>(new Set());
  const [showUpload, setShowUpload] = useState(false);

  // Upload form state
  const [uploadContent, setUploadContent] = useState('');
  const [uploadSubject, setUploadSubject] = useState('');
  const [uploadFromName, setUploadFromName] = useState('');
  const [uploadFromEmail, setUploadFromEmail] = useState('');
  const [uploadSourceType, setUploadSourceType] = useState('email');
  const [uploading, setUploading] = useState(false);

  const fetchItems = useCallback(async () => {
    try {
      const data = await callIntakeApi.list();
      setItems((data ?? []) as CallIntakeItem[]);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchItems(); }, [fetchItems]);

  // Poll for items in processing state
  useEffect(() => {
    const hasProcessing = items.some(i => i.status === 'processing');
    if (!hasProcessing) return;
    const t = setTimeout(fetchItems, 3000);
    return () => clearTimeout(t);
  }, [items, fetchItems]);

  const triggerProcess = async (id: string) => {
    setProcessing(prev => new Set([...prev, id]));
    try {
      await callIntakeApi.process(id);
      await fetchItems();
    } catch {
      // ignore
    } finally {
      setProcessing(prev => { const s = new Set(prev); s.delete(id); return s; });
    }
  };

  const handleUpload = async () => {
    if (!uploadContent.trim()) return;
    setUploading(true);
    try {
      await callIntakeApi.submitEmail({
        raw_content: uploadContent,
        subject: uploadSubject || null,
        from_name: uploadFromName || null,
        from_email: uploadFromEmail || null,
        source_type: uploadSourceType,
        auto_process: true,
      });
      setShowUpload(false);
      setUploadContent('');
      setUploadSubject('');
      setUploadFromName('');
      setUploadFromEmail('');
      await fetchItems();
    } finally {
      setUploading(false);
    }
  };

  const statusCounts = {
    pending: items.filter(i => i.status === 'pending').length,
    processing: items.filter(i => i.status === 'processing').length,
    processed: items.filter(i => i.status === 'processed').length,
    failed: items.filter(i => i.status === 'failed').length,
  };

  return (
    <div className="flex-1 overflow-auto bg-gray-950 min-h-screen">
      <div className="max-w-5xl mx-auto px-6 py-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center">
              <PhoneIncoming className="w-5 h-5 text-blue-400" />
            </div>
            <div>
              <h1 className="text-xl font-semibold text-white">Call Intake</h1>
              <p className="text-sm text-gray-400">Process call transcripts and email summaries into CRM intelligence</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={fetchItems} className="text-gray-400 hover:text-white">
              <RefreshCw className="w-4 h-4" />
            </Button>
            <Button size="sm" onClick={() => setShowUpload(v => !v)}
              className="bg-blue-600 hover:bg-blue-700 text-white gap-2">
              <Plus className="w-4 h-4" /> Add Intake
            </Button>
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          {Object.entries(statusCounts).map(([status, count]) => {
            const Icon = STATUS_ICON[status] ?? Clock;
            return (
              <div key={status} className="bg-gray-900 border border-gray-800 rounded-xl p-4">
                <div className="flex items-center gap-2 mb-1">
                  <Icon className="w-4 h-4 text-gray-400" />
                  <span className="text-xs text-gray-400 capitalize">{status}</span>
                </div>
                <div className="text-2xl font-bold text-white">{count}</div>
              </div>
            );
          })}
        </div>

        {/* Upload form */}
        {showUpload && (
          <div className="bg-gray-900 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
              <Upload className="w-4 h-4 text-blue-400" /> Add Call / Email Content
            </h3>
            <div className="grid grid-cols-2 gap-4 mb-4">
              <div>
                <Label className="text-xs text-gray-400 mb-1 block">Source Type</Label>
                <select
                  value={uploadSourceType}
                  onChange={e => setUploadSourceType(e.target.value)}
                  className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white">
                  <option value="email">Email</option>
                  <option value="transcript">Call Transcript</option>
                  <option value="summary">Call Summary</option>
                  <option value="notes">Meeting Notes</option>
                </select>
              </div>
              <div>
                <Label className="text-xs text-gray-400 mb-1 block">Subject / Title</Label>
                <Input value={uploadSubject} onChange={e => setUploadSubject(e.target.value)}
                  placeholder="e.g. Follow-up with Acme Corp" className="bg-gray-800 border-gray-700 text-white" />
              </div>
              <div>
                <Label className="text-xs text-gray-400 mb-1 block">From Name</Label>
                <Input value={uploadFromName} onChange={e => setUploadFromName(e.target.value)}
                  placeholder="Contact name" className="bg-gray-800 border-gray-700 text-white" />
              </div>
              <div>
                <Label className="text-xs text-gray-400 mb-1 block">From Email</Label>
                <Input value={uploadFromEmail} onChange={e => setUploadFromEmail(e.target.value)}
                  placeholder="contact@company.com" className="bg-gray-800 border-gray-700 text-white" />
              </div>
            </div>
            <div className="mb-4">
              <Label className="text-xs text-gray-400 mb-1 block">Content (paste transcript, email, or notes)</Label>
              <Textarea
                value={uploadContent}
                onChange={e => setUploadContent(e.target.value)}
                rows={8}
                placeholder="Paste the call transcript, email thread, or meeting notes here..."
                className="bg-gray-800 border-gray-700 text-white text-sm font-mono resize-none"
              />
            </div>
            <div className="flex gap-2">
              <Button onClick={handleUpload} disabled={uploading || !uploadContent.trim()}
                className="bg-blue-600 hover:bg-blue-700 text-white gap-2">
                {uploading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
                {uploading ? 'Processing…' : 'Submit & Process'}
              </Button>
              <Button variant="ghost" onClick={() => setShowUpload(false)} className="text-gray-400">Cancel</Button>
            </div>
          </div>
        )}

        {/* Item list */}
        <div className="space-y-2">
          {loading ? (
            <div className="text-center py-16 text-gray-500">
              <Loader2 className="w-6 h-6 animate-spin mx-auto mb-2" />
              Loading intake items…
            </div>
          ) : items.length === 0 ? (
            <div className="text-center py-16 text-gray-500">
              <div className="rounded-full bg-gray-800 p-4 mb-4 inline-flex">
                <PhoneIncoming className="w-8 h-8 text-gray-500" />
              </div>
              <h3 className="text-base font-medium text-gray-300 mb-1">No intake items yet</h3>
              <p className="text-sm">Add call transcripts or email summaries to start building CRM intelligence.</p>
              <Button variant="outline" size="sm" className="mt-4" onClick={() => setShowUpload(true)}>
                <Plus className="w-4 h-4 mr-1" /> Add Intake
              </Button>
            </div>
          ) : (
            items.map(item => {
              const isExpanded = expanded === item.id;
              const isProcessing = processing.has(item.id) || item.status === 'processing';
              const StatusIcon = STATUS_ICON[item.status] ?? Clock;
              const participants = parseJson<string[]>(item.extracted_participants, []);
              const topics = parseJson<string[]>(item.extracted_topics, []);
              const painPoints = parseJson<string[]>(item.extracted_pain_points, []);
              const actions = parseJson<string[]>(item.extracted_action_items, []);

              return (
                <div key={item.id}
                  className="bg-gray-900 border border-gray-800 rounded-xl overflow-hidden">
                  <div
                    className="flex items-center gap-3 p-4 cursor-pointer hover:bg-gray-800/50 transition-colors"
                    onClick={() => setExpanded(isExpanded ? null : item.id)}>
                    <div className="flex-shrink-0">
                      {isExpanded ? <ChevronDown className="w-4 h-4 text-gray-400" /> : <ChevronRight className="w-4 h-4 text-gray-400" />}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-0.5">
                        <span className="text-sm font-medium text-white truncate">
                          {item.subject ?? item.from_name ?? `Intake #${item.id.slice(0, 8)}`}
                        </span>
                        <Badge variant="outline" className={`text-xs shrink-0 ${STATUS_COLOR[item.status] ?? ''}`}>
                          <StatusIcon className={`w-3 h-3 mr-1 ${isProcessing ? 'animate-spin' : ''}`} />
                          {item.status}
                        </Badge>
                        <Badge variant="outline" className="text-xs shrink-0 text-gray-400 border-gray-700">
                          {item.source_type}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-3 text-xs text-gray-500">
                        {item.from_name && <span className="flex items-center gap-1"><User className="w-3 h-3" />{item.from_name}</span>}
                        {item.from_email && <span>{item.from_email}</span>}
                        <span>{formatDateTime(item.created_at)}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-2 flex-shrink-0" onClick={e => e.stopPropagation()}>
                      {item.person_id && (
                        <Button size="sm" variant="ghost" className="text-gray-400 hover:text-white h-7 px-2 text-xs gap-1"
                          onClick={() => navigate(`/people/${item.person_id}`)}>
                          <User className="w-3 h-3" /> Profile
                        </Button>
                      )}
                      {item.company_id && (
                        <Button size="sm" variant="ghost" className="text-indigo-400 hover:text-indigo-300 h-7 px-2 text-xs gap-1"
                          onClick={() => navigate(`/companies/${item.company_id}?tab=intelligence`)}>
                          <Brain className="w-3 h-3" /> Co. Intel
                        </Button>
                      )}
                      {item.report_id && (
                        <Button size="sm" variant="ghost" className="text-gray-400 hover:text-white h-7 px-2 text-xs gap-1"
                          onClick={() => navigate(`/business-reports/${item.report_id}`)}>
                          <FileText className="w-3 h-3" /> Report
                        </Button>
                      )}
                      {item.person_id && !item.crm_deal_id && item.status === 'processed' && (
                        <Button size="sm" className="bg-green-600 hover:bg-green-700 text-white h-7 px-3 text-xs gap-1"
                          onClick={async () => {
                            try {
                              const dealName = [item.from_name, item.company_name].filter(Boolean).join(' — ') || 'New Deal';
                              const res = await fetch('/api/crm/deals', {
                                method: 'POST',
                                headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${localStorage.getItem('session_id') ?? ''}` },
                                credentials: 'include',
                                body: JSON.stringify({
                                  organization_id: user?.home_organization_id ?? (user as any)?.organizations?.[0]?.id,
                                  crm_contact_id: null,
                                  name: dealName,
                                  description: item.call_summary ?? item.subject ?? '',
                                }),
                              });
                              if (res.ok) {
                                toast.success('Deal created — check the Pipeline');
                              } else {
                                toast.error('Failed to create deal');
                              }
                            } catch { toast.error('Failed to create deal'); }
                          }}>
                          <Briefcase className="w-3 h-3" /> Create Deal
                        </Button>
                      )}
                      {item.status === 'pending' && (
                        <Button size="sm" disabled={isProcessing}
                          className="bg-blue-600 hover:bg-blue-700 text-white h-7 px-3 text-xs gap-1"
                          onClick={() => triggerProcess(item.id)}>
                          {isProcessing ? <Loader2 className="w-3 h-3 animate-spin" /> : <Play className="w-3 h-3" />}
                          Process
                        </Button>
                      )}
                    </div>
                  </div>

                  {isExpanded && (
                    <div className="border-t border-gray-800 p-4 space-y-4">
                      {item.call_summary && (
                        <div>
                          <h4 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Summary</h4>
                          <p className="text-sm text-gray-300 leading-relaxed">{item.call_summary}</p>
                        </div>
                      )}

                      <div className="grid grid-cols-2 gap-4">
                        {participants.length > 0 && (
                          <div>
                            <h4 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2 flex items-center gap-1">
                              <User className="w-3 h-3" /> Participants
                            </h4>
                            <ul className="space-y-1">
                              {participants.map((p, i) => (
                                <li key={i} className="text-sm text-gray-300 flex items-start gap-2">
                                  <span className="text-gray-600 mt-0.5">•</span>{p}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}
                        {topics.length > 0 && (
                          <div>
                            <h4 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Topics</h4>
                            <ul className="space-y-1">
                              {topics.map((t, i) => (
                                <li key={i} className="text-sm text-gray-300 flex items-start gap-2">
                                  <span className="text-gray-600 mt-0.5">•</span>{t}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}
                        {painPoints.length > 0 && (
                          <div>
                            <h4 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Pain Points</h4>
                            <ul className="space-y-1">
                              {painPoints.map((p, i) => (
                                <li key={i} className="text-sm text-red-300 flex items-start gap-2">
                                  <span className="text-red-600 mt-0.5">•</span>{p}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}
                        {actions.length > 0 && (
                          <div>
                            <h4 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Action Items</h4>
                            <ul className="space-y-1">
                              {actions.map((a, i) => (
                                <li key={i} className="text-sm text-blue-300 flex items-start gap-2">
                                  <span className="text-blue-600 mt-0.5">•</span>{a}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}
                      </div>

                      {item.extracted_sentiment && (
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-gray-500">Sentiment:</span>
                          <Badge variant="outline" className="text-xs">
                            {item.extracted_sentiment}
                          </Badge>
                        </div>
                      )}

                      {item.error && (
                        <div className="bg-red-500/10 border border-red-500/20 rounded-lg p-3">
                          <p className="text-xs text-red-400 font-mono">{item.error}</p>
                        </div>
                      )}

                      {item.status === 'processed' && !item.report_id && item.person_id && (
                        <Button size="sm" variant="outline"
                          className="border-gray-700 text-gray-300 hover:bg-gray-800 gap-2 text-xs"
                          onClick={async () => {
                            await reportsApi.generate(item.person_id!);
                            fetchItems();
                          }}>
                          <FileText className="w-3 h-3" /> Generate Business Report
                        </Button>
                      )}
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
