import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

// OSRS Fire Cape - Earned from TzHaar Fight Cave
// Features organic lava blob pattern on orange/yellow background
// with olive/brown clasp and pointed bottom

export function FireCapeEquipment() {
  const capeGroupRef = useRef<THREE.Group>(null);

  // Create cape shape - pointed bottom like OSRS reference
  const { capeGeometry, edgeGeometry } = useMemo(() => {
    const topWidth = 0.38;
    const midWidth = 0.75;
    const height = 1.35;

    const shape = new THREE.Shape();
    // Start from top left
    shape.moveTo(-topWidth / 2, 0);
    // Top edge
    shape.lineTo(topWidth / 2, 0);
    // Right side - curves outward then tapers to point
    shape.quadraticCurveTo(topWidth * 0.6, -0.15, midWidth / 2, -height * 0.3);
    shape.quadraticCurveTo(midWidth / 2 + 0.08, -height * 0.6, midWidth * 0.35, -height * 0.85);
    // Bottom point
    shape.lineTo(0, -height);
    // Left side - mirror of right
    shape.lineTo(-midWidth * 0.35, -height * 0.85);
    shape.quadraticCurveTo(-midWidth / 2 - 0.08, -height * 0.6, -midWidth / 2, -height * 0.3);
    shape.quadraticCurveTo(-topWidth * 0.6, -0.15, -topWidth / 2, 0);
    shape.closePath();

    const geometry = new THREE.ShapeGeometry(shape, 32);
    const edges = new THREE.EdgesGeometry(geometry, 60);

    return { capeGeometry: geometry, edgeGeometry: edges };
  }, []);

  // Create lava texture matching OSRS style - organic blobs
  const fireCapeTexture = useMemo(() => {
    if (typeof document === 'undefined') return null;

    const size = 512;
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    // Base gradient - light orange/yellow at top, deeper orange at bottom
    const gradient = ctx.createLinearGradient(0, 0, 0, size);
    gradient.addColorStop(0, '#f5c563'); // Light golden yellow
    gradient.addColorStop(0.3, '#f0a03c'); // Orange-yellow
    gradient.addColorStop(0.7, '#e88a2a'); // Orange
    gradient.addColorStop(1, '#d97020'); // Deeper orange
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, size, size);

    // Draw organic lava blobs - darker orange spots
    // These should look like flowing lava pools
    const drawLavaBlob = (x: number, y: number, w: number, h: number, rotation: number) => {
      ctx.save();
      ctx.translate(x, y);
      ctx.rotate(rotation);

      // Create organic blob shape
      const blobGradient = ctx.createRadialGradient(0, 0, 0, 0, 0, Math.max(w, h));
      blobGradient.addColorStop(0, 'rgba(180, 80, 30, 0.85)'); // Dark orange center
      blobGradient.addColorStop(0.5, 'rgba(200, 100, 40, 0.7)'); // Medium orange
      blobGradient.addColorStop(1, 'rgba(220, 130, 50, 0.0)'); // Fade to transparent

      ctx.fillStyle = blobGradient;
      ctx.beginPath();

      // Organic blob with irregular edges
      const points = 12;
      for (let i = 0; i <= points; i++) {
        const angle = (i / points) * Math.PI * 2;
        const radiusVariation = 0.7 + Math.sin(i * 3.7) * 0.3;
        const rx = w * radiusVariation;
        const ry = h * radiusVariation;
        const px = Math.cos(angle) * rx;
        const py = Math.sin(angle) * ry;
        if (i === 0) {
          ctx.moveTo(px, py);
        } else {
          ctx.lineTo(px, py);
        }
      }
      ctx.closePath();
      ctx.fill();
      ctx.restore();
    };

    // Large flowing blobs
    drawLavaBlob(size * 0.3, size * 0.2, 60, 45, 0.3);
    drawLavaBlob(size * 0.7, size * 0.15, 50, 40, -0.2);
    drawLavaBlob(size * 0.5, size * 0.35, 70, 50, 0.1);
    drawLavaBlob(size * 0.25, size * 0.5, 55, 65, -0.4);
    drawLavaBlob(size * 0.75, size * 0.45, 60, 55, 0.5);
    drawLavaBlob(size * 0.4, size * 0.65, 65, 50, 0.2);
    drawLavaBlob(size * 0.6, size * 0.7, 55, 60, -0.3);
    drawLavaBlob(size * 0.35, size * 0.85, 50, 45, 0.4);
    drawLavaBlob(size * 0.65, size * 0.88, 45, 55, -0.1);
    drawLavaBlob(size * 0.5, size * 0.55, 45, 40, 0.6);

    // Medium blobs for detail
    for (let i = 0; i < 15; i++) {
      const x = (Math.random() * 0.7 + 0.15) * size;
      const y = (Math.random() * 0.85 + 0.1) * size;
      const w = 25 + Math.random() * 30;
      const h = 20 + Math.random() * 35;
      const rot = (Math.random() - 0.5) * Math.PI;
      drawLavaBlob(x, y, w, h, rot);
    }

    // Small accent blobs
    for (let i = 0; i < 25; i++) {
      const x = Math.random() * size;
      const y = Math.random() * size;
      const w = 10 + Math.random() * 20;
      const h = 8 + Math.random() * 18;
      const rot = Math.random() * Math.PI;
      drawLavaBlob(x, y, w, h, rot);
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.wrapS = THREE.ClampToEdgeWrapping;
    texture.wrapT = THREE.ClampToEdgeWrapping;
    texture.needsUpdate = true;
    return texture;
  }, []);

  useFrame(({ clock }) => {
    if (!capeGroupRef.current) return;
    const time = clock.elapsedTime;
    // Subtle flutter animation
    const flutter = Math.sin(time * 1.5) * 0.03;
    capeGroupRef.current.rotation.x = 0.32 + flutter;
    capeGroupRef.current.rotation.z = Math.sin(time * 0.8) * 0.015;
    capeGroupRef.current.position.z = -0.02 - Math.cos(time * 1.2) * 0.01;
  });

  // OSRS-style olive/brown clasp color
  const claspColor = '#5a5030'; // Olive brown like the reference

  return (
    <group position={[0, 0.68, -0.26]}>
      {/* Hexagonal clasp/hook - olive/brown like OSRS reference */}
      <mesh position={[0, 0.02, -0.02]} rotation={[Math.PI / 2, 0, 0]}>
        <cylinderGeometry args={[0.06, 0.07, 0.04, 6]} />
        <meshStandardMaterial
          color={claspColor}
          metalness={0.4}
          roughness={0.6}
        />
      </mesh>

      {/* Inner clasp detail */}
      <mesh position={[0, 0.02, -0.02]} rotation={[Math.PI / 2, 0, 0]}>
        <cylinderGeometry args={[0.035, 0.04, 0.05, 6]} />
        <meshStandardMaterial
          color="#3d3520"
          metalness={0.3}
          roughness={0.7}
        />
      </mesh>

      {/* Cloth attachment loop */}
      <mesh position={[0, -0.06, -0.02]}>
        <boxGeometry args={[0.07, 0.1, 0.035]} />
        <meshStandardMaterial
          color="#d98c30"
          metalness={0.1}
          roughness={0.7}
        />
      </mesh>

      {/* Main cape */}
      <group ref={capeGroupRef} position={[0, -0.14, -0.03]}>
        <mesh geometry={capeGeometry} castShadow receiveShadow>
          <meshStandardMaterial
            color="#f0a040"
            map={fireCapeTexture ?? undefined}
            emissive="#e08020"
            emissiveIntensity={0.15}
            roughness={0.75}
            metalness={0}
            side={THREE.DoubleSide}
          />
        </mesh>

        {/* Subtle edge outline */}
        <lineSegments geometry={edgeGeometry}>
          <lineBasicMaterial color="#8a4010" linewidth={1} transparent opacity={0.5} />
        </lineSegments>
      </group>
    </group>
  );
}
