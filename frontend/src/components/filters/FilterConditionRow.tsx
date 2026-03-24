import { X } from 'lucide-react';

import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { FilterableField, FilterCondition, FilterOperator } from '@/types/filters';

interface FilterConditionRowProps {
  condition: FilterCondition;
  onUpdate: (updates: Partial<FilterCondition>) => void;
  onRemove: () => void;
}

const FIELD_OPTIONS: { value: FilterableField; label: string }[] = [
  { value: 'title', label: 'Title' },
  { value: 'description', label: 'Description' },
  { value: 'status', label: 'Status' },
  { value: 'priority', label: 'Priority' },
  { value: 'assignee', label: 'Assignee' },
  { value: 'created_at', label: 'Created Date' },
  { value: 'updated_at', label: 'Updated Date' },
];

const OPERATOR_OPTIONS: Record<string, { value: FilterOperator; label: string }[]> = {
  title: [
    { value: 'contains', label: 'Contains' },
    { value: 'not_contains', label: 'Does not contain' },
    { value: 'equals', label: 'Equals' },
    { value: 'not_equals', label: 'Not equals' },
  ],
  description: [
    { value: 'contains', label: 'Contains' },
    { value: 'not_contains', label: 'Does not contain' },
  ],
  status: [
    { value: 'equals', label: 'Is' },
    { value: 'not_equals', label: 'Is not' },
    { value: 'in', label: 'Is any of' },
  ],
  priority: [
    { value: 'equals', label: 'Is' },
    { value: 'not_equals', label: 'Is not' },
    { value: 'in', label: 'Is any of' },
  ],
  assignee: [
    { value: 'equals', label: 'Is' },
    { value: 'not_equals', label: 'Is not' },
    { value: 'contains', label: 'Contains' },
  ],
  created_at: [
    { value: 'before', label: 'Before' },
    { value: 'after', label: 'After' },
    { value: 'equals', label: 'On' },
  ],
  updated_at: [
    { value: 'before', label: 'Before' },
    { value: 'after', label: 'After' },
    { value: 'equals', label: 'On' },
  ],
};

const STATUS_OPTIONS = ['todo', 'inprogress', 'inreview', 'done', 'cancelled'];
const PRIORITY_OPTIONS = ['low', 'medium', 'high'];

export function FilterConditionRow({
  condition,
  onUpdate,
  onRemove,
}: FilterConditionRowProps) {
  const operators = OPERATOR_OPTIONS[condition.field] || [];

  const renderValueInput = () => {
    // Date fields
    if (condition.field === 'created_at' || condition.field === 'updated_at') {
      return (
        <Input
          type="date"
          value={condition.value || ''}
          onChange={(e) => onUpdate({ value: e.target.value })}
          className="flex-1"
        />
      );
    }

    // Status field
    if (condition.field === 'status') {
      if (condition.operator === 'in') {
        return (
          <Select
            value={Array.isArray(condition.value) ? condition.value.join(',') : ''}
            onValueChange={(value) => onUpdate({ value: value.split(',') })}
          >
            <SelectTrigger className="flex-1">
              <SelectValue placeholder="Select statuses..." />
            </SelectTrigger>
            <SelectContent>
              {STATUS_OPTIONS.map((status) => (
                <SelectItem key={status} value={status}>
                  {status}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        );
      }
      return (
        <Select
          value={condition.value || ''}
          onValueChange={(value) => onUpdate({ value })}
        >
          <SelectTrigger className="flex-1">
            <SelectValue placeholder="Select status..." />
          </SelectTrigger>
          <SelectContent>
            {STATUS_OPTIONS.map((status) => (
              <SelectItem key={status} value={status}>
                {status}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      );
    }

    // Priority field
    if (condition.field === 'priority') {
      if (condition.operator === 'in') {
        return (
          <Select
            value={Array.isArray(condition.value) ? condition.value.join(',') : ''}
            onValueChange={(value) => onUpdate({ value: value.split(',') })}
          >
            <SelectTrigger className="flex-1">
              <SelectValue placeholder="Select priorities..." />
            </SelectTrigger>
            <SelectContent>
              {PRIORITY_OPTIONS.map((priority) => (
                <SelectItem key={priority} value={priority}>
                  {priority}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        );
      }
      return (
        <Select
          value={condition.value || ''}
          onValueChange={(value) => onUpdate({ value })}
        >
          <SelectTrigger className="flex-1">
            <SelectValue placeholder="Select priority..." />
          </SelectTrigger>
          <SelectContent>
            {PRIORITY_OPTIONS.map((priority) => (
              <SelectItem key={priority} value={priority}>
                {priority}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      );
    }

    // Default text input
    return (
      <Input
        type="text"
        value={condition.value || ''}
        onChange={(e) => onUpdate({ value: e.target.value })}
        placeholder="Enter value..."
        className="flex-1"
      />
    );
  };

  return (
    <div className="flex items-center gap-2">
      {/* Field Select */}
      <Select
        value={condition.field}
        onValueChange={(value) => onUpdate({ field: value as FilterableField, value: '' })}
      >
        <SelectTrigger className="w-[140px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {FIELD_OPTIONS.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Operator Select */}
      <Select
        value={condition.operator}
        onValueChange={(value) => onUpdate({ operator: value as FilterOperator })}
      >
        <SelectTrigger className="w-[140px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {operators.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Value Input */}
      {renderValueInput()}

      {/* Remove Button */}
      <IconButton
        variant="ghost" onClick={onRemove}
        className="shrink-0"
        icon={X}
        label="Remove condition"
      />
    </div>
  );
}