import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import { useOrchaStatus } from '@/hooks/api/useOrchaStatus';

interface OrchaAvatarProps {
  /** Current user position to follow */
  userPosition: [number, number, number];
}

/** Personal orchestrator agent that follows the user around the VE.
 *  NORA for admin, ORCHA-{username} for regular users.
 *  Distinguished by a halo torus and slightly larger scale.
 */
export function OrchaAvatar({ userPosition }: OrchaAvatarProps) {
  const { data: status } = useOrchaStatus();
  const groupRef = useRef<THREE.Group>(null);
  const bodyRef = useRef<THREE.Mesh>(null);
  const haloRef = useRef<THREE.Mesh>(null);
  const prevPosition = useRef<THREE.Vector3>(new THREE.Vector3(
    userPosition[0] + 7,
    userPosition[1],
    userPosition[2] + 7,
  ));
  const isWalking = useRef(false);

  const name = status?.orchestratorName ?? 'ORCHA';
  const isAdmin = status?.isAdmin ?? false;
  const bodyColor = isAdmin ? '#ffd700' : '#ff8c00';
  const haloColor = isAdmin ? '#ffd700' : '#ffa500';

  useFrame((state) => {
    if (!groupRef.current) return;

    const t = state.clock.elapsedTime;

    // Target: follow user at 7-unit distance behind and to the right
    const targetX = userPosition[0] + 5;
    const targetY = userPosition[1] + 0.5;
    const targetZ = userPosition[2] + 5;
    const target = new THREE.Vector3(targetX, targetY, targetZ);

    // Lerp toward target
    const current = prevPosition.current;
    const distance = current.distanceTo(target);

    // Only move if more than 1 unit away; stop within 5-10 range
    const lerpFactor = distance > 10 ? 0.03 : distance > 2 ? 0.015 : 0.005;
    current.lerp(target, lerpFactor);

    groupRef.current.position.set(current.x, current.y, current.z);

    // Walking detection
    isWalking.current = distance > 2;

    // Bob animation when walking
    if (bodyRef.current) {
      if (isWalking.current) {
        bodyRef.current.position.y = 1.0 + Math.abs(Math.sin(t * 6)) * 0.15;
      } else {
        // Idle hover
        bodyRef.current.position.y = 1.0 + Math.sin(t * 1.5) * 0.05;
      }
    }

    // Halo rotation
    if (haloRef.current) {
      haloRef.current.rotation.z = t * 1.2;
      haloRef.current.rotation.x = Math.sin(t * 0.5) * 0.15;
    }

    // Face toward user
    const lookTarget = new THREE.Vector3(userPosition[0], current.y, userPosition[2]);
    groupRef.current.lookAt(lookTarget);
  });

  return (
    <group ref={groupRef} scale={[1.15, 1.15, 1.15]}>
      {/* Body — capsule shape */}
      <mesh ref={bodyRef} position={[0, 1.0, 0]} castShadow>
        <capsuleGeometry args={[0.35, 0.8, 8, 16]} />
        <meshStandardMaterial
          color={bodyColor}
          metalness={0.6}
          roughness={0.3}
          emissive={bodyColor}
          emissiveIntensity={0.2}
        />
      </mesh>

      {/* Head */}
      <mesh position={[0, 2.0, 0]} castShadow>
        <sphereGeometry args={[0.3, 16, 16]} />
        <meshStandardMaterial
          color={bodyColor}
          metalness={0.5}
          roughness={0.3}
          emissive={bodyColor}
          emissiveIntensity={0.15}
        />
      </mesh>

      {/* Eyes — two small emissive spheres */}
      <mesh position={[-0.1, 2.05, 0.25]}>
        <sphereGeometry args={[0.06, 8, 8]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>
      <mesh position={[0.1, 2.05, 0.25]}>
        <sphereGeometry args={[0.06, 8, 8]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>

      {/* Halo torus — distinguishes orchestrator from sub-agents */}
      <mesh ref={haloRef} position={[0, 2.5, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[0.35, 0.04, 8, 32]} />
        <meshBasicMaterial color={haloColor} transparent opacity={0.8} />
      </mesh>

      {/* Glow light */}
      <pointLight position={[0, 1.5, 0]} intensity={0.8} color={bodyColor} distance={6} />

      {/* Name label */}
      <Text
        position={[0, 2.9, 0]}
        fontSize={0.22}
        color={haloColor}
        anchorX="center"
        anchorY="bottom"
        outlineWidth={0.02}
        outlineColor="#000000"
      >
        {name}
      </Text>
    </group>
  );
}
