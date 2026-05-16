import { Canvas, useFrame } from '@react-three/fiber';
import { useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';

// ── Types ─────────────────────────────────────────────────────────────────────

export type OrbState = null | 'thinking' | 'listening' | 'talking';

interface OrbVisualizerProps {
  state: OrbState;
  isAdmin?: boolean;
  inputVolumeRef?: React.RefObject<number>;
  outputVolumeRef?: React.RefObject<number>;
  size?: number;
  seed?: number; // override default seed for unique sub-agent identity
}

// ── GLSL Shaders (ElevenLabs-compatible) ─────────────────────────────────────

const vertexShader = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const fragmentShader = /* glsl */ `
  uniform float uTime;
  uniform float uAnimation;
  uniform float uInverted;
  uniform float uOffsets[7];
  uniform vec3 uColor1;
  uniform vec3 uColor2;
  uniform float uInputVolume;
  uniform float uOutputVolume;
  uniform float uOpacity;
  uniform sampler2D uPerlinTexture;
  varying vec2 vUv;

  const float PI = 3.14159265358979323846;

  bool drawOval(vec2 polarUv, vec2 polarCenter, float a, float b,
                bool reverseGradient, float softness, out vec4 color) {
    vec2 p = polarUv - polarCenter;
    float oval = (p.x * p.x) / (a * a) + (p.y * p.y) / (b * b);
    float edge = smoothstep(1.0, 1.0 - softness, oval);
    if (edge > 0.0) {
      float gradient = reverseGradient
        ? (1.0 - (p.x / a + 1.0) / 2.0)
        : ((p.x / a + 1.0) / 2.0);
      gradient = mix(0.5, gradient, 0.1);
      color = vec4(vec3(gradient), 0.85 * edge);
      return true;
    }
    return false;
  }

  vec3 colorRamp(float g, vec3 c1, vec3 c2, vec3 c3, vec3 c4) {
    if (g < 0.33) return mix(c1, c2, g * 3.0);
    else if (g < 0.66) return mix(c2, c3, (g - 0.33) * 3.0);
    else return mix(c3, c4, (g - 0.66) * 3.0);
  }

  vec2 hash2(vec2 p) {
    return fract(sin(vec2(dot(p, vec2(127.1, 311.7)),
                          dot(p, vec2(269.5, 183.3)))) * 43758.5453);
  }

  float noise2D(vec2 p) {
    vec2 i = floor(p); vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float n = mix(
      mix(dot(hash2(i + vec2(0,0)), f - vec2(0,0)),
          dot(hash2(i + vec2(1,0)), f - vec2(1,0)), u.x),
      mix(dot(hash2(i + vec2(0,1)), f - vec2(0,1)),
          dot(hash2(i + vec2(1,1)), f - vec2(1,1)), u.x), u.y);
    return 0.5 + 0.5 * n;
  }

  float sharpRing(vec3 d, float t) {
    float n = mix(noise2D(vec2(d.x, t) * 5.0), noise2D(vec2(d.y, t) * 5.0), d.z);
    return 1.0 + (n - 0.5) * 2.5 * 0.3 * 1.5;
  }

  float smoothRing(vec3 d, float t) {
    float n = mix(noise2D(vec2(d.x, t) * 6.0), noise2D(vec2(d.y, t) * 6.0), d.z);
    return 0.9 + (n - 0.5) * 5.0 * 0.2;
  }

  float flow(vec3 d, float t) {
    return mix(texture(uPerlinTexture, vec2(t, d.x / 2.0)).r,
               texture(uPerlinTexture, vec2(t, d.y / 2.0)).r, d.z);
  }

  void main() {
    vec2 uv = vUv * 2.0 - 1.0;
    float radius = length(uv);
    float theta = atan(uv.y, uv.x);
    if (theta < 0.0) theta += 2.0 * PI;

    vec3 decomposed = vec3(
      theta / (2.0 * PI),
      mod(theta / (2.0 * PI) + 0.5, 1.0) + 1.0,
      abs(theta / PI - 1.0)
    );

    float noise = flow(decomposed, radius * 0.03 - uAnimation * 0.2) - 0.5;
    theta += noise * mix(0.08, 0.25, uOutputVolume);

    vec4 color = vec4(1.0);

    float originalCenters[7];
    originalCenters[0] = 0.0;
    originalCenters[1] = 0.5 * PI;
    originalCenters[2] = 1.0 * PI;
    originalCenters[3] = 1.5 * PI;
    originalCenters[4] = 2.0 * PI;
    originalCenters[5] = 2.5 * PI;
    originalCenters[6] = 3.0 * PI;

    float centers[7];
    for (int i = 0; i < 7; i++)
      centers[i] = originalCenters[i] + 0.5 * sin(uTime / 20.0 + uOffsets[i]);

    for (int i = 0; i < 7; i++) {
      float n = texture(uPerlinTexture, vec2(mod(centers[i] + uTime * 0.05, 1.0), 0.5)).r;
      float a = 0.5 + n * 0.3;
      float b = n * mix(3.5, 2.5, uInputVolume);
      bool rev = (mod(float(i), 2.0) > 0.5);

      float distTheta = min(abs(theta - centers[i]),
                        min(abs(theta + 2.0 * PI - centers[i]),
                            abs(theta - 2.0 * PI - centers[i])));

      vec4 ovalColor;
      if (drawOval(vec2(distTheta, radius), vec2(0.0), a, b, rev, 0.6, ovalColor)) {
        color.rgb = mix(color.rgb, ovalColor.rgb, ovalColor.a);
        color.a = max(color.a, ovalColor.a);
      }
    }

    float r1 = sharpRing(decomposed, uTime * 0.1);
    float r2 = smoothRing(decomposed, uTime * 0.1);
    float ir1 = radius + uInputVolume * 0.2;
    float ir2 = radius + uInputVolume * 0.15;
    float a1 = (ir2 >= r1) ? mix(0.2, 0.6, uInputVolume) : 0.0;
    float a2 = smoothstep(r2 - 0.05, r2 + 0.05, ir1) * mix(0.15, 0.45, uInputVolume);
    float totalRing = max(a1, a2);

    color.rgb = 1.0 - (1.0 - color.rgb) * (1.0 - vec3(1.0) * totalRing);

    float luminance = mix(color.r, 1.0 - color.r, uInverted);
    color.rgb = colorRamp(luminance, vec3(0.0), uColor1, uColor2, vec3(1.0));
    color.a *= uOpacity;
    gl_FragColor = color;
  }
`;

// ── Seeded RNG ────────────────────────────────────────────────────────────────

function seededRng(seed: number) {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
  };
}

// ── State → colors ────────────────────────────────────────────────────────────

function getTargetColors(
  state: OrbState,
  isAdmin: boolean
): [THREE.Color, THREE.Color] {
  if (isAdmin) {
    switch (state) {
      case 'listening':
        return [new THREE.Color('#6d28d9'), new THREE.Color('#c4b5fd')];
      case 'thinking':
        return [new THREE.Color('#b45309'), new THREE.Color('#fde68a')];
      case 'talking':
        return [new THREE.Color('#065f46'), new THREE.Color('#6ee7b7')];
      default:
        return [new THREE.Color('#4c1d95'), new THREE.Color('#a78bfa')];
    }
  } else {
    switch (state) {
      case 'listening':
        return [new THREE.Color('#1d4ed8'), new THREE.Color('#93c5fd')];
      case 'thinking':
        return [new THREE.Color('#c2410c'), new THREE.Color('#fed7aa')];
      case 'talking':
        return [new THREE.Color('#065f46'), new THREE.Color('#6ee7b7')];
      default:
        return [new THREE.Color('#1e3a5f'), new THREE.Color('#60a5fa')];
    }
  }
}

// ── Simulated volume per state (mirrors ElevenLabs "auto" mode) ───────────────

function getSimulatedVolumes(state: OrbState, t: number): [number, number] {
  switch (state) {
    case 'listening':
      return [0.55 + Math.sin(t * 3.2) * 0.35, 0.45];
    case 'talking':
      return [0.65 + Math.sin(t * 4.8) * 0.22, 0.75 + Math.sin(t * 3.6) * 0.22];
    case 'thinking': {
      const wander = Math.sin(t * 0.5) * 0.1 + Math.sin(t * 0.3) * 0.08;
      return [0.38 + wander, 0.3 + wander * 0.5];
    }
    default:
      return [0.0, 0.3];
  }
}

// ── The actual Three.js mesh ──────────────────────────────────────────────────

interface OrbMeshProps {
  state: OrbState;
  isAdmin: boolean;
  seed: number;
  inputVolumeRef?: React.RefObject<number>;
  outputVolumeRef?: React.RefObject<number>;
}

/**
 * Procedural fallback for the perlin noise texture the orb shader samples.
 * The original `/perlin-noise.png` asset is missing from public/; generating
 * a 256×256 value-noise texture at runtime avoids the 404 and keeps the orb
 * shader happy.
 */
function buildNoiseTexture(): THREE.DataTexture {
  const size = 256;
  const data = new Uint8Array(size * size * 4);
  for (let i = 0; i < size * size; i++) {
    const v = Math.floor(Math.random() * 256);
    data[i * 4 + 0] = v;
    data[i * 4 + 1] = v;
    data[i * 4 + 2] = v;
    data[i * 4 + 3] = 255;
  }
  const tex = new THREE.DataTexture(data, size, size, THREE.RGBAFormat);
  tex.wrapS = THREE.RepeatWrapping;
  tex.wrapT = THREE.RepeatWrapping;
  tex.minFilter = THREE.LinearMipMapLinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.generateMipmaps = true;
  tex.needsUpdate = true;
  return tex;
}

function OrbMesh({
  state,
  isAdmin,
  seed,
  inputVolumeRef,
  outputVolumeRef,
}: OrbMeshProps) {
  const meshRef = useRef<THREE.Mesh>(null);
  const texture = useMemo(() => buildNoiseTexture(), []);

  const offsets = useMemo(() => {
    const rng = seededRng(seed);
    return Array.from({ length: 7 }, () => rng() * Math.PI * 2);
  }, [seed]);

  const uniformsRef = useRef({
    uTime: { value: 0 },
    uAnimation: { value: 0 },
    uInverted: { value: 1.0 }, // dark mode
    uOffsets: { value: offsets },
    uColor1: { value: new THREE.Color('#4c1d95') },
    uColor2: { value: new THREE.Color('#a78bfa') },
    uInputVolume: { value: 0.0 },
    uOutputVolume: { value: 0.3 },
    uOpacity: { value: 0.0 },
    uPerlinTexture: { value: texture },
  });

  const curInRef = useRef(0);
  const curOutRef = useRef(0.3);

  useEffect(() => {
    uniformsRef.current.uPerlinTexture.value = texture;
  }, [texture]);

  useFrame((_, delta) => {
    const u = uniformsRef.current;
    const t = u.uTime.value;

    // Time
    u.uTime.value += delta * 0.5;

    // Volume
    let targetIn: number;
    let targetOut: number;

    if (
      inputVolumeRef?.current !== undefined &&
      outputVolumeRef?.current !== undefined
    ) {
      targetIn = inputVolumeRef.current ?? 0;
      targetOut = outputVolumeRef.current ?? 0;
    } else {
      [targetIn, targetOut] = getSimulatedVolumes(state, t);
    }

    curInRef.current += (targetIn - curInRef.current) * 0.2;
    curOutRef.current += (targetOut - curOutRef.current) * 0.2;

    u.uInputVolume.value = curInRef.current;
    u.uOutputVolume.value = curOutRef.current;

    // Animation speed driven by output volume
    const animSpeed = 0.1 + (1 - Math.pow(curOutRef.current - 1, 2)) * 0.9;
    u.uAnimation.value += delta * animSpeed;

    // Fade in opacity
    if (u.uOpacity.value < 1)
      u.uOpacity.value = Math.min(1, u.uOpacity.value + delta * 2);

    // Lerp colors
    const [tgt1, tgt2] = getTargetColors(state, isAdmin);
    u.uColor1.value.lerp(tgt1, 0.08);
    u.uColor2.value.lerp(tgt2, 0.08);

    // Write to material
    const mat = meshRef.current?.material as THREE.ShaderMaterial;
    if (mat?.uniforms) {
      Object.assign(mat.uniforms, u);
    }
  });

  return (
    <mesh ref={meshRef}>
      <circleGeometry args={[3.5, 64]} />
      <shaderMaterial
        vertexShader={vertexShader}
        fragmentShader={fragmentShader}
        uniforms={uniformsRef.current}
        transparent
        depthWrite={false}
      />
    </mesh>
  );
}

// ── Public component ──────────────────────────────────────────────────────────

export function OrbVisualizer({
  state,
  isAdmin = false,
  inputVolumeRef,
  outputVolumeRef,
  size,
  seed: seedProp,
}: OrbVisualizerProps) {
  const seed = seedProp ?? (isAdmin ? 42 : 7);

  return (
    <div
      style={
        size != null
          ? { width: size, height: size }
          : { width: '100%', height: '100%' }
      }
      className="select-none"
    >
      <Canvas
        camera={{ position: [0, 0, 5], fov: 75 }}
        gl={{ antialias: true, alpha: true }}
        style={{ background: 'transparent' }}
      >
        <OrbMesh
          state={state}
          isAdmin={isAdmin}
          seed={seed}
          inputVolumeRef={inputVolumeRef}
          outputVolumeRef={outputVolumeRef}
        />
      </Canvas>
    </div>
  );
}
