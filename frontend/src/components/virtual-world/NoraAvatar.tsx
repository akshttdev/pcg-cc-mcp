import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

export type NoraMood = 'neutral' | 'speaking' | 'thinking' | 'alert' | 'happy' | 'processing';

interface NoraAvatarProps {
  position?: [number, number, number];
  mood?: NoraMood;
  speaking?: boolean;
}

export function NoraAvatar({ position = [0, 6, 0], mood = 'neutral', speaking = false }: NoraAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Mesh>(null);
  const bodyRef = useRef<THREE.Group>(null);
  const particlesRef = useRef<THREE.Points>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const coreRef = useRef<THREE.Mesh>(null);

  // Particle positions orbiting around avatar
  const particleCount = 300;
  const particlePositions = useMemo(() => {
    const positions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const radius = 2.5 + Math.random() * 2.5;

      positions[i * 3] = radius * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = radius * Math.cos(phi) + 3;
      positions[i * 3 + 2] = radius * Math.sin(phi) * Math.sin(theta);
    }
    return positions;
  }, []);

  useFrame((state) => {
    const time = state.clock.elapsedTime;

    // Mood-based floating animation
    if (groupRef.current) {
      let floatAmount = 0.3;
      let floatSpeed = 0.5;
      if (mood === 'alert') {
        floatAmount = 0.5;
        floatSpeed = 1.2;
      } else if (mood === 'processing') {
        floatSpeed = 0.8;
      }
      groupRef.current.position.y = position[1] + Math.sin(time * floatSpeed) * floatAmount;
      groupRef.current.rotation.y = Math.sin(time * 0.2) * 0.1;
    }

    // Breathing effect
    if (bodyRef.current) {
      const breatheSpeed = speaking ? 2.5 : 0.8;
      const breatheAmount = speaking ? 0.05 : 0.03;
      const breathe = Math.sin(time * breatheSpeed) * breatheAmount + 1;
      bodyRef.current.scale.setScalar(breathe);

      if (mood === 'processing') {
        bodyRef.current.rotation.y = time * 0.3;
      }
    }

    // Core pulsing
    if (coreRef.current) {
      const pulse = Math.sin(time * 2) * 0.15 + 1;
      coreRef.current.scale.setScalar(pulse);
    }

    // Head animations based on mood
    if (headRef.current) {
      switch (mood) {
        case 'thinking':
          headRef.current.rotation.x = Math.sin(time * 0.6) * 0.15;
          headRef.current.rotation.z = 0.15;
          break;
        case 'alert':
          headRef.current.rotation.x = 0;
          headRef.current.rotation.z = Math.cos(time * 3) * 0.05;
          break;
        case 'happy':
          headRef.current.rotation.x = Math.sin(time * 1.2) * 0.08;
          headRef.current.rotation.z = Math.cos(time * 0.7) * 0.05;
          break;
        case 'speaking':
          headRef.current.rotation.x = Math.sin(time * 1.5) * 0.08;
          headRef.current.rotation.z = Math.cos(time * 0.9) * 0.04;
          break;
        default:
          headRef.current.rotation.x = Math.sin(time * 0.6) * 0.05;
          headRef.current.rotation.z = Math.cos(time * 0.7) * 0.03;
      }
    }

    // Arm gestures
    if (leftArmRef.current && rightArmRef.current) {
      switch (mood) {
        case 'speaking':
          leftArmRef.current.rotation.x = Math.sin(time * 2) * 0.3;
          rightArmRef.current.rotation.x = Math.sin(time * 2 + Math.PI) * 0.3;
          break;
        case 'thinking':
          rightArmRef.current.rotation.x = -0.8;
          leftArmRef.current.rotation.x = Math.sin(time * 0.5) * 0.1;
          break;
        case 'alert':
          leftArmRef.current.rotation.x = -0.3;
          rightArmRef.current.rotation.x = -0.3;
          break;
        case 'happy':
          leftArmRef.current.rotation.z = Math.sin(time * 1.2) * 0.2 + 0.3;
          rightArmRef.current.rotation.z = Math.sin(time * 1.2) * 0.2 - 0.3;
          leftArmRef.current.rotation.x = -0.2;
          rightArmRef.current.rotation.x = -0.2;
          break;
        default:
          leftArmRef.current.rotation.x = Math.sin(time * 0.4) * 0.1;
          rightArmRef.current.rotation.x = Math.sin(time * 0.4 + Math.PI) * 0.1;
      }
    }

    // Particle rotation
    if (particlesRef.current) {
      const particleSpeed = mood === 'processing' ? 0.008 : mood === 'thinking' ? 0.004 : 0.002;
      particlesRef.current.rotation.y += particleSpeed;
    }
  });

  const primaryColor = '#00ffff';
  const secondaryColor = '#0088ff';
  const accentColor = '#ffffff';

  return (
    <group ref={groupRef} position={position}>
      {/* Ground ring glow removed - Nora hovers in the hologram beam */}

      {/* Body group */}
      <group ref={bodyRef}>
        {/* Torso - solid glowing */}
        <mesh position={[0, 3.2, 0]}>
          <capsuleGeometry args={[0.55, 1.4, 16, 32]} />
          <meshBasicMaterial color={primaryColor} />
        </mesh>
        {/* Torso inner glow */}
        <mesh position={[0, 3.2, 0]}>
          <capsuleGeometry args={[0.58, 1.45, 16, 32]} />
          <meshBasicMaterial color={secondaryColor} transparent opacity={0.3} />
        </mesh>

        {/* Head */}
        <group position={[0, 5.2, 0]}>
          <mesh ref={headRef}>
            <sphereGeometry args={[0.55, 32, 32]} />
            <meshBasicMaterial color={primaryColor} />
          </mesh>
          {/* Head outer glow */}
          <mesh>
            <sphereGeometry args={[0.6, 32, 32]} />
            <meshBasicMaterial color={secondaryColor} transparent opacity={0.3} />
          </mesh>

          {/* Face - Eyes */}
          <mesh position={[-0.18, 0.1, 0.45]}>
            <sphereGeometry args={[0.12, 16, 16]} />
            <meshBasicMaterial color={accentColor} />
          </mesh>
          <mesh position={[0.18, 0.1, 0.45]}>
            <sphereGeometry args={[0.12, 16, 16]} />
            <meshBasicMaterial color={accentColor} />
          </mesh>
          {/* Eye pupils */}
          <mesh position={[-0.18, 0.1, 0.52]}>
            <sphereGeometry args={[0.05, 12, 12]} />
            <meshBasicMaterial color="#003366" />
          </mesh>
          <mesh position={[0.18, 0.1, 0.52]}>
            <sphereGeometry args={[0.05, 12, 12]} />
            <meshBasicMaterial color="#003366" />
          </mesh>
        </group>

        {/* Core glow (heart/chest) */}
        <mesh ref={coreRef} position={[0, 3.5, 0.3]}>
          <sphereGeometry args={[0.25, 16, 16]} />
          <meshBasicMaterial color={accentColor} />
        </mesh>
        <pointLight position={[0, 3.5, 0.3]} intensity={3} color={primaryColor} distance={10} />

        {/* Left Arm - hangs straight down, hand at bottom */}
        <group ref={leftArmRef} position={[-0.7, 3.9, 0]}>
          <mesh position={[-0.1, -0.7, 0]}>
            <capsuleGeometry args={[0.12, 1.0, 8, 12]} />
            <meshBasicMaterial color={primaryColor} />
          </mesh>
          {/* Hand - out to the side */}
          <mesh position={[-0.15, -1.4, 0]}>
            <sphereGeometry args={[0.15, 12, 12]} />
            <meshBasicMaterial color={secondaryColor} />
          </mesh>
        </group>

        {/* Right Arm - hangs straight down, hand at bottom */}
        <group ref={rightArmRef} position={[0.7, 3.9, 0]}>
          <mesh position={[0.1, -0.7, 0]}>
            <capsuleGeometry args={[0.12, 1.0, 8, 12]} />
            <meshBasicMaterial color={primaryColor} />
          </mesh>
          {/* Hand - out to the side */}
          <mesh position={[0.15, -1.4, 0]}>
            <sphereGeometry args={[0.15, 12, 12]} />
            <meshBasicMaterial color={secondaryColor} />
          </mesh>
        </group>

        {/* Legs */}
        <mesh position={[-0.25, 1.3, 0]}>
          <capsuleGeometry args={[0.15, 1.6, 8, 12]} />
          <meshBasicMaterial color={primaryColor} />
        </mesh>
        <mesh position={[0.25, 1.3, 0]}>
          <capsuleGeometry args={[0.15, 1.6, 8, 12]} />
          <meshBasicMaterial color={primaryColor} />
        </mesh>

        {/* Feet */}
        <mesh position={[-0.25, 0.35, 0.1]}>
          <boxGeometry args={[0.22, 0.12, 0.35]} />
          <meshBasicMaterial color={primaryColor} />
        </mesh>
        <mesh position={[0.25, 0.35, 0.1]}>
          <boxGeometry args={[0.22, 0.12, 0.35]} />
          <meshBasicMaterial color={primaryColor} />
        </mesh>
      </group>

      {/* Orbiting particles */}
      <points ref={particlesRef}>
        <bufferGeometry>
          <bufferAttribute
            attach="attributes-position"
            count={particleCount}
            array={particlePositions}
            itemSize={3}
          />
        </bufferGeometry>
        <pointsMaterial
          size={0.12}
          color={primaryColor}
          transparent
          opacity={0.8}
          sizeAttenuation
          blending={THREE.AdditiveBlending}
        />
      </points>

      {/* Vertical energy beams */}
      {[0, 1, 2, 3].map((i) => {
        const angle = (i / 4) * Math.PI * 2;
        const radius = 1.2;
        return (
          <mesh
            key={i}
            position={[Math.cos(angle) * radius, 2.8, Math.sin(angle) * radius]}
          >
            <cylinderGeometry args={[0.03, 0.03, 5, 8]} />
            <meshBasicMaterial color={primaryColor} transparent opacity={0.5} />
          </mesh>
        );
      })}

      {/* Horizontal ring at waist */}
      <mesh position={[0, 2.8, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[1.2, 0.03, 8, 32]} />
        <meshBasicMaterial color={primaryColor} />
      </mesh>

      {/* Name label */}
      <Text position={[0, 6.5, 0]} fontSize={0.6} color={accentColor} anchorX="center" anchorY="middle">
        NORA
      </Text>
      <Text position={[0, 6.0, 0]} fontSize={0.25} color={primaryColor} anchorX="center" anchorY="middle">
        Executive AI
      </Text>
      {mood !== 'neutral' && (
        <Text
          position={[0, 5.6, 0]}
          fontSize={0.2}
          color={mood === 'alert' ? '#ff4444' : mood === 'happy' ? '#44ff44' : primaryColor}
          anchorX="center"
          anchorY="middle"
        >
          [{mood.toUpperCase()}]
        </Text>
      )}

      {/* Speaking waveform */}
      {speaking && (
        <group position={[0, 5.2, 0.6]}>
          {[0, 1, 2, 3, 4].map((i) => (
            <mesh key={i} position={[(i - 2) * 0.15, 0, 0]}>
              <boxGeometry args={[0.06, 0.2, 0.04]} />
              <meshBasicMaterial color={accentColor} />
            </mesh>
          ))}
        </group>
      )}

      {/* Main ambient light */}
      <pointLight
        position={[0, 3.5, 0]}
        intensity={mood === 'alert' ? 4 : 2.5}
        color={mood === 'alert' ? '#ff4444' : primaryColor}
        distance={20}
        decay={2}
      />

      {/* Secondary light for visibility */}
      <pointLight
        position={[0, 5, 2]}
        intensity={1.5}
        color={primaryColor}
        distance={15}
      />
    </group>
  );
}
