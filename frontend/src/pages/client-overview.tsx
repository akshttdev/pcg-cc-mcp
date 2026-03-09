import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api';
import { UserCircle, Users } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader } from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import NiceModal from '@ebay/nice-modal-react';
import '@/components/dialogs/shared/ConvertEntityDialog';
import { ClientMembersDialog } from '@/components/dialogs/client-members-dialog';

export function ClientOverview() {
  const { orgId, clientId } = useParams<{ orgId: string; clientId: string }>();
  const [membersOpen, setMembersOpen] = useState(false);

  const { data: clients = [], isLoading } = useQuery({
    queryKey: ['orgClients', orgId],
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const client = clients.find((c: any) => c.id === clientId);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader message="Loading client..." size={28} />
      </div>
    );
  }

  if (!client) {
    return (
      <div className="flex items-center justify-center min-h-[400px] text-muted-foreground">
        Client not found
      </div>
    );
  }

  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <UserCircle className="h-8 w-8 text-blue-500" />
          <div>
            <h1 className="text-2xl font-bold">{client.name}</h1>
            {client.description && (
              <p className="text-muted-foreground mt-1">{client.description}</p>
            )}
          </div>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setMembersOpen(true)}
          >
            <Users className="h-4 w-4 mr-2" />
            Members
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              NiceModal.show('convert-entity', {
                sourceType: 'client',
                sourceId: client.id,
                sourceName: client.name,
              })
            }
          >
            Convert to...
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Status</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{client.is_active ? 'Active' : 'Inactive'}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Slug</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-mono">{client.slug}</div>
          </CardContent>
        </Card>
      </div>
      {clientId && (
        <ClientMembersDialog
          open={membersOpen}
          onOpenChange={setMembersOpen}
          clientId={clientId}
          clientName={client.name}
        />
      )}
    </div>
  );
}

export default ClientOverview;
