/**
 * Embed Virtual Environment Page - Jungleverse Access
 *
 * A restricted version of the virtual environment for Jungleverse iframe embedding.
 * - Chat console enabled for global user communication
 * - NO access to NORA or any AI agents
 * - Movement restricted to Jungleverse project space only
 * - Accepts a JWT token via URL parameter for authentication.
 */
import React, { Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Canvas, useFrame } from '@react-three/fiber';
import { Grid, Environment, Stars } from '@react-three/drei';
import * as THREE from 'three';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { ProjectBuilding } from '@/components/virtual-world/ProjectBuilding';
import { UserAvatar } from '@/components/virtual-world/UserAvatar';
import { validateExternalToken, type UserProfile } from '@/lib/external-auth-api';
import { cn } from '@/lib/utils';

// Jungleverse-specific constants
const JUNGLEVERSE_PROJECT_NAME = 'jungleverse';
const MOVEMENT_BOUNDARY_RADIUS = 100; // Restrict movement to this radius around Jungleverse building

declare const __TOPOS_PROJECTS__: string[] | undefined;

const safeProjectList = (typeof __TOPOS_PROJECTS__ !== 'undefined'
  ? __TOPOS_PROJECTS__
  : []) as string[];

const PROJECT_HALF_WIDTH = 25;
const PROJECT_HALF_LENGTH = 50;
const PROJECT_FOOTPRINT_RADIUS = Math.sqrt(PROJECT_HALF_WIDTH ** 2 + PROJECT_HALF_LENGTH ** 2);
const BASE_PROJECT_RADIUS = 220;
const TARGET_ARC_SPACING = PROJECT_FOOTPRINT_RADIUS * 2.2;
const SPAWN_HEIGHT = 8;

interface ProjectData {
  name: string;
  position: [number, number, number];
  energy: number;
}

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
  const y = 0;

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
      <hemisphereLight args={['#1d2a3f', '#000000', 0.4]} />
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
      <pointLight position={[-100, 50, -100]} intensity={1} color="#ff8000" distance={200} decay={2} />
      <pointLight position={[100, 50, 100]} intensity={1} color="#0080ff" distance={200} decay={2} />
    </>
  );
}

function EnhancedParticles() {
  const particleCount = 300;
  const particlesRef = useRef<THREE.Points>(null);

  const { positions, colors, sizes } = useMemo(() => {
    const pos = new Float32Array(particleCount * 3);
    const cols = new Float32Array(particleCount * 3);
    const szs = new Float32Array(particleCount);

    const particleTypes = [
      { color: new THREE.Color('#00ffff'), size: 0.3 },
      { color: new THREE.Color('#ff8000'), size: 0.5 },
      { color: new THREE.Color('#00ff80'), size: 0.2 },
    ];

    for (let i = 0; i < particleCount; i++) {
      pos[i * 3] = (Math.random() - 0.5) * 200;
      pos[i * 3 + 1] = Math.random() * 80 + 10;
      pos[i * 3 + 2] = (Math.random() - 0.5) * 200;

      const type = particleTypes[Math.floor(Math.random() * particleTypes.length)];
      cols[i * 3] = type.color.r;
      cols[i * 3 + 1] = type.color.g;
      cols[i * 3 + 2] = type.color.b;
      szs[i] = type.size;
    }

    return { positions: pos, colors: cols, sizes: szs };
  }, []);

  useFrame((state) => {
    if (!particlesRef.current) return;

    const time = state.clock.elapsedTime;
    const windX = Math.sin(time * 0.1) * 0.02;
    const windZ = Math.cos(time * 0.1) * 0.02;
    const windY = 0.01;

    const positionsAttr = particlesRef.current.geometry.attributes.position;

    for (let i = 0; i < particleCount; i++) {
      positionsAttr.array[i * 3] += windX;
      positionsAttr.array[i * 3 + 1] += windY;
      positionsAttr.array[i * 3 + 2] += windZ;

      if (positionsAttr.array[i * 3] > 100) positionsAttr.array[i * 3] = -100;
      if (positionsAttr.array[i * 3] < -100) positionsAttr.array[i * 3] = 100;
      if (positionsAttr.array[i * 3 + 1] > 90) positionsAttr.array[i * 3 + 1] = 10;
      if (positionsAttr.array[i * 3 + 2] > 100) positionsAttr.array[i * 3 + 2] = -100;
      if (positionsAttr.array[i * 3 + 2] < -100) positionsAttr.array[i * 3 + 2] = 100;
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

// Simple chat console for global communication (NO agent access)
interface ChatMessage {
  id: string;
  user: string;
  message: string;
  timestamp: Date;
}

interface GlobalChatConsoleProps {
  user: UserProfile | null;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  isInputActive: boolean;
  onActivateInput: () => void;
  onDeactivateInput: () => void;
}

function GlobalChatConsole({
  user,
  isCollapsed,
  onToggleCollapse,
  isInputActive,
  onActivateInput,
  onDeactivateInput,
}: GlobalChatConsoleProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      user: 'System',
      message: 'Welcome to the Jungleverse space. Use global chat to communicate with other visitors.',
      timestamp: new Date(),
    },
  ]);
  const [inputValue, setInputValue] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isInputActive && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isInputActive]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    // Empty input - close chat
    if (!inputValue.trim()) {
      onDeactivateInput();
      inputRef.current?.blur();
      return;
    }

    if (!user) return;

    const newMessage: ChatMessage = {
      id: Date.now().toString(),
      user: user.full_name || user.username,
      message: inputValue.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, newMessage]);
    setInputValue('');

    // Close chat after sending
    onDeactivateInput();
    inputRef.current?.blur();

    // TODO: In production, send to WebSocket for real global chat
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onDeactivateInput();
      inputRef.current?.blur();
    }
  };

  return (
    <div className="rounded-lg border border-emerald-600/60 bg-[#0b1209]/90 shadow-[0_12px_40px_rgba(0,0,0,0.6)]">
      <div className="flex items-center justify-between border-b border-emerald-500/40 px-3 py-1 text-[11px] uppercase tracking-[0.3em] text-emerald-200">
        <div className="flex items-center gap-2 font-semibold">
          <span className="rounded border border-emerald-500/50 bg-black/30 px-2 py-0.5 text-[10px] tracking-[0.2em]">
            Global Chat
          </span>
          <span className="text-emerald-400/60 text-[9px] normal-case tracking-normal">
            Jungleverse Visitors
          </span>
        </div>
        <button
          type="button"
          onClick={onToggleCollapse}
          className="rounded border border-emerald-600/50 bg-black/30 p-1 text-emerald-200 transition hover:text-white"
        >
          {isCollapsed ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
        </button>
      </div>

      <div
        className={cn(
          'overflow-hidden transition-all duration-300',
          isCollapsed ? 'max-h-0' : 'max-h-[20rem]'
        )}
      >
        <div className="h-40 overflow-y-auto p-3 space-y-2">
          {messages.map((msg) => (
            <div key={msg.id} className="text-[12px]">
              <span className="font-semibold text-emerald-300">{msg.user}:</span>{' '}
              <span className="text-emerald-100/80">{msg.message}</span>
            </div>
          ))}
          <div ref={messagesEndRef} />
        </div>

        <form onSubmit={handleSubmit} className="border-t border-emerald-500/30 p-2">
          <input
            ref={inputRef}
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onFocus={onActivateInput}
            onKeyDown={handleKeyDown}
            placeholder="Type a message..."
            className="w-full bg-black/40 border border-emerald-500/30 rounded px-3 py-1.5 text-[12px] text-emerald-100 placeholder-emerald-500/50 focus:outline-none focus:border-emerald-400/60"
          />
        </form>
      </div>

      <div className="border-t border-emerald-500/30 px-3 py-1 text-[10px] text-emerald-200/80">
        {isCollapsed ? 'Press Enter to open chat' : 'Type message + Enter to send · Empty Enter to close'}
      </div>
    </div>
  );
}

type AuthState = 'loading' | 'authenticated' | 'error';

export function EmbedVirtualEnvironmentPage() {
  const [searchParams] = useSearchParams();
  const token = searchParams.get('token');

  const [authState, setAuthState] = useState<AuthState>('loading');
  const [user, setUser] = useState<UserProfile | null>(null);
  const [errorMessage, setErrorMessage] = useState<string>('');

  const allProjects = useMemo(() => generateProjects(safeProjectList), []);

  // Find the Jungleverse project
  const jungleverseProject = useMemo(() => {
    return allProjects.find(
      (p) => p.name.toLowerCase() === JUNGLEVERSE_PROJECT_NAME
    ) || (allProjects.length > 0 ? allProjects[0] : null);
  }, [allProjects]);

  // Initial spawn position near Jungleverse building
  const initialPosition: [number, number, number] = useMemo(() => {
    if (jungleverseProject) {
      return [
        jungleverseProject.position[0],
        SPAWN_HEIGHT,
        jungleverseProject.position[2] + 40, // Spawn in front of building
      ];
    }
    return [0, SPAWN_HEIGHT, 0];
  }, [jungleverseProject]);

  // Movement boundary center (Jungleverse building position)
  const boundaryCenter = useMemo(() => {
    if (jungleverseProject) {
      return { x: jungleverseProject.position[0], z: jungleverseProject.position[2] };
    }
    return { x: 0, z: 0 };
  }, [jungleverseProject]);

  const [selectedProject, setSelectedProject] = useState<ProjectData | null>(null);
  const [userPosition, setUserPosition] = useState<[number, number, number]>(initialPosition);
  const [isChatCollapsed, setIsChatCollapsed] = useState(false);
  const [isChatInputActive, setIsChatInputActive] = useState(false);

  // Validate token on mount
  useEffect(() => {
    async function authenticate() {
      if (!token) {
        setAuthState('error');
        setErrorMessage('No authentication token provided');
        return;
      }

      try {
        const userProfile = await validateExternalToken(token);
        if (userProfile) {
          setUser(userProfile);
          setAuthState('authenticated');
        } else {
          setAuthState('error');
          setErrorMessage('Invalid or expired token');
        }
      } catch {
        setAuthState('error');
        setErrorMessage('Authentication failed');
      }
    }

    authenticate();
  }, [token]);

  const handleSelect = useCallback((project: ProjectData) => {
    setSelectedProject(project);
  }, []);

  // Position change handler with boundary enforcement
  const handleUserPositionChange = useCallback((vector: THREE.Vector3) => {
    // Enforce movement boundary around Jungleverse building
    const dx = vector.x - boundaryCenter.x;
    const dz = vector.z - boundaryCenter.z;
    const distance = Math.sqrt(dx * dx + dz * dz);

    if (distance > MOVEMENT_BOUNDARY_RADIUS) {
      // Clamp position to boundary
      const angle = Math.atan2(dz, dx);
      vector.x = boundaryCenter.x + Math.cos(angle) * MOVEMENT_BOUNDARY_RADIUS;
      vector.z = boundaryCenter.z + Math.sin(angle) * MOVEMENT_BOUNDARY_RADIUS;
    }

    setUserPosition([vector.x, vector.y, vector.z]);
  }, [boundaryCenter]);

  // Handle Enter key to open chat
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Enter' && !isChatInputActive) {
        event.preventDefault();
        setIsChatCollapsed(false);
        setIsChatInputActive(true);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isChatInputActive]);

  // Loading state
  if (authState === 'loading') {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-black text-white">
        <div className="text-center">
          <div className="mb-4 h-8 w-8 animate-spin rounded-full border-2 border-emerald-400 border-t-transparent mx-auto" />
          <p className="text-emerald-400">Entering Jungleverse...</p>
        </div>
      </div>
    );
  }

  // Error state
  if (authState === 'error') {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-black text-white">
        <div className="text-center">
          <p className="text-red-400 text-lg mb-2">Access Denied</p>
          <p className="text-gray-400 text-sm">{errorMessage}</p>
        </div>
      </div>
    );
  }

  // No Jungleverse project found
  if (!jungleverseProject) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-black text-white">
        <div className="text-center">
          <p className="text-amber-400 text-lg mb-2">Space Not Found</p>
          <p className="text-gray-400 text-sm">The Jungleverse space is not available.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="relative h-screen w-screen bg-black text-white overflow-hidden">
      {/* User info badge */}
      <div className="absolute top-4 left-4 z-10 rounded-lg border border-emerald-500/30 bg-black/80 px-3 py-2 backdrop-blur-sm">
        <p className="text-xs text-emerald-400">
          {user?.full_name || user?.username || 'Visitor'}
        </p>
        <p className="text-[10px] text-emerald-400/60">Jungleverse Space</p>
      </div>

      <Canvas
        camera={{ position: [initialPosition[0] + 40, initialPosition[1] + 30, initialPosition[2] + 40], fov: 60 }}
        shadows
        gl={{ antialias: true, toneMapping: THREE.ACESFilmicToneMapping }}
      >
        <color attach="background" args={['#030508']} />
        <fog attach="fog" args={['#030508', 80, 250]} />

        <Suspense fallback={null}>
          <AtmosphericLighting />
          <Environment preset="night" />
          <Stars radius={200} depth={40} count={1500} factor={4} saturation={0} fade speed={0.5} />

          <Grid
            args={[300, 300]}
            position={[boundaryCenter.x, 0, boundaryCenter.z]}
            cellSize={2}
            sectionSize={10}
            infiniteGrid
            fadeDistance={150}
            fadeStrength={3}
            cellColor="#0a2740"
            sectionColor="#0df2ff"
          />

          <EnhancedParticles />

          {/* Only render Jungleverse building */}
          <ProjectBuilding
            name={jungleverseProject.name}
            position={jungleverseProject.position}
            energy={jungleverseProject.energy}
            isSelected={selectedProject?.name === jungleverseProject.name}
            onSelect={() => handleSelect(jungleverseProject)}
          />

          {/* User avatar with boundary enforcement */}
          <UserAvatar
            initialPosition={initialPosition}
            color="#10b981" // Emerald color for Jungleverse visitors
            onPositionChange={handleUserPositionChange}
            isSuspended={isChatInputActive}
            canFly
            // Movement boundary is enforced in handleUserPositionChange
          />
        </Suspense>
      </Canvas>

      {/* Chat console - NO agent access, global chat only */}
      <div className="pointer-events-auto absolute bottom-4 left-4 w-[min(26rem,calc(100%-2rem))]">
          <GlobalChatConsole
            user={user}
            isCollapsed={isChatCollapsed}
            onToggleCollapse={() => setIsChatCollapsed(!isChatCollapsed)}
            isInputActive={isChatInputActive}
            onActivateInput={() => setIsChatInputActive(true)}
            onDeactivateInput={() => setIsChatInputActive(false)}
          />
      </div>

      {/* Boundary warning */}
      {(() => {
        const dx = userPosition[0] - boundaryCenter.x;
        const dz = userPosition[2] - boundaryCenter.z;
        const distance = Math.sqrt(dx * dx + dz * dz);
        const nearBoundary = distance > MOVEMENT_BOUNDARY_RADIUS * 0.85;

        if (nearBoundary) {
          return (
            <div className="pointer-events-none absolute top-20 inset-x-0 flex justify-center">
              <div className="rounded-full border border-amber-400/40 bg-black/70 px-4 py-1.5 text-xs uppercase tracking-widest text-amber-200">
                Approaching boundary limit
              </div>
            </div>
          );
        }
        return null;
      })()}

    </div>
  );
}

export default EmbedVirtualEnvironmentPage;
