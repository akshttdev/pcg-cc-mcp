import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Coins,
  TrendingUp,
  Cpu,
  CircleDollarSign,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TaskWithAttemptStatus } from 'shared/types';

interface VibeBalance {
  total_deposited: number;
  total_withdrawn: number;
  total_spent: number;
  available_balance: number;
}

interface VibeTransaction {
  id: string;
  amount_vibe: number;
  model: string | null;
  description: string | null;
  task_id: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  created_at: string;
}

interface VibeBudgetTabProps {
  task: TaskWithAttemptStatus;
  vibeBalance: VibeBalance | null;
  vibeTransactions: VibeTransaction[];
  vibeLoading: boolean;
}

export function VibeBudgetTab({
  task,
  vibeBalance,
  vibeTransactions,
  vibeLoading,
}: VibeBudgetTabProps) {
  return (
    <ScrollArea className="h-full">
      <div className="p-4 space-y-4">
        {/* Task-level VIBE cost */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-lg border bg-card p-4 space-y-1">
            <div className="flex items-center gap-2 text-muted-foreground text-xs font-medium">
              <Coins className="h-3.5 w-3.5" />
              VIBE Spent (Task)
            </div>
            <div className="text-2xl font-semibold">
              {task.vibe_cost ? Number(task.vibe_cost).toLocaleString() : '0'}
            </div>
            <div className="text-xs text-muted-foreground">
              ≈ ${task.vibe_cost ? (Number(task.vibe_cost) / 100).toFixed(2) : '0.00'} USD
            </div>
          </div>
          <div className="rounded-lg border bg-card p-4 space-y-1">
            <div className="flex items-center gap-2 text-muted-foreground text-xs font-medium">
              <Cpu className="h-3.5 w-3.5" />
              Model
            </div>
            <div className="text-sm font-semibold truncate">
              {task.vibe_model ?? '—'}
            </div>
            <div className="text-xs text-muted-foreground">
              {task.assigned_agent ?? 'No agent'}
            </div>
          </div>
        </div>

        {/* Project VIBE balance */}
        <div>
          <h3 className="text-sm font-medium mb-2 flex items-center gap-2">
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
            Project Budget
          </h3>
          {vibeLoading ? (
            <div className="grid grid-cols-2 gap-3">
              <div className="h-20 rounded-lg border bg-muted animate-pulse" />
              <div className="h-20 rounded-lg border bg-muted animate-pulse" />
            </div>
          ) : vibeBalance ? (
            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-lg border bg-card p-3 space-y-1">
                <div className="text-xs text-muted-foreground">Available Balance</div>
                <div className={cn('text-xl font-semibold', vibeBalance.available_balance < 0 && 'text-destructive')}>
                  {vibeBalance.available_balance.toLocaleString()}
                </div>
                <div className="text-xs text-muted-foreground">VIBE</div>
              </div>
              <div className="rounded-lg border bg-card p-3 space-y-1">
                <div className="text-xs text-muted-foreground">Total Spent (Project)</div>
                <div className="text-xl font-semibold">
                  {vibeBalance.total_spent.toLocaleString()}
                </div>
                <div className="text-xs text-muted-foreground">VIBE</div>
              </div>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No budget data available.</p>
          )}
        </div>

        {/* Task transactions */}
        {vibeTransactions.length > 0 && (
          <div>
            <h3 className="text-sm font-medium mb-2 flex items-center gap-2">
              <CircleDollarSign className="h-4 w-4 text-muted-foreground" />
              Task Transactions ({vibeTransactions.length})
            </h3>
            <div className="space-y-2">
              {vibeTransactions.map((tx) => (
                <div key={tx.id} className="flex items-center justify-between rounded-md border px-3 py-2 text-sm">
                  <div className="min-w-0">
                    <div className="truncate text-xs font-medium">
                      {tx.description ?? tx.model ?? 'AI execution'}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {tx.model && <span className="mr-2">{tx.model}</span>}
                      {tx.input_tokens != null && tx.output_tokens != null && (
                        <span>{tx.input_tokens.toLocaleString()} in / {tx.output_tokens.toLocaleString()} out</span>
                      )}
                    </div>
                  </div>
                  <div className="shrink-0 ml-3 text-right">
                    <div className="font-semibold text-orange-500">{tx.amount_vibe.toLocaleString()} V</div>
                    <div className="text-xs text-muted-foreground">
                      ${(tx.amount_vibe / 100).toFixed(2)}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {!vibeLoading && vibeTransactions.length === 0 && (
          <p className="text-sm text-muted-foreground pt-2">No VIBE transactions recorded for this task yet.</p>
        )}
      </div>
    </ScrollArea>
  );
}
