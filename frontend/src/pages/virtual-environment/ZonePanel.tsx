import type { Project } from 'shared/types';
import type { VirtualZone } from './types';

interface ZoneEntryPromptProps {
  enterZoneTarget: VirtualZone;
}

export function ZoneEntryPrompt({ enterZoneTarget }: ZoneEntryPromptProps) {
  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-28 flex justify-center">
      <div
        className="rounded-full px-6 py-2 text-xs uppercase tracking-[0.4em]"
        style={{
          border: `1px solid ${enterZoneTarget.color}66`,
          backgroundColor: 'rgba(0,0,0,0.7)',
          color: enterZoneTarget.color,
        }}
      >
        Press <span className="mx-1 font-semibold text-white">E</span> to open {enterZoneTarget.space_name}
      </div>
    </div>
  );
}

interface ZoneSidePanelProps {
  activeZone: VirtualZone;
  allProjects: Project[];
  onClose: () => void;
}

export function ZoneSidePanel({ activeZone, allProjects, onClose }: ZoneSidePanelProps) {
  const matchingProject = allProjects.find(p => p.name === activeZone.space_name);

  return (
    <div className="pointer-events-auto absolute inset-y-0 right-0 flex w-[min(28rem,100%)] flex-col border-l border-amber-500/30 bg-[#08060a]/95 backdrop-blur-md shadow-[-20px_0_60px_rgba(0,0,0,0.7)]">
      {/* Header */}
      <div
        className="flex items-center justify-between border-b border-amber-500/20 px-6 py-4"
        style={{ borderBottomColor: `${activeZone.color}33` }}
      >
        <div className="flex items-center gap-3">
          <div className="h-3 w-3 rounded-full" style={{ backgroundColor: activeZone.color, boxShadow: `0 0 8px ${activeZone.color}` }} />
          <div>
            <p className="text-base font-semibold tracking-wide text-white">{activeZone.space_name}</p>
            <p className="text-xs tracking-[0.2em] text-amber-200/60">HOST: {activeZone.host_username.toUpperCase()}</p>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="rounded border border-amber-500/30 px-3 py-1 text-xs tracking-[0.2em] text-amber-200/80 transition hover:bg-amber-500/20 hover:text-white"
        >
          ESC / CLOSE
        </button>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4">
        <p className="text-xs tracking-[0.15em] text-amber-200/50 uppercase">Virtual Space</p>
        <p className="text-sm text-amber-100/80 leading-relaxed">
          You are at the entrance of <span className="text-white font-medium">{activeZone.space_name}</span>.
          Navigate to the project workspace to collaborate with the team.
        </p>

        {matchingProject ? (
          <a
            href={`/projects/${matchingProject.id}`}
            className="flex items-center justify-between rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-100 transition hover:bg-amber-500/20 hover:text-white"
            style={{ borderColor: `${activeZone.color}44` }}
          >
            <span className="tracking-[0.1em]">Open Project Workspace</span>
            <span className="text-amber-400">&rarr;</span>
          </a>
        ) : (
          <div className="rounded-lg border border-amber-500/20 bg-black/30 px-4 py-3 text-xs text-amber-200/50 tracking-[0.1em]">
            Project workspace not linked
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="border-t border-amber-500/20 px-6 py-3 text-xs tracking-[0.2em] text-amber-200/40">
        YOU REMAIN IN THE GLOBAL ENVIRONMENT &middot; ESC TO CLOSE
      </div>
    </div>
  );
}
