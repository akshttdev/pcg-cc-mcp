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
  const leftArmRef = useRef<THREE.Mesh>(null);
  const rightArmRef = useRef<THREE.Mesh>(null);

  // Hologram shader material
  const hologramMaterial = useMemo(() => {
    return new THREE.ShaderMaterial({
      uniforms: {
        time: { value: 0 },
        scanlineSpeed: { value: 2.0 },
        opacity: { value: 0.7 },
        color: { value: new THREE.Color(0x00ffff) },
      },
      vertexShader: `
        varying vec2 vUv;
        varying vec3 vNormal;
        varying vec3 vPosition;

        void main() {
          vUv = uv;
          vNormal = normalize(normalMatrix * normal);
          vPosition = (modelMatrix * vec4(position, 1.0)).xyz;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: `
        uniform float time;
        uniform float scanlineSpeed;
        uniform float opacity;
        uniform vec3 color;

        varying vec2 vUv;
        varying vec3 vNormal;
        varying vec3 vPosition;

        void main() {
          // Scanline effect
          float scanline = sin(vPosition.y * 10.0 - time * scanlineSpeed) * 0.5 + 0.5;

          // Fresnel effect (edge glow)
          vec3 viewDirection = normalize(cameraPosition - vPosition);
          float fresnel = pow(1.0 - abs(dot(viewDirection, vNormal)), 2.0);

          // Flickering
          float flicker = sin(time * 3.0) * 0.05 + 0.95;

          // Vertical fade
          float fade = smoothstep(0.0, 0.2, vUv.y) * smoothstep(1.0, 0.8, vUv.y);

          // Combine effects
          float alpha = (scanline * 0.3 + fresnel * 0.7) * opacity * flicker * fade;
          vec3 finalColor = color * (1.0 + fresnel * 0.5);

          gl_FragColor = vec4(finalColor, alpha);
        }
      `,
      transparent: true,
      side: THREE.DoubleSide,
    });
  }, []);

  // Particle positions orbiting around avatar
  const particleCount = 200;
  const particlePositions = useMemo(() => {
    const positions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const radius = 2 + Math.random() * 2;

      positions[i * 3] = radius * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = radius * Math.cos(phi) + 3;
      positions[i * 3 + 2] = radius * Math.sin(phi) * Math.sin(theta);
    }
    return positions;
  }, []);

  useFrame((state) => {
    const time = state.clock.elapsedTime;

    // Update shader time
    if (hologramMaterial.uniforms) {
      hologramMaterial.uniforms.time.value = time;
      // Adjust opacity based on mood
      const moodOpacity = mood === 'alert' ? 0.9 : mood === 'happy' ? 0.85 : 0.7;
      hologramMaterial.uniforms.opacity.value = moodOpacity;
    }

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

    // Breathing effect (scale pulse) - more pronounced when speaking
    if (bodyRef.current) {
      const breatheSpeed = speaking ? 2.5 : 0.8;
      const breatheAmount = speaking ? 0.05 : 0.03;
      const breathe = Math.sin(time * breatheSpeed) * breatheAmount + 1;
      bodyRef.current.scale.setScalar(breathe);

      // Processing mode: spin body
      if (mood === 'processing') {
        bodyRef.current.rotation.y = time * 0.3;
      }
    }

    // Head animations based on mood
    if (headRef.current) {
      switch (mood) {
        case 'thinking':
          headRef.current.rotation.x = Math.sin(time * 0.6) * 0.15;
          headRef.current.rotation.z = 0.15; // Tilt to side
          break;
        case 'alert':
          headRef.current.rotation.x = 0; // Straight ahead
          headRef.current.rotation.z = Math.cos(time * 3) * 0.05; // Quick scanning
          break;
        case 'happy':
          headRef.current.rotation.x = Math.sin(time * 1.2) * 0.08; // Enthusiastic nod
          headRef.current.rotation.z = Math.cos(time * 0.7) * 0.05;
          break;
        case 'speaking':
          headRef.current.rotation.x = Math.sin(time * 1.5) * 0.08; // Nod while speaking
          headRef.current.rotation.z = Math.cos(time * 0.9) * 0.04;
          break;
        default:
          headRef.current.rotation.x = Math.sin(time * 0.6) * 0.05;
          headRef.current.rotation.z = Math.cos(time * 0.7) * 0.03;
      }
    }

    // Arm gestures based on mood
    if (leftArmRef.current && rightArmRef.current) {
      switch (mood) {
        case 'speaking':
          // Expressive hand movements
          leftArmRef.current.rotation.x = Math.sin(time * 2) * 0.3;
          rightArmRef.current.rotation.x = Math.sin(time * 2 + Math.PI) * 0.3;
          break;
        case 'thinking':
          // Hand near chin
          rightArmRef.current.rotation.x = -0.8;
          leftArmRef.current.rotation.x = Math.sin(time * 0.5) * 0.1;
          break;
        case 'alert':
          // Arms slightly raised
          leftArmRef.current.rotation.x = -0.3;
          rightArmRef.current.rotation.x = -0.3;
          break;
        case 'happy':
          // Arms open wide (welcoming)
          leftArmRef.current.rotation.z = Math.sin(time * 1.2) * 0.2 + 0.3;
          rightArmRef.current.rotation.z = Math.sin(time * 1.2) * 0.2 - 0.3;
          leftArmRef.current.rotation.x = -0.2;
          rightArmRef.current.rotation.x = -0.2;
          break;
        default:
          // Idle slight movement
          leftArmRef.current.rotation.x = Math.sin(time * 0.4) * 0.1;
          rightArmRef.current.rotation.x = Math.sin(time * 0.4 + Math.PI) * 0.1;
      }
    }

    // Particle speed based on mood
    if (particlesRef.current) {
      const particleSpeed = mood === 'processing' ? 0.008 : mood === 'thinking' ? 0.004 : 0.002;
      particlesRef.current.rotation.y += particleSpeed;
    }
  });

  return (
    <group ref={groupRef} position={position}>
      {/* Holographic projection base */}
      <mesh position={[0, 0, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[2.5, 3, 64]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.5} side={THREE.DoubleSide} />
      </mesh>

      {/* Base column/pedestal */}
      <mesh position={[0, 0.5, 0]}>
        <cylinderGeometry args={[2.5, 2.7, 1, 8]} />
        <meshStandardMaterial
          color="#0a2f4a"
          emissive="#00ffff"
          emissiveIntensity={0.3}
          metalness={0.9}
          roughness={0.1}
        />
      </mesh>

      {/* Body - humanoid form */}
      <group ref={bodyRef}>
        {/* Torso */}
        <mesh position={[0, 3.5, 0]} material={hologramMaterial}>
          <capsuleGeometry args={[0.6, 1.5, 16, 32]} />
        </mesh>

        {/* Head */}
        <mesh ref={headRef} position={[0, 5.5, 0]} material={hologramMaterial}>
          <sphereGeometry args={[0.5, 32, 32]} />
        </mesh>

        {/* Face elements */}
        {/* Eyes */}
        <mesh position={[-0.2, 5.6, 0.4]}>
          <sphereGeometry args={[0.08, 16, 16]} />
          <meshBasicMaterial color="#ffffff" />
        </mesh>
        <mesh position={[0.2, 5.6, 0.4]}>
          <sphereGeometry args={[0.08, 16, 16]} />
          <meshBasicMaterial color="#ffffff" />
        </mesh>

        {/* Core glow (heart) */}
        <mesh position={[0, 3.8, 0]}>
          <sphereGeometry args={[0.3, 16, 16]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={0.8} />
          <pointLight intensity={1} color="#00ffff" distance={5} />
        </mesh>

        {/* Arms with animation refs */}
        <mesh
          ref={leftArmRef}
          position={[-0.9, 3.3, 0]}
          rotation={[0, 0, Math.PI / 8]}
          material={hologramMaterial}
        >
          <cylinderGeometry args={[0.15, 0.12, 1.8, 12]} />
        </mesh>
        <mesh
          ref={rightArmRef}
          position={[0.9, 3.3, 0]}
          rotation={[0, 0, -Math.PI / 8]}
          material={hologramMaterial}
        >
          <cylinderGeometry args={[0.15, 0.12, 1.8, 12]} />
        </mesh>

        {/* Legs */}
        <mesh position={[-0.3, 1.5, 0]} rotation={[0, 0, Math.PI / 32]} material={hologramMaterial}>
          <cylinderGeometry args={[0.18, 0.15, 2.2, 12]} />
        </mesh>
        <mesh position={[0.3, 1.5, 0]} rotation={[0, 0, -Math.PI / 32]} material={hologramMaterial}>
          <cylinderGeometry args={[0.18, 0.15, 2.2, 12]} />
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
          size={0.08}
          color="#00ffff"
          transparent
          opacity={0.7}
          sizeAttenuation
          blending={THREE.AdditiveBlending}
        />
      </points>

      {/* Energy streams */}
      {[0, 1, 2, 3].map((i) => {
        const angle = (i / 4) * Math.PI * 2;
        return (
          <mesh
            key={i}
            position={[Math.cos(angle) * 0.8, 3.5, Math.sin(angle) * 0.8]}
            rotation={[0, angle, 0]}
          >
            <cylinderGeometry args={[0.02, 0.02, 3, 8]} />
            <meshBasicMaterial color="#00ffff" transparent opacity={0.4} />
          </mesh>
        );
      })}

      {/* Name label with mood indicator */}
      <Text position={[0, 7, 0]} fontSize={0.5} color="#00ffff" anchorX="center" anchorY="middle">
        NORA
      </Text>
      {mood !== 'neutral' && (
        <Text
          position={[0, 6.5, 0]}
          fontSize={0.3}
          color={mood === 'alert' ? '#ff4444' : mood === 'happy' ? '#44ff44' : '#00ffff'}
          anchorX="center"
          anchorY="middle"
        >
          {mood.toUpperCase()}
        </Text>
      )}

      {/* Speaking visualization (waveform ring) */}
      {speaking && (
        <group position={[0, 5.5, 0]}>
          {[0, 1, 2, 3, 4, 5].map((i) => {
            const angle = (i / 6) * Math.PI * 2;
            return (
              <mesh
                key={i}
                position={[Math.cos(angle) * 0.8, 0, Math.sin(angle) * 0.8]}
                rotation={[0, angle, 0]}
              >
                <boxGeometry args={[0.05, 0.15, 0.05]} />
                <meshBasicMaterial color="#00ffff" transparent opacity={0.8} />
              </mesh>
            );
          })}
        </group>
      )}

      {/* Mood-based ambient light */}
      <pointLight
        position={[0, 4, 0]}
        intensity={mood === 'alert' ? 2.5 : mood === 'happy' ? 2.0 : 1.5}
        color={mood === 'alert' ? '#ff4444' : '#00ffff'}
        distance={mood === 'alert' ? 20 : 15}
        decay={2}
      />
    </group>
  );
}
