import type { LucideIcon } from 'lucide-react';
import {
  Database,
  FileSearch,
  Brain,
  Zap,
  Settings2,
  Filter,
  Merge,
  Users,
  Building2,
  Handshake,
  ListTodo,
  GitBranch,
  Bell,
  Bot,
  Globe,
  PenSquare,
  RefreshCw,
} from 'lucide-react';

// ── Node type registry (n8n-style) ──────────────────────────────────────────

export interface NodeTypeDefinition {
  type: string;
  label: string;
  description: string;
  icon: LucideIcon;
  color: string;
  defaultParameters: Record<string, unknown>;
}

export const NODE_TYPES: NodeTypeDefinition[] = [
  {
    type: 'data_source',
    label: 'Data Source',
    description: 'Marks this workflow as data-source-driven (source selected at run time)',
    icon: Database,
    color: 'bg-slate-600',
    defaultParameters: {
      accepted_types: '', // optional: comma-separated data types this workflow can process
    },
  },
  {
    type: 'llm_extract',
    label: 'LLM Extract',
    description: 'Extract structured data using an LLM prompt',
    icon: FileSearch,
    color: 'bg-blue-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
      output_mode: 'auto',
    },
  },
  {
    type: 'llm_analyze',
    label: 'LLM Analyze',
    description: 'Analyze content and generate insights',
    icon: Brain,
    color: 'bg-purple-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
      output_mode: 'auto',
    },
  },
  {
    type: 'llm_summarize',
    label: 'LLM Summarize',
    description: 'Summarize and condense content',
    icon: Zap,
    color: 'bg-amber-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
      output_mode: 'auto',
    },
  },
  {
    type: 'transform',
    label: 'Transform',
    description: 'Transform data format or structure',
    icon: Settings2,
    color: 'bg-green-500',
    defaultParameters: {
      transform_expression: '',
    },
  },
  {
    type: 'filter',
    label: 'Filter',
    description: 'Filter results based on conditions',
    icon: Filter,
    color: 'bg-orange-500',
    defaultParameters: {
      condition: '',
    },
  },
  {
    type: 'merge',
    label: 'Merge',
    description: 'Merge results from multiple inputs',
    icon: Merge,
    color: 'bg-teal-500',
    defaultParameters: {
      merge_strategy: 'combine',
    },
  },
  {
    type: 'output_crm_contacts',
    label: 'Output: CRM Contacts',
    description: 'Send extracted contacts to CRM',
    icon: Users,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'crm_contact',
      on_duplicate: 'flag_for_review',
    },
  },
  {
    type: 'output_crm_companies',
    label: 'Output: Companies',
    description: 'Send extracted companies to company records',
    icon: Building2,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'company',
      on_duplicate: 'flag_for_review',
    },
  },
  {
    type: 'output_crm_deals',
    label: 'Output: CRM Deals',
    description: 'Create deals/opportunities in CRM pipeline',
    icon: Handshake,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'crm_deal',
      on_duplicate: 'flag_for_review',
    },
  },
  {
    type: 'output_tasks',
    label: 'Output: Tasks',
    description: 'Create tasks from workflow results',
    icon: ListTodo,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'task',
      on_duplicate: 'skip',
    },
  },
  // --- Control Flow ---
  {
    type: 'conditional',
    label: 'Conditional',
    description: 'Branch execution based on a condition (routes to different downstream nodes)',
    icon: GitBranch,
    color: 'bg-yellow-600',
    defaultParameters: {
      condition: '',
      true_label: 'Yes',
      false_label: 'No',
    },
  },
  // --- Action Nodes ---
  {
    type: 'send_notification',
    label: 'Send Notification',
    description: 'Send an in-app notification or email alert',
    icon: Bell,
    color: 'bg-pink-500',
    defaultParameters: {
      notification_type: 'in_app',
      recipient: '',
      subject: '',
      message_template: '',
    },
  },
  {
    type: 'assign_to_agent',
    label: 'Assign to Agent',
    description: 'Create a task and assign it to an AI agent for autonomous execution',
    icon: Bot,
    color: 'bg-indigo-600',
    defaultParameters: {
      agent_codename: '',
      task_title_template: '',
      task_description_template: '',
      completion_criteria: '',
      auto_start: true,
    },
  },
  {
    type: 'http_request',
    label: 'HTTP Request',
    description: 'Make an external API call and use the response downstream',
    icon: Globe,
    color: 'bg-sky-500',
    defaultParameters: {
      method: 'GET',
      url: '',
      headers: '{}',
      body: '',
      output_path: '', // JSONPath to extract from response
    },
  },
  // --- CRM Action Nodes ---
  {
    type: 'update_crm_contact',
    label: 'Update CRM Contact',
    description: 'Update fields on existing CRM contacts (e.g., lifecycle stage, tags)',
    icon: PenSquare,
    color: 'bg-cyan-600',
    defaultParameters: {
      match_field: 'email',
      update_fields: '{}',
    },
  },
  {
    type: 'update_crm_deal',
    label: 'Update CRM Deal',
    description: 'Update deal stage, value, or other fields on existing deals',
    icon: RefreshCw,
    color: 'bg-cyan-600',
    defaultParameters: {
      match_field: 'name',
      update_fields: '{}',
    },
  },
  {
    type: 'update_crm_company',
    label: 'Update Company',
    description: 'Update fields on existing company records (e.g., relationship, industry)',
    icon: Building2,
    color: 'bg-cyan-600',
    defaultParameters: {
      match_field: 'name',
      update_fields: '{}',
    },
  },
];

export function getNodeTypeDef(type: string): NodeTypeDefinition | undefined {
  return NODE_TYPES.find((t) => t.type === type);
}

// ── Target schema definitions for output nodes ──────────────────────────────

export const TARGET_SCHEMAS: Record<string, { label: string; fields: { name: string; type: string; required?: boolean }[] }> = {
  crm_contact: {
    label: 'CRM Contact',
    fields: [
      { name: 'first_name', type: 'string', required: true },
      { name: 'last_name', type: 'string', required: true },
      { name: 'email', type: 'string', required: true },
      { name: 'phone', type: 'string' },
      { name: 'company_name', type: 'string' },
      { name: 'job_title', type: 'string' },
      { name: 'department', type: 'string' },
      { name: 'linkedin_url', type: 'url' },
      { name: 'lifecycle_stage', type: 'enum: subscriber|lead|mql|sql|opportunity|customer' },
      { name: 'source', type: 'string' },
      { name: 'tags', type: 'string[]' },
    ],
  },
  company: {
    label: 'Company',
    fields: [
      { name: 'name', type: 'string', required: true },
      { name: 'domain', type: 'url' },
      { name: 'industry', type: 'string' },
      { name: 'size', type: 'string' },
      { name: 'description', type: 'string' },
      { name: 'phone', type: 'string' },
      { name: 'email', type: 'string' },
      { name: 'address', type: 'string' },
      { name: 'city', type: 'string' },
      { name: 'country', type: 'string' },
      { name: 'linkedin_url', type: 'url' },
      { name: 'tags', type: 'string[]' },
    ],
  },
  crm_deal: {
    label: 'CRM Deal',
    fields: [
      { name: 'name', type: 'string', required: true },
      { name: 'amount', type: 'number' },
      { name: 'currency', type: 'string (ISO 4217)' },
      { name: 'probability', type: 'number (0-100)' },
      { name: 'expected_close_date', type: 'date (YYYY-MM-DD)' },
      { name: 'description', type: 'string' },
      { name: 'contact_email', type: 'string' },
      { name: 'tags', type: 'string[]' },
    ],
  },
  task: {
    label: 'Task',
    fields: [
      { name: 'title', type: 'string', required: true },
      { name: 'description', type: 'string' },
      { name: 'status', type: 'enum: todo|inprogress|done' },
      { name: 'priority', type: 'enum: low|medium|high|critical' },
      { name: 'tags', type: 'string[]' },
    ],
  },
};
