import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface FireEmbersProps {
  bottomY: number;
  width: number;
}

export function FireCapeEquipment() {
  const capeGroupRef = useRef<THREE.Group>(null);

  const { capeGeometry, edgeGeometry, bottomExtent, width } = useMemo(() => {
    const topWidth = 0.42;
    const midWidth = 0.82;
    const bottomWidth = 1.05;
    const height = 1.45;
    const notchDepth = 0.2;

    const shape = new THREE.Shape();
    shape.moveTo(-topWidth / 2, 0);
    shape.lineTo(topWidth / 2, 0);
    shape.quadraticCurveTo(topWidth * 0.45, -0.18, midWidth / 2, -height * 0.35);
    shape.quadraticCurveTo(bottomWidth / 2 + 0.05, -height * 0.7, bottomWidth / 2, -height);
    shape.lineTo(bottomWidth / 2 - 0.18, -height - notchDepth * 0.1);
    shape.lineTo(0.18, -height - notchDepth);
    shape.lineTo(-0.02, -height - notchDepth * 1.35);
    shape.lineTo(-(bottomWidth / 2 - 0.22), -height - notchDepth * 0.25);
    shape.lineTo(-(bottomWidth / 2 + 0.06), -height * 0.7);
    shape.quadraticCurveTo(-(midWidth / 2), -height * 0.35, -topWidth / 2, 0);
    shape.closePath();

    const geometry = new THREE.ShapeGeometry(shape, 64);
    geometry.computeBoundingBox();
    const edges = new THREE.EdgesGeometry(geometry, 60);
    const bbox = geometry.boundingBox;
    const bottomY = bbox ? bbox.min.y : -height;
    const totalWidth = bbox ? bbox.max.x - bbox.min.x : bottomWidth;

    return { capeGeometry: geometry, edgeGeometry: edges, bottomExtent: bottomY, width: totalWidth };
  }, []);

  const fireCapeTexture = useMemo(() => {
    if (typeof document === 'undefined') return null;

    const size = 512;
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const gradient = ctx.createLinearGradient(0, 0, 0, size);
    gradient.addColorStop(0, '#fdd275');
    gradient.addColorStop(1, '#f0781d');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, size, size);

    const blobCount = 85;
    for (let i = 0; i < blobCount; i++) {
      const radius = (Math.random() * 0.12 + 0.05) * size;
      const x = (Math.random() * 0.7 + 0.15) * size;
      const y = (Math.random() * 0.8 + 0.1) * size;
      ctx.save();
      ctx.translate(x, y);
      ctx.rotate((Math.random() - 0.5) * Math.PI);
      ctx.scale(1, Math.random() * 0.5 + 0.6);
      const blobGradient = ctx.createRadialGradient(0, 0, radius * 0.15, 0, 0, radius);
      blobGradient.addColorStop(0, 'rgba(255, 160, 40, 0.9)');
      blobGradient.addColorStop(1, 'rgba(206, 69, 16, 0.8)');
      ctx.fillStyle = blobGradient;
      ctx.beginPath();
      ctx.ellipse(0, 0, radius, radius * 0.6, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    ctx.globalCompositeOperation = 'multiply';
    ctx.strokeStyle = 'rgba(130, 42, 6, 0.55)';
    ctx.lineWidth = 6;
    for (let i = 0; i < blobCount / 2; i++) {
      ctx.beginPath();
      ctx.moveTo(Math.random() * size, Math.random() * size);
      ctx.lineTo(Math.random() * size, Math.random() * size);
      ctx.stroke();
    }
    ctx.globalCompositeOperation = 'source-over';

    const texture = new THREE.CanvasTexture(canvas);
    texture.wrapS = THREE.ClampToEdgeWrapping;
    texture.wrapT = THREE.ClampToEdgeWrapping;
    texture.anisotropy = 4;
    texture.needsUpdate = true;
    return texture;
  }, []);

  useFrame(({ clock }) => {
    if (!capeGroupRef.current) return;
    const flutter = Math.sin(clock.elapsedTime * 1.8) * 0.04;
    capeGroupRef.current.rotation.x = 0.35 + flutter;
    capeGroupRef.current.rotation.z = Math.sin(clock.elapsedTime * 0.9) * 0.02;
    capeGroupRef.current.position.z = -0.02 - Math.cos(clock.elapsedTime * 1.4) * 0.015;
  });

  return (
    <group position={[0, 0.68, -0.28]}>
      {/* Hexagonal hook like the OSRS fire cape */}
      <mesh position={[0, 0.02, -0.03]} rotation={[Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.07, 0.12, 6]} />
        <meshStandardMaterial
          color="#4d2f00"
          metalness={0.85}
          roughness={0.25}
          emissive="#a66c1d"
          emissiveIntensity={0.2}
        />
      </mesh>

      {/* Cloth loop attaching hook to cape */}
      <mesh position={[0, -0.08, -0.02]}>
        <boxGeometry args={[0.08, 0.16, 0.04]} />
        <meshStandardMaterial
          color="#d9931a"
          metalness={0.4}
          roughness={0.6}
          emissive="#f9ae32"
          emissiveIntensity={0.25}
        />
      </mesh>

      <group ref={capeGroupRef} position={[0, -0.18, -0.04]}>
        <mesh geometry={capeGeometry} castShadow receiveShadow>
          <meshStandardMaterial
            color="#f59f2c"
            map={fireCapeTexture ?? undefined}
            emissive="#ff9e1a"
            emissiveIntensity={0.4}
            roughness={0.65}
            metalness={0.05}
            side={THREE.DoubleSide}
          />
        </mesh>
        <lineSegments geometry={edgeGeometry}>
          <lineBasicMaterial color="#5f2304" linewidth={1} />
        </lineSegments>
        <FireEmbers bottomY={bottomExtent} width={width * 0.7} />
      </group>
    </group>
  );
}

function FireEmbers({ bottomY, width }: FireEmbersProps) {
  const particlesRef = useRef<THREE.Points>(null);
  const particleCount = 15;

  const { positions, velocities, lifetimes } = useMemo(() => {
    const pos = new Float32Array(particleCount * 3);
    const vel = new Float32Array(particleCount * 3);
    const life = new Float32Array(particleCount);

    for (let i = 0; i < particleCount; i++) {
      pos[i * 3] = (Math.random() - 0.5) * width;
      pos[i * 3 + 1] = bottomY + Math.random() * 0.3;
      pos[i * 3 + 2] = -0.04 + Math.random() * 0.08;

      vel[i * 3] = (Math.random() - 0.5) * 0.015;
      vel[i * 3 + 1] = 0.02 + Math.random() * 0.025;
      vel[i * 3 + 2] = -0.005 - Math.random() * 0.01;

      life[i] = Math.random();
    }

    return { positions: pos, velocities: vel, lifetimes: life };
  }, [bottomY, width]);

  useFrame((_, delta) => {
    if (!particlesRef.current) return;

    const posAttr = particlesRef.current.geometry.attributes.position;
    const posArray = posAttr.array as Float32Array;

    for (let i = 0; i < particleCount; i++) {
      lifetimes[i] += delta * 0.3;

      if (lifetimes[i] > 1) {
        lifetimes[i] = 0;
        posArray[i * 3] = (Math.random() - 0.5) * width;
        posArray[i * 3 + 1] = bottomY + Math.random() * 0.3;
        posArray[i * 3 + 2] = -0.04 + Math.random() * 0.08;
      } else {
        posArray[i * 3] += velocities[i * 3];
        posArray[i * 3 + 1] += velocities[i * 3 + 1];
        posArray[i * 3 + 2] += velocities[i * 3 + 2];
      }
    }

    posAttr.needsUpdate = true;
  });

  return (
    <points ref={particlesRef}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={particleCount}
          array={positions}
          itemSize={3}
        />
      </bufferGeometry>
      <pointsMaterial
        size={0.04}
        color="#ffaa00"
        transparent
        opacity={0.8}
        sizeAttenuation
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}
