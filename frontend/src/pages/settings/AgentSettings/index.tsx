import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Skeleton } from '@/components/ui/skeleton';
import { Mail } from 'lucide-react';
import { useProfiles } from '@/hooks/useProfiles';

import { NoraCommunicationChannels } from './NoraCommunicationChannels';
import { AgentDirectoryCard } from './AgentDirectoryCard';
import { WalletSection } from './WalletSection';

export function AgentSettings() {
  const {
    isLoading: profilesLoading,
    error: profilesError,
  } = useProfiles();

  if (profilesLoading) {
    return (
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-48 mb-2" />
            <Skeleton className="h-4 w-72" />
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3, 4, 5, 6].map((i) => (
                <div key={i} className="rounded-xl border bg-card overflow-hidden">
                  <Skeleton className="aspect-square w-full" />
                  <div className="p-4 space-y-2">
                    <Skeleton className="h-5 w-32" />
                    <Skeleton className="h-4 w-24" />
                    <Skeleton className="h-3 w-full" />
                    <Skeleton className="h-3 w-3/4" />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {!!profilesError && (
        <Alert variant="destructive">
          <AlertDescription>
            {profilesError instanceof Error
              ? profilesError.message
              : String(profilesError)}
          </AlertDescription>
        </Alert>
      )}

      <AgentDirectoryCard />

      {/* Nora Channel Integrations */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mail className="h-5 w-5" />
            Nora Channel Integrations
          </CardTitle>
          <CardDescription>
            Connect Nora's own communication channels — email and SMS identity.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <NoraCommunicationChannels />
        </CardContent>
      </Card>

      <WalletSection />
    </div>
  );
}
