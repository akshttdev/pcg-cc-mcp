import { useState, useEffect } from 'react';
import { Network, RotateCcw, Save, Loader2 } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { resolveApiUrl } from '@/lib/api';
import { toast } from 'sonner';

type PromptMode = 'standard' | 'sudolang';

interface AdminPromptData {
  prompt: string | null;
  mode: PromptMode;
  prompt_sudolang: string | null;
}

const DEFAULT_PROMPT = `You are Topsi, the platform orchestrator. Help users manage projects, tasks, CRM contacts, and workflows.`;
const DEFAULT_SUDOLANG_PROMPT = `Topsi {
  role: platform_orchestrator
  capabilities: [projects, tasks, crm, workflows]
}`;

export function TopsiAdminSettings() {
  const [data, setData] = useState<AdminPromptData>({
    prompt: null,
    mode: 'standard',
    prompt_sudolang: null,
  });
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    fetchPrompt();
  }, []);

  const fetchPrompt = async () => {
    setIsLoading(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/admin/prompt'), {
        credentials: 'include',
      });
      if (res.ok) {
        const json = await res.json();
        setData(json);
      }
    } catch (err) {
      console.error('Failed to fetch admin prompt:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/admin/prompt'), {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          prompt: data.prompt,
          mode: data.mode,
          prompt_sudolang: data.prompt_sudolang,
        }),
      });
      if (res.ok) {
        toast.success('System prompt saved');
      } else {
        const err = await res.text();
        toast.error(`Failed to save: ${err}`);
      }
    } catch (err) {
      toast.error('Failed to save prompt');
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = () => {
    setData((prev) => ({
      ...prev,
      prompt: DEFAULT_PROMPT,
      prompt_sudolang: DEFAULT_SUDOLANG_PROMPT,
    }));
  };

  const activePrompt = data.mode === 'sudolang' ? data.prompt_sudolang : data.prompt;

  const setActivePrompt = (value: string) => {
    if (data.mode === 'sudolang') {
      setData((prev) => ({ ...prev, prompt_sudolang: value }));
    } else {
      setData((prev) => ({ ...prev, prompt: value }));
    }
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
          <Network className="h-6 w-6 text-cyan-600" />
          Topsi Configuration
        </h2>
        <p className="text-sm text-muted-foreground mt-1">
          Manage system prompt and agent configuration
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>System Prompt</CardTitle>
          <CardDescription>
            The system prompt sent to the LLM for all Topsi interactions
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label className="mb-2 block">Prompt Mode</Label>
            <div className="flex gap-2">
              <Button
                variant={data.mode === 'standard' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setData((prev) => ({ ...prev, mode: 'standard' }))}
              >
                Standard
              </Button>
              <Button
                variant={data.mode === 'sudolang' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setData((prev) => ({ ...prev, mode: 'sudolang' }))}
              >
                SudoLang
              </Button>
            </div>
          </div>

          <div>
            <Label className="mb-2 block">
              {data.mode === 'sudolang' ? 'SudoLang Prompt' : 'Standard Prompt'}
            </Label>
            <Textarea
              value={activePrompt ?? ''}
              onChange={(e) => setActivePrompt(e.target.value)}
              className="font-mono text-sm min-h-[300px]"
              placeholder="Enter system prompt..."
            />
          </div>

          <div className="flex gap-2 justify-end">
            <Button variant="outline" onClick={handleReset}>
              <RotateCcw className="h-4 w-4 mr-1.5" />
              Reset to Default
            </Button>
            <Button onClick={handleSave} disabled={isSaving}>
              {isSaving ? (
                <Loader2 className="h-4 w-4 mr-1.5 animate-spin" />
              ) : (
                <Save className="h-4 w-4 mr-1.5" />
              )}
              Save
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
