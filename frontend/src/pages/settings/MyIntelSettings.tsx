import { MyIntelPanel } from '@/components/intel/MyIntelPanel';

export function MyIntelSettings() {
  return (
    <div className="max-w-2xl space-y-4">
      <div>
        <h1 className="text-lg font-semibold">My Intel</h1>
        <p className="text-sm text-muted-foreground">
          Your personal knowledge graph — private research on companies, people,
          and projects.
        </p>
      </div>
      <MyIntelPanel />
    </div>
  );
}
