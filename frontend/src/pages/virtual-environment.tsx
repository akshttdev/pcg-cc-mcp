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
import { CommandCenter } from '@/components/vibeland/CommandCenter';
import { NoraAvatar } from '@/components/vibeland/NoraAvatar';
import { WanderingAgent } from '@/components/vibeland/WanderingAgent';
import { ToposDataSphere } from '@/components/vibeland/ToposDataSphere';
import { UserAvatar } from '@/components/vibeland/UserAvatar';
import { MultiplayerManager } from '@/components/vibeland/MultiplayerManager';
import { useMultiplayerStore } from '@/stores/useMultiplayerStore';
import { AgentWorkspaceLevel, getAgentBayBounds } from '@/components/vibeland/AgentWorkspaceLevel';
import { SpiralStaircase } from '@/components/vibeland/SpiralStaircase';
import { AgentChatConsole } from '@/components/nora/AgentChatConsole';
import { OrchaAvatar } from '@/components/vibeland/OrchaAvatar';
import { InventoryPanel, EquipmentPanel } from '@/components/vibeland/hud';
import { ENTRY_TRIGGER_DISTANCE, BUILDING_HALF_LENGTH } from '@/lib/vibeland/constants';
import { ProjectBuilding } from '@/components/vibeland/ProjectBuilding';
import { cn } from '@/lib/utils';
import { useProjectList } from '@/hooks/api/useProjectList';
import { useAuth } from '@/contexts/AuthContext';
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
  /** true = user is a member or building is universally public */
  accessible: boolean;
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

// Spawn positions based on role
const PLAYER_COLOR = '#ff8800';
// Admin: spawn on command center floor, outside hologram railing (R > 10)
// Y = floor (80) + AVATAR_RADIUS (0.5) + 0.1 puts player within the onGround threshold
// so they snap to the floor in the first frame instead of floating above it.
const SPAWN_ADMIN: [number, number, number] = [20, COMMAND_CENTER_FLOOR_Y + 0.6, 0];
// User: spawn south of command center, facing inward — can see the world and command center above
const SPAWN_USER: [number, number, number] = [0, 1, 60];

// ─── Virtual Zone System ──────────────────────────────────────────────────────
// The global world has PCG Command Center at origin and other zones arranged
// in a ring around it at ZONE_RING_RADIUS.

const ZONE_RING_RADIUS = 280;

// Static zone definitions that are always present in the world
// DB-backed zones (from /api/virtual-spaces) supplement and override these
const STATIC_ZONES: VirtualZone[] = [
  {
    space_name: 'Fine Art Society',
    host_username: 'pcg',
    color: '#ffd700',
    staticAngle: 0,
  },
  {
    space_name: 'Veritwin',
    host_username: 'Andre',
    color: '#00aaff',
    staticAngle: 1,
  },
  {
    space_name: 'Jungleverse',
    host_username: 'pcg',
    color: '#00cc44',
    staticAngle: 2,
  },
  {
    space_name: 'Media Monsters HQ',
    host_username: 'Travers',
    color: '#ff5500',
    staticAngle: 3,
  },
  {
    space_name: 'Sirak Studios',
    host_username: 'Sirak',
    color: '#aa00ff',
    staticAngle: 4,
  },
];

interface VirtualZone {
  space_name: string;
  host_username: string;
  color: string;
  staticAngle: number; // index determining angular position in ring
  world_x?: number;
  spawn_x?: number;
  spawn_y?: number;
  spawn_z?: number;
}

// Evenly distribute N zones around a ring, returning [x, z] for each index
function zonePosition(index: number, total: number, radius: number): [number, number] {
  const angle = (index / total) * Math.PI * 2 - Math.PI / 2; // start at top
  return [Math.cos(angle) * radius, Math.sin(angle) * radius];
}


// Static demo project for Fine Art Society (always available)
const FINE_ART_SOCIETY_PROJECT: Project = {
  id: 'fine-art-society-demo',
  name: 'Fine Art Society',
  git_repo_path: '/demo/fine-art-society',
  setup_script: null,
  dev_script: null,
  cleanup_script: null,
  copy_files: null,
  vibe_budget_limit: null,
  vibe_spent_amount: 0,
  created_at: new Date(),
  updated_at: new Date(),
};

const PUBLIC_PROJECTS = new Set(['Fine Art Society']);

function stringEnergy(input: string) {
  let hash = 0;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash + input.charCodeAt(i) * (i + 11)) % 1000;
  }
  return 0.35 + (hash / 1000) * 0.65;
}

function generateProjectsFromAPI(apiProjects: Project[], accessibleIds: Set<string>): ProjectData[] {
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
      // Public projects are always accessible; otherwise check membership
      accessible: PUBLIC_PROJECTS.has(project.name) || accessibleIds.has(project.id),
    };
  });
}

// ─── Zone Landmark Component ──────────────────────────────────────────────────


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

  // Fetch virtual zones from API
  const [worldZones, setWorldZones] = useState<VirtualZone[]>(STATIC_ZONES);

  useEffect(() => {
    // Fetch all virtual spaces to enrich static zone list
    fetch('/api/virtual-spaces')
      .then(r => r.json())
      .then((d: { success: boolean; data: Array<{ space_name: string; host_username: string; world_x: number; spawn_x: number; spawn_y: number; spawn_z: number }> }) => {
        if (!d.success) return;
        setWorldZones(prev => prev.map(zone => {
          const found = d.data.find(s => s.space_name === zone.space_name);
          if (found) {
            return { ...zone, world_x: found.world_x, spawn_x: found.spawn_x, spawn_y: found.spawn_y, spawn_z: found.spawn_z };
          }
          return zone;
        }));
      })
      .catch(() => {});
  }, []);

  // Fetch projects from the Dashboard API
  const { data: apiProjects = [], isLoading: projectsLoading, error: projectsError, refetch: refetchProjects } = useProjectList();

  // Combine API projects with static demo projects (Fine Art Society)
  const allProjects = useMemo(() => {
    // Always include Fine Art Society demo project
    const staticProjects = [FINE_ART_SOCIETY_PROJECT];
    // Merge with API projects (avoid duplicates by name)
    const apiProjectNames = new Set(apiProjects.map(p => p.name));
    const uniqueStaticProjects = staticProjects.filter(p => !apiProjectNames.has(p.name));
    return [...apiProjects, ...uniqueStaticProjects];
  }, [apiProjects]);

  // Set of project IDs the current user is a member of (from the backend-filtered API)
  const accessibleIds = useMemo(() => new Set(apiProjects.map(p => p.id)), [apiProjects]);

  // Generate positioned project data from all projects
  const projects = useMemo(() => generateProjectsFromAPI(allProjects, accessibleIds), [allProjects, accessibleIds]);

  // Zone buildings — each zone has a full ProjectBuilding in the global world
  // Admin always has access; others need to be members or the project must be public
  const zoneBuildings = useMemo(() =>
    worldZones.map((zone, idx) => {
      const [x, z] = zonePosition(idx, worldZones.length, ZONE_RING_RADIUS);
      const position: [number, number, number] = [x, 0, z];
      // Angle so the building's door faces PCG Command Center at origin
      const facingAngle = Math.atan2(-x, -z);
      return {
        zone,
        position,
        facingAngle,
        accessible: isAdmin || PUBLIC_PROJECTS.has(zone.space_name),
      };
    }),
    [worldZones, isAdmin]
  );

  const [selectedProject] = useState<ProjectData | null>(null);
  const [noraLine, setNoraLine] = useState('Command Center online. Syncing with Dashboard...');
  const [noraStatusVersion, setNoraStatusVersion] = useState(1);

  // Unified global world spawn: admin → command center floor, everyone else → global ground
  // There are no building interiors — all spaces exist in the same shared world.
  const spawnPosition = useMemo<[number, number, number]>(() => {
    if (isAdmin) return SPAWN_ADMIN;
    return SPAWN_USER;
  }, [isAdmin]);

  const [userPosition, setUserPosition] = useState<[number, number, number]>(spawnPosition);
  // activeZone = the zone beacon the user has entered (null = in global world)
  const [activeZone, setActiveZone] = useState<VirtualZone | null>(null);
  const [isConsoleInputActive, setIsConsoleInputActive] = useState(false);

  // DEBUG: track keyboard events reaching window
  const [debugLastKey, setDebugLastKey] = useState('none');
  const [debugKeyCount, setDebugKeyCount] = useState(0);
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      setDebugLastKey(e.key);
      setDebugKeyCount(c => c + 1);
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, []);
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
    if (activeZone) return;
    setIsConsoleInputActive(true);
    bumpConsoleFocus();
  }, [activeZone, bumpConsoleFocus]);

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

  const sendPositionUpdate = useMultiplayerStore((s) => s.sendPositionUpdate);
  const multiplayerIsConnected = useMultiplayerStore((s) => s.isConnected);
  const isMovingRef = useRef(false);
  const wasMovingRef = useRef(false);
  const lastPositionRef = useRef<[number, number, number]>(spawnPosition);
  const lastZoneRef = useRef('ground');
  const stoppedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
    lastZoneRef.current = zone;

    // Clear any pending "stopped" update since we got new movement
    if (stoppedTimeoutRef.current) {
      clearTimeout(stoppedTimeoutRef.current);
      stoppedTimeoutRef.current = null;
    }

    // Send position to multiplayer
    if (multiplayerIsConnected) {
      sendPositionUpdate(
        { x: vector.x, y: vector.y, z: vector.z },
        { y: 0 }, // Rotation - UserAvatar doesn't expose this yet
        zone,
        isMovingRef.current
      );

      // If we were moving but now stopped, schedule a "stopped" confirmation
      // This ensures remote clients know we stopped even if position updates stop
      if (wasMovingRef.current && !isMovingRef.current) {
        stoppedTimeoutRef.current = setTimeout(() => {
          sendPositionUpdate(
            { x: newPos[0], y: newPos[1], z: newPos[2] },
            { y: 0 },
            lastZoneRef.current,
            false
          );
        }, 150);
      }
    }

    wasMovingRef.current = isMovingRef.current;
  }, [sendPositionUpdate, multiplayerIsConnected]);

  // Detect which zone building the user is near (within entry range of the door)
  const ZONE_BUILDING_ENTRY_DISTANCE = ENTRY_TRIGGER_DISTANCE + BUILDING_HALF_LENGTH;
  const enterZoneTarget = useMemo(() => {
    if (activeZone) return null; // suppress prompt when panel already open
    let closest: { zone: VirtualZone; distance: number } | null = null;
    for (const { zone, position, accessible } of zoneBuildings) {
      if (!accessible) continue;
      const dx = position[0] - userPosition[0];
      const dz = position[2] - userPosition[2];
      const distance = Math.hypot(dx, dz);
      if (distance > ZONE_BUILDING_ENTRY_DISTANCE) continue;
      if (!closest || distance < closest.distance) {
        closest = { zone, distance };
      }
    }
    return closest?.zone ?? null;
  }, [activeZone, zoneBuildings, userPosition]);

  // Open a zone panel (stays in global world — MMO style)
  const handleAttemptEnter = useCallback(() => {
    if (!enterZoneTarget) return;
    setActiveZone(enterZoneTarget);
    updateNoraLine(`${enterZoneTarget.space_name} — ${enterZoneTarget.host_username}'s space. Press Esc to close.`);
  }, [enterZoneTarget, updateNoraLine]);

  const closeZonePanel = useCallback(() => {
    setActiveZone(null);
    updateNoraLine('Global environment active.');
  }, [updateNoraLine]);

  const toggleHudPanel = useCallback((panel: HudPanelId) => {
    setActiveHudPanel((prev) => (prev === panel ? null : panel));
  }, []);

  useEffect(() => {
    if (!activeZone) return undefined;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeZonePanel();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeZone, closeZonePanel]);

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

      if (event.key === 'Escape') {
        event.preventDefault();
        releaseConsoleInput();
      }
    };

    window.addEventListener('keydown', handleConsoleToggle);
    return () => window.removeEventListener('keydown', handleConsoleToggle);
  }, [activateConsoleInput, isConsoleInputActive, releaseConsoleInput]);

  useEffect(() => {
    if (activeZone && isConsoleInputActive) {
      setIsConsoleInputActive(false);
    }
  }, [activeZone, isConsoleInputActive]);

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
  }, [activeHudPanel, projects, selectedProject, userPosition]);

  return (
    <div className="relative h-full min-h-[calc(100vh-6rem)] bg-black text-white">
      {/* DEBUG OVERLAY — remove once movement is confirmed working */}
      <div style={{position:'fixed',top:8,left:'50%',transform:'translateX(-50%)',zIndex:99999,background:'rgba(0,0,0,0.85)',color:'#0ff',padding:'4px 14px',borderRadius:6,fontSize:11,fontFamily:'monospace',pointerEvents:'none',whiteSpace:'nowrap'}}>
        key: <b>{debugLastKey}</b> ({debugKeyCount}) | suspended: <b style={{color: isConsoleInputActive ? '#f55' : '#0f0'}}>{isConsoleInputActive ? 'YES ⛔' : 'no ✓'}</b>
      </div>
      {/* Loading overlay */}
      {projectsLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/80">
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-12 w-12 animate-spin text-cyan-400" />
            <p className="text-lg text-cyan-100">Syncing with Command Center...</p>
          </div>
        </div>
      )}

      {/* Non-blocking error banner — projects unavailable but world still usable */}
      {projectsError && (
        <div className="absolute top-2 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 px-4 py-2 rounded-lg bg-black/80 border border-red-500/40 text-sm">
          <span className="text-red-400">Projects unavailable</span>
          <button
            onClick={() => refetchProjects()}
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
        onPointerDown={releaseConsoleInput}
      >
        {/* Background */}
        <color attach="background" args={['#030508']} />

        {/* Fog for depth perception */}
        <fog attach="fog" args={['#030508', 150, 600]} />

        <Suspense fallback={null}>
          {/* Lighting */}
          <AtmosphericLighting />

          {/* Environment (HDR-like ambient) */}
          <Environment preset="night" />

          {/* Stars */}
          <Stars radius={600} depth={60} count={3000} factor={4} saturation={0} fade speed={0.5} />

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

          {/* Zone District Buildings - full structures arranged in a ring around PCG Command Center.
              Each building faces inward toward the command center. Admin has access to all. */}
          {zoneBuildings.map(({ zone, position, facingAngle, accessible }) => (
            <group key={zone.space_name} position={position} rotation={[0, facingAngle, 0]}>
              <ProjectBuilding
                name={zone.space_name}
                position={[0, 0, 0]}
                energy={0.85}
                isSelected={false}
                onSelect={() => {}}
                isEnterTarget={enterZoneTarget?.space_name === zone.space_name}
                entryHotkey="E"
                locked={!accessible}
              />
            </group>
          ))}

          {/* User avatar */}
          <UserAvatar
            initialPosition={spawnPosition}
            color={PLAYER_COLOR}
            isAdmin={isAdmin}
            onPositionChange={handleUserPositionChange}
            onInteract={handleAttemptEnter}
            isSuspended={isConsoleInputActive}
            canFly={isAdmin}
          />

          {/* ORCHA orchestrator avatar follows admin only */}
          {isAdmin && <OrchaAvatar userPosition={userPosition} />}

          {/* Multiplayer - renders other players */}
          <MultiplayerManager />

        </Suspense>
      </Canvas>

      <>
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
                  projectId={selectedProject?.id}
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

      {/* Zone entry prompt — shown when nearby a building and no panel is open */}
      {!activeZone && enterZoneTarget && (
        <div className="pointer-events-none absolute inset-x-0 bottom-28 flex justify-center">
          <div
            className="rounded-full px-6 py-2 text-[11px] uppercase tracking-[0.4em]"
            style={{
              border: `1px solid ${enterZoneTarget.color}66`,
              backgroundColor: 'rgba(0,0,0,0.7)',
              color: enterZoneTarget.color,
            }}
          >
            Press <span className="mx-1 font-semibold text-white">E</span> to open {enterZoneTarget.space_name}
          </div>
        </div>
      )}

      {/* Zone panel — slides in from the right, player stays in global world */}
      {activeZone && (
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
                <p className="text-[11px] tracking-[0.2em] text-amber-200/60">HOST: {activeZone.host_username.toUpperCase()}</p>
              </div>
            </div>
            <button
              type="button"
              onClick={closeZonePanel}
              className="rounded border border-amber-500/30 px-3 py-1 text-[10px] tracking-[0.2em] text-amber-200/80 transition hover:bg-amber-500/20 hover:text-white"
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

            {/* Find matching project */}
            {(() => {
              const matchingProject = allProjects.find(p => p.name === activeZone.space_name);
              return matchingProject ? (
                <a
                  href={`/projects/${matchingProject.id}`}
                  className="flex items-center justify-between rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-100 transition hover:bg-amber-500/20 hover:text-white"
                  style={{ borderColor: `${activeZone.color}44` }}
                >
                  <span className="tracking-[0.1em]">Open Project Workspace</span>
                  <span className="text-amber-400">→</span>
                </a>
              ) : (
                <div className="rounded-lg border border-amber-500/20 bg-black/30 px-4 py-3 text-xs text-amber-200/50 tracking-[0.1em]">
                  Project workspace not linked
                </div>
              );
            })()}
          </div>

          {/* Footer */}
          <div className="border-t border-amber-500/20 px-6 py-3 text-[10px] tracking-[0.2em] text-amber-200/40">
            YOU REMAIN IN THE GLOBAL ENVIRONMENT · ESC TO CLOSE
          </div>
        </div>
      )}

    </div>
  );
}

export default VirtualEnvironmentPage;
interface SystemsPanelProps {
  projects: ProjectData[];
  selectedProject: ProjectData | null;
  userPosition: [number, number, number];
  zones?: VirtualZone[];
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
  zones?: VirtualZone[];
}

function SystemsPanel({ projects, selectedProject, userPosition, zones = STATIC_ZONES }: SystemsPanelProps) {
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
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Structures</p>
            <p className="text-2xl font-bold text-white">{projects.length}</p>
            <p className="text-[10px] text-amber-200/60">Deployed across grid</p>
          </div>
          <div>
            <p className="text-[10px] uppercase tracking-[0.3em] text-amber-200/70">Active Zones</p>
            <p className="text-2xl font-bold text-white">{zones.length + 1}</p>
            <p className="text-[10px] text-amber-200/60">Including PCG Command Center</p>
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
        <MiniMap projects={projects} selectedProject={selectedProject} userPosition={userPosition} zones={zones} size={200} />
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

function MapPanel({ projects, selectedProject, userPosition, zones = STATIC_ZONES }: MapPanelProps) {
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
          zones={zones}
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

function MiniMap({ projects, selectedProject, userPosition, size = 220, zones = STATIC_ZONES }: MiniMapProps) {
  const [userX, , userZ] = userPosition;
  // Scale to show the zone ring + some margin
  const worldExtent = ZONE_RING_RADIUS + 60;
  const margin = 14;
  const scale = (size / 2 - margin) / worldExtent;
  const patternId = useMemo(() => `mini-map-grid-${Math.random().toString(36).slice(2)}`, []);

  const toMapX = (value: number) => size / 2 + value * scale;
  const toMapY = (value: number) => size / 2 + value * scale;

  return (
    <div>
      <div className="mb-2 flex items-center justify-between text-[11px] uppercase tracking-[0.3em] text-amber-200">
        <span>Global Map</span>
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

          {/* Zone ring guide */}
          <circle
            cx={size / 2}
            cy={size / 2}
            r={ZONE_RING_RADIUS * scale}
            fill="none"
            stroke="#1a3a2a"
            strokeWidth={1}
            strokeDasharray="4 4"
          />

          {/* Zone landmarks */}
          {zones.map((zone, idx) => {
            const [zx, zz] = zonePosition(idx, zones.length, ZONE_RING_RADIUS);
            const mx = toMapX(zx);
            const mz = toMapY(zz);
            const col = zone.color;
            return (
              <g key={zone.space_name}>
                <circle cx={mx} cy={mz} r={7} fill={col} opacity={0.25} />
                <circle cx={mx} cy={mz} r={4} fill={col} opacity={0.9} />
              </g>
            );
          })}

          {/* Project buildings (small dots) */}
          {projects.map((project) => {
            const x = toMapX(project.position[0]);
            const y = toMapY(project.position[2]);
            const isSelected = selectedProject?.name === project.name;
            return (
              <circle
                key={project.name}
                cx={x}
                cy={y}
                r={isSelected ? 5 : 3}
                fill={isSelected ? '#fbbf24' : '#38bdf8'}
                opacity={isSelected ? 0.95 : 0.5}
              />
            );
          })}

          {/* PCG Command Center (center) */}
          <circle cx={size / 2} cy={size / 2} r={6} fill="#00ffff" opacity={0.9} />
          <circle cx={size / 2} cy={size / 2} r={12} fill="none" stroke="#00ffff" strokeWidth={0.8} opacity={0.4} />

          {/* Player position */}
          <circle cx={toMapX(userX)} cy={toMapY(userZ)} r={5} fill="#f472b6" stroke="#ffffff" strokeWidth={1} />
        </svg>
        <span className="pointer-events-none absolute right-4 top-3 text-[10px] font-semibold text-amber-200">N</span>
      </div>
      {/* Zone legend */}
      <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1">
        {zones.map((z) => (
          <span key={z.space_name} className="flex items-center gap-1 text-[9px] text-amber-200/70">
            <span style={{ backgroundColor: z.color }} className="inline-block h-2 w-2 rounded-full" />
            {z.space_name}
          </span>
        ))}
      </div>
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
