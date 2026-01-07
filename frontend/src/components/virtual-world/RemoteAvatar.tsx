import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import type { RemotePlayer } from '@/types/multiplayer';
import { CrownEquipment, FireCapeEquipment, GodBookEquipment } from './equipment';

interface RemoteAvatarProps {
  player: RemotePlayer;
}

// Color schemes
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

export function RemoteAvatar({ player }: RemoteAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const bodyRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Group>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const leftLegRef = useRef<THREE.Group>(null);
  const rightLegRef = useRef<THREE.Group>(null);

  // Interpolation for smooth movement
  const targetPosition = useRef(new THREE.Vector3(player.position.x, player.position.y, player.position.z));
  const targetRotation = useRef(player.rotation.y);
  const currentPosition = useRef(new THREE.Vector3(player.position.x, player.position.y, player.position.z));
  const currentRotation = useRef(player.rotation.y);

  // Update targets when player data changes
  useMemo(() => {
    targetPosition.current.set(player.position.x, player.position.y, player.position.z);
    targetRotation.current = player.rotation.y;
  }, [player.position.x, player.position.y, player.position.z, player.rotation.y]);

  const colors = player.isAdmin ? ADMIN_COLORS : DEFAULT_COLORS;

  useFrame((state, delta) => {
    if (!groupRef.current || !bodyRef.current) return;

    // Interpolate position
    currentPosition.current.lerp(targetPosition.current, Math.min(delta * 10, 1));
    groupRef.current.position.copy(currentPosition.current);

    // Interpolate rotation
    let rotDiff = targetRotation.current - currentRotation.current;
    while (rotDiff > Math.PI) rotDiff -= Math.PI * 2;
    while (rotDiff < -Math.PI) rotDiff += Math.PI * 2;
    currentRotation.current += rotDiff * Math.min(delta * 10, 1);
    bodyRef.current.rotation.y = currentRotation.current;

    // Animation
    const time = state.clock.elapsedTime;
    const isMoving = player.isMoving;
    const cycleSpeed = isMoving ? 9 : 2;
    const cycle = Math.sin(time * cycleSpeed);
    const oppositeCycle = Math.sin(time * cycleSpeed + Math.PI);

    // Arms animation
    if (leftArmRef.current && rightArmRef.current) {
      const armSwing = isMoving ? 0.5 : 0.03;
      leftArmRef.current.rotation.x = cycle * armSwing;
      rightArmRef.current.rotation.x = oppositeCycle * armSwing;
    }

    // Legs animation
    if (leftLegRef.current && rightLegRef.current) {
      const legSwing = isMoving ? 0.4 : 0;
      leftLegRef.current.rotation.x = oppositeCycle * legSwing;
      rightLegRef.current.rotation.x = cycle * legSwing;
    }

    // Head bob
    if (headRef.current) {
      const bobAmount = isMoving ? 0.02 : 0.01;
      const bobSpeed = isMoving ? 18 : 1.5;
      headRef.current.position.y = 2.4 + Math.abs(Math.sin(time * bobSpeed)) * bobAmount;
    }

    // Body bounce
    if (bodyRef.current && isMoving) {
      const bounce = Math.abs(Math.sin(time * cycleSpeed * 2)) * 0.03;
      bodyRef.current.position.y = bounce;
    }
  });

  return (
    <group ref={groupRef}>
      {/* Nameplate */}
      <Text
        position={[0, 4.2, 0]}
        fontSize={0.35}
        color={colors.accent}
        anchorX="center"
        anchorY="bottom"
        outlineWidth={0.02}
        outlineColor="#000000"
      >
        {player.displayName}
      </Text>

      {/* Admin indicator */}
      {player.isAdmin && (
        <Text
          position={[0, 3.85, 0]}
          fontSize={0.2}
          color="#ffd700"
          anchorX="center"
          anchorY="bottom"
        >
          ADMIN
        </Text>
      )}

      <group ref={bodyRef}>
        {/* Head */}
        <group ref={headRef} position={[0, 2.4, 0]}>
          <mesh castShadow>
            <sphereGeometry args={[0.5, 32, 32]} />
            <meshStandardMaterial color={colors.main} metalness={0} roughness={1} />
          </mesh>
          <mesh position={[0, 0.05, 0.35]} rotation={[0.1, 0, 0]}>
            <boxGeometry args={[0.6, 0.25, 0.15]} />
            <meshPhysicalMaterial
              color={colors.accent}
              emissive={colors.accent}
              emissiveIntensity={0.5}
              metalness={0.9}
              roughness={0.1}
              transparent
              opacity={0.8}
            />
          </mesh>
          <group position={[0.2, 0.45, -0.1]}>
            <mesh>
              <cylinderGeometry args={[0.02, 0.015, 0.25, 8]} />
              <meshStandardMaterial color={colors.dark} metalness={0.8} />
            </mesh>
            <mesh position={[0, 0.15, 0]}>
              <sphereGeometry args={[0.04, 12, 12]} />
              <meshBasicMaterial color={colors.accent} />
              <pointLight color={colors.accent} intensity={0.3} distance={2} />
            </mesh>
          </group>

          {/* Crown - based on equipped items */}
          {player.equipment?.head === 'crown' && <CrownEquipment />}
        </group>

        {/* Torso */}
        <group position={[0, 1.3, 0]}>
          <mesh castShadow>
            <capsuleGeometry args={[0.4, 0.9, 12, 24]} />
            <meshStandardMaterial
              color={colors.main}
              emissive={colors.accent}
              emissiveIntensity={0.03}
              metalness={0.1}
              roughness={0.85}
            />
          </mesh>
          <mesh position={[0, 0.15, 0.3]}>
            <boxGeometry args={[0.5, 0.4, 0.15]} />
            <meshStandardMaterial color={colors.dark} metalness={0.7} roughness={0.2} />
          </mesh>
          <mesh position={[0, 0.15, 0.38]}>
            <circleGeometry args={[0.08, 16]} />
            <meshBasicMaterial color={colors.accent} />
            <pointLight color={colors.accent} intensity={0.4} distance={3} />
          </mesh>
          <mesh position={[0, -0.35, 0]}>
            <torusGeometry args={[0.42, 0.06, 12, 24]} />
            <meshStandardMaterial color={colors.dark} metalness={0.8} roughness={0.2} />
          </mesh>
          {/* Fire Cape - based on equipped items */}
          {player.equipment?.back === 'fireCape' && <FireCapeEquipment />}
        </group>

        {/* Left Arm */}
        <group ref={leftArmRef} position={[0.65, 1.5, 0]}>
          <mesh castShadow position={[0, -0.25, 0]}>
            <capsuleGeometry args={[0.12, 0.4, 8, 12]} />
            <meshStandardMaterial
              color={colors.main}
              emissive={colors.accent}
              emissiveIntensity={0.02}
              metalness={0.1}
              roughness={0.85}
            />
          </mesh>
          <mesh castShadow position={[0, -0.6, 0]}>
            <capsuleGeometry args={[0.1, 0.35, 8, 12]} />
            <meshStandardMaterial color={colors.dark} metalness={0.6} roughness={0.3} />
          </mesh>
          <mesh position={[0, -0.85, 0]}>
            <sphereGeometry args={[0.1, 12, 12]} />
            <meshStandardMaterial color={colors.dark} metalness={0.7} />
          </mesh>
          {/* God Book - based on equipped items */}
          {player.equipment?.secondaryHand === 'godBook' && <GodBookEquipment />}
        </group>

        {/* Right Arm */}
        <group ref={rightArmRef} position={[-0.65, 1.5, 0]}>
          <mesh castShadow position={[0, -0.25, 0]}>
            <capsuleGeometry args={[0.12, 0.4, 8, 12]} />
            <meshStandardMaterial
              color={colors.main}
              emissive={colors.accent}
              emissiveIntensity={0.02}
              metalness={0.1}
              roughness={0.85}
            />
          </mesh>
          <mesh castShadow position={[0, -0.6, 0]}>
            <capsuleGeometry args={[0.1, 0.35, 8, 12]} />
            <meshStandardMaterial color={colors.dark} metalness={0.6} roughness={0.3} />
          </mesh>
          <mesh position={[0, -0.85, 0]}>
            <sphereGeometry args={[0.1, 12, 12]} />
            <meshStandardMaterial color={colors.dark} metalness={0.7} />
          </mesh>
          {/* Blunt - only for admin (note: needs arm refs which we don't pass here, so skip for remotes) */}
        </group>

        {/* Left Leg */}
        <group ref={leftLegRef} position={[0.22, 0.4, 0]}>
          <mesh castShadow position={[0, -0.25, 0]}>
            <capsuleGeometry args={[0.14, 0.4, 8, 12]} />
            <meshStandardMaterial
              color={colors.main}
              emissive={colors.accent}
              emissiveIntensity={0.02}
              metalness={0.1}
              roughness={0.85}
            />
          </mesh>
          <mesh castShadow position={[0, -0.65, 0]}>
            <capsuleGeometry args={[0.11, 0.4, 8, 12]} />
            <meshStandardMaterial color={colors.dark} metalness={0.5} roughness={0.3} />
          </mesh>
          <mesh position={[0, -0.95, 0.05]}>
            <boxGeometry args={[0.18, 0.12, 0.28]} />
            <meshStandardMaterial color={colors.dark} metalness={0.7} roughness={0.2} />
          </mesh>
          <mesh position={[0, -0.92, 0.15]}>
            <boxGeometry args={[0.19, 0.04, 0.04]} />
            <meshBasicMaterial color={colors.accent} />
          </mesh>
        </group>

        {/* Right Leg */}
        <group ref={rightLegRef} position={[-0.22, 0.4, 0]}>
          <mesh castShadow position={[0, -0.25, 0]}>
            <capsuleGeometry args={[0.14, 0.4, 8, 12]} />
            <meshStandardMaterial color={colors.main} emissive={colors.main} emissiveIntensity={0.15} />
          </mesh>
          <mesh castShadow position={[0, -0.65, 0]}>
            <capsuleGeometry args={[0.11, 0.4, 8, 12]} />
            <meshStandardMaterial color={colors.dark} metalness={0.5} roughness={0.3} />
          </mesh>
          <mesh position={[0, -0.95, 0.05]}>
            <boxGeometry args={[0.18, 0.12, 0.28]} />
            <meshStandardMaterial color={colors.dark} metalness={0.7} roughness={0.2} />
          </mesh>
          <mesh position={[0, -0.92, 0.15]}>
            <boxGeometry args={[0.19, 0.04, 0.04]} />
            <meshBasicMaterial color={colors.accent} />
          </mesh>
        </group>
      </group>
    </group>
  );
}
