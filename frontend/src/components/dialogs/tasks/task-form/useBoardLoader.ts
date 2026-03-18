import { useState, useEffect } from 'react';
import { projectsApi } from '@/lib/api';
import type { ProjectBoard } from 'shared/types';

export function useBoardLoader({
  projectId,
  isEditMode,
  modalVisible,
  initialBoardId,
}: {
  projectId?: string;
  isEditMode: boolean;
  modalVisible: boolean;
  initialBoardId?: string | null;
}): {
  boards: ProjectBoard[];
  boardsLoading: boolean;
  boardsError: string | null;
  selectedBoardId: string | null;
  setSelectedBoardId: (id: string | null) => void;
} {
  const [boards, setBoards] = useState<ProjectBoard[]>([]);
  const [boardsLoading, setBoardsLoading] = useState(false);
  const [boardsError, setBoardsError] = useState<string | null>(null);
  const [selectedBoardId, setSelectedBoardId] = useState<string | null>(null);

  // Track the projectId that boards were loaded for to prevent stale validation
  const [boardsProjectId, setBoardsProjectId] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (!projectId || !modalVisible) {
      setBoards([]);
      setBoardsError(null);
      setBoardsLoading(false);
      setBoardsProjectId(undefined);
      return;
    }

    let cancelled = false;
    const loadBoards = async () => {
      setBoardsLoading(true);
      setBoardsError(null);
      try {
        const results = await projectsApi.listBoards(projectId);
        if (!cancelled) {
          setBoards(results);
          setBoardsProjectId(projectId); // Track which project these boards belong to
        }
      } catch (error) {
        console.error('Failed to load boards', error);
        if (!cancelled) {
          setBoardsError(
            error instanceof Error ? error.message : 'Failed to load boards'
          );
        }
      } finally {
        if (!cancelled) {
          setBoardsLoading(false);
        }
      }
    };

    loadBoards();

    return () => {
      cancelled = true;
    };
  }, [projectId, modalVisible]);

  useEffect(() => {
    // Only validate if boards are loaded for the CURRENT project
    if (!selectedBoardId) return;
    if (!boards.length) return;
    if (boardsProjectId !== projectId) return; // Don't validate against stale boards

    const exists = boards.some((board) => board.id === selectedBoardId);
    if (!exists) {
      // Only reset if the board genuinely doesn't exist (not due to stale data)
      setSelectedBoardId(null);
    }
  }, [boards, selectedBoardId, boardsProjectId, projectId]);

  useEffect(() => {
    if (isEditMode) return;
    if (!modalVisible) return;
    if (boardsLoading) return;
    if (!boards.length) return;
    // Ensure boards are loaded for the current project before selecting
    if (boardsProjectId !== projectId) return;

    if (selectedBoardId && boards.some((board) => board.id === selectedBoardId)) {
      return;
    }

    // If initialBoardId is provided and exists in the boards list, use it
    if (initialBoardId && boards.some((board) => board.id === initialBoardId)) {
      setSelectedBoardId(initialBoardId);
      return;
    }

    // Fallback: prefer the default board, then first available
    const preferred =
      boards.find((board) => board.board_type === 'default') || boards[0];
    if (preferred) {
      setSelectedBoardId(preferred.id);
    }
  }, [boards, boardsLoading, isEditMode, selectedBoardId, modalVisible, initialBoardId, boardsProjectId, projectId]);

  return {
    boards,
    boardsLoading,
    boardsError,
    selectedBoardId,
    setSelectedBoardId,
  };
}
