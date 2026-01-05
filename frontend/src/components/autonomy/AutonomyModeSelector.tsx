import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Bot, Users, Eye } from 'lucide-react';
import {
  useTaskAutonomyMode,
  useSetTaskAutonomyMode,
  type AutonomyMode,
} from '@/hooks/useAutonomy';

interface AutonomyModeSelectorProps {
  taskId: string;
  className?: string;
}

const AUTONOMY_MODES: {
  value: AutonomyMode;
  label: string;
  description: string;
  icon: React.ReactNode;
  badge: { variant: 'default' | 'secondary' | 'outline'; label: string };
}[] = [
  {
    value: 'agent_driven',
    label: 'Agent-Driven',
    description: 'Agent executes fully, human reviews final output. Best for greenfield, low-risk, trusted patterns.',
    icon: <Bot className="h-5 w-5" />,
    badge: { variant: 'default', label: 'Full Autonomy' },
  },
  {
    value: 'agent_assisted',
    label: 'Agent-Assisted',
    description: 'Agent proposes, human approves major steps. Gates on large changes and external calls.',
    icon: <Users className="h-5 w-5" />,
    badge: { variant: 'secondary', label: 'Balanced' },
  },
  {
    value: 'review_driven',
    label: 'Review-Driven',
    description: 'Human drives, agent assists. All major actions require approval.',
    icon: <Eye className="h-5 w-5" />,
    badge: { variant: 'outline', label: 'High Control' },
  },
];

export function AutonomyModeSelector({ taskId, className }: AutonomyModeSelectorProps) {
  const { data: currentMode, isLoading } = useTaskAutonomyMode(taskId);
  const { mutate: setMode, isPending } = useSetTaskAutonomyMode();

  const handleModeChange = (mode: AutonomyMode) => {
    setMode({ taskId, mode });
  };

  if (isLoading) {
    return (
      <Card className={className}>
        <CardHeader>
          <CardTitle className="text-base">Autonomy Mode</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 bg-muted rounded-lg" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle className="text-base">Autonomy Mode</CardTitle>
        <CardDescription>
          Control how much autonomy the agent has during execution
        </CardDescription>
      </CardHeader>
      <CardContent>
        <RadioGroup
          value={currentMode || 'agent_assisted'}
          onValueChange={(value) => handleModeChange(value as AutonomyMode)}
          disabled={isPending}
          className="space-y-3"
        >
          {AUTONOMY_MODES.map((mode) => (
            <div
              key={mode.value}
              className={`flex items-start space-x-3 p-3 rounded-lg border cursor-pointer transition-colors ${
                currentMode === mode.value
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:border-primary/50'
              }`}
              onClick={() => !isPending && handleModeChange(mode.value)}
            >
              <RadioGroupItem value={mode.value} id={mode.value} className="mt-1" />
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  {mode.icon}
                  <Label
                    htmlFor={mode.value}
                    className="font-medium cursor-pointer"
                  >
                    {mode.label}
                  </Label>
                  <Badge variant={mode.badge.variant}>{mode.badge.label}</Badge>
                </div>
                <p className="text-sm text-muted-foreground mt-1">
                  {mode.description}
                </p>
              </div>
            </div>
          ))}
        </RadioGroup>
      </CardContent>
    </Card>
  );
}
