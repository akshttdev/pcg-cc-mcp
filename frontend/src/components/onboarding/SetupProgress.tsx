// Setup progress indicator — shows incomplete onboarding steps with a button
// to return to the wizard. Displayed in sidebar/settings area.
import { useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Settings, CheckCircle, Circle } from 'lucide-react';
import { useUserSystem } from '@/components/config-provider';
import NiceModal from '@ebay/nice-modal-react';

interface OnboardingStep {
  id: string;
  label: string;
  configKey: 'onboarding_acknowledged' | 'github_login_acknowledged' | 'telemetry_acknowledged';
}

const ONBOARDING_STEPS: OnboardingStep[] = [
  { id: 'agent-editor', label: 'Agent & Editor', configKey: 'onboarding_acknowledged' },
  { id: 'github', label: 'GitHub', configKey: 'github_login_acknowledged' },
  { id: 'privacy', label: 'Privacy', configKey: 'telemetry_acknowledged' },
];

export function SetupProgress() {
  const { config } = useUserSystem();

  const { completedCount, totalSteps, allComplete } = useMemo(() => {
    if (!config) return { completedCount: 0, totalSteps: ONBOARDING_STEPS.length, allComplete: false };

    const completed = ONBOARDING_STEPS.filter(
      (step) => config[step.configKey] === true
    ).length;

    return {
      completedCount: completed,
      totalSteps: ONBOARDING_STEPS.length,
      allComplete: completed === ONBOARDING_STEPS.length,
    };
  }, [config]);

  // Don't show if all steps are complete
  if (allComplete || !config) return null;

  const handleOpenWizard = async () => {
    try {
      await NiceModal.show('welcome-wizard');
    } catch {
      // User dismissed wizard — partial state persists
    }
  };

  return (
    <div className="px-3 py-2">
      <Button
        variant="ghost"
        size="sm"
        onClick={handleOpenWizard}
        className="w-full justify-start gap-2 text-xs text-muted-foreground hover:text-foreground"
      >
        <Settings className="h-3.5 w-3.5" />
        <span>Setup ({completedCount}/{totalSteps})</span>
        <div className="flex gap-0.5 ml-auto">
          {ONBOARDING_STEPS.map((step) => {
            const done = config[step.configKey] === true;
            return done ? (
              <CheckCircle
                key={step.id}
                className="h-3 w-3 text-green-500"
              />
            ) : (
              <Circle
                key={step.id}
                className="h-3 w-3 text-muted-foreground/50"
              />
            );
          })}
        </div>
      </Button>
    </div>
  );
}
