import { useState, useEffect } from 'react';
import { Plus, X, Edit2, Trash2, Save, ChevronDown, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Card } from '@/components/ui/card';

export type CustomFieldType =
  | 'text'
  | 'number'
  | 'date'
  | 'select'
  | 'multiselect'
  | 'checkbox'
  | 'url';

export interface CustomFieldDefinition {
  id: string;
  name: string;
  type: CustomFieldType;
  required: boolean;
  options?: string[]; // For select/multiselect
  defaultValue?: string;
}

export interface CustomFieldValue {
  fieldId: string;
  value: string | string[] | boolean;
}

interface CustomPropertiesPanelProps {
  projectId: string;
  taskId?: string;
  values?: CustomFieldValue[];
  onChange?: (values: CustomFieldValue[]) => void;
  readOnly?: boolean;
}

const FIELD_TYPE_LABELS: Record<CustomFieldType, string> = {
  text: 'Text',
  number: 'Number',
  date: 'Date',
  select: 'Select',
  multiselect: 'Multi-Select',
  checkbox: 'Checkbox',
  url: 'URL',
};

// LocalStorage keys
const getFieldsKey = (projectId: string) => `custom-fields-${projectId}`;
const getValuesKey = (projectId: string, taskId: string) =>
  `custom-values-${projectId}-${taskId}`;

export function CustomPropertiesPanel({
  projectId,
  taskId,
  values = [],
  onChange,
  readOnly = false,
}: CustomPropertiesPanelProps) {
  const [fields, setFields] = useState<CustomFieldDefinition[]>([]);
  const [localValues, setLocalValues] = useState<CustomFieldValue[]>(values);
  const [isExpanded, setIsExpanded] = useState(false);

  // Load field definitions from localStorage
  useEffect(() => {
    const stored = localStorage.getItem(getFieldsKey(projectId));
    if (stored) {
      try {
        setFields(JSON.parse(stored));
      } catch (e) {
        console.error('Failed to parse custom fields:', e);
      }
    }
  }, [projectId]);

  // Load values from localStorage if taskId is provided
  useEffect(() => {
    if (taskId) {
      const stored = localStorage.getItem(getValuesKey(projectId, taskId));
      if (stored) {
        try {
          const parsed = JSON.parse(stored);
          setLocalValues(parsed);
          onChange?.(parsed);
        } catch (e) {
          console.error('Failed to parse custom values:', e);
        }
      }
    }
  }, [projectId, taskId]);

  const handleValueChange = (fieldId: string, value: string | string[] | boolean) => {
    const newValues = localValues.filter((v) => v.fieldId !== fieldId);
    newValues.push({ fieldId, value });
    setLocalValues(newValues);
    onChange?.(newValues);

    // Persist to localStorage if taskId is provided
    if (taskId) {
      localStorage.setItem(getValuesKey(projectId, taskId), JSON.stringify(newValues));
    }
  };

  const getValue = (fieldId: string) => {
    return localValues.find((v) => v.fieldId === fieldId)?.value;
  };

  if (fields.length === 0) {
    return (
      <Card className="p-4 border-dashed">
        <div className="flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            No custom properties defined for this project
          </div>
          <ManageFieldsDialog
            projectId={projectId}
            fields={fields}
            onFieldsChange={setFields}
            trigger={
              <Button variant="outline" size="sm">
                <Plus className="h-4 w-4 mr-1" />
                Add Properties
              </Button>
            }
          />
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-4">
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex items-center gap-2 text-sm font-medium hover:text-foreground transition-colors"
          >
            {isExpanded ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
            Custom Properties ({fields.length})
          </button>
          <ManageFieldsDialog
            projectId={projectId}
            fields={fields}
            onFieldsChange={setFields}
            trigger={
              <Button variant="ghost" size="sm">
                <Edit2 className="h-3 w-3" />
              </Button>
            }
          />
        </div>

        {isExpanded && (
          <div className="space-y-3 pt-2">
            {fields.map((field) => (
              <CustomFieldInput
                key={field.id}
                field={field}
                value={getValue(field.id)}
                onChange={(value) => handleValueChange(field.id, value)}
                readOnly={readOnly}
              />
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}

interface CustomFieldInputProps {
  field: CustomFieldDefinition;
  value?: string | string[] | boolean;
  onChange: (value: string | string[] | boolean) => void;
  readOnly?: boolean;
}

function CustomFieldInput({ field, value, onChange, readOnly }: CustomFieldInputProps) {
  const renderInput = () => {
    switch (field.type) {
      case 'text':
      case 'url':
        return (
          <Input
            type={field.type === 'url' ? 'url' : 'text'}
            value={(value as string) || ''}
            onChange={(e) => onChange(e.target.value)}
            placeholder={`Enter ${field.name.toLowerCase()}...`}
            disabled={readOnly}
            className="mt-1"
          />
        );

      case 'number':
        return (
          <Input
            type="number"
            value={(value as string) || ''}
            onChange={(e) => onChange(e.target.value)}
            placeholder={`Enter ${field.name.toLowerCase()}...`}
            disabled={readOnly}
            className="mt-1"
          />
        );

      case 'date':
        return (
          <Input
            type="date"
            value={(value as string) || ''}
            onChange={(e) => onChange(e.target.value)}
            disabled={readOnly}
            className="mt-1"
          />
        );

      case 'checkbox':
        return (
          <div className="flex items-center gap-2 mt-1">
            <input
              type="checkbox"
              checked={!!value}
              onChange={(e) => onChange(e.target.checked)}
              disabled={readOnly}
              className="h-4 w-4 rounded border-border"
            />
          </div>
        );

      case 'select':
        return (
          <Select
            value={(value as string) || ''}
            onValueChange={onChange}
            disabled={readOnly}
          >
            <SelectTrigger className="mt-1">
              <SelectValue placeholder={`Select ${field.name.toLowerCase()}...`} />
            </SelectTrigger>
            <SelectContent>
              {field.options?.map((option) => (
                <SelectItem key={option} value={option}>
                  {option}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        );

      case 'multiselect': {
        const selectedValues = (value as string[]) || [];
        return (
          <div className="space-y-2 mt-1">
            {field.options?.map((option) => (
              <label key={option} className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={selectedValues.includes(option)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      onChange([...selectedValues, option]);
                    } else {
                      onChange(selectedValues.filter((v) => v !== option));
                    }
                  }}
                  disabled={readOnly}
                  className="h-4 w-4 rounded border-border"
                />
                {option}
              </label>
            ))}
          </div>
        );
      }

      default:
        return null;
    }
  };

  return (
    <div>
      <Label className="text-sm font-medium flex items-center gap-1">
        {field.name}
        {field.required && <span className="text-destructive">*</span>}
      </Label>
      {renderInput()}
    </div>
  );
}

interface ManageFieldsDialogProps {
  projectId: string;
  fields: CustomFieldDefinition[];
  onFieldsChange: (fields: CustomFieldDefinition[]) => void;
  trigger: React.ReactNode;
}

function ManageFieldsDialog({
  projectId,
  fields,
  onFieldsChange,
  trigger,
}: ManageFieldsDialogProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [localFields, setLocalFields] = useState<CustomFieldDefinition[]>(fields);
  const [editingField, setEditingField] = useState<string | null>(null);

  useEffect(() => {
    setLocalFields(fields);
  }, [fields]);

  const saveFields = () => {
    localStorage.setItem(getFieldsKey(projectId), JSON.stringify(localFields));
    onFieldsChange(localFields);
    setIsOpen(false);
  };

  const addField = () => {
    const newField: CustomFieldDefinition = {
      id: `field-${Date.now()}`,
      name: 'New Field',
      type: 'text',
      required: false,
    };
    setLocalFields([...localFields, newField]);
    setEditingField(newField.id);
  };

  const updateField = (id: string, updates: Partial<CustomFieldDefinition>) => {
    setLocalFields(
      localFields.map((f) => (f.id === id ? { ...f, ...updates } : f))
    );
  };

  const deleteField = (id: string) => {
    setLocalFields(localFields.filter((f) => f.id !== id));
  };

  return (
    <>
      <div onClick={() => setIsOpen(true)}>{trigger}</div>
      <Dialog open={isOpen} onOpenChange={setIsOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Manage Custom Properties</DialogTitle>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-3">
            {localFields.map((field) => (
              <Card key={field.id} className="p-4">
                {editingField === field.id ? (
                  <EditFieldForm
                    field={field}
                    onSave={(updates) => {
                      updateField(field.id, updates);
                      setEditingField(null);
                    }}
                    onCancel={() => setEditingField(null)}
                  />
                ) : (
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <div className="font-medium">{field.name}</div>
                      <div className="text-sm text-muted-foreground">
                        {FIELD_TYPE_LABELS[field.type]}
                        {field.required && ' • Required'}
                        {field.options && ` • ${field.options.length} options`}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setEditingField(field.id)}
                      >
                        <Edit2 className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => deleteField(field.id)}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                )}
              </Card>
            ))}
            </div>

          <Button onClick={addField} variant="outline" className="w-full">
            <Plus className="h-4 w-4 mr-2" />
            Add Field
          </Button>

          <DialogFooter className="pt-4">
            <Button variant="outline" onClick={() => setIsOpen(false)}>
              Cancel
            </Button>
            <Button onClick={saveFields}>
              <Save className="h-4 w-4 mr-2" />
              Save Changes
            </Button>
          </DialogFooter>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

interface EditFieldFormProps {
  field: CustomFieldDefinition;
  onSave: (updates: Partial<CustomFieldDefinition>) => void;
  onCancel: () => void;
}

function EditFieldForm({ field, onSave, onCancel }: EditFieldFormProps) {
  const [name, setName] = useState(field.name);
  const [type, setType] = useState<CustomFieldType>(field.type);
  const [required, setRequired] = useState(field.required);
  const [options, setOptions] = useState<string[]>(field.options || []);
  const [newOption, setNewOption] = useState('');

  const needsOptions = type === 'select' || type === 'multiselect';

  const handleSave = () => {
    onSave({
      name,
      type,
      required,
      options: needsOptions ? options : undefined,
    });
  };

  const addOption = () => {
    if (newOption.trim()) {
      setOptions([...options, newOption.trim()]);
      setNewOption('');
    }
  };

  const removeOption = (index: number) => {
    setOptions(options.filter((_, i) => i !== index));
  };

  return (
    <div className="space-y-3">
      <div>
        <Label className="text-sm font-medium">Field Name</Label>
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter field name..."
          className="mt-1"
        />
      </div>

      <div>
        <Label className="text-sm font-medium">Field Type</Label>
        <Select value={type} onValueChange={(v) => setType(v as CustomFieldType)}>
          <SelectTrigger className="mt-1">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.entries(FIELD_TYPE_LABELS).map(([value, label]) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={required}
          onChange={(e) => setRequired(e.target.checked)}
          className="h-4 w-4 rounded border-border"
          id={`required-${field.id}`}
        />
        <Label htmlFor={`required-${field.id}`} className="text-sm font-medium">
          Required field
        </Label>
      </div>

      {needsOptions && (
        <div>
          <Label className="text-sm font-medium">Options</Label>
          <div className="space-y-2 mt-1">
            {options.map((option, index) => (
              <div key={index} className="flex items-center gap-2">
                <Input value={option} readOnly className="flex-1" />
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => removeOption(index)}
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ))}
            <div className="flex items-center gap-2">
              <Input
                value={newOption}
                onChange={(e) => setNewOption(e.target.value)}
                placeholder="Add option..."
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    addOption();
                  }
                }}
              />
              <Button onClick={addOption} size="sm">
                <Plus className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      )}

      <DialogFooter className="pt-2">
        <Button variant="outline" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button size="sm" onClick={handleSave}>
          Save
        </Button>
      </DialogFooter>
    </div>
  );
}
