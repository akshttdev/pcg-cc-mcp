import { useCallback, useEffect, useState } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  CheckCircle2,
  ExternalLink,
  Import,
  Link2,
  Link2Off,
  Loader2,
  Settings,
  Table2,
  XCircle,
} from 'lucide-react';
import { airtableApi } from '@/lib/api';
import {
  AirtableBaseInfo,
  AirtableBase,
  AirtableTable,
  AirtableRecord,
  AirtableImportResult,
} from 'shared/types';
import { useUserSystem } from '@/components/config-provider';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';

interface AirtableBaseConnectProps {
  projectId: string;
  projectName: string;
  onConnectionsChange?: (connections: AirtableBase[]) => void;
}

export function AirtableBaseConnect({
  projectId,
  projectName,
  onConnectionsChange,
}: AirtableBaseConnectProps) {
  const navigate = useNavigate();
  const { config } = useUserSystem();

  // Connection state
  const [connections, setConnections] = useState<AirtableBase[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Connect dialog state
  const [showConnectDialog, setShowConnectDialog] = useState(false);
  const [availableBases, setAvailableBases] = useState<AirtableBaseInfo[]>([]);
  const [loadingBases, setLoadingBases] = useState(false);
  const [selectedBaseId, setSelectedBaseId] = useState<string>('');
  const [connecting, setConnecting] = useState(false);

  // Import dialog state
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importConnectionId, setImportConnectionId] = useState<string | null>(null);
  const [importTables, setImportTables] = useState<AirtableTable[]>([]);
  const [importRecords, setImportRecords] = useState<AirtableRecord[]>([]);
  const [loadingTables, setLoadingTables] = useState(false);
  const [selectedTableId, setSelectedTableId] = useState<string>('');
  const [loadingRecords, setLoadingRecords] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importResult, setImportResult] = useState<AirtableImportResult | null>(null);

  const isConfigured = !!config?.airtable?.token;

  // Load connections for this project
  const loadConnections = useCallback(async () => {
    if (!isConfigured) {
      setLoading(false);
      onConnectionsChange?.([]);
      return;
    }

    try {
      setLoading(true);
      setError(null);
      const data = await airtableApi.listConnections(projectId);
      setConnections(data);
      onConnectionsChange?.(data);
    } catch (err) {
      console.error('Failed to load Airtable connections:', err);
      setError('Failed to load Airtable connections');
    } finally {
      setLoading(false);
    }
  }, [projectId, isConfigured, onConnectionsChange]);

  useEffect(() => {
    loadConnections();
  }, [loadConnections]);

  // Load available bases when connect dialog opens
  const handleOpenConnectDialog = async () => {
    setShowConnectDialog(true);
    setLoadingBases(true);
    setSelectedBaseId('');

    try {
      const bases = await airtableApi.listUserBases();
      setAvailableBases(bases);
    } catch (err) {
      console.error('Failed to load Airtable bases:', err);
      toast.error('Failed to load Airtable bases');
    } finally {
      setLoadingBases(false);
    }
  };

  // Connect a base
  const handleConnect = async () => {
    if (!selectedBaseId) return;

    const selectedBase = availableBases.find((b) => b.id === selectedBaseId);
    if (!selectedBase) return;

    setConnecting(true);
    try {
      await airtableApi.createConnection({
        project_id: projectId,
        airtable_base_id: selectedBaseId,
        airtable_base_name: selectedBase.name,
        default_table_id: null,
      });

      toast.success(`Connected to "${selectedBase.name}"`);
      setShowConnectDialog(false);
      loadConnections();
    } catch (err) {
      console.error('Failed to connect base:', err);
      toast.error('Failed to connect base');
    } finally {
      setConnecting(false);
    }
  };

  // Disconnect a base
  const handleDisconnect = async (connectionId: string, baseName?: string) => {
    if (
      !confirm(
        `Disconnect "${baseName || 'this base'}"? Existing task links will be preserved.`
      )
    ) {
      return;
    }

    try {
      await airtableApi.deleteConnection(connectionId);
      toast.success('Base disconnected');
      loadConnections();
    } catch (err) {
      console.error('Failed to disconnect base:', err);
      toast.error('Failed to disconnect base');
    }
  };

  // Open import dialog
  const handleOpenImportDialog = async (connection: AirtableBase) => {
    setImportConnectionId(connection.id);
    setShowImportDialog(true);
    setSelectedTableId('');
    setImportRecords([]);
    setImportResult(null);
    setLoadingTables(true);

    try {
      const tables = await airtableApi.getBaseTables(connection.id);
      setImportTables(tables);
    } catch (err) {
      console.error('Failed to load tables:', err);
      toast.error('Failed to load Airtable tables');
    } finally {
      setLoadingTables(false);
    }
  };

  // Load records when table is selected
  const handleTableSelect = async (tableId: string) => {
    setSelectedTableId(tableId);
    setImportRecords([]);
    setLoadingRecords(true);

    if (!importConnectionId) return;

    try {
      const records = await airtableApi.getTableRecords(importConnectionId, tableId);
      setImportRecords(records);
    } catch (err) {
      console.error('Failed to load records:', err);
      toast.error('Failed to load records');
    } finally {
      setLoadingRecords(false);
    }
  };

  // Get record display name from fields
  const getRecordName = (record: AirtableRecord): string => {
    const fields = record.fields as Record<string, unknown>;
    // Try common field names for the primary field
    for (const key of ['Name', 'Title', 'name', 'title']) {
      if (fields[key] && typeof fields[key] === 'string') {
        return fields[key] as string;
      }
    }
    // Fall back to first string field
    for (const value of Object.values(fields)) {
      if (typeof value === 'string' && value.length > 0) {
        return value;
      }
    }
    return record.id;
  };

  // Import records
  const handleImport = async () => {
    if (!importConnectionId || !selectedTableId) return;

    setImporting(true);
    try {
      const result = await airtableApi.importRecords(importConnectionId, {
        table_id: selectedTableId,
        project_id: projectId,
        board_id: null,
        title_field: null,
        description_field: null,
      });

      setImportResult(result);
      toast.success(
        `Imported ${result.imported_count} tasks, skipped ${result.skipped_count}`
      );
    } catch (err) {
      console.error('Failed to import records:', err);
      toast.error('Failed to import records');
    } finally {
      setImporting(false);
    }
  };

  if (!isConfigured) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Table2 className="h-5 w-5" />
            Airtable Integration
          </CardTitle>
          <CardDescription>
            Import records from Airtable and sync deliverables back.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Alert>
            <Settings className="h-4 w-4" />
            <AlertDescription>
              Airtable is not configured.{' '}
              <Button
                variant="link"
                className="h-auto p-0"
                onClick={() => navigate('/settings/airtable')}
              >
                Connect your Airtable account
              </Button>{' '}
              to enable this feature.
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    );
  }

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Table2 className="h-5 w-5" />
              Airtable Integration
            </CardTitle>
            <CardDescription>
              Import records from Airtable and sync deliverables back.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleOpenConnectDialog}
          >
            <Link2 className="h-4 w-4 mr-2" />
            Connect Base
          </Button>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <XCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          ) : connections.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Table2 className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No Airtable bases connected to this project.</p>
              <p className="text-sm">
                Connect a base to import records and sync deliverables.
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {connections.map((conn) => (
                <div
                  key={conn.id}
                  className="flex items-center justify-between p-3 border rounded-lg"
                >
                  <div className="flex items-center gap-3">
                    <Table2 className="h-5 w-5 text-blue-600" />
                    <div>
                      <p className="font-medium">
                        {conn.airtable_base_name || 'Unknown Base'}
                      </p>
                      {conn.last_synced_at && (
                        <p className="text-xs text-muted-foreground">
                          Last synced:{' '}
                          {new Date(conn.last_synced_at).toLocaleDateString()}
                        </p>
                      )}
                    </div>
                    <Badge
                      variant={conn.sync_enabled ? 'default' : 'secondary'}
                    >
                      {conn.sync_enabled ? 'Sync On' : 'Sync Off'}
                    </Badge>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleOpenImportDialog(conn)}
                    >
                      <Import className="h-4 w-4 mr-2" />
                      Import
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      asChild
                    >
                      <a
                        href={`https://airtable.com/${conn.airtable_base_id}`}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        <ExternalLink className="h-4 w-4" />
                      </a>
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() =>
                        handleDisconnect(conn.id, conn.airtable_base_name || undefined)
                      }
                    >
                      <Link2Off className="h-4 w-4 text-red-500" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Connect Base Dialog */}
      <Dialog open={showConnectDialog} onOpenChange={setShowConnectDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Connect Airtable Base</DialogTitle>
            <DialogDescription>
              Select an Airtable base to connect to "{projectName}".
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {loadingBases ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin" />
              </div>
            ) : (
              <Select
                value={selectedBaseId}
                onValueChange={setSelectedBaseId}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select a base..." />
                </SelectTrigger>
                <SelectContent>
                  {availableBases.map((base) => (
                    <SelectItem key={base.id} value={base.id}>
                      {base.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowConnectDialog(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleConnect}
              disabled={!selectedBaseId || connecting}
            >
              {connecting ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Connecting...
                </>
              ) : (
                <>
                  <Link2 className="h-4 w-4 mr-2" />
                  Connect
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Import Dialog */}
      <Dialog open={showImportDialog} onOpenChange={setShowImportDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Import Records from Airtable</DialogTitle>
            <DialogDescription>
              Select a table to import records as tasks.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            {importResult ? (
              <div className="space-y-4">
                <Alert className="border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950">
                  <CheckCircle2 className="h-4 w-4 text-green-600" />
                  <AlertDescription>
                    Successfully imported {importResult.imported_count} tasks.
                    {importResult.skipped_count > 0 &&
                      ` Skipped ${importResult.skipped_count} (already imported).`}
                  </AlertDescription>
                </Alert>
                {importResult.tasks.length > 0 && (
                  <div className="border rounded-lg max-h-60 overflow-y-auto">
                    <div className="p-2 space-y-1">
                      {importResult.tasks.map((t) => (
                        <div
                          key={t.task.id}
                          className="flex items-center gap-2 p-2 text-sm hover:bg-muted rounded"
                        >
                          <CheckCircle2 className="h-4 w-4 text-green-500 flex-shrink-0" />
                          <span className="truncate">{t.task.title}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Select Table</label>
                  {loadingTables ? (
                    <div className="flex items-center justify-center py-4">
                      <Loader2 className="h-5 w-5 animate-spin" />
                    </div>
                  ) : (
                    <Select
                      value={selectedTableId}
                      onValueChange={handleTableSelect}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select a table..." />
                      </SelectTrigger>
                      <SelectContent>
                        {importTables.map((table) => (
                          <SelectItem key={table.id} value={table.id}>
                            {table.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}
                </div>

                {selectedTableId && (
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      Records to Import ({importRecords.length})
                    </label>
                    {loadingRecords ? (
                      <div className="flex items-center justify-center py-4">
                        <Loader2 className="h-5 w-5 animate-spin" />
                      </div>
                    ) : importRecords.length === 0 ? (
                      <p className="text-sm text-muted-foreground py-4 text-center">
                        No records in this table.
                      </p>
                    ) : (
                      <div className="border rounded-lg max-h-60 overflow-y-auto">
                        <div className="p-2 space-y-1">
                          {importRecords.map((record) => (
                            <div
                              key={record.id}
                              className="flex items-start gap-2 p-2 text-sm hover:bg-muted rounded"
                            >
                              <Table2 className="h-4 w-4 text-blue-500 flex-shrink-0 mt-0.5" />
                              <div className="flex-1 min-w-0">
                                <p className="truncate font-medium">
                                  {getRecordName(record)}
                                </p>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </>
            )}
          </div>
          <DialogFooter>
            {importResult ? (
              <Button onClick={() => setShowImportDialog(false)}>Done</Button>
            ) : (
              <>
                <Button
                  variant="outline"
                  onClick={() => setShowImportDialog(false)}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleImport}
                  disabled={
                    !selectedTableId || importRecords.length === 0 || importing
                  }
                >
                  {importing ? (
                    <>
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      Importing...
                    </>
                  ) : (
                    <>
                      <Import className="h-4 w-4 mr-2" />
                      Import {importRecords.length} Records
                    </>
                  )}
                </Button>
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
