import { Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Grid, Environment, Stars, SpotLight } from '@react-three/drei';
import * as THREE from 'three';
import {
  type LucideIcon,
  Backpack,
  ChevronDown,
  ChevronUp,
  Compass,
  Cpu,
  Gamepad2,
  Layers,
  Loader2,
  Map as MapIcon,
  Shirt,
} from 'lucide-react';
import { CommandCenter } from '@/components/virtual-world/CommandCenter';
import { NoraAvatar } from '@/components/virtual-world/NoraAvatar';
import { WanderingAgent } from '@/components/virtual-world/WanderingAgent';
import { ProjectBuilding } from '@/components/virtual-world/ProjectBuilding';
import { ToposDataSphere } from '@/components/virtual-world/ToposDataSphere';
import { UserAvatar, type BuildingCollider } from '@/components/virtual-world/UserAvatar';
import { BuildingInterior } from '@/components/virtual-world/BuildingInterior';
import { MultiplayerManager } from '@/components/virtual-world/MultiplayerManager';
import { useMultiplayerStore } from '@/stores/useMultiplayerStore';
import { AgentWorkspaceLevel, getAgentBayBounds } from '@/components/virtual-world/AgentWorkspaceLevel';
import { SpiralStaircase } from '@/components/virtual-world/SpiralStaircase';
import { AgentChatConsole } from '@/components/nora/AgentChatConsole';
import { InventoryPanel, EquipmentPanel } from '@/components/virtual-world/hud';
import { getBuildingType } from '@/lib/virtual-world/buildingTypes';
import { ENTRY_TRIGGER_DISTANCE } from '@/lib/virtual-world/constants';
import { cn } from '@/lib/utils';
import { useProjectList } from '@/hooks/api/useProjectList';
import { useAuth } from '@/contexts/AuthContext';
import { useEquipmentStore } from '@/stores/useEquipmentStore';
import type { Project } from 'shared/types';

// Topos directory items for the Data Sphere visualization
const toposDirectoryItems = (typeof __TOPOS_PROJECTS__ !== 'undefined'
  ? __TOPOS_PROJECTS__
  : []) as string[];

const PROJECT_HALF_WIDTH = 25;
const PROJECT_HALF_LENGTH = 50;
const PROJECT_FOOTPRINT_RADIUS = Math.sqrt(PROJECT_HALF_WIDTH ** 2 + PROJECT_HALF_LENGTH ** 2);
const COMMAND_CENTER_FLOOR_Y = 80; // Elevated floating platform
// DATA_SPHERE_RADIUS moved to ToposDataSphere component
const BASE_PROJECT_RADIUS = 180; // Projects arranged around the data sphere
const TARGET_ARC_SPACING = PROJECT_FOOTPRINT_RADIUS * 2.2;

interface ProjectData {
  id: string;
  name: string;
  position: [number, number, number];
  energy: number;
  project: Project; // Full project data from API
}

type HudPanelId = 'systems' | 'intel' | 'map' | 'controls' | 'inventory' | 'equipment';

const HUD_NAV_ITEMS: { id: HudPanelId; label: string; description: string; icon: LucideIcon }[] = [
  { id: 'systems', label: 'Systems', description: 'Server diagnostics', icon: Cpu },
  { id: 'intel', label: 'Intel', description: 'Project dossiers', icon: Layers },
  { id: 'map', label: 'Cartography', description: 'Spatial telemetry', icon: MapIcon },
  { id: 'controls', label: 'Controls', description: 'Piloting reference', icon: Gamepad2 },
  { id: 'inventory', label: 'Inventory', description: 'Personal belongings', icon: Backpack },
  { id: 'equipment', label: 'Gear', description: 'Equipped items', icon: Shirt },
];

const HUD_PANEL_META: Record<HudPanelId, { title: string; description: string }> = {
  systems: { title: 'Systems Console', description: 'Monitor command center throughput and structural integrity.' },
  intel: { title: 'Intel Ledger', description: 'Active engagements ranked by signal strength.' },
  map: { title: 'Aerial Cartography', description: 'Top-down sweep of the monumental grid.' },
  controls: { title: 'Flight Controls', description: 'Reference for movement, chat, and interaction shortcuts.' },
  inventory: { title: 'Inventory', description: 'Items in your possession. Click to equip.' },
  equipment: { title: 'Equipment', description: 'Currently equipped gear. Click slots to unequip.' },
};

const noraAcknowledgements = [
  'Routing orchestration energy to',
  'Illuminating systems for',
  'Calibrating systems for',
  'Summoning agents around',
  'Focusing the grid on',
  'Deploying sub-agents to',
  'Synchronizing timelines with',
  'Amplifying signal for',
];

// Spawn player ON the Command Center platform with Nora
// Command Center is at y=80, add offset for avatar feet
const PLAYER_COLOR = '#ff8800';
// Spawn on command center floor, outside hologram railing (R > 10)
const INITIAL_PLAYER_POSITION: [number, number, number] = [15, COMMAND_CENTER_FLOOR_Y + 1, 15];

// Static demo project for Fine Art Society (always available)
const FINE_ART_SOCIETY_PROJECT: Project = {
  id: 'fine-art-society-demo',
  name: 'Fine Art Society',
  git_repo_path: '/demo/fine-art-society',
  setup_script: null,
  dev_script: null,
  cleanup_script: null,
  copy_files: null,
  created_at: new Date(),
  updated_at: new Date(),
};

function stringEnergy(input: string) {
  let hash = 0;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash + input.charCodeAt(i) * (i + 11)) % 1000;
  }
  return 0.35 + (hash / 1000) * 0.65;
}

function generateProjectsFromAPI(apiProjects: Project[]): ProjectData[] {
  if (!apiProjects.length) return [];

  const radiusForSpacing = (apiProjects.length * TARGET_ARC_SPACING) / (Math.PI * 2);
  const minVisualRadius = PROJECT_FOOTPRINT_RADIUS * 2.8;
  const radius = Math.max(BASE_PROJECT_RADIUS, minVisualRadius, radiusForSpacing);
  const y = 0; // Buildings rest on ground

  return apiProjects.map((project, index) => {
    const angle = (index / apiProjects.length) * Math.PI * 2;
    const position: [number, number, number] = [
      Math.cos(angle) * radius,
      y,
      Math.sin(angle) * radius,
    ];
    return {
      id: project.id,
      name: project.name,
      position,
      energy: stringEnergy(project.name),
      project,
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

// Enhanced particle system with wind effects
function EnhancedParticles() {
  const particleCount = 500;
  const particlesRef = useRef<THREE.Points>(null);

  // Initialize particle positions and types
  const { positions, colors, sizes } = useMemo(() => {
    const pos = new Float32Array(particleCount * 3);
    const cols = new Float32Array(particleCount * 3);
    const szs = new Float32Array(particleCount);

    const particleTypes = [
      { color: new THREE.Color('#00ffff'), size: 0.3 }, // Dust (cyan)
      { color: new THREE.Color('#ff8000'), size: 0.5 }, // Sparks (orange)
      { color: new THREE.Color('#00ff80'), size: 0.2 }, // Data bits (green)
    ];

    for (let i = 0; i < particleCount; i++) {
      // Initial positions
      pos[i * 3] = (Math.random() - 0.5) * 400;
      pos[i * 3 + 1] = Math.random() * 100 + 10;
      pos[i * 3 + 2] = (Math.random() - 0.5) * 400;

      // Particle type (random distribution)
      const type = particleTypes[Math.floor(Math.random() * particleTypes.length)];
      cols[i * 3] = type.color.r;
      cols[i * 3 + 1] = type.color.g;
      cols[i * 3 + 2] = type.color.b;
      szs[i] = type.size;
    }

    return { positions: pos, colors: cols, sizes: szs };
  }, []);

  // Wind effect animation
  useFrame((state) => {
    if (!particlesRef.current) return;

    const time = state.clock.elapsedTime;

    // Wind direction (circular, slowly changing)
    const windX = Math.sin(time * 0.1) * 0.02;
    const windZ = Math.cos(time * 0.1) * 0.02;
    const windY = 0.01; // Slight upward drift

    const positionsAttr = particlesRef.current.geometry.attributes.position;

    for (let i = 0; i < particleCount; i++) {
      // Apply wind
      positionsAttr.array[i * 3] += windX;
      positionsAttr.array[i * 3 + 1] += windY;
      positionsAttr.array[i * 3 + 2] += windZ;

      // Wrap around boundaries
      if (positionsAttr.array[i * 3] > 200) positionsAttr.array[i * 3] = -200;
      if (positionsAttr.array[i * 3] < -200) positionsAttr.array[i * 3] = 200;
      if (positionsAttr.array[i * 3 + 1] > 110) positionsAttr.array[i * 3 + 1] = 10;
      if (positionsAttr.array[i * 3 + 2] > 200) positionsAttr.array[i * 3 + 2] = -200;
      if (positionsAttr.array[i * 3 + 2] < -200) positionsAttr.array[i * 3 + 2] = 200;
    }

    positionsAttr.needsUpdate = true;
  });

  return (
    <points ref={particlesRef}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={particleCount}
          array={positions}
          itemSize={3}
        />
        <bufferAttribute
          attach="attributes-color"
          count={particleCount}
          array={colors}
          itemSize={3}
        />
        <bufferAttribute
          attach="attributes-size"
          count={particleCount}
          array={sizes}
          itemSize={1}
        />
      </bufferGeometry>
      <pointsMaterial
        size={0.3}
        vertexColors
        transparent
        opacity={0.5}
        sizeAttenuation
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}

export function VirtualEnvironmentPage() {
  // Get current user info
  const { user } = useAuth();
  const isAdmin = user?.is_admin ?? false;

  // Debug: Equipment store state
  const equipped = useEquipmentStore((s) => s.equipped);
  const inventory = useEquipmentStore((s) => s.inventory);
  const initializedForUser = useEquipmentStore((s) => s.initializedForUser);

  // Fetch projects from the Dashboard API
  const { data: apiProjects = [], isLoading: projectsLoading, error: projectsError } = useProjectList();

  // Combine API projects with static demo projects (Fine Art Society)
  const allProjects = useMemo(() => {
    // Always include Fine Art Society demo project
    const staticProjects = [FINE_ART_SOCIETY_PROJECT];
    // Merge with API projects (avoid duplicates by name)
    const apiProjectNames = new Set(apiProjects.map(p => p.name));
    const uniqueStaticProjects = staticProjects.filter(p => !apiProjectNames.has(p.name));
    return [...apiProjects, ...uniqueStaticProjects];
  }, [apiProjects]);

  // Generate positioned project data from all projects
  const projects = useMemo(() => generateProjectsFromAPI(allProjects), [allProjects]);

  // Create building colliders for collision detection
  const buildingColliders = useMemo<BuildingCollider[]>(() => {
    return projects.map((project) => {
      const dir = new THREE.Vector3(-project.position[0], 0, -project.position[2]);
      if (dir.lengthSq() === 0) {
        dir.set(0, 0, 1);
      }
      dir.normalize();
      return {
        position: project.position,
        entranceDirection: dir,
      };
    });
  }, [projects]);

  const [selectedProject, setSelectedProject] = useState<ProjectData | null>(null);
  const [noraLine, setNoraLine] = useState('Command Center online. Syncing with Dashboard...');
  const [noraStatusVersion, setNoraStatusVersion] = useState(1);
  const [userPosition, setUserPosition] = useState<[number, number, number]>(INITIAL_PLAYER_POSITION);
  const [activeInterior, setActiveInterior] = useState<ProjectData | null>(null);
  const [isConsoleInputActive, setIsConsoleInputActive] = useState(false);
  const [consoleFocusVersion, setConsoleFocusVersion] = useState(0);
  const [isChatCollapsed, setIsChatCollapsed] = useState(false);
  const [activeHudPanel, setActiveHudPanel] = useState<HudPanelId | null>(null);

  const updateNoraLine = useCallback((line: string) => {
    setNoraLine(line);
    setNoraStatusVersion((prev) => prev + 1);
  }, []);

  // Update Nora when projects finish loading
  useEffect(() => {
    if (!projectsLoading && !projectsError && apiProjects.length > 0) {
      updateNoraLine(`Dashboard sync complete. ${apiProjects.length} projects online. Topos data sphere active.`);
    } else if (!projectsLoading && !projectsError && apiProjects.length === 0) {
      updateNoraLine('Dashboard offline. Demo projects available. Fine Art Society space ready to explore.');
    }
  }, [projectsLoading, projectsError, apiProjects.length, updateNoraLine]);

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

  const toggleChatCollapse = useCallback(() => {
    setIsChatCollapsed((prev) => {
      if (!prev) {
        releaseConsoleInput();
      }
      return !prev;
    });
  }, [releaseConsoleInput]);

  const handleSelect = useCallback((project: ProjectData) => {
    setSelectedProject(project);
    const line = noraAcknowledgements[
      Math.floor(Math.random() * noraAcknowledgements.length)
    ];
    updateNoraLine(`${line} ${project.name}.`);
  }, [updateNoraLine]);

  const sendPositionUpdate = useMultiplayerStore((s) => s.sendPositionUpdate);
  const multiplayerIsConnected = useMultiplayerStore((s) => s.isConnected);
  const isMovingRef = useRef(false);
  const lastPositionRef = useRef<[number, number, number]>(INITIAL_PLAYER_POSITION);

  const handleUserPositionChange = useCallback((vector: THREE.Vector3) => {
    const newPos: [number, number, number] = [vector.x, vector.y, vector.z];
    setUserPosition(newPos);

    // Determine if player is moving (position changed significantly)
    const [lx, ly, lz] = lastPositionRef.current;
    const dist = Math.hypot(vector.x - lx, vector.y - ly, vector.z - lz);
    isMovingRef.current = dist > 0.01;
    lastPositionRef.current = newPos;

    // Determine zone based on position
    let zone = 'ground';
    const distFromCenter = Math.hypot(vector.x, vector.z);
    if (vector.y >= 75 && distFromCenter <= 45) {
      zone = 'command_center';
    } else if (vector.y >= 65 && vector.y < 78) {
      zone = 'workspace';
    }

    // Send position to multiplayer
    if (multiplayerIsConnected) {
      sendPositionUpdate(
        { x: vector.x, y: vector.y, z: vector.z },
        { y: 0 }, // Rotation - UserAvatar doesn't expose this yet
        zone,
        isMovingRef.current
      );
    }
  }, [sendPositionUpdate, multiplayerIsConnected]);

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

  const toggleHudPanel = useCallback((panel: HudPanelId) => {
    setActiveHudPanel((prev) => (prev === panel ? null : panel));
  }, []);

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
    if (activeInterior) {
      setActiveHudPanel(null);
    }
  }, [activeInterior]);

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

  const hudPanelContent = useMemo(() => {
    if (!activeHudPanel) return null;
    switch (activeHudPanel) {
      case 'systems':
        return (
          <SystemsPanel
            projects={projects}
            selectedProject={selectedProject}
            userPosition={userPosition}
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
  }, [activeHudPanel, projects, selectedProject, userPosition]);

  return (
    <div className="relative h-full min-h-[calc(100vh-6rem)] bg-black text-white">
      {/* Loading overlay */}
      {projectsLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/80">
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-12 w-12 animate-spin text-cyan-400" />
            <p className="text-lg text-cyan-100">Syncing with Command Center...</p>
          </div>
        </div>
      )}

      {/* Error overlay */}
      {projectsError && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/80">
          <div className="flex flex-col items-center gap-4 text-center">
            <p className="text-lg text-red-400">Failed to connect to Dashboard API</p>
            <p className="text-sm text-red-300">{projectsError.message}</p>
          </div>
        </div>
      )}

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

          {/* Enhanced particles with wind */}
          <EnhancedParticles />

          {/* Volumetric lighting from command center (god rays) */}
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

          {/* Topos Data Sphere at ground level (center of map) */}
          {/* Uses filesystem directory items for the knowledge visualization */}
          <ToposDataSphere
            toposItems={toposDirectoryItems}
            commandCenterHeight={COMMAND_CENTER_FLOOR_Y}
          />

          {/* Command Center floating above */}
          <CommandCenter />

          {/* Agent Workspace Level (below Command Center) */}
          <AgentWorkspaceLevel />

          {/* Spiral Staircase connecting Command Center to Agent Workspace */}
          <SpiralStaircase />

          {/* NORA avatar in the Command Center */}
          <NoraAvatar position={[0, COMMAND_CENTER_FLOOR_Y + 2, 0]} />

          {/* Agents in their designated workspaces on the lower level */}
          <WanderingAgent
            name="Maci"
            role="cinematographer"
            bayBounds={getAgentBayBounds('Maci') || undefined}
          />
          <WanderingAgent
            name="Editron"
            role="editor"
            bayBounds={getAgentBayBounds('Editron') || undefined}
          />
          <WanderingAgent
            name="Bowser"
            role="browser"
            bayBounds={getAgentBayBounds('Bowser') || undefined}
          />
          <WanderingAgent
            name="Auri"
            role="oracle"
            bayBounds={getAgentBayBounds('Auri') || undefined}
          />

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
            color={PLAYER_COLOR}
            isAdmin={isAdmin}
            onPositionChange={handleUserPositionChange}
            onInteract={handleAttemptEnter}
            isSuspended={Boolean(activeInterior || isConsoleInputActive)}
            canFly
            buildings={buildingColliders}
          />

          {/* Multiplayer - renders other players */}
          <MultiplayerManager />

        </Suspense>
      </Canvas>

      {!activeInterior && (
        <>
          <div className="pointer-events-auto absolute top-4 right-4 w-[min(20rem,calc(100%-2rem))]">
            <div className="rounded-2xl border border-amber-500/30 bg-[#050403]/90 p-3 backdrop-blur-sm shadow-[0_8px_30px_rgba(0,0,0,0.5)]">
              <MiniMap
                projects={projects}
                selectedProject={selectedProject}
                userPosition={userPosition}
                size={220}
              />
            </div>
          </div>

          {/* Debug Panel - Remove after testing */}
          <div className="pointer-events-auto absolute top-4 left-4 w-64 rounded-lg border border-red-500/50 bg-black/90 p-3 font-mono text-xs text-red-400">
            <div className="mb-2 font-bold text-red-300">DEBUG STATE</div>
            <div>User: {user?.username || 'none'}</div>
            <div>user.is_admin: {String(user?.is_admin)}</div>
            <div>isAdmin (derived): {String(isAdmin)}</div>
            <div>initializedForUser: {initializedForUser || 'null'}</div>
            <div>inventory: [{inventory.join(', ')}]</div>
            <div>equipped.head: {equipped.head || 'null'}</div>
            <div>equipped.primaryHand: {equipped.primaryHand || 'null'}</div>
            <div>equipped.secondaryHand: {equipped.secondaryHand || 'null'}</div>
            <div>equipped.back: {equipped.back || 'null'}</div>
          </div>

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
                  onClick={toggleChatCollapse}
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
                  onRequestCloseInput={releaseConsoleInput}
                  focusToken={consoleFocusVersion}
                  showHeader={false}
                />
              </div>
              <div className="border-t border-amber-500/30 px-3 py-1 text-[10px] text-amber-200/80">
                {isChatCollapsed ? 'Press Enter to reopen the command net.' : 'Enter engages the net · Esc cancels typing'}
              </div>
            </div>
          </div>

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
                    onClick={() => setActiveHudPanel(null)}
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
                    onClick={() => toggleHudPanel(item.id)}
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
      )}

      {!activeInterior && enterTarget && (
        <div className="pointer-events-none absolute inset-x-0 bottom-28 flex justify-center">
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
          playerColor={PLAYER_COLOR}
          onExit={exitInterior}
        />
      )}

    </div>
  );
}

export default VirtualEnvironmentPage;
interface SystemsPanelProps {
  projects: ProjectData[];
  selectedProject: ProjectData | null;
  userPosition: [number, number, number];
}

interface IntelPanelProps {
  projects: ProjectData[];
}

type MapPanelProps = SystemsPanelProps;

interface MiniMapProps {
  projects: ProjectData[];
  selectedProject: ProjectData | null;
  userPosition: [number, number, number];
  size?: number;
}

function SystemsPanel({ projects, selectedProject, userPosition }: SystemsPanelProps) {
  const [userX, , userZ] = userPosition;
  const averageEnergy = projects.length
    ? projects.reduce((sum, project) => sum + project.energy, 0) / projects.length
    : 0;
  const topProject = projects.length
    ? [...projects].sort((a, b) => b.energy - a.energy)[0]
    : null;
  const distanceFromCenter = Math.hypot(userX, userZ);

  return (
    <div className="flex flex-col gap-4 lg:flex-row">
      <div className="flex-1 space-y-3">
        <div className="grid grid-cols-2 gap-3 rounded-lg border border-amber-500/20 bg-black/30 p-4 text-xs text-amber-100/80">
          <div>
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Structures</p>
            <p className="text-2xl font-bold text-white">{projects.length}</p>
            <p className="text-[10px] text-amber-200/60">Deployed across grid</p>
          </div>
          <div>
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Average Signal</p>
            <p className="text-2xl font-bold text-white">{(averageEnergy * 100).toFixed(0)}%</p>
            <p className="text-[10px] text-amber-200/60">Energy distribution</p>
          </div>
          <div>
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Command Center</p>
            <p className="text-lg font-semibold text-green-300">● Operational</p>
            <p className="text-[10px] text-amber-200/60">Core services nominal</p>
          </div>
          <div>
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Pilot Position</p>
            <p className="text-lg font-semibold text-white">{distanceFromCenter.toFixed(0)}m from core</p>
            <p className="text-[10px] text-amber-200/60">Warden altitude stable</p>
          </div>
        </div>

        <div className="rounded-lg border border-amber-500/20 bg-black/40 p-4 text-xs text-amber-100/80">
          <p className="mb-2 text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Live status feed</p>
          <ul className="space-y-1">
            <li>• Grid integrity holding at 100%.</li>
            <li>
              • Nora channel {selectedProject ? `linked to ${selectedProject.name}.` : 'idle and awaiting directive.'}
            </li>
            {topProject && (
              <li>
                • {topProject.name} broadcasting strongest signal at {(topProject.energy * 100).toFixed(1)}%.
              </li>
            )}
          </ul>
        </div>
      </div>

      <div className="w-full shrink-0 lg:w-64">
        <MiniMap projects={projects} selectedProject={selectedProject} userPosition={userPosition} />
      </div>
    </div>
  );
}

function IntelPanel({ projects }: IntelPanelProps) {
  if (!projects.length) {
    return <p className="text-sm text-amber-200/80">No structures are synced with this environment yet.</p>;
  }

  const ranked = [...projects].sort((a, b) => b.energy - a.energy).slice(0, 8);

  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2">
        {ranked.map((project, index) => (
          <div key={project.name} className="rounded-lg border border-amber-500/20 bg-black/30 p-3">
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">#{index + 1}</p>
            <p className="text-base font-semibold text-white">{project.name}</p>
            <p className="text-[11px] text-amber-100/70">Energy {(project.energy * 100).toFixed(1)}%</p>
            <p className="text-[11px] text-amber-100/60">Status: {project.energy > 0.65 ? 'Prime' : 'Stable'}</p>
          </div>
        ))}
      </div>
      <p className="text-[11px] text-amber-200/70">
        Rankings update live as MCP worktrees spin up or wind down.
      </p>
    </div>
  );
}

function MapPanel({ projects, selectedProject, userPosition }: MapPanelProps) {
  const [userX, , userZ] = userPosition;
  const closestProject = useMemo(() => {
    if (!projects.length) return null;
    return projects.reduce<null | { project: ProjectData; distance: number }>((closest, project) => {
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
        />
      </div>
      <div className="flex-1 space-y-3 text-sm text-amber-100/80">
        <div className="rounded-lg border border-amber-500/20 bg-black/30 p-4">
          <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Navigator</p>
          <p className="text-lg font-semibold text-white">{distanceFromCenter.toFixed(0)}m from command core</p>
          <p className="text-[12px] text-amber-100/70">
            Hover vector ready. Use WASD + Q/E to strafe above the ring of structures.
          </p>
        </div>
        <div className="rounded-lg border border-amber-500/20 bg-black/30 p-4">
          <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Nearest signal</p>
          {closestProject ? (
            <>
              <p className="text-lg font-semibold text-white">{closestProject.project.name}</p>
              <p className="text-[12px] text-amber-100/70">
                {closestProject.distance.toFixed(1)}m away · Energy {(closestProject.project.energy * 100).toFixed(1)}%
              </p>
            </>
          ) : (
            <p className="text-[12px] text-amber-100/70">No structures detected on this shard yet.</p>
          )}
        </div>
        <p className="text-[11px] text-amber-200/70">
          Tip: engage the Systems tab to pin stats, then keep this map floating for quick orientation
          during flyovers.
        </p>
      </div>
    </div>
  );
}

function MiniMap({ projects, selectedProject, userPosition, size = 220 }: MiniMapProps) {
  const [userX, , userZ] = userPosition;
  const maxRadius = projects.reduce((max, project) => {
    const radius = Math.hypot(project.position[0], project.position[2]);
    return Math.max(max, radius);
  }, 1);
  const margin = 18;
  const scale = (size / 2 - margin) / (maxRadius || 1);
  const patternId = useMemo(() => `mini-map-grid-${Math.random().toString(36).slice(2)}`, []);

  const toMapX = (value: number) => size / 2 + value * scale;
  const toMapY = (value: number) => size / 2 + value * scale;

  return (
    <div>
      <div className="mb-2 flex items-center justify-between text-[11px] uppercase tracking-[0.3em] text-amber-200">
        <span>Mini Map</span>
        <Compass className="h-4 w-4" />
      </div>
      <div className="relative rounded-lg border border-amber-500/30 bg-black/50 p-2">
        <svg width={size} height={size} className="rounded bg-[#050403]">
          <defs>
            <pattern id={patternId} width="16" height="16" patternUnits="userSpaceOnUse">
              <path d="M 16 0 L 0 0 0 16" stroke="#3f2c16" strokeWidth="0.5" fill="none" />
            </pattern>
          </defs>
          <rect width={size} height={size} fill="#050403" />
          <rect width={size} height={size} fill={`url(#${patternId})`} opacity={0.7} />
          <circle cx={size / 2} cy={size / 2} r={4} fill="#f97316" opacity={0.8} />
          {projects.map((project) => {
            const x = toMapX(project.position[0]);
            const y = toMapY(project.position[2]);
            const isSelected = selectedProject?.name === project.name;
            return (
              <circle
                key={project.name}
                cx={x}
                cy={y}
                r={isSelected ? 6 : 4}
                fill={isSelected ? '#fbbf24' : '#38bdf8'}
                opacity={isSelected ? 0.95 : 0.7}
              />
            );
          })}
          <circle cx={toMapX(userX)} cy={toMapY(userZ)} r={5} fill="#f472b6" stroke="#ffffff" strokeWidth={1} />
        </svg>
        <span className="pointer-events-none absolute right-4 top-3 text-[10px] font-semibold text-amber-200">N</span>
      </div>
      <p className="mt-2 text-[11px] text-amber-200/70">
        Orange dot represents the command core. Pink indicator marks your current hovercraft.
      </p>
    </div>
  );
}

function ControlsPanel() {
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
          <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">{binding.action}</p>
          <p className="text-base font-semibold text-white">{binding.detail}</p>
        </div>
      ))}
      <p className="sm:col-span-2 text-[11px] text-amber-200/70">
        Slash shortcuts: <span className="font-mono">/global</span>, <span className="font-mono">/help</span>, or <span className="font-mono">/agent</span> mirror MMO chat conventions.
      </p>
    </div>
  );
}
