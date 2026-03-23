import { useMemo } from 'react';
import { STATIC_ZONES } from './constants';
import { MiniMap } from './MiniMap';
import type { SystemsPanelProps, IntelPanelProps, MapPanelProps } from './types';

export function SystemsPanel({ projects, selectedProject, userPosition, zones = STATIC_ZONES }: SystemsPanelProps) {
  const [userX, , userZ] = userPosition;
  const topProject = projects.length
    ? [...projects].sort((a, b) => b.energy - a.energy)[0]
    : null;
  const distanceFromCenter = Math.hypot(userX, userZ);

  return (
    <div className="flex flex-col gap-4 lg:flex-row">
      <div className="flex-1 space-y-3">
        <div className="grid grid-cols-2 gap-3 rounded-lg border border-amber-500/20 bg-black/30 p-4 text-xs text-amber-100/80">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">Structures</p>
            <p className="text-2xl font-semibold text-white">{projects.length}</p>
            <p className="text-xs text-amber-200/60">Deployed across grid</p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">Active Zones</p>
            <p className="text-2xl font-semibold text-white">{zones.length + 1}</p>
            <p className="text-xs text-amber-200/60">Including PCG Command Center</p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">Command Center</p>
            <p className="text-lg font-semibold text-green-300">&#x25CF; Operational</p>
            <p className="text-xs text-amber-200/60">Core services nominal</p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">Pilot Position</p>
            <p className="text-lg font-semibold text-white">{distanceFromCenter.toFixed(0)}m from core</p>
            <p className="text-xs text-amber-200/60">Warden altitude stable</p>
          </div>
        </div>

        <div className="rounded-lg border border-amber-500/20 bg-black/40 p-4 text-xs text-amber-100/80">
          <p className="mb-2 text-xs uppercase tracking-[0.3em] text-amber-200/70">Live status feed</p>
          <ul className="space-y-1">
            <li>&#x2022; Grid integrity holding at 100%.</li>
            <li>
              &#x2022; Nora channel {selectedProject ? `linked to ${selectedProject.name}.` : 'idle and awaiting directive.'}
            </li>
            {topProject && (
              <li>
                &#x2022; {topProject.name} broadcasting strongest signal at {(topProject.energy * 100).toFixed(1)}%.
              </li>
            )}
          </ul>
        </div>
      </div>

      <div className="w-full shrink-0 lg:w-64">
        <MiniMap projects={projects} selectedProject={selectedProject} userPosition={userPosition} zones={zones} size={200} />
      </div>
    </div>
  );
}

export function IntelPanel({ projects }: IntelPanelProps) {
  if (!projects.length) {
    return <p className="text-sm text-amber-200/80">No structures are synced with this environment yet.</p>;
  }

  const ranked = [...projects].sort((a, b) => b.energy - a.energy).slice(0, 8);

  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2">
        {ranked.map((project, index) => (
          <div key={project.name} className="rounded-lg border border-amber-500/20 bg-black/30 p-3">
            <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">#{index + 1}</p>
            <p className="text-base font-semibold text-white">{project.name}</p>
            <p className="text-xs text-amber-100/70">Energy {(project.energy * 100).toFixed(1)}%</p>
            <p className="text-xs text-amber-100/60">Status: {project.energy > 0.65 ? 'Prime' : 'Stable'}</p>
          </div>
        ))}
      </div>
      <p className="text-xs text-amber-200/70">
        Rankings update live as MCP worktrees spin up or wind down.
      </p>
    </div>
  );
}

export function MapPanel({ projects, selectedProject, userPosition, zones = STATIC_ZONES }: MapPanelProps) {
  const [userX, , userZ] = userPosition;
  const closestProject = useMemo(() => {
    if (!projects.length) return null;
    return projects.reduce<null | { project: (typeof projects)[number]; distance: number }>((closest, project) => {
      const distance = Math.hypot(project.position[0] - userX, project.position[2] - userZ);
      if (!closest || distance < closest.distance) {
        return { project, distance };
      }
      return closest;
    }, null);
  }, [projects, userX, userZ]);

  const distanceFromCenter = Math.hypot(userX, userZ);

  return (
    <div className="flex flex-col gap-4 lg:flex-row">
      <div className="w-full shrink-0 lg:w-80">
        <MiniMap
          projects={projects}
          selectedProject={selectedProject}
          userPosition={userPosition}
          size={320}
          zones={zones}
        />
      </div>
      <div className="flex-1 space-y-3 text-sm text-amber-100/80">
        <div className="rounded-lg border border-amber-500/20 bg-black/30 p-4">
          <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">Navigator</p>
          <p className="text-lg font-semibold text-white">{distanceFromCenter.toFixed(0)}m from command core</p>
          <p className="text-xs text-amber-100/70">
            Hover vector ready. Use WASD + Q/E to strafe above the ring of structures.
          </p>
        </div>
        <div className="rounded-lg border border-amber-500/20 bg-black/30 p-4">
          <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">Nearest signal</p>
          {closestProject ? (
            <>
              <p className="text-lg font-semibold text-white">{closestProject.project.name}</p>
              <p className="text-xs text-amber-100/70">
                {closestProject.distance.toFixed(1)}m away &middot; Energy {(closestProject.project.energy * 100).toFixed(1)}%
              </p>
            </>
          ) : (
            <p className="text-xs text-amber-100/70">No structures detected on this shard yet.</p>
          )}
        </div>
        <p className="text-xs text-amber-200/70">
          Tip: engage the Systems tab to pin stats, then keep this map floating for quick orientation
          during flyovers.
        </p>
      </div>
    </div>
  );
}

export function ControlsPanel() {
  const bindings = [
    { action: 'W / A / S / D', detail: 'Strafe across the grid' },
    { action: 'Shift', detail: 'Sprint burst' },
    { action: 'Space / Ctrl', detail: 'Ascend / descend' },
    { action: 'Mouse', detail: 'Look around' },
    { action: 'E', detail: 'Enter highlighted structure' },
    { action: 'Enter', detail: 'Toggle command net' },
    { action: 'Esc', detail: 'Exit typing / interiors' },
    { action: '/nora', detail: 'Direct Nora instruction' },
  ];

  return (
    <div className="grid gap-4 text-sm text-amber-100/80 sm:grid-cols-2">
      {bindings.map((binding) => (
        <div key={binding.action} className="rounded-lg border border-amber-500/20 bg-black/30 p-4">
          <p className="text-xs uppercase tracking-[0.3em] text-amber-200/70">{binding.action}</p>
          <p className="text-base font-semibold text-white">{binding.detail}</p>
        </div>
      ))}
      <p className="sm:col-span-2 text-xs text-amber-200/70">
        Slash shortcuts: <span className="font-mono">/global</span>, <span className="font-mono">/help</span>, or <span className="font-mono">/agent</span> mirror MMO chat conventions.
      </p>
    </div>
  );
}
