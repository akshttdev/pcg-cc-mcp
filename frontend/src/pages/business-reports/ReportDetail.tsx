import { useCallback, useEffect } from 'react';
import { useParams, Link, useSearchParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { reportsApi, type BusinessReportRecord } from '@/lib/api';
import { businessKeys } from '@/lib/query-keys';
import { InlineEdit } from '@/components/ui/inline-edit';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  ArrowLeft, User, Building2, Globe, ExternalLink,
  TrendingUp, Users, Target, Zap, MapPin, AlertCircle, CheckCircle,
  Star, ChevronRight, Printer, BookOpen, Loader2, Download,
} from 'lucide-react';

import { parseJson, fmtDate, severityColor, priorityDot, threatBadge, SectionHeader, IntelCard } from './report-helpers';
import { IntelSourcePanels } from './IntelSourcePanels';
import { ReviewBanner } from './ReviewBanner';

export function ReportDetail() {
  const { id } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();

  // Auto-trigger print when ?print=true (e.g. from PDF redirect)
  useEffect(() => {
    if (searchParams.get('print') === 'true') {
      const timer = setTimeout(() => window.print(), 800);
      return () => clearTimeout(timer);
    }
  }, [searchParams]);

  const { data: report, isLoading } = useQuery({
    queryKey: businessKeys.report(id!),
    queryFn: () => reportsApi.get(id!),
    enabled: !!id,
    refetchInterval: (q) => {
      const r = q.state.data;
      return r?.status === 'draft' ? 5000 : false;
    },
  });

  const patchMut = useMutation({
    mutationFn: (data: Partial<BusinessReportRecord>) => reportsApi.patch(id!, data),
    onSuccess: (updated) => {
      queryClient.setQueryData(businessKeys.report(id!), updated);
    },
  });

  const approveMut = useMutationWithToast({
    mutationFn: () => reportsApi.approve(id!),
    successMessage: 'Report approved and proposal generated',
    errorMessage: 'Failed to approve report',
    invalidateKeys: [businessKeys.reports()],
    onSuccess: (result) => {
      queryClient.setQueryData(businessKeys.report(id!), result.report);
    },
  });

  const revisionMut = useMutationWithToast({
    mutationFn: (notes: string) => reportsApi.requestRevision(id!, notes),
    successMessage: 'Revision requested',
    errorMessage: 'Failed to request revision',
    invalidateKeys: [businessKeys.reports()],
    onSuccess: (updated) => {
      queryClient.setQueryData(businessKeys.report(id!), updated);
    },
  });

  const save = useCallback((field: string, value: string) => {
    patchMut.mutate({ [field]: value } as Partial<BusinessReportRecord>);
  }, [patchMut]);

  if (isLoading) return (
    <div className="flex items-center justify-center h-64">
      <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
    </div>
  );

  if (!report) return (
    <div className="p-8 text-slate-500">Report not found.</div>
  );

  const painPoints = parseJson<Array<{ point: string; severity: string }>>(report.pain_points, []);
  const opportunities = parseJson<Array<{ title: string; description: string; priority: string; estimated_value: string }>>(report.opportunities, []);
  const recommendedServices = parseJson<Array<{ name: string; rationale: string; timeline: string }>>(report.recommended_services, []);
  const nextSteps = parseJson<Array<{ action: string; owner: string; deadline: string }>>(report.next_steps, []);
  const individuals = parseJson<Array<{ name: string; role: string; company: string; linkedin?: string; summary: string; key_insights: string }>>(report.individual_profiles, []);
  const competitors = parseJson<Array<{ name: string; website?: string; strengths: string; weaknesses: string; threat_level: string }>>(report.competitor_analysis, []);
  const sources = parseJson<Array<{ title: string; url: string; excerpt?: string }>>(report.sources, []);

  return (
    <div className="max-w-5xl mx-auto px-4 py-8 space-y-8 print:px-0 print:py-0">
      {/* Header bar */}
      <div className="flex items-start justify-between gap-4 print:hidden">
        <div className="flex items-center gap-3">
          <Link to="/business-reports">
            <Button variant="ghost" size="sm" className="gap-2 text-slate-400">
              <ArrowLeft className="w-4 h-4" /> Reports
            </Button>
          </Link>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className={
            report.status === 'ready' ? 'border-emerald-700 text-emerald-400' :
            report.status === 'draft' ? 'border-amber-700 text-amber-400' :
            'border-slate-700 text-slate-400'
          }>
            {report.status}
          </Badge>
          <Button variant="ghost" size="sm" className="gap-2 text-slate-400" onClick={() => window.print()}>
            <Printer className="w-4 h-4" /> Print
          </Button>
          <a href={`/api/business-reports/${id}/pdf`} target="_blank" rel="noopener noreferrer">
            <Button variant="outline" size="sm" className="gap-2 border-emerald-700/60 text-emerald-400 hover:bg-emerald-950/40">
              <Download className="w-4 h-4" /> Download PDF
            </Button>
          </a>
        </div>
      </div>

      {/* Review Banner */}
      {report.review_status && (
        <ReviewBanner
          report={report}
          onApprove={() => approveMut.mutate()}
          onRequestRevision={(notes) => revisionMut.mutate(notes)}
          isLoading={approveMut.isPending || revisionMut.isPending}
        />
      )}

      {/* Title */}
      <div className="print:mt-8">
        <p className="text-xs font-medium uppercase tracking-[0.2em] text-indigo-400 mb-2">
          Business Analytics Report · {fmtDate(report.created_at)}
        </p>
        <InlineEdit
          value={report.title}
          onSave={(v) => save('title', v)}
          className="text-3xl font-semibold text-white tracking-tight"
          placeholder="Report Title"
        />
        <div className="mt-3 h-px bg-gradient-to-r from-indigo-500/40 to-transparent" />
      </div>

      {/* Intelligence Source Panels */}
      <IntelSourcePanels personId={report.person_id} companyId={report.company_id} />

      {/* Executive Summary */}
      <IntelCard>
        <SectionHeader icon={BookOpen} title="Executive Summary" />
        <InlineEdit
          value={report.executive_summary ?? ''}
          onSave={(v) => save('executive_summary', v)}
          multiline
          placeholder="Click to add executive summary..."
          className="text-slate-200 text-base leading-relaxed"
          inputClassName="min-h-[120px]"
        />
      </IntelCard>

      {/* Individual Profiles */}
      {individuals.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Users} title="Individual Profiles" />
          <div className="space-y-4">
            {individuals.map((ind, i) => (
              <div key={i} className="flex gap-4 p-4 rounded-xl bg-slate-800/50 border border-slate-700/50">
                <div className="w-10 h-10 rounded-full bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0">
                  <User className="w-5 h-5 text-indigo-400" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <p className="font-semibold text-white">{ind.name}</p>
                    {ind.linkedin && (
                      <a href={ind.linkedin} target="_blank" rel="noopener noreferrer"
                        className="text-indigo-400 hover:text-indigo-300">
                        <ExternalLink className="w-3.5 h-3.5" />
                      </a>
                    )}
                  </div>
                  <p className="text-xs text-slate-400 mb-2">
                    {ind.role && <span>{ind.role}</span>}
                    {ind.role && ind.company && <span className="mx-1.5">·</span>}
                    {ind.company && <span>{ind.company}</span>}
                  </p>
                  {ind.summary && <p className="text-sm text-slate-300 mb-1">{ind.summary}</p>}
                  {ind.key_insights && (
                    <p className="text-xs text-indigo-300 bg-indigo-950/40 border border-indigo-900/60 rounded-lg px-3 py-2 mt-2">
                      {ind.key_insights}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Company Overview + Market Analysis row */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <IntelCard>
          <SectionHeader icon={Building2} title="Company Overview" />
          <InlineEdit
            value={report.company_overview ?? ''}
            onSave={(v) => save('company_overview', v)}
            multiline
            placeholder="Company overview..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
        <IntelCard>
          <SectionHeader icon={TrendingUp} title="Market Analysis" />
          <InlineEdit
            value={report.market_analysis ?? ''}
            onSave={(v) => save('market_analysis', v)}
            multiline
            placeholder="Market position, size, trends..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
      </div>

      {/* Brand Positioning + Target Clients row */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <IntelCard>
          <SectionHeader icon={Target} title="Brand Positioning" />
          <InlineEdit
            value={report.brand_positioning ?? ''}
            onSave={(v) => save('brand_positioning', v)}
            multiline
            placeholder="How they position themselves..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
        <IntelCard>
          <SectionHeader icon={Users} title="Target Clients" />
          <InlineEdit
            value={report.target_clients ?? ''}
            onSave={(v) => save('target_clients', v)}
            multiline
            placeholder="Their ideal customer profile..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
      </div>

      {/* Digital Presence */}
      <IntelCard>
        <SectionHeader icon={Globe} title="Digital Presence" />
        <InlineEdit
          value={report.digital_presence ?? ''}
          onSave={(v) => save('digital_presence', v)}
          multiline
          placeholder="Web, social media, SEO, content presence..."
          className="text-slate-300 text-sm leading-relaxed"
          inputClassName="min-h-[80px]"
        />
      </IntelCard>

      {/* Competitive Landscape */}
      {competitors.length > 0 && (
        <IntelCard>
          <SectionHeader icon={MapPin} title="Competitive Landscape" />
          <div className="space-y-3">
            {competitors.map((c, i) => (
              <div key={i} className="flex items-start gap-4 p-4 rounded-xl bg-slate-800/40 border border-slate-700/40">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <p className="font-semibold text-slate-200">{c.name}</p>
                    <span className={`text-xs px-2 py-0.5 rounded-full border font-medium ${threatBadge(c.threat_level)}`}>
                      {c.threat_level} threat
                    </span>
                    {c.website && (
                      <a href={c.website} target="_blank" rel="noopener noreferrer"
                        className="text-slate-500 hover:text-slate-400">
                        <Globe className="w-3 h-3" />
                      </a>
                    )}
                  </div>
                  <div className="grid grid-cols-2 gap-3 mt-2">
                    <div>
                      <p className="text-xs uppercase tracking-wider text-emerald-500 mb-1">Strengths</p>
                      <p className="text-xs text-slate-400">{c.strengths}</p>
                    </div>
                    <div>
                      <p className="text-xs uppercase tracking-wider text-red-500 mb-1">Weaknesses</p>
                      <p className="text-xs text-slate-400">{c.weaknesses}</p>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Pain Points */}
      {painPoints.length > 0 && (
        <IntelCard>
          <SectionHeader icon={AlertCircle} title="Pain Points & Challenges" />
          <div className="space-y-2">
            {painPoints.map((p, i) => (
              <div key={i} className={`flex items-start gap-3 px-4 py-3 rounded-lg border text-sm ${severityColor(p.severity)}`}>
                <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
                <p>{p.point}</p>
                <span className="ml-auto text-xs uppercase tracking-wider font-medium opacity-70">{p.severity}</span>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Opportunities */}
      {opportunities.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Zap} title="Opportunities for PCG" />
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {opportunities.map((o, i) => (
              <div key={i} className="p-4 rounded-xl border border-slate-700/60 bg-slate-800/30">
                <div className="flex items-center gap-2 mb-2">
                  <div className={`w-2 h-2 rounded-full ${priorityDot(o.priority)}`} />
                  <p className="font-semibold text-slate-200 text-sm">{o.title}</p>
                </div>
                <p className="text-xs text-slate-400 leading-relaxed mb-3">{o.description}</p>
                {o.estimated_value && (
                  <span className="text-xs px-2 py-1 rounded-full border border-emerald-800 text-emerald-400 bg-emerald-950/30 font-medium">
                    {o.estimated_value}
                  </span>
                )}
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Recommended Services */}
      {recommendedServices.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Star} title="Recommended Services" />
          <div className="space-y-3">
            {recommendedServices.map((s, i) => (
              <div key={i} className="flex items-start gap-4">
                <div className="w-6 h-6 rounded-full bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-semibold text-indigo-400">{i + 1}</span>
                </div>
                <div>
                  <p className="font-semibold text-slate-200 text-sm">{s.name}</p>
                  <p className="text-xs text-slate-400 mt-0.5">{s.rationale}</p>
                  {s.timeline && (
                    <p className="text-xs text-slate-500 mt-1">Timeline: {s.timeline}</p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Next Steps */}
      {nextSteps.length > 0 && (
        <IntelCard>
          <SectionHeader icon={ChevronRight} title="Next Steps" />
          <div className="space-y-3">
            {nextSteps.map((step, i) => (
              <div key={i} className="flex items-start gap-3 p-3 rounded-lg bg-slate-800/40 border border-slate-700/40">
                <CheckCircle className="w-4 h-4 text-emerald-500 mt-0.5 shrink-0" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm text-slate-200">{step.action}</p>
                  <div className="flex items-center gap-2 mt-1 text-xs text-slate-500">
                    {step.owner && <span>Owner: {step.owner}</span>}
                    {step.owner && step.deadline && <span>·</span>}
                    {step.deadline && <span>By: {step.deadline}</span>}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Sources & References */}
      {sources.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Globe} title="Sources & References" />
          <div className="space-y-2">
            {sources.map((src, i) => (
              <div key={i} className="flex items-start gap-3 py-2 border-b border-slate-800/60 last:border-0">
                <span className="text-xs text-slate-600 w-5 pt-0.5 shrink-0">{i + 1}.</span>
                <div className="min-w-0 flex-1">
                  <a
                    href={src.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm text-indigo-400 hover:text-indigo-300 flex items-center gap-1 group"
                  >
                    {src.title || src.url}
                    <ExternalLink className="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                  </a>
                  {src.excerpt && (
                    <p className="text-xs text-slate-500 mt-0.5 italic">{src.excerpt}</p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Full Report Markdown */}
      {report.full_report_md && (
        <IntelCard className="print:block">
          <SectionHeader icon={BookOpen} title="Full Narrative Report" />
          <div className="prose prose-invert prose-sm max-w-none prose-headings:text-slate-200 prose-p:text-slate-400 prose-li:text-slate-400 prose-strong:text-slate-200">
            <pre className="whitespace-pre-wrap text-sm text-slate-400 font-sans leading-relaxed">
              {report.full_report_md}
            </pre>
          </div>
        </IntelCard>
      )}
    </div>
  );
}
