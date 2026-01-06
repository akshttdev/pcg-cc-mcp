import { useCallback, useState } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Checkbox } from '@/components/ui/checkbox';
import {
  CheckCircle2,
  ExternalLink,
  Loader2,
  Trash2,
  Table2,
  XCircle,
} from 'lucide-react';
import { useUserSystem } from '@/components/config-provider';
import { airtableApi } from '@/lib/api';

export function AirtableSettings() {
  const { config, updateAndSaveConfig, loading } = useUserSystem();

  // Form state
  const [token, setToken] = useState(config?.airtable?.token || '');

  // UI state
  const [verifying, setVerifying] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [verifiedEmail, setVerifiedEmail] = useState<string | null>(
    config?.airtable?.user_email || null
  );

  const isConnected = !!(config?.airtable?.token);

  const handleVerifyAndSave = async () => {
    if (!token) {
      setError('Please enter your Personal Access Token');
      return;
    }

    setVerifying(true);
    setError(null);
    setSuccess(null);

    try {
      const result = await airtableApi.verifyCredentials({
        token: token,
      });

      if (result.valid) {
        setVerifiedEmail(result.user_email);
        setSaving(true);

        await updateAndSaveConfig({
          airtable: {
            ...config!.airtable,
            token: token,
            user_email: result.user_email,
          },
        });

        setSuccess(`Connected${result.user_email ? ` as ${result.user_email}` : ''}`);
      } else {
        setError('Invalid token. Please check your Personal Access Token.');
      }
    } catch (err) {
      console.error('Airtable verification error:', err);
      setError(
        'Failed to verify token. Please check your Personal Access Token.'
      );
    } finally {
      setVerifying(false);
      setSaving(false);
    }
  };

  const handleDisconnect = useCallback(async () => {
    setSaving(true);
    setError(null);

    try {
      await updateAndSaveConfig({
        airtable: {
          token: null,
          user_email: null,
          sync_deliverables_as_comments: true,
          auto_import_new_records: false,
        },
      });

      setToken('');
      setVerifiedEmail(null);
      setSuccess('Airtable disconnected successfully');
    } catch (err) {
      console.error('Error disconnecting Airtable:', err);
      setError('Failed to disconnect Airtable');
    } finally {
      setSaving(false);
    }
  }, [updateAndSaveConfig]);

  const handleSyncSettingChange = async (
    setting: 'sync_deliverables_as_comments' | 'auto_import_new_records',
    value: boolean
  ) => {
    try {
      await updateAndSaveConfig({
        airtable: {
          ...config!.airtable,
          [setting]: value,
        },
      });
    } catch (err) {
      console.error('Error updating Airtable setting:', err);
      setError('Failed to update setting');
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin" />
        <span className="ml-2">Loading settings...</span>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <Alert variant="destructive">
          <XCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {success && (
        <Alert className="border-green-200 bg-green-50 text-green-800 dark:border-green-800 dark:bg-green-950 dark:text-green-200">
          <CheckCircle2 className="h-4 w-4" />
          <AlertDescription className="font-medium">{success}</AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Table2 className="h-5 w-5" />
            Airtable Integration
          </CardTitle>
          <CardDescription>
            Connect your Airtable account to import records as tasks and sync
            deliverables back to Airtable.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isConnected ? (
            <div className="space-y-4">
              <div className="flex items-center justify-between p-4 border rounded-lg bg-green-50 dark:bg-green-950 border-green-200 dark:border-green-800">
                <div className="flex items-center gap-3">
                  <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />
                  <div>
                    <p className="font-medium text-green-800 dark:text-green-200">
                      Connected to Airtable
                    </p>
                    {verifiedEmail && (
                      <p className="text-sm text-green-600 dark:text-green-400">
                        {verifiedEmail}
                      </p>
                    )}
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDisconnect}
                  disabled={saving}
                  className="text-red-600 hover:text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:text-red-300 dark:hover:bg-red-950"
                >
                  {saving ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Trash2 className="h-4 w-4 mr-2" />
                  )}
                  Disconnect
                </Button>
              </div>

              <div className="space-y-4 pt-4 border-t">
                <h4 className="font-medium">Sync Settings</h4>

                <div className="flex items-start space-x-2">
                  <Checkbox
                    id="sync-deliverables"
                    checked={config?.airtable?.sync_deliverables_as_comments ?? true}
                    onCheckedChange={(checked: boolean) =>
                      handleSyncSettingChange(
                        'sync_deliverables_as_comments',
                        checked
                      )
                    }
                  />
                  <div className="space-y-0.5">
                    <Label htmlFor="sync-deliverables" className="cursor-pointer">
                      Sync deliverables as comments
                    </Label>
                    <p className="text-sm text-muted-foreground">
                      When a task execution completes, post a summary as a comment
                      on the linked Airtable record.
                    </p>
                  </div>
                </div>

                <div className="flex items-start space-x-2">
                  <Checkbox
                    id="auto-import"
                    checked={config?.airtable?.auto_import_new_records ?? false}
                    onCheckedChange={(checked: boolean) =>
                      handleSyncSettingChange('auto_import_new_records', checked)
                    }
                  />
                  <div className="space-y-0.5">
                    <Label htmlFor="auto-import" className="cursor-pointer">
                      Auto-import new records
                    </Label>
                    <p className="text-sm text-muted-foreground">
                      Automatically import new records when syncing a connected
                      base. (Coming soon)
                    </p>
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="token">Personal Access Token</Label>
                <Input
                  id="token"
                  type="password"
                  placeholder="Enter your Airtable Personal Access Token"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                />
                <p className="text-sm text-muted-foreground">
                  Create a Personal Access Token at{' '}
                  <a
                    href="https://airtable.com/create/tokens"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:underline inline-flex items-center gap-1"
                  >
                    airtable.com/create/tokens
                    <ExternalLink className="h-3 w-3" />
                  </a>
                  . Grant read/write access to your bases.
                </p>
              </div>

              <Button
                onClick={handleVerifyAndSave}
                disabled={verifying || !token}
                className="w-full"
              >
                {verifying ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Verifying...
                  </>
                ) : (
                  <>
                    <Table2 className="mr-2 h-4 w-4" />
                    Connect to Airtable
                  </>
                )}
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>How it works</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <div className="flex items-start gap-3">
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs font-medium">
              1
            </div>
            <div>
              <p className="font-medium text-foreground">
                Connect an Airtable base to a project
              </p>
              <p>
                In your project settings, link an Airtable base to import records as
                tasks.
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs font-medium">
              2
            </div>
            <div>
              <p className="font-medium text-foreground">Import records as tasks</p>
              <p>
                Select a table and import records. Each record becomes a task with the
                record's primary field as the title.
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs font-medium">
              3
            </div>
            <div>
              <p className="font-medium text-foreground">
                Execute with AI agents
              </p>
              <p>
                Run tasks through your configured AI agents. Results are tracked
                in PCG.
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs font-medium">
              4
            </div>
            <div>
              <p className="font-medium text-foreground">
                Sync deliverables back
              </p>
              <p>
                After execution, sync the results back to Airtable as comments on
                the original record.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
