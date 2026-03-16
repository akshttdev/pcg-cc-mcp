// ─── InlineEditField ──────────────────────────────────────────────────────────
//
// Schema-aware inline editable field for the staging expanded row detail panel.

import { useState, useCallback } from 'react';
import type { FieldDef } from '@/lib/api';
import { cn } from '@/lib/utils';

interface InlineEditFieldProps {
  fieldKey: string;
  value: string;
  recordId: string;
  onSave: (recordId: string, key: string, value: string) => void;
  editable: boolean;
  fieldDef?: FieldDef;
}

export function InlineEditField({
  fieldKey,
  value,
  recordId,
  onSave,
  editable,
  fieldDef,
}: InlineEditFieldProps) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(value);
  const inputRef = useCallback((el: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null) => {
    if (el) el.focus();
  }, []);

  const handleSave = useCallback(() => {
    if (editValue !== value) {
      onSave(recordId, fieldKey, editValue);
    }
    setEditing(false);
  }, [editValue, value, onSave, recordId, fieldKey]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSave();
    }
    if (e.key === 'Escape') {
      setEditValue(value);
      setEditing(false);
    }
  }, [handleSave, value]);

  const handleStartEdit = useCallback(() => {
    if (!editable) return;
    setEditValue(value);
    setEditing(true);
  }, [editable, value]);

  const isLong = value.length > 80;
  const hasEnum = fieldDef?.enum_values && fieldDef.enum_values.length > 0;
  const inputType = fieldDef?.type === 'number' ? 'number'
    : fieldDef?.format === 'email' ? 'email'
    : fieldDef?.format === 'url' ? 'url'
    : (fieldDef?.format === 'date' || fieldDef?.format === 'date-time') ? 'date'
    : 'text';

  if (editing) {
    // Enum dropdown
    if (hasEnum) {
      return (
        <select
          ref={inputRef as React.Ref<HTMLSelectElement>}
          value={editValue}
          onChange={(e) => { setEditValue(e.target.value); }}
          onBlur={handleSave}
          onKeyDown={handleKeyDown}
          className="w-full text-xs bg-background border rounded px-1 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring h-6"
        >
          <option value="">— select —</option>
          {fieldDef!.enum_values!.map(v => (
            <option key={v} value={v}>{v.replace(/_/g, ' ')}</option>
          ))}
        </select>
      );
    }
    // Long text / array -> textarea
    if (isLong || fieldDef?.type === 'array') {
      return (
        <textarea
          ref={inputRef as React.Ref<HTMLTextAreaElement>}
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onBlur={handleSave}
          onKeyDown={handleKeyDown}
          placeholder={fieldDef?.type === 'array' ? 'Comma-separated values' : undefined}
          className="w-full text-xs bg-background border rounded px-1.5 py-1 focus:outline-none focus:ring-1 focus:ring-ring min-h-[60px] resize-y"
        />
      );
    }
    // Typed input
    return (
      <input
        ref={inputRef as React.Ref<HTMLInputElement>}
        type={inputType}
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={handleSave}
        onKeyDown={handleKeyDown}
        className="w-full text-xs bg-background border rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring h-6"
      />
    );
  }

  // Display format hint
  const formatHint = fieldDef?.required ? ' *' : '';

  return (
    <dd
      onClick={editable ? handleStartEdit : undefined}
      className={cn(
        'text-xs mt-0.5',
        isLong ? 'whitespace-pre-wrap break-words' : 'truncate',
        editable && 'cursor-text hover:bg-muted/40 rounded px-1 -mx-1 transition-colors',
      )}
      title={editable ? `Click to edit${fieldDef ? ` (${fieldDef.type}${formatHint})` : ''}` : fieldDef?.description}
    >
      {value || <span className="text-muted-foreground italic">empty</span>}
    </dd>
  );
}
