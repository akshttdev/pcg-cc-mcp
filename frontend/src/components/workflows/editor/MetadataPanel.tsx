import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ChevronDown, ChevronRight, Lock, LockOpen } from 'lucide-react';
import type { AvailableModel } from '@/lib/api';

interface MetadataPanelProps {
  name: string;
  setName: (name: string) => void;
  description: string;
  setDescription: (desc: string) => void;
  id: string;
  setId: (id: string) => void;
  idLocked: boolean;
  setIdLocked: (locked: boolean) => void;
  isNew: boolean;
  defaultModel: string;
  setDefaultModel: (model: string) => void;
  availableModels: AvailableModel[];
  metadataExpanded: boolean;
  setMetadataExpanded: (expanded: boolean) => void;
}

export function MetadataPanel({
  name,
  setName,
  description,
  setDescription,
  id,
  setId,
  idLocked,
  setIdLocked,
  isNew,
  defaultModel,
  setDefaultModel,
  availableModels,
  metadataExpanded,
  setMetadataExpanded,
}: MetadataPanelProps) {
  return (
    <div className="border-b bg-muted/30">
      <button
        onClick={() => setMetadataExpanded(!metadataExpanded)}
        className="w-full flex items-center gap-2 px-4 py-2 hover:bg-muted/50 transition-colors text-left"
      >
        {metadataExpanded ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        )}
        <span className="text-sm font-medium truncate flex-1">
          {name || 'Untitled Workflow'}
        </span>
        {!metadataExpanded && description && (
          <span className="text-xs text-muted-foreground truncate max-w-[140px]">
            {description}
          </span>
        )}
      </button>
      {metadataExpanded && (
        <div className="px-4 pb-3 space-y-2">
          <div>
            <Label className="text-xs text-muted-foreground">Name</Label>
            <Input
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                if (isNew && idLocked) {
                  setId(
                    e.target.value
                      .toLowerCase()
                      .replace(/[^a-z0-9]+/g, '_')
                      .replace(/^_|_$/g, '')
                  );
                }
              }}
              placeholder="My Workflow"
              className="h-8 text-sm mt-1"
            />
            {isNew && (
              <div className="flex items-center gap-1.5 mt-1">
                <span className="text-xs text-muted-foreground">
                  ID: <code className="font-mono">{id || 'auto_generated'}</code>
                </span>
                <button
                  type="button"
                  onClick={() => setIdLocked(!idLocked)}
                  className="p-0.5 rounded hover:bg-muted transition-colors"
                  title={idLocked ? 'Unlock to edit ID manually' : 'Lock to auto-generate from name'}
                >
                  {idLocked ? (
                    <Lock className="h-3 w-3 text-muted-foreground" />
                  ) : (
                    <LockOpen className="h-3 w-3 text-primary" />
                  )}
                </button>
              </div>
            )}
            {isNew && !idLocked && (
              <Input
                value={id}
                onChange={(e) =>
                  setId(
                    e.target.value
                      .toLowerCase()
                      .replace(/[^a-z0-9_]/g, '_')
                  )
                }
                placeholder="my_workflow"
                className="h-7 text-xs mt-1 font-mono"
              />
            )}
          </div>
          <div>
            <Label className="text-xs text-muted-foreground">
              Description
            </Label>
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What does this workflow do?"
              className="h-8 text-sm mt-1"
            />
          </div>
          <div>
            <Label className="text-xs text-muted-foreground">Default Model</Label>
            <Select value={defaultModel || '__auto__'} onValueChange={(v) => setDefaultModel(v === '__auto__' ? '' : v)}>
              <SelectTrigger className="h-8 text-sm mt-1">
                <SelectValue placeholder="Auto (highest priority)" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__auto__">Auto (highest priority)</SelectItem>
                {availableModels.map((m: AvailableModel) => (
                  <SelectItem key={m.id} value={m.id}>
                    <span className="flex items-center gap-2">
                      <span>{m.label}</span>
                      <span className="text-muted-foreground text-xs">
                        ${(m.cost_per_million_input / 100).toFixed(2)}/M in
                      </span>
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
      )}
    </div>
  );
}
