import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { reportsApi } from '@/lib/api';
import { businessKeys } from '@/lib/query-keys';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  ClipboardList, ChevronRight, RefreshCw, Loader2,
} from 'lucide-react';

import { parseJson, fmtDate } from './report-helpers';

// Re-export ReportDetail for lazy route in App.tsx
export { ReportDetail } from './ReportDetail';

export default function BusinessReportsPage() {
  const { data: reports = [], isLoading, refetch, isFetching } = useQuery({
    queryKey: businessKeys.reports(),
    queryFn: reportsApi.list,
    refetchInterval: 30000,
  });

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center">
            <ClipboardList className="w-5 h-5 text-indigo-400" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-white">Business Analytics</h1>
            <p className="text-xs text-slate-500">{reports.length} report{reports.length !== 1 ? 's' : ''}</p>
          </div>
        </div>
        <Button variant="ghost" size="sm" onClick={() => refetch()} disabled={isFetching}
          className="gap-2 text-slate-400">
          <RefreshCw className={`w-4 h-4 ${isFetching ? 'animate-spin' : ''}`} />
          Refresh
        </Button>
      </div>

      {/* List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
        </div>
      ) : reports.length === 0 ? (
        <div className="text-center py-20 text-slate-500">
          <ClipboardList className="w-10 h-10 mx-auto mb-3 opacity-30" />
          <p>No analytics reports yet. Process call transcripts via Call Intake to generate reports.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {reports.map((r) => (
            <Link key={r.id} to={`/business-reports/${r.id}`} className="block group">
              <div className="rounded-xl border border-slate-800 bg-slate-900/50 hover:border-slate-700 hover:bg-slate-900 transition-all p-5">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <p className="font-semibold text-slate-200 group-hover:text-white transition-colors truncate">
                        {r.title}
                      </p>
                      <Badge variant="outline" className={`shrink-0 text-xs ${
                        r.status === 'ready' ? 'border-emerald-700 text-emerald-400' :
                        r.status === 'draft' ? 'border-amber-700 text-amber-400' :
                        'border-slate-700 text-slate-500'
                      }`}>
                        {r.status}
                      </Badge>
                      {r.review_status && r.review_status !== 'pending_review' && (
                        <Badge variant="outline" className={`shrink-0 text-xs ${
                          r.review_status === 'approved' ? 'border-emerald-700 text-emerald-400' :
                          r.review_status === 'rejected' ? 'border-amber-700 text-amber-400' :
                          'border-slate-700 text-slate-500'
                        }`}>
                          {r.review_status === 'approved' ? 'approved' : 'revision needed'}
                        </Badge>
                      )}
                      {r.review_status === 'pending_review' && (
                        <Badge variant="outline" className="shrink-0 text-xs border-yellow-700 text-yellow-400">
                          awaiting review
                        </Badge>
                      )}
                    </div>
                    {r.executive_summary && (
                      <p className="text-sm text-slate-500 leading-relaxed line-clamp-2">
                        {r.executive_summary}
                      </p>
                    )}
                    <div className="flex items-center gap-3 mt-2 text-xs text-slate-600">
                      <span>{fmtDate(r.created_at)}</span>
                      {(() => {
                        const pp = parseJson<unknown[]>(r.pain_points, []).length;
                        const op = parseJson<unknown[]>(r.opportunities, []).length;
                        const src = parseJson<unknown[]>(r.sources, []).length;
                        return (
                          <>
                            {pp > 0 && <span className="text-red-600">{pp} pain points</span>}
                            {op > 0 && <span className="text-emerald-600">{op} opportunities</span>}
                            {src > 0 && <span className="text-indigo-600">{src} sources</span>}
                          </>
                        );
                      })()}
                    </div>
                  </div>
                  <ChevronRight className="w-4 h-4 text-slate-600 group-hover:text-slate-400 transition-colors shrink-0 mt-1" />
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
