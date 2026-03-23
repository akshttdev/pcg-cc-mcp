import { Bot, Send } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { type AgentContext,useAgentChatStore } from '@/stores/useAgentChatStore';

type EntityType = AgentContext['entityType'];

const SUGGESTED_QUESTIONS: Record<EntityType, string[]> = {
  project: [
    'Summarize {name}',
    'What tasks are blocked?',
    'Show recent activity',
    'Run a workflow for {name}',
  ],
  task: [
    "What's blocking this?",
    'Assign an agent',
    'Break into subtasks',
    'Update status',
  ],
  crm_contact: [
    'What deals involve {name}?',
    'Draft follow-up email',
    'Find related contacts',
  ],
  crm_deal: [
    'Pipeline status for {name}?',
    'Create tasks to advance',
    'Summarize deal history',
  ],
};

const ENTITY_LABELS: Record<EntityType, string> = {
  project: 'project',
  task: 'task',
  crm_contact: 'contact',
  crm_deal: 'deal',
};

interface AskTopsiButtonProps {
  entityType: EntityType;
  entityId: string;
  entityName: string;
  agent?: string;
  className?: string;
}

export function AskTopsiButton({
  entityType,
  entityId,
  entityName,
  agent = 'topsi',
  className,
}: AskTopsiButtonProps) {
  const [freeText, setFreeText] = useState('');
  const [open, setOpen] = useState(false);
  const openChat = useAgentChatStore((s) => s.openChat);

  const context: AgentContext = { entityType, entityId, entityName };

  const handleSend = (message: string) => {
    if (!message.trim()) return;
    openChat(message.trim(), context, agent);
    setFreeText('');
    setOpen(false);
  };

  const questions = SUGGESTED_QUESTIONS[entityType].map((q) =>
    q.replace('{name}', entityName)
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" className={className}>
          <Bot className="h-4 w-4 mr-1.5" />
          Ask Topsi
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-3" align="start">
        <p className="text-sm font-medium mb-2">
          Ask Topsi about this {ENTITY_LABELS[entityType]}
        </p>
        <div className="space-y-1 mb-3">
          {questions.map((q) => (
            <button
              key={q}
              onClick={() => handleSend(q)}
              className="w-full text-left text-sm px-2 py-1.5 rounded hover:bg-accent transition-colors text-muted-foreground hover:text-foreground"
            >
              {q}
            </button>
          ))}
        </div>
        <div className="flex gap-1.5">
          <Input
            placeholder="Ask something..."
            value={freeText}
            onChange={(e) => setFreeText(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSend(freeText)}
            className="text-sm h-8"
          />
          <IconButton
            className="h-8 w-8 shrink-0"
            onClick={() => handleSend(freeText)}
            disabled={!freeText.trim()}
            icon={Send}
            label="Send"
            iconClassName="h-3.5 w-3.5"
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}
