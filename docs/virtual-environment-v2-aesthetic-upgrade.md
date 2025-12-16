# Virtual Environment V2: Monumental Architecture & Avatar Systems

## Executive Summary

Transform the PCG virtual environment from a **prototype visualization** into an **immersive architectural workspace** where humans and AI agents collaborate inside monumental structures. This upgrade prioritizes scale, presence, and spatial narrative before functional integration.

---

## Vision Statement

**"Each project is not a cube—it's a building you can enter."**

The virtual environment should evoke the feeling of walking through a futuristic campus where:
- **Projects** are monolithic structures (think Tron's Recognizer ships, Blade Runner megastructures)
- **The Command Center** is a glass citadel at the origin—the spawn point and NORA's residence
- **User Avatar** provides embodied presence (first-person or third-person perspective)
- **NORA** is a holographic humanoid guide, not just a floating mesh
- **Agents** appear as distinct avatars performing visible work inside project buildings
- **Scale** conveys enterprise gravity—you should feel small walking between projects

---

## Current State Analysis

### What Exists (V1 Prototype)
```typescript
// frontend/src/pages/virtual-environment.tsx
- Infinite grid (orange/purple gradient)
- Small project cubes (2x2x2 units, radius 18-40 units from origin)
- Basic NORA avatar (sphere + torus, 1.5 units tall)
- Simple Command Center (4x4x2 glass box)
- Static camera (OrbitControls only)
- No user avatar
- No interiors
- Flat lighting (ambient + directional)
```

### Visual Problems
1. **Scale mismatch**: Cubes feel like toys, not buildings
2. **No entry points**: Can't go "inside" projects
3. **NORA underdeveloped**: Basic geometry, no personality
4. **No user presence**: Disembodied camera floating
5. **Command Center underwhelming**: Doesn't feel like a hub
6. **Lighting flat**: No atmosphere or depth

---

## Design Targets

### 1. Monumental Project Buildings

#### Scale Specifications
```
Current: 2×2×2 unit cubes
Target:  20×40×20 unit structures (10x scale increase)

Visual reference: Think skyscrapers, not boxes
- Height: 40 units (approx 10-story building equivalent)
- Footprint: 20×20 units (large enough to feel substantial)
- Spacing: 120+ unit radius (prevent crowding)
```

#### Architectural Variants (Per Project Type)
Design 3-5 distinct building types that auto-assign based on project characteristics:

**Type A: Development Tower** (for code-heavy projects like `pcg-cc-mcp`, `duck-rs`)
- Vertical emphasis (40 units tall)
- Glowing code-like patterns on facade (animated shaders)
- Multiple floors visible through glass panels
- Data streams flowing vertically (particle effects)
- Landing pad on roof for agent arrivals

**Type B: Creative Studio** (for design/content projects like `jungleverse`, brand assets)
- Horizontal spread (20 units tall, wider footprint)
- Holographic displays on exterior walls
- Warm lighting (amber/gold tones vs cool blue of dev towers)
- Rotating showcase of project artifacts
- Garden terrace with seating areas

**Type C: Infrastructure Hub** (for system projects like `ducknet`, `ComfyUI`)
- Fortress-like, solid materials
- Server rack aesthetic (stacked modules)
- Pulsing energy cores visible through vents
- Minimal windows, more about function
- Underground access points

**Type D: Research Facility** (for experimental/AI projects)
- Organic curves, less geometric
- Translucent materials (can see activity inside)
- Floating components (anti-gravity effect)
- Bioluminescent accents
- Observatory dome on top

**Type E: Command & Control** (reserved for special projects)
- Pyramid or ziggurat shape
- Elevated on platform
- 360° observation decks
- Central spire with beacon
- Bridge connections to nearby buildings

#### Material System
```typescript
// Physically-based materials for realism
import { MeshPhysicalMaterial } from 'three';

const buildingMaterials = {
  glass: {
    transmission: 0.9,
    thickness: 0.5,
    roughness: 0.05,
    metalness: 0.1,
    envMapIntensity: 1.5,
    color: '#0a4a6e', // Tron blue tint
  },
  metal: {
    roughness: 0.3,
    metalness: 0.95,
    color: '#2a3f5f',
    emissive: '#004080',
    emissiveIntensity: 0.2,
  },
  holographic: {
    transparent: true,
    opacity: 0.6,
    emissive: '#00ffff',
    emissiveIntensity: 0.8,
    blending: AdditiveBlending, // Glow effect
  },
};
```

#### Entry Points & Interiors
Each building must have:
1. **Main entrance**: Large doorway (4 units wide, 8 units tall)
2. **Interior space**: Simplified room with task visualization
   - Task boards floating as holograms
   - Agent workstations (desks/terminals)
   - Central column showing project metrics
   - Exit portal back to grid
3. **Transition**: Smooth camera zoom/fade when entering
4. **Signage**: Project name floating above entrance (3D text, 5 units tall)

---

### 2. Command Center Citadel

#### Architectural Vision
The PCG Command Center is the **spawn point** and **NORA's office**—it must be the most impressive structure.

**Design Specifications:**
```
Shape: Octagonal glass pavilion with central spire
Dimensions: 30×30 base, 50 units tall (tallest structure)
Position: [0, 0, 0] (world origin)
Elevation: Raised on platform 5 units above grid

Components:
- Central spire (10 units diameter, 50 tall) - glowing energy core
- Observation ring (20 units diameter) at 30 units height
- Ground floor (open plaza, 30×30)
- NORA's holographic platform (center of ground floor)
- Surrounding moat/channel (glowing cyan liquid)
- Bridge access points (4 bridges, one per cardinal direction)
```

#### Interior Layout
```
Ground Floor (0-10 units):
- Open plaza with holographic displays
- NORA's central platform (5 unit diameter, raised 0.5 units)
- 4 console stations around perimeter (for human operators)
- Transparent floor panels showing data streams below
- Ambient holographic particles floating

Observation Ring (30-35 units):
- 360° glass walls
- View of entire project grid
- Strategic planning table (holographic map of all projects)
- Elevator access from ground floor

Spire Core (10-50 units):
- Energy beam shooting upward (visible from anywhere)
- Pulsing with "heartbeat" of system activity
- Particle emissions at intervals
- Acts as navigation beacon
```

#### Lighting & Effects
```typescript
// Dramatic lighting for Command Center
const commandCenterLights = [
  new SpotLight(0x00ffff, 3, 50, Math.PI / 4), // From spire top
  new PointLight(0xffffff, 2, 30), // NORA's platform
  new HemisphereLight(0x0080ff, 0xff8000, 0.5), // Sky/ground fill
  new RectAreaLight(0x0066ff, 1, 30, 30), // Floor glow
];

// Volumetric fog/god rays
<Volumetric
  geometry={<cylinderGeometry args={[15, 15, 50, 32, 1, true]} />}
  position={[0, 25, 0]}
  opacity={0.15}
/>
```

---

### 3. NORA Avatar Refinement

#### Current Issues
- Too small (1.5 units tall)
- Basic geometry (sphere + torus)
- No personality or movement
- No eye contact or gaze direction

#### Target Design

**Option A: Holographic Humanoid (Recommended)**
```
Style: Cortana (Halo) meets Joi (Blade Runner 2049)
Height: 6 units (human-scale, slightly elevated on platform)
Form: Female-presenting humanoid silhouette
Material: Translucent cyan hologram with scan lines
Animation: Gentle floating/breathing, subtle particles

Features:
- Head with facial geometry (eyes, mouth for lip-sync)
- Body outline (don't need full detail, suggest form)
- Glowing core at chest (heart metaphor)
- Hands that gesture when speaking
- Hair-like particle streams
```

**Technical Implementation:**
```typescript
// Use FBX/GLB humanoid model with custom shader
import { useGLTF } from '@react-three/drei';

function NoraAvatar() {
  const model = useGLTF('/models/nora-base.glb');

  return (
    <group position={[0, 3, 0]}>
      <mesh geometry={model.scene.children[0].geometry}>
        <shaderMaterial
          vertexShader={hologramVert}
          fragmentShader={hologramFrag}
          transparent
          uniforms={{
            time: { value: 0 },
            scanlineSpeed: { value: 2.0 },
            opacity: { value: 0.7 },
            color: { value: new Color(0x00ffff) },
          }}
        />
      </mesh>

      {/* Holographic projection base */}
      <mesh position={[0, -3, 0]} rotation={[-Math.PI/2, 0, 0]}>
        <ringGeometry args={[2.5, 3, 64]} />
        <meshBasicMaterial color={0x00ffff} transparent opacity={0.5} />
      </mesh>

      {/* Ambient particles */}
      <Points>
        <sphereGeometry args={[4, 32, 32]} />
        <pointsMaterial
          size={0.05}
          color={0x00ffff}
          transparent
          opacity={0.6}
          sizeAttenuation
        />
      </Points>
    </group>
  );
}
```

**Animation System:**
```typescript
// Idle animations (always running)
- Float: sine wave bobbing (amplitude: 0.2 units, period: 3s)
- Rotate: slow spin on Y axis (0.05 rad/s)
- Breathe: scale pulse (0.98 - 1.02, period: 4s)
- Particles: orbit around body (radius: 4 units)

// Speaking animations (triggered by voice output)
- Head tilt toward user
- Mouth open/close sync with audio
- Hand gestures (3-4 preset poses)
- Increased particle activity
- Glow intensity boost

// Listening animations (triggered by voice input)
- Face toward microphone source
- Particle convergence toward head
- Color shift to green (acknowledging)
- Subtle head nod
```

**Option B: Abstract Intelligence (Alternative)**
```
Style: AI consciousness, not humanoid
Form: Geometric core with orbiting elements
- Central dodecahedron (2 units diameter)
- 12 orbiting data fragments
- Connecting light beams
- Voice emanates from center

Pros: Unique, no uncanny valley
Cons: Less relatable, harder to convey emotion
```

**Recommendation**: **Use Option A** (Holographic Humanoid) for these reasons:
1. Humans instinctively read faces and body language
2. Gestures reinforce voice commands
3. Eye contact builds trust with AI
4. Aligns with "executive assistant" persona
5. Easier to animate emotions

---

### 4. User Avatar System

#### First-Person vs Third-Person
**Recommended**: **Hybrid approach**
- Default: Third-person (see your avatar)
- Toggle: First-person (immersive VR mode)
- Shortcut: Press "V" to switch views

#### Avatar Design (Third-Person)

**Style: Minimalist Tron Suit**
```
Form: Humanoid silhouette (6 units tall)
Material: Dark matte body with glowing circuit lines
Color: User-customizable accent color (default: orange)
Features:
- Helmeted head (no face, privacy-preserving)
- Glowing visor slit
- Light trails when moving
- Jetpack or hover-disc for traversal
```

**Implementation:**
```typescript
function UserAvatar({ position, rotation, color = 0xff8000 }) {
  return (
    <group position={position} rotation={rotation}>
      {/* Body - use simple capsule geometry */}
      <mesh position={[0, 3, 0]}>
        <capsuleGeometry args={[0.5, 2, 8, 16]} />
        <meshStandardMaterial
          color={0x0a0a0a}
          emissive={color}
          emissiveIntensity={0.3}
          roughness={0.8}
        />
      </mesh>

      {/* Head */}
      <mesh position={[0, 4.5, 0]}>
        <sphereGeometry args={[0.6, 16, 16]} />
        <meshStandardMaterial color={0x0a0a0a} />
      </mesh>

      {/* Visor glow */}
      <mesh position={[0, 4.5, 0.5]}>
        <planeGeometry args={[0.8, 0.2]} />
        <meshBasicMaterial color={color} transparent opacity={0.8} />
      </mesh>

      {/* Circuit lines on body */}
      <Line
        points={[[0, 2, 0.5], [0, 4, 0.5], [0.3, 4.5, 0.5]]}
        color={color}
        lineWidth={2}
      />

      {/* Light trail when moving */}
      <Trail
        width={0.5}
        length={10}
        color={new Color(color)}
        attenuation={(t) => t * t}
      >
        <mesh />
      </Trail>
    </group>
  );
}
```

#### Camera System

**Third-Person Camera:**
```typescript
// Follow camera behind/above avatar
const cameraOffset = [0, 8, 12]; // X, Y, Z from avatar
const lookAtOffset = [0, 4, 0]; // Look at head, not feet

// Smooth follow (lerp)
camera.position.lerp(
  avatarPosition.clone().add(cameraOffset),
  0.1 // Smoothing factor
);

// Collision detection (don't clip through buildings)
raycaster.set(avatarPosition, cameraDirection);
const intersects = raycaster.intersectObjects(buildings);
if (intersects.length > 0 && intersects[0].distance < 12) {
  camera.position.lerp(intersects[0].point, 0.2); // Pull closer
}
```

**First-Person Camera:**
```typescript
// Mount camera at avatar's head position
camera.position.copy(avatarPosition).add([0, 4.5, 0]);
camera.rotation.copy(avatarRotation);

// Hide avatar body (or just show hands for immersion)
avatarMesh.visible = false;
```

#### Movement System

**Keyboard Controls:**
```
WASD: Move forward/left/back/right
Shift: Sprint (2x speed)
Space: Jump/boost upward
Ctrl: Crouch/descend
Mouse: Look around (first-person) or orbit (third-person)
V: Toggle camera view
E: Interact (when near building entrance)
```

**Movement Physics:**
```typescript
const moveSpeed = 0.5; // Units per frame
const sprintMultiplier = 2.0;
const acceleration = 0.1;
const deceleration = 0.95;

// Smooth acceleration
velocity.x += (input.left - input.right) * acceleration;
velocity.z += (input.forward - input.backward) * acceleration;

// Apply friction
velocity.multiplyScalar(deceleration);

// Update position
avatarPosition.add(velocity);

// Ground collision (keep on grid)
if (avatarPosition.y < 0) {
  avatarPosition.y = 0;
  velocity.y = 0;
}
```

---

### 5. Enhanced Lighting & Atmosphere

#### Global Illumination
```typescript
// Replace flat lighting with dramatic environment
import { Environment, Lightformer } from '@react-three/drei';

<Environment preset="night">
  {/* Custom HDR environment */}
  <Lightformer
    intensity={4}
    position={[0, 50, 0]}
    scale={[100, 100, 1]}
    color="#0080ff"
    form="ring"
  />
</Environment>

// Fog for depth perception
<fog attach="fog" args={['#0a0a1a', 50, 200]} />

// Volumetric lighting (god rays from Command Center)
<Volumetric
  geometry={<coneGeometry args={[20, 50, 32, 1, true]} />}
  position={[0, 25, 0]}
  rotation={[Math.PI, 0, 0]}
  opacity={0.1}
  color="#00ffff"
/>
```

#### Dynamic Lighting Per Building
```typescript
// Each building emits light based on activity
function ProjectBuilding({ activityLevel = 0.5, projectType }) {
  const lightIntensity = 1 + activityLevel * 2; // 1-3 range
  const lightColor = projectType === 'dev' ? 0x0080ff : 0xff8000;

  return (
    <group>
      {/* Building mesh */}
      <mesh>...</mesh>

      {/* Dynamic area light */}
      <RectAreaLight
        intensity={lightIntensity}
        color={lightColor}
        width={20}
        height={40}
        position={[0, 20, 0]}
      />

      {/* Spotlight from roof */}
      <SpotLight
        intensity={2}
        angle={Math.PI / 6}
        penumbra={0.5}
        position={[0, 40, 0]}
        target-position={[0, 0, 0]}
      />
    </group>
  );
}
```

#### Particle Systems
```typescript
// Ambient particles throughout scene
function AmbientParticles({ count = 1000 }) {
  const particlesRef = useRef();

  useFrame((state) => {
    // Drift particles slowly
    particlesRef.current.rotation.y += 0.0001;
  });

  return (
    <Points ref={particlesRef} limit={count}>
      <sphereGeometry args={[100, 32, 32]} />
      <pointsMaterial
        size={0.1}
        color={0x00ffff}
        transparent
        opacity={0.3}
        sizeAttenuation
        blending={AdditiveBlending}
      />
    </Points>
  );
}
```

---

## Implementation Roadmap

### Phase 1: Scale & Structure (Week 1)
**Goal**: Increase project building scale and add Command Center citadel

**Tasks:**
1. **Upgrade project buildings** (2 days)
   - Increase cube size from 2x2x2 → 20x40x20
   - Increase grid radius from 18-40 → 120+
   - Add basic architectural geometry (windows, floors)
   - Implement building type assignment logic
   - Add entry doors (non-functional placeholder)

2. **Build Command Center citadel** (2 days)
   - Model octagonal pavilion with spire
   - Add holographic platform for NORA
   - Implement bridge access points
   - Add surrounding moat/channel effect
   - Integrate volumetric lighting for spire

3. **Upgrade grid & environment** (1 day)
   - Extend grid to 500+ unit radius
   - Add fog for depth perception
   - Implement advanced lighting (Environment, HDR)
   - Add ambient particle systems
   - Optimize rendering (LOD for distant buildings)

**Success Criteria:**
- [ ] Buildings feel monumental (40 units tall)
- [ ] Command Center is visually dominant
- [ ] User can see 10+ buildings without performance drop
- [ ] Lighting creates atmosphere (not flat)

---

### Phase 2: Avatars & Presence (Week 2)
**Goal**: Add user avatar and refine NORA hologram

**Tasks:**
1. **User avatar system** (3 days)
   - Create Tron-style humanoid mesh
   - Implement WASD + mouse controls
   - Add third-person follow camera
   - Add smooth movement physics
   - Implement camera collision detection
   - Add light trails when moving

2. **NORA avatar refinement** (2 days)
   - Replace sphere with humanoid hologram mesh
   - Implement idle animations (float, breathe, particles)
   - Add facial geometry (eyes, mouth for future lip-sync)
   - Position on central platform in Command Center
   - Add holographic projection base effect
   - Implement gaze direction (look at user)

3. **Camera system polish** (1 day)
   - Add first-person mode toggle (V key)
   - Smooth transitions between views
   - Implement cinematic camera paths (optional)
   - Add "Focus" mode (zoom to specific building)

**Success Criteria:**
- [ ] User feels embodied in the space
- [ ] Avatar moves smoothly with responsive controls
- [ ] NORA looks like intelligent holographic assistant
- [ ] Camera never clips through geometry

---

### Phase 3: Interiors & Details (Week 3)
**Goal**: Make buildings explorable and add visual polish

**Tasks:**
1. **Building interiors** (3 days)
   - Create interior room template (30x30x10 units)
   - Add door interaction (E key to enter)
   - Implement camera transition (zoom in, fade, load interior)
   - Add interior furnishings (terminals, holographic boards)
   - Add exit portal back to grid
   - Instance interiors per building type

2. **Visual polish** (2 days)
   - Add building-specific materials (glass, metal, holographic)
   - Implement animated shaders (scan lines, data flows)
   - Add rooftop details (landing pads, antennas)
   - Add 3D signage above entrances (project names)
   - Add dynamic lights per building (based on activity)

3. **Performance optimization** (1 day)
   - Implement LOD (level of detail) for distant buildings
   - Add occlusion culling
   - Optimize particle counts
   - Reduce draw calls (instanced meshes)
   - Add loading screen for initial load

**Success Criteria:**
- [ ] User can enter at least 3 different building interiors
- [ ] Each building type has distinct visual identity
- [ ] 60 FPS maintained with 20+ buildings visible
- [ ] No pop-in or jarring LOD transitions

---

### Phase 4: Animation & Life (Week 4)
**Goal**: Add movement and activity to make space feel alive

**Tasks:**
1. **Agent avatars** (2 days)
   - Create 3-4 distinct agent avatar meshes
   - Position agents inside building interiors
   - Add idle animations (typing, walking, gesturing)
   - Add floating name tags (show agent type)
   - Color-code by role (coding=blue, research=purple, etc.)

2. **NORA animations** (2 days)
   - Implement speaking animations (mouth, gestures)
   - Add listening animations (head turn, particle convergence)
   - Create 4-5 gesture presets (point, wave, nod, etc.)
   - Add audio-reactive particle intensity
   - Implement gaze tracking (look at user when approached)

3. **Environmental animations** (1 day)
   - Add data stream particles flowing up buildings
   - Implement pulsing emissive materials (heartbeat effect)
   - Add occasional spotlight sweeps across grid
   - Add floating holographic text (project metrics)
   - Add ambient sound design (hum, electrical buzz)

**Success Criteria:**
- [ ] Space feels inhabited (agents visible working)
- [ ] NORA displays personality through animation
- [ ] Visual interest at all times (not static)
- [ ] Animations don't hurt performance (<10% FPS drop)

---

## Technical Architecture

### File Structure
```
frontend/src/
├── pages/
│   └── virtual-environment.tsx         # Main scene container
├── components/
│   └── virtual-world/
│       ├── Grid.tsx                    # Infinite grid
│       ├── CommandCenter.tsx           # Central citadel
│       ├── ProjectBuilding.tsx         # Modular buildings
│       ├── BuildingInterior.tsx        # Interior spaces
│       ├── NoraAvatar.tsx              # NORA hologram
│       ├── UserAvatar.tsx              # Player character
│       ├── AgentAvatar.tsx             # AI agent representations
│       ├── CameraController.tsx        # Camera system
│       ├── LightingRig.tsx             # Scene lighting
│       ├── ParticleSystems.tsx         # Ambient particles
│       └── shaders/
│           ├── hologram.vert           # Holographic vertex shader
│           ├── hologram.frag           # Holographic fragment shader
│           ├── scanlines.frag          # Scan line effect
│           └── dataflow.vert           # Animated data streams
├── hooks/
│   └── virtual-world/
│       ├── useMovement.ts              # WASD controls
│       ├── useCameraFollow.ts          # Third-person camera
│       ├── useInteraction.ts           # Building entry/exit
│       └── usePerformance.ts           # LOD & optimization
└── lib/
    └── virtual-world/
        ├── buildingTypes.ts            # Architecture definitions
        ├── materials.ts                # Shared materials
        └── constants.ts                # World constants (scale, speeds)
```

### Performance Targets
```
Minimum: 30 FPS on mid-range GPU (GTX 1060)
Target: 60 FPS on modern GPU (RTX 3060)
Maximum: 120 FPS for high-end (RTX 4080)

Draw calls: < 200 per frame
Triangles: < 2M visible per frame
Texture memory: < 500MB
Particle count: < 50k active
```

### Dependencies (Add to package.json)
```json
{
  "@react-three/fiber": "^8.15.0",
  "@react-three/drei": "^9.92.0",
  "@react-three/postprocessing": "^2.15.0",
  "three": "^0.160.0",
  "three-stdlib": "^2.28.0",
  "leva": "^0.9.35"
}
```

---

## Design Assets Needed

### 3D Models (GLB/FBX format)
1. **NORA base mesh** - Humanoid hologram template
   - Low-poly (< 5k triangles)
   - Rigged for animation (mixamo-compatible)
   - T-pose default

2. **User avatar base** - Tron-style suit
   - Low-poly (< 3k triangles)
   - Rigged for animation
   - Modular (swap helmet, torso, legs)

3. **Agent avatars** (3-4 variants)
   - Abstract geometric forms
   - Color-coded by role
   - < 2k triangles each

### Textures
1. **Building materials** (2048x2048 PBR sets)
   - Glass (albedo, normal, roughness, metallic, emissive)
   - Metal panels (albedo, normal, roughness, metallic)
   - Holographic surfaces (emissive, opacity)

2. **Grid texture** (1024x1024)
   - Tron-style circuit pattern
   - Seamless tileable

### Audio Assets
1. **Ambient soundscape** (looping, 2min)
   - Low electrical hum
   - Distant data processing sounds
   - Spatial audio (stereo)

2. **UI sounds** (short, < 1s each)
   - Building enter/exit whoosh
   - Camera zoom
   - Avatar footsteps (optional)
   - Hologram flicker

---

## Visual References

### Architectural Style
- **Tron Legacy** - Grid, color palette, glass materials
- **Blade Runner 2049** - Scale, lighting, holographic interfaces
- **Cyberpunk 2077** - Building density, neon accents
- **Mirror's Edge** - Clean geometry, color coding
- **Control (game)** - Brutalist architecture, floating objects

### Avatar Style
- **Tron** - Light suits, minimal faces
- **Daft Punk** - Helmets, reflective visors
- **Ghost in the Shell** - Holographic effects
- **Halo (Cortana)** - AI hologram aesthetic

### Lighting & Atmosphere
- **Tron Legacy** - Orange vs blue color dichotomy
- **Blade Runner** - Volumetric fog, god rays
- **AKIRA** - Neon reflections, wet surfaces
- **Ghost in the Shell** - Teal/magenta palette

---

## Success Metrics

### Quantitative
- [ ] 60 FPS maintained with 20+ buildings
- [ ] Load time < 5 seconds (initial scene)
- [ ] User can navigate 200+ units without stutter
- [ ] Avatar responds to input < 16ms (60 FPS)
- [ ] Buildings visible from 500+ units away

### Qualitative
- [ ] Users describe space as "immersive"
- [ ] NORA feels like a character, not a prop
- [ ] Scale makes users feel sense of enterprise gravity
- [ ] Users instinctively try to explore buildings
- [ ] Camera never frustrates (good collision, smooth follow)

---

## Risk Mitigation

### Performance Risks
**Risk**: Too many buildings cause FPS drop
**Mitigation**: LOD system, render only buildings in view frustum

**Risk**: Particle systems tank performance
**Mitigation**: GPU instancing, limit active particles to 10k

**Risk**: Shader complexity causes lag
**Mitigation**: Simplified shaders for mobile, progressive enhancement

### UX Risks
**Risk**: Users get lost in space
**Mitigation**: Minimap, compass, Command Center beacon always visible

**Risk**: Movement feels sluggish or floaty
**Mitigation**: Tunable movement constants, playtesting

**Risk**: Camera clips through buildings
**Mitigation**: Raycasting collision, smooth pull-back

### Technical Risks
**Risk**: Three.js bundle size too large
**Mitigation**: Code-split 3D page, lazy-load models

**Risk**: Models don't load on slow connections
**Mitigation**: Low-poly fallback meshes, loading progress bar

---

## Next Actions

### Immediate (Today)
1. Create file structure (`components/virtual-world/`)
2. Extract Command Center into separate component
3. Increase project cube scale 10x (quick test)
4. Add fog and better lighting (quick visual win)

### This Week
1. Implement full Command Center citadel
2. Design and add 2 building types (dev tower + creative studio)
3. Create user avatar mesh and movement system
4. Refine NORA hologram (humanoid form)

### Next Sprint
1. Build interior spaces (at least 3 variants)
2. Add agent avatars inside buildings
3. Implement all 5 building types
4. Performance optimization pass

---

## Open Questions

1. **Building Count**: How many projects will we typically display? (affects LOD strategy)
2. **Multiplayer**: Should we plan for multiple user avatars eventually?
3. **VR Support**: Is WebXR/VR a future requirement? (affects camera/interaction design)
4. **Mobile**: Do we need mobile/tablet support? (affects controls and performance targets)
5. **Branding**: Any specific PCG visual identity requirements? (colors, logos, typography)
6. **Audio**: Priority on spatial audio vs silent experience?
7. **Customization**: Can users customize avatar appearance/color?

---

## Conclusion

This upgrade transforms the virtual environment from a **proof-of-concept visualization** into a **spatial workspace** worthy of the PCG Command Center vision. By prioritizing scale, presence, and atmosphere, we create an environment where collaboration between humans and AI agents feels tangible and cinematic—before wiring up any backend functionality.

**Key Principle**: *Make it feel like a place you want to explore, before making it functional.*
