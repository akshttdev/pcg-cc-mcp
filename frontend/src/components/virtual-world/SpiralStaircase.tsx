import { useMemo } from 'react';
import * as THREE from 'three';
import {
  COMMAND_CENTER_Y,
  WORKSPACE_FLOOR_Y,
  STAIRWELL_START_ANGLE,
  STAIRWELL_END_ANGLE,
  STAIRWELL_WIDTH,
  STAIRWELL_INNER_RADIUS,
  STAIRWELL_OUTER_RADIUS,
  HOLOGRAM_RADIUS,
  HOLOGRAM_RAILING_RADIUS,
} from '@/lib/virtual-world/spatialSystem';

// Staircase visual configuration
const STAIR_COUNT = 32;
const STAIR_HEIGHT = 0.5;
const RAILING_HEIGHT = 1.2;
const RAILING_THICKNESS = 0.08;

// Heights
const TOP_Y = COMMAND_CENTER_Y;
const BOTTOM_Y = WORKSPACE_FLOOR_Y;
const HEIGHT_DIFF = TOP_Y - BOTTOM_Y;
const ROTATION_AMOUNT = STAIRWELL_END_ANGLE - STAIRWELL_START_ANGLE;

// Stair center radius and depth
const STAIR_RADIUS = (STAIRWELL_INNER_RADIUS + STAIRWELL_OUTER_RADIUS) / 2;  // = 12
const STAIR_DEPTH = STAIRWELL_OUTER_RADIUS - STAIRWELL_INNER_RADIUS;  // = 4

export function SpiralStaircase() {
  // Individual stair steps
  const steps = useMemo(() => {
    const stepGeometries: JSX.Element[] = [];

    for (let i = 0; i < STAIR_COUNT; i++) {
      const t = i / STAIR_COUNT;
      const angle = STAIRWELL_START_ANGLE + t * ROTATION_AMOUNT;
      const y = TOP_Y - t * HEIGHT_DIFF;

      const x = Math.cos(angle) * STAIR_RADIUS;
      const z = Math.sin(angle) * STAIR_RADIUS;

      const tangentAngle = angle + Math.PI / 2;

      stepGeometries.push(
        <group key={`step-${i}`} position={[x, y - STAIR_HEIGHT / 2, z]} rotation={[0, -tangentAngle, 0]}>
          <mesh castShadow receiveShadow>
            <boxGeometry args={[STAIRWELL_WIDTH, STAIR_HEIGHT, STAIR_DEPTH]} />
            <meshStandardMaterial
              color="#1a2a3a"
              metalness={0.7}
              roughness={0.3}
              emissive="#004060"
              emissiveIntensity={0.05}
            />
          </mesh>
          {/* Step edge glow */}
          <mesh position={[0, STAIR_HEIGHT / 2 + 0.02, STAIR_DEPTH / 2 - 0.1]}>
            <boxGeometry args={[STAIRWELL_WIDTH - 0.2, 0.05, 0.1]} />
            <meshBasicMaterial color="#00ffff" transparent opacity={0.6} />
          </mesh>
        </group>
      );
    }

    return stepGeometries;
  }, []);

  // Railing posts on BOTH sides of stairs
  const railingPosts = useMemo(() => {
    const posts: JSX.Element[] = [];
    const postInterval = 3;

    for (let i = 0; i <= STAIR_COUNT; i += postInterval) {
      const t = i / STAIR_COUNT;
      const angle = STAIRWELL_START_ANGLE + t * ROTATION_AMOUNT;
      const y = TOP_Y - t * HEIGHT_DIFF;

      // Inner railing post
      const innerX = Math.cos(angle) * STAIRWELL_INNER_RADIUS;
      const innerZ = Math.sin(angle) * STAIRWELL_INNER_RADIUS;
      posts.push(
        <mesh key={`inner-post-${i}`} position={[innerX, y + RAILING_HEIGHT / 2, innerZ]} castShadow>
          <cylinderGeometry args={[RAILING_THICKNESS, RAILING_THICKNESS, RAILING_HEIGHT, 8]} />
          <meshStandardMaterial color="#2a3a4a" metalness={0.8} roughness={0.2} />
        </mesh>
      );

      // Outer railing post
      const outerX = Math.cos(angle) * STAIRWELL_OUTER_RADIUS;
      const outerZ = Math.sin(angle) * STAIRWELL_OUTER_RADIUS;
      posts.push(
        <mesh key={`outer-post-${i}`} position={[outerX, y + RAILING_HEIGHT / 2, outerZ]} castShadow>
          <cylinderGeometry args={[RAILING_THICKNESS, RAILING_THICKNESS, RAILING_HEIGHT, 8]} />
          <meshStandardMaterial color="#2a3a4a" metalness={0.8} roughness={0.2} />
        </mesh>
      );
    }

    return posts;
  }, []);

  // Railing tubes on BOTH sides
  const railingTubes = useMemo(() => {
    const segments = 60;
    const innerPoints: THREE.Vector3[] = [];
    const outerPoints: THREE.Vector3[] = [];

    for (let i = 0; i <= segments; i++) {
      const t = i / segments;
      const angle = STAIRWELL_START_ANGLE + t * ROTATION_AMOUNT;
      const y = TOP_Y - t * HEIGHT_DIFF + RAILING_HEIGHT;

      innerPoints.push(new THREE.Vector3(
        Math.cos(angle) * STAIRWELL_INNER_RADIUS,
        y,
        Math.sin(angle) * STAIRWELL_INNER_RADIUS
      ));

      outerPoints.push(new THREE.Vector3(
        Math.cos(angle) * STAIRWELL_OUTER_RADIUS,
        y,
        Math.sin(angle) * STAIRWELL_OUTER_RADIUS
      ));
    }

    const innerCurve = new THREE.CatmullRomCurve3(innerPoints);
    const outerCurve = new THREE.CatmullRomCurve3(outerPoints);

    return (
      <>
        <mesh>
          <tubeGeometry args={[innerCurve, 64, 0.06, 8, false]} />
          <meshStandardMaterial color="#00ffff" emissive="#00ffff" emissiveIntensity={0.3} metalness={0.9} roughness={0.1} />
        </mesh>
        <mesh>
          <tubeGeometry args={[outerCurve, 64, 0.06, 8, false]} />
          <meshStandardMaterial color="#00ffff" emissive="#00ffff" emissiveIntensity={0.3} metalness={0.9} roughness={0.1} />
        </mesh>
      </>
    );
  }, []);

  // Landing platforms
  const landings = useMemo(() => {
    const topAngle = STAIRWELL_START_ANGLE;
    const topX = Math.cos(topAngle) * STAIR_RADIUS;
    const topZ = Math.sin(topAngle) * STAIR_RADIUS;

    const bottomAngle = STAIRWELL_END_ANGLE;
    const bottomX = Math.cos(bottomAngle) * STAIR_RADIUS;
    const bottomZ = Math.sin(bottomAngle) * STAIR_RADIUS;

    return (
      <>
        <mesh position={[topX, TOP_Y - 0.15, topZ]} rotation={[0, -topAngle - Math.PI / 2, 0]} receiveShadow>
          <boxGeometry args={[STAIRWELL_WIDTH + 1, 0.3, STAIR_DEPTH + 1]} />
          <meshStandardMaterial color="#1a2a3a" metalness={0.7} roughness={0.3} />
        </mesh>
        <mesh position={[bottomX, BOTTOM_Y - 0.15, bottomZ]} rotation={[0, -bottomAngle - Math.PI / 2, 0]} receiveShadow>
          <boxGeometry args={[STAIRWELL_WIDTH + 1, 0.3, STAIR_DEPTH + 1]} />
          <meshStandardMaterial color="#1a2a3a" metalness={0.7} roughness={0.3} />
        </mesh>
      </>
    );
  }, []);

  // 360° RAILING AROUND HOLOGRAM - separate railings at each level (not full height)
  const hologramRailing = useMemo(() => {
    const postCount = 32;
    const floorPosts: JSX.Element[] = [];
    const commandPosts: JSX.Element[] = [];

    // Floor level railing posts (Y=65) - continuous around hologram
    for (let i = 0; i < postCount; i++) {
      const angle = (i / postCount) * Math.PI * 2;
      const x = Math.cos(angle) * HOLOGRAM_RAILING_RADIUS;
      const z = Math.sin(angle) * HOLOGRAM_RAILING_RADIUS;

      floorPosts.push(
        <mesh key={`floor-post-${i}`} position={[x, BOTTOM_Y + RAILING_HEIGHT / 2, z]} castShadow>
          <cylinderGeometry args={[RAILING_THICKNESS, RAILING_THICKNESS, RAILING_HEIGHT, 8]} />
          <meshStandardMaterial color="#2a3a4a" metalness={0.8} roughness={0.2} />
        </mesh>
      );
    }

    // Command center level railing posts (Y=80)
    for (let i = 0; i < postCount; i++) {
      const angle = (i / postCount) * Math.PI * 2;
      const x = Math.cos(angle) * HOLOGRAM_RAILING_RADIUS;
      const z = Math.sin(angle) * HOLOGRAM_RAILING_RADIUS;

      // No gap - railing continues all the way around hologram at command center level
      commandPosts.push(
        <mesh key={`cmd-post-${i}`} position={[x, TOP_Y + RAILING_HEIGHT / 2, z]} castShadow>
          <cylinderGeometry args={[RAILING_THICKNESS, RAILING_THICKNESS, RAILING_HEIGHT, 8]} />
          <meshStandardMaterial color="#2a3a4a" metalness={0.8} roughness={0.2} />
        </mesh>
      );
    }

    // Floor level rail tube - continuous circle around hologram
    const floorRailPoints: THREE.Vector3[] = [];
    for (let i = 0; i <= 64; i++) {
      const angle = (i / 64) * Math.PI * 2;
      floorRailPoints.push(new THREE.Vector3(
        Math.cos(angle) * HOLOGRAM_RAILING_RADIUS,
        BOTTOM_Y + RAILING_HEIGHT,
        Math.sin(angle) * HOLOGRAM_RAILING_RADIUS
      ));
    }
    const floorRailCurve = new THREE.CatmullRomCurve3(floorRailPoints, true);  // closed loop

    // Command center level rail tube (full circle, no gap)
    const cmdRailPoints: THREE.Vector3[] = [];
    for (let i = 0; i <= 64; i++) {
      const angle = (i / 64) * Math.PI * 2;
      cmdRailPoints.push(new THREE.Vector3(
        Math.cos(angle) * HOLOGRAM_RAILING_RADIUS,
        TOP_Y + RAILING_HEIGHT,
        Math.sin(angle) * HOLOGRAM_RAILING_RADIUS
      ));
    }
    const cmdRailCurve = new THREE.CatmullRomCurve3(cmdRailPoints, true);  // closed loop

    return (
      <group>
        {/* Floor level railing */}
        {floorPosts}
        <mesh>
          <tubeGeometry args={[floorRailCurve, 60, 0.06, 8, false]} />
          <meshStandardMaterial color="#00ffff" emissive="#00ffff" emissiveIntensity={0.3} metalness={0.9} roughness={0.1} />
        </mesh>

        {/* Command center level railing */}
        {commandPosts}
        <mesh>
          <tubeGeometry args={[cmdRailCurve, 60, 0.06, 8, false]} />
          <meshStandardMaterial color="#00ffff" emissive="#00ffff" emissiveIntensity={0.3} metalness={0.9} roughness={0.1} />
        </mesh>

        {/* Base strip at floor level */}
        <mesh position={[0, BOTTOM_Y + 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[HOLOGRAM_RAILING_RADIUS - 0.3, HOLOGRAM_RAILING_RADIUS + 0.3, 64]} />
          <meshStandardMaterial color="#2a2a3a" metalness={0.6} roughness={0.4} />
        </mesh>
      </group>
    );
  }, []);

  // Hologram visual effect (no solid geometry - just glow effects)
  const hologramEffect = useMemo(() => {
    return (
      <group>
        {/* Hologram glow cylinder - transparent, no collision */}
        <mesh position={[0, (TOP_Y + BOTTOM_Y) / 2, 0]}>
          <cylinderGeometry args={[HOLOGRAM_RADIUS - 1, HOLOGRAM_RADIUS - 1, HEIGHT_DIFF, 32, 1, true]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={0.08} side={THREE.DoubleSide} />
        </mesh>

        {/* Inner core glow */}
        <mesh position={[0, (TOP_Y + BOTTOM_Y) / 2, 0]}>
          <cylinderGeometry args={[2, 2, HEIGHT_DIFF * 0.9, 32, 1, true]} />
          <meshBasicMaterial color="#00aaff" transparent opacity={0.12} side={THREE.DoubleSide} />
        </mesh>

        {/* Ring accents */}
        {[0.2, 0.4, 0.6, 0.8].map((t, i) => (
          <mesh key={`ring-${i}`} position={[0, BOTTOM_Y + t * HEIGHT_DIFF, 0]} rotation={[Math.PI / 2, 0, 0]}>
            <torusGeometry args={[HOLOGRAM_RADIUS - 1, 0.05, 8, 32]} />
            <meshBasicMaterial color="#00ffff" transparent opacity={0.4} />
          </mesh>
        ))}

        {/* Central light */}
        <pointLight position={[0, (TOP_Y + BOTTOM_Y) / 2, 0]} color="#00ffff" intensity={1} distance={30} />
      </group>
    );
  }, []);

  return (
    <group>
      {hologramEffect}
      {hologramRailing}
      {steps}
      {railingPosts}
      {railingTubes}
      {landings}

      {/* Ambient lighting */}
      <pointLight
        position={[
          Math.cos(STAIRWELL_START_ANGLE + ROTATION_AMOUNT / 3) * STAIR_RADIUS,
          (TOP_Y + BOTTOM_Y) / 2 + 3,
          Math.sin(STAIRWELL_START_ANGLE + ROTATION_AMOUNT / 3) * STAIR_RADIUS
        ]}
        color="#00ffff"
        intensity={0.5}
        distance={20}
      />
      <pointLight
        position={[
          Math.cos(STAIRWELL_START_ANGLE + ROTATION_AMOUNT * 2 / 3) * STAIR_RADIUS,
          (TOP_Y + BOTTOM_Y) / 2 - 3,
          Math.sin(STAIRWELL_START_ANGLE + ROTATION_AMOUNT * 2 / 3) * STAIR_RADIUS
        ]}
        color="#00ffff"
        intensity={0.5}
        distance={20}
      />
    </group>
  );
}
