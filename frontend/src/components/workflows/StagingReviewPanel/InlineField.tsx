import { useState, useCallback } from 'react';
import type { FieldDef } from '@/lib/api';

interface InlineFieldProps {
  fieldKey: string;
  value: string;
  recordId: string;
  onSave: (recordId: string, key: string, value: string) => void;
  fieldDef?: FieldDef;
}

export function InlineField({
  fieldKey,
  value,
  recordId,
  onSave,
  fieldDef,
}: InlineFieldProps) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(value);
  const inputRef = useCallback((el: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null) => {
    if (el) el.focus();
  }, []);

  const handleSave = useCallback(() => {
    setEditing(false);
    if (editValue !== value) onSave(recordId, fieldKey, editValue);
  }, [editValue, value, recordId, fieldKey, onSave]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSave(); }
    if (e.key === 'Escape') { setEditValue(value); setEditing(false); }
  }, [handleSave, value]);

  if (editing) {
    const hasEnum = fieldDef?.enum_values && fieldDef.enum_values.length > 0;
    const isLong = value.length > 80 || fieldDef?.type === 'array';
    const inputType = fieldDef?.type === 'number' ? 'number'
      : fieldDef?.format === 'email' ? 'email'
      : fieldDef?.format === 'url' ? 'url'
      : (fieldDef?.format === 'date' || fieldDef?.format === 'date-time') ? 'date'
      : 'text';

    if (hasEnum) {
      return (
        <select ref={inputRef} value={editValue} onChange={e => { setEditValue(e.target.value); }}
          onBlur={handleSave} className="w-full text-xs bg-background border rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring">
          <option value="">— select —</option>
          {fieldDef!.enum_values!.map(v => <option key={v} value={v}>{v.replace(/_/g, ' ')}</option>)}
        </select>
      );
    }
    if (isLong) {
      return (
        <textarea ref={inputRef} value={editValue} onChange={e => setEditValue(e.target.value)}
          onBlur={handleSave} onKeyDown={handleKeyDown}
          placeholder={fieldDef?.type === 'array' ? 'Comma-separated values' : undefined}
          className="w-full text-xs bg-background border rounded px-1.5 py-1 focus:outline-none focus:ring-1 focus:ring-ring min-h-[60px] resize-y" />
      );
    }
    return (
      <input ref={inputRef} type={inputType} value={editValue} onChange={e => setEditValue(e.target.value)}
        onBlur={handleSave} onKeyDown={handleKeyDown}
        className="w-full text-xs bg-background border rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring" />
    );
  }

  return (
    <dd
      onClick={() => { setEditValue(value); setEditing(true); }}
      className="text-xs mt-0.5 cursor-text hover:bg-muted/40 rounded px-1 -mx-1 transition-colors truncate"
      title={`Click to edit${fieldDef ? ` (${fieldDef.type})` : ''}`}
    >
      {value || <span className="text-muted-foreground italic">empty</span>}
    </dd>
  );
}
