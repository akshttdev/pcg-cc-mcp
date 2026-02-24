import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import { useOrchaStatus } from '@/hooks/api/useOrchaStatus';

interface OrchaAvatarProps {
  userPosition: [number, number, number];
}

/**
 * NORA as a tiny fairy that floats above and follows the admin.
 * Four translucent wings that flutter, an orbiting sparkle trail,
 * a wand with star tip, and a soft glow aura.
 */
export function OrchaAvatar({ userPosition }: OrchaAvatarProps) {
  const { data: status } = useOrchaStatus();
  const groupRef = useRef<THREE.Group>(null);
  const bodyRef = useRef<THREE.Mesh>(null);
  const wingULRef = useRef<THREE.Mesh>(null);
  const wingURRef = useRef<THREE.Mesh>(null);
  const wingLLRef = useRef<THREE.Mesh>(null);
  const wingLRRef = useRef<THREE.Mesh>(null);
  const sparkle1Ref = useRef<THREE.Mesh>(null);
  const sparkle2Ref = useRef<THREE.Mesh>(null);
  const sparkle3Ref = useRef<THREE.Mesh>(null);

  const prevPosition = useRef<THREE.Vector3>(new THREE.Vector3(
    userPosition[0] + 6,
    userPosition[1] + 2.5,
    userPosition[2] + 6,
  ));

  const name = status?.orchestratorName ?? 'NORA';
  const isAdmin = status?.isAdmin ?? false;

  // Color palette — gold/pearl for NORA admin, warm amber for ORCHA
  const dressColor  = isAdmin ? '#ffd700' : '#ff8c00';
  const wingColor   = isAdmin ? '#e8f4ff' : '#ffe8c0';
  const glowColor   = isAdmin ? '#fffacd' : '#ffd080';
  const hairColor   = isAdmin ? '#c0a060' : '#a06020';

  useFrame((state) => {
    if (!groupRef.current) return;
    const t = state.clock.elapsedTime;

    // Float 2.5 units above user, trailing slightly behind/beside
    const targetX = userPosition[0] + 4;
    const targetY = userPosition[1] + 2.5;
    const targetZ = userPosition[2] + 4;
    const target = new THREE.Vector3(targetX, targetY, targetZ);

    const current = prevPosition.current;
    const distance = current.distanceTo(target);
    const lerpFactor = distance > 10 ? 0.03 : distance > 2 ? 0.015 : 0.005;
    current.lerp(target, lerpFactor);
    groupRef.current.position.set(current.x, current.y, current.z);

    // Body hover — energetic bounce while moving, gentle idle drift
    if (bodyRef.current) {
      bodyRef.current.position.y = distance > 2
        ? Math.abs(Math.sin(t * 7)) * 0.12
        : Math.sin(t * 2) * 0.06;
    }

    // Wing flutter — rapid Y-axis oscillation, mirrored L/R
    const beat = Math.sin(t * 12) * 0.55;
    if (wingULRef.current) wingULRef.current.rotation.y = -beat;
    if (wingURRef.current) wingURRef.current.rotation.y =  beat;
    if (wingLLRef.current) wingLLRef.current.rotation.y = -beat * 0.65;
    if (wingLRRef.current) wingLRRef.current.rotation.y =  beat * 0.65;

    // Orbiting sparkle trail — three dots at different phases
    const r = 0.48;
    if (sparkle1Ref.current) {
      sparkle1Ref.current.position.set(
        Math.cos(t * 3) * r,
        0.8 + Math.sin(t * 2) * 0.18,
        Math.sin(t * 3) * r,
      );
    }
    if (sparkle2Ref.current) {
      sparkle2Ref.current.position.set(
        Math.cos(t * 3 + (Math.PI * 2) / 3) * (r - 0.06),
        0.9 + Math.sin(t * 2.5) * 0.18,
        Math.sin(t * 3 + (Math.PI * 2) / 3) * (r - 0.06),
      );
    }
    if (sparkle3Ref.current) {
      sparkle3Ref.current.position.set(
        Math.cos(t * 3 + (Math.PI * 4) / 3) * (r - 0.1),
        1.0 + Math.sin(t * 3) * 0.15,
        Math.sin(t * 3 + (Math.PI * 4) / 3) * (r - 0.1),
      );
    }

    // Always face toward the user
    const lookAt = new THREE.Vector3(userPosition[0], current.y, userPosition[2]);
    groupRef.current.lookAt(lookAt);
  });

  return (
    // Scale 0.5 keeps her fairy-tiny relative to the world
    <group ref={groupRef} scale={[0.5, 0.5, 0.5]}>

      {/* ── Soft outer glow aura ── */}
      <mesh position={[0, 1.1, 0]}>
        <sphereGeometry args={[0.78, 12, 8]} />
        <meshStandardMaterial
          color={glowColor}
          transparent
          opacity={0.07}
          emissive={glowColor}
          emissiveIntensity={0.6}
        />
      </mesh>

      {/* ── Dress / body (cone) ── */}
      <mesh ref={bodyRef} position={[0, 0.6, 0]} castShadow>
        <coneGeometry args={[0.22, 0.55, 10]} />
        <meshStandardMaterial
          color={dressColor}
          emissive={dressColor}
          emissiveIntensity={0.4}
          metalness={0.15}
          roughness={0.55}
        />
      </mesh>

      {/* ── Torso nub connecting dress to head ── */}
      <mesh position={[0, 0.92, 0]}>
        <sphereGeometry args={[0.13, 10, 8]} />
        <meshStandardMaterial color={dressColor} emissive={dressColor} emissiveIntensity={0.25} />
      </mesh>

      {/* ── Head ── */}
      <mesh position={[0, 1.18, 0]} castShadow>
        <sphereGeometry args={[0.21, 16, 12]} />
        <meshStandardMaterial color="#ffe8cc" roughness={0.85} metalness={0.0} />
      </mesh>

      {/* ── Hair ── */}
      <mesh position={[0, 1.38, 0]}>
        <sphereGeometry args={[0.17, 12, 8]} />
        <meshStandardMaterial color={hairColor} roughness={0.95} />
      </mesh>
      <mesh position={[0, 1.42, -0.1]} scale={[0.75, 0.55, 1.15]}>
        <sphereGeometry args={[0.15, 10, 6]} />
        <meshStandardMaterial color={hairColor} roughness={0.95} />
      </mesh>

      {/* ── Eyes ── */}
      <mesh position={[-0.08, 1.2, 0.18]}>
        <sphereGeometry args={[0.045, 8, 8]} />
        <meshBasicMaterial color="#1a0a2e" />
      </mesh>
      <mesh position={[0.08, 1.2, 0.18]}>
        <sphereGeometry args={[0.045, 8, 8]} />
        <meshBasicMaterial color="#1a0a2e" />
      </mesh>
      {/* Eye shine */}
      <mesh position={[-0.065, 1.215, 0.216]}>
        <sphereGeometry args={[0.014, 6, 6]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>
      <mesh position={[0.095, 1.215, 0.216]}>
        <sphereGeometry args={[0.014, 6, 6]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>

      {/* ── Wings — upper pair ── */}
      <mesh ref={wingULRef}
        position={[-0.32, 1.1, -0.08]}
        rotation={[0.15, 0, 0.45]}
        scale={[1, 0.54, 0.07]}
      >
        <sphereGeometry args={[0.5, 14, 8]} />
        <meshStandardMaterial
          color={wingColor}
          transparent opacity={0.72}
          emissive={wingColor} emissiveIntensity={0.28}
          side={THREE.DoubleSide}
        />
      </mesh>
      <mesh ref={wingURRef}
        position={[0.32, 1.1, -0.08]}
        rotation={[0.15, 0, -0.45]}
        scale={[1, 0.54, 0.07]}
      >
        <sphereGeometry args={[0.5, 14, 8]} />
        <meshStandardMaterial
          color={wingColor}
          transparent opacity={0.72}
          emissive={wingColor} emissiveIntensity={0.28}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* ── Wings — lower pair (smaller) ── */}
      <mesh ref={wingLLRef}
        position={[-0.23, 0.82, -0.08]}
        rotation={[0.2, 0, 0.78]}
        scale={[0.66, 0.42, 0.07]}
      >
        <sphereGeometry args={[0.38, 12, 8]} />
        <meshStandardMaterial
          color={wingColor}
          transparent opacity={0.58}
          emissive={wingColor} emissiveIntensity={0.18}
          side={THREE.DoubleSide}
        />
      </mesh>
      <mesh ref={wingLRRef}
        position={[0.23, 0.82, -0.08]}
        rotation={[0.2, 0, -0.78]}
        scale={[0.66, 0.42, 0.07]}
      >
        <sphereGeometry args={[0.38, 12, 8]} />
        <meshStandardMaterial
          color={wingColor}
          transparent opacity={0.58}
          emissive={wingColor} emissiveIntensity={0.18}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* ── Wand ── */}
      <mesh position={[0.28, 1.15, 0.18]} rotation={[0, 0, -Math.PI / 4]}>
        <cylinderGeometry args={[0.012, 0.012, 0.42, 6]} />
        <meshStandardMaterial color="#b8860b" metalness={0.9} roughness={0.15} />
      </mesh>
      {/* Wand star tip */}
      <mesh position={[0.43, 1.0, 0.18]}>
        <octahedronGeometry args={[0.072, 0]} />
        <meshStandardMaterial
          color="#fffde0"
          emissive="#ffffff"
          emissiveIntensity={1.6}
        />
      </mesh>

      {/* ── Orbiting sparkle trail ── */}
      <mesh ref={sparkle1Ref}>
        <sphereGeometry args={[0.042, 6, 6]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>
      <mesh ref={sparkle2Ref}>
        <sphereGeometry args={[0.034, 6, 6]} />
        <meshBasicMaterial color={wingColor} />
      </mesh>
      <mesh ref={sparkle3Ref}>
        <sphereGeometry args={[0.028, 6, 6]} />
        <meshBasicMaterial color={glowColor} />
      </mesh>

      {/* ── Point light (warm glow) ── */}
      <pointLight position={[0, 1.1, 0]} intensity={1.3} color={glowColor} distance={8} />

      {/* ── Name label ── */}
      <Text
        position={[0, 2.05, 0]}
        fontSize={0.26}
        color={glowColor}
        anchorX="center"
        anchorY="bottom"
        outlineWidth={0.025}
        outlineColor="#000000"
      >
        {name}
      </Text>
    </group>
  );
}
