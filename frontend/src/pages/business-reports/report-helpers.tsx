import React from 'react';

// ── Helpers ───────────────────────────────────────────────────────────────────

export function parseJson<T>(str: string | undefined | null, fallback: T): T {
  if (!str) return fallback;
  try { return JSON.parse(str); } catch { return fallback; }
}

export function fmtDate(dt: string | null | undefined) {
  if (!dt) return '\u2014';
  return new Date(dt).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

export function severityColor(s: string) {
  if (s === 'high') return 'text-red-400 bg-red-950/40 border-red-900';
  if (s === 'medium') return 'text-amber-400 bg-amber-950/40 border-amber-900';
  return 'text-blue-400 bg-blue-950/40 border-blue-900';
}

export function priorityDot(p: string) {
  if (p === 'high') return 'bg-red-500';
  if (p === 'medium') return 'bg-amber-500';
  return 'bg-blue-500';
}

export function threatBadge(t: string) {
  if (t === 'high') return 'border-red-800 text-red-400';
  if (t === 'medium') return 'border-amber-800 text-amber-400';
  return 'border-slate-700 text-slate-400';
}

// ── Sub-components ─────────────────────────────────────────────────────────────

export function SectionHeader({ icon: Icon, title, action }: {
  icon: React.ElementType;
  title: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between mb-5">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center">
          <Icon className="w-4 h-4 text-indigo-400" />
        </div>
        <h2 className="text-sm font-semibold uppercase tracking-widest text-slate-400">{title}</h2>
      </div>
      {action}
    </div>
  );
}

export function IntelCard({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-slate-800 bg-slate-900/60 p-6 ${className}`}>
      {children}
    </div>
  );
}
