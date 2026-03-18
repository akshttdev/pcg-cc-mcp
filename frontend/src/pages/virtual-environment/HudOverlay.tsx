import { useMemo } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { cn } from '@/lib/utils';
import { AgentChatConsole } from '@/components/nora/AgentChatConsole';
import { InventoryPanel, EquipmentPanel } from '@/components/vibeland/hud';
import { HUD_NAV_ITEMS, HUD_PANEL_META } from './constants';
import { MiniMap } from './MiniMap';
import { SystemsPanel, IntelPanel, MapPanel, ControlsPanel } from './HudPanels';
import type { ProjectData, HudPanelId, VirtualZone } from './types';

interface HudOverlayProps {
  projects: ProjectData[];
  selectedProject: ProjectData | null;
  userPosition: [number, number, number];
  worldZones: VirtualZone[];
  noraLine: string;
  noraStatusVersion: number;
  isConsoleInputActive: boolean;
  consoleFocusVersion: number;
  isChatCollapsed: boolean;
  activeHudPanel: HudPanelId | null;
  onToggleChatCollapse: () => void;
  onReleaseConsoleInput: () => void;
  onSetActiveHudPanel: (panel: HudPanelId | null) => void;
  onToggleHudPanel: (panel: HudPanelId) => void;
}

export function HudOverlay({
  projects,
  selectedProject,
  userPosition,
  worldZones,
  noraLine,
  noraStatusVersion,
  isConsoleInputActive,
  consoleFocusVersion,
  isChatCollapsed,
  activeHudPanel,
  onToggleChatCollapse,
  onReleaseConsoleInput,
  onSetActiveHudPanel,
  onToggleHudPanel,
}: HudOverlayProps) {
  const hudPanelContent = useMemo(() => {
    if (!activeHudPanel) return null;
    switch (activeHudPanel) {
      case 'systems':
        return (
          <SystemsPanel
            projects={projects}
            selectedProject={selectedProject}
            userPosition={userPosition}
            zones={worldZones}
          />
        );
      case 'intel':
        return <IntelPanel projects={projects} />;
      case 'map':
        return (
          <MapPanel
            projects={projects}
            selectedProject={selectedProject}
            userPosition={userPosition}
            zones={worldZones}
          />
        );
      case 'controls':
        return <ControlsPanel />;
      case 'inventory':
        return <InventoryPanel />;
      case 'equipment':
        return <EquipmentPanel />;
      default:
        return null;
    }
  }, [activeHudPanel, projects, selectedProject, userPosition, worldZones]);

  return (
    <>
      {/* Mini-map (top-right) */}
      <div className="pointer-events-auto absolute top-4 right-4 w-[min(20rem,calc(100%-2rem))]">
        <div className="rounded-2xl border border-amber-500/30 bg-[#050403]/90 p-3 backdrop-blur-sm shadow-[0_8px_30px_rgba(0,0,0,0.5)]">
          <MiniMap
            projects={projects}
            selectedProject={selectedProject}
            userPosition={userPosition}
            size={220}
            zones={worldZones}
          />
        </div>
      </div>

      {/* Chat console (bottom-left) */}
      <div className="pointer-events-auto absolute bottom-4 left-4 w-[min(30rem,calc(100%-2rem))]">
        <div className="rounded-lg border border-amber-600/60 bg-[#1b1209]/90 shadow-[0_12px_40px_rgba(0,0,0,0.6)]">
          <div className="flex items-center justify-between border-b border-amber-500/40 px-3 py-1 text-[11px] uppercase tracking-[0.3em] text-amber-200">
            <div className="flex flex-wrap items-center gap-2 font-semibold">
              {['All', 'Grid', 'Direct', 'System'].map((label) => (
                <span
                  key={label}
                  className="rounded border border-amber-500/50 bg-black/30 px-2 py-0.5 text-[10px] tracking-[0.2em]"
                >
                  {label}
                </span>
              ))}
            </div>
            <button
              type="button"
              onClick={onToggleChatCollapse}
              className="rounded border border-amber-600/50 bg-black/30 p-1 text-amber-200 transition hover:text-white"
            >
              {isChatCollapsed ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
            </button>
          </div>
          <div className="border-b border-amber-500/30 px-3 py-2 text-[11px] text-amber-100/80">
            {noraLine}
          </div>
          <div
            className={cn(
              'overflow-hidden transition-all duration-300',
              isChatCollapsed
                ? 'pointer-events-none max-h-0 opacity-0'
                : 'max-h-[32rem] opacity-100'
            )}
          >
            <AgentChatConsole
              className="rounded-none border-0 bg-[#0b0905]/90 text-[13px] text-amber-100"
              statusLine={noraLine}
              statusVersion={noraStatusVersion}
              selectedProject={selectedProject}
              isInputActive={isConsoleInputActive}
              onRequestCloseInput={onReleaseConsoleInput}
              focusToken={consoleFocusVersion}
              showHeader={false}
              projectId={selectedProject?.id}
            />
          </div>
          <div className="border-t border-amber-500/30 px-3 py-1 text-[10px] text-amber-200/80">
            {isChatCollapsed ? 'Press Enter to reopen the command net.' : 'Enter engages the net \u00B7 Esc cancels typing'}
          </div>
        </div>
      </div>

      {/* HUD panels and nav bar (bottom-right) */}
      <div className="pointer-events-auto absolute bottom-4 right-4 flex flex-col items-end gap-3">
        {activeHudPanel && (
          <div className="w-[min(34rem,calc(100%-2rem))] rounded-2xl border border-amber-500/60 bg-[#080705]/95 shadow-[0_20px_45px_rgba(0,0,0,0.65)]">
            <div className="flex items-center justify-between border-b border-amber-500/40 px-5 py-3 text-[11px] uppercase tracking-[0.3em] text-amber-100">
              <div>
                <p className="text-sm font-semibold tracking-[0.2em]">{HUD_PANEL_META[activeHudPanel].title}</p>
                <p className="text-[10px] tracking-[0.15em] text-amber-200/70">
                  {HUD_PANEL_META[activeHudPanel].description}
                </p>
              </div>
              <button
                type="button"
                onClick={() => onSetActiveHudPanel(null)}
                className="rounded border border-amber-500/40 px-3 py-1 text-[10px] tracking-[0.2em] text-amber-100 transition hover:bg-amber-500/20"
              >
                CLOSE
              </button>
            </div>
            <div className="p-5 text-sm text-amber-100/90">{hudPanelContent}</div>
          </div>
        )}
        <div className="flex items-end gap-2 rounded-full border border-amber-600/60 bg-[#14100b]/95 px-4 py-2 shadow-[0_8px_30px_rgba(0,0,0,0.55)]">
          {HUD_NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const isActive = activeHudPanel === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onToggleHudPanel(item.id)}
                className={cn(
                  'flex flex-col items-center gap-1 rounded-md px-3 py-1 text-[10px] tracking-[0.2em] transition',
                  isActive
                    ? 'bg-amber-500/30 text-amber-50'
                    : 'text-amber-200/80 hover:bg-amber-500/10'
                )}
              >
                <Icon className="h-4 w-4" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </div>
      </div>
    </>
  );
}
