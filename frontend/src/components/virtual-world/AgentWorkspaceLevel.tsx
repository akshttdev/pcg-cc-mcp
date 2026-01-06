import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import {
  WORKSPACE_FLOOR_Y,
  WORKSPACE_OUTER_RADIUS,
  WORKSPACE_INNER_RADIUS,
  COMMAND_CENTER_Y,
} from '@/lib/virtual-world/spatialSystem';

// Re-export for backward compatibility
export { WORKSPACE_FLOOR_Y, COMMAND_CENTER_Y };

// Platform constants
const PLATFORM_THICKNESS = 1;
const BAY_COUNT = 8;
const BAY_ANGLE = (Math.PI * 2) / BAY_COUNT;
const BAY_HEIGHT = COMMAND_CENTER_Y - WORKSPACE_FLOOR_Y; // 15 units (Y=65 to Y=80)
const DIVIDER_THICKNESS = 0.25;
const WALL_HEIGHT = BAY_HEIGHT;

// Colors
const FLOOR_COLOR = '#1a1a2e';
const WALL_COLOR = '#1a1a2e';
const GLASS_COLOR = '#88ccff';
const ACCENT_COLOR = '#00ffff';
const EMPTY_BAY_COLOR = '#1a1a2a';
const EMPTY_GLOW_COLOR = '#333344';

interface AgentBayConfig {
  id: string;
  name: string;
  agentName?: string;
  color: string;
  glowColor: string;
  occupied: boolean;
}

const AGENT_BAYS: AgentBayConfig[] = [
  { id: 'bay-0', name: 'Maci', agentName: 'Maci', color: '#ff6b9d', glowColor: '#ff69b4', occupied: true },
  { id: 'bay-1', name: 'Editron', agentName: 'Editron', color: '#4a9eff', glowColor: '#4169e1', occupied: true },
  { id: 'bay-2', name: 'Bowser', agentName: 'Bowser', color: '#9b59b6', glowColor: '#8a2be2', occupied: true },
  { id: 'bay-3', name: 'Auri', agentName: 'Auri', color: '#ffd700', glowColor: '#ffaa00', occupied: true },
  { id: 'bay-4', name: 'Empty Bay 5', color: EMPTY_BAY_COLOR, glowColor: EMPTY_GLOW_COLOR, occupied: false },
  { id: 'bay-5', name: 'Empty Bay 6', color: EMPTY_BAY_COLOR, glowColor: EMPTY_GLOW_COLOR, occupied: false },
  { id: 'bay-6', name: 'Empty Bay 7', color: EMPTY_BAY_COLOR, glowColor: EMPTY_GLOW_COLOR, occupied: false },
  { id: 'bay-7', name: 'Empty Bay 8', color: EMPTY_BAY_COLOR, glowColor: EMPTY_GLOW_COLOR, occupied: false },
];

export function AgentWorkspaceLevel() {
  return (
    <group position={[0, WORKSPACE_FLOOR_Y, 0]}>
      {/* Main ring platform with stairwell cutout */}
      <RingPlatformWithStairwell />

      {/* Ceiling removed - CommandCenter floor at Y=80 serves as the ceiling */}

      {/* Solid outer wall structure */}
      <OuterWallStructure />

      {/* Bay dividers (radial walls) */}
      {AGENT_BAYS.map((bay, index) => (
        <WorkspaceBay key={bay.id} config={bay} index={index} />
      ))}
    </group>
  );
}

/**
 * Solid floor - extends from hologram railing to outer wall
 * Hole is inside the hologram railing (R < 10)
 */
function RingPlatformWithStairwell() {
  const geometry = useMemo(() => {
    const shape = new THREE.Shape();

    // Outer circle - extends to outer wall
    shape.absarc(0, 0, WORKSPACE_OUTER_RADIUS, 0, Math.PI * 2, false);

    // Hole inside the hologram railing - railing is at R=10, hole is R=9.5
    const hologramHole = new THREE.Path();
    hologramHole.absarc(0, 0, 9.5, 0, Math.PI * 2, true);
    shape.holes.push(hologramHole);

    return new THREE.ExtrudeGeometry(shape, { depth: PLATFORM_THICKNESS, bevelEnabled: false });
  }, []);

  return (
    <mesh geometry={geometry} rotation={[-Math.PI / 2, 0, 0]} position={[0, -PLATFORM_THICKNESS, 0]} receiveShadow castShadow>
      <meshStandardMaterial color={FLOOR_COLOR} metalness={0.7} roughness={0.3} />
    </mesh>
  );
}

// CeilingWithStairwell removed - CommandCenter floor at Y=80 serves as ceiling

/**
 * Glass window wall - transparent with structural frames
 */
function OuterWallStructure() {
  const segments = 64;

  return (
    <group>
      {/* Main glass wall - highly transparent */}
      <mesh position={[0, WALL_HEIGHT / 2, 0]}>
        <cylinderGeometry args={[WORKSPACE_OUTER_RADIUS - 0.1, WORKSPACE_OUTER_RADIUS - 0.1, WALL_HEIGHT, segments, 1, true]} />
        <meshPhysicalMaterial
          color={GLASS_COLOR}
          metalness={0.0}
          roughness={0.05}
          transparent
          opacity={0.08}
          transmission={0.95}
          thickness={0.1}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* Structural frame rings - thin horizontal bars */}
      {[0.5, 4, 8, 11.5].map((y) => (
        <mesh key={y} position={[0, y, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[WORKSPACE_OUTER_RADIUS - 0.2, WORKSPACE_OUTER_RADIUS + 0.1, segments]} />
          <meshStandardMaterial color="#1a2a3a" metalness={0.7} roughness={0.3} />
        </mesh>
      ))}

      {/* Vertical frame posts - evenly spaced */}
      {Array.from({ length: 16 }).map((_, i) => {
        const angle = (i / 16) * Math.PI * 2;
        const x = Math.cos(angle) * WORKSPACE_OUTER_RADIUS;
        const z = Math.sin(angle) * WORKSPACE_OUTER_RADIUS;
        return (
          <mesh key={`post-${i}`} position={[x, WALL_HEIGHT / 2, z]} rotation={[0, -angle, 0]}>
            <boxGeometry args={[0.15, WALL_HEIGHT, 0.15]} />
            <meshStandardMaterial color="#1a2a3a" metalness={0.7} roughness={0.3} />
          </mesh>
        );
      })}

      {/* Accent lighting strip at top and bottom */}
      <mesh position={[0, WALL_HEIGHT - 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[WORKSPACE_OUTER_RADIUS - 0.25, WORKSPACE_OUTER_RADIUS - 0.05, segments]} />
        <meshBasicMaterial color={ACCENT_COLOR} transparent opacity={0.5} />
      </mesh>
      <mesh position={[0, 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[WORKSPACE_OUTER_RADIUS - 0.25, WORKSPACE_OUTER_RADIUS - 0.05, segments]} />
        <meshBasicMaterial color={ACCENT_COLOR} transparent opacity={0.3} />
      </mesh>
    </group>
  );
}

// Inner railing removed - the hologram railing is now part of SpiralStaircase component

/**
 * Individual workspace bay with divider wall
 */
interface WorkspaceBayProps {
  config: AgentBayConfig;
  index: number;
}

function WorkspaceBay({ config, index }: WorkspaceBayProps) {
  const pulseRef = useRef<THREE.PointLight>(null);

  const startAngle = index * BAY_ANGLE;
  const midAngle = startAngle + BAY_ANGLE / 2;
  const midRadius = (WORKSPACE_INNER_RADIUS + WORKSPACE_OUTER_RADIUS) / 2;

  useFrame((state) => {
    if (pulseRef.current && config.occupied) {
      pulseRef.current.intensity = 0.4 + Math.sin(state.clock.elapsedTime * 2 + index) * 0.15;
    }
  });

  return (
    <group>
      {/* Radial divider wall */}
      <RadialDividerWall angle={startAngle} />

      {/* Bay nameplate */}
      <Text
        position={[
          Math.cos(midAngle) * (midRadius - 3),
          BAY_HEIGHT - 1.5,
          Math.sin(midAngle) * (midRadius - 3)
        ]}
        rotation={[0, -midAngle + Math.PI, 0]}
        fontSize={0.9}
        color={config.occupied ? config.glowColor : '#666666'}
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.04}
        outlineColor="#000000"
      >
        {config.name}
      </Text>

      {/* Floor accent wedge */}
      <WedgeFloorAccent
        startAngle={startAngle + 0.03}
        endAngle={startAngle + BAY_ANGLE - 0.03}
        color={config.occupied ? config.color : EMPTY_BAY_COLOR}
        glowColor={config.occupied ? config.glowColor : EMPTY_GLOW_COLOR}
        occupied={config.occupied}
      />

      {/* Workspace ambient light */}
      <pointLight
        ref={pulseRef}
        position={[Math.cos(midAngle) * midRadius, BAY_HEIGHT - 2, Math.sin(midAngle) * midRadius]}
        color={config.glowColor}
        intensity={config.occupied ? 0.4 : 0.1}
        distance={18}
      />

      {/* Status indicator on floor */}
      <mesh position={[Math.cos(midAngle) * (WORKSPACE_INNER_RADIUS + 2.5), 0.1, Math.sin(midAngle) * (WORKSPACE_INNER_RADIUS + 2.5)]}>
        <sphereGeometry args={[0.18, 12, 12]} />
        <meshBasicMaterial color={config.occupied ? '#00ff00' : '#333333'} />
      </mesh>
    </group>
  );
}

/**
 * Radial divider wall (spoke from inner to outer)
 */
function RadialDividerWall({ angle }: { angle: number }) {
  const length = WORKSPACE_OUTER_RADIUS - WORKSPACE_INNER_RADIUS;
  const midRadius = (WORKSPACE_INNER_RADIUS + WORKSPACE_OUTER_RADIUS) / 2;
  const x = Math.cos(angle) * midRadius;
  const z = Math.sin(angle) * midRadius;
  const rotationY = Math.PI / 2 - angle;

  return (
    <mesh position={[x, WALL_HEIGHT / 2, z]} rotation={[0, rotationY, 0]} castShadow>
      <boxGeometry args={[DIVIDER_THICKNESS, WALL_HEIGHT, length]} />
      <meshStandardMaterial color={WALL_COLOR} metalness={0.5} roughness={0.5} />
    </mesh>
  );
}

/**
 * Wedge-shaped floor accent
 */
function WedgeFloorAccent({
  startAngle,
  endAngle,
  color,
  glowColor,
  occupied,
}: {
  startAngle: number;
  endAngle: number;
  color: string;
  glowColor: string;
  occupied: boolean;
}) {
  const geometry = useMemo(() => {
    const shape = new THREE.Shape();
    const innerR = WORKSPACE_INNER_RADIUS + 0.8;
    const outerR = WORKSPACE_OUTER_RADIUS - 0.8;
    const segments = 16;

    // Start point
    shape.moveTo(Math.cos(startAngle) * innerR, -Math.sin(startAngle) * innerR);

    // Outer arc
    for (let i = 0; i <= segments; i++) {
      const t = i / segments;
      const a = startAngle + (endAngle - startAngle) * t;
      shape.lineTo(Math.cos(a) * outerR, -Math.sin(a) * outerR);
    }

    // Inner arc (reverse)
    for (let i = segments; i >= 0; i--) {
      const t = i / segments;
      const a = startAngle + (endAngle - startAngle) * t;
      shape.lineTo(Math.cos(a) * innerR, -Math.sin(a) * innerR);
    }

    return new THREE.ShapeGeometry(shape);
  }, [startAngle, endAngle]);

  return (
    <mesh geometry={geometry} rotation={[-Math.PI / 2, 0, 0]} position={[0, 0.03, 0]} receiveShadow>
      <meshStandardMaterial
        color={color}
        metalness={occupied ? 0.6 : 0.3}
        roughness={occupied ? 0.4 : 0.7}
        emissive={occupied ? glowColor : '#000000'}
        emissiveIntensity={occupied ? 0.04 : 0}
      />
    </mesh>
  );
}

// DirectoryHologram removed - hologram visual is now in SpiralStaircase

// ============================================================================
// EXPORTS FOR NAVIGATION SYSTEM
// ============================================================================

// Re-export from spatial system
export { getFloorHeightAt as getRampYAtPosition } from '@/lib/virtual-world/spatialSystem';

export function getAgentBayPosition(agentName: string): [number, number, number] | null {
  const bayIndex = AGENT_BAYS.findIndex(bay => bay.agentName === agentName);
  if (bayIndex === -1) return null;

  const midAngle = bayIndex * BAY_ANGLE + BAY_ANGLE / 2;
  const radius = (WORKSPACE_INNER_RADIUS + WORKSPACE_OUTER_RADIUS) / 2;

  return [Math.cos(midAngle) * radius, WORKSPACE_FLOOR_Y + 1, Math.sin(midAngle) * radius];
}

export interface BayBounds {
  startAngle: number;
  endAngle: number;
  innerRadius: number;
  outerRadius: number;
  centerY: number;
}

export function getAgentBayBounds(agentName: string): BayBounds | null {
  const bayIndex = AGENT_BAYS.findIndex(bay => bay.agentName === agentName);
  if (bayIndex === -1) return null;

  return {
    startAngle: bayIndex * BAY_ANGLE,
    endAngle: (bayIndex + 1) * BAY_ANGLE,
    innerRadius: WORKSPACE_INNER_RADIUS + 1.5,
    outerRadius: WORKSPACE_OUTER_RADIUS - 1.5,
    centerY: WORKSPACE_FLOOR_Y + 1,
  };
}

export { AGENT_BAYS, WORKSPACE_OUTER_RADIUS as OUTER_RADIUS, WORKSPACE_INNER_RADIUS as INNER_RADIUS, BAY_ANGLE };
