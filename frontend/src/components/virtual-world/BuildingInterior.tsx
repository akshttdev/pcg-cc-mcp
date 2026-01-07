import { Suspense, useRef, useState, useEffect, useMemo } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Text, useGLTF } from '@react-three/drei';
import * as THREE from 'three';
import { BUILDING_THEMES, BuildingTheme, BuildingType } from '@/lib/virtual-world/buildingTypes';
import { INTERIOR_CAMERA, INTERIOR_ROOM } from '@/lib/virtual-world/constants';
import { UserAvatar } from '@/components/virtual-world/UserAvatar';
import { useAuth } from '@/contexts/AuthContext';

interface CustomEnvironmentConfig {
  url: string;
  scale: number;
  position: [number, number, number];
  cameraPosition?: [number, number, number];
  cameraFov?: number;
  spawnPosition?: [number, number, number];
}

const PROJECT_ENVIRONMENTS: Record<string, CustomEnvironmentConfig> = {
  'Fine Art Society': {
    url: '/environments/fine-art-gallery.glb',
    scale: 4,
    position: [0, 0, 0],
    cameraPosition: [0, 5, 0],
    spawnPosition: [0, 0.7, 0],
  },
};

interface InteriorProject {
  name: string;
  energy: number;
  type: BuildingType;
}

interface BuildingInteriorProps {
  project: InteriorProject;
  onExit: () => void;
  playerColor: string;
}

export function BuildingInterior({ project, onExit, playerColor }: BuildingInteriorProps) {
  const { user } = useAuth();
  const isAdmin = user?.is_admin ?? false;
  const theme = BUILDING_THEMES[project.type];
  const energyPercent = (project.energy * 100).toFixed(1);
  const customEnv = PROJECT_ENVIRONMENTS[project.name];
  const [loading, setLoading] = useState(!!customEnv);
  const cameraPosition = customEnv?.cameraPosition ?? INTERIOR_CAMERA.position;
  const cameraFov = customEnv?.cameraFov ?? INTERIOR_CAMERA.fov;
  const spawnPosition: [number, number, number] = customEnv?.spawnPosition ?? [0, 1.5, 10];

  return (
    <div className="pointer-events-auto absolute inset-0 z-30 bg-black/90 backdrop-blur">
      {customEnv && loading && (
        <div className="absolute inset-0 z-50 flex flex-col items-center justify-center bg-black/85 text-center text-cyan-200">
          <div className="mb-4 h-12 w-12 animate-spin rounded-full border-4 border-cyan-500 border-t-transparent" />
          <p className="text-lg">Loading {project.name} environment…</p>
          <p className="text-sm text-cyan-200/70">High-resolution gallery assets, please wait.</p>
        </div>
      )}

      <Canvas
        shadows
        camera={{ position: cameraPosition, fov: cameraFov }}
      >
        <color attach="background" args={['#05070e']} />
        {customEnv ? (
          <>
            <ambientLight intensity={0.7} color="#f5d5b5" />
            <directionalLight
              position={[12, 18, 6]}
              intensity={1.4}
              castShadow
              shadow-mapSize-width={2048}
              shadow-mapSize-height={2048}
            />
            <pointLight position={[0, 10, 0]} intensity={1.2} color="#ffe7cc" distance={60} />
            <Suspense fallback={null}>
              <GLBEnvironment
                url={customEnv.url}
                scale={customEnv.scale}
                position={customEnv.position}
                onLoaded={() => setLoading(false)}
              />
            </Suspense>
          </>
        ) : (
          <>
            <fog attach="fog" args={[theme.interior.wallColor, 10, 80]} />
            <ambientLight intensity={0.4} color={theme.interior.glowColor} />
            <spotLight
              position={[10, 25, 10]}
              intensity={1.5}
              angle={Math.PI / 4}
              penumbra={0.6}
              color={theme.accentColor}
              castShadow
            />
            <pointLight position={[0, 8, 0]} intensity={1.2} color={theme.hologramColor} distance={50} />

            <Suspense fallback={null}>
              <InteriorRoom project={project} theme={theme} />
            </Suspense>
          </>
        )}

        <UserAvatar
          initialPosition={spawnPosition}
          color={playerColor}
          isAdmin={isAdmin}
          onInteract={onExit}
          canFly={true}
          buildings={[]}
          isSuspended={false}
          baseFloorHeight={0}
        />
      </Canvas>

      <div className="pointer-events-none absolute inset-x-0 top-10 text-center">
        <p className="text-xs uppercase tracking-[0.4em] text-cyan-200/80">Interior Access</p>
        <p className="text-3xl font-semibold text-white">{project.name}</p>
        <p className="text-sm text-cyan-200/70">Energy {energyPercent}% · Building Type {project.type}</p>
      </div>

      <div className="pointer-events-auto absolute top-6 right-8 flex gap-3">
        <button
          type="button"
          onClick={onExit}
          className="rounded-lg border border-cyan-400/60 bg-black/40 px-4 py-2 text-xs uppercase tracking-[0.35em] text-cyan-100 transition hover:bg-cyan-500/10"
        >
          Exit To Grid (Esc)
        </button>
      </div>

      <div className="pointer-events-none absolute bottom-8 left-8 max-w-sm text-sm text-cyan-100/80">
        <p className="font-semibold uppercase tracking-[0.3em] text-cyan-200/70 mb-2">Interior Metrics</p>
        <p>Agent uplinks synchronized. Use Esc or the button to return to the monumental grid.</p>
      </div>
    </div>
  );
}

function InteriorRoom({ project, theme }: { project: InteriorProject; theme: BuildingTheme }) {
  const wallOffset = INTERIOR_ROOM.width / 2;
  const energyColumnHeight = 4 + project.energy * 6;

  return (
    <group>
      {/* Floor */}
      <mesh rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <planeGeometry args={[INTERIOR_ROOM.width, INTERIOR_ROOM.depth]} />
        <meshStandardMaterial
          color={theme.interior.floorColor}
          metalness={0.85}
          roughness={0.25}
          emissive={theme.interior.glowColor}
          emissiveIntensity={0.05}
        />
      </mesh>

      {/* Walls */}
      {[
        [0, INTERIOR_ROOM.height / 2, -wallOffset] as [number, number, number],
        [0, INTERIOR_ROOM.height / 2, wallOffset] as [number, number, number],
        [-wallOffset, INTERIOR_ROOM.height / 2, 0] as [number, number, number],
        [wallOffset, INTERIOR_ROOM.height / 2, 0] as [number, number, number],
      ].map((wallPosition, index) => (
        <mesh
          key={`wall-${index}`}
          position={wallPosition}
          rotation={index < 2 ? [0, 0, 0] : [0, Math.PI / 2, 0]}
        >
          <planeGeometry args={[INTERIOR_ROOM.width, INTERIOR_ROOM.height]} />
          <meshStandardMaterial
            color={theme.interior.wallColor}
            transparent
            opacity={0.65}
            metalness={0.15}
            roughness={0.4}
          />
        </mesh>
      ))}

      {/* Ceiling light strip */}
      <mesh position={[0, INTERIOR_ROOM.height - 1, 0]}>
        <boxGeometry args={[INTERIOR_ROOM.width, 0.3, INTERIOR_ROOM.depth]} />
        <meshBasicMaterial color={theme.interior.glowColor} transparent opacity={0.08} />
      </mesh>

      {/* Central holo table */}
      <mesh position={[0, 1, 0]} castShadow>
        <cylinderGeometry args={[4, 4, 1.2, 32]} />
        <meshStandardMaterial color={theme.baseColor} metalness={0.9} roughness={0.2} />
      </mesh>
      <mesh position={[0, 1.8, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[2.5, 3.3, 64]} />
        <meshBasicMaterial color={theme.hologramColor} transparent opacity={0.5} />
      </mesh>

      {/* Energy column */}
      <EnergyColumn height={energyColumnHeight} color={theme.hologramColor} />

      {/* Portal back to grid */}
      <mesh position={[0, 6, INTERIOR_ROOM.depth / 2 - 0.5]} rotation={[0, Math.PI, 0]}>
        <planeGeometry args={[8, 12]} />
        <meshBasicMaterial color={theme.hologramColor} transparent opacity={0.35} />
      </mesh>

      {/* Project label inside room */}
      <Text
        position={[0, 7, -INTERIOR_ROOM.depth / 2 + 2]}
        fontSize={2}
        color={theme.hologramColor}
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.1}
        outlineColor="#000000"
      >
        {project.name}
      </Text>

      {/* Holographic panels */}
      <HologramPanel
        position={[-10, 5, -8] as [number, number, number]}
        title="Energy"
        value={`${(project.energy * 100).toFixed(1)}%`}
        color={theme.hologramColor}
      />
      <HologramPanel
        position={[10, 5, -8] as [number, number, number]}
        title="Agents"
        value="03 On Duty"
        color={theme.interior.agentColor}
      />
      <HologramPanel
        position={[0, 4.5, 10] as [number, number, number]}
        title="Status"
        value="SYNCHRONIZED"
        color={theme.accentColor}
      />

      {/* Data streams */}
      {[-6, 0, 6].map((x) => (
        <DataStream
          key={`stream-${x}`}
          position={[x, 0, -5] as [number, number, number]}
          color={theme.interior.glowColor}
        />
      ))}
    </group>
  );
}

function EnergyColumn({ height, color }: { height: number; color: string }) {
  const columnRef = useRef<THREE.Mesh>(null);
  const baseHeight = height;

  useFrame((state) => {
    if (!columnRef.current) return;
    const t = state.clock.elapsedTime;
    const scale = 1 + Math.sin(t * 3) * 0.04;
    columnRef.current.scale.set(1, scale, 1);
  });

  return (
    <mesh ref={columnRef} position={[0, 1 + baseHeight / 2, 0]}
      rotation={[0, 0, 0]}
    >
      <cylinderGeometry args={[1.6, 1.8, baseHeight, 32]} />
      <meshBasicMaterial color={color} transparent opacity={0.35} />
    </mesh>
  );
}

function HologramPanel({
  position,
  title,
  value,
  color,
}: {
  position: [number, number, number];
  title: string;
  value: string;
  color: string;
}) {
  return (
    <group position={position}>
      <mesh rotation={[-0.1, 0, 0]}>
        <planeGeometry args={[6, 4]} />
        <meshBasicMaterial color={color} transparent opacity={0.2} />
      </mesh>
      <Text position={[0, 1.2, 0.1]} fontSize={0.7} color={color} anchorX="center" anchorY="bottom">
        {title}
      </Text>
      <Text position={[0, 0.1, 0.1]} fontSize={1.2} color="#ffffff" anchorX="center" anchorY="middle">
        {value}
      </Text>
    </group>
  );
}

function DataStream({ position, color }: { position: [number, number, number]; color: string }) {
  const streamRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (!streamRef.current) return;
    const t = state.clock.elapsedTime;
    streamRef.current.position.y = 0.5 + Math.sin(t * 4 + position[0]) * 0.4;
    const material = streamRef.current.material as THREE.MeshBasicMaterial;
    material.opacity = 0.2 + Math.sin(t * 5 + position[0]) * 0.1;
  });

  return (
    <mesh ref={streamRef} position={position}>
      <boxGeometry args={[0.2, 4, 0.2]} />
      <meshBasicMaterial color={color} transparent opacity={0.3} />
    </mesh>
  );
}

function GLBEnvironment({ url, scale, position, onLoaded }: CustomEnvironmentConfig & { onLoaded?: () => void }) {
  const { scene } = useGLTF(url);
  const clonedScene = useMemo(() => scene.clone(), [scene]);

  useEffect(() => {
    // Enable shadows on all meshes
    clonedScene.traverse((child) => {
      if (child instanceof THREE.Mesh) {
        child.castShadow = true;
        child.receiveShadow = true;
      }
    });
    // Signal that loading is complete
    onLoaded?.();
  }, [clonedScene, onLoaded]);

  return (
    <group position={position} scale={[scale, scale, scale]}>
      <primitive object={clonedScene} />
    </group>
  );
}

useGLTF.preload('/environments/fine-art-gallery.glb');
