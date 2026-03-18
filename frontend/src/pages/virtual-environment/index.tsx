import { Suspense } from 'react';
import { Canvas } from '@react-three/fiber';
import { Grid, Environment, Stars, SpotLight } from '@react-three/drei';
import * as THREE from 'three';
import { Loader2 } from 'lucide-react';
import { CommandCenter } from '@/components/vibeland/CommandCenter';
import { NoraAvatar } from '@/components/vibeland/NoraAvatar';
import { WanderingAgent } from '@/components/vibeland/WanderingAgent';
import { ToposDataSphere } from '@/components/vibeland/ToposDataSphere';
import { UserAvatar } from '@/components/vibeland/UserAvatar';
import { MultiplayerManager } from '@/components/vibeland/MultiplayerManager';
import { AgentWorkspaceLevel, getAgentBayBounds } from '@/components/vibeland/AgentWorkspaceLevel';
import { SpiralStaircase } from '@/components/vibeland/SpiralStaircase';
import { OrchaAvatar } from '@/components/vibeland/OrchaAvatar';
import { ProjectBuilding } from '@/components/vibeland/ProjectBuilding';

import { COMMAND_CENTER_FLOOR_Y, PLAYER_COLOR, toposDirectoryItems } from './constants';
import { AtmosphericLighting, EnhancedParticles } from './SceneElements';
import { HudOverlay } from './HudOverlay';
import { ZoneEntryPrompt, ZoneSidePanel } from './ZonePanel';
import { useVirtualEnvironment } from './useVirtualEnvironment';

export function VirtualEnvironmentPage() {
  const env = useVirtualEnvironment();

  return (
    <div className="relative h-full min-h-[calc(100vh-6rem)] bg-black text-white">
      {/* DEBUG OVERLAY */}
      <div style={{position:'fixed',top:8,left:'50%',transform:'translateX(-50%)',zIndex:99999,background:'rgba(0,0,0,0.85)',color:'#0ff',padding:'4px 14px',borderRadius:6,fontSize:11,fontFamily:'monospace',pointerEvents:'none',whiteSpace:'nowrap'}}>
        key: <b>{env.debugLastKey}</b> ({env.debugKeyCount}) | suspended: <b style={{color: env.isConsoleInputActive ? '#f55' : '#0f0'}}>{env.isConsoleInputActive ? 'YES' : 'no'}</b>
      </div>

      {/* Loading overlay */}
      {env.projectsLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/80">
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-12 w-12 animate-spin text-cyan-400" />
            <p className="text-lg text-cyan-100">Syncing with Command Center...</p>
          </div>
        </div>
      )}

      {/* Non-blocking error banner */}
      {env.projectsError && (
        <div className="absolute top-2 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 px-4 py-2 rounded-lg bg-black/80 border border-red-500/40 text-sm">
          <span className="text-red-400">Projects unavailable</span>
          <button
            onClick={() => env.refetchProjects()}
            className="text-xs text-red-300/70 hover:text-red-200 underline"
          >
            Retry
          </button>
        </div>
      )}

      <Canvas
        camera={{ position: [80, 60, 80], fov: 60 }}
        shadows
        gl={{ antialias: true, toneMapping: THREE.ACESFilmicToneMapping }}
        onPointerDown={env.releaseConsoleInput}
      >
        <color attach="background" args={['#030508']} />
        <fog attach="fog" args={['#030508', 150, 600]} />

        <Suspense fallback={null}>
          <AtmosphericLighting />
          <Environment preset="night" />
          <Stars radius={600} depth={60} count={3000} factor={4} saturation={0} fade speed={0.5} />
          <Grid
            args={[400, 400]}
            position={[0, 0, 0]}
            cellSize={2}
            sectionSize={10}
            infiniteGrid
            fadeDistance={200}
            fadeStrength={3}
            cellColor="#0a2740"
            sectionColor="#0df2ff"
          />
          <EnhancedParticles />
          <SpotLight
            position={[0, 120, 0]}
            angle={0.6}
            penumbra={0.5}
            intensity={2}
            color="#00ffff"
            distance={200}
            castShadow
            volumetric
            opacity={0.15}
          />
          <ToposDataSphere
            toposItems={toposDirectoryItems}
            commandCenterHeight={COMMAND_CENTER_FLOOR_Y}
          />
          <CommandCenter />
          <AgentWorkspaceLevel />
          <SpiralStaircase />
          <NoraAvatar position={[0, COMMAND_CENTER_FLOOR_Y + 2, 0]} />
          <WanderingAgent name="Maci" role="cinematographer" bayBounds={getAgentBayBounds('Maci') || undefined} />
          <WanderingAgent name="Editron" role="editor" bayBounds={getAgentBayBounds('Editron') || undefined} />
          <WanderingAgent name="Bowser" role="browser" bayBounds={getAgentBayBounds('Bowser') || undefined} />
          <WanderingAgent name="Auri" role="oracle" bayBounds={getAgentBayBounds('Auri') || undefined} />

          {env.zoneBuildings.map(({ zone, position, facingAngle, accessible }) => (
            <group key={zone.space_name} position={position} rotation={[0, facingAngle, 0]}>
              <ProjectBuilding
                name={zone.space_name}
                position={[0, 0, 0]}
                energy={0.85}
                isSelected={false}
                onSelect={() => {}}
                isEnterTarget={env.enterZoneTarget?.space_name === zone.space_name}
                entryHotkey="E"
                locked={!accessible}
              />
            </group>
          ))}

          <UserAvatar
            initialPosition={env.spawnPosition}
            color={PLAYER_COLOR}
            isAdmin={env.isAdmin}
            onPositionChange={env.handleUserPositionChange}
            onInteract={env.handleAttemptEnter}
            isSuspended={env.isConsoleInputActive}
            canFly={env.isAdmin}
          />

          {env.isAdmin && <OrchaAvatar userPosition={env.userPosition} />}
          <MultiplayerManager />
        </Suspense>
      </Canvas>

      <HudOverlay
        projects={env.projects}
        selectedProject={env.selectedProject}
        userPosition={env.userPosition}
        worldZones={env.worldZones}
        noraLine={env.noraLine}
        noraStatusVersion={env.noraStatusVersion}
        isConsoleInputActive={env.isConsoleInputActive}
        consoleFocusVersion={env.consoleFocusVersion}
        isChatCollapsed={env.isChatCollapsed}
        activeHudPanel={env.activeHudPanel}
        onToggleChatCollapse={env.toggleChatCollapse}
        onReleaseConsoleInput={env.releaseConsoleInput}
        onSetActiveHudPanel={env.setActiveHudPanel}
        onToggleHudPanel={env.toggleHudPanel}
      />

      {/* Zone entry prompt */}
      {!env.activeZone && env.enterZoneTarget && (
        <ZoneEntryPrompt enterZoneTarget={env.enterZoneTarget} />
      )}

      {/* Zone panel */}
      {env.activeZone && (
        <ZoneSidePanel
          activeZone={env.activeZone}
          allProjects={env.allProjects}
          onClose={env.closeZonePanel}
        />
      )}
    </div>
  );
}

export default VirtualEnvironmentPage;
