import { useRef, useState } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

// Smoking animation timing constants (in seconds)
const PUFF_INTERVAL_MIN = 10;
const PUFF_INTERVAL_MAX = 15;
const RAISE_DURATION = 0.6;
const PUFF_DURATION = 1.2;
const LOWER_DURATION = 0.5;
const EXHALE_DURATION = 2.0;

type AnimationPhase = 'idle' | 'raising' | 'puffing' | 'lowering' | 'exhaling';

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

interface BluntEquipmentProps {
  armRef: React.RefObject<THREE.Group>;
  headRef: React.RefObject<THREE.Group>;
}

export function BluntEquipment({ armRef, headRef }: BluntEquipmentProps) {
  const cherryRef = useRef<THREE.Mesh>(null);
  const [isExhaling, setIsExhaling] = useState(false);

  // Animation state refs (to avoid re-renders)
  const phaseRef = useRef<AnimationPhase>('idle');
  const phaseStartTimeRef = useRef(0);
  const nextPuffTimeRef = useRef(
    Math.random() * (PUFF_INTERVAL_MAX - PUFF_INTERVAL_MIN) + PUFF_INTERVAL_MIN
  );

  // Store original arm rotation
  const originalArmRotation = useRef({ x: 0, z: 0 });

  useFrame((state) => {
    const time = state.clock.elapsedTime;

    // Check if it's time to start a puff
    if (phaseRef.current === 'idle' && time >= nextPuffTimeRef.current) {
      phaseRef.current = 'raising';
      phaseStartTimeRef.current = time;
      // Store current arm rotation
      if (armRef.current) {
        originalArmRotation.current = {
          x: armRef.current.rotation.x,
          z: armRef.current.rotation.z,
        };
      }
    }

    // Handle animation phases
    if (armRef.current) {
      const elapsed = time - phaseStartTimeRef.current;

      switch (phaseRef.current) {
        case 'raising': {
          if (elapsed < RAISE_DURATION) {
            const progress = easeInOutCubic(elapsed / RAISE_DURATION);
            // Rotate arm up and inward to bring blunt to mouth
            // Negative X rotation raises the arm forward/up
            // Positive Z rotation brings it toward the body
            armRef.current.rotation.x = originalArmRotation.current.x - Math.PI * 0.55 * progress;
            armRef.current.rotation.z = originalArmRotation.current.z + Math.PI * 0.15 * progress;
          } else {
            phaseRef.current = 'puffing';
            phaseStartTimeRef.current = time;
          }
          break;
        }

        case 'puffing': {
          // Keep arm at mouth position
          armRef.current.rotation.x = originalArmRotation.current.x - Math.PI * 0.55;
          armRef.current.rotation.z = originalArmRotation.current.z + Math.PI * 0.15;

          // Intensify cherry glow during puff
          if (cherryRef.current) {
            const material = cherryRef.current.material as THREE.MeshStandardMaterial;
            material.emissiveIntensity = 1.5 + Math.sin(time * 15) * 0.4;
          }

          if (elapsed >= PUFF_DURATION) {
            phaseRef.current = 'lowering';
            phaseStartTimeRef.current = time;
          }
          break;
        }

        case 'lowering': {
          if (elapsed < LOWER_DURATION) {
            const progress = easeInOutCubic(elapsed / LOWER_DURATION);
            // Return arm to original position
            armRef.current.rotation.x = originalArmRotation.current.x - Math.PI * 0.55 * (1 - progress);
            armRef.current.rotation.z = originalArmRotation.current.z + Math.PI * 0.15 * (1 - progress);
          } else {
            // Start exhaling
            phaseRef.current = 'exhaling';
            phaseStartTimeRef.current = time;
            setIsExhaling(true);
            // Reset arm
            armRef.current.rotation.x = originalArmRotation.current.x;
            armRef.current.rotation.z = originalArmRotation.current.z;
          }

          // Fade cherry back to normal
          if (cherryRef.current) {
            const material = cherryRef.current.material as THREE.MeshStandardMaterial;
            material.emissiveIntensity = 0.8;
          }
          break;
        }

        case 'exhaling': {
          if (elapsed >= EXHALE_DURATION) {
            phaseRef.current = 'idle';
            setIsExhaling(false);
            // Schedule next puff
            nextPuffTimeRef.current =
              time + PUFF_INTERVAL_MIN + Math.random() * (PUFF_INTERVAL_MAX - PUFF_INTERVAL_MIN);
          }
          break;
        }

        default: {
          // Idle state - arm controlled by walk animation
        }
      }
    }
  });

  return (
    <>
      {/* Blunt held in hand - positioned at hand location in arm group */}
      <group position={[0, -0.85, 0.12]} rotation={[Math.PI * 0.4, 0, 0]}>
        {/* Blunt body */}
        <mesh>
          <cylinderGeometry args={[0.035, 0.03, 0.35, 12]} />
          <meshStandardMaterial color="#5c4033" roughness={0.9} metalness={0} />
        </mesh>
        {/* Wrap lines */}
        <mesh position={[0, -0.06, 0]}>
          <cylinderGeometry args={[0.037, 0.037, 0.02, 12]} />
          <meshStandardMaterial color="#3d2817" roughness={1} metalness={0} />
        </mesh>
        <mesh position={[0, -0.12, 0]}>
          <cylinderGeometry args={[0.032, 0.032, 0.018, 12]} />
          <meshStandardMaterial color="#3d2817" roughness={1} metalness={0} />
        </mesh>
        {/* Cherry/lit end */}
        <mesh ref={cherryRef} position={[0, 0.18, 0]}>
          <sphereGeometry args={[0.038, 12, 12]} />
          <meshStandardMaterial
            color="#ff4500"
            emissive="#ff2200"
            emissiveIntensity={0.8}
            roughness={0.5}
          />
        </mesh>
        {/* Ash tip */}
        <mesh position={[0, 0.21, 0]}>
          <cylinderGeometry args={[0.015, 0.032, 0.04, 8]} />
          <meshStandardMaterial color="#606060" roughness={1} metalness={0} />
        </mesh>
        {/* Ambient smoke wisp from cherry */}
        <mesh position={[0, 0.26, 0]}>
          <sphereGeometry args={[0.02, 8, 8]} />
          <meshStandardMaterial color="#aaaaaa" transparent opacity={0.25} />
        </mesh>
      </group>

      {/* Exhale smoke from mouth area - positioned relative to head */}
      {isExhaling && headRef.current && (
        <group position={[
          headRef.current.position.x + 0.65, // Offset from arm to head (arm is at -0.65 from body center)
          headRef.current.position.y - 1.5 + 0.9, // Adjust for arm position offset, mouth height
          0.5 // In front of face
        ]}>
          <ExhaleSmokeCloud />
        </group>
      )}
    </>
  );
}

// Exhale smoke cloud component
function ExhaleSmokeCloud() {
  const groupRef = useRef<THREE.Group>(null);
  const startTime = useRef(0);

  useFrame((state) => {
    if (startTime.current === 0) {
      startTime.current = state.clock.elapsedTime;
    }

    const elapsed = state.clock.elapsedTime - startTime.current;

    if (groupRef.current) {
      // Expand and rise
      const scale = 1 + elapsed * 0.8;
      groupRef.current.scale.setScalar(scale);
      groupRef.current.position.y = elapsed * 0.15;
      groupRef.current.position.z = elapsed * 0.1;

      // Fade out
      groupRef.current.children.forEach((child) => {
        if (child instanceof THREE.Mesh && child.material instanceof THREE.MeshStandardMaterial) {
          child.material.opacity = Math.max(0, 0.5 - elapsed * 0.25);
        }
      });
    }
  });

  return (
    <group ref={groupRef}>
      <mesh position={[0, 0, 0]}>
        <sphereGeometry args={[0.08, 8, 8]} />
        <meshStandardMaterial color="#cccccc" transparent opacity={0.5} />
      </mesh>
      <mesh position={[0.05, 0.03, 0.02]}>
        <sphereGeometry args={[0.06, 8, 8]} />
        <meshStandardMaterial color="#bbbbbb" transparent opacity={0.4} />
      </mesh>
      <mesh position={[-0.04, 0.05, 0.03]}>
        <sphereGeometry args={[0.07, 8, 8]} />
        <meshStandardMaterial color="#dddddd" transparent opacity={0.35} />
      </mesh>
      <mesh position={[0.02, 0.08, 0.05]}>
        <sphereGeometry args={[0.05, 8, 8]} />
        <meshStandardMaterial color="#cccccc" transparent opacity={0.3} />
      </mesh>
    </group>
  );
}
