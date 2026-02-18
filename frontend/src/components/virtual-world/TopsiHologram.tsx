import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import { useTopsiRecommendations, type Recommendation } from '@/hooks/api/useTopsiRecommendations';

interface TopsiHologramProps {
  position?: [number, number, number];
  projectId?: string;
}

/** Holographic cube on the holo table showing VIBE-earning recommendations from Topsi */
export function TopsiHologram({ position = [0, 3.5, 0], projectId }: TopsiHologramProps) {
  const { data } = useTopsiRecommendations(projectId);
  const groupRef = useRef<THREE.Group>(null);
  const particlesRef = useRef<THREE.Points>(null);
  const cubeRef = useRef<THREE.Mesh>(null);

  // Recommendation data
  const recommendations = data?.recommendations ?? [];
  const top3 = recommendations.slice(0, 3);

  // Particle positions for orbiting effect
  const particlePositions = useMemo(() => {
    const count = 50;
    const positions = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.random() * Math.PI;
      const r = 1.2 + Math.random() * 0.6;
      positions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = r * Math.cos(phi);
      positions[i * 3 + 2] = r * Math.sin(phi) * Math.sin(theta);
    }
    return positions;
  }, []);

  useFrame((state) => {
    const t = state.clock.elapsedTime;

    // Rotate the whole hologram slowly
    if (groupRef.current) {
      groupRef.current.rotation.y = t * 0.3;
    }

    // Pulse the cube on data refresh
    if (cubeRef.current) {
      const pulse = 1 + Math.sin(t * 2) * 0.03;
      cubeRef.current.scale.set(pulse, pulse, pulse);
    }

    // Orbit particles
    if (particlesRef.current) {
      particlesRef.current.rotation.y = t * 0.5;
      particlesRef.current.rotation.x = Math.sin(t * 0.2) * 0.1;
    }
  });

  return (
    <group position={position}>
      {/* Glass cube */}
      <group ref={groupRef}>
        <mesh ref={cubeRef}>
          <boxGeometry args={[1.5, 1.5, 1.5]} />
          <meshPhysicalMaterial
            color="#00e5ff"
            transmission={0.85}
            thickness={0.5}
            roughness={0.05}
            metalness={0.1}
            emissive="#00bcd4"
            emissiveIntensity={0.15}
            transparent
            opacity={0.3}
            side={THREE.DoubleSide}
          />
        </mesh>

        {/* Inner glow core */}
        <mesh>
          <sphereGeometry args={[0.3, 16, 16]} />
          <meshBasicMaterial color="#ffd700" transparent opacity={0.6} />
        </mesh>
      </group>

      {/* Orbiting particles */}
      <points ref={particlesRef}>
        <bufferGeometry>
          <bufferAttribute
            attach="attributes-position"
            args={[particlePositions, 3]}
          />
        </bufferGeometry>
        <pointsMaterial color="#00e5ff" size={0.04} transparent opacity={0.7} sizeAttenuation />
      </points>

      {/* Point light glow */}
      <pointLight position={[0, 0, 0]} intensity={1.5} color="#00e5ff" distance={8} />
      <pointLight position={[0, 0.5, 0]} intensity={0.5} color="#ffd700" distance={5} />

      {/* Recommendation text labels floating above */}
      {top3.map((rec, i) => (
        <RecommendationLabel
          key={rec.taskId}
          recommendation={rec}
          index={i}
        />
      ))}

      {/* "TOPSI" label */}
      <Text
        position={[0, -1.2, 0]}
        fontSize={0.25}
        color="#00e5ff"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.02}
        outlineColor="#000000"
      >
        TOPSI
      </Text>
    </group>
  );
}

function RecommendationLabel({
  recommendation,
  index,
}: {
  recommendation: Recommendation;
  index: number;
}) {
  const groupRef = useRef<THREE.Group>(null);
  const yOffset = 1.5 + index * 1.0;

  // Truncate long names
  const name =
    recommendation.taskName.length > 28
      ? recommendation.taskName.slice(0, 25) + '...'
      : recommendation.taskName;

  const vibeStr = recommendation.vibeEstimate
    ? `+${Math.round(recommendation.vibeEstimate)} VIBE`
    : '';

  useFrame((state) => {
    if (!groupRef.current) return;
    const t = state.clock.elapsedTime;
    // Gentle float
    groupRef.current.position.y = yOffset + Math.sin(t * 1.5 + index * 1.2) * 0.08;
    // Face camera
    groupRef.current.lookAt(state.camera.position);
  });

  return (
    <group ref={groupRef} position={[0, yOffset, 0]}>
      {/* Task name */}
      <Text
        position={[0, 0.15, 0]}
        fontSize={0.18}
        color="#ffffff"
        anchorX="center"
        anchorY="bottom"
        maxWidth={3}
      >
        {name}
      </Text>
      {/* VIBE estimate */}
      {vibeStr && (
        <Text
          position={[0, -0.05, 0]}
          fontSize={0.14}
          color="#ffd700"
          anchorX="center"
          anchorY="top"
        >
          {vibeStr}
        </Text>
      )}
    </group>
  );
}
