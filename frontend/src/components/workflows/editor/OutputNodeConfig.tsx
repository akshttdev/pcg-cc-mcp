import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Users,
  Building2,
  Handshake,
  ListTodo,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { crmPipelinesApi } from '@/lib/api';
import type { WorkflowNode } from '@/lib/api';
import type { CrmPipeline, CrmPipelineStage, CrmPipelineWithStages } from '@/types/crm';
import { TARGET_SCHEMAS } from './node-types';

interface OutputNodeConfigProps {
  node: WorkflowNode;
  onUpdateParameter: (key: string, value: string | undefined) => void;
}

export function OutputNodeConfig({ node, onUpdateParameter }: OutputNodeConfigProps) {
  const targetType = node.parameters.target_type as string;
  const schema = TARGET_SCHEMAS[targetType];
  const [schemaExpanded, setSchemaExpanded] = useState(false);

  // Fetch pipelines for CRM Deals output
  const { data: pipelines = [] } = useQuery({
    queryKey: ['crmPipelines'],
    queryFn: async () => {
      try {
        return await crmPipelinesApi.listOrgPipelines('01010101-0101-0101-0101-010101010101');
      } catch {
        return [];
      }
    },
    enabled: node.type === 'output_crm_deals',
    staleTime: 5 * 60 * 1000,
  });

  // Fetch stages for selected pipeline
  const selectedPipelineId = node.parameters.pipeline_id as string | undefined;
  const { data: pipelineWithStages } = useQuery({
    queryKey: ['crmPipelineStages', selectedPipelineId],
    queryFn: () => crmPipelinesApi.getPipeline(selectedPipelineId!),
    enabled: !!selectedPipelineId && node.type === 'output_crm_deals',
    staleTime: 5 * 60 * 1000,
  });
  const stages: CrmPipelineStage[] = (pipelineWithStages as CrmPipelineWithStages | undefined)?.stages ?? [];

  return (
    <div className="space-y-3">
      <div>
        <Label className="text-xs">Target</Label>
        <div className="mt-1 text-sm text-muted-foreground bg-muted/50 rounded px-2 py-1.5 flex items-center gap-2">
          {node.type === 'output_crm_contacts' && <><Users className="h-3.5 w-3.5 text-blue-500" /> CRM Contacts</>}
          {node.type === 'output_crm_companies' && <><Building2 className="h-3.5 w-3.5 text-purple-500" /> Companies</>}
          {node.type === 'output_crm_deals' && <><Handshake className="h-3.5 w-3.5 text-green-500" /> CRM Deals</>}
          {node.type === 'output_tasks' && <><ListTodo className="h-3.5 w-3.5 text-orange-500" /> Tasks</>}
        </div>
      </div>

      {/* Pipeline & Stage selector for CRM Deals */}
      {node.type === 'output_crm_deals' && (
        <>
          <div>
            <Label className="text-xs">Pipeline</Label>
            <Select
              value={selectedPipelineId || '__none__'}
              onValueChange={(v) => {
                onUpdateParameter('pipeline_id', v === '__none__' ? undefined : v);
                onUpdateParameter('stage_id', undefined);
              }}
            >
              <SelectTrigger className="h-8 text-sm mt-1">
                <SelectValue placeholder="Select pipeline..." />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__none__">Auto-assign</SelectItem>
                {pipelines.map((p: CrmPipeline) => (
                  <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-[10px] text-muted-foreground mt-1">
              Which CRM pipeline to create deals in.
            </p>
          </div>
          {stages.length > 0 && (
            <div>
              <Label className="text-xs">Initial Stage</Label>
              <Select
                value={(node.parameters.stage_id as string) || '__first__'}
                onValueChange={(v) => onUpdateParameter('stage_id', v === '__first__' ? undefined : v)}
              >
                <SelectTrigger className="h-8 text-sm mt-1">
                  <SelectValue placeholder="First stage" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__first__">First stage ({stages[0]?.name})</SelectItem>
                  {stages.map((s: CrmPipelineStage) => (
                    <SelectItem key={s.id} value={s.id}>{s.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-[10px] text-muted-foreground mt-1">
                Stage new deals start in.
              </p>
            </div>
          )}
        </>
      )}

      <div>
        <Label className="text-xs">On Duplicate</Label>
        <Select
          value={node.parameters.on_duplicate || 'flag_for_review'}
          onValueChange={(v) => onUpdateParameter('on_duplicate', v)}
        >
          <SelectTrigger className="h-8 text-sm mt-1">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="flag_for_review">Flag for review</SelectItem>
            <SelectItem value="skip">Skip duplicates</SelectItem>
            <SelectItem value="update_existing">Update existing</SelectItem>
            <SelectItem value="create_anyway">Create anyway</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-[10px] text-muted-foreground mt-1">
          What to do when a matching record already exists.
        </p>
      </div>

      {/* Schema preview */}
      {schema && (
        <div className="rounded-md border border-muted-foreground/20 overflow-hidden">
          <button
            onClick={() => setSchemaExpanded(!schemaExpanded)}
            className="w-full flex items-center gap-2 px-2.5 py-1.5 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
          >
            {schemaExpanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
            ) : (
              <ChevronRight className="h-3 w-3 text-muted-foreground shrink-0" />
            )}
            <span className="text-[11px] font-medium text-muted-foreground">
              {schema.label} Schema ({schema.fields.length} fields)
            </span>
          </button>
          {schemaExpanded && (
            <div className="px-2.5 py-2 space-y-0.5 bg-muted/10">
              {schema.fields.map((f) => (
                <div key={f.name} className="flex items-center gap-2 text-[10px] font-mono">
                  <span className={cn('truncate', f.required ? 'text-foreground font-semibold' : 'text-muted-foreground')}>
                    {f.name}{f.required ? '*' : ''}
                  </span>
                  <span className="text-muted-foreground/60 truncate">{f.type}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="rounded-md border border-dashed border-muted-foreground/30 p-2.5 bg-muted/20">
        <p className="text-[10px] text-muted-foreground">
          Connect an LLM node as input. The schema for the target type will be automatically injected into the upstream LLM prompt.
        </p>
      </div>
    </div>
  );
}
