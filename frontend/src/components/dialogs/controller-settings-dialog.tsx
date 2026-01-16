import { useState, useEffect } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { Loader2, Bot, Sparkles } from 'lucide-react';
import { projectControllersApi, type ProjectControllerConfig, type UpdateControllerConfig } from '@/lib/api';
import { toast } from 'sonner';

interface ControllerSettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  config: ProjectControllerConfig | undefined;
}

const PERSONALITY_OPTIONS = [
  { value: 'professional', label: 'Professional', description: 'Formal and business-focused communication' },
  { value: 'friendly', label: 'Friendly', description: 'Warm and approachable tone' },
  { value: 'technical', label: 'Technical', description: 'Detailed and precise explanations' },
  { value: 'creative', label: 'Creative', description: 'Imaginative and innovative approach' },
  { value: 'concise', label: 'Concise', description: 'Brief and to-the-point responses' },
];

const MODEL_OPTIONS = [
  { value: 'gpt-4o-mini', label: 'GPT-4o Mini', description: 'Fast and cost-effective' },
  { value: 'gpt-4o', label: 'GPT-4o', description: 'Most capable model' },
  { value: 'claude-3-haiku', label: 'Claude 3 Haiku', description: 'Fast Anthropic model' },
  { value: 'claude-3-sonnet', label: 'Claude 3 Sonnet', description: 'Balanced Anthropic model' },
];

export function ControllerSettingsDialog({
  open,
  onOpenChange,
  projectId,
  config,
}: ControllerSettingsDialogProps) {
  const queryClient = useQueryClient();

  // Form state
  const [name, setName] = useState(config?.name || 'Controller');
  const [personality, setPersonality] = useState(config?.personality || 'professional');
  const [systemPrompt, setSystemPrompt] = useState(config?.system_prompt || '');
  const [model, setModel] = useState(config?.model || 'gpt-4o-mini');
  const [temperature, setTemperature] = useState(config?.temperature || 0.7);
  const [maxTokens, setMaxTokens] = useState(config?.max_tokens || 2048);

  // Update form when config changes
  useEffect(() => {
    if (config) {
      setName(config.name);
      setPersonality(config.personality);
      setSystemPrompt(config.system_prompt || '');
      setModel(config.model || 'gpt-4o-mini');
      setTemperature(config.temperature || 0.7);
      setMaxTokens(config.max_tokens || 2048);
    }
  }, [config]);

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: (data: UpdateControllerConfig) =>
      projectControllersApi.updateConfig(projectId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-controller-config', projectId] });
      toast.success('Controller settings have been saved successfully.');
      onOpenChange(false);
    },
    onError: (error) => {
      toast.error('Failed to update controller settings.');
      console.error('Failed to update controller:', error);
    },
  });

  const handleSave = () => {
    updateMutation.mutate({
      name,
      personality,
      system_prompt: systemPrompt || undefined,
      model,
      temperature,
      max_tokens: maxTokens,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Bot className="h-5 w-5 text-purple-600" />
            Controller Settings
          </DialogTitle>
          <DialogDescription>
            Customize your project controller's personality and behavior.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Name */}
          <div className="space-y-2">
            <Label htmlFor="name">Controller Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Controller"
            />
            <p className="text-xs text-muted-foreground">
              The name shown in the chat interface
            </p>
          </div>

          {/* Personality */}
          <div className="space-y-2">
            <Label htmlFor="personality">Personality</Label>
            <Select value={personality} onValueChange={setPersonality}>
              <SelectTrigger>
                <SelectValue placeholder="Select personality" />
              </SelectTrigger>
              <SelectContent>
                {PERSONALITY_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    <div className="flex flex-col">
                      <span>{option.label}</span>
                      <span className="text-xs text-muted-foreground">
                        {option.description}
                      </span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Model */}
          <div className="space-y-2">
            <Label htmlFor="model">AI Model</Label>
            <Select value={model} onValueChange={setModel}>
              <SelectTrigger>
                <SelectValue placeholder="Select model" />
              </SelectTrigger>
              <SelectContent>
                {MODEL_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    <div className="flex flex-col">
                      <span>{option.label}</span>
                      <span className="text-xs text-muted-foreground">
                        {option.description}
                      </span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Temperature */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Temperature</Label>
              <span className="text-sm text-muted-foreground">{temperature.toFixed(1)}</span>
            </div>
            <Slider
              value={[temperature]}
              onValueChange={([value]) => setTemperature(value)}
              min={0}
              max={1}
              step={0.1}
              className="w-full"
            />
            <p className="text-xs text-muted-foreground">
              Lower values make responses more focused, higher values more creative
            </p>
          </div>

          {/* Max Tokens */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Max Response Length</Label>
              <span className="text-sm text-muted-foreground">{maxTokens} tokens</span>
            </div>
            <Slider
              value={[maxTokens]}
              onValueChange={([value]) => setMaxTokens(value)}
              min={256}
              max={4096}
              step={256}
              className="w-full"
            />
          </div>

          {/* System Prompt */}
          <div className="space-y-2">
            <Label htmlFor="systemPrompt" className="flex items-center gap-2">
              <Sparkles className="h-4 w-4" />
              Custom Instructions (Optional)
            </Label>
            <Textarea
              id="systemPrompt"
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              placeholder="Add custom instructions for how the controller should behave..."
              rows={4}
            />
            <p className="text-xs text-muted-foreground">
              Additional context or instructions for the controller
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={updateMutation.isPending}>
            {updateMutation.isPending && (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            )}
            Save Changes
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
