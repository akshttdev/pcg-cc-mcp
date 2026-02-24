import { useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

export type AgentRole = 'developer' | 'designer' | 'analyst';
export type AgentStatus = 'idle' | 'working' | 'thinking' | 'reporting';

interface VoxelAgentAvatarProps {
  position: [number, number, number];
  role: AgentRole;
  status?: AgentStatus;
  pulseOffset?: number;
  label?: string;
  energy?: number; // 0-1
}

const ROLE_COLORS = {
  developer: '#0080ff',  // Electric Blue
  designer: '#ff8000',   // Creative Orange
  analyst: '#00ff80',    // Analytical Green
};

const AVATAR_SCALE = 0.7; // 70% of user avatar size

export function VoxelAgentAvatar({
  position,
  role,
  status = 'idle',
  pulseOffset = 0,
  label,
  energy = 1.0,
}: VoxelAgentAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Group>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const equipmentRef = useRef<THREE.Group>(null);
  const energyBarRef = useRef<THREE.Mesh>(null);

  const color = ROLE_COLORS[role];
  const colorObj = useMemo(() => new THREE.Color(color), [color]);

  // Blocky/pixelated material
  const voxelMaterial = useMemo(() => {
    return new THREE.MeshStandardMaterial({
      color: colorObj,
      emissive: colorObj,
      emissiveIntensity: 0.4,
      metalness: 0.3,
      roughness: 0.6,
      flatShading: true, // Gives blocky look
    });
  }, [colorObj]);

  useFrame((state) => {
    if (!groupRef.current) return;
    const t = state.clock.elapsedTime + pulseOffset;

    // Floating hover effect
    groupRef.current.position.y = position[1] + Math.sin(t * 1.2) * 0.3;

    // Head bobbing
    if (headRef.current) {
      headRef.current.position.y = 1.8 + Math.sin(t * 1.5) * 0.02;
      if (status === 'thinking') {
        headRef.current.rotation.z = Math.sin(t * 0.8) * 0.15;
      } else {
        headRef.current.rotation.z = Math.sin(t * 0.8) * 0.03;
      }
    }

    // Arm animations based on status
    if (leftArmRef.current && rightArmRef.current) {
      switch (status) {
        case 'working':
          leftArmRef.current.rotation.x = Math.sin(t * 4) * 0.3 - 0.2;
          rightArmRef.current.rotation.x = Math.sin(t * 4 + Math.PI) * 0.3 - 0.2;
          break;
        case 'thinking':
          rightArmRef.current.rotation.x = -1.2;
          leftArmRef.current.rotation.x = Math.sin(t * 0.5) * 0.1;
          break;
        case 'reporting':
          rightArmRef.current.rotation.x = -0.8;
          leftArmRef.current.rotation.x = Math.sin(t * 0.5) * 0.1;
          break;
        default:
          leftArmRef.current.rotation.x = Math.sin(t * 0.6) * 0.05;
          rightArmRef.current.rotation.x = Math.sin(t * 0.6 + Math.PI) * 0.05;
      }
    }

    // Equipment rotation
    if (equipmentRef.current) {
      equipmentRef.current.rotation.y = t * 0.5;
    }

    // Energy bar scale
    if (energyBarRef.current) {
      energyBarRef.current.scale.x = Math.max(0.05, energy);
    }
  });

  return (
    <group ref={groupRef} position={position} scale={AVATAR_SCALE}>
      {/* Voxel Robot Body */}
      <VoxelRobotBody color={color} material={voxelMaterial} />

      {/* Voxel Head */}
      <group ref={headRef} position={[0, 1.8, 0]}>
        {/* Main head - blocky cube */}
        <mesh material={voxelMaterial} castShadow>
          <boxGeometry args={[0.5, 0.5, 0.5]} />
        </mesh>

        {/* Visor/Face screen (inspired by ChestA preview) */}
        <mesh position={[0, 0, 0.26]}>
          <planeGeometry args={[0.4, 0.25]} />
          <meshStandardMaterial
            color={role === 'developer' ? '#00ffff' : color}
            emissive={color}
            emissiveIntensity={0.8}
            transparent
            opacity={0.85}
          />
        </mesh>

        {/* Antenna (small cube on top) */}
        <mesh position={[0, 0.3, 0]}>
          <boxGeometry args={[0.08, 0.08, 0.08]} />
          <meshBasicMaterial color="#00ffff" />
          <pointLight intensity={0.4} color="#00ffff" distance={2} />
        </mesh>

        {/* Role-specific head feature */}
        <RoleHeadFeature role={role} color={color} />
      </group>

      {/* Blocky Arms */}
      <group ref={leftArmRef} position={[0.6, 0.9, 0]}>
        {/* Upper arm */}
        <mesh position={[0, -0.2, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.2, 0.6, 0.2]} />
        </mesh>
        {/* Forearm */}
        <mesh position={[0, -0.7, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.18, 0.5, 0.18]} />
        </mesh>
        {/* Hand - blocky cube */}
        <mesh position={[0, -1.1, 0]}>
          <boxGeometry args={[0.22, 0.22, 0.22]} />
          <meshStandardMaterial color="#1a1a1a" emissive={color} emissiveIntensity={0.2} metalness={0.8} />
        </mesh>
      </group>

      <group ref={rightArmRef} position={[-0.6, 0.9, 0]}>
        {/* Upper arm */}
        <mesh position={[0, -0.2, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.2, 0.6, 0.2]} />
        </mesh>
        {/* Forearm */}
        <mesh position={[0, -0.7, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.18, 0.5, 0.18]} />
        </mesh>
        {/* Hand */}
        <mesh position={[0, -1.1, 0]}>
          <boxGeometry args={[0.22, 0.22, 0.22]} />
          <meshStandardMaterial color="#1a1a1a" emissive={color} emissiveIntensity={0.2} metalness={0.8} />
        </mesh>
      </group>

      {/* Blocky Legs */}
      <group position={[0.25, 0, 0]}>
        {/* Thigh */}
        <mesh position={[0, -0.3, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.24, 0.6, 0.24]} />
        </mesh>
        {/* Shin */}
        <mesh position={[0, -0.8, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.22, 0.5, 0.22]} />
        </mesh>
        {/* Foot */}
        <mesh position={[0, -1.15, 0.08]}>
          <boxGeometry args={[0.26, 0.15, 0.38]} />
          <meshStandardMaterial color="#1a1a1a" metalness={0.8} />
        </mesh>
      </group>
      <group position={[-0.25, 0, 0]}>
        {/* Thigh */}
        <mesh position={[0, -0.3, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.24, 0.6, 0.24]} />
        </mesh>
        {/* Shin */}
        <mesh position={[0, -0.8, 0]} material={voxelMaterial} castShadow>
          <boxGeometry args={[0.22, 0.5, 0.22]} />
        </mesh>
        {/* Foot */}
        <mesh position={[0, -1.15, 0.08]}>
          <boxGeometry args={[0.26, 0.15, 0.38]} />
          <meshStandardMaterial color="#1a1a1a" metalness={0.8} />
        </mesh>
      </group>

      {/* Role-specific equipment */}
      <group ref={equipmentRef}>
        <RoleEquipment role={role} color={color} status={status} />
      </group>

      {/* Energy bar above head */}
      <group position={[0, 2.7, 0]}>
        <mesh position={[0, 0, 0]}>
          <boxGeometry args={[0.8, 0.08, 0.02]} />
          <meshBasicMaterial color="#1a1a1a" transparent opacity={0.7} />
        </mesh>
        <mesh ref={energyBarRef} position={[-0.4 + (0.4 * energy), 0, 0.01]}>
          <boxGeometry args={[0.8, 0.06, 0.02]} />
          <meshBasicMaterial color={color} transparent opacity={0.9} />
        </mesh>
      </group>

      {/* Label */}
      {label && (
        <Text
          position={[0, 3.2, 0]}
          fontSize={0.35}
          color={color}
          anchorX="center"
          anchorY="bottom"
          outlineWidth={0.02}
          outlineColor="#000000"
        >
          {label}
        </Text>
      )}

      {/* Status indicator */}
      {status !== 'idle' && (
        <StatusIndicator status={status} color={color} />
      )}
    </group>
  );
}

// Voxel robot body (blocky torso)
function VoxelRobotBody({ color, material }: { color: string; material: THREE.Material }) {
  return (
    <group>
      {/* Main chest - wide box (like ChestA) */}
      <mesh position={[0, 1.0, 0]} material={material} castShadow>
        <boxGeometry args={[0.7, 0.6, 0.4]} />
      </mesh>

      {/* Belly/lower torso */}
      <mesh position={[0, 0.5, 0]} material={material} castShadow>
        <boxGeometry args={[0.6, 0.4, 0.35]} />
      </mesh>

      {/* Hip/pelvis */}
      <mesh position={[0, 0.1, 0]} material={material} castShadow>
        <boxGeometry args={[0.5, 0.2, 0.3]} />
      </mesh>

      {/* Chest indicator/screen (voxel style) */}
      <mesh position={[0, 1.0, 0.21]}>
        <planeGeometry args={[0.35, 0.25]} />
        <meshBasicMaterial color={color} transparent opacity={0.8} />
        <pointLight intensity={0.3} color={color} distance={2} />
      </mesh>
    </group>
  );
}

// Role-specific head features (voxel style)
function RoleHeadFeature({ role, color }: { role: AgentRole; color: string }) {
  switch (role) {
    case 'developer':
      // Terminal screen on side of head
      return (
        <mesh position={[0.26, 0, 0]} rotation={[0, Math.PI / 2, 0]}>
          <planeGeometry args={[0.3, 0.2]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={0.6} />
        </mesh>
      );

    case 'designer':
      // Color indicator cubes
      return (
        <group>
          {['#ff0000', '#00ff00', '#0000ff'].map((col, i) => (
            <mesh key={i} position={[-0.15 + i * 0.15, 0.3, 0]}>
              <boxGeometry args={[0.08, 0.08, 0.08]} />
              <meshBasicMaterial color={col} />
            </mesh>
          ))}
        </group>
      );

    case 'analyst':
      // Scanner beam (small boxes stacked)
      return (
        <group position={[0, 0.3, 0.26]}>
          <mesh>
            <boxGeometry args={[0.05, 0.15, 0.05]} />
            <meshBasicMaterial color={color} transparent opacity={0.7} />
            <pointLight intensity={0.3} color={color} distance={2} />
          </mesh>
        </group>
      );

    default:
      return null;
  }
}

// Role-specific equipment (keeping particle systems from before)
function RoleEquipment({ role, color, status }: { role: AgentRole; color: string; status: AgentStatus }) {
  const particleCount = status === 'working' ? 20 : 5;
  const positions = useMemo(() => {
    const pos = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount; i++) {
      const angle = (i / particleCount) * Math.PI * 2;
      const radius = 0.8 + Math.random() * 0.4;
      pos[i * 3] = Math.cos(angle) * radius;
      pos[i * 3 + 1] = Math.random() * 1.5 + 0.5;
      pos[i * 3 + 2] = Math.sin(angle) * radius;
    }
    return pos;
  }, [particleCount]);

  return (
    <group>
      {/* Particles for all roles */}
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
          size={role === 'developer' ? 0.08 : 0.06}
          color={role === 'developer' ? '#00ff00' : color}
          transparent
          opacity={status === 'working' ? 0.8 : 0.4}
          sizeAttenuation
        />
      </points>

      {/* Working mode hologram (blocky style) */}
      {status === 'working' && role === 'developer' && (
        <mesh position={[0, 0.6, 0.6]} rotation={[-Math.PI / 3, 0, 0]}>
          <boxGeometry args={[0.6, 0.04, 0.4]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={0.3} wireframe />
        </mesh>
      )}

      {status === 'working' && role === 'analyst' && (
        <group position={[0, 1.2, 0.6]}>
          {[0.3, 0.6, 0.4, 0.7, 0.5].map((height, i) => (
            <mesh key={i} position={[(i - 2) * 0.15, height / 2, 0]}>
              <boxGeometry args={[0.1, height, 0.05]} />
              <meshBasicMaterial color={color} transparent opacity={0.6} />
            </mesh>
          ))}
        </group>
      )}
    </group>
  );
}

// Status indicator (voxel style)
function StatusIndicator({ status, color }: { status: AgentStatus; color: string }) {
  const indicatorScale =
    status === 'working' ? 1.3 : status === 'thinking' ? 1.1 : status === 'reporting' ? 1.4 : 1;
  const emissiveIntensity = status === 'reporting' ? 0.9 : 0.5;

  return (
    <group position={[0, 3.4, 0]} scale={[indicatorScale, indicatorScale, indicatorScale]}>
      <mesh>
        <boxGeometry args={[0.2, 0.2, 0.2]} />
        <meshStandardMaterial color={color} emissive={color} emissiveIntensity={emissiveIntensity} />
      </mesh>
      <pointLight intensity={0.4 + emissiveIntensity * 0.4} color={color} distance={3} />
    </group>
  );
}
