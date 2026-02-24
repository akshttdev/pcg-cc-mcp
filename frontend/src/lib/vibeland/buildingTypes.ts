export type BuildingType =
  | 'dev-tower'
  | 'creative-studio'
  | 'infrastructure'
  | 'research'
  | 'command'
  | 'bank'
  | 'casino'
  | 'gallery';

const KEYWORD_MAP: Record<BuildingType, string[]> = {
  'dev-tower': ['mcp', 'rs', 'code', 'api', 'frontend', 'backend', 'builder', 'web', 'app', 'site'],
  'creative-studio': ['brand', 'design', 'studio', 'glyph', 'creative', 'film', 'cinema', 'music'],
  infrastructure: ['ducknet', 'comfy', 'distribution', 'linux', 'infra', 'ops', 'network', 'server', 'cloud'],
  research: ['extract', 'lab', 'research', 'ai', 'agent', 'ml', 'twin', 'data', 'analytics'],
  command: ['command', 'control', 'hq', 'monsters', 'headquarters', 'hub'],
  bank: ['bank', 'finance', 'vault'],
  casino: ['casino', 'verse', 'jungle'],
  gallery: ['gallery', 'art', 'society', 'fine', 'museum'],
};

// Named project overrides — specific projects always map to a given type
const NAME_OVERRIDES: Partial<Record<string, BuildingType>> = {
  'fine art society': 'gallery',
  'veritwin': 'bank',
  'jungleverse': 'casino',
  'media monsters hq': 'command',
  'sirak studios': 'creative-studio',
};

export function getBuildingType(name: string): BuildingType {
  const lower = name.toLowerCase();

  if (lower === 'pcg-cc-mcp' || lower.includes('command-center') || lower.includes('pcg command')) {
    return 'command';
  }

  // Check named overrides first
  if (NAME_OVERRIDES[lower]) {
    return NAME_OVERRIDES[lower]!;
  }

  for (const [type, keywords] of Object.entries(KEYWORD_MAP) as [BuildingType, string[]][]) {
    if (type === 'command') continue;
    if (keywords.some((keyword) => lower.includes(keyword))) {
      return type;
    }
  }

  return 'dev-tower';
}

export interface BuildingTheme {
  baseColor: string;
  accentColor: string;
  hologramColor: string;
  doorColor: string;
  labelColor: string;
  interior: {
    wallColor: string;
    floorColor: string;
    glowColor: string;
    agentColor: string;
  };
}

export const BUILDING_THEMES: Record<BuildingType, BuildingTheme> = {
  'dev-tower': {
    baseColor: '#0a2f4a',
    accentColor: '#00b4ff',
    hologramColor: '#00ffff',
    doorColor: '#13d2ff',
    labelColor: '#9de5ff',
    interior: {
      wallColor: '#071423',
      floorColor: '#0b1f33',
      glowColor: '#00c6ff',
      agentColor: '#10d7ff',
    },
  },
  'creative-studio': {
    baseColor: '#4a2f0a',
    accentColor: '#ff9d3c',
    hologramColor: '#ffca7a',
    doorColor: '#ffae4a',
    labelColor: '#ffe3c4',
    interior: {
      wallColor: '#1f1405',
      floorColor: '#2a1c08',
      glowColor: '#ffb347',
      agentColor: '#ff954f',
    },
  },
  infrastructure: {
    baseColor: '#1a1a1a',
    accentColor: '#ff4242',
    hologramColor: '#ff4d4d',
    doorColor: '#ff6262',
    labelColor: '#ffd7d7',
    interior: {
      wallColor: '#050505',
      floorColor: '#0c0c0c',
      glowColor: '#ff3d3d',
      agentColor: '#ff5f5f',
    },
  },
  research: {
    baseColor: '#2a0a4a',
    accentColor: '#c267ff',
    hologramColor: '#aa00ff',
    doorColor: '#d28bff',
    labelColor: '#f4d2ff',
    interior: {
      wallColor: '#12031f',
      floorColor: '#1d0431',
      glowColor: '#c467ff',
      agentColor: '#f067ff',
    },
  },
  command: {
    baseColor: '#0b2035',
    accentColor: '#00ffff',
    hologramColor: '#00ffff',
    doorColor: '#b5f5ff',
    labelColor: '#ffffff',
    interior: {
      wallColor: '#07111a',
      floorColor: '#081c28',
      glowColor: '#00f0ff',
      agentColor: '#00c6ff',
    },
  },
  bank: {
    baseColor: '#d4c9a8',
    accentColor: '#b8a46e',
    hologramColor: '#f0e6c8',
    doorColor: '#c8b87a',
    labelColor: '#f5f0e0',
    interior: {
      wallColor: '#c2b89a',
      floorColor: '#a89868',
      glowColor: '#d4c090',
      agentColor: '#e0d0a0',
    },
  },
  casino: {
    baseColor: '#1a0a2e',
    accentColor: '#ff2d78',
    hologramColor: '#ffe040',
    doorColor: '#ff6030',
    labelColor: '#ffe880',
    interior: {
      wallColor: '#12061e',
      floorColor: '#1e0a30',
      glowColor: '#ff3080',
      agentColor: '#ffcc00',
    },
  },
  gallery: {
    baseColor: '#e8e2d6',
    accentColor: '#c8a84a',
    hologramColor: '#fff4d0',
    doorColor: '#ddc070',
    labelColor: '#ffffff',
    interior: {
      wallColor: '#f0ece4',
      floorColor: '#d8d0bc',
      glowColor: '#ffe8a0',
      agentColor: '#ffd060',
    },
  },
};
