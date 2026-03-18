// Unified onboarding wizard — replaces 4 sequential blocking modals with a single
// stepped dialog. Steps: Agent/Editor setup → GitHub connect → Privacy preferences.
// Safety disclaimer becomes an inline collapsible section.
import { useState, useEffect, useCallback } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Alert } from '@/components/ui/alert';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  HandMetal,
  Sparkles,
  Code,
  ChevronDown,
  Github,
  Shield,
  Check,
  Clipboard,
  CheckCircle,
  XCircle,
  Settings,
  AlertTriangle,
  ChevronRight,
  ChevronUp,
} from 'lucide-react';
import { Loader } from '@/components/ui/loader';
import {
  BaseCodingAgent,
  EditorType,
  DeviceFlowStartResponse,
  DevicePollStatus,
} from 'shared/types';
import type { ExecutorProfileId } from 'shared/types';
import { useUserSystem } from '@/components/config-provider';
import { githubAuthApi } from '@/lib/api';
import { toPrettyCase } from '@/utils/string';
import NiceModal, { useModal } from '@ebay/nice-modal-react';

// --- Constants ---

const WIZARD_STEPS = [
  { id: 'agent-editor', label: 'Agent & Editor', icon: Sparkles },
  { id: 'github', label: 'GitHub', icon: Github },
  { id: 'privacy', label: 'Privacy', icon: Shield },
] as const;

const TOTAL_STEPS = WIZARD_STEPS.length;

type StepId = (typeof WIZARD_STEPS)[number]['id'];

export interface WelcomeWizardResult {
  profile: ExecutorProfileId;
  editor: { editor_type: EditorType; custom_command: string | null };
  githubConnected: boolean;
  analyticsEnabled: boolean;
}

// --- Step Indicator ---

function StepIndicator({ currentStep }: { currentStep: number }) {
  return (
    <div className="flex items-center justify-center gap-1 py-2">
      {WIZARD_STEPS.map((step, index) => {
        const Icon = step.icon;
        const isActive = index === currentStep;
        const isComplete = index < currentStep;
        return (
          <div key={step.id} className="flex items-center gap-1">
            {index > 0 && (
              <div
                className={`h-px w-6 ${isComplete ? 'bg-primary' : 'bg-border'}`}
              />
            )}
            <div
              className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium transition-colors ${
                isActive
                  ? 'bg-primary text-primary-foreground'
                  : isComplete
                    ? 'bg-primary/10 text-primary'
                    : 'bg-muted text-muted-foreground'
              }`}
            >
              {isComplete ? (
                <Check className="h-3 w-3" />
              ) : (
                <Icon className="h-3 w-3" />
              )}
              <span className="hidden sm:inline">{step.label}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}

// --- Safety Disclaimer (inline collapsible) ---

function SafetyDisclaimer() {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="rounded-lg border border-amber-200 dark:border-amber-800 bg-amber-50/50 dark:bg-amber-950/20">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center justify-between w-full p-3 text-left text-sm"
      >
        <div className="flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-amber-500 flex-shrink-0" />
          <span className="font-medium text-amber-700 dark:text-amber-400">
            Safety Notice
          </span>
        </div>
        {expanded ? (
          <ChevronUp className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
      </button>
      {expanded && (
        <div className="px-3 pb-3 text-xs text-muted-foreground space-y-2">
          <p>
            Duck Kanban runs AI coding agents with{' '}
            <code className="bg-muted px-1 rounded">
              --dangerously-skip-permissions
            </code>{' '}
            / <code className="bg-muted px-1 rounded">--yolo</code> by default,
            giving them unrestricted access to execute code and run commands on
            your system.
          </p>
          <p>
            <strong>Important:</strong> Always review what agents are doing and
            ensure you have backups of important work.
          </p>
          <p>
            Learn more at{' '}
            <a
              href="https://www.duckkanban.com/docs/getting-started#safety-notice"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 dark:text-blue-400 underline hover:no-underline"
            >
              duckkanban.com/docs
            </a>
          </p>
        </div>
      )}
    </div>
  );
}

// --- Step 1: Agent & Editor ---

interface AgentEditorStepProps {
  profile: ExecutorProfileId;
  setProfile: (p: ExecutorProfileId) => void;
  editorType: EditorType;
  setEditorType: (e: EditorType) => void;
  customCommand: string;
  setCustomCommand: (c: string) => void;
}

function AgentEditorStep({
  profile,
  setProfile,
  editorType,
  setEditorType,
  customCommand,
  setCustomCommand,
}: AgentEditorStepProps) {
  const { profiles } = useUserSystem();

  return (
    <div className="space-y-4">
      <SafetyDisclaimer />

      <div className="space-y-2">
        <h3 className="text-base font-medium flex items-center gap-2">
          <Sparkles className="h-4 w-4" />
          Default Coding Agent
        </h3>
        <div className="flex gap-2">
          <Select
            value={profile.executor}
            onValueChange={(v) =>
              setProfile({ executor: v as BaseCodingAgent, variant: null })
            }
          >
            <SelectTrigger className="flex-1">
              <SelectValue placeholder="Select your preferred coding agent" />
            </SelectTrigger>
            <SelectContent>
              {profiles &&
                (Object.keys(profiles) as BaseCodingAgent[]).map((agent) => (
                  <SelectItem key={agent} value={agent}>
                    {agent}
                  </SelectItem>
                ))}
            </SelectContent>
          </Select>

          {(() => {
            const selectedProfile = profiles?.[profile.executor];
            const hasVariants =
              selectedProfile && Object.keys(selectedProfile).length > 0;

            if (hasVariants) {
              return (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="outline"
                      className="w-24 px-2 flex items-center justify-between"
                    >
                      <span className="text-xs truncate flex-1 text-left">
                        {profile.variant || 'DEFAULT'}
                      </span>
                      <ChevronDown className="h-3 w-3 ml-1 flex-shrink-0" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent>
                    {Object.keys(selectedProfile).map((variant) => (
                      <DropdownMenuItem
                        key={variant}
                        onClick={() => setProfile({ ...profile, variant })}
                        className={
                          profile.variant === variant ? 'bg-accent' : ''
                        }
                      >
                        {variant}
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              );
            } else if (selectedProfile) {
              return (
                <Button
                  variant="outline"
                  className="w-24 px-2 flex items-center justify-between"
                  disabled
                >
                  <span className="text-xs truncate flex-1 text-left">
                    Default
                  </span>
                </Button>
              );
            }
            return null;
          })()}
        </div>
      </div>

      <div className="space-y-2">
        <h3 className="text-base font-medium flex items-center gap-2">
          <Code className="h-4 w-4" />
          Code Editor
        </h3>
        <Select
          value={editorType}
          onValueChange={(value: string) => setEditorType(value as EditorType)}
        >
          <SelectTrigger>
            <SelectValue placeholder="Select your preferred editor" />
          </SelectTrigger>
          <SelectContent>
            {Object.values(EditorType).map((type) => (
              <SelectItem key={type} value={type}>
                {toPrettyCase(type)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          Used to open task attempts and project files.
        </p>

        {editorType === EditorType.CUSTOM && (
          <div className="space-y-1.5">
            <Label htmlFor="wizard-custom-command">Custom Command</Label>
            <Input
              id="wizard-custom-command"
              placeholder='e.g., code, subl, vim, "code --wait"'
              value={customCommand}
              onChange={(e) => setCustomCommand(e.target.value)}
            />
          </div>
        )}
      </div>
    </div>
  );
}

// --- Step 2: GitHub Connect ---

interface GitHubStepProps {
  onConnected: () => void;
}

function GitHubStep({ onConnected }: GitHubStepProps) {
  const { config, loading, githubTokenInvalid, reloadSystem } =
    useUserSystem();
  const [fetching, setFetching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deviceState, setDeviceState] =
    useState<DeviceFlowStartResponse | null>(null);
  const [polling, setPolling] = useState(false);
  const [copied, setCopied] = useState(false);

  const isAuthenticated =
    !!(config?.github?.username && config?.github?.oauth_token) &&
    !githubTokenInvalid;

  useEffect(() => {
    if (isAuthenticated) {
      onConnected();
    }
  }, [isAuthenticated, onConnected]);

  const handleLogin = async () => {
    setFetching(true);
    setError(null);
    setDeviceState(null);
    try {
      const data = await githubAuthApi.start();
      setDeviceState(data);
      setPolling(true);
    } catch (e: unknown) {
      const message =
        e instanceof Error ? e.message : 'Network error';
      console.error(e);
      setError(message);
    } finally {
      setFetching(false);
    }
  };

  // Poll for completion
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    if (polling && deviceState) {
      const doPoll = async () => {
        try {
          const pollStatus = await githubAuthApi.poll();
          switch (pollStatus) {
            case DevicePollStatus.SUCCESS:
              setPolling(false);
              setDeviceState(null);
              setError(null);
              await reloadSystem();
              break;
            case DevicePollStatus.AUTHORIZATION_PENDING:
              timer = setTimeout(doPoll, deviceState.interval * 1000);
              break;
            case DevicePollStatus.SLOW_DOWN:
              timer = setTimeout(doPoll, (deviceState.interval + 5) * 1000);
              break;
          }
        } catch (e: unknown) {
          const message =
            e instanceof Error ? e.message : 'Login failed.';
          if (message === 'expired_token') {
            setError('Device code expired. Please try again.');
          } else {
            setError(message);
          }
          setPolling(false);
          setDeviceState(null);
        }
      };
      timer = setTimeout(doPoll, deviceState.interval * 1000);
    }
    return () => {
      if (timer) clearTimeout(timer);
    };
  }, [polling, deviceState, reloadSystem]);

  // Auto-copy code when available
  useEffect(() => {
    if (deviceState?.user_code) {
      copyToClipboard(deviceState.user_code);
    }
  }, [deviceState?.user_code]);

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      console.warn('Copy to clipboard failed');
    }
  };

  if (loading) {
    return <Loader message="Loading..." size={28} className="py-6" />;
  }

  if (isAuthenticated) {
    return (
      <Card>
        <CardContent className="text-center py-6">
          <div className="flex items-center justify-center gap-3 mb-3">
            <Check className="h-7 w-7 text-green-500" />
            <Github className="h-7 w-7 text-muted-foreground" />
          </div>
          <div className="text-base font-medium mb-1">Connected!</div>
          <div className="text-sm text-muted-foreground">
            Signed in as <b>{config?.github?.username ?? ''}</b>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (deviceState) {
    return (
      <div className="space-y-4">
        <div className="flex items-start gap-3">
          <span className="flex-shrink-0 w-8 h-8 bg-background border rounded-full flex items-center justify-center text-sm font-semibold">
            1
          </span>
          <div>
            <p className="text-sm font-medium mb-1">
              Go to GitHub Device Authorization
            </p>
            <a
              href={deviceState.verification_uri}
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm underline"
            >
              {deviceState.verification_uri}
            </a>
          </div>
        </div>

        <div className="flex items-start gap-3">
          <span className="flex-shrink-0 w-8 h-8 bg-background border rounded-full flex items-center justify-center text-sm font-semibold">
            2
          </span>
          <div className="flex-1">
            <p className="text-sm font-medium mb-2">Enter this code:</p>
            <div className="flex items-center gap-2">
              <span className="text-sm font-mono font-bold tracking-[0.2em] bg-muted border flex h-8 px-2 items-center">
                {deviceState.user_code}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => copyToClipboard(deviceState.user_code)}
                disabled={copied}
              >
                {copied ? (
                  <>
                    <Check className="w-3.5 h-3.5 mr-1" />
                    Copied
                  </>
                ) : (
                  <>
                    <Clipboard className="w-3.5 h-3.5 mr-1" />
                    Copy
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs text-muted-foreground bg-muted/50 p-2 rounded-lg">
          <Github className="h-3 w-3 flex-shrink-0" />
          <span>
            {copied
              ? 'Code copied! Complete the authorization on GitHub.'
              : 'Waiting for GitHub authorization...'}
          </span>
        </div>

        {error && <Alert variant="destructive">{error}</Alert>}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Why GitHub access?</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 pt-0">
          {[
            ['Create pull requests', 'Generate PRs from task attempts'],
            ['Manage repositories', 'Push changes and create branches'],
            ['Streamline workflow', 'Skip manual PR creation'],
          ].map(([title, desc]) => (
            <div key={title} className="flex items-start gap-2">
              <Check className="h-4 w-4 text-green-500 mt-0.5 flex-shrink-0" />
              <div>
                <p className="text-sm font-medium">{title}</p>
                <p className="text-xs text-muted-foreground">{desc}</p>
              </div>
            </div>
          ))}
        </CardContent>
      </Card>

      {error && <Alert variant="destructive">{error}</Alert>}

      <Button onClick={handleLogin} disabled={fetching} className="w-full">
        <Github className="h-4 w-4 mr-2" />
        {fetching ? 'Starting...' : 'Sign in with GitHub'}
      </Button>
    </div>
  );
}

// --- Step 3: Privacy ---

interface PrivacyStepProps {
  analyticsEnabled: boolean;
  setAnalyticsEnabled: (enabled: boolean) => void;
}

function PrivacyStep({
  analyticsEnabled,
  setAnalyticsEnabled,
}: PrivacyStepProps) {
  const { config } = useUserSystem();
  const isGitHubAuthenticated =
    config?.github?.username && config?.github?.oauth_token;

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        Help us improve Duck Kanban by sharing usage data.
      </p>

      <div className="space-y-2">
        {isGitHubAuthenticated && (
          <div className="flex items-start gap-2">
            <CheckCircle className="h-4 w-4 text-green-500 mt-0.5 flex-shrink-0" />
            <div className="min-w-0">
              <p className="text-sm font-medium">GitHub profile info</p>
              <p className="text-xs text-muted-foreground">
                Username and email for important updates only
              </p>
            </div>
          </div>
        )}
        <div className="flex items-start gap-2">
          <CheckCircle className="h-4 w-4 text-green-500 mt-0.5 flex-shrink-0" />
          <div className="min-w-0">
            <p className="text-sm font-medium">Usage metrics</p>
            <p className="text-xs text-muted-foreground">
              Tasks created, features used, performance data
            </p>
          </div>
        </div>
        <div className="flex items-start gap-2">
          <XCircle className="h-4 w-4 text-destructive mt-0.5 flex-shrink-0" />
          <div className="min-w-0">
            <p className="text-sm font-medium">We do NOT collect</p>
            <p className="text-xs text-muted-foreground">
              Task contents, code, project names, or personal data
            </p>
          </div>
        </div>
      </div>

      <div className="flex gap-2 pt-2">
        <Button
          variant={analyticsEnabled ? 'outline' : 'default'}
          onClick={() => setAnalyticsEnabled(false)}
          className="flex-1"
        >
          <XCircle className="h-4 w-4 mr-1.5" />
          No thanks
        </Button>
        <Button
          variant={analyticsEnabled ? 'default' : 'outline'}
          onClick={() => setAnalyticsEnabled(true)}
          className="flex-1"
        >
          <CheckCircle className="h-4 w-4 mr-1.5" />
          Yes, help improve
        </Button>
      </div>

      <div className="flex items-center gap-2 text-xs text-muted-foreground bg-muted/50 p-2 rounded-lg">
        <Settings className="h-3 w-3 flex-shrink-0" />
        <span>You can change this anytime in Settings.</span>
      </div>
    </div>
  );
}

// --- Main Wizard ---

const WelcomeWizard = NiceModal.create(() => {
  const modal = useModal();
  const { config } = useUserSystem();

  // Step tracking — start at the first incomplete step
  const getInitialStep = (): number => {
    if (!config) return 0;
    if (!config.onboarding_acknowledged) return 0;
    if (!config.github_login_acknowledged) return 1;
    if (!config.telemetry_acknowledged) return 2;
    return 0;
  };

  const [currentStep, setCurrentStep] = useState(getInitialStep);

  // Agent/Editor state
  const [profile, setProfile] = useState<ExecutorProfileId>(
    config?.executor_profile || {
      executor: BaseCodingAgent.CLAUDE_CODE,
      variant: null,
    }
  );
  const [editorType, setEditorType] = useState<EditorType>(
    config?.editor?.editor_type || EditorType.VS_CODE
  );
  const [customCommand, setCustomCommand] = useState<string>(
    config?.editor?.custom_command || ''
  );

  // GitHub state
  const [githubConnected, setGithubConnected] = useState(false);

  // Privacy state
  const [analyticsEnabled, setAnalyticsEnabled] = useState(false);

  const isAgentEditorValid =
    editorType !== EditorType.CUSTOM ||
    customCommand.trim() !== '';

  const handleNext = () => {
    if (currentStep < TOTAL_STEPS - 1) {
      setCurrentStep((s) => s + 1);
    }
  };

  const handleBack = () => {
    if (currentStep > 0) {
      setCurrentStep((s) => s - 1);
    }
  };

  const handleComplete = () => {
    const result: WelcomeWizardResult = {
      profile,
      editor: {
        editor_type: editorType,
        custom_command:
          editorType === EditorType.CUSTOM ? customCommand || null : null,
      },
      githubConnected,
      analyticsEnabled,
    };
    modal.resolve(result);
  };

  const handleSkipForNow = () => {
    // Resolve with current state — partial completion is persisted by AppShell
    const result: WelcomeWizardResult = {
      profile,
      editor: {
        editor_type: editorType,
        custom_command:
          editorType === EditorType.CUSTOM ? customCommand || null : null,
      },
      githubConnected,
      analyticsEnabled,
    };
    modal.resolve(result);
  };

  const handleGithubConnected = useCallback(() => {
    setGithubConnected(true);
  }, []);

  const stepTitles: Record<StepId, string> = {
    'agent-editor': 'Set up your workspace',
    github: 'Connect GitHub',
    privacy: 'Privacy preferences',
  };

  const currentStepId = WIZARD_STEPS[currentStep].id;

  return (
    <Dialog open={modal.visible} uncloseable={true}>
      <DialogContent className="sm:max-w-[550px]">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <HandMetal className="h-6 w-6 text-primary" />
            <DialogTitle>Welcome to Duck Kanban</DialogTitle>
          </div>
          <DialogDescription className="text-left pt-1">
            {stepTitles[currentStepId]} ({currentStep + 1} of {TOTAL_STEPS})
          </DialogDescription>
        </DialogHeader>

        <StepIndicator currentStep={currentStep} />

        <div className="min-h-[200px]">
          {currentStepId === 'agent-editor' && (
            <AgentEditorStep
              profile={profile}
              setProfile={setProfile}
              editorType={editorType}
              setEditorType={setEditorType}
              customCommand={customCommand}
              setCustomCommand={setCustomCommand}
            />
          )}
          {currentStepId === 'github' && (
            <GitHubStep onConnected={handleGithubConnected} />
          )}
          {currentStepId === 'privacy' && (
            <PrivacyStep
              analyticsEnabled={analyticsEnabled}
              setAnalyticsEnabled={setAnalyticsEnabled}
            />
          )}
        </div>

        <DialogFooter className="gap-2 flex-col sm:flex-row pt-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleSkipForNow}
            className="text-muted-foreground"
          >
            Skip for now
          </Button>
          <div className="flex gap-2 flex-1 justify-end">
            {currentStep > 0 && (
              <Button variant="outline" onClick={handleBack}>
                Back
              </Button>
            )}
            {currentStep < TOTAL_STEPS - 1 ? (
              <Button
                onClick={handleNext}
                disabled={currentStepId === 'agent-editor' && !isAgentEditorValid}
              >
                Next
                <ChevronRight className="h-4 w-4 ml-1" />
              </Button>
            ) : (
              <Button onClick={handleComplete}>
                <Check className="h-4 w-4 mr-1" />
                Get Started
              </Button>
            )}
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

export { WelcomeWizard };
