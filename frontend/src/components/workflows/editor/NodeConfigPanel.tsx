import { useState } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  X,
  Zap,
  Link,
  Unlink,
  FileText,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  WorkflowNode,
  WorkflowConnection,
  AvailableModel,
} from '@/lib/api';
import { NODE_TYPES, getNodeTypeDef } from './node-types';
import { OutputNodeConfig } from './OutputNodeConfig';
import { DataSourceNodeConfig } from './DataSourceNodeConfig';

const PROMPT_TEMPLATES = [
  {
    label: 'Contact Extraction',
    prompt: 'Extract all people mentioned. For each person, provide: first_name, last_name, email, phone, company_name, job_title.',
  },
  {
    label: 'Company Research',
    prompt: 'Extract all companies mentioned. For each company, provide: name, website, industry, description, employee_count.',
  },
  {
    label: 'Deal Pipeline',
    prompt: 'Extract all potential deals or opportunities. For each, provide: deal_name, contact_name, company_name, estimated_value, stage, next_steps.',
  },
] as const;

const KNOWN_OUTPUT_SCHEMAS = [
  { value: 'contacts[]', label: 'Contacts' },
  { value: 'companies[]', label: 'Companies' },
  { value: 'crm_deals[]', label: 'Deals' },
  { value: 'tasks[]', label: 'Tasks' },
];

function OutputSchemaField({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const isKnown = KNOWN_OUTPUT_SCHEMAS.some(s => s.value === value);
  const [showCustom, setShowCustom] = useState(!isKnown && value !== '');

  return (
    <div>
      <Label className="text-xs">Output Schema</Label>
      <Select
        value={showCustom ? '__custom__' : (value || '__none__')}
        onValueChange={(v) => {
          if (v === '__custom__') {
            setShowCustom(true);
          } else if (v === '__none__') {
            setShowCustom(false);
            onChange('');
          } else {
            setShowCustom(false);
            onChange(v);
          }
        }}
      >
        <SelectTrigger className="h-8 text-sm mt-1">
          <SelectValue placeholder="Select output schema..." />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="__none__">None</SelectItem>
          {KNOWN_OUTPUT_SCHEMAS.map((s) => (
            <SelectItem key={s.value} value={s.value}>{s.label} ({s.value})</SelectItem>
          ))}
          <SelectItem value="__custom__">Custom...</SelectItem>
        </SelectContent>
      </Select>
      {showCustom && (
        <Input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="e.g. opportunities[], custom_type[]"
          className="h-8 text-sm mt-1.5"
          autoFocus
        />
      )}
    </div>
  );
}

interface NodeConfigPanelProps {
  node: WorkflowNode;
  allNodes: WorkflowNode[];
  connections: WorkflowConnection[];
  availableModels: AvailableModel[];
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onUpdateParameter: (key: string, value: unknown) => void;
  onAddConnection: (sourceId: string) => void;
  onRemoveConnection: (sourceId: string) => void;
  onClose: () => void;
}

export function NodeConfigPanel({
  node,
  allNodes,
  connections,
  availableModels,
  onUpdate,
  onUpdateParameter,
  onAddConnection,
  onRemoveConnection,
  onClose,
}: NodeConfigPanelProps) {
  const typeDef = getNodeTypeDef(node.type);
  const Icon = typeDef?.icon ?? Zap;

  // Current inputs (nodes connected TO this node)
  const currentInputs = connections
    .filter((c) => c.target === node.id)
    .map((c) => c.source);

  // Available nodes to connect from (exclude self, already-connected, and output nodes)
  const availableInputs = allNodes.filter(
    (n) => n.id !== node.id && !currentInputs.includes(n.id) && !n.type.startsWith('output_')
  );

  const isLLMNode = ['llm_extract', 'llm_analyze', 'llm_summarize'].includes(
    node.type
  );

  return (
    <div className="flex flex-col h-full">
      {/* Panel header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b">
        <div
          className={cn(
            'w-8 h-8 rounded-lg flex items-center justify-center text-white shrink-0',
            typeDef?.color ?? 'bg-gray-500'
          )}
        >
          <Icon className="h-4 w-4" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="font-medium text-sm">{typeDef?.label}</div>
          <div className="text-xs text-muted-foreground">{node.id}</div>
        </div>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-muted transition-colors"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-5">
          {/* Node name */}
          <div>
            <Label className="text-xs text-muted-foreground">Node Name</Label>
            <Input
              value={node.name}
              onChange={(e) => onUpdate({ name: e.target.value })}
              className="h-8 text-sm mt-1"
            />
          </div>

          {/* Node type */}
          <div>
            <Label className="text-xs text-muted-foreground">Node Type</Label>
            <Select
              value={node.type}
              onValueChange={(v) => onUpdate({ type: v })}
            >
              <SelectTrigger className="h-8 text-sm mt-1">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {NODE_TYPES.map((nt) => (
                  <SelectItem key={nt.type} value={nt.type}>
                    {nt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Input connections */}
          <div>
            <Label className="text-xs text-muted-foreground mb-2 block">
              Input Connections
            </Label>
            <div className="space-y-1.5">
              {currentInputs.map((sourceId) => {
                const sourceNode = allNodes.find((n) => n.id === sourceId);
                const srcDef = sourceNode
                  ? getNodeTypeDef(sourceNode.type)
                  : undefined;
                const SrcIcon = srcDef?.icon ?? Zap;
                return (
                  <div
                    key={sourceId}
                    className="flex items-center gap-2 rounded-md border bg-primary/5 border-primary/20 px-2 py-1.5"
                  >
                    <div
                      className={cn(
                        'w-5 h-5 rounded flex items-center justify-center text-white shrink-0',
                        srcDef?.color ?? 'bg-gray-500'
                      )}
                    >
                      <SrcIcon className="h-3 w-3" />
                    </div>
                    <span className="text-sm flex-1 truncate">
                      {sourceNode?.name ?? sourceId}
                    </span>
                    <button
                      onClick={() => onRemoveConnection(sourceId)}
                      className="p-0.5 rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
                      title="Disconnect"
                    >
                      <Unlink className="h-3.5 w-3.5" />
                    </button>
                  </div>
                );
              })}

              {currentInputs.length === 0 && (
                <p className="text-xs text-muted-foreground italic">
                  No inputs — receives raw data source content.
                </p>
              )}

              {availableInputs.length > 0 && (
                <Select onValueChange={(v) => onAddConnection(v)}>
                  <SelectTrigger className="h-8 text-sm border-dashed">
                    <div className="flex items-center gap-1.5 text-muted-foreground">
                      <Link className="h-3.5 w-3.5" />
                      <span>Add input connection...</span>
                    </div>
                  </SelectTrigger>
                  <SelectContent>
                    {availableInputs.map((n) => {
                      const nDef = getNodeTypeDef(n.type);
                      return (
                        <SelectItem key={n.id} value={n.id}>
                          <span className="flex items-center gap-2">
                            <span
                              className={cn(
                                'w-4 h-4 rounded flex items-center justify-center text-white shrink-0 text-xs',
                                nDef?.color ?? 'bg-gray-500'
                              )}
                            >
                              {(nDef?.label ?? '?')[0]}
                            </span>
                            {n.name}
                          </span>
                        </SelectItem>
                      );
                    })}
                  </SelectContent>
                </Select>
              )}
            </div>
          </div>

          {/* Divider */}
          <div className="border-t" />

          {/* Type-specific parameters */}
          <div>
            <Label className="text-xs text-muted-foreground mb-2 block">
              Parameters
            </Label>

            {node.type === 'data_source' && (
              <DataSourceNodeConfig
                acceptedTypes={node.parameters.accepted_types || ''}
                onUpdateAcceptedTypes={(v) => onUpdateParameter('accepted_types', v)}
              />
            )}

            {isLLMNode && (
              <div className="space-y-3">
                {/* Model selection */}
                <div>
                  <Label className="text-xs">Model</Label>
                  <Select
                    value={node.parameters.model || '__default__'}
                    onValueChange={(v) => onUpdateParameter('model', v === '__default__' ? undefined : v)}
                  >
                    <SelectTrigger className="h-8 text-sm mt-1">
                      <SelectValue placeholder="Use workflow default" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__default__">Use workflow default</SelectItem>
                      {availableModels.map((m) => (
                        <SelectItem key={m.id} value={m.id}>
                          {m.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground mt-1">
                    Override the workflow's default model for this node.
                  </p>
                </div>
                <div>
                  <div className="flex items-center justify-between">
                    <Label className="text-xs">Prompt Template</Label>
                    <Select onValueChange={(v) => {
                      const tpl = PROMPT_TEMPLATES.find(t => t.label === v);
                      if (tpl) onUpdateParameter('prompt_template', tpl.prompt);
                    }}>
                      <SelectTrigger className="h-6 w-auto text-xs gap-1 border-dashed px-2">
                        <FileText className="h-3 w-3" />
                        <span>Templates</span>
                      </SelectTrigger>
                      <SelectContent>
                        {PROMPT_TEMPLATES.map((tpl) => (
                          <SelectItem key={tpl.label} value={tpl.label}>
                            {tpl.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex flex-wrap gap-1 mt-1.5 mb-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        const ta = document.querySelector<HTMLTextAreaElement>(`[data-prompt-node="${node.id}"]`);
                        if (ta) {
                          const pos = ta.selectionStart ?? ta.value.length;
                          const before = ta.value.slice(0, pos);
                          const after = ta.value.slice(pos);
                          onUpdateParameter('prompt_template', before + '{{content}}' + after);
                        } else {
                          onUpdateParameter('prompt_template', (node.parameters.prompt_template ?? '') + '{{content}}');
                        }
                      }}
                      className="inline-flex items-center gap-1 rounded-md bg-blue-500/10 border border-blue-500/20 px-2 py-0.5 text-xs font-mono text-blue-700 dark:text-blue-300 hover:bg-blue-500/20 transition-colors"
                    >
                      {'{{content}}'}
                      <span className="text-[9px] font-sans text-muted-foreground">raw input</span>
                    </button>
                    {currentInputs.map((sourceId) => {
                      const sourceNode = allNodes.find((n) => n.id === sourceId);
                      const rawSchema = (sourceNode?.parameters?.output_schema as string) ?? '';
                      const schemaName = rawSchema.replace(/\[\]$/, '');
                      if (!schemaName) return null;
                      const varName = `{{${schemaName}}}`;
                      return (
                        <button
                          key={sourceId}
                          type="button"
                          onClick={() => {
                            const ta = document.querySelector<HTMLTextAreaElement>(`[data-prompt-node="${node.id}"]`);
                            if (ta) {
                              const pos = ta.selectionStart ?? ta.value.length;
                              const before = ta.value.slice(0, pos);
                              const after = ta.value.slice(pos);
                              onUpdateParameter('prompt_template', before + varName + after);
                            } else {
                              onUpdateParameter('prompt_template', (node.parameters.prompt_template ?? '') + varName);
                            }
                          }}
                          className="inline-flex items-center gap-1 rounded-md bg-green-500/10 border border-green-500/20 px-2 py-0.5 text-xs font-mono text-green-700 dark:text-green-300 hover:bg-green-500/20 transition-colors"
                        >
                          {varName}
                          <span className="text-[9px] font-sans text-muted-foreground">
                            from {sourceNode?.name ?? sourceId}
                          </span>
                        </button>
                      );
                    })}
                    {currentInputs.length > 0 && (
                      <button
                        type="button"
                        onClick={() => {
                          const ta = document.querySelector<HTMLTextAreaElement>(`[data-prompt-node="${node.id}"]`);
                          if (ta) {
                            const pos = ta.selectionStart ?? ta.value.length;
                            const before = ta.value.slice(0, pos);
                            const after = ta.value.slice(pos);
                            onUpdateParameter('prompt_template', before + '{{previous_results}}' + after);
                          } else {
                            onUpdateParameter('prompt_template', (node.parameters.prompt_template ?? '') + '{{previous_results}}');
                          }
                        }}
                        className="inline-flex items-center gap-1 rounded-md bg-purple-500/10 border border-purple-500/20 px-2 py-0.5 text-xs font-mono text-purple-700 dark:text-purple-300 hover:bg-purple-500/20 transition-colors"
                      >
                        {'{{previous_results}}'}
                        <span className="text-[9px] font-sans text-muted-foreground">
                          all inputs combined
                        </span>
                      </button>
                    )}
                  </div>
                  <Textarea
                    data-prompt-node={node.id}
                    value={node.parameters.prompt_template ?? ''}
                    onChange={(e) =>
                      onUpdateParameter('prompt_template', e.target.value)
                    }
                    placeholder="Describe what to extract (e.g., 'Extract all contacts with name, email, and company'). Use Templates above for starters."
                    className="text-sm font-mono min-h-[200px] resize-y"
                  />
                </div>
                <OutputSchemaField
                  value={node.parameters.output_schema ?? ''}
                  onChange={(v) => onUpdateParameter('output_schema', v)}
                />
                <div>
                  <Label className="text-xs">Output Mode</Label>
                  <div className="flex gap-1 mt-1">
                    {([
                      { value: 'auto', label: 'Auto', desc: 'JSON if schema set' },
                      { value: 'structured', label: 'Structured', desc: 'Always JSON' },
                      { value: 'text', label: 'Text', desc: 'Raw response' },
                    ] as const).map((opt) => (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() => onUpdateParameter('output_mode', opt.value)}
                        className={cn(
                          'flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-all border',
                          (node.parameters.output_mode || 'auto') === opt.value
                            ? 'bg-primary text-primary-foreground border-primary'
                            : 'bg-transparent border-border/60 hover:bg-accent text-muted-foreground'
                        )}
                      >
                        {opt.label}
                      </button>
                    ))}
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    Auto uses JSON when output schema is defined, text otherwise.
                  </p>
                </div>
              </div>
            )}

            {node.type === 'transform' && (
              <div>
                <Label className="text-xs">Transform Expression</Label>
                <Textarea
                  value={node.parameters.transform_expression ?? ''}
                  onChange={(e) =>
                    onUpdateParameter('transform_expression', e.target.value)
                  }
                  placeholder="Describe the transformation to apply..."
                  className="mt-1 text-sm font-mono min-h-[120px]"
                />
              </div>
            )}

            {node.type === 'filter' && (
              <div>
                <Label className="text-xs">Filter Condition</Label>
                <Textarea
                  value={node.parameters.condition ?? ''}
                  onChange={(e) =>
                    onUpdateParameter('condition', e.target.value)
                  }
                  placeholder="Describe the filter condition..."
                  className="mt-1 text-sm font-mono min-h-[120px]"
                />
              </div>
            )}

            {node.type === 'merge' && (
              <div>
                <Label className="text-xs">Merge Strategy</Label>
                <Select
                  value={node.parameters.merge_strategy ?? 'combine'}
                  onValueChange={(v) =>
                    onUpdateParameter('merge_strategy', v)
                  }
                >
                  <SelectTrigger className="h-8 text-sm mt-1">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="combine">
                      Combine (merge all inputs)
                    </SelectItem>
                    <SelectItem value="append">
                      Append (concatenate)
                    </SelectItem>
                    <SelectItem value="deduplicate">
                      Deduplicate
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            )}

            {node.type === 'conditional' && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Condition</Label>
                  <Textarea
                    value={node.parameters.condition ?? ''}
                    onChange={(e) => onUpdateParameter('condition', e.target.value)}
                    placeholder="Describe the branching condition. e.g., 'If confidence > 0.8' or 'If lifecycle_stage equals SQL'"
                    className="mt-1 text-sm font-mono min-h-[100px]"
                  />
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <Label className="text-xs">True Branch Label</Label>
                    <Input
                      value={node.parameters.true_label ?? 'Yes'}
                      onChange={(e) => onUpdateParameter('true_label', e.target.value)}
                      className="h-8 text-sm mt-1"
                    />
                  </div>
                  <div>
                    <Label className="text-xs">False Branch Label</Label>
                    <Input
                      value={node.parameters.false_label ?? 'No'}
                      onChange={(e) => onUpdateParameter('false_label', e.target.value)}
                      className="h-8 text-sm mt-1"
                    />
                  </div>
                </div>
              </div>
            )}

            {node.type === 'send_notification' && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Notification Type</Label>
                  <Select
                    value={node.parameters.notification_type ?? 'in_app'}
                    onValueChange={(v) => onUpdateParameter('notification_type', v)}
                  >
                    <SelectTrigger className="h-8 text-sm mt-1"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="in_app">In-App Notification</SelectItem>
                      <SelectItem value="email">Email</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Recipient</Label>
                  <Input
                    value={node.parameters.recipient ?? ''}
                    onChange={(e) => onUpdateParameter('recipient', e.target.value)}
                    placeholder="User email or 'admin' or 'assignee'"
                    className="h-8 text-sm mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs">Subject</Label>
                  <Input
                    value={node.parameters.subject ?? ''}
                    onChange={(e) => onUpdateParameter('subject', e.target.value)}
                    placeholder="Notification subject"
                    className="h-8 text-sm mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs">Message Template</Label>
                  <Textarea
                    value={node.parameters.message_template ?? ''}
                    onChange={(e) => onUpdateParameter('message_template', e.target.value)}
                    placeholder="Message body. Use {{variable}} for dynamic content."
                    className="mt-1 text-sm min-h-[80px]"
                  />
                </div>
              </div>
            )}

            {node.type === 'assign_to_agent' && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Agent Codename</Label>
                  <Input
                    value={node.parameters.agent_codename ?? ''}
                    onChange={(e) => onUpdateParameter('agent_codename', e.target.value)}
                    placeholder="e.g., auri, nora, astra"
                    className="h-8 text-sm mt-1"
                  />
                  <p className="text-xs text-muted-foreground mt-1">The agent to assign the task to.</p>
                </div>
                <div>
                  <Label className="text-xs">Task Title Template</Label>
                  <Input
                    value={node.parameters.task_title_template ?? ''}
                    onChange={(e) => onUpdateParameter('task_title_template', e.target.value)}
                    placeholder="e.g., Implement {{feature_name}}"
                    className="h-8 text-sm mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs">Task Description Template</Label>
                  <Textarea
                    value={node.parameters.task_description_template ?? ''}
                    onChange={(e) => onUpdateParameter('task_description_template', e.target.value)}
                    placeholder="Describe what the agent should do. Use {{variable}} for upstream data."
                    className="mt-1 text-sm min-h-[80px]"
                  />
                </div>
                <div>
                  <Label className="text-xs">Completion Criteria</Label>
                  <Textarea
                    value={node.parameters.completion_criteria ?? ''}
                    onChange={(e) => onUpdateParameter('completion_criteria', e.target.value)}
                    placeholder="What must be true for this task to be done?"
                    className="mt-1 text-sm min-h-[60px]"
                  />
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={node.parameters.auto_start ?? true}
                    onChange={(e) => onUpdateParameter('auto_start', e.target.checked)}
                    className="rounded"
                  />
                  <Label className="text-xs">Auto-start agent execution after task creation</Label>
                </div>
              </div>
            )}

            {node.type === 'http_request' && (
              <div className="space-y-3">
                <div className="grid grid-cols-3 gap-2">
                  <div>
                    <Label className="text-xs">Method</Label>
                    <Select
                      value={node.parameters.method ?? 'GET'}
                      onValueChange={(v) => onUpdateParameter('method', v)}
                    >
                      <SelectTrigger className="h-8 text-sm mt-1"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="GET">GET</SelectItem>
                        <SelectItem value="POST">POST</SelectItem>
                        <SelectItem value="PUT">PUT</SelectItem>
                        <SelectItem value="PATCH">PATCH</SelectItem>
                        <SelectItem value="DELETE">DELETE</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="col-span-2">
                    <Label className="text-xs">URL</Label>
                    <Input
                      value={node.parameters.url ?? ''}
                      onChange={(e) => onUpdateParameter('url', e.target.value)}
                      placeholder="https://api.example.com/endpoint"
                      className="h-8 text-sm mt-1"
                    />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Headers (JSON)</Label>
                  <Textarea
                    value={node.parameters.headers ?? '{}'}
                    onChange={(e) => onUpdateParameter('headers', e.target.value)}
                    placeholder='{"Authorization": "Bearer {{token}}"}'
                    className="mt-1 text-sm font-mono min-h-[60px]"
                  />
                </div>
                <div>
                  <Label className="text-xs">Request Body</Label>
                  <Textarea
                    value={node.parameters.body ?? ''}
                    onChange={(e) => onUpdateParameter('body', e.target.value)}
                    placeholder="Request body (for POST/PUT). Use {{variable}} for upstream data."
                    className="mt-1 text-sm font-mono min-h-[60px]"
                  />
                </div>
                <div>
                  <Label className="text-xs">Output Path (JSONPath)</Label>
                  <Input
                    value={node.parameters.output_path ?? ''}
                    onChange={(e) => onUpdateParameter('output_path', e.target.value)}
                    placeholder="e.g., data.results or leave empty for full response"
                    className="h-8 text-sm mt-1"
                  />
                </div>
              </div>
            )}

            {(node.type === 'update_crm_contact' || node.type === 'update_crm_deal' || node.type === 'update_crm_company') && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Match Field</Label>
                  <Input
                    value={node.parameters.match_field ?? 'email'}
                    onChange={(e) => onUpdateParameter('match_field', e.target.value)}
                    placeholder={node.type === 'update_crm_contact' ? 'email' : 'name'}
                    className="h-8 text-sm mt-1"
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Field used to find the existing record to update.
                  </p>
                </div>
                <div>
                  <Label className="text-xs">Fields to Update (JSON)</Label>
                  <Textarea
                    value={node.parameters.update_fields ?? '{}'}
                    onChange={(e) => onUpdateParameter('update_fields', e.target.value)}
                    placeholder={'{"lifecycle_stage": "{{new_stage}}", "tags": ["{{tag}}"]}\nUse {{variable}} for upstream data.'}
                    className="mt-1 text-sm font-mono min-h-[100px]"
                  />
                </div>
              </div>
            )}

            {node.type.startsWith('output_') && (
              <OutputNodeConfig
                node={node}
                onUpdateParameter={onUpdateParameter}
              />
            )}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
