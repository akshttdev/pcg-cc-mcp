import { Calendar } from 'lucide-react';

export function DueDateBadge({ dueDate }: { dueDate: string }) {
  const due = new Date(dueDate);
  const now = new Date();
  const diffDays = Math.ceil((due.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

  let colorClasses: string;
  if (diffDays < 0) {
    colorClasses = 'bg-red-100 text-red-700 dark:bg-red-950/50 dark:text-red-400 border-red-200 dark:border-red-800';
  } else if (diffDays <= 2) {
    colorClasses = 'bg-amber-100 text-amber-700 dark:bg-amber-950/50 dark:text-amber-400 border-amber-200 dark:border-amber-800';
  } else {
    colorClasses = 'bg-blue-50 text-blue-600 dark:bg-blue-950/40 dark:text-blue-400 border-blue-200 dark:border-blue-800';
  }

  const label = diffDays < 0
    ? `${Math.abs(diffDays)}d overdue`
    : diffDays === 0
    ? 'Today'
    : diffDays === 1
    ? 'Tomorrow'
    : `${due.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}`;

  return (
    <span
      className={`inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium border ${colorClasses}`}
      title={`Due: ${due.toLocaleDateString()}`}
    >
      <Calendar className="h-2.5 w-2.5" />
      <span>{label}</span>
    </span>
  );
}
