import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { reportsApi, crmDealsApi, intelligenceApi } from '@/lib/api';
import { toast } from 'sonner';

/**
 * Consolidates all deal-panel mutations (advance stage, trigger research,
 * approve / revise report) so tab components stay presentation-only.
 */
export function useDealActions() {
  const queryClient = useQueryClient();
  const [advanceLoading, setAdvanceLoading] = useState(false);
  const [researchLoading, setResearchLoading] = useState(false);
  const [reportLoading, setReportLoading] = useState(false);

  const advanceDeal = async (dealId: string, dealName: string) => {
    const confirmed = window.confirm(
      `Advance "${dealName}" to the next pipeline stage? This action cannot be undone.`
    );
    if (!confirmed) return;

    setAdvanceLoading(true);
    try {
      await crmDealsApi.advanceDeal(dealId);
      toast.success('Deal advanced to next stage');
      queryClient.invalidateQueries({ queryKey: ['kanban'] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Failed to advance deal');
    } finally {
      setAdvanceLoading(false);
    }
  };

  const triggerResearch = async (personId: string) => {
    setResearchLoading(true);
    try {
      await intelligenceApi.triggerResearch(personId);
      toast.success('Research triggered — Nora is gathering intel');
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Failed to trigger research');
    } finally {
      setResearchLoading(false);
    }
  };

  const approveReport = async (reportId: string) => {
    setReportLoading(true);
    try {
      await reportsApi.approve(reportId);
      toast.success('Report approved');
    } catch {
      toast.error('Failed to approve report');
    } finally {
      setReportLoading(false);
    }
  };

  const requestRevision = async (reportId: string, notes: string) => {
    setReportLoading(true);
    try {
      await reportsApi.requestRevision(reportId, notes);
      toast.success('Revision requested');
      return true; // signals success so caller can clear input
    } catch {
      toast.error('Failed to request revision');
      return false;
    } finally {
      setReportLoading(false);
    }
  };

  return {
    advanceDeal,
    advanceLoading,
    triggerResearch,
    researchLoading,
    approveReport,
    requestRevision,
    reportLoading,
  } as const;
}
