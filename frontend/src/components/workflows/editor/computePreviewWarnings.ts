import type { PreviewNodeResult } from '@/lib/api';

/**
 * Compute which nodes have validation issues or zero records in preview results.
 */
export function computePreviewWarnings(previewResults: PreviewNodeResult[] | null): Map<string, string> {
  const warnings = new Map<string, string>();
  if (!previewResults) return warnings;
  for (const r of previewResults) {
    try {
      const parsed = JSON.parse(r.output);
      if (Array.isArray(parsed)) {
        const withErrors = parsed.filter((rec: Record<string, unknown>) =>
          rec.validation_errors && Array.isArray(rec.validation_errors) && rec.validation_errors.length > 0
        );
        if (withErrors.length > 0) {
          warnings.set(r.node_id, `${withErrors.length} record${withErrors.length !== 1 ? 's' : ''} with validation issues`);
        } else if (parsed.length === 0 && (r.node_type === 'llm_extract' || r.node_type === 'output')) {
          warnings.set(r.node_id, 'No records extracted');
        }
      } else if (parsed && typeof parsed === 'object') {
        if (parsed.validation_errors && Array.isArray(parsed.validation_errors) && parsed.validation_errors.length > 0) {
          warnings.set(r.node_id, `${parsed.validation_errors.length} validation issue${parsed.validation_errors.length !== 1 ? 's' : ''}`);
        }
        if (parsed.records && Array.isArray(parsed.records)) {
          const withErrors = parsed.records.filter((rec: Record<string, unknown>) =>
            rec.validation_errors && Array.isArray(rec.validation_errors) && rec.validation_errors.length > 0
          );
          if (withErrors.length > 0) {
            warnings.set(r.node_id, `${withErrors.length} record${withErrors.length !== 1 ? 's' : ''} with validation issues`);
          } else if (parsed.records.length === 0 && (r.node_type === 'llm_extract' || r.node_type === 'output')) {
            warnings.set(r.node_id, 'No records extracted');
          }
        }
      }
    } catch {
      // Not JSON, skip
    }
  }
  return warnings;
}
