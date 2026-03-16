import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { DATA_TYPE_OPTIONS } from '@/lib/api';

interface DataSourceNodeConfigProps {
  acceptedTypes: string;
  onUpdateAcceptedTypes: (v: string) => void;
}

export function DataSourceNodeConfig({ acceptedTypes, onUpdateAcceptedTypes }: DataSourceNodeConfigProps) {
  return (
    <div className="space-y-3">
      <div className="rounded-md border border-dashed border-muted-foreground/30 p-2.5 bg-muted/20">
        <p className="text-[10px] text-muted-foreground">
          This node marks the workflow as data-source-driven. The data source will be selected at run time.
          Workflows with this node will appear in the Data Library's "Run Workflow" action.
        </p>
      </div>
      <div>
        <Label className="text-xs">Accepted Data Type</Label>
        <Select
          value={acceptedTypes || '__any__'}
          onValueChange={(v) => onUpdateAcceptedTypes(v === '__any__' ? '' : v)}
        >
          <SelectTrigger className="h-8 text-sm mt-1">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__any__">Any data source</SelectItem>
            {DATA_TYPE_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-[10px] text-muted-foreground mt-1">
          Filter which data source types this workflow can process.
        </p>
      </div>
    </div>
  );
}
