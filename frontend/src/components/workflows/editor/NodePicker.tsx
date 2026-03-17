import { useState, useRef, useEffect, useMemo } from 'react';
import { X, Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { NODE_TYPES } from './node-types';

interface NodePickerProps {
  onSelect: (type: string) => void;
  onClose: () => void;
}

const CATEGORIES = [
  { key: 'sources', label: 'Sources', filter: (t: string) => t === 'data_source' },
  { key: 'processing', label: 'Processing', filter: (t: string) => ['llm_extract', 'llm_analyze', 'llm_summarize', 'transform', 'filter', 'merge', 'conditional'].includes(t) },
  { key: 'actions', label: 'Actions', filter: (t: string) => ['send_notification', 'assign_to_agent', 'http_request', 'update_crm_contact', 'update_crm_deal', 'update_crm_company'].includes(t) },
  { key: 'outputs', label: 'Outputs', filter: (t: string) => t.startsWith('output_') },
] as const;

export function NodePicker({ onSelect, onClose }: NodePickerProps) {
  const [search, setSearch] = useState('');
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  const filteredByCategory = useMemo(() => {
    const lower = search.toLowerCase();
    const filtered = search
      ? NODE_TYPES.filter(
          (nt) =>
            nt.label.toLowerCase().includes(lower) ||
            nt.description.toLowerCase().includes(lower) ||
            nt.type.toLowerCase().includes(lower)
        )
      : NODE_TYPES;

    return CATEGORIES.map((cat) => ({
      ...cat,
      nodes: filtered.filter((nt) => cat.filter(nt.type)),
    })).filter((cat) => cat.nodes.length > 0);
  }, [search]);

  const totalResults = filteredByCategory.reduce((sum, cat) => sum + cat.nodes.length, 0);

  return (
    <div className="rounded-lg border bg-card shadow-lg p-2 space-y-1">
      <div className="flex items-center justify-between px-2 pb-1">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Add Node
        </span>
        <button
          onClick={onClose}
          className="p-0.5 rounded hover:bg-muted transition-colors"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Search input */}
      <div className="relative px-1 pb-1">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
        <Input
          ref={searchRef}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search nodes..."
          className="h-7 pl-8 text-xs"
        />
      </div>

      <div className="max-h-[400px] overflow-y-auto">
        {totalResults === 0 && (
          <div className="text-center py-6 text-muted-foreground">
            <p className="text-xs">No nodes match "{search}"</p>
          </div>
        )}

        {filteredByCategory.map((cat) => (
          <div key={cat.key}>
            <div className="px-2 py-1 sticky top-0 bg-card z-10">
              <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                {cat.label} ({cat.nodes.length})
              </span>
            </div>
            {cat.nodes.map((nt) => {
              const Icon = nt.icon;
              return (
                <button
                  key={nt.type}
                  onClick={() => onSelect(nt.type)}
                  className={cn(
                    'w-full flex items-center gap-3 rounded-md px-2 py-2',
                    'hover:bg-accent transition-colors text-left'
                  )}
                >
                  <div
                    className={cn(
                      'w-8 h-8 rounded-md flex items-center justify-center text-white shrink-0',
                      nt.color
                    )}
                  >
                    <Icon className="h-4 w-4" />
                  </div>
                  <div className="min-w-0">
                    <div className="text-sm font-medium">{nt.label}</div>
                    <div className="text-xs text-muted-foreground">
                      {nt.description}
                    </div>
                  </div>
                </button>
              );
            })}
            {cat.key !== filteredByCategory[filteredByCategory.length - 1].key && (
              <div className="border-t my-1" />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
