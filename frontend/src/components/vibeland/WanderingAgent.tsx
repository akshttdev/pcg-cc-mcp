import { useRef, useMemo, useEffect, useCallback } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';
import type { BayBounds } from './AgentWorkspaceLevel';

export type AgentRole = 'cinematographer' | 'editor' | 'browser' | 'oracle';

interface WanderingAgentProps {
  name: string;
  role: AgentRole;
  // Legacy props for command center ring wandering
  platformCenter?: [number, number, number];
  platformRadius?: number;
  startPosition?: [number, number, number];
  // New props for workspace bay wandering (wedge-shaped)
  bayBounds?: BayBounds;
}

const AGENT_COLORS: Record<AgentRole, { primary: string; accent: string; glow: string }> = {
  cinematographer: {
    primary: '#ff6b9d',   // Pink/magenta - artistic
    accent: '#ffd700',    // Gold - cinematic
    glow: '#ff69b4',
  },
  editor: {
    primary: '#4a9eff',   // Blue - technical
    accent: '#00ff88',    // Green - timeline/editing
    glow: '#4169e1',
  },
  browser: {
    primary: '#9b59b6',   // Purple - digital
    accent: '#00ffff',    // Cyan - web/data
    glow: '#8a2be2',
  },
  oracle: {
    primary: '#ffd700',   // Gold - wisdom/insight
    accent: '#ffffff',    // White - clarity
    glow: '#ffaa00',      // Warm gold glow
  },
};

export function WanderingAgent({
  name,
  role,
  platformCenter,
  platformRadius,
  startPosition,
  bayBounds,
}: WanderingAgentProps) {
  const groupRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Mesh>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const leftLegRef = useRef<THREE.Mesh>(null);
  const rightLegRef = useRef<THREE.Mesh>(null);

  // Movement state
  const velocityRef = useRef(new THREE.Vector3());
  const targetRef = useRef(new THREE.Vector3());
  const waitTimeRef = useRef(0);
  const rotationRef = useRef(0);

  const colors = AGENT_COLORS[role];

  // Helper to generate random position within wedge
  const getRandomWedgePosition = useCallback(() => {
    if (!bayBounds) return null;

    const angle = bayBounds.startAngle + Math.random() * (bayBounds.endAngle - bayBounds.startAngle);
    const radius = bayBounds.innerRadius + Math.random() * (bayBounds.outerRadius - bayBounds.innerRadius);

    return new THREE.Vector3(
      Math.cos(angle) * radius,
      bayBounds.centerY,
      Math.sin(angle) * radius
    );
  }, [bayBounds]);

  // Legacy ring position helpers (for command center)
  const minWanderRadius = platformCenter ? 14 : 0; // Nora exclusion
  const maxWanderRadius = platformRadius ? Math.max(minWanderRadius + 3, platformRadius - 1) : 0;

  const keepPositionInRing = useCallback((vector: THREE.Vector3) => {
    if (!platformCenter) return;
    const dx = vector.x - platformCenter[0];
    const dz = vector.z - platformCenter[2];
    let angle = Math.atan2(dz, dx);
    if (!Number.isFinite(angle)) {
      angle = Math.random() * Math.PI * 2;
    }
    let dist = Math.sqrt(dx * dx + dz * dz);
    if (!Number.isFinite(dist) || dist === 0) {
      dist = minWanderRadius;
    }
    dist = Math.min(Math.max(dist, minWanderRadius), maxWanderRadius);
    vector.x = platformCenter[0] + Math.cos(angle) * dist;
    vector.z = platformCenter[2] + Math.sin(angle) * dist;
    vector.y = platformCenter[1];
  }, [platformCenter, maxWanderRadius, minWanderRadius]);

  const assignNewTarget = useCallback(() => {
    // Use wedge bounds if available
    if (bayBounds) {
      const newPos = getRandomWedgePosition();
      if (newPos) {
        targetRef.current.copy(newPos);
      }
      return;
    }

    // Legacy: ring-based wandering
    if (platformCenter && platformRadius) {
      const angle = Math.random() * Math.PI * 2;
      const wanderBand = Math.max(0.1, maxWanderRadius - minWanderRadius);
      const dist = minWanderRadius + Math.random() * wanderBand;
      targetRef.current.set(
        platformCenter[0] + Math.cos(angle) * dist,
        platformCenter[1],
        platformCenter[2] + Math.sin(angle) * dist
      );
      keepPositionInRing(targetRef.current);
    }
  }, [bayBounds, getRandomWedgePosition, keepPositionInRing, maxWanderRadius, minWanderRadius, platformCenter, platformRadius]);

  const initialPosition = useMemo(() => {
    // Use wedge if available
    if (bayBounds) {
      const pos = getRandomWedgePosition();
      if (pos) return [pos.x, pos.y, pos.z] as [number, number, number];
    }

    // Use start position or random in ring
    if (startPosition) {
      const base = new THREE.Vector3(...startPosition);
      if (platformCenter) keepPositionInRing(base);
      return [base.x, base.y, base.z] as [number, number, number];
    }

    if (platformCenter && platformRadius) {
      const base = new THREE.Vector3(
        platformCenter[0] + (Math.random() - 0.5) * platformRadius * 0.6,
        platformCenter[1],
        platformCenter[2] + (Math.random() - 0.5) * platformRadius * 0.6
      );
      keepPositionInRing(base);
      return [base.x, base.y, base.z] as [number, number, number];
    }

    return [0, 66, 30] as [number, number, number];
  }, [bayBounds, getRandomWedgePosition, keepPositionInRing, platformCenter, platformRadius, startPosition]);

  useEffect(() => {
    assignNewTarget();
  }, [assignNewTarget]);

  useFrame((_, delta) => {
    if (!groupRef.current) return;

    const dt = Math.min(delta, 0.1);
    const pos = groupRef.current.position;

    // Decrease wait time
    if (waitTimeRef.current > 0) {
      waitTimeRef.current -= dt;

      // Idle animation while waiting
      if (leftArmRef.current && rightArmRef.current) {
        const idleTime = performance.now() * 0.001;
        leftArmRef.current.rotation.x = Math.sin(idleTime * 0.5) * 0.05;
        rightArmRef.current.rotation.x = Math.sin(idleTime * 0.5 + Math.PI) * 0.05;
      }
      return;
    }

    // Check if reached target
    const distToTarget = Math.sqrt(
      (pos.x - targetRef.current.x) ** 2 +
      (pos.z - targetRef.current.z) ** 2
    );

    if (distToTarget < 0.5) {
      // Pick new target and wait
      waitTimeRef.current = 1 + Math.random() * 3; // Wait 1-4 seconds
      assignNewTarget();
      return;
    }

    // Move toward target
    const dirX = targetRef.current.x - pos.x;
    const dirZ = targetRef.current.z - pos.z;
    const len = Math.sqrt(dirX * dirX + dirZ * dirZ);

    if (len === 0) {
      assignNewTarget();
      return;
    }

    const speed = 1.5; // Walking speed
    velocityRef.current.x = (dirX / len) * speed * dt;
    velocityRef.current.z = (dirZ / len) * speed * dt;

    const nextX = pos.x + velocityRef.current.x;
    const nextZ = pos.z + velocityRef.current.z;

    // Check bounds based on mode
    if (bayBounds) {
      // Check if still within wedge
      const nextDist = Math.sqrt(nextX * nextX + nextZ * nextZ);
      let nextAngle = Math.atan2(nextZ, nextX);
      if (nextAngle < 0) nextAngle += Math.PI * 2;

      const inRadialBounds = nextDist >= bayBounds.innerRadius && nextDist <= bayBounds.outerRadius;
      const inAngularBounds = nextAngle >= bayBounds.startAngle && nextAngle <= bayBounds.endAngle;

      if (!inRadialBounds || !inAngularBounds) {
        // Heading out of bounds, pick new target
        waitTimeRef.current = 0.1;
        assignNewTarget();
        return;
      }
    } else if (platformCenter) {
      // Legacy: check Nora's zone
      const nextDistFromCenter = Math.sqrt(
        (nextX - platformCenter[0]) ** 2 +
        (nextZ - platformCenter[2]) ** 2
      );
      if (nextDistFromCenter < 12) { // Nora exclusion radius
        waitTimeRef.current = 0.1;
        return;
      }
    }

    pos.x = nextX;
    pos.z = nextZ;

    // Rotate to face movement direction
    const targetRotation = Math.atan2(dirX, dirZ);
    const rotDiff = targetRotation - rotationRef.current;
    const normalizedDiff = Math.atan2(Math.sin(rotDiff), Math.cos(rotDiff));
    rotationRef.current += normalizedDiff * 0.1;
    groupRef.current.rotation.y = rotationRef.current;

    // Walking animation
    const walkCycle = performance.now() * 0.005;
    if (leftLegRef.current && rightLegRef.current) {
      leftLegRef.current.rotation.x = Math.sin(walkCycle) * 0.4;
      rightLegRef.current.rotation.x = Math.sin(walkCycle + Math.PI) * 0.4;
    }
    if (leftArmRef.current && rightArmRef.current) {
      leftArmRef.current.rotation.x = Math.sin(walkCycle + Math.PI) * 0.3;
      rightArmRef.current.rotation.x = Math.sin(walkCycle) * 0.3;
    }
  });

  return (
    <group ref={groupRef} position={initialPosition}>
      {/* Body */}
      <mesh position={[0, 1.0, 0]}>
        <capsuleGeometry args={[0.25, 0.6, 8, 16]} />
        <meshStandardMaterial color={colors.primary} emissive={colors.primary} emissiveIntensity={0.3} />
      </mesh>

      {/* Head */}
      <mesh ref={headRef} position={[0, 1.7, 0]}>
        <sphereGeometry args={[0.22, 16, 16]} />
        <meshStandardMaterial color={colors.primary} emissive={colors.primary} emissiveIntensity={0.3} />
      </mesh>

      {/* Eyes */}
      <mesh position={[-0.08, 1.75, 0.18]}>
        <sphereGeometry args={[0.05, 8, 8]} />
        <meshBasicMaterial color={colors.accent} />
      </mesh>
      <mesh position={[0.08, 1.75, 0.18]}>
        <sphereGeometry args={[0.05, 8, 8]} />
        <meshBasicMaterial color={colors.accent} />
      </mesh>

      {/* Left Arm */}
      <group ref={leftArmRef} position={[-0.35, 1.2, 0]}>
        <mesh position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.08, 0.4, 6, 8]} />
          <meshStandardMaterial color={colors.primary} emissive={colors.primary} emissiveIntensity={0.2} />
        </mesh>
      </group>

      {/* Right Arm */}
      <group ref={rightArmRef} position={[0.35, 1.2, 0]}>
        <mesh position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.08, 0.4, 6, 8]} />
          <meshStandardMaterial color={colors.primary} emissive={colors.primary} emissiveIntensity={0.2} />
        </mesh>
      </group>

      {/* Left Leg */}
      <mesh ref={leftLegRef} position={[-0.12, 0.35, 0]}>
        <capsuleGeometry args={[0.1, 0.45, 6, 8]} />
        <meshStandardMaterial color={colors.primary} emissive={colors.primary} emissiveIntensity={0.2} />
      </mesh>

      {/* Right Leg */}
      <mesh ref={rightLegRef} position={[0.12, 0.35, 0]}>
        <capsuleGeometry args={[0.1, 0.45, 6, 8]} />
        <meshStandardMaterial color={colors.primary} emissive={colors.primary} emissiveIntensity={0.2} />
      </mesh>

      {/* Glow effect */}
      <pointLight color={colors.glow} intensity={2} distance={4} />

      {/* Name label - floating above head */}
      <Text
        position={[0, 2.3, 0]}
        fontSize={0.3}
        color={colors.accent}
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.02}
        outlineColor="#000000"
      >
        {name}
      </Text>
    </group>
  );
}
