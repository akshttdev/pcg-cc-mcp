import { useState, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

interface ProjectBuildingProps {
  name: string;
  position: [number, number, number];
  energy: number;
  isSelected: boolean;
  onSelect: () => void;
}

type BuildingType = 'dev-tower' | 'creative-studio' | 'infrastructure' | 'research' | 'command';

function getBuildingType(name: string): BuildingType {
  const lowerName = name.toLowerCase();

  // Development towers
  if (
    lowerName.includes('mcp') ||
    lowerName.includes('rs') ||
    lowerName.includes('code') ||
    lowerName.includes('api') ||
    lowerName.includes('frontend') ||
    lowerName.includes('backend')
  ) {
    return 'dev-tower';
  }

  // Creative studios
  if (
    lowerName.includes('jungle') ||
    lowerName.includes('brand') ||
    lowerName.includes('design') ||
    lowerName.includes('studio')
  ) {
    return 'creative-studio';
  }

  // Infrastructure
  if (
    lowerName.includes('ducknet') ||
    lowerName.includes('comfy') ||
    lowerName.includes('distribution') ||
    lowerName.includes('linux') ||
    lowerName.includes('infra')
  ) {
    return 'infrastructure';
  }

  // Research
  if (
    lowerName.includes('extract') ||
    lowerName.includes('lab') ||
    lowerName.includes('research') ||
    lowerName.includes('ai')
  ) {
    return 'research';
  }

  // Default to dev tower
  return 'dev-tower';
}

function DevTower({ position, energy, isSelected, hovered }: any) {
  const groupRef = useRef<THREE.Group>(null);

  useFrame(() => {
    if (groupRef.current && (isSelected || hovered)) {
      // Subtle rotation when selected/hovered
    }
  });

  return (
    <group ref={groupRef} position={position}>
      {/* Main tower */}
      <mesh castShadow receiveShadow>
        <boxGeometry args={[20, 40, 20]} />
        <meshPhysicalMaterial
          color="#0a2f4a"
          transmission={0.3}
          thickness={0.5}
          roughness={0.1}
          metalness={0.9}
          emissive="#0080ff"
          emissiveIntensity={energy * (hovered || isSelected ? 1.5 : 1)}
          clearcoat={1}
          clearcoatRoughness={0.1}
        />
      </mesh>

      {/* Floor divisions (visible levels) */}
      {Array.from({ length: 10 }).map((_, i) => (
        <mesh key={i} position={[0, -15 + i * 4, 10.1]}>
          <boxGeometry args={[19, 0.2, 0.1]} />
          <meshBasicMaterial color="#00ffff" />
        </mesh>
      ))}

      {/* Windows */}
      {Array.from({ length: 4 }).map((_, side) => {
        const angle = (side / 4) * Math.PI * 2;
        const x = Math.sin(angle) * 10.1;
        const z = Math.cos(angle) * 10.1;
        return (
          <group key={side}>
            {Array.from({ length: 8 }).map((_, i) => (
              <mesh key={i} position={[x, -12 + i * 5, z]} rotation={[0, angle, 0]}>
                <boxGeometry args={[3, 2, 0.1]} />
                <meshBasicMaterial
                  color="#00d9ff"
                  transparent
                  opacity={0.6 + Math.random() * 0.2}
                />
              </mesh>
            ))}
          </group>
        );
      })}

      {/* Roof landing pad */}
      <mesh position={[0, 21, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[6, 8, 8]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.6} />
      </mesh>

      {/* Data streams (particle effect placeholder) */}
      <mesh position={[0, 0, 0]}>
        <cylinderGeometry args={[0.1, 0.1, 40, 8]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.3} />
      </mesh>

      {/* Area light */}
      <rectAreaLight
        position={[0, 20, 0]}
        width={20}
        height={40}
        intensity={1 + energy * 2}
        color="#0080ff"
      />
    </group>
  );
}

function CreativeStudio({ position, energy, isSelected, hovered }: any) {
  return (
    <group position={position}>
      {/* Horizontal spread structure */}
      <mesh castShadow receiveShadow>
        <boxGeometry args={[30, 20, 25]} />
        <meshPhysicalMaterial
          color="#4a2f0a"
          transmission={0.2}
          thickness={0.5}
          roughness={0.2}
          metalness={0.7}
          emissive="#ff8000"
          emissiveIntensity={energy * (hovered || isSelected ? 1.5 : 1)}
          clearcoat={1}
        />
      </mesh>

      {/* Holographic displays on exterior */}
      {[-12, 0, 12].map((x, i) => (
        <mesh key={i} position={[x, 5, 12.6]}>
          <planeGeometry args={[6, 8]} />
          <meshBasicMaterial color="#ffaa00" transparent opacity={0.4} />
        </mesh>
      ))}

      {/* Terrace */}
      <mesh position={[0, 11, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <boxGeometry args={[28, 23, 0.5]} />
        <meshStandardMaterial color="#2a1f0a" metalness={0.5} roughness={0.5} />
      </mesh>

      {/* Warm lighting */}
      <rectAreaLight
        position={[0, 10, 0]}
        width={30}
        height={20}
        intensity={1 + energy * 2}
        color="#ff8000"
      />
    </group>
  );
}

function InfrastructureHub({ position, energy, isSelected, hovered }: any) {
  return (
    <group position={position}>
      {/* Fortress-like modules */}
      {[-8, 0, 8].map((y, i) => (
        <mesh key={i} position={[0, -5 + y, 0]} castShadow receiveShadow>
          <boxGeometry args={[18, 8, 18]} />
          <meshStandardMaterial
            color="#1a1a1a"
            roughness={0.9}
            metalness={0.9}
            emissive="#ff0000"
            emissiveIntensity={energy * 0.3 * (hovered || isSelected ? 1.5 : 1)}
          />
        </mesh>
      ))}

      {/* Server rack vents */}
      {Array.from({ length: 6 }).map((_, i) => (
        <mesh key={i} position={[0, -10 + i * 4, 9.1]}>
          <planeGeometry args={[16, 2]} />
          <meshStandardMaterial color="#ff4444" emissive="#ff0000" emissiveIntensity={0.5} />
        </mesh>
      ))}

      {/* Energy core (visible through vents) */}
      <mesh position={[0, 0, 0]}>
        <sphereGeometry args={[3, 16, 16]} />
        <meshBasicMaterial color="#ff0000" transparent opacity={0.6} />
        <pointLight intensity={2 * energy} color="#ff0000" distance={30} />
      </mesh>

      {/* Underground access indicators */}
      <mesh position={[0, -15, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[4, 5, 4]} />
        <meshBasicMaterial color="#ff0000" transparent opacity={0.5} />
      </mesh>
    </group>
  );
}

function ResearchFacility({ position, energy, isSelected, hovered }: any) {
  const groupRef = useRef<THREE.Group>(null);

  useFrame(() => {
    if (groupRef.current) {
      groupRef.current.rotation.y += 0.001;
    }
  });

  return (
    <group ref={groupRef} position={position}>
      {/* Organic curved structure */}
      <mesh castShadow receiveShadow>
        <sphereGeometry args={[12, 32, 32, 0, Math.PI * 2, 0, Math.PI / 2]} />
        <meshPhysicalMaterial
          color="#2a0a4a"
          transmission={0.7}
          thickness={0.8}
          roughness={0.05}
          metalness={0.3}
          emissive="#aa00ff"
          emissiveIntensity={energy * (hovered || isSelected ? 1.5 : 1)}
          transparent
          opacity={0.8}
        />
      </mesh>

      {/* Floating components */}
      {[0, 120, 240].map((angle, i) => {
        const rad = (angle * Math.PI) / 180;
        return (
          <mesh
            key={i}
            position={[Math.cos(rad) * 8, 10 + i * 3, Math.sin(rad) * 8]}
            castShadow
          >
            <octahedronGeometry args={[2, 0]} />
            <meshStandardMaterial
              color="#6a0a9a"
              emissive="#aa00ff"
              emissiveIntensity={energy * 0.8}
              metalness={0.9}
              roughness={0.1}
            />
          </mesh>
        );
      })}

      {/* Observatory dome */}
      <mesh position={[0, 20, 0]}>
        <sphereGeometry args={[6, 32, 16, 0, Math.PI * 2, 0, Math.PI / 2]} />
        <meshPhysicalMaterial
          color="#2a0a4a"
          transmission={0.9}
          thickness={0.3}
          roughness={0.02}
          transparent
          opacity={0.5}
        />
      </mesh>

      {/* Bioluminescent accents */}
      {Array.from({ length: 20 }).map((_, i) => {
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.random() * Math.PI / 2;
        const r = 12;
        return (
          <mesh
            key={i}
            position={[r * Math.sin(phi) * Math.cos(theta), r * Math.cos(phi), r * Math.sin(phi) * Math.sin(theta)]}
          >
            <sphereGeometry args={[0.3, 8, 8]} />
            <meshBasicMaterial color="#ff00ff" />
          </mesh>
        );
      })}

      {/* Purple lighting */}
      <pointLight position={[0, 15, 0]} intensity={2 * energy} color="#aa00ff" distance={40} />
    </group>
  );
}

export function ProjectBuilding({
  name,
  position,
  energy,
  isSelected,
  onSelect,
}: ProjectBuildingProps) {
  const [hovered, setHovered] = useState(false);
  const buildingType = getBuildingType(name);

  const props = {
    position: [position[0], position[1] + 20, position[2]] as [number, number, number], // Elevated 20 units
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
        position={[position[0], position[1] + 45, position[2]]}
        fontSize={5}
        color={hovered || isSelected ? '#ffffff' : '#9de5ff'}
        anchorX="center"
        anchorY="bottom"
        outlineWidth={0.2}
        outlineColor="#001925"
      >
        {name}
      </Text>

      {/* Ground plate */}
      <mesh position={[position[0], position[1] + 0.1, position[2]]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <circleGeometry args={[15, 32]} />
        <meshStandardMaterial
          color="#0a1f35"
          metalness={0.8}
          roughness={0.3}
          emissive="#004080"
          emissiveIntensity={isSelected || hovered ? 0.5 : 0.2}
        />
      </mesh>

      {/* Selection indicator ring */}
      {(isSelected || hovered) && (
        <mesh position={[position[0], position[1] + 0.2, position[2]]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[14, 16, 64]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={isSelected ? 0.8 : 0.5} />
        </mesh>
      )}
    </group>
  );
}
