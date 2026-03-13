import { useState, useEffect } from 'react';
import { Bot, Save, Loader2 } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
// Label removed — not currently needed
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { resolveApiUrl } from '@/lib/api';
import { toast } from 'sonner';

type ConfirmationMode = 'always_confirm' | 'confirm_destructive' | 'autonomous';

interface UserSettings {
  default_confirmation_mode: ConfirmationMode;
  per_tool_overrides: Record<string, ConfirmationMode>;
  auto_approve_timeout_minutes: number | null;
}

const CONFIRMATION_MODE_LABELS: Record<ConfirmationMode, string> = {
  always_confirm: 'Always Confirm',
  confirm_destructive: 'Confirm Destructive Only',
  autonomous: 'Autonomous',
};

// Tool risk classification mirroring backend classify_tool_risk()
const TOOL_RISK_MAP: Record<'red' | 'yellow' | 'green', { label: string; color: string; tools: string[] }> = {
  red: {
    label: 'Destructive',
    color: 'text-red-600',
    tools: ['delete_task', 'bulk_update_tasks'],
  },
  yellow: {
    label: 'Create / Update',
    color: 'text-yellow-600',
    tools: [
      'create_project', 'update_project',
      'create_task', 'update_task', 'start_task_execution',
      'create_crm_contact', 'create_crm_deal', 'update_crm_deal',
      'approve_staged_records', 'build_workflow',
    ],
  },
  green: {
    label: 'Read-only',
    color: 'text-green-600',
    tools: [
      'list_projects', 'list_tasks', 'get_topology',
      'search_contacts', 'list_workflows',
    ],
  },
};

export function TopsiUserSettings() {
  const [settings, setSettings] = useState<UserSettings>({
    default_confirmation_mode: 'confirm_destructive',
    per_tool_overrides: {},
    auto_approve_timeout_minutes: null,
  });
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    fetchSettings();
  }, []);

  const fetchSettings = async () => {
    setIsLoading(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/user-settings'), {
        credentials: 'include',
      });
      if (res.ok) {
        const json = await res.json();
        setSettings({
          default_confirmation_mode: json.default_confirmation_mode ?? 'confirm_destructive',
          per_tool_overrides: json.per_tool_overrides
            ? (typeof json.per_tool_overrides === 'string'
              ? JSON.parse(json.per_tool_overrides)
              : json.per_tool_overrides)
            : {},
          auto_approve_timeout_minutes: json.auto_approve_timeout_minutes ?? null,
        });
      }
    } catch (err) {
      console.error('Failed to fetch user settings:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/user-settings'), {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          default_confirmation_mode: settings.default_confirmation_mode,
          per_tool_overrides: settings.per_tool_overrides,
          auto_approve_timeout_minutes: settings.auto_approve_timeout_minutes,
        }),
      });
      if (res.ok) {
        toast.success('Preferences saved');
      } else {
        toast.error('Failed to save preferences');
      }
    } catch (err) {
      toast.error('Failed to save preferences');
    } finally {
      setIsSaving(false);
    }
  };

  const setToolOverride = (tool: string, mode: ConfirmationMode | 'default') => {
    setSettings((prev) => {
      const overrides = { ...prev.per_tool_overrides };
      if (mode === 'default') {
        delete overrides[tool];
      } else {
        overrides[tool] = mode;
      }
      return { ...prev, per_tool_overrides: overrides };
    });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold flex items-center gap-2">
          <Bot className="h-6 w-6 text-cyan-600" />
          Topsi Preferences
        </h2>
        <p className="text-sm text-muted-foreground mt-1">
          Control tool confirmation and autonomy settings
        </p>
      </div>

      {/* Confirmation Mode */}
      <Card>
        <CardHeader>
          <CardTitle>Confirmation Mode</CardTitle>
          <CardDescription>
            How Topsi should handle tool execution confirmation
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Select
            value={settings.default_confirmation_mode}
            onValueChange={(v) =>
              setSettings((prev) => ({ ...prev, default_confirmation_mode: v as ConfirmationMode }))
            }
          >
            <SelectTrigger className="w-64">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {Object.entries(CONFIRMATION_MODE_LABELS).map(([value, label]) => (
                <SelectItem key={value} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </CardContent>
      </Card>

      {/* Per-Tool Overrides */}
      <Card>
        <CardHeader>
          <CardTitle>Per-Tool Overrides</CardTitle>
          <CardDescription>
            Override confirmation mode for specific tools
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {(Object.entries(TOOL_RISK_MAP) as [string, typeof TOOL_RISK_MAP.red][]).map(
              ([risk, { label, color, tools }]) => (
                <div key={risk}>
                  <h4 className={`text-sm font-medium mb-2 ${color}`}>
                    {label} Tools
                  </h4>
                  <div className="space-y-2">
                    {tools.map((tool) => (
                      <div
                        key={tool}
                        className="flex items-center justify-between px-3 py-2 rounded-md border"
                      >
                        <code className="text-sm">{tool}</code>
                        <Select
                          value={settings.per_tool_overrides[tool] ?? 'default'}
                          onValueChange={(v) =>
                            setToolOverride(tool, v as ConfirmationMode | 'default')
                          }
                        >
                          <SelectTrigger className="w-48 h-8 text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="default">Use Default</SelectItem>
                            {Object.entries(CONFIRMATION_MODE_LABELS).map(([value, lbl]) => (
                              <SelectItem key={value} value={value}>
                                {lbl}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    ))}
                  </div>
                </div>
              )
            )}
          </div>
        </CardContent>
      </Card>

      {/* Auto-Approve Timeout */}
      <Card>
        <CardHeader>
          <CardTitle>Auto-Approve Timeout</CardTitle>
          <CardDescription>
            Automatically approve pending confirmations after this many minutes (leave empty to disable)
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={1}
              max={60}
              placeholder="Disabled"
              value={settings.auto_approve_timeout_minutes ?? ''}
              onChange={(e) =>
                setSettings((prev) => ({
                  ...prev,
                  auto_approve_timeout_minutes: e.target.value ? Number(e.target.value) : null,
                }))
              }
              className="w-32"
            />
            <span className="text-sm text-muted-foreground">minutes</span>
          </div>
        </CardContent>
      </Card>

      <div className="flex justify-end">
        <Button onClick={handleSave} disabled={isSaving}>
          {isSaving ? (
            <Loader2 className="h-4 w-4 mr-1.5 animate-spin" />
          ) : (
            <Save className="h-4 w-4 mr-1.5" />
          )}
          Save Preferences
        </Button>
      </div>
    </div>
  );
}
