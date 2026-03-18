import { useState, useEffect } from 'react';
import { templatesApi, projectsApi, attemptsApi } from '@/lib/api';
import type { TaskTemplate, GitBranch } from 'shared/types';

export function useBranchLoader({
  projectId,
  isEditMode,
  modalVisible,
  initialBaseBranch,
  parentTaskAttemptId,
}: {
  projectId?: string;
  isEditMode: boolean;
  modalVisible: boolean;
  initialBaseBranch?: string;
  parentTaskAttemptId?: string;
}): {
  templates: TaskTemplate[];
  branches: GitBranch[];
  selectedBranch: string;
  setSelectedBranch: (branch: string) => void;
} {
  const [templates, setTemplates] = useState<TaskTemplate[]>([]);
  const [branches, setBranches] = useState<GitBranch[]>([]);
  const [selectedBranch, setSelectedBranch] = useState<string>('');

  // Fetch templates and branches when dialog opens in create mode
  useEffect(() => {
    if (modalVisible && !isEditMode && projectId) {
      // Fetch templates and branches
      Promise.all([
        templatesApi.listByProject(projectId),
        templatesApi.listGlobal(),
        projectsApi.getBranches(projectId),
      ])
        .then(([projectTemplates, globalTemplates, projectBranches]) => {
          // Combine templates with project templates first
          setTemplates([...projectTemplates, ...globalTemplates]);

          // Set branches and default to initialBaseBranch if provided, otherwise current branch
          setBranches(projectBranches);

          if (
            initialBaseBranch &&
            projectBranches.some((b) => b.name === initialBaseBranch)
          ) {
            // Use initialBaseBranch if it exists in the project branches (for spinoff)
            setSelectedBranch(initialBaseBranch);
          } else {
            // Default behavior: use current branch or first available
            const currentBranch = projectBranches.find((b) => b.is_current);
            const defaultBranch = currentBranch || projectBranches[0];
            if (defaultBranch) {
              setSelectedBranch(defaultBranch.name);
            }
          }
        })
        .catch(console.error);
    }
  }, [modalVisible, isEditMode, projectId, initialBaseBranch]);

  // Fetch parent base branch when parentTaskAttemptId is provided
  useEffect(() => {
    if (
      modalVisible &&
      !isEditMode &&
      parentTaskAttemptId &&
      !initialBaseBranch &&
      branches.length > 0
    ) {
      attemptsApi
        .get(parentTaskAttemptId)
        .then((attempt) => {
          const parentBranch = attempt.branch || attempt.base_branch;
          if (parentBranch && branches.some((b) => b.name === parentBranch)) {
            setSelectedBranch(parentBranch);
          }
        })
        .catch((err) => {
          console.error('Failed to load parent task attempt branch:', err);
        });
    }
  }, [
    modalVisible,
    isEditMode,
    parentTaskAttemptId,
    initialBaseBranch,
    branches,
  ]);

  return {
    templates,
    branches,
    selectedBranch,
    setSelectedBranch,
  };
}
