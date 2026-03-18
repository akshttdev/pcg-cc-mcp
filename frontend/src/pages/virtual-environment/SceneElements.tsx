import { useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

export function AtmosphericLighting() {
  return (
    <>
      {/* Hemisphere for ambient fill */}
      <hemisphereLight args={['#1d2a3f', '#000000', 0.4]} />

      {/* Directional moonlight */}
      <directionalLight
        position={[50, 100, 50]}
        intensity={0.5}
        color="#9db4ff"
        castShadow
        shadow-mapSize-width={2048}
        shadow-mapSize-height={2048}
        shadow-camera-far={500}
        shadow-camera-left={-200}
        shadow-camera-right={200}
        shadow-camera-top={200}
        shadow-camera-bottom={-200}
      />

      {/* Accent lights */}
      <pointLight position={[-100, 50, -100]} intensity={1} color="#ff8000" distance={200} decay={2} />
      <pointLight position={[100, 50, 100]} intensity={1} color="#0080ff" distance={200} decay={2} />
    </>
  );
}

// Enhanced particle system with wind effects
const PARTICLE_COUNT = 500;

export function EnhancedParticles() {
  const particlesRef = useRef<THREE.Points>(null);

  // Initialize particle positions and types
  const { positions, colors, sizes } = useMemo(() => {
    const pos = new Float32Array(PARTICLE_COUNT * 3);
    const cols = new Float32Array(PARTICLE_COUNT * 3);
    const szs = new Float32Array(PARTICLE_COUNT);

    const particleTypes = [
      { color: new THREE.Color('#00ffff'), size: 0.3 }, // Dust (cyan)
      { color: new THREE.Color('#ff8000'), size: 0.5 }, // Sparks (orange)
      { color: new THREE.Color('#00ff80'), size: 0.2 }, // Data bits (green)
    ];

    for (let i = 0; i < PARTICLE_COUNT; i++) {
      // Initial positions
      pos[i * 3] = (Math.random() - 0.5) * 400;
      pos[i * 3 + 1] = Math.random() * 100 + 10;
      pos[i * 3 + 2] = (Math.random() - 0.5) * 400;

      // Particle type (random distribution)
      const type = particleTypes[Math.floor(Math.random() * particleTypes.length)];
      cols[i * 3] = type.color.r;
      cols[i * 3 + 1] = type.color.g;
      cols[i * 3 + 2] = type.color.b;
      szs[i] = type.size;
    }

    return { positions: pos, colors: cols, sizes: szs };
  }, []);

  // Wind effect animation
  useFrame((state) => {
    if (!particlesRef.current) return;

    const time = state.clock.elapsedTime;

    // Wind direction (circular, slowly changing)
    const windX = Math.sin(time * 0.1) * 0.02;
    const windZ = Math.cos(time * 0.1) * 0.02;
    const windY = 0.01; // Slight upward drift

    const positionsAttr = particlesRef.current.geometry.attributes.position;

    for (let i = 0; i < PARTICLE_COUNT; i++) {
      // Apply wind
      positionsAttr.array[i * 3] += windX;
      positionsAttr.array[i * 3 + 1] += windY;
      positionsAttr.array[i * 3 + 2] += windZ;

      // Wrap around boundaries
      if (positionsAttr.array[i * 3] > 200) positionsAttr.array[i * 3] = -200;
      if (positionsAttr.array[i * 3] < -200) positionsAttr.array[i * 3] = 200;
      if (positionsAttr.array[i * 3 + 1] > 110) positionsAttr.array[i * 3 + 1] = 10;
      if (positionsAttr.array[i * 3 + 2] > 200) positionsAttr.array[i * 3 + 2] = -200;
      if (positionsAttr.array[i * 3 + 2] < -200) positionsAttr.array[i * 3 + 2] = 200;
    }

    positionsAttr.needsUpdate = true;
  });

  return (
    <points ref={particlesRef}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={PARTICLE_COUNT}
          array={positions}
          itemSize={3}
        />
        <bufferAttribute
          attach="attributes-color"
          count={PARTICLE_COUNT}
          array={colors}
          itemSize={3}
        />
        <bufferAttribute
          attach="attributes-size"
          count={PARTICLE_COUNT}
          array={sizes}
          itemSize={1}
        />
      </bufferGeometry>
      <pointsMaterial
        size={0.3}
        vertexColors
        transparent
        opacity={0.5}
        sizeAttenuation
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}
