import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { NODE_COLOR } from '@/lib/graph/adapter';

export interface FilterState {
  nodeTypes: Set<string>;
  edgeTypes: Set<string>;
  query: string;
}

export interface FilterPaneProps {
  available: { nodeTypes: string[]; edgeTypes: string[] };
  state: FilterState;
  onChange: (next: FilterState) => void;
}

/**
 * Side pane with node-type / edge-type checkboxes and a label search.
 * Host owns state; this component is purely presentational.
 */
export function FilterPane({ available, state, onChange }: FilterPaneProps) {
  const toggleNode = (t: string) => {
    const next = new Set(state.nodeTypes);
    next.has(t) ? next.delete(t) : next.add(t);
    onChange({ ...state, nodeTypes: next });
  };
  const toggleEdge = (t: string) => {
    const next = new Set(state.edgeTypes);
    next.has(t) ? next.delete(t) : next.add(t);
    onChange({ ...state, edgeTypes: next });
  };

  return (
    <div
      className="flex w-56 flex-col gap-4 border-r border-slate-800 bg-slate-900/70 p-3 text-sm text-slate-200"
      data-testid="topology-filter-pane"
    >
      <div className="flex flex-col gap-1">
        <Label htmlFor="topology-search" className="text-xs text-slate-400">
          Search
        </Label>
        <Input
          id="topology-search"
          value={state.query}
          onChange={(e) => onChange({ ...state, query: e.target.value })}
          placeholder="Label contains…"
          data-testid="topology-filter-search"
          className="h-7 bg-slate-800 text-xs"
        />
      </div>

      <div className="flex flex-col gap-1">
        <div className="text-xs uppercase tracking-wide text-slate-500">
          Node types
        </div>
        <div className="flex flex-col gap-1">
          {available.nodeTypes.map((t) => (
            <label
              key={t}
              className="flex cursor-pointer items-center gap-2 text-xs"
              data-testid={`topology-filter-node-${t}`}
            >
              <Checkbox
                checked={state.nodeTypes.has(t)}
                onCheckedChange={() => toggleNode(t)}
              />
              <span
                className="inline-block h-2 w-2 rounded-full"
                style={{ backgroundColor: NODE_COLOR[t] ?? '#aaaaff' }}
              />
              <span className="truncate">{t}</span>
            </label>
          ))}
        </div>
      </div>

      <div className="flex flex-col gap-1">
        <div className="text-xs uppercase tracking-wide text-slate-500">
          Edge types
        </div>
        <div className="flex flex-col gap-1">
          {available.edgeTypes.map((t) => (
            <label
              key={t}
              className="flex cursor-pointer items-center gap-2 text-xs"
              data-testid={`topology-filter-edge-${t}`}
            >
              <Checkbox
                checked={state.edgeTypes.has(t)}
                onCheckedChange={() => toggleEdge(t)}
              />
              <span className="truncate">{t}</span>
            </label>
          ))}
        </div>
      </div>
    </div>
  );
}
