import { MeshPanel } from '@/components/mesh';
import { MobileLayout } from '@/components/mobile';
import { useMobile } from '@/hooks/useMobile';

export default function MeshPage() {
  const { isMobile } = useMobile();

  const content = (
    <>
      {!isMobile && (
        <div className="mb-6">
          <h1 className="text-2xl font-bold">Mesh Network</h1>
          <p className="text-muted-foreground">
            Monitor your Alpha Protocol Network node and resource contributions
          </p>
        </div>
      )}
      <MeshPanel />
    </>
  );

  if (isMobile) {
    return (
      <MobileLayout title="Mesh Network">
        {content}
      </MobileLayout>
    );
  }

  return (
    <div className="container mx-auto p-6 max-w-4xl">
      {content}
    </div>
  );
}
