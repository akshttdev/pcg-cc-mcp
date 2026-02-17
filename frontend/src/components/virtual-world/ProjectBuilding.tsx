import { Suspense, useState, useRef, useMemo, useEffect } from 'react';
import { useFrame } from '@react-three/fiber';
import { Text, useGLTF } from '@react-three/drei';
import * as THREE from 'three';

// Preload the gallery model so it's ready when the building renders
useGLTF.preload('/environments/fine-art-gallery.glb');
import { BuildingType, BUILDING_THEMES, getBuildingType } from '@/lib/virtual-world/buildingTypes';
import { ENTRY_PROMPT_HEIGHT, DOOR_WIDTH, DOOR_HEIGHT } from '@/lib/virtual-world/constants';

// Default footprint for hover/select ring and walkway calculations
const DEFAULT_HALF_WIDTH = 25;
const DEFAULT_HALF_LENGTH = 50;
const DEFAULT_FOOTPRINT_RADIUS = Math.sqrt(DEFAULT_HALF_WIDTH ** 2 + DEFAULT_HALF_LENGTH ** 2);

export const PUBLIC_PROJECTS = new Set(['Fine Art Society']);

interface ProjectBuildingProps {
  name: string;
  position: [number, number, number];
  energy: number;
  isSelected: boolean;
  onSelect: () => void;
  isEnterTarget?: boolean;
  entryHotkey?: string;
  locked?: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// SIRAK STUDIOS — narrow 10×70×20 studio loft
// ─────────────────────────────────────────────────────────────────────────────
function SirakStudio({
  energy, isSelected, hovered, accentColor, hologramColor, baseColor, locked,
}: { energy: number; isSelected: boolean; hovered: boolean; accentColor: string; hologramColor: string; baseColor: string; locked: boolean }) {
  const boost = energy * (hovered || isSelected ? 1.5 : 1) * (locked ? 0.3 : 1);
  const W = 10; const L = 70; const H = 20;

  // Tall window cutout positions along each long side (represented as glow planes)
  const windowPositionsZ = [-25, -10, 5, 20];

  return (
    <group>
      {/* Main studio volume */}
      <mesh position={[0, H / 2, 0]} castShadow receiveShadow>
        <boxGeometry args={[W, H, L]} />
        <meshStandardMaterial color={baseColor} metalness={0.3} roughness={0.6}
          emissive={accentColor} emissiveIntensity={boost * 0.04} />
      </mesh>

      {/* Flat industrial roof with slight overhang */}
      <mesh position={[0, H + 0.3, 0]} castShadow>
        <boxGeometry args={[W + 1.5, 0.6, L + 1.5]} />
        <meshStandardMaterial color={accentColor} metalness={0.8} roughness={0.2}
          emissive={hologramColor} emissiveIntensity={boost * 0.3} />
      </mesh>

      {/* Roof edge light strip — long sides */}
      {([-L / 2 - 0.76, L / 2 + 0.76] as number[]).map((z, i) => (
        <mesh key={`rstrip-${i}`} position={[0, H + 0.62, z]} rotation={[-Math.PI / 2, 0, 0]}>
          <planeGeometry args={[W + 1.5, 0.4]} />
          <meshBasicMaterial color={accentColor} transparent opacity={boost * 0.7} />
        </mesh>
      ))}

      {/* Tall windows — glowing planes flush on east wall */}
      {windowPositionsZ.map((z, i) => (
        <mesh key={`winE-${i}`} position={[W / 2 + 0.01, H * 0.55, z]}>
          <planeGeometry args={[0.1, H * 0.6]} />
          <meshBasicMaterial color={hologramColor} transparent opacity={boost * 0.5} side={THREE.DoubleSide} />
        </mesh>
      ))}
      {/* West wall windows */}
      {windowPositionsZ.map((z, i) => (
        <mesh key={`winW-${i}`} position={[-W / 2 - 0.01, H * 0.55, z]}>
          <planeGeometry args={[0.1, H * 0.6]} />
          <meshBasicMaterial color={hologramColor} transparent opacity={boost * 0.5} side={THREE.DoubleSide} />
        </mesh>
      ))}

      {/* Ground trim strip */}
      <mesh position={[0, 0.4, 0]}>
        <boxGeometry args={[W + 0.2, 0.8, L + 0.2]} />
        <meshStandardMaterial color={accentColor} metalness={0.9} roughness={0.1}
          emissive={accentColor} emissiveIntensity={boost * 0.4} />
      </mesh>

      {/* Interior fill light */}
      <pointLight position={[0, H * 0.5, 0]} color={hologramColor}
        intensity={boost * 0.8} distance={40} decay={2} />
    </group>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// MEDIA MONSTERS HQ — original solid rectangle 50×100×20
// ─────────────────────────────────────────────────────────────────────────────
function MediaMonstersHQ({
  energy, isSelected, hovered, accentColor, hologramColor, baseColor, locked,
}: { energy: number; isSelected: boolean; hovered: boolean; accentColor: string; hologramColor: string; baseColor: string; locked: boolean }) {
  const boost = energy * (hovered || isSelected ? 1.5 : 1) * (locked ? 0.3 : 1);
  const W = 50; const L = 100; const H = 20;

  return (
    <group>
      {/* Main solid box */}
      <mesh position={[0, H / 2, 0]} castShadow receiveShadow>
        <boxGeometry args={[W, H, L]} />
        <meshStandardMaterial color={baseColor} metalness={0.5} roughness={0.5}
          emissive={accentColor} emissiveIntensity={boost * 0.05} />
      </mesh>

      {/* Roof slab */}
      <mesh position={[0, H + 0.4, 0]} castShadow>
        <boxGeometry args={[W + 1, 0.8, L + 1]} />
        <meshStandardMaterial color={accentColor} metalness={0.8} roughness={0.15}
          emissive={hologramColor} emissiveIntensity={boost * 0.4} />
      </mesh>

      {/* HQ sign band below roof — front face */}
      <mesh position={[0, H - 2.5, L / 2 + 0.01]}>
        <planeGeometry args={[W * 0.7, 4]} />
        <meshBasicMaterial color={hologramColor} transparent opacity={boost * 0.6} side={THREE.DoubleSide} />
      </mesh>

      {/* Corner accent columns */}
      {([-W / 2 + 1.5, W / 2 - 1.5] as number[]).flatMap((x) =>
        ([-L / 2 + 1.5, L / 2 - 1.5] as number[]).map((z, j) => (
          <mesh key={`col-${x}-${j}`} position={[x, H / 2, z]} castShadow>
            <boxGeometry args={[3, H + 1, 3]} />
            <meshStandardMaterial color={accentColor} metalness={0.85} roughness={0.1}
              emissive={hologramColor} emissiveIntensity={boost * 0.5} />
          </mesh>
        ))
      )}

      {/* Ground base trim */}
      <mesh position={[0, 0.5, 0]}>
        <boxGeometry args={[W + 0.4, 1, L + 0.4]} />
        <meshStandardMaterial color={accentColor} metalness={0.9} roughness={0.1}
          emissive={accentColor} emissiveIntensity={boost * 0.3} />
      </mesh>

      {/* Interior light */}
      <pointLight position={[0, H * 0.5, 0]} color={hologramColor}
        intensity={boost * 1.5} distance={80} decay={2} />
    </group>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// VERITWIN — classical bank: wide steps, columns, pediment
// ─────────────────────────────────────────────────────────────────────────────
function VeritwinBank({
  energy, isSelected, hovered, accentColor, hologramColor, baseColor, locked,
}: { energy: number; isSelected: boolean; hovered: boolean; accentColor: string; hologramColor: string; baseColor: string; locked: boolean }) {
  const boost = energy * (hovered || isSelected ? 1.5 : 1) * (locked ? 0.3 : 1);
  const W = 50; const L = 80; const H = 20;

  // 6 columns across the front face
  const colCount = 6;
  const colSpacing = W / (colCount - 1);
  const colPositions = Array.from({ length: colCount }, (_, i) => -W / 2 + i * colSpacing);

  return (
    <group>
      {/* ── Podium / base steps ── */}
      {[0, 1, 2].map((s) => (
        <mesh key={`step-${s}`} position={[0, s * 1.1, L / 2 - 2 + s * 1.5]} receiveShadow>
          <boxGeometry args={[W + 4 - s * 2, 1.1, 6 - s * 0.8]} />
          <meshStandardMaterial color={baseColor} metalness={0.2} roughness={0.8} />
        </mesh>
      ))}

      {/* ── Main building body ── */}
      <mesh position={[0, H / 2 + 3.3, 0]} castShadow receiveShadow>
        <boxGeometry args={[W, H, L]} />
        <meshStandardMaterial color={baseColor} metalness={0.15} roughness={0.75}
          emissive={accentColor} emissiveIntensity={boost * 0.04} />
      </mesh>

      {/* ── Front colonnade — 6 fluted columns ── */}
      {colPositions.map((x, i) => (
        <mesh key={`col-${i}`} position={[x, H / 2 + 3.3, L / 2 + 1]} castShadow>
          <cylinderGeometry args={[1.2, 1.4, H + 2, 12]} />
          <meshStandardMaterial color={baseColor} metalness={0.1} roughness={0.85}
            emissive={hologramColor} emissiveIntensity={boost * 0.08} />
        </mesh>
      ))}
      {/* Column capitals */}
      {colPositions.map((x, i) => (
        <mesh key={`cap-${i}`} position={[x, H + 4.3 + 1.1, L / 2 + 1]}>
          <boxGeometry args={[3.2, 1.2, 3.2]} />
          <meshStandardMaterial color={accentColor} metalness={0.3} roughness={0.6}
            emissive={hologramColor} emissiveIntensity={boost * 0.2} />
        </mesh>
      ))}

      {/* ── Entablature — horizontal beam above columns ── */}
      <mesh position={[0, H + 5.6 + 1.1, L / 2 + 1]}>
        <boxGeometry args={[W + 2, 2, 3]} />
        <meshStandardMaterial color={accentColor} metalness={0.3} roughness={0.6}
          emissive={hologramColor} emissiveIntensity={boost * 0.15} />
      </mesh>

      {/* ── Triangular pediment ── */}
      {/* Left slope */}
      <mesh position={[-W / 4, H + 9.6, L / 2 + 0.5]} rotation={[0, 0, Math.PI / 6]}>
        <boxGeometry args={[W / 2 + 1, 1.2, 3.5]} />
        <meshStandardMaterial color={baseColor} metalness={0.15} roughness={0.75} />
      </mesh>
      {/* Right slope */}
      <mesh position={[W / 4, H + 9.6, L / 2 + 0.5]} rotation={[0, 0, -Math.PI / 6]}>
        <boxGeometry args={[W / 2 + 1, 1.2, 3.5]} />
        <meshStandardMaterial color={baseColor} metalness={0.15} roughness={0.75} />
      </mesh>
      {/* Pediment back fill */}
      <mesh position={[0, H + 8.5, L / 2 + 0.3]}>
        <boxGeometry args={[W, 4, 2.5]} />
        <meshStandardMaterial color={baseColor} metalness={0.15} roughness={0.75}
          emissive={hologramColor} emissiveIntensity={boost * 0.06} />
      </mesh>

      {/* ── Flat roof ── */}
      <mesh position={[0, H + 3.3 + H * 0.5 + 0.5, 0]}>
        <boxGeometry args={[W + 0.5, 0.8, L + 0.5]} />
        <meshStandardMaterial color={accentColor} metalness={0.5} roughness={0.4} />
      </mesh>

      {/* ── Side pilasters (shallow wall columns) ── */}
      {[-L / 3, 0, L / 3].map((z, i) =>
        ([-W / 2 - 0.2, W / 2 + 0.2] as number[]).map((x, j) => (
          <mesh key={`pil-${i}-${j}`} position={[x, H / 2 + 3.3, z]} castShadow>
            <boxGeometry args={[1, H, 2]} />
            <meshStandardMaterial color={accentColor} metalness={0.2} roughness={0.7}
              emissive={hologramColor} emissiveIntensity={boost * 0.1} />
          </mesh>
        ))
      )}

      {/* ── Ground light around base ── */}
      <pointLight position={[0, 4, L / 2 + 4]} color={hologramColor}
        intensity={boost * 1.2} distance={30} decay={2} />
      <pointLight position={[0, 4, -L / 2 - 2]} color={hologramColor}
        intensity={boost * 0.6} distance={25} decay={2} />
    </group>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// JUNGLEVERSE CASINO — flashy marquee, neon canopy, layered facade
// ─────────────────────────────────────────────────────────────────────────────
function JungleversCasino({
  energy, isSelected, hovered, accentColor, hologramColor, baseColor, locked,
}: { energy: number; isSelected: boolean; hovered: boolean; accentColor: string; hologramColor: string; baseColor: string; locked: boolean }) {
  const marqueeRef = useRef<THREE.Mesh>(null);
  const boost = energy * (hovered || isSelected ? 1.5 : 1) * (locked ? 0.3 : 1);
  const W = 50; const L = 80; const H = 20;

  useFrame((state) => {
    if (marqueeRef.current) {
      const mat = marqueeRef.current.material as THREE.MeshBasicMaterial;
      mat.opacity = boost * (0.5 + 0.4 * Math.sin(state.clock.elapsedTime * 3));
    }
  });

  return (
    <group>
      {/* ── Main building box ── */}
      <mesh position={[0, H / 2, 0]} castShadow receiveShadow>
        <boxGeometry args={[W, H, L]} />
        <meshStandardMaterial color={baseColor} metalness={0.4} roughness={0.5}
          emissive={accentColor} emissiveIntensity={boost * 0.06} />
      </mesh>

      {/* ── Stepped setback — upper tier ── */}
      <mesh position={[0, H + 5, 0]} castShadow>
        <boxGeometry args={[W - 8, 10, L - 12]} />
        <meshStandardMaterial color={baseColor} metalness={0.4} roughness={0.45}
          emissive={accentColor} emissiveIntensity={boost * 0.08} />
      </mesh>

      {/* ── Grand marquee sign tower above entrance ── */}
      <mesh position={[0, H + 16, L / 2 + 2]} castShadow>
        <boxGeometry args={[W * 0.8, 12, 2]} />
        <meshStandardMaterial color={hologramColor} metalness={0.9} roughness={0.05}
          emissive={hologramColor} emissiveIntensity={boost * 0.8} />
      </mesh>
      {/* Flashing marquee overlay */}
      <mesh ref={marqueeRef} position={[0, H + 16, L / 2 + 3.1]}>
        <planeGeometry args={[W * 0.75, 11]} />
        <meshBasicMaterial color={accentColor} transparent opacity={boost * 0.7} side={THREE.DoubleSide} />
      </mesh>
      {/* Marquee frame */}
      <mesh position={[0, H + 16, L / 2 + 1.5]}>
        <boxGeometry args={[W * 0.8 + 2, 13, 0.5]} />
        <meshBasicMaterial color={accentColor} />
      </mesh>

      {/* ── Entrance canopy — curved awning ── */}
      <mesh position={[0, H * 0.6, L / 2 + 8]} rotation={[Math.PI / 8, 0, 0]}>
        <boxGeometry args={[W * 0.7, 0.6, 14]} />
        <meshPhysicalMaterial color={accentColor} metalness={0.7} roughness={0.15}
          emissive={hologramColor} emissiveIntensity={boost * 0.5}
          transmission={0.2} transparent opacity={0.85} />
      </mesh>
      {/* Canopy support columns */}
      {([-W * 0.3, W * 0.3] as number[]).map((x, i) => (
        <mesh key={`csup-${i}`} position={[x, H * 0.3, L / 2 + 8]}>
          <cylinderGeometry args={[0.5, 0.5, H * 0.6, 8]} />
          <meshStandardMaterial color={hologramColor} metalness={0.9} roughness={0.05}
            emissive={hologramColor} emissiveIntensity={boost * 0.6} />
        </mesh>
      ))}

      {/* ── Neon panel strips on facade ── */}
      {[-6, 0, 6].map((z, i) =>
        ([-W / 2 + 1.5, W / 2 - 1.5] as number[]).map((x, j) => (
          <mesh key={`neon-${i}-${j}`} position={[x, H * 0.5, z]}>
            <boxGeometry args={[0.3, H * 0.5, 0.5]} />
            <meshBasicMaterial color={i % 2 === 0 ? accentColor : hologramColor}
              transparent opacity={boost * (0.6 + 0.3 * (i % 2))} />
          </mesh>
        ))
      )}

      {/* ── Rooftop parapet with bulb lights ── */}
      <mesh position={[0, H + 0.5, 0]}>
        <boxGeometry args={[W + 1, 1, L + 1]} />
        <meshStandardMaterial color={accentColor} metalness={0.8} roughness={0.1}
          emissive={accentColor} emissiveIntensity={boost * 0.5} />
      </mesh>
      {/* Parapet bulbs — front edge */}
      {Array.from({ length: 9 }, (_, i) => -W / 2 + 6 + i * 5).map((x, i) => (
        <mesh key={`bulb-${i}`} position={[x, H + 1.4, L / 2 + 0.5]}>
          <sphereGeometry args={[0.5, 8, 8]} />
          <meshBasicMaterial color={hologramColor} transparent opacity={boost * 0.9} />
        </mesh>
      ))}

      {/* ── Bright entrance glow ── */}
      <pointLight position={[0, H * 0.7, L / 2 + 4]} color={hologramColor}
        intensity={boost * 3} distance={40} decay={2} />
      <pointLight position={[0, H + 18, L / 2 + 3]} color={accentColor}
        intensity={boost * 2} distance={50} decay={2} />
    </group>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Fine Art Society — GLB interior model loader
// The model is designed with its floor at y=0 (original BuildingInterior used
// position:[0,0,0] with no vertical correction). We match that exactly.
// ─────────────────────────────────────────────────────────────────────────────
const GLB_SCALE = 4;

function GalleryInteriorModel() {
  const { scene } = useGLTF('/environments/fine-art-gallery.glb');
  const groupRef = useRef<THREE.Group>(null);

  useEffect(() => {
    const group = groupRef.current;
    if (!group) return;

    // Fix texture colorspace and enable shadows
    scene.traverse((child) => {
      if (!(child instanceof THREE.Mesh)) return;
      child.castShadow = true;
      child.receiveShadow = true;

      const mats = Array.isArray(child.material) ? child.material : [child.material];
      mats.forEach((mat) => {
        if (!mat) return;
        for (const key of ['map', 'emissiveMap'] as const) {
          const tex = (mat as Record<string, THREE.Texture | null>)[key];
          if (tex) { tex.colorSpace = THREE.SRGBColorSpace; tex.needsUpdate = true; }
        }
        for (const key of ['normalMap', 'roughnessMap', 'metalnessMap', 'aoMap', 'lightMap'] as const) {
          const tex = (mat as Record<string, THREE.Texture | null>)[key];
          if (tex) { tex.colorSpace = THREE.LinearSRGBColorSpace; tex.needsUpdate = true; }
        }
        if ('envMapIntensity' in mat) (mat as THREE.MeshStandardMaterial).envMapIntensity = 1;
        mat.needsUpdate = true;
      });
    });

    // The GLB was authored with the floor at y=0 in model space.
    // With scale=4 the floor sits at world y=0 naturally — no offset needed.
    group.position.y = 0;
  }, [scene]);

  return (
    <group ref={groupRef} scale={[GLB_SCALE, GLB_SCALE, GLB_SCALE]}>
      <primitive object={scene} />
    </group>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// FINE ART SOCIETY — no exterior, just the Hermitage GLB in-world
// ─────────────────────────────────────────────────────────────────────────────
function FineArtGallery({
  energy, isSelected, hovered, locked,
}: { energy: number; isSelected: boolean; hovered: boolean; accentColor: string; hologramColor: string; baseColor: string; locked: boolean }) {
  const boost = energy * (hovered || isSelected ? 1.5 : 1) * (locked ? 0.3 : 1);

  return (
    <group>
      {/* Gallery lighting — multiple warm fills spread through the space */}
      <pointLight position={[0, 12, 0]}   color="#fff5e0" intensity={boost * 6}   distance={120} decay={1.5} />
      <pointLight position={[0, 12, 30]}  color="#ffe8c8" intensity={boost * 5}   distance={100} decay={1.5} />
      <pointLight position={[0, 12, -30]} color="#ffe8c8" intensity={boost * 5}   distance={100} decay={1.5} />
      <pointLight position={[20, 10, 15]} color="#fff0d0" intensity={boost * 3.5} distance={80}  decay={2} />
      <pointLight position={[-20, 10, 15]}color="#fff0d0" intensity={boost * 3.5} distance={80}  decay={2} />
      <pointLight position={[20, 10,-15]} color="#fff0d0" intensity={boost * 3.5} distance={80}  decay={2} />
      <pointLight position={[-20, 10,-15]}color="#fff0d0" intensity={boost * 3.5} distance={80}  decay={2} />
      {/* Low fill near floor level */}
      <pointLight position={[0, 3, 0]}    color="#ffedda" intensity={boost * 2}   distance={60}  decay={2} />

      {/* Hermitage GLB — floor at y=0 by design */}
      <Suspense fallback={null}>
        <GalleryInteriorModel />
      </Suspense>
    </group>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Main ProjectBuilding — dispatches to the right structure
// ─────────────────────────────────────────────────────────────────────────────
function getBuildingFootprint(type: BuildingType): { hw: number; hl: number } {
  if (type === 'creative-studio') return { hw: 5, hl: 35 };   // Sirak Studios 10×70
  if (type === 'bank')            return { hw: 25, hl: 40 };  // Veritwin bank 50×80
  if (type === 'casino')          return { hw: 25, hl: 40 };  // Jungleverse casino 50×80
  if (type === 'gallery')         return { hw: 42, hl: 62 };  // Fine Art Society — sized for GLB at scale=4
  return { hw: DEFAULT_HALF_WIDTH, hl: DEFAULT_HALF_LENGTH }; // default 50×100
}

export function ProjectBuilding({
  name,
  position,
  energy,
  isSelected,
  onSelect,
  isEnterTarget = false,
  entryHotkey = 'E',
  locked = false,
}: ProjectBuildingProps) {
  const [hovered, setHovered] = useState(false);
  const buildingType = getBuildingType(name);
  const theme = BUILDING_THEMES[buildingType];
  const isPublic = PUBLIC_PROJECTS.has(name);
  const effectiveEnergy = locked ? energy * 0.3 : energy;

  const { hw, hl } = getBuildingFootprint(buildingType);
  const footprintRadius = Math.sqrt(hw ** 2 + hl ** 2);

  const entranceDirection = useMemo(() => {
    const dir = new THREE.Vector3(-position[0], 0, -position[2]);
    if (dir.lengthSq() === 0) dir.set(0, 0, 1);
    return dir.normalize();
  }, [position]);

  const doorRotation = useMemo(
    () => Math.atan2(entranceDirection.x, entranceDirection.z),
    [entranceDirection],
  );

  // Place arch at the front face of the building (+Z = entrance side), 5 units high
  const doorPosition = useMemo(
    () => [0, 5, hl] as [number, number, number],
    [hl],
  );

  const walkwayPosition = useMemo(() => {
    const offset = entranceDirection.clone().multiplyScalar(hl + 22);
    return [position[0] + offset.x, position[1] + 0.11, position[2] + offset.z] as [number, number, number];
  }, [entranceDirection, position, hl]);

  const labelColor = locked ? '#666666' : hovered || isSelected ? '#ffffff' : theme.labelColor;
  const doorColor = locked ? '#ff2200' : theme.doorColor;
  const showEnterPrompt = isEnterTarget && Boolean(entryHotkey) && !locked;
  const showLockedPrompt = isEnterTarget && locked;

  const buildingProps = {
    energy: effectiveEnergy,
    isSelected,
    hovered,
    accentColor: theme.accentColor,
    hologramColor: theme.hologramColor,
    baseColor: theme.baseColor,
    locked,
  };

  return (
    <group
      onClick={(e) => { e.stopPropagation(); onSelect(); }}
      onPointerOver={(e) => { e.stopPropagation(); setHovered(true); document.body.style.cursor = 'pointer'; }}
      onPointerOut={(e) => { e.stopPropagation(); setHovered(false); document.body.style.cursor = 'default'; }}
    >
      {/* Dispatch to the correct building shape */}
      {buildingType === 'creative-studio' && name.toLowerCase().includes('sirak') && (
        <SirakStudio {...buildingProps} />
      )}
      {buildingType === 'command' && (
        <MediaMonstersHQ {...buildingProps} />
      )}
      {buildingType === 'bank' && (
        <VeritwinBank {...buildingProps} />
      )}
      {buildingType === 'casino' && (
        <JungleversCasino {...buildingProps} />
      )}
      {buildingType === 'gallery' && (
        <FineArtGallery {...buildingProps} />
      )}
      {/* Fallback for any other type */}
      {!['creative-studio', 'command', 'bank', 'casino', 'gallery'].includes(buildingType) && (
        <MediaMonstersHQ {...buildingProps} />
      )}

      {/* Building name label */}
      <Text
        position={[position[0], position[1] + 50, position[2]]}
        fontSize={5}
        color={labelColor}
        anchorX="center"
        anchorY="bottom"
        outlineWidth={0.2}
        outlineColor="#001925"
      >
        {name}
      </Text>

      {isPublic && (
        <Text
          position={[position[0], position[1] + 42, position[2]]}
          fontSize={2.5}
          color="#ffd700"
          anchorX="center"
          anchorY="bottom"
        >
          ◆ Open to all ◆
        </Text>
      )}

      {locked && (
        <Text
          position={[position[0], position[1] + 42, position[2]]}
          fontSize={2.5}
          color="#ff4444"
          anchorX="center"
          anchorY="bottom"
        >
          ⊘ Access Restricted
        </Text>
      )}

      {/* Ground plate */}
      <mesh position={[position[0], position[1] + 0.1, position[2]]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <circleGeometry args={[footprintRadius + 10, 64]} />
        <meshStandardMaterial
          color="#0a1f35"
          metalness={0.8}
          roughness={0.3}
          emissive="#004080"
          emissiveIntensity={isSelected || hovered ? 0.5 : 0.2}
        />
      </mesh>

      {/* Approach walkway */}
      <mesh position={walkwayPosition} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[12, 40]} />
        <meshBasicMaterial
          color={theme.accentColor}
          transparent
          opacity={isEnterTarget ? 0.45 : 0.15}
        />
      </mesh>

      {/* Hover/select ring */}
      {(isSelected || hovered) && (
        <mesh position={[position[0], position[1] + 0.2, position[2]]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[footprintRadius + 4, footprintRadius + 7, 64]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={isSelected ? 0.8 : 0.5} />
        </mesh>
      )}

      {/* Entrance arch — not shown for gallery (open walkable space) */}
      {buildingType !== 'gallery' && <group position={doorPosition} rotation={[0, doorRotation, 0]}>
        <mesh position={[-(DOOR_WIDTH / 2 + 0.5), 0, 0]}>
          <boxGeometry args={[1, DOOR_HEIGHT + 2, 1.5]} />
          <meshStandardMaterial color={theme.accentColor} metalness={0.8} roughness={0.2}
            emissive={theme.hologramColor} emissiveIntensity={isEnterTarget ? 0.8 : 0.2} />
        </mesh>
        <mesh position={[(DOOR_WIDTH / 2 + 0.5), 0, 0]}>
          <boxGeometry args={[1, DOOR_HEIGHT + 2, 1.5]} />
          <meshStandardMaterial color={theme.accentColor} metalness={0.8} roughness={0.2}
            emissive={theme.hologramColor} emissiveIntensity={isEnterTarget ? 0.8 : 0.2} />
        </mesh>
        <mesh position={[0, DOOR_HEIGHT / 2 + 0.5, 0]}>
          <boxGeometry args={[DOOR_WIDTH + 2, 1, 1.5]} />
          <meshStandardMaterial color={theme.accentColor} metalness={0.8} roughness={0.2}
            emissive={theme.hologramColor} emissiveIntensity={isEnterTarget ? 0.8 : 0.2} />
        </mesh>
        <mesh position={[0, -DOOR_HEIGHT / 2 + 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <planeGeometry args={[DOOR_WIDTH, 3]} />
          <meshBasicMaterial color={doorColor} transparent
            opacity={isEnterTarget ? 0.7 : locked ? 0.5 : 0.2} />
        </mesh>
        <pointLight
          position={[0, DOOR_HEIGHT / 2 + 1.5, 1]}
          intensity={isEnterTarget ? 2 : locked ? 0.3 : 0.5}
          color={locked ? '#ff0000' : theme.hologramColor}
          distance={15}
        />
      </group>}

      {buildingType !== 'gallery' && showEnterPrompt && (
        <Text
          position={[doorPosition[0], doorPosition[1] + ENTRY_PROMPT_HEIGHT, doorPosition[2]]}
          fontSize={1.5}
          color={isPublic ? '#ffd700' : theme.hologramColor}
          anchorX="center"
          anchorY="bottom"
          outlineWidth={0.08}
          outlineColor="#000a10"
        >
          Press {entryHotkey} to enter{isPublic ? ' (Public)' : ''}
        </Text>
      )}

      {buildingType !== 'gallery' && showLockedPrompt && (
        <Text
          position={[doorPosition[0], doorPosition[1] + ENTRY_PROMPT_HEIGHT, doorPosition[2]]}
          fontSize={1.5}
          color="#ff4444"
          anchorX="center"
          anchorY="bottom"
          outlineWidth={0.08}
          outlineColor="#220000"
        >
          Access Restricted
        </Text>
      )}
    </group>
  );
}
