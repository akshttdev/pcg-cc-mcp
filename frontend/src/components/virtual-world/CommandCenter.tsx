import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import {
  STAIRWELL_START_ANGLE,
  STAIRWELL_OUTER_RADIUS,
  WORKSPACE_OUTER_RADIUS,
  HOLOGRAM_RAILING_RADIUS,
} from '@/lib/virtual-world/spatialSystem';

const FLOOR_ELEVATION = 80;
const SPIRE_HEIGHT = 20;

// Hologram hole - matches the floor below
const HOLOGRAM_HOLE_RADIUS = HOLOGRAM_RAILING_RADIUS - 0.5; // R = 9.5

export function CommandCenter() {
  const spireRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (spireRef.current) {
      const pulse = Math.sin(state.clock.elapsedTime * 2) * 0.1 + 0.9;
      const material = spireRef.current.material as THREE.MeshStandardMaterial;
      material.emissiveIntensity = pulse * 1.6;
    }
  });

  // Floor geometry - solid circle with hologram hole and stair hole
  const floorGeometry = useMemo(() => {
    const shape = new THREE.Shape();
    const outerR = WORKSPACE_OUTER_RADIUS - 0.5;

    // Main floor circle
    shape.absarc(0, 0, outerR, 0, Math.PI * 2, false);

    // Hologram hole in center (matching the floor below)
    const hologramHole = new THREE.Path();
    hologramHole.absarc(0, 0, HOLOGRAM_HOLE_RADIUS, 0, Math.PI * 2, true);
    shape.holes.push(hologramHole);

    // Stair hole - arc shape where stairs arrive at command center
    // NOTE: Shape coordinates are flipped when rotated to world coords (Y becomes -Z)
    // So we negate the angle to place the hole at the correct world position
    const holeInnerR = HOLOGRAM_HOLE_RADIUS + 0.5; // R = 10 (lines up with inner railing)
    const holeOuterR = STAIRWELL_OUTER_RADIUS;      // R = 14 (lines up with outer railing)
    // Negate angles: shape angle -π/2 appears at world South (+Z)
    // Hole from 6:00 (S) to 9:00 (W) - full quarter arc
    const holeStartAngle = -STAIRWELL_START_ANGLE + 0.1; // Just past 6:00 (towards E)
    const holeEndAngle = -Math.PI;  // 9:00 position (West)

    const stairHole = new THREE.Path();
    stairHole.moveTo(
      Math.cos(holeStartAngle) * holeInnerR,
      Math.sin(holeStartAngle) * holeInnerR
    );
    // Arc directions swapped because start > end numerically
    stairHole.absarc(0, 0, holeInnerR, holeStartAngle, holeEndAngle, true);  // clockwise (short way)
    stairHole.lineTo(
      Math.cos(holeEndAngle) * holeOuterR,
      Math.sin(holeEndAngle) * holeOuterR
    );
    stairHole.absarc(0, 0, holeOuterR, holeEndAngle, holeStartAngle, false); // counterclockwise back
    stairHole.lineTo(
      Math.cos(holeStartAngle) * holeInnerR,
      Math.sin(holeStartAngle) * holeInnerR
    );

    shape.holes.push(stairHole);

    return new THREE.ExtrudeGeometry(shape, { depth: 0.5, bevelEnabled: false });
  }, []);

  // Compass radial lines
  const compassLines = useMemo(() => {
    const lines: JSX.Element[] = [];
    const innerR = HOLOGRAM_HOLE_RADIUS + 1;
    const outerR = WORKSPACE_OUTER_RADIUS - 5;

    // 8 main radial lines (N, NE, E, SE, S, SW, W, NW)
    for (let i = 0; i < 8; i++) {
      const angle = (i / 8) * Math.PI * 2;
      const isCardinal = i % 2 === 0;
      const lineWidth = isCardinal ? 0.3 : 0.15;
      const lineLength = outerR - innerR;
      const midR = (innerR + outerR) / 2;

      lines.push(
        <mesh
          key={`radial-${i}`}
          position={[
            Math.cos(angle) * midR,
            0.3,
            Math.sin(angle) * midR
          ]}
          rotation={[0, -angle + Math.PI / 2, 0]}
        >
          <boxGeometry args={[lineWidth, 0.1, lineLength]} />
          <meshStandardMaterial
            color="#00ffff"
            emissive="#00ffff"
            emissiveIntensity={0.3}
            transparent
            opacity={isCardinal ? 0.6 : 0.3}
          />
        </mesh>
      );
    }

    // Concentric rings
    const ringRadii = [15, 25, 35];
    ringRadii.forEach((r, idx) => {
      lines.push(
        <mesh key={`ring-${idx}`} position={[0, 0.25, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[r - 0.1, r + 0.1, 64]} />
          <meshStandardMaterial
            color="#00ffff"
            emissive="#00ffff"
            emissiveIntensity={0.2}
            transparent
            opacity={0.3}
          />
        </mesh>
      );
    });

    return lines;
  }, []);

  return (
    <group>
      {/* Floor - solid with hologram hole and stair hole */}
      <mesh geometry={floorGeometry} position={[0, FLOOR_ELEVATION - 0.25, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <meshStandardMaterial
          color="#0a1f35"
          metalness={0.8}
          roughness={0.35}
          emissive="#004080"
          emissiveIntensity={0.2}
        />
      </mesh>

      {/* Compass design on floor */}
      <group position={[0, FLOOR_ELEVATION, 0]}>
        {compassLines}

        {/* Cardinal direction markers - N is +Z, E is +X, S is -Z, W is -X */}
        <Text position={[0, 0.5, -38]} rotation={[-Math.PI / 2, 0, 0]} fontSize={3} color="#00ffff" anchorX="center" anchorY="middle">
          N
        </Text>
        <Text position={[0, 0.5, 38]} rotation={[-Math.PI / 2, 0, Math.PI]} fontSize={3} color="#00ffff" anchorX="center" anchorY="middle">
          S
        </Text>
        <Text position={[38, 0.5, 0]} rotation={[-Math.PI / 2, 0, -Math.PI / 2]} fontSize={3} color="#00ffff" anchorX="center" anchorY="middle">
          E
        </Text>
        <Text position={[-38, 0.5, 0]} rotation={[-Math.PI / 2, 0, Math.PI / 2]} fontSize={3} color="#00ffff" anchorX="center" anchorY="middle">
          W
        </Text>

        {/* Compass center marker */}
        <mesh position={[0, 0.3, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[HOLOGRAM_HOLE_RADIUS + 0.5, HOLOGRAM_HOLE_RADIUS + 1, 32]} />
          <meshStandardMaterial color="#00ffff" emissive="#00ffff" emissiveIntensity={0.5} />
        </mesh>
      </group>

      {/* Lights and signage */}
      <pointLight
        position={[0, FLOOR_ELEVATION + SPIRE_HEIGHT + 35, 0]}
        intensity={3}
        color="#00ffff"
        distance={160}
        decay={2}
      />

      <mesh ref={spireRef} position={[0, FLOOR_ELEVATION + SPIRE_HEIGHT + 35, 0]}>
        <sphereGeometry args={[3, 24, 24]} />
        <meshBasicMaterial color="#00ffff" />
      </mesh>

      <Text position={[0, FLOOR_ELEVATION + SPIRE_HEIGHT + 45, 0]} fontSize={3.5} color="#baf4ff" anchorX="center" anchorY="middle">
        PCG COMMAND CENTER
      </Text>

      <spotLight
        position={[0, FLOOR_ELEVATION + SPIRE_HEIGHT + 40, 0]}
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
