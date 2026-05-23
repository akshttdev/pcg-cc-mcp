export interface SpeechRecognitionAlternative {
  transcript: string;
}

export interface SpeechRecognitionResultItem {
  isFinal: boolean;
  length: number;
  [index: number]: SpeechRecognitionAlternative;
}

export interface SpeechRecognitionEvent {
  resultIndex: number;
  results: ArrayLike<SpeechRecognitionResultItem>;
}

export interface SpeechRecognition extends EventTarget {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  onend: ((event: Event) => void) | null;
  onspeechend: (() => void) | null;
  start: () => void;
  stop: () => void;
}

export interface NoraResponse {
  responseId: string;
  requestId: string;
  sessionId: string;
  responseType: NoraResponseType;
  content: string;
  actions: ExecutiveAction[];
  voiceResponse?: string; // Base64 encoded audio
  followUpSuggestions: string[];
  contextUpdates: ContextUpdate[];
  planId?: string;
  timestamp: string;
  processingTimeMs: number;
}

export type NoraRequestType =
  | 'voiceInteraction'
  | 'textInteraction'
  | 'taskCoordination'
  | 'strategyPlanning'
  | 'performanceAnalysis'
  | 'communicationManagement'
  | 'decisionSupport'
  | 'proactiveNotification'
  | 'cinematicBrief';

export type NoraResponseType =
  | 'DirectResponse'
  | 'TaskDelegation'
  | 'StrategyRecommendation'
  | 'PerformanceInsight'
  | 'DecisionSupport'
  | 'CoordinationAction'
  | 'ProactiveAlert';

export type RequestPriority =
  | 'Low'
  | 'Normal'
  | 'High'
  | 'Urgent'
  | 'Executive'
  | 'low'
  | 'normal'
  | 'high'
  | 'urgent'
  | 'executive';

export interface ExecutiveAction {
  actionId: string;
  actionType: string;
  description: string;
  parameters: unknown;
  requiresApproval: boolean;
  estimatedDuration?: string;
  assignedTo?: string;
}

export interface ContextUpdate {
  updateType: string;
  key: string;
  value: unknown;
  confidence: number;
  source: string;
}

export interface NoraAssistantProps {
  className?: string;
  defaultSessionId?: string;
  /** Project ID for file uploads - required for attachment functionality */
  projectId?: string;
}

export interface ConversationEntry {
  type: 'user' | 'nora';
  content: string;
  timestamp: Date;
  response?: NoraResponse;
}

export interface CinematicFormState {
  projectId: string;
  title: string;
  summary: string;
  styleTags: string;
  assetIds: string;
  autoRender: boolean;
}
