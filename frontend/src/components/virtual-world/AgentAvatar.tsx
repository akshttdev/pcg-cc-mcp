import { useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

export type AgentRole = 'developer' | 'designer' | 'analyst';
export type AgentStatus = 'idle' | 'working' | 'thinking' | 'reporting';

interface AgentAvatarProps {
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

export function AgentAvatar({
  position,
  role,
  status = 'idle',
  pulseOffset = 0,
  label,
  energy = 1.0,
}: AgentAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Group>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const equipmentRef = useRef<THREE.Group>(null);
  const energyBarRef = useRef<THREE.Mesh>(null);

  const color = ROLE_COLORS[role];

  useFrame((state) => {
    if (!groupRef.current) return;
    const t = state.clock.elapsedTime + pulseOffset;

    // Floating hover effect
    groupRef.current.position.y = position[1] + Math.sin(t * 1.2) * 0.3;

    // Head bobbing
    if (headRef.current) {
      headRef.current.position.y = 1.68 + Math.sin(t * 1.5) * 0.02;
      // Thinking state: head tilt
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
          // Typing/working motion
          leftArmRef.current.rotation.x = Math.sin(t * 4) * 0.3 - 0.2;
          rightArmRef.current.rotation.x = Math.sin(t * 4 + Math.PI) * 0.3 - 0.2;
          break;
        case 'thinking':
          // Hand on chin
          rightArmRef.current.rotation.x = -1.2;
          leftArmRef.current.rotation.x = Math.sin(t * 0.5) * 0.1;
          break;
        case 'reporting':
          // Pointing gesture
          rightArmRef.current.rotation.x = -0.8;
          leftArmRef.current.rotation.x = Math.sin(t * 0.5) * 0.1;
          break;
        default:
          // Idle subtle movement
          leftArmRef.current.rotation.x = Math.sin(t * 0.6) * 0.05;
          rightArmRef.current.rotation.x = Math.sin(t * 0.6 + Math.PI) * 0.05;
      }
    }

    // Equipment rotation (role-specific effects)
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
      {/* Main humanoid body */}
      <HumanoidAgentBody color={color} />

      {/* Head with role-specific features */}
      <group ref={headRef} position={[0, 1.68, 0]}>
        {/* Main head */}
        <mesh castShadow>
          <sphereGeometry args={[0.38, 24, 24]} />
          <meshStandardMaterial
            color={color}
            emissive={color}
            emissiveIntensity={0.6}
            metalness={0.3}
            roughness={0.3}
          />
        </mesh>

        {/* Eyes */}
        <mesh position={[-0.14, 0.05, 0.32]}>
          <sphereGeometry args={[0.05, 12, 12]} />
          <meshBasicMaterial color="#ffffff" />
        </mesh>
        <mesh position={[0.14, 0.05, 0.32]}>
          <sphereGeometry args={[0.05, 12, 12]} />
          <meshBasicMaterial color="#ffffff" />
        </mesh>

        {/* Role-specific headgear */}
        <RoleHeadgear role={role} color={color} />
      </group>

      {/* Arms */}
      <group ref={leftArmRef} position={[0.6, 0.84, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.12, 0.7, 10, 12]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.3} />
        </mesh>
      </group>
      <group ref={rightArmRef} position={[-0.6, 0.84, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.12, 0.7, 10, 12]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.3} />
        </mesh>
      </group>

      {/* Legs (slightly spread for stability) */}
      <group position={[0.25, 0, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.15, 0.84, 10, 12]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.25} />
        </mesh>
      </group>
      <group position={[-0.25, 0, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.15, 0.84, 10, 12]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.25} />
        </mesh>
      </group>

      {/* Role-specific equipment */}
      <group ref={equipmentRef}>
        <RoleEquipment role={role} color={color} status={status} />
      </group>

      {/* Energy bar above head */}
      <group position={[0, 2.5, 0]}>
        {/* Background bar */}
        <mesh position={[0, 0, 0]}>
          <boxGeometry args={[0.8, 0.08, 0.02]} />
          <meshBasicMaterial color="#1a1a1a" transparent opacity={0.7} />
        </mesh>
        {/* Energy fill */}
        <mesh ref={energyBarRef} position={[-0.4 + (0.4 * energy), 0, 0.01]}>
          <boxGeometry args={[0.8, 0.06, 0.02]} />
          <meshBasicMaterial
            color={color}
            transparent
            opacity={0.9}
          />
        </mesh>
      </group>

      {/* Label */}
      {label && (
        <Text
          position={[0, 3, 0]}
          fontSize={0.4}
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

// Humanoid body component
function HumanoidAgentBody({ color }: { color: string }) {
  return (
    <group>
      {/* Torso */}
      <mesh position={[0, 0.77, 0]} castShadow>
        <capsuleGeometry args={[0.38, 1.19, 12, 24]} />
        <meshStandardMaterial
          color={color}
          emissive={color}
          emissiveIntensity={0.4}
          metalness={0.2}
          roughness={0.35}
        />
      </mesh>

      {/* Chest emblem (role icon) */}
      <mesh position={[0, 1.1, 0.35]}>
        <circleGeometry args={[0.15, 16]} />
        <meshBasicMaterial color={color} transparent opacity={0.9} />
        <pointLight intensity={0.3} color={color} distance={2} />
      </mesh>
    </group>
  );
}

// Role-specific headgear
function RoleHeadgear({ role, color }: { role: AgentRole; color: string }) {
  switch (role) {
    case 'developer':
      // Terminal visor
      return (
        <mesh position={[0, 0, 0.36]}>
          <planeGeometry args={[0.6, 0.3]} />
          <meshPhysicalMaterial
            color="#00ffff"
            transmission={0.8}
            roughness={0.1}
            metalness={0.9}
            transparent
            opacity={0.4}
            emissive="#00ffff"
            emissiveIntensity={0.3}
          />
        </mesh>
      );

    case 'designer':
      // Artistic lens
      return (
        <group>
          <mesh position={[0.2, 0, 0.36]} rotation={[0, 0, Math.PI / 4]}>
            <torusGeometry args={[0.1, 0.02, 8, 16]} />
            <meshStandardMaterial
              color={color}
              emissive={color}
              emissiveIntensity={0.6}
              metalness={0.8}
            />
          </mesh>
        </group>
      );

    case 'analyst':
      // Scanner beam
      return (
        <group position={[0, 0.15, 0.3]}>
          <mesh>
            <cylinderGeometry args={[0.03, 0.03, 0.2, 8]} />
            <meshBasicMaterial color={color} transparent opacity={0.6} />
            <pointLight intensity={0.4} color={color} distance={3} />
          </mesh>
        </group>
      );

    default:
      return null;
  }
}

// Role-specific equipment
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

  switch (role) {
    case 'developer':
      // Holographic keyboard and code particles
      return (
        <group>
          {/* Keyboard hologram */}
          {status === 'working' && (
            <mesh position={[0, 0.6, 0.6]} rotation={[-Math.PI / 3, 0, 0]}>
              <boxGeometry args={[0.8, 0.02, 0.4]} />
              <meshBasicMaterial color="#00ffff" transparent opacity={0.4} wireframe />
            </mesh>
          )}

          {/* Code particles */}
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
              size={0.05}
              color="#00ff00"
              transparent
              opacity={status === 'working' ? 0.8 : 0.4}
              sizeAttenuation
            />
          </points>

          {/* Binary data ring */}
          <mesh position={[0, 1.5, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <ringGeometry args={[0.9, 1.1, 32]} />
            <meshBasicMaterial color={color} transparent opacity={0.3} side={THREE.DoubleSide} />
          </mesh>
        </group>
      );

    case 'designer':
      // Color palette and creative sparks
      return (
        <group>
          {/* Color palette swatches */}
          {['#ff0000', '#00ff00', '#0000ff', '#ffff00'].map((swatchColor, i) => (
            <mesh
              key={i}
              position={[
                Math.cos((i / 4) * Math.PI * 2) * 0.7,
                1.2,
                Math.sin((i / 4) * Math.PI * 2) * 0.7,
              ]}
            >
              <sphereGeometry args={[0.08, 12, 12]} />
              <meshBasicMaterial color={swatchColor} />
            </mesh>
          ))}

          {/* Stylus tool (when working) */}
          {status === 'working' && (
            <mesh position={[-0.6, 0.4, 0.3]} rotation={[0, 0, -Math.PI / 4]}>
              <cylinderGeometry args={[0.02, 0.04, 0.4, 8]} />
              <meshStandardMaterial color={color} metalness={0.8} />
            </mesh>
          )}

          {/* Creative spark particles */}
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
              size={0.06}
              color={color}
              transparent
              opacity={status === 'working' ? 0.9 : 0.5}
              sizeAttenuation
              blending={THREE.AdditiveBlending}
            />
          </points>
        </group>
      );

    case 'analyst':
      // Holographic charts and data streams
      return (
        <group>
          {/* Chart hologram */}
          {status === 'working' && (
            <group position={[0, 1.2, 0.6]}>
              {/* Bar chart bars */}
              {[0.3, 0.6, 0.4, 0.7, 0.5].map((height, i) => (
                <mesh key={i} position={[(i - 2) * 0.15, height / 2, 0]}>
                  <boxGeometry args={[0.1, height, 0.05]} />
                  <meshBasicMaterial color={color} transparent opacity={0.6} />
                </mesh>
              ))}
            </group>
          )}

          {/* Data stream from eyes */}
          <mesh position={[0, 1.8, 0.4]} rotation={[-Math.PI / 4, 0, 0]}>
            <cylinderGeometry args={[0.08, 0.02, 0.6, 8]} />
            <meshBasicMaterial color={color} transparent opacity={0.4} />
          </mesh>

          {/* Scanner ring */}
          <mesh position={[0, 1.0, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <ringGeometry args={[0.7, 0.8, 32]} />
            <meshBasicMaterial color={color} transparent opacity={0.5} side={THREE.DoubleSide} />
          </mesh>

          {/* Data particles */}
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
              size={0.04}
              color={color}
              transparent
              opacity={status === 'working' ? 0.8 : 0.4}
              sizeAttenuation
            />
          </points>
        </group>
      );

    default:
      return null;
  }
}

// Status indicator hologram
function StatusIndicator({ status, color }: { status: AgentStatus; color: string }) {
  let icon: React.ReactNode;

  switch (status) {
    case 'working':
      // Rotating gear
      icon = (
        <mesh rotation={[0, 0, 0]}>
          <torusGeometry args={[0.15, 0.04, 8, 6]} />
          <meshBasicMaterial color={color} />
        </mesh>
      );
      break;
    case 'thinking':
      // Question mark (simplified as sphere + curve)
      icon = (
        <group>
          <mesh position={[0, 0.1, 0]}>
            <sphereGeometry args={[0.08, 12, 12]} />
            <meshBasicMaterial color={color} />
          </mesh>
          <mesh position={[0, 0.25, 0]}>
            <torusGeometry args={[0.1, 0.03, 8, 12, Math.PI]} />
            <meshBasicMaterial color={color} />
          </mesh>
        </group>
      );
      break;
    case 'reporting':
      // Upward arrow
      icon = (
        <group>
          <mesh>
            <cylinderGeometry args={[0.02, 0.02, 0.3, 8]} />
            <meshBasicMaterial color={color} />
          </mesh>
          <mesh position={[0, 0.2, 0]}>
            <coneGeometry args={[0.08, 0.15, 8]} />
            <meshBasicMaterial color={color} />
          </mesh>
        </group>
      );
      break;
    default:
      return null;
  }

  return (
    <group position={[0, 3.2, 0]}>
      {icon}
      <pointLight intensity={0.5} color={color} distance={3} />
    </group>
  );
}
