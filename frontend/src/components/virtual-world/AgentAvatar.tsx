import { useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

interface AgentAvatarProps {
  position: [number, number, number];
  color: string;
  pulseOffset?: number;
  label?: string;
}

export function AgentAvatar({ position, color, pulseOffset = 0, label }: AgentAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const orbitRef = useRef<THREE.Mesh>(null);
  const colorObj = useMemo(() => new THREE.Color(color), [color]);

  useFrame((state) => {
    if (!groupRef.current) return;
    const t = state.clock.elapsedTime + pulseOffset;
    groupRef.current.position.y = position[1] + Math.sin(t * 1.2) * 0.5;
    groupRef.current.rotation.y = t * 0.4;

    if (orbitRef.current) {
      const material = orbitRef.current.material as THREE.MeshBasicMaterial;
      material.opacity = 0.4 + Math.sin(t * 2) * 0.2;
      orbitRef.current.scale.setScalar(1 + Math.sin(t * 1.5) * 0.05);
    }
  });

  return (
    <group ref={groupRef} position={position}>
      <mesh>
        <icosahedronGeometry args={[0.9, 0]} />
        <meshStandardMaterial
          color={colorObj}
          emissive={colorObj}
          emissiveIntensity={1.2}
          metalness={0.2}
          roughness={0.1}
        />
      </mesh>

      <mesh ref={orbitRef} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[1.2, 1.6, 64]} />
        <meshBasicMaterial color={colorObj} transparent opacity={0.5} side={THREE.DoubleSide} />
      </mesh>

      <mesh position={[0, -0.9, 0]}>
        <cylinderGeometry args={[0.1, 0.2, 1.2, 8]} />
        <meshStandardMaterial color={colorObj} transparent opacity={0.5} />
      </mesh>

      {label && (
        <Text position={[0, 1.5, 0]} fontSize={0.6} color={color} anchorX="center" anchorY="bottom">
          {label}
        </Text>
      )}
    </group>
  );
}
