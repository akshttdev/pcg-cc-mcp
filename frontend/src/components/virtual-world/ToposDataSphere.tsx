import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface ToposDataSphereProps {
  toposItems: string[];
  commandCenterHeight?: number;
}

// Sphere configuration
const SPHERE_RADIUS = 40;
const SPHERE_CENTER_Y = -20; // Sphere center below ground, so top portion visible

// Beam configuration
const BEAM_RADIUS = 10; // Consistent width cylinder
const CONE_START_HEIGHT = 95; // Where cone starts (above Command Center at 80)
const CONE_END_HEIGHT = 140; // Where cone fades out
const CONE_END_RADIUS = 50; // How wide the cone expands

// Particle counts
const PARTICLES_PER_ITEM = 30;
const MIN_SPHERE_PARTICLES = 800;
const MAX_SPHERE_PARTICLES = 3000;
const BEAM_PARTICLE_COUNT = 400;
const CONE_PARTICLE_COUNT = 200;

export function ToposDataSphere({
  toposItems,
  commandCenterHeight = 80
}: ToposDataSphereProps) {
  const sphereParticlesRef = useRef<THREE.Points>(null);
  const beamParticlesRef = useRef<THREE.Points>(null);
  const coneParticlesRef = useRef<THREE.Points>(null);

  // Calculate sphere particle count based on topos data
  const sphereParticleCount = useMemo(() => {
    const count = Math.max(MIN_SPHERE_PARTICLES, toposItems.length * PARTICLES_PER_ITEM);
    return Math.min(count, MAX_SPHERE_PARTICLES);
  }, [toposItems.length]);

  // Initialize sphere particles - CONTAINED inside the sphere
  const { spherePositions, sphereColors, sphereBasePositions } = useMemo(() => {
    const positions = new Float32Array(sphereParticleCount * 3);
    const basePositions = new Float32Array(sphereParticleCount * 3);
    const colors = new Float32Array(sphereParticleCount * 3);

    const colorPalette = [
      new THREE.Color('#00ffff'), // Cyan
      new THREE.Color('#40a0ff'), // Light blue
      new THREE.Color('#80c0ff'), // Pale blue
      new THREE.Color('#ffffff'), // White
    ];

    for (let i = 0; i < sphereParticleCount; i++) {
      // Spherical distribution INSIDE the sphere - denser toward center
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      // Higher power = more concentrated in center
      const r = Math.pow(Math.random(), 0.6) * SPHERE_RADIUS * 0.95;

      const x = r * Math.sin(phi) * Math.cos(theta);
      const y = SPHERE_CENTER_Y + r * Math.cos(phi);
      const z = r * Math.sin(phi) * Math.sin(theta);

      positions[i * 3] = x;
      positions[i * 3 + 1] = y;
      positions[i * 3 + 2] = z;

      basePositions[i * 3] = x;
      basePositions[i * 3 + 1] = y;
      basePositions[i * 3 + 2] = z;

      // Color - mostly cyan/blue with some white highlights
      const color = colorPalette[Math.floor(Math.random() * colorPalette.length)];
      const intensity = 0.6 + Math.random() * 0.4;
      colors[i * 3] = color.r * intensity;
      colors[i * 3 + 1] = color.g * intensity;
      colors[i * 3 + 2] = color.b * intensity;
    }

    return { spherePositions: positions, sphereColors: colors, sphereBasePositions: basePositions };
  }, [sphereParticleCount]);

  // Initialize beam particles - cylindrical, flowing upward
  const { beamPositions, beamColors, beamSpeeds } = useMemo(() => {
    const positions = new Float32Array(BEAM_PARTICLE_COUNT * 3);
    const colors = new Float32Array(BEAM_PARTICLE_COUNT * 3);
    const speeds = new Float32Array(BEAM_PARTICLE_COUNT);

    for (let i = 0; i < BEAM_PARTICLE_COUNT; i++) {
      // Cylindrical distribution
      const angle = Math.random() * Math.PI * 2;
      const r = Math.random() * BEAM_RADIUS;
      // From top of sphere to command center
      const y = (SPHERE_CENTER_Y + SPHERE_RADIUS * 0.8) + Math.random() * (commandCenterHeight - SPHERE_CENTER_Y);

      positions[i * 3] = Math.cos(angle) * r;
      positions[i * 3 + 1] = y;
      positions[i * 3 + 2] = Math.sin(angle) * r;

      // Cyan/white color
      const whiteness = Math.random() * 0.5;
      colors[i * 3] = 0.5 + whiteness;
      colors[i * 3 + 1] = 1.0;
      colors[i * 3 + 2] = 1.0;

      speeds[i] = 0.8 + Math.random() * 1.2;
    }

    return { beamPositions: positions, beamColors: colors, beamSpeeds: speeds };
  }, [commandCenterHeight]);

  // Initialize cone particles - expanding after passing Nora
  const { conePositions, coneColors, coneSpeeds } = useMemo(() => {
    const positions = new Float32Array(CONE_PARTICLE_COUNT * 3);
    const colors = new Float32Array(CONE_PARTICLE_COUNT * 3);
    const speeds = new Float32Array(CONE_PARTICLE_COUNT);

    for (let i = 0; i < CONE_PARTICLE_COUNT; i++) {
      // Start in the cone region
      const t = Math.random(); // 0 = start of cone, 1 = end of cone
      const y = CONE_START_HEIGHT + t * (CONE_END_HEIGHT - CONE_START_HEIGHT);

      // Radius expands as we go up
      const radiusAtY = BEAM_RADIUS + t * (CONE_END_RADIUS - BEAM_RADIUS);
      const angle = Math.random() * Math.PI * 2;
      const r = Math.random() * radiusAtY;

      positions[i * 3] = Math.cos(angle) * r;
      positions[i * 3 + 1] = y;
      positions[i * 3 + 2] = Math.sin(angle) * r;

      // Fading cyan color
      const fade = 1 - t * 0.7;
      colors[i * 3] = 0.5 * fade;
      colors[i * 3 + 1] = 1.0 * fade;
      colors[i * 3 + 2] = 1.0 * fade;

      speeds[i] = 0.5 + Math.random() * 0.8;
    }

    return { conePositions: positions, coneColors: colors, coneSpeeds: speeds };
  }, []);

  // Animate particles
  useFrame((state) => {
    const time = state.clock.elapsedTime;

    // Animate sphere particles - swirling inside the sphere
    if (sphereParticlesRef.current) {
      const positions = sphereParticlesRef.current.geometry.attributes.position;

      for (let i = 0; i < sphereParticleCount; i++) {
        const baseX = sphereBasePositions[i * 3];
        const baseY = sphereBasePositions[i * 3 + 1];
        const baseZ = sphereBasePositions[i * 3 + 2];

        // Calculate distance from sphere center for orbit speed
        const distFromCenter = Math.sqrt(baseX * baseX + (baseY - SPHERE_CENTER_Y) ** 2 + baseZ * baseZ);
        const orbitSpeed = 0.1 + (1 - distFromCenter / SPHERE_RADIUS) * 0.2;

        // Gentle orbital motion around Y axis
        const cos = Math.cos(time * orbitSpeed);
        const sin = Math.sin(time * orbitSpeed);

        // Keep particles inside the sphere
        let newX = baseX * cos - baseZ * sin;
        let newZ = baseX * sin + baseZ * cos;
        let newY = baseY + Math.sin(time * 0.5 + i * 0.1) * 1.5;

        // Ensure particles stay within sphere bounds
        const newDist = Math.sqrt(newX * newX + (newY - SPHERE_CENTER_Y) ** 2 + newZ * newZ);
        if (newDist > SPHERE_RADIUS * 0.95) {
          const scale = (SPHERE_RADIUS * 0.95) / newDist;
          newX *= scale;
          newZ *= scale;
          newY = SPHERE_CENTER_Y + (newY - SPHERE_CENTER_Y) * scale;
        }

        positions.array[i * 3] = newX;
        positions.array[i * 3 + 1] = newY;
        positions.array[i * 3 + 2] = newZ;
      }

      positions.needsUpdate = true;
    }

    // Animate beam particles - flow upward through cylinder
    if (beamParticlesRef.current) {
      const positions = beamParticlesRef.current.geometry.attributes.position;
      const beamBottom = SPHERE_CENTER_Y + SPHERE_RADIUS * 0.5;

      for (let i = 0; i < BEAM_PARTICLE_COUNT; i++) {
        const speed = beamSpeeds[i];

        // Move upward
        positions.array[i * 3 + 1] += speed * 0.3;

        // Reset to bottom when reaching command center
        if (positions.array[i * 3 + 1] > CONE_START_HEIGHT) {
          positions.array[i * 3 + 1] = beamBottom;

          // Randomize horizontal position within cylinder
          const angle = Math.random() * Math.PI * 2;
          const r = Math.random() * BEAM_RADIUS;
          positions.array[i * 3] = Math.cos(angle) * r;
          positions.array[i * 3 + 2] = Math.sin(angle) * r;
        }

        // Subtle spiral motion
        const currentX = positions.array[i * 3];
        const currentZ = positions.array[i * 3 + 2];
        const currentR = Math.sqrt(currentX * currentX + currentZ * currentZ);

        if (currentR > 0.1) {
          const currentAngle = Math.atan2(currentZ, currentX);
          const newAngle = currentAngle + 0.01;
          positions.array[i * 3] = Math.cos(newAngle) * currentR;
          positions.array[i * 3 + 2] = Math.sin(newAngle) * currentR;
        }
      }

      positions.needsUpdate = true;
    }

    // Animate cone particles - expand and fade upward
    if (coneParticlesRef.current) {
      const positions = coneParticlesRef.current.geometry.attributes.position;
      const colors = coneParticlesRef.current.geometry.attributes.color;

      for (let i = 0; i < CONE_PARTICLE_COUNT; i++) {
        const speed = coneSpeeds[i];

        // Move upward
        positions.array[i * 3 + 1] += speed * 0.2;

        // Calculate t for current position
        const y = positions.array[i * 3 + 1];
        const t = (y - CONE_START_HEIGHT) / (CONE_END_HEIGHT - CONE_START_HEIGHT);

        // Expand outward as rising
        if (t >= 0 && t <= 1) {
          const targetRadius = BEAM_RADIUS + t * (CONE_END_RADIUS - BEAM_RADIUS);
          const currentX = positions.array[i * 3];
          const currentZ = positions.array[i * 3 + 2];
          const currentR = Math.sqrt(currentX * currentX + currentZ * currentZ);

          if (currentR < targetRadius * 0.9) {
            const expandFactor = 1.01;
            positions.array[i * 3] *= expandFactor;
            positions.array[i * 3 + 2] *= expandFactor;
          }

          // Update color (fade out)
          const fade = Math.max(0, 1 - t * 0.8);
          colors.array[i * 3] = 0.5 * fade;
          colors.array[i * 3 + 1] = 1.0 * fade;
          colors.array[i * 3 + 2] = 1.0 * fade;
        }

        // Reset when past cone end
        if (y > CONE_END_HEIGHT) {
          positions.array[i * 3 + 1] = CONE_START_HEIGHT;
          const angle = Math.random() * Math.PI * 2;
          const r = Math.random() * BEAM_RADIUS;
          positions.array[i * 3] = Math.cos(angle) * r;
          positions.array[i * 3 + 2] = Math.sin(angle) * r;
        }
      }

      positions.needsUpdate = true;
      colors.needsUpdate = true;
    }
  });

  return (
    <group>
      {/* === DATA SPHERE === */}
      {/* Outer sphere shell (transparent) */}
      <mesh position={[0, SPHERE_CENTER_Y, 0]}>
        <sphereGeometry args={[SPHERE_RADIUS, 64, 64]} />
        <meshBasicMaterial
          color="#00ffff"
          transparent
          opacity={0.08}
          side={THREE.BackSide}
        />
      </mesh>

      {/* Core light only - no visible sphere */}
      <pointLight position={[0, SPHERE_CENTER_Y, 0]} intensity={8} color="#00ffff" distance={100} decay={2} />

      {/* Sphere particles (contained inside) */}
      <points ref={sphereParticlesRef}>
        <bufferGeometry>
          <bufferAttribute
            attach="attributes-position"
            count={sphereParticleCount}
            array={spherePositions}
            itemSize={3}
          />
          <bufferAttribute
            attach="attributes-color"
            count={sphereParticleCount}
            array={sphereColors}
            itemSize={3}
          />
        </bufferGeometry>
        <pointsMaterial
          size={0.12}
          vertexColors
          transparent
          opacity={0.9}
          sizeAttenuation
          blending={THREE.AdditiveBlending}
        />
      </points>

      {/* === CYLINDRICAL BEAM (consistent width) === */}
      {/* Beam cylinder visual */}
      <mesh position={[0, (SPHERE_CENTER_Y + SPHERE_RADIUS + CONE_START_HEIGHT) / 2, 0]}>
        <cylinderGeometry
          args={[BEAM_RADIUS, BEAM_RADIUS, CONE_START_HEIGHT - SPHERE_CENTER_Y - SPHERE_RADIUS, 32, 1, true]}
        />
        <meshBasicMaterial
          color="#00ffff"
          transparent
          opacity={0.06}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* Beam particles */}
      <points ref={beamParticlesRef}>
        <bufferGeometry>
          <bufferAttribute
            attach="attributes-position"
            count={BEAM_PARTICLE_COUNT}
            array={beamPositions}
            itemSize={3}
          />
          <bufferAttribute
            attach="attributes-color"
            count={BEAM_PARTICLE_COUNT}
            array={beamColors}
            itemSize={3}
          />
        </bufferGeometry>
        <pointsMaterial
          size={0.15}
          vertexColors
          transparent
          opacity={0.85}
          sizeAttenuation
          blending={THREE.AdditiveBlending}
        />
      </points>

      {/* === CONE (expands after Command Center) === */}
      {/* Cone mesh */}
      <mesh position={[0, (CONE_START_HEIGHT + CONE_END_HEIGHT) / 2, 0]}>
        <cylinderGeometry
          args={[CONE_END_RADIUS, BEAM_RADIUS, CONE_END_HEIGHT - CONE_START_HEIGHT, 32, 1, true]}
        />
        <meshBasicMaterial
          color="#00ffff"
          transparent
          opacity={0.03}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* Cone particles */}
      <points ref={coneParticlesRef}>
        <bufferGeometry>
          <bufferAttribute
            attach="attributes-position"
            count={CONE_PARTICLE_COUNT}
            array={conePositions}
            itemSize={3}
          />
          <bufferAttribute
            attach="attributes-color"
            count={CONE_PARTICLE_COUNT}
            array={coneColors}
            itemSize={3}
          />
        </bufferGeometry>
        <pointsMaterial
          size={0.18}
          vertexColors
          transparent
          opacity={0.6}
          sizeAttenuation
          blending={THREE.AdditiveBlending}
        />
      </points>

      {/* === ACCENT LIGHTING === */}
      {/* Ground ring where sphere meets ground */}
      <mesh position={[0, 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[SPHERE_RADIUS * 0.7, SPHERE_RADIUS * 0.75, 64]} />
        <meshBasicMaterial
          color="#00ffff"
          transparent
          opacity={0.4}
          side={THREE.DoubleSide}
        />
      </mesh>

      {/* Upward spotlight from sphere */}
      <spotLight
        position={[0, SPHERE_CENTER_Y, 0]}
        angle={Math.PI / 12}
        penumbra={0.5}
        intensity={5}
        color="#00ffff"
        distance={200}
        target-position={[0, 150, 0]}
      />
    </group>
  );
}
