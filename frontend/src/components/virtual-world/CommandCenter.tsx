import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

export function CommandCenter() {
  const spireRef = useRef<THREE.Mesh>(null);
  const ringRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (spireRef.current) {
      // Pulsing spire
      const pulse = Math.sin(state.clock.elapsedTime * 2) * 0.1 + 0.9;
      const material = spireRef.current.material as THREE.MeshStandardMaterial;
      if (material.emissiveIntensity !== undefined) {
        material.emissiveIntensity = pulse * 1.5;
      }
    }
    if (ringRef.current) {
      // Rotating base ring
      ringRef.current.rotation.z += 0.001;
    }
  });

  return (
    <group position={[0, 0, 0]}>
      {/* Elevated platform */}
      <mesh position={[0, 2.5, 0]} castShadow receiveShadow>
        <cylinderGeometry args={[15, 16, 5, 8]} />
        <meshStandardMaterial
          color="#0a1929"
          metalness={0.9}
          roughness={0.2}
          emissive="#003d5c"
          emissiveIntensity={0.3}
        />
      </mesh>

      {/* Moat/channel around platform */}
      <mesh ref={ringRef} position={[0, 0.2, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[16, 18, 64]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.6} side={THREE.DoubleSide} />
      </mesh>

      {/* Ground floor - octagonal pavilion */}
      <group position={[0, 5, 0]}>
        {/* Octagonal walls (glass) */}
        {Array.from({ length: 8 }).map((_, i) => {
          const angle = (i / 8) * Math.PI * 2;
          const x = Math.cos(angle) * 14;
          const z = Math.sin(angle) * 14;
          return (
            <mesh key={i} position={[x, 5, z]} rotation={[0, angle, 0]}>
              <boxGeometry args={[12, 10, 0.5]} />
              <meshPhysicalMaterial
                color="#0a4a6e"
                transmission={0.9}
                thickness={0.5}
                roughness={0.05}
                metalness={0.1}
                transparent
                opacity={0.3}
              />
            </mesh>
          );
        })}

        {/* Floor */}
        <mesh position={[0, 0, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
          <circleGeometry args={[15, 64]} />
          <meshStandardMaterial
            color="#0a1f35"
            metalness={0.8}
            roughness={0.3}
            emissive="#004080"
            emissiveIntensity={0.2}
          />
        </mesh>

        {/* Ceiling */}
        <mesh position={[0, 10, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <circleGeometry args={[15, 64]} />
          <meshPhysicalMaterial
            color="#0a4a6e"
            transmission={0.95}
            thickness={0.3}
            roughness={0.05}
            metalness={0.1}
            transparent
            opacity={0.2}
          />
        </mesh>
      </group>

      {/* Central spire */}
      <mesh ref={spireRef} position={[0, 30, 0]} castShadow>
        <cylinderGeometry args={[5, 6, 50, 8]} />
        <meshStandardMaterial
          color="#0b2035"
          emissive="#00c1ff"
          emissiveIntensity={1.2}
          transparent
          opacity={0.85}
          metalness={0.9}
          roughness={0.1}
        />
      </mesh>

      {/* Spire top beacon */}
      <pointLight position={[0, 55, 0]} intensity={3} color="#00ffff" distance={100} decay={2} />
      <mesh position={[0, 55, 0]}>
        <sphereGeometry args={[2, 16, 16]} />
        <meshBasicMaterial color="#00ffff" />
      </mesh>

      {/* Observation ring */}
      <group position={[0, 30, 0]}>
        <mesh rotation={[-Math.PI / 2, 0, 0]}>
          <torusGeometry args={[10, 0.5, 16, 64]} />
          <meshStandardMaterial
            color="#0a4a6e"
            emissive="#00c1ff"
            emissiveIntensity={0.8}
            metalness={0.9}
            roughness={0.1}
            transparent
            opacity={0.9}
          />
        </mesh>
      </group>

      {/* NORA's platform (holographic projection base) */}
      <mesh position={[0, 5.5, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <circleGeometry args={[3, 64]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.4} />
      </mesh>

      {/* Bridge access points */}
      {[0, Math.PI / 2, Math.PI, (Math.PI * 3) / 2].map((angle, i) => {
        const x = Math.cos(angle) * 15;
        const z = Math.sin(angle) * 15;
        return (
          <mesh key={i} position={[x, 2.5, z]} rotation={[0, angle, 0]}>
            <boxGeometry args={[10, 0.5, 4]} />
            <meshStandardMaterial
              color="#0a2f4a"
              metalness={0.8}
              roughness={0.3}
              emissive="#004080"
              emissiveIntensity={0.3}
            />
          </mesh>
        );
      })}

      {/* Label */}
      <Text position={[0, 60, 0]} fontSize={3} color="#baf4ff" anchorX="center" anchorY="middle">
        PCG COMMAND CENTER
      </Text>

      {/* Area lighting */}
      <spotLight
        position={[0, 60, 0]}
        angle={Math.PI / 4}
        penumbra={0.5}
        intensity={2}
        color="#00ffff"
        castShadow
        target-position={[0, 0, 0]}
      />
    </group>
  );
}
