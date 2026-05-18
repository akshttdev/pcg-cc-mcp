import { useQuery } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
  ChevronDown,
  ChevronRight,
  Code2,
  ExternalLink,
  GitBranch,
} from 'lucide-react';
import { useState } from 'react';

import { auriApi, type AuriTaskLog, type AuriTaskSummary } from '@/lib/api';

const STATUS_STYLES: Record<string, string> = {
  inprogress: 'bg-amber-900 text-amber-300',
  done: 'bg-green-900 text-green-300',
  cancelled: 'bg-red-900 text-red-400',
  todo: 'bg-gray-800 text-gray-400',
};

function StatusBadge({ status }: { status: string | null }) {
  const s = status ?? 'todo';
  return (
    <span
      className={`text-xs px-2 py-0.5 rounded font-medium shrink-0 ${STATUS_STYLES[s] ?? 'bg-gray-800 text-gray-400'}`}
    >
      {s === 'inprogress' ? 'running' : s}
    </span>
  );
}

function LogPanel({ taskId }: { taskId: string }) {
  const { data, isLoading } = useQuery<AuriTaskLog>({
    queryKey: ['auri-log', taskId],
    queryFn: () => auriApi.getLog(taskId),
    refetchInterval: 5000,
  });

  if (isLoading)
    return <p className="text-xs text-gray-500 p-3">Loading log…</p>;
  if (!data?.log)
    return <p className="text-xs text-gray-500 p-3">No log yet.</p>;

  return (
    <pre className="text-xs text-gray-300 bg-gray-900 rounded p-3 overflow-auto max-h-64 whitespace-pre-wrap font-mono leading-relaxed">
      {data.log}
    </pre>
  );
}

function TaskRow({ task }: { task: AuriTaskSummary }) {
  const [expanded, setExpanded] = useState(false);

  const repoShort = task.github_repo?.split('/').slice(-2).join('/') ?? '—';
  const updatedAt = task.updated_at ?? task.created_at;

  return (
    <div className="border border-gray-800 rounded-lg bg-gray-900/60 overflow-hidden">
      <button
        onClick={() => setExpanded((v) => !v)}
        className="w-full text-left p-4 flex items-start gap-3 hover:bg-gray-800/40 transition-colors"
      >
        <span className="mt-0.5 text-gray-600 shrink-0">
          {expanded ? (
            <ChevronDown className="h-4 w-4" />
          ) : (
            <ChevronRight className="h-4 w-4" />
          )}
        </span>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1 flex-wrap">
            <StatusBadge status={task.status} />
            <span className="text-sm font-medium text-white truncate">
              {task.title}
            </span>
          </div>
          <div className="flex items-center gap-4 text-xs text-gray-500 flex-wrap">
            <span className="flex items-center gap-1">
              <Code2 className="h-3 w-3" />
              {repoShort}
            </span>
            {task.github_branch && (
              <span className="flex items-center gap-1">
                <GitBranch className="h-3 w-3" />
                {task.github_branch}
              </span>
            )}
            {updatedAt && (
              <span>
                {formatDistanceToNow(new Date(updatedAt), { addSuffix: true })}
              </span>
            )}
          </div>
        </div>

        {task.auri_pr_url && (
          <a
            href={task.auri_pr_url}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(e) => e.stopPropagation()}
            className="flex items-center gap-1 text-xs text-violet-400 hover:text-violet-300 shrink-0 mt-0.5"
          >
            View PR <ExternalLink className="h-3 w-3" />
          </a>
        )}
      </button>

      {expanded && (
        <div className="px-4 pb-4 border-t border-gray-800">
          <p className="text-xs text-gray-500 mb-2 mt-3 font-medium uppercase tracking-wide">
            Execution Log
          </p>
          <LogPanel taskId={task.id} />
        </div>
      )}
    </div>
  );
}

export function AuriTasksPage() {
  const [filter, setFilter] = useState<
    'all' | 'inprogress' | 'done' | 'cancelled'
  >('all');

  const { data: all = [], isLoading } = useQuery<AuriTaskSummary[]>({
    queryKey: ['auri-tasks'],
    queryFn: auriApi.listTasks,
    refetchInterval: 8000,
  });

  const tasks = filter === 'all' ? all : all.filter((t) => t.status === filter);

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 p-6">
      <div className="max-w-4xl mx-auto">
        <div className="mb-6">
          <div className="flex items-center gap-2 mb-1">
            <Code2 className="h-5 w-5 text-violet-400" />
            <h1 className="text-xl font-semibold text-white">Auri Tasks</h1>
          </div>
          <p className="text-sm text-gray-400">
            AI-executed coding tasks assigned by Nora
          </p>
        </div>

        <div className="flex gap-2 mb-5">
          {(['all', 'inprogress', 'done', 'cancelled'] as const).map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                filter === f
                  ? 'bg-violet-600 text-white'
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
              }`}
            >
              {f === 'inprogress'
                ? 'Running'
                : f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>

        {isLoading ? (
          <div className="text-sm text-gray-500 py-8 text-center">
            Loading tasks…
          </div>
        ) : tasks.length === 0 ? (
          <div className="text-sm text-gray-500 py-16 text-center">
            {filter === 'all'
              ? 'No Auri tasks yet. Ask Nora to assign a coding task to Auri.'
              : `No ${filter} tasks.`}
          </div>
        ) : (
          <div className="space-y-3">
            {tasks.map((t) => (
              <TaskRow key={t.id} task={t} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
