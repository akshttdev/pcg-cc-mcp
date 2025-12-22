import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

const BASE_RADIUS = 40;
const FLOOR_ELEVATION = 8;
const WALL_HEIGHT = 24;
const GLASS_SEGMENTS = 12;
const DOORWAYS = [0, Math.PI / 2, Math.PI, (Math.PI * 3) / 2];

export function CommandCenter() {
  const spireRef = useRef<THREE.Mesh>(null);
  const ringRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (spireRef.current) {
      const pulse = Math.sin(state.clock.elapsedTime * 2) * 0.1 + 0.9;
      const material = spireRef.current.material as THREE.MeshStandardMaterial;
      material.emissiveIntensity = pulse * 1.6;
    }
    if (ringRef.current) {
      ringRef.current.rotation.z += 0.001;
    }
  });

  return (
    <group>
      <mesh position={[0, FLOOR_ELEVATION / 2, 0]} castShadow receiveShadow>
        <cylinderGeometry args={[BASE_RADIUS, BASE_RADIUS + 2, FLOOR_ELEVATION, 24]} />
        <meshStandardMaterial
          color="#08121f"
          metalness={0.85}
          roughness={0.25}
          emissive="#003d5c"
          emissiveIntensity={0.35}
        />
      </mesh>

      <mesh ref={ringRef} position={[0, FLOOR_ELEVATION + 0.2, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[BASE_RADIUS + 1, BASE_RADIUS + 3, 48]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.5} side={THREE.DoubleSide} />
      </mesh>

      {/* Interior floor */}
      <mesh position={[0, FLOOR_ELEVATION, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <circleGeometry args={[BASE_RADIUS - 4, 48]} />
        <meshStandardMaterial
          color="#0a1f35"
          metalness={0.8}
          roughness={0.35}
          emissive="#004080"
          emissiveIntensity={0.2}
        />
      </mesh>

      {/* Glass curtain walls */}
      {Array.from({ length: GLASS_SEGMENTS }).map((_, idx) => {
        const angle = (idx / GLASS_SEGMENTS) * Math.PI * 2;
        const doorway = DOORWAYS.some((door) => {
          const diff = Math.atan2(Math.sin(angle - door), Math.cos(angle - door));
          return Math.abs(diff) < 0.25;
        });
        if (doorway) return null;
        const x = Math.cos(angle) * (BASE_RADIUS - 2);
        const z = Math.sin(angle) * (BASE_RADIUS - 2);
        return (
          <mesh key={idx} position={[x, FLOOR_ELEVATION + WALL_HEIGHT / 2, z]} rotation={[0, angle, 0]}>
            <boxGeometry args={[12, WALL_HEIGHT, 0.8]} />
            <meshPhysicalMaterial
              color="#0a4a6e"
              transmission={0.95}
              thickness={0.6}
              roughness={0.05}
              metalness={0.1}
              transparent
              opacity={0.3}
            />
          </mesh>
        );
      })}

      {/* Ceiling glass */}
      <mesh position={[0, FLOOR_ELEVATION + WALL_HEIGHT, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <circleGeometry args={[BASE_RADIUS - 2, 48]} />
        <meshPhysicalMaterial
          color="#0a4a6e"
          transmission={0.95}
          thickness={0.4}
          roughness={0.04}
          metalness={0.15}
          transparent
          opacity={0.25}
        />
      </mesh>

      {/* Interior skybridge ring */}
      <mesh position={[0, FLOOR_ELEVATION + 2, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <ringGeometry args={[BASE_RADIUS - 10, BASE_RADIUS - 6, 40]} />
        <meshStandardMaterial color="#052c38" metalness={0.6} roughness={0.4} />
      </mesh>

      {/* NORA dais */}
      <mesh position={[0, FLOOR_ELEVATION + 1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <circleGeometry args={[6, 32]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.35} />
      </mesh>

      {/* Command-center bridges */}
      {DOORWAYS.map((angle, idx) => {
        const x = Math.cos(angle) * (BASE_RADIUS - 2);
        const z = Math.sin(angle) * (BASE_RADIUS - 2);
        return (
          <mesh key={idx} position={[x, FLOOR_ELEVATION, z]} rotation={[0, angle, 0]}>
            <boxGeometry args={[20, 0.6, 6]} />
            <meshStandardMaterial
              color="#0a2f4a"
              metalness={0.8}
              roughness={0.3}
              emissive="#004080"
              emissiveIntensity={0.4}
            />
          </mesh>
        );
      })}

      {/* Central spire */}
      <mesh ref={spireRef} position={[0, FLOOR_ELEVATION + WALL_HEIGHT + 10, 0]} castShadow>
        <cylinderGeometry args={[6, 8, 50, 16]} />
        <meshStandardMaterial
          color="#0b2035"
          emissive="#00c1ff"
          emissiveIntensity={1.2}
          transparent
          opacity={0.8}
          metalness={0.95}
          roughness={0.08}
        />
      </mesh>

      <pointLight
        position={[0, FLOOR_ELEVATION + WALL_HEIGHT + 35, 0]}
        intensity={3}
        color="#00ffff"
        distance={160}
        decay={2}
      />

      <mesh position={[0, FLOOR_ELEVATION + WALL_HEIGHT + 35, 0]}>
        <sphereGeometry args={[3, 24, 24]} />
        <meshBasicMaterial color="#00ffff" />
      </mesh>

      <Text position={[0, FLOOR_ELEVATION + WALL_HEIGHT + 45, 0]} fontSize={3.5} color="#baf4ff" anchorX="center" anchorY="middle">
        PCG COMMAND CENTER
      </Text>

      <spotLight
        position={[0, FLOOR_ELEVATION + WALL_HEIGHT + 40, 0]}
        angle={Math.PI / 4}
        penumbra={0.5}
        intensity={2}
        color="#00ffff"
        castShadow
        target-position={[0, FLOOR_ELEVATION, 0]}
      />
    </group>
  );
}
