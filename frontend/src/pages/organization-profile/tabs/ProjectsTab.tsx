import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { FolderOpen, Briefcase, X } from 'lucide-react';
import { ProjectRow } from '../components/ProjectRow';
import type { SidebarOrg, SidebarClient, SidebarProject } from '@/lib/api';

/** Sidebar org with optional internal_folders (legacy field) */
interface SidebarOrgWithFolders extends SidebarOrg {
  internal_folders?: Array<{ name: string; projects: SidebarProject[] }>;
}

/** Sidebar client with optional folders (legacy field) */
interface SidebarClientWithFolders extends SidebarClient {
  folders?: Array<{ name: string; projects: SidebarProject[] }>;
}

export function ProjectsTab({
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, unused-imports/no-unused-vars
  orgId: _orgId,
  sidebarOrg,
  clientFilter,
  onClearClientFilter,
}: {
  orgId: string;
  sidebarOrg: SidebarOrgWithFolders | null;
  clientFilter: string | null;
  onClearClientFilter: () => void;
}) {
  if (!sidebarOrg) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <FolderOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
        <p>No projects found</p>
      </div>
    );
  }

  const filteredClient = clientFilter
    ? (sidebarOrg.clients as SidebarClientWithFolders[] | undefined)?.find((c) => c.id === clientFilter)
    : null;

  return (
    <div className="space-y-4">
      {filteredClient && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-accent/30 border border-accent/50">
          <Briefcase className="h-4 w-4 text-purple-500" />
          <span className="text-sm font-medium">Viewing: {filteredClient.name}</span>
          <button
            onClick={onClearClientFilter}
            className="ml-auto p-1 rounded hover:bg-accent"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Internal projects (hidden if client filter active) */}
        {!clientFilter && (
          <Card className="bg-card/80 backdrop-blur-sm border-border/50">
            <CardHeader>
              <CardTitle className="text-base">Internal Projects</CardTitle>
              <CardDescription>Projects not assigned to a specific client</CardDescription>
            </CardHeader>
            <CardContent>
              {(sidebarOrg.internal_projects?.length === 0 &&
                (sidebarOrg.internal_folders || []).length === 0) ? (
                <p className="text-sm text-muted-foreground text-center py-4">No internal projects</p>
              ) : (
                <div className="space-y-2">
                  {sidebarOrg.internal_projects?.map((p) => (
                    <ProjectRow key={p.id} project={p} />
                  ))}
                  {(sidebarOrg.internal_folders || []).map((folder) =>
                    folder.projects.map((p) => (
                      <ProjectRow key={p.id} project={p} folderName={folder.name} />
                    ))
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        )}

        {/* Client projects */}
        {(clientFilter ? [filteredClient].filter((c): c is SidebarClientWithFolders => c != null) : (sidebarOrg.clients as SidebarClientWithFolders[]) || []).map((client) => (
          <Card key={client.id} className="bg-card/80 backdrop-blur-sm border-border/50">
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Briefcase className="h-4 w-4 text-purple-600" />
                {client?.name}
              </CardTitle>
              <CardDescription>Client projects</CardDescription>
            </CardHeader>
            <CardContent>
              {(client?.projects?.length === 0 && (client?.folders || []).length === 0) ? (
                <p className="text-sm text-muted-foreground text-center py-4">No projects</p>
              ) : (
                <div className="space-y-2">
                  {client?.projects?.map((p) => (
                    <ProjectRow key={p.id} project={p} />
                  ))}
                  {(client?.folders || []).map((folder) =>
                    folder.projects.map((p) => (
                      <ProjectRow key={p.id} project={p} folderName={folder.name} />
                    ))
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
