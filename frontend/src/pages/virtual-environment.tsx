import { Suspense, useCallback, useEffect, useMemo, useState } from 'react';
import { Canvas } from '@react-three/fiber';
import { Grid, Environment, Stars } from '@react-three/drei';
import * as THREE from 'three';
import { CommandCenter } from '@/components/virtual-world/CommandCenter';
import { NoraAvatar } from '@/components/virtual-world/NoraAvatar';
import { ProjectBuilding } from '@/components/virtual-world/ProjectBuilding';
import { UserAvatar } from '@/components/virtual-world/UserAvatar';
import { BuildingInterior } from '@/components/virtual-world/BuildingInterior';
import { AgentChatConsole } from '@/components/nora/AgentChatConsole';
import { getBuildingType } from '@/lib/virtual-world/buildingTypes';
import { ENTRY_TRIGGER_DISTANCE } from '@/lib/virtual-world/constants';

const safeProjectList = (typeof __TOPOS_PROJECTS__ !== 'undefined'
  ? __TOPOS_PROJECTS__
  : []) as string[];

const PROJECT_HALF_WIDTH = 25;
const PROJECT_HALF_LENGTH = 50;
const PROJECT_FOOTPRINT_RADIUS = Math.sqrt(PROJECT_HALF_WIDTH ** 2 + PROJECT_HALF_LENGTH ** 2);
const COMMAND_CENTER_SPAWN_Y = 8;
const BASE_PROJECT_RADIUS = 220;
const TARGET_ARC_SPACING = PROJECT_FOOTPRINT_RADIUS * 2.2;

interface ProjectData {
  name: string;
  position: [number, number, number];
  energy: number;
}

const noraAcknowledgements = [
  'Routing orchestration energy to',
  'Illuminating systems for',
  'Calibrating pods against',
  'Summoning agents around',
  'Focusing the grid on',
  'Deploying sub-agents to',
  'Synchronizing timelines with',
  'Amplifying signal for',
];

const INITIAL_PLAYER_POSITION: [number, number, number] = [0, COMMAND_CENTER_SPAWN_Y, 0];

function stringEnergy(input: string) {
  let hash = 0;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash + input.charCodeAt(i) * (i + 11)) % 1000;
  }
  return 0.35 + (hash / 1000) * 0.65;
}

function generateProjects(names: string[]): ProjectData[] {
  const filtered = names.filter((name) => !name.startsWith('.'));
  if (!filtered.length) return [];

  const radiusForSpacing = (filtered.length * TARGET_ARC_SPACING) / (Math.PI * 2);
  const minVisualRadius = PROJECT_FOOTPRINT_RADIUS * 2.8;
  const radius = Math.max(BASE_PROJECT_RADIUS, minVisualRadius, radiusForSpacing);
  const y = 0; // Buildings rest on ground

  return filtered.map((name, index) => {
    const angle = (index / filtered.length) * Math.PI * 2;
    const position: [number, number, number] = [
      Math.cos(angle) * radius,
      y,
      Math.sin(angle) * radius,
    ];
    return {
      name,
      position,
      energy: stringEnergy(name),
    };
  });
}

function AtmosphericLighting() {
  return (
    <>
      {/* Hemisphere for ambient fill */}
      <hemisphereLight args={['#1d2a3f', '#000000', 0.4]} />

      {/* Directional moonlight */}
      <directionalLight
        position={[50, 100, 50]}
        intensity={0.5}
        color="#9db4ff"
        castShadow
        shadow-mapSize-width={2048}
        shadow-mapSize-height={2048}
        shadow-camera-far={500}
        shadow-camera-left={-200}
        shadow-camera-right={200}
        shadow-camera-top={200}
        shadow-camera-bottom={-200}
      />

      {/* Accent lights */}
      <pointLight position={[-100, 50, -100]} intensity={1} color="#ff8000" distance={200} decay={2} />
      <pointLight position={[100, 50, 100]} intensity={1} color="#0080ff" distance={200} decay={2} />
    </>
  );
}

function AmbientParticles() {
  const particleCount = 300;
  const positions = useMemo(() => {
    const pos = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount; i++) {
      // Spread particles across large area
      pos[i * 3] = (Math.random() - 0.5) * 400;
      pos[i * 3 + 1] = Math.random() * 100 + 10;
      pos[i * 3 + 2] = (Math.random() - 0.5) * 400;
    }
    return pos;
  }, []);

  return (
    <points>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={particleCount}
          array={positions}
          itemSize={3}
        />
      </bufferGeometry>
      <pointsMaterial
        size={0.3}
        color="#00ffff"
        transparent
        opacity={0.4}
        sizeAttenuation
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}

export function VirtualEnvironmentPage() {
  const projects = useMemo(() => generateProjects(safeProjectList), []);
  const [selectedProject, setSelectedProject] = useState<ProjectData | null>(null);
  const [noraLine, setNoraLine] = useState('Command Center online. Awaiting directive...');
  const [noraStatusVersion, setNoraStatusVersion] = useState(1);
  const [userPosition, setUserPosition] = useState<[number, number, number]>(INITIAL_PLAYER_POSITION);
  const [activeInterior, setActiveInterior] = useState<ProjectData | null>(null);
  const [isConsoleInputActive, setIsConsoleInputActive] = useState(false);
  const [consoleFocusVersion, setConsoleFocusVersion] = useState(0);

  const updateNoraLine = useCallback((line: string) => {
    setNoraLine(line);
    setNoraStatusVersion((prev) => prev + 1);
  }, []);

  const bumpConsoleFocus = useCallback(() => {
    setConsoleFocusVersion((prev) => prev + 1);
  }, []);

  const activateConsoleInput = useCallback(() => {
    if (activeInterior) return;
    setIsConsoleInputActive(true);
    bumpConsoleFocus();
  }, [activeInterior, bumpConsoleFocus]);

  const releaseConsoleInput = useCallback(() => {
    setIsConsoleInputActive(false);
  }, []);

  const handleSelect = useCallback((project: ProjectData) => {
    setSelectedProject(project);
    const line = noraAcknowledgements[
      Math.floor(Math.random() * noraAcknowledgements.length)
    ];
    updateNoraLine(`${line} ${project.name}.`);
  }, [updateNoraLine]);

  const handleUserPositionChange = useCallback((vector: THREE.Vector3) => {
    setUserPosition([vector.x, vector.y, vector.z]);
  }, []);

  const enterTarget = useMemo(() => {
    if (activeInterior) return null;

    let closest: { project: ProjectData; distance: number } | null = null;
    for (const project of projects) {
      const dx = project.position[0] - userPosition[0];
      const dz = project.position[2] - userPosition[2];
      const distance = Math.hypot(dx, dz);
      if (distance > ENTRY_TRIGGER_DISTANCE) continue;
      if (!closest || distance < closest.distance) {
        closest = { project, distance };
      }
    }
    return closest?.project ?? null;
  }, [activeInterior, projects, userPosition]);

  const handleAttemptEnter = useCallback(() => {
    if (activeInterior || !enterTarget) return;
    handleSelect(enterTarget);
    setActiveInterior(enterTarget);
    updateNoraLine(`Opening interior for ${enterTarget.name}. Agents syncing.`);
  }, [activeInterior, enterTarget, handleSelect, updateNoraLine]);

  const exitInterior = useCallback(() => {
    if (activeInterior) {
      updateNoraLine(`Grid perspective restored. ${activeInterior.name} interior sealed.`);
    }
    setActiveInterior(null);
  }, [activeInterior, updateNoraLine]);

  useEffect(() => {
    if (!activeInterior) return undefined;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' || event.key.toLowerCase() === 'q') {
        exitInterior();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeInterior, exitInterior]);

  useEffect(() => {
    const handleConsoleToggle = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const tag = target?.tagName.toLowerCase();
      const isTypingTarget =
        tag === 'input' ||
        tag === 'textarea' ||
        tag === 'select' ||
        target?.isContentEditable;

      if (isTypingTarget) {
        return;
      }

      if (event.key === 'Enter' && !event.repeat) {
        event.preventDefault();
        activateConsoleInput();
      }

      if (event.key === 'Escape' && isConsoleInputActive) {
        event.preventDefault();
        releaseConsoleInput();
      }
    };

    window.addEventListener('keydown', handleConsoleToggle);
    return () => window.removeEventListener('keydown', handleConsoleToggle);
  }, [activateConsoleInput, isConsoleInputActive, releaseConsoleInput]);

  useEffect(() => {
    if (activeInterior && isConsoleInputActive) {
      setIsConsoleInputActive(false);
    }
  }, [activeInterior, isConsoleInputActive]);

  return (
    <div className="relative h-full min-h-[calc(100vh-6rem)] bg-black text-white">
      <Canvas
        camera={{ position: [80, 60, 80], fov: 60 }}
        shadows
        gl={{ antialias: true, toneMapping: THREE.ACESFilmicToneMapping }}
      >
        {/* Background */}
        <color attach="background" args={['#030508']} />

        {/* Fog for depth perception */}
        <fog attach="fog" args={['#030508', 100, 400]} />

        <Suspense fallback={null}>
          {/* Lighting */}
          <AtmosphericLighting />

          {/* Environment (HDR-like ambient) */}
          <Environment preset="night" />

          {/* Stars */}
          <Stars radius={280} depth={40} count={2500} factor={4} saturation={0} fade speed={0.5} />

          {/* Infinite grid */}
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

          {/* Ambient particles */}
          <AmbientParticles />

          {/* Command Center at origin */}
          <CommandCenter />

          {/* NORA avatar */}
          <NoraAvatar position={[0, COMMAND_CENTER_SPAWN_Y + 2, 0]} />

          {/* Project buildings */}
          {projects.map((project) => (
            <ProjectBuilding
              key={project.name}
              name={project.name}
              position={project.position}
              energy={project.energy}
              isSelected={selectedProject?.name === project.name}
              onSelect={() => handleSelect(project)}
              isEnterTarget={!activeInterior && enterTarget?.name === project.name}
              entryHotkey="E"
            />
          ))}

          {/* User avatar */}
        <UserAvatar
          initialPosition={INITIAL_PLAYER_POSITION}
          color="#ff8000"
          onPositionChange={handleUserPositionChange}
          onInteract={handleAttemptEnter}
          isSuspended={Boolean(activeInterior || isConsoleInputActive)}
          canFly
        />
        </Suspense>
      </Canvas>

      {/* UI Overlays */}
      {!activeInterior && (
        <div className="pointer-events-none absolute top-6 left-6 bg-black/70 border border-cyan-400/40 rounded-lg p-5 max-w-lg backdrop-blur-sm">
          <p className="text-cyan-200 text-xs uppercase tracking-[0.3em] mb-2 font-semibold">
            PCG Virtual Environment V2
          </p>
          <h1 className="text-2xl font-bold text-white mb-3">Monumental Grid</h1>
          <p className="text-sm text-cyan-100/90 leading-relaxed mb-3">
            Explore the spatial command center. Each structure represents a project at monumental scale.
          </p>
          <div className="text-xs text-cyan-200/70 space-y-1">
            <p><span className="font-semibold">WASD</span> - Move</p>
            <p><span className="font-semibold">Shift</span> - Sprint</p>
            <p><span className="font-semibold">Space/Ctrl</span> - Up/Down</p>
            <p><span className="font-semibold">Mouse</span> - Look around</p>
            <p><span className="font-semibold">Click Building</span> - Select</p>
            <p><span className="font-semibold">E</span> - Enter nearby structure</p>
            <p><span className="font-semibold">Enter</span> - Engage command net</p>
            <p><span className="font-semibold">Esc</span> - Return to flight controls</p>
          </div>
        </div>
      )}

      {!activeInterior && (
        <AgentChatConsole
          className="pointer-events-auto absolute bottom-6 left-6 w-[28rem]"
          statusLine={noraLine}
          statusVersion={noraStatusVersion}
          selectedProject={selectedProject}
          isInputActive={isConsoleInputActive}
          onRequestCloseInput={releaseConsoleInput}
          focusToken={consoleFocusVersion}
        />
      )}

      {!activeInterior && (
        <div className="pointer-events-auto absolute bottom-6 right-6 w-80 bg-black/70 border border-cyan-400/30 rounded-lg p-4 backdrop-blur-sm">
          <h2 className="text-cyan-200 text-sm uppercase tracking-[0.2em] mb-3 font-semibold">
            System Status
          </h2>
          <div className="space-y-2 text-xs text-cyan-100/80">
            <div className="flex items-center justify-between">
              <span>Structures Deployed</span>
              <span className="font-mono text-cyan-300">{projects.length}</span>
            </div>
            <div className="flex items-center justify-between">
              <span>Command Center</span>
              <span className="text-green-400">● OPERATIONAL</span>
            </div>
            <div className="flex items-center justify-between">
              <span>NORA Status</span>
              <span className="text-green-400">● ONLINE</span>
            </div>
            <div className="flex items-center justify-between">
              <span>Grid Integrity</span>
              <span className="font-mono text-cyan-300">100%</span>
            </div>
          </div>
          <div className="mt-4 pt-3 border-t border-cyan-400/20">
            <p className="text-[10px] text-cyan-200/60 leading-relaxed">
              Monumental architecture prototype. Each building represents a project workspace where agents and humans collaborate.
            </p>
          </div>
        </div>
      )}

      {!activeInterior && (
        <div className="pointer-events-none absolute top-6 right-6 bg-yellow-900/20 border border-yellow-500/30 rounded px-3 py-2 text-xs text-yellow-200/80">
          <span className="font-semibold">V2 AESTHETIC UPGRADE</span> - Scale 10x increased
        </div>
      )}

      {!activeInterior && enterTarget && (
        <div className="pointer-events-none absolute inset-x-0 bottom-8 flex justify-center">
          <div className="rounded-full border border-cyan-400/40 bg-black/70 px-6 py-2 text-[11px] uppercase tracking-[0.4em] text-cyan-100">
            Press <span className="mx-1 font-semibold text-white">E</span> to enter {enterTarget.name}
          </div>
        </div>
      )}

      {activeInterior && (
        <BuildingInterior
          project={{
            name: activeInterior.name,
            energy: activeInterior.energy,
            type: getBuildingType(activeInterior.name),
          }}
          onExit={exitInterior}
        />
      )}
    </div>
  );
}

export default VirtualEnvironmentPage;
