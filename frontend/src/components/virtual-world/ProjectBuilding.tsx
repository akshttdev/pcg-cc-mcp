import { useState, useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import { BUILDING_THEMES, getBuildingType } from '@/lib/virtual-world/buildingTypes';
import { ENTRY_PROMPT_HEIGHT } from '@/lib/virtual-world/constants';

const CONTAINER_WIDTH = 50;
const CONTAINER_LENGTH = 100;
const CONTAINER_HEIGHT = 20;
const HALF_HEIGHT = CONTAINER_HEIGHT / 2;
const HALF_WIDTH = CONTAINER_WIDTH / 2;
const HALF_LENGTH = CONTAINER_LENGTH / 2;
const FOOTPRINT_RADIUS = Math.sqrt(HALF_WIDTH ** 2 + HALF_LENGTH ** 2);

interface ProjectBuildingProps {
  name: string;
  position: [number, number, number];
  energy: number;
  isSelected: boolean;
  onSelect: () => void;
  isEnterTarget?: boolean;
  entryHotkey?: string;
}

function DevTower({ position, energy, isSelected, hovered }: any) {
  const groupRef = useRef<THREE.Group>(null);
  const emissiveBoost = energy * (hovered || isSelected ? 1.5 : 1);

  useFrame(() => {
    if (groupRef.current && (hovered || isSelected)) {
      groupRef.current.rotation.y += 0.0005;
    }
  });

  return (
    <group ref={groupRef} position={position}>
      <mesh castShadow receiveShadow>
        <boxGeometry args={[CONTAINER_WIDTH, CONTAINER_HEIGHT, CONTAINER_LENGTH]} />
        <meshPhysicalMaterial
          color="#0a2f4a"
          transmission={0.25}
          thickness={1.2}
          roughness={0.08}
          metalness={0.9}
          emissive="#0080ff"
          emissiveIntensity={emissiveBoost}
          clearcoat={1}
          clearcoatRoughness={0.08}
        />
      </mesh>

      {Array.from({ length: 5 }).map((_, i) => (
        <mesh key={i} position={[0, -HALF_HEIGHT + (i + 1) * (CONTAINER_HEIGHT / 6), HALF_LENGTH + 0.2]}>
          <boxGeometry args={[CONTAINER_WIDTH - 4, 0.3, 0.6]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={0.6} />
        </mesh>
      ))}

      {[
        [-HALF_WIDTH, 0, -HALF_LENGTH],
        [-HALF_WIDTH, 0, HALF_LENGTH],
        [HALF_WIDTH, 0, -HALF_LENGTH],
        [HALF_WIDTH, 0, HALF_LENGTH],
      ].map(([x, y, z], idx) => (
        <mesh key={idx} position={[x, y, z]}>
          <boxGeometry args={[2, CONTAINER_HEIGHT + 6, 2]} />
          <meshStandardMaterial
            color="#003a5a"
            emissive="#00c9ff"
            emissiveIntensity={emissiveBoost * 0.6}
            metalness={0.9}
            roughness={0.2}
          />
        </mesh>
      ))}

      <mesh position={[0, HALF_HEIGHT + 1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[HALF_WIDTH * 0.6, HALF_WIDTH * 0.8, 16]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.5} />
      </mesh>

      <rectAreaLight
        position={[0, HALF_HEIGHT, 0]}
        width={CONTAINER_WIDTH}
        height={CONTAINER_LENGTH}
        intensity={1 + energy * 2}
        color="#0080ff"
      />
    </group>
  );
}

function CreativeStudio({ position, energy, isSelected, hovered }: any) {
  const emissiveBoost = energy * (hovered || isSelected ? 1.4 : 1);

  return (
    <group position={position}>
      <mesh castShadow receiveShadow>
        <boxGeometry args={[CONTAINER_WIDTH, CONTAINER_HEIGHT, CONTAINER_LENGTH]} />
        <meshPhysicalMaterial
          color="#4a2f0a"
          transmission={0.15}
          thickness={0.8}
          roughness={0.25}
          metalness={0.7}
          emissive="#ff8000"
          emissiveIntensity={emissiveBoost}
          clearcoat={1}
        />
      </mesh>

      {[-HALF_WIDTH + 6, 0, HALF_WIDTH - 6].map((x, i) => (
        <mesh key={i} position={[x, 3, HALF_LENGTH + 0.3]}>
          <planeGeometry args={[12, 10]} />
          <meshBasicMaterial color="#ffaa00" transparent opacity={0.35} />
        </mesh>
      ))}

      <mesh position={[0, HALF_HEIGHT + 0.5, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[CONTAINER_WIDTH - 6, CONTAINER_LENGTH - 10]} />
        <meshStandardMaterial color="#2a1f0a" metalness={0.4} roughness={0.6} />
      </mesh>

      {[-HALF_WIDTH + 4, HALF_WIDTH - 4].map((x, idx) => (
        <mesh key={idx} position={[x, HALF_HEIGHT + 0.6, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <planeGeometry args={[6, CONTAINER_LENGTH - 12]} />
          <meshStandardMaterial color="#1c4d32" emissive="#1b8f41" emissiveIntensity={0.2} />
        </mesh>
      ))}

      <rectAreaLight
        position={[0, HALF_HEIGHT - 2, 0]}
        width={CONTAINER_WIDTH}
        height={CONTAINER_LENGTH}
        intensity={1 + energy * 1.5}
        color="#ff8000"
      />
    </group>
  );
}

function InfrastructureHub({ position, energy, isSelected, hovered }: any) {
  const emissiveBoost = energy * 0.4 * (hovered || isSelected ? 1.5 : 1);

  return (
    <group position={position}>
      {[-HALF_HEIGHT + 3, 0, HALF_HEIGHT - 3].map((y, i) => (
        <mesh key={i} position={[0, y, 0]} castShadow receiveShadow>
          <boxGeometry args={[CONTAINER_WIDTH - 10, CONTAINER_HEIGHT / 3, CONTAINER_LENGTH - 20]} />
          <meshStandardMaterial
            color="#181818"
            roughness={0.8}
            metalness={0.95}
            emissive="#ff0000"
            emissiveIntensity={emissiveBoost}
          />
        </mesh>
      ))}

      {Array.from({ length: 6 }).map((_, i) => (
        <mesh key={i} position={[0, -HALF_HEIGHT + 2 + i * 3, HALF_LENGTH + 0.4]}>
          <planeGeometry args={[CONTAINER_WIDTH - 12, 1.5]} />
          <meshStandardMaterial color="#ff4d4d" emissive="#ff0000" emissiveIntensity={0.6} />
        </mesh>
      ))}

      <mesh position={[0, 0, 0]}>
        <cylinderGeometry args={[4, 4, CONTAINER_HEIGHT, 16]} />
        <meshBasicMaterial color="#ff0000" transparent opacity={0.4} />
        <pointLight intensity={1.5 * energy} color="#ff0000" distance={60} />
      </mesh>
    </group>
  );
}

function ResearchFacility({ position, energy, isSelected, hovered }: any) {
  const groupRef = useRef<THREE.Group>(null);
  const emissiveBoost = energy * (hovered || isSelected ? 1.4 : 1);

  useFrame(() => {
    if (groupRef.current) {
      groupRef.current.rotation.y += 0.0008;
    }
  });

  return (
    <group ref={groupRef} position={position}>
      <mesh castShadow receiveShadow>
        <boxGeometry args={[CONTAINER_WIDTH, CONTAINER_HEIGHT * 0.6, CONTAINER_LENGTH]} />
        <meshPhysicalMaterial
          color="#2a0a4a"
          transmission={0.5}
          thickness={0.9}
          roughness={0.08}
          metalness={0.4}
          emissive="#aa00ff"
          emissiveIntensity={emissiveBoost}
          transparent
          opacity={0.7}
        />
      </mesh>

      <mesh position={[0, HALF_HEIGHT, 0]}>
        <sphereGeometry args={[Math.min(HALF_WIDTH, HALF_LENGTH) * 0.9, 48, 32, 0, Math.PI * 2, 0, Math.PI / 2]} />
        <meshPhysicalMaterial
          color="#3a0f5c"
          transmission={0.85}
          thickness={0.6}
          roughness={0.03}
          emissive="#d400ff"
          emissiveIntensity={emissiveBoost}
          transparent
          opacity={0.6}
        />
      </mesh>

      {[0, 120, 240].map((angle, i) => {
        const rad = (angle * Math.PI) / 180;
        const radius = HALF_WIDTH * 0.7;
        return (
          <mesh key={i} position={[Math.cos(rad) * radius, HALF_HEIGHT - 2 + i, Math.sin(rad) * radius]}>
            <octahedronGeometry args={[3, 0]} />
            <meshStandardMaterial
              color="#6a0a9a"
              emissive="#ff00ff"
              emissiveIntensity={energy}
              metalness={0.9}
              roughness={0.1}
            />
          </mesh>
        );
      })}

      <pointLight position={[0, HALF_HEIGHT + 4, 0]} intensity={2 * energy} color="#aa00ff" distance={80} />
    </group>
  );
}

export function ProjectBuilding({
  name,
  position,
  energy,
  isSelected,
  onSelect,
  isEnterTarget = false,
  entryHotkey = 'E',
}: ProjectBuildingProps) {
  const [hovered, setHovered] = useState(false);
  const buildingType = getBuildingType(name);
  const theme = BUILDING_THEMES[buildingType];

  const elevatedPosition = useMemo(
    () => [position[0], position[1] + HALF_HEIGHT, position[2]] as [number, number, number],
    [position],
  );

  const entranceDirection = useMemo(() => {
    const dir = new THREE.Vector3(-position[0], 0, -position[2]);
    if (dir.lengthSq() === 0) {
      dir.set(0, 0, 1);
    }
    return dir.normalize();
  }, [position]);

  const doorPosition = useMemo(() => {
    const offset = entranceDirection.clone().multiplyScalar(HALF_LENGTH + 0.5);
    return [position[0] + offset.x, position[1] + 5, position[2] + offset.z] as [
      number,
      number,
      number,
    ];
  }, [entranceDirection, position]);

  const doorRotation = useMemo(() => Math.atan2(entranceDirection.x, entranceDirection.z), [entranceDirection]);

  const walkwayPosition = useMemo(() => {
    const offset = entranceDirection.clone().multiplyScalar(HALF_LENGTH + 22);
    return [position[0] + offset.x, position[1] + 0.11, position[2] + offset.z] as [number, number, number];
  }, [entranceDirection, position]);

  const labelColor = hovered || isSelected ? '#ffffff' : theme.labelColor;

  const showEnterPrompt = isEnterTarget && Boolean(entryHotkey);

  const props = {
    position: elevatedPosition,
    energy,
    isSelected,
    hovered,
  };

  return (
    <group
      onClick={(e) => {
        e.stopPropagation();
        onSelect();
      }}
      onPointerOver={(e) => {
        e.stopPropagation();
        setHovered(true);
        document.body.style.cursor = 'pointer';
      }}
      onPointerOut={(e) => {
        e.stopPropagation();
        setHovered(false);
        document.body.style.cursor = 'default';
      }}
    >
      {/* Render building based on type */}
      {buildingType === 'dev-tower' && <DevTower {...props} />}
      {buildingType === 'creative-studio' && <CreativeStudio {...props} />}
      {buildingType === 'infrastructure' && <InfrastructureHub {...props} />}
      {buildingType === 'research' && <ResearchFacility {...props} />}

      {/* Project name label (large, above building) */}
      <Text
        position={[position[0], position[1] + CONTAINER_HEIGHT + 30, position[2]]}
        fontSize={5}
        color={labelColor}
        anchorX="center"
        anchorY="bottom"
        outlineWidth={0.2}
        outlineColor="#001925"
      >
        {name}
      </Text>

      {/* Ground plate */}
      <mesh position={[position[0], position[1] + 0.1, position[2]]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <circleGeometry args={[FOOTPRINT_RADIUS + 10, 64]} />
        <meshStandardMaterial
          color="#0a1f35"
          metalness={0.8}
          roughness={0.3}
          emissive="#004080"
          emissiveIntensity={isSelected || hovered ? 0.5 : 0.2}
        />
      </mesh>

      {/* Entry walkway */}
      <mesh position={walkwayPosition} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[12, 40]} />
        <meshBasicMaterial
          color={theme.accentColor}
          transparent
          opacity={isEnterTarget ? 0.45 : 0.15}
        />
      </mesh>

      {/* Selection indicator ring */}
      {(isSelected || hovered) && (
        <mesh position={[position[0], position[1] + 0.2, position[2]]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[FOOTPRINT_RADIUS + 4, FOOTPRINT_RADIUS + 7, 64]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={isSelected ? 0.8 : 0.5} />
        </mesh>
      )}

      {/* Entry portal */}
      <mesh position={doorPosition} rotation={[0, doorRotation, 0]}>
        <planeGeometry args={[4, 8]} />
        <meshStandardMaterial
          color={theme.doorColor}
          emissive={isEnterTarget ? theme.hologramColor : '#00121d'}
          emissiveIntensity={isEnterTarget ? 1.2 : 0.2}
          transparent
          opacity={0.85}
          metalness={0.2}
          roughness={0.3}
        />
      </mesh>

      {showEnterPrompt && (
        <Text
          position={[doorPosition[0], doorPosition[1] + ENTRY_PROMPT_HEIGHT, doorPosition[2]]}
          fontSize={1.2}
          color={theme.hologramColor}
          anchorX="center"
          anchorY="bottom"
          outlineWidth={0.05}
          outlineColor="#000a10"
        >
          Press {entryHotkey} to enter
        </Text>
      )}
    </group>
  );
}
