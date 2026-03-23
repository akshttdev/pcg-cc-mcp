import { useMemo } from 'react';
import { Compass } from 'lucide-react';
import { ZONE_RING_RADIUS, STATIC_ZONES, zonePosition } from './constants';
import type { MiniMapProps } from './types';

export function MiniMap({ projects, selectedProject, userPosition, size = 220, zones = STATIC_ZONES }: MiniMapProps) {
  const [userX, , userZ] = userPosition;
  // Scale to show the zone ring + some margin
  const worldExtent = ZONE_RING_RADIUS + 60;
  const margin = 14;
  const scale = (size / 2 - margin) / worldExtent;
  const patternId = useMemo(() => `mini-map-grid-${Math.random().toString(36).slice(2)}`, []);

  const toMapX = (value: number) => size / 2 + value * scale;
  const toMapY = (value: number) => size / 2 + value * scale;

  return (
    <div>
      <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.3em] text-amber-200">
        <span>Global Map</span>
        <Compass className="h-4 w-4" />
      </div>
      <div className="relative rounded-lg border border-amber-500/30 bg-black/50 p-2">
        <svg width={size} height={size} className="rounded bg-[#050403]">
          <defs>
            <pattern id={patternId} width="16" height="16" patternUnits="userSpaceOnUse">
              <path d="M 16 0 L 0 0 0 16" stroke="#3f2c16" strokeWidth="0.5" fill="none" />
            </pattern>
          </defs>
          <rect width={size} height={size} fill="#050403" />
          <rect width={size} height={size} fill={`url(#${patternId})`} opacity={0.7} />

          {/* Zone ring guide */}
          <circle
            cx={size / 2}
            cy={size / 2}
            r={ZONE_RING_RADIUS * scale}
            fill="none"
            stroke="#1a3a2a"
            strokeWidth={1}
            strokeDasharray="4 4"
          />

          {/* Zone landmarks */}
          {zones.map((zone, idx) => {
            const [zx, zz] = zonePosition(idx, zones.length, ZONE_RING_RADIUS);
            const mx = toMapX(zx);
            const mz = toMapY(zz);
            const col = zone.color;
            return (
              <g key={zone.space_name}>
                <circle cx={mx} cy={mz} r={7} fill={col} opacity={0.25} />
                <circle cx={mx} cy={mz} r={4} fill={col} opacity={0.9} />
              </g>
            );
          })}

          {/* Project buildings (small dots) */}
          {projects.map((project) => {
            const x = toMapX(project.position[0]);
            const y = toMapY(project.position[2]);
            const isSelected = selectedProject?.name === project.name;
            return (
              <circle
                key={project.name}
                cx={x}
                cy={y}
                r={isSelected ? 5 : 3}
                fill={isSelected ? '#fbbf24' : '#38bdf8'}
                opacity={isSelected ? 0.95 : 0.5}
              />
            );
          })}

          {/* PCG Command Center (center) */}
          <circle cx={size / 2} cy={size / 2} r={6} fill="#00ffff" opacity={0.9} />
          <circle cx={size / 2} cy={size / 2} r={12} fill="none" stroke="#00ffff" strokeWidth={0.8} opacity={0.4} />

          {/* Player position */}
          <circle cx={toMapX(userX)} cy={toMapY(userZ)} r={5} fill="#f472b6" stroke="#ffffff" strokeWidth={1} />
        </svg>
        <span className="pointer-events-none absolute right-4 top-3 text-xs font-semibold text-amber-200">N</span>
      </div>
      {/* Zone legend */}
      <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1">
        {zones.map((z) => (
          <span key={z.space_name} className="flex items-center gap-1 text-[9px] text-amber-200/70">
            <span style={{ backgroundColor: z.color }} className="inline-block h-2 w-2 rounded-full" />
            {z.space_name}
          </span>
        ))}
      </div>
    </div>
  );
}
