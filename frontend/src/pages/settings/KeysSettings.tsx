import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import {
  Key,
  Loader2,
  Check,
  X,
  Eye,
  EyeOff,
  Trash2,
  Save,
} from 'lucide-react';
import { pcgRouterApi, type ProviderKeyStatus } from '@/lib/api';

const PROVIDER_INFO: Record<string, { label: string; envVar: string; placeholder: string; docsHint: string }> = {
  anthropic: {
    label: 'Anthropic',
    envVar: 'ANTHROPIC_API_KEY',
    placeholder: 'sk-ant-api03-...',
    docsHint: 'Get your key at console.anthropic.com',
  },
  openai: {
    label: 'OpenAI',
    envVar: 'OPENAI_API_KEY',
    placeholder: 'sk-...',
    docsHint: 'Get your key at platform.openai.com',
  },
  gemini: {
    label: 'Google Gemini',
    envVar: 'GEMINI_API_KEY',
    placeholder: 'AIza...',
    docsHint: 'Get your key at aistudio.google.com',
  },
  xai: {
    label: 'xAI (Grok)',
    envVar: 'XAI_API_KEY',
    placeholder: 'xai-...',
    docsHint: 'Get your key at console.x.ai',
  },
  mistral: {
    label: 'Mistral AI',
    envVar: 'MISTRAL_API_KEY',
    placeholder: '...',
    docsHint: 'Get your key at console.mistral.ai',
  },
  deepseek: {
    label: 'DeepSeek',
    envVar: 'DEEPSEEK_API_KEY',
    placeholder: 'sk-...',
    docsHint: 'Get your key at platform.deepseek.com',
  },
  groq: {
    label: 'Groq',
    envVar: 'GROQ_API_KEY',
    placeholder: 'gsk_...',
    docsHint: 'Get your key at console.groq.com',
  },
  cohere: {
    label: 'Cohere',
    envVar: 'COHERE_API_KEY',
    placeholder: '...',
    docsHint: 'Get your key at dashboard.cohere.com',
  },
  qwen: {
    label: 'Qwen (DashScope)',
    envVar: 'DASHSCOPE_API_KEY',
    placeholder: 'sk-...',
    docsHint: 'Get your key at dashscope.console.aliyun.com',
  },
  openrouter: {
    label: 'OpenRouter',
    envVar: 'OPENROUTER_API_KEY',
    placeholder: 'sk-or-...',
    docsHint: 'Get your key at openrouter.ai',
  },
};

function ProviderKeyCard({
  status,
  onSave,
  onDelete,
  isSaving,
}: {
  status: ProviderKeyStatus;
  onSave: (provider: string, key: string) => void;
  onDelete: (provider: string) => void;
  isSaving: boolean;
}) {
  const [keyValue, setKeyValue] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [editing, setEditing] = useState(false);

  const info = PROVIDER_INFO[status.provider] || {
    label: status.provider,
    envVar: status.env_var || `${status.provider.toUpperCase()}_API_KEY`,
    placeholder: '...',
    docsHint: '',
  };

  const handleSave = () => {
    if (keyValue.trim()) {
      onSave(status.provider, keyValue.trim());
      setKeyValue('');
      setEditing(false);
      setShowKey(false);
    }
  };

  const handleDelete = () => {
    onDelete(status.provider);
    setKeyValue('');
    setEditing(false);
  };

  return (
    <div className="flex items-start gap-4 p-4 rounded-lg border bg-card">
      <div className="flex-1 min-w-0 space-y-2">
        <div className="flex items-center gap-2">
          <span className="font-medium">{info.label}</span>
          {status.has_key ? (
            <Badge variant="outline" className="text-green-600 border-green-300 bg-green-50 dark:bg-green-950 dark:border-green-800 dark:text-green-400">
              <Check className="h-3 w-3 mr-1" />
              Configured
            </Badge>
          ) : (
            <Badge variant="outline" className="text-amber-600 border-amber-300 bg-amber-50 dark:bg-amber-950 dark:border-amber-800 dark:text-amber-400">
              <X className="h-3 w-3 mr-1" />
              Not set
            </Badge>
          )}
          <span className="text-xs text-muted-foreground">
            {status.enabled_count}/{status.model_count} models enabled
          </span>
        </div>

        {info.docsHint && (
          <p className="text-xs text-muted-foreground">{info.docsHint}</p>
        )}

        {(editing || !status.has_key) && (
          <div className="flex items-center gap-2 mt-2">
            <div className="relative flex-1">
              <Input
                type={showKey ? 'text' : 'password'}
                placeholder={info.placeholder}
                value={keyValue}
                onChange={(e) => setKeyValue(e.target.value)}
                className="pr-10 font-mono text-sm"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleSave();
                }}
              />
              <button
                type="button"
                onClick={() => setShowKey(!showKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              >
                {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </button>
            </div>
            <Button
              size="sm"
              onClick={handleSave}
              disabled={!keyValue.trim() || isSaving}
            >
              {isSaving ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Save className="h-4 w-4 mr-1" />
              )}
              Save
            </Button>
            {editing && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  setEditing(false);
                  setKeyValue('');
                }}
              >
                Cancel
              </Button>
            )}
          </div>
        )}

        {status.has_key && !editing && (
          <div className="flex items-center gap-2 mt-1">
            <span className="font-mono text-xs text-muted-foreground">
              ••••••••••••••••
            </span>
          </div>
        )}
      </div>

      {status.has_key && !editing && (
        <div className="flex items-center gap-1 shrink-0">
          <Button
            size="sm"
            variant="outline"
            onClick={() => setEditing(true)}
          >
            Update
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="text-destructive hover:text-destructive"
            onClick={handleDelete}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      )}
    </div>
  );
}

export function KeysSettings() {
  const queryClient = useQueryClient();
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const {
    data: providerKeys = [],
    isLoading,
    error,
  } = useQuery<ProviderKeyStatus[]>({
    queryKey: ['provider-keys'],
    queryFn: pcgRouterApi.listProviderKeys,
  });

  const saveMutation = useMutation({
    mutationFn: ({ provider, apiKey }: { provider: string; apiKey: string }) =>
      pcgRouterApi.setProviderKey(provider, apiKey),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['provider-keys'] });
      const info = PROVIDER_INFO[data.provider];
      setSuccessMsg(`${info?.label || data.provider} API key saved (${data.models_updated} models updated)`);
      setErrorMsg(null);
      setTimeout(() => setSuccessMsg(null), 4000);
    },
    onError: (err: Error) => {
      setErrorMsg(err.message);
      setSuccessMsg(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (provider: string) => pcgRouterApi.deleteProviderKey(provider),
    onSuccess: (_data, provider) => {
      queryClient.invalidateQueries({ queryKey: ['provider-keys'] });
      const info = PROVIDER_INFO[provider];
      setSuccessMsg(`${info?.label || provider} API key removed`);
      setErrorMsg(null);
      setTimeout(() => setSuccessMsg(null), 4000);
    },
    onError: (err: Error) => {
      setErrorMsg(err.message);
      setSuccessMsg(null);
    },
  });

  // Sort: configured providers first, then by label
  const sortedKeys = [...providerKeys].sort((a, b) => {
    if (a.has_key !== b.has_key) return a.has_key ? -1 : 1;
    const labelA = PROVIDER_INFO[a.provider]?.label || a.provider;
    const labelB = PROVIDER_INFO[b.provider]?.label || b.provider;
    return labelA.localeCompare(labelB);
  });

  const configuredCount = providerKeys.filter((p) => p.has_key).length;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin" />
        <span className="ml-2">Loading API keys...</span>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {successMsg && (
        <Alert className="border-green-200 bg-green-50 text-green-800 dark:border-green-800 dark:bg-green-950 dark:text-green-200">
          <AlertDescription className="font-medium">{successMsg}</AlertDescription>
        </Alert>
      )}

      {errorMsg && (
        <Alert variant="destructive">
          <AlertDescription>{errorMsg}</AlertDescription>
        </Alert>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertDescription>
            {error instanceof Error ? error.message : 'Failed to load provider keys'}
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Key className="h-5 w-5" />
            API Keys
          </CardTitle>
          <CardDescription>
            Configure API keys for LLM providers. Keys are stored securely and used by the PCG Router
            to authenticate requests to provider APIs. {configuredCount > 0
              ? `${configuredCount} of ${providerKeys.length} providers configured.`
              : 'No providers configured yet — add at least one key to enable AI features.'}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {sortedKeys.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4">
              No AI providers registered. Add models in the Models settings first.
            </p>
          ) : (
            <div className="space-y-3">
              {sortedKeys.map((status) => (
                <ProviderKeyCard
                  key={status.provider}
                  status={status}
                  onSave={(provider, key) =>
                    saveMutation.mutate({ provider, apiKey: key })
                  }
                  onDelete={(provider) => deleteMutation.mutate(provider)}
                  isSaving={saveMutation.isPending}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Environment Variables</CardTitle>
          <CardDescription>
            As an alternative to storing keys in the database, you can set environment variables
            before starting the server. Stored keys take priority over environment variables.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="rounded-md border">
            <div className="p-3 space-y-1">
              {Object.entries(PROVIDER_INFO).map(([provider, info]) => {
                const status = providerKeys.find((p) => p.provider === provider);
                if (!status) return null;
                return (
                  <div key={provider} className="flex items-center gap-3 py-1">
                    <code className="text-xs font-mono bg-muted px-2 py-0.5 rounded">
                      {info.envVar}
                    </code>
                    <span className="text-xs text-muted-foreground">{info.label}</span>
                  </div>
                );
              })}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
