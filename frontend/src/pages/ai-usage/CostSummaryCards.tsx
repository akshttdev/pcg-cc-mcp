import { Card, CardContent } from '@/components/ui/card';
import { Loader } from '@/components/ui/loader';
import {
  Activity,
  ArrowUpRight,
  ArrowDownRight,
  Coins,
  Cpu,
} from 'lucide-react';
import { formatCost, formatTokens } from '@/lib/format';

export interface PeriodTotals {
  tokens: number;
  input: number;
  output: number;
  cost: number;
  requests: number;
}

interface CostSummaryCardsProps {
  periodTotals: PeriodTotals;
  isLoading: boolean;
}

export function CostSummaryCards({ periodTotals, isLoading }: CostSummaryCardsProps) {
  if (isLoading) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        {Array.from({ length: 5 }).map((_, i) => (
          <Card key={i}>
            <CardContent className="p-4">
              <Loader message="Loading..." />
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
            <Activity className="w-4 h-4" />
            Total Tokens
          </div>
          <div className="text-2xl font-bold">{formatTokens(periodTotals.tokens)}</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
            <ArrowUpRight className="w-4 h-4 text-blue-500" />
            Input Tokens
          </div>
          <div className="text-2xl font-bold">{formatTokens(periodTotals.input)}</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
            <ArrowDownRight className="w-4 h-4 text-green-500" />
            Output Tokens
          </div>
          <div className="text-2xl font-bold">{formatTokens(periodTotals.output)}</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
            <Coins className="w-4 h-4 text-yellow-500" />
            Total Cost
          </div>
          <div className="text-2xl font-bold">{formatCost(periodTotals.cost)}</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
            <Cpu className="w-4 h-4 text-purple-500" />
            Requests
          </div>
          <div className="text-2xl font-bold">{periodTotals.requests.toLocaleString()}</div>
        </CardContent>
      </Card>
    </div>
  );
}
