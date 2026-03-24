import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useEquipmentStore } from '@/stores/useEquipmentStore';
import { CrownEquipment, BluntEquipment, FireCapeEquipment, GodBookEquipment } from '../equipment';

// Color schemes for different user types
const ADMIN_COLORS = {
  main: '#f5f5f5',    // White
  accent: '#ffd700',  // Gold
  dark: '#1a1a1a',
};

const DEFAULT_COLORS = {
  main: '#7a8b99',    // Steel gray
  accent: '#00bcd4',  // Cyan
  dark: '#2a2a2a',
};

export interface AnimationDescriptor {
  mode: 'idle' | 'walk' | 'run' | 'jump' | 'fly';
  intensity: number;
  airborne: boolean;
  tiltX: number;
  tiltZ: number;
}

interface HumanoidAvatarProps {
  color: string;
  isAdmin: boolean;
  animationRef: React.MutableRefObject<AnimationDescriptor>;
  showJetpack?: boolean;
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars, unused-imports/no-unused-vars
export function HumanoidAvatar({ color: _color, isAdmin, animationRef, showJetpack = false }: HumanoidAvatarProps) {
  const bodyRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Group>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const leftLegRef = useRef<THREE.Group>(null);
  const rightLegRef = useRef<THREE.Group>(null);

  // Equipment state
  const equipped = useEquipmentStore((s) => s.equipped);
  const hasCrown = equipped.head === 'crown';
  const hasBlunt = equipped.primaryHand === 'blunt';
  const hasGodBook = equipped.secondaryHand === 'godBook';
  const hasFireCape = equipped.back === 'fireCape';

  // Select colors based on admin status
  const colors = isAdmin ? ADMIN_COLORS : DEFAULT_COLORS;

  useFrame((state) => {
    const { mode, intensity } = animationRef.current;
    const time = state.clock.elapsedTime;

    const cycleSpeed = mode === 'run' ? 14 : mode === 'walk' ? 9 : 2;
    const cycle = Math.sin(time * cycleSpeed);
    const oppositeCycle = Math.sin(time * cycleSpeed + Math.PI);

    // Arms
    if (leftArmRef.current && rightArmRef.current) {
      const armSwing = mode === 'idle' ? 0.03 : (mode === 'run' ? 0.9 : 0.5) * intensity;
      leftArmRef.current.rotation.x = cycle * armSwing;
      rightArmRef.current.rotation.x = oppositeCycle * armSwing;
      if (mode === 'run') {
        leftArmRef.current.rotation.z = -0.1;
        rightArmRef.current.rotation.z = 0.1;
      } else {
        leftArmRef.current.rotation.z = 0;
        rightArmRef.current.rotation.z = 0;
      }
    }

    // Legs
    if (leftLegRef.current && rightLegRef.current) {
      const legSwing = mode === 'idle' ? 0 : (mode === 'run' ? 0.7 : 0.4) * intensity;
      leftLegRef.current.rotation.x = oppositeCycle * legSwing;
      rightLegRef.current.rotation.x = cycle * legSwing;
    }

    // Head bob
    if (headRef.current) {
      const bobAmount = mode === 'run' ? 0.04 : mode === 'walk' ? 0.02 : 0.01;
      const bobSpeed = mode === 'run' ? 28 : mode === 'walk' ? 18 : 1.5;
      headRef.current.position.y = 2.4 + Math.abs(Math.sin(time * bobSpeed)) * bobAmount * intensity;
      headRef.current.rotation.y = mode === 'idle' ? Math.sin(time * 0.3) * 0.1 : 0;
    }

    // Body bounce
    if (bodyRef.current) {
      if (mode === 'run' || mode === 'walk') {
        const bounce = Math.abs(Math.sin(time * cycleSpeed * 2)) * 0.03 * intensity;
        bodyRef.current.position.y = bounce;
      } else {
        bodyRef.current.position.y = 0;
      }
    }
  });

  const mainColor = colors.main;
  const accentColor = colors.accent;
  const darkColor = colors.dark;

  return (
    <group ref={bodyRef}>
      {/* Head */}
      <group ref={headRef} position={[0, 2.4, 0]}>
        <mesh castShadow>
          <sphereGeometry args={[0.5, 32, 32]} />
          <meshStandardMaterial color={mainColor} metalness={0} roughness={1} />
        </mesh>
        <mesh position={[0, 0.05, 0.35]} rotation={[0.1, 0, 0]}>
          <boxGeometry args={[0.6, 0.25, 0.15]} />
          <meshPhysicalMaterial color={accentColor} emissive={accentColor} emissiveIntensity={0.5} metalness={0.9} roughness={0.1} transparent opacity={0.8} />
        </mesh>
        <group position={[0.2, 0.45, -0.1]}>
          <mesh>
            <cylinderGeometry args={[0.02, 0.015, 0.25, 8]} />
            <meshStandardMaterial color={darkColor} metalness={0.8} />
          </mesh>
          <mesh position={[0, 0.15, 0]}>
            <sphereGeometry args={[0.04, 12, 12]} />
            <meshBasicMaterial color={accentColor} />
            <pointLight color={accentColor} intensity={0.3} distance={2} />
          </mesh>
        </group>

        {/* Crown - conditional based on equipment */}
        {hasCrown && <CrownEquipment />}
      </group>

      {/* Torso */}
      <group position={[0, 1.3, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.4, 0.9, 12, 24]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.03} metalness={0.1} roughness={0.85} />
        </mesh>
        <mesh position={[0, 0.15, 0.3]}>
          <boxGeometry args={[0.5, 0.4, 0.15]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} roughness={0.2} />
        </mesh>
        <mesh position={[0, 0.15, 0.38]}>
          <circleGeometry args={[0.08, 16]} />
          <meshBasicMaterial color={accentColor} />
          <pointLight color={accentColor} intensity={0.4} distance={3} />
        </mesh>
        <mesh position={[0, -0.35, 0]}>
          <torusGeometry args={[0.42, 0.06, 12, 24]} />
          <meshStandardMaterial color={darkColor} metalness={0.8} roughness={0.2} />
        </mesh>
        {/* Fire Cape - back slot */}
        {hasFireCape && <FireCapeEquipment />}
      </group>

      {/* Left Arm */}
      <group ref={leftArmRef} position={[0.65, 1.5, 0]}>
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.12, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.02} metalness={0.1} roughness={0.85} />
        </mesh>
        <mesh castShadow position={[0, -0.6, 0]}>
          <capsuleGeometry args={[0.1, 0.35, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.6} roughness={0.3} />
        </mesh>
        <mesh position={[0, -0.85, 0]}>
          <sphereGeometry args={[0.1, 12, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} />
        </mesh>
        {/* God Book held in offhand */}
        {hasGodBook && <GodBookEquipment />}
      </group>

      {/* Right Arm */}
      <group ref={rightArmRef} position={[-0.65, 1.5, 0]}>
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.12, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.02} metalness={0.1} roughness={0.85} />
        </mesh>
        <mesh castShadow position={[0, -0.6, 0]}>
          <capsuleGeometry args={[0.1, 0.35, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.6} roughness={0.3} />
        </mesh>
        <mesh position={[0, -0.85, 0]}>
          <sphereGeometry args={[0.1, 12, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} />
        </mesh>
        {/* Blunt held in hand */}
        {hasBlunt && <BluntEquipment armRef={rightArmRef} headRef={headRef} />}
      </group>

      {/* Left Leg */}
      <group ref={leftLegRef} position={[0.22, 0.4, 0]}>
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.14, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.02} metalness={0.1} roughness={0.85} />
        </mesh>
        <mesh castShadow position={[0, -0.65, 0]}>
          <capsuleGeometry args={[0.11, 0.4, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.5} roughness={0.3} />
        </mesh>
        <mesh position={[0, -0.95, 0.05]}>
          <boxGeometry args={[0.18, 0.12, 0.28]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} roughness={0.2} />
        </mesh>
        <mesh position={[0, -0.92, 0.15]}>
          <boxGeometry args={[0.19, 0.04, 0.04]} />
          <meshBasicMaterial color={accentColor} />
        </mesh>
      </group>

      {/* Right Leg */}
      <group ref={rightLegRef} position={[-0.22, 0.4, 0]}>
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.14, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={mainColor} emissiveIntensity={0.15} />
        </mesh>
        <mesh castShadow position={[0, -0.65, 0]}>
          <capsuleGeometry args={[0.11, 0.4, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.5} roughness={0.3} />
        </mesh>
        <mesh position={[0, -0.95, 0.05]}>
          <boxGeometry args={[0.18, 0.12, 0.28]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} roughness={0.2} />
        </mesh>
        <mesh position={[0, -0.92, 0.15]}>
          <boxGeometry args={[0.19, 0.04, 0.04]} />
          <meshBasicMaterial color={accentColor} />
        </mesh>
      </group>

      {/* Jetpack */}
      {showJetpack && (
        <group position={[0, 1.3, -0.45]}>
          <mesh>
            <boxGeometry args={[0.5, 0.7, 0.25]} />
            <meshStandardMaterial color={darkColor} metalness={0.8} roughness={0.2} />
          </mesh>
          <mesh position={[-0.15, -0.35, 0]}>
            <cylinderGeometry args={[0.1, 0.12, 0.2, 12]} />
            <meshStandardMaterial color={darkColor} metalness={0.9} />
          </mesh>
          <mesh position={[0.15, -0.35, 0]}>
            <cylinderGeometry args={[0.1, 0.12, 0.2, 12]} />
            <meshStandardMaterial color={darkColor} metalness={0.9} />
          </mesh>
          <mesh position={[-0.15, -0.5, 0]}>
            <coneGeometry args={[0.08, 0.25, 8]} />
            <meshBasicMaterial color={accentColor} transparent opacity={0.8} />
            <pointLight color={accentColor} intensity={1.5} distance={4} />
          </mesh>
          <mesh position={[0.15, -0.5, 0]}>
            <coneGeometry args={[0.08, 0.25, 8]} />
            <meshBasicMaterial color={accentColor} transparent opacity={0.8} />
            <pointLight color={accentColor} intensity={1.5} distance={4} />
          </mesh>
        </group>
      )}
    </group>
  );
}
