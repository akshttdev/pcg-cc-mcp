import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { useMultiplayerStore } from '@/stores/useMultiplayerStore';
import { virtualSpacesApi } from '@/lib/api';
import { useProjectList } from '@/hooks/api/useProjectList';
import { useAuth } from '@/contexts/AuthContext';
import { ENTRY_TRIGGER_DISTANCE, BUILDING_HALF_LENGTH } from '@/lib/vibeland/constants';

import {
  FINE_ART_SOCIETY_PROJECT,
  PUBLIC_PROJECTS,
  SPAWN_ADMIN,
  SPAWN_USER,
  STATIC_ZONES,
  ZONE_RING_RADIUS,
  generateProjectsFromAPI,
  zonePosition,
} from './constants';
import type { HudPanelId, ProjectData, VirtualZone } from './types';

export function useVirtualEnvironment() {
  const { user } = useAuth();
  const isAdmin = user?.is_admin ?? false;

  // Fetch virtual zones from API
  const [worldZones, setWorldZones] = useState<VirtualZone[]>(STATIC_ZONES);

  useEffect(() => {
    virtualSpacesApi.list()
      .then((spaces) => {
        setWorldZones(prev => prev.map(zone => {
          const found = spaces.find(s => s.space_name === zone.space_name);
          if (found) {
            return { ...zone, world_x: found.world_x, spawn_x: found.spawn_x, spawn_y: found.spawn_y, spawn_z: found.spawn_z };
          }
          return zone;
        }));
      })
      .catch(() => {});
  }, []);

  // Fetch projects from the Dashboard API
  const { data: apiProjects = [], isLoading: projectsLoading, error: projectsError, refetch: refetchProjects } = useProjectList();

  const allProjects = useMemo(() => {
    const staticProjects = [FINE_ART_SOCIETY_PROJECT];
    const apiProjectNames = new Set(apiProjects.map(p => p.name));
    const uniqueStaticProjects = staticProjects.filter(p => !apiProjectNames.has(p.name));
    return [...apiProjects, ...uniqueStaticProjects];
  }, [apiProjects]);

  const accessibleIds = useMemo(() => new Set(apiProjects.map(p => p.id)), [apiProjects]);
  const projects = useMemo(() => generateProjectsFromAPI(allProjects, accessibleIds), [allProjects, accessibleIds]);

  // Zone buildings
  const zoneBuildings = useMemo(() =>
    worldZones.map((zone, idx) => {
      const [x, z] = zonePosition(idx, worldZones.length, ZONE_RING_RADIUS);
      const position: [number, number, number] = [x, 0, z];
      const facingAngle = Math.atan2(-x, -z);
      return { zone, position, facingAngle, accessible: isAdmin || PUBLIC_PROJECTS.has(zone.space_name) };
    }),
    [worldZones, isAdmin]
  );

  const [selectedProject] = useState<ProjectData | null>(null);
  const [noraLine, setNoraLine] = useState('Command Center online. Syncing with Dashboard...');
  const [noraStatusVersion, setNoraStatusVersion] = useState(1);

  // Spawn position
  const spawnPosition = useMemo<[number, number, number]>(() => {
    if (isAdmin) return SPAWN_ADMIN;
    const homeOrgSlug = user?.organizations?.[0]?.slug;
    const homeOrg = user?.home_organization_id
      ? user.organizations?.find(o => o.id === user.home_organization_id)
      : null;
    const slug = homeOrg?.slug ?? homeOrgSlug;
    const zoneSlugMap: Record<string, string> = {
      'media-monsters': 'Media Monsters HQ',
      'sirak-studios': 'Sirak Studios',
      'jungleverse': 'Jungleverse',
      'veratwin': 'Veritwin',
    };
    const targetZoneName = slug ? zoneSlugMap[slug] : null;
    if (targetZoneName) {
      const idx = STATIC_ZONES.findIndex(z => z.space_name === targetZoneName);
      if (idx !== -1) {
        const [zx, zz] = zonePosition(idx, STATIC_ZONES.length, ZONE_RING_RADIUS * 0.85);
        return [zx, 1, zz];
      }
    }
    return SPAWN_USER;
  }, [isAdmin, user]);

  const [userPosition, setUserPosition] = useState<[number, number, number]>(spawnPosition);
  const [activeZone, setActiveZone] = useState<VirtualZone | null>(null);
  const [isConsoleInputActive, setIsConsoleInputActive] = useState(false);

  // DEBUG: track keyboard events reaching window
  const [debugLastKey, setDebugLastKey] = useState('none');
  const [debugKeyCount, setDebugKeyCount] = useState(0);
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      setDebugLastKey(e.key);
      setDebugKeyCount(c => c + 1);
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, []);

  const [consoleFocusVersion, setConsoleFocusVersion] = useState(0);
  const [isChatCollapsed, setIsChatCollapsed] = useState(false);
  const [activeHudPanel, setActiveHudPanel] = useState<HudPanelId | null>(null);

  const updateNoraLine = useCallback((line: string) => {
    setNoraLine(line);
    setNoraStatusVersion((prev) => prev + 1);
  }, []);

  useEffect(() => {
    if (!projectsLoading && !projectsError && apiProjects.length > 0) {
      updateNoraLine(`Dashboard sync complete. ${apiProjects.length} projects online. Topos data sphere active.`);
    } else if (!projectsLoading && !projectsError && apiProjects.length === 0) {
      updateNoraLine('Dashboard offline. Demo projects available. Fine Art Society space ready to explore.');
    }
  }, [projectsLoading, projectsError, apiProjects.length, updateNoraLine]);

  const bumpConsoleFocus = useCallback(() => {
    setConsoleFocusVersion((prev) => prev + 1);
  }, []);

  const activateConsoleInput = useCallback(() => {
    if (activeZone) return;
    setIsConsoleInputActive(true);
    bumpConsoleFocus();
  }, [activeZone, bumpConsoleFocus]);

  const releaseConsoleInput = useCallback(() => {
    setIsConsoleInputActive(false);
  }, []);

  const toggleChatCollapse = useCallback(() => {
    setIsChatCollapsed((prev) => {
      if (!prev) { releaseConsoleInput(); }
      return !prev;
    });
  }, [releaseConsoleInput]);

  // Multiplayer position tracking
  const sendPositionUpdate = useMultiplayerStore((s) => s.sendPositionUpdate);
  const multiplayerIsConnected = useMultiplayerStore((s) => s.isConnected);
  const isMovingRef = useRef(false);
  const wasMovingRef = useRef(false);
  const lastPositionRef = useRef<[number, number, number]>(spawnPosition);
  const lastZoneRef = useRef('ground');
  const stoppedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleUserPositionChange = useCallback((vector: THREE.Vector3) => {
    const newPos: [number, number, number] = [vector.x, vector.y, vector.z];
    setUserPosition(newPos);

    const [lx, ly, lz] = lastPositionRef.current;
    const dist = Math.hypot(vector.x - lx, vector.y - ly, vector.z - lz);
    isMovingRef.current = dist > 0.01;
    lastPositionRef.current = newPos;

    let zone = 'ground';
    const distFromCenter = Math.hypot(vector.x, vector.z);
    if (vector.y >= 75 && distFromCenter <= 45) {
      zone = 'command_center';
    } else if (vector.y >= 65 && vector.y < 78) {
      zone = 'workspace';
    }
    lastZoneRef.current = zone;

    if (stoppedTimeoutRef.current) {
      clearTimeout(stoppedTimeoutRef.current);
      stoppedTimeoutRef.current = null;
    }

    if (multiplayerIsConnected) {
      sendPositionUpdate(
        { x: vector.x, y: vector.y, z: vector.z },
        { y: 0 },
        zone,
        isMovingRef.current
      );
      if (wasMovingRef.current && !isMovingRef.current) {
        stoppedTimeoutRef.current = setTimeout(() => {
          sendPositionUpdate(
            { x: newPos[0], y: newPos[1], z: newPos[2] },
            { y: 0 },
            lastZoneRef.current,
            false
          );
        }, 150);
      }
    }
    wasMovingRef.current = isMovingRef.current;
  }, [sendPositionUpdate, multiplayerIsConnected]);

  // Zone entry detection
  const ZONE_BUILDING_ENTRY_DISTANCE = ENTRY_TRIGGER_DISTANCE + BUILDING_HALF_LENGTH;
  const enterZoneTarget = useMemo(() => {
    if (activeZone) return null;
    let closest: { zone: VirtualZone; distance: number } | null = null;
    for (const { zone, position, accessible } of zoneBuildings) {
      if (!accessible) continue;
      const dx = position[0] - userPosition[0];
      const dz = position[2] - userPosition[2];
      const distance = Math.hypot(dx, dz);
      if (distance > ZONE_BUILDING_ENTRY_DISTANCE) continue;
      if (!closest || distance < closest.distance) {
        closest = { zone, distance };
      }
    }
    return closest?.zone ?? null;
  }, [activeZone, zoneBuildings, userPosition, ZONE_BUILDING_ENTRY_DISTANCE]);

  const handleAttemptEnter = useCallback(() => {
    if (!enterZoneTarget) return;
    setActiveZone(enterZoneTarget);
    updateNoraLine(`${enterZoneTarget.space_name} \u2014 ${enterZoneTarget.host_username}'s space. Press Esc to close.`);
  }, [enterZoneTarget, updateNoraLine]);

  const closeZonePanel = useCallback(() => {
    setActiveZone(null);
    updateNoraLine('Global environment active.');
  }, [updateNoraLine]);

  const toggleHudPanel = useCallback((panel: HudPanelId) => {
    setActiveHudPanel((prev) => (prev === panel ? null : panel));
  }, []);

  // Keyboard: close zone panel on Esc
  useEffect(() => {
    if (!activeZone) return undefined;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { closeZonePanel(); }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeZone, closeZonePanel]);

  // Keyboard: Enter to open console, Esc to release
  useEffect(() => {
    const handleConsoleToggle = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const tag = target?.tagName.toLowerCase();
      const isTypingTarget =
        tag === 'input' || tag === 'textarea' || tag === 'select' || target?.isContentEditable;
      if (isTypingTarget) return;
      if (event.key === 'Enter' && !event.repeat) {
        event.preventDefault();
        activateConsoleInput();
      }
      if (event.key === 'Escape') {
        event.preventDefault();
        releaseConsoleInput();
      }
    };
    window.addEventListener('keydown', handleConsoleToggle);
    return () => window.removeEventListener('keydown', handleConsoleToggle);
  }, [activateConsoleInput, isConsoleInputActive, releaseConsoleInput]);

  // Deactivate console when zone panel opens
  useEffect(() => {
    if (activeZone && isConsoleInputActive) {
      setIsConsoleInputActive(false);
    }
  }, [activeZone, isConsoleInputActive]);

  return {
    isAdmin,
    worldZones,
    allProjects,
    projects,
    projectsLoading,
    projectsError,
    refetchProjects,
    zoneBuildings,
    selectedProject,
    noraLine,
    noraStatusVersion,
    spawnPosition,
    userPosition,
    activeZone,
    isConsoleInputActive,
    debugLastKey,
    debugKeyCount,
    consoleFocusVersion,
    isChatCollapsed,
    activeHudPanel,
    enterZoneTarget,
    setActiveHudPanel,
    releaseConsoleInput,
    toggleChatCollapse,
    toggleHudPanel,
    handleUserPositionChange,
    handleAttemptEnter,
    closeZonePanel,
  };
}
