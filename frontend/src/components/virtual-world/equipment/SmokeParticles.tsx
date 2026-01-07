import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface SmokeParticlesProps {
  position: [number, number, number];
  intensity?: number;
  count?: number;
}

export function SmokeParticles({ position, intensity = 1.0, count = 20 }: SmokeParticlesProps) {
  const particlesRef = useRef<THREE.Points>(null);

  const { positions, velocities, lifetimes } = useMemo(() => {
    const pos = new Float32Array(count * 3);
    const vel = new Float32Array(count * 3);
    const life = new Float32Array(count);

    for (let i = 0; i < count; i++) {
      // Start at origin
      pos[i * 3] = 0;
      pos[i * 3 + 1] = 0;
      pos[i * 3 + 2] = 0;

      // Random upward drift with spread
      vel[i * 3] = (Math.random() - 0.5) * 0.02;
      vel[i * 3 + 1] = 0.01 + Math.random() * 0.02;
      vel[i * 3 + 2] = (Math.random() - 0.5) * 0.02;

      // Staggered lifetimes
      life[i] = Math.random();
    }

    return { positions: pos, velocities: vel, lifetimes: life };
  }, [count]);

  useFrame((_, delta) => {
    if (!particlesRef.current) return;

    const posAttr = particlesRef.current.geometry.attributes.position;
    const posArray = posAttr.array as Float32Array;

    for (let i = 0; i < count; i++) {
      // Update lifetime
      lifetimes[i] += delta * 0.5;

      if (lifetimes[i] > 1) {
        // Reset particle
        lifetimes[i] = 0;
        posArray[i * 3] = 0;
        posArray[i * 3 + 1] = 0;
        posArray[i * 3 + 2] = 0;
      } else {
        // Move particle
        posArray[i * 3] += velocities[i * 3] * intensity;
        posArray[i * 3 + 1] += velocities[i * 3 + 1] * intensity;
        posArray[i * 3 + 2] += velocities[i * 3 + 2] * intensity;
      }
    }

    posAttr.needsUpdate = true;
  });

  return (
    <points ref={particlesRef} position={position}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={count}
          array={positions}
          itemSize={3}
        />
      </bufferGeometry>
      <pointsMaterial
        size={0.04}
        color="#cccccc"
        transparent
        opacity={0.4}
        sizeAttenuation
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}
