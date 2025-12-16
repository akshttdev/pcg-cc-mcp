import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text } from '@react-three/drei';
import * as THREE from 'three';

interface NoraAvatarProps {
  position?: [number, number, number];
}

export function NoraAvatar({ position = [0, 6, 0] }: NoraAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Mesh>(null);
  const bodyRef = useRef<THREE.Group>(null);
  const particlesRef = useRef<THREE.Points>(null);

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
    }

    // Gentle floating animation
    if (groupRef.current) {
      groupRef.current.position.y = position[1] + Math.sin(time * 0.5) * 0.3;
      groupRef.current.rotation.y = Math.sin(time * 0.2) * 0.1;
    }

    // Breathing effect (scale pulse)
    if (bodyRef.current) {
      const breathe = Math.sin(time * 0.8) * 0.03 + 1;
      bodyRef.current.scale.setScalar(breathe);
    }

    // Head slight tilt
    if (headRef.current) {
      headRef.current.rotation.x = Math.sin(time * 0.6) * 0.05;
      headRef.current.rotation.z = Math.cos(time * 0.7) * 0.03;
    }

    // Rotate particles
    if (particlesRef.current) {
      particlesRef.current.rotation.y += 0.002;
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

        {/* Arms */}
        <mesh position={[-0.9, 3.3, 0]} rotation={[0, 0, Math.PI / 8]} material={hologramMaterial}>
          <cylinderGeometry args={[0.15, 0.12, 1.8, 12]} />
        </mesh>
        <mesh position={[0.9, 3.3, 0]} rotation={[0, 0, -Math.PI / 8]} material={hologramMaterial}>
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

      {/* Name label */}
      <Text position={[0, 7, 0]} fontSize={0.5} color="#00ffff" anchorX="center" anchorY="middle">
        NORA
      </Text>

      {/* Ambient light from avatar */}
      <pointLight position={[0, 4, 0]} intensity={1.5} color="#00ffff" distance={15} decay={2} />
    </group>
  );
}
