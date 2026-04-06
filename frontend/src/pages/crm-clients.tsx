import { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';

import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';
import { CrmPipelineSettings } from '@/components/crm/CrmPipelineSettings';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function CrmClientsPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();

  if (!projectId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Project not found
      </div>
    );
  }

  return (
    <CrmPipelineBoard
      projectId={projectId}
      pipelineType="clients"
      title="Clients Pipeline"
      onSettingsClick={() => navigate(`/projects/${projectId}/crm`)}
    />
  );
}

export function CrmAcquisitionPage() {
  const { orgId } = useParams<{ orgId: string }>();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsPipelineId, setSettingsPipelineId] = useState<
    string | undefined
  >();

  if (!orgId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Organization not found
      </div>
    );
  }

  return (
    <>
      <CrmPipelineBoard
        orgId={orgId}
        pipelineType="sales"
        title="Acquisition Pipeline"
        onSettingsClick={(pipelineId) => {
          setSettingsPipelineId(pipelineId);
          setSettingsOpen(true);
        }}
      />
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="max-w-4xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Pipeline Settings</DialogTitle>
          </DialogHeader>
          <CrmPipelineSettings
            organizationId={orgId}
            initialPipelineId={settingsPipelineId}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}

export function CrmLifecyclePage() {
  const { orgId } = useParams<{ orgId: string }>();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsPipelineId, setSettingsPipelineId] = useState<
    string | undefined
  >();

  if (!orgId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Organization not found
      </div>
    );
  }

  return (
    <>
      <CrmPipelineBoard
        orgId={orgId}
        pipelineType="delivery"
        title="Client Lifecycle"
        onSettingsClick={(pipelineId) => {
          setSettingsPipelineId(pipelineId);
          setSettingsOpen(true);
        }}
      />
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="max-w-4xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Pipeline Settings</DialogTitle>
          </DialogHeader>
          <CrmPipelineSettings
            organizationId={orgId}
            initialPipelineId={settingsPipelineId}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
