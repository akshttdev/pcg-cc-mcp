import React, { useState, useEffect, useRef } from 'react';
import ReactMarkdown from 'react-markdown';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Loader } from '@/components/ui/loader';
import {
  Mic,
  MicOff,
  Send,
  Volume2,
  VolumeX,
  Crown,
  Settings
} from 'lucide-react';
import { toast } from 'sonner';

interface SpeechRecognitionAlternative {
  transcript: string;
}

interface SpeechRecognitionResultItem {
  isFinal: boolean;
  length: number;
  [index: number]: SpeechRecognitionAlternative;
}

interface SpeechRecognitionEvent {
  resultIndex: number;
  results: ArrayLike<SpeechRecognitionResultItem>;
}

type SpeechRecognitionConstructor = new () => SpeechRecognition;

interface SpeechRecognition extends EventTarget {
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

interface WindowWithSpeechRecognition extends Window {
  SpeechRecognition?: SpeechRecognitionConstructor;
  webkitSpeechRecognition?: SpeechRecognitionConstructor;
}

interface NoraResponse {
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

type NoraRequestType =
  | 'voiceInteraction'
  | 'textInteraction'
  | 'taskCoordination'
  | 'strategyPlanning'
  | 'performanceAnalysis'
  | 'communicationManagement'
  | 'decisionSupport'
  | 'proactiveNotification'
  | 'cinematicBrief';

type NoraResponseType =
  | 'DirectResponse'
  | 'TaskDelegation'
  | 'StrategyRecommendation'
  | 'PerformanceInsight'
  | 'DecisionSupport'
  | 'CoordinationAction'
  | 'ProactiveAlert';

type RequestPriority =
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

interface ExecutiveAction {
  actionId: string;
  actionType: string;
  description: string;
  parameters: any;
  requiresApproval: boolean;
  estimatedDuration?: string;
  assignedTo?: string;
}

interface ContextUpdate {
  updateType: string;
  key: string;
  value: any;
  confidence: number;
  source: string;
}

interface NoraAssistantProps {
  className?: string;
  defaultSessionId?: string;
}

interface ConversationEntry {
  type: 'user' | 'nora';
  content: string;
  timestamp: Date;
  response?: NoraResponse;
}

const getSpeechRecognitionConstructor = (): SpeechRecognitionConstructor | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  const win = window as WindowWithSpeechRecognition;
  return win.SpeechRecognition ?? win.webkitSpeechRecognition ?? null;
};

export function NoraAssistant({ className, defaultSessionId }: NoraAssistantProps) {
  const [isInitialized, setIsInitialized] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [voiceEnabled, setVoiceEnabled] = useState(false); // Disabled due to quota limits
  const [currentInput, setCurrentInput] = useState('');
  const [conversationHistory, setConversationHistory] = useState<ConversationEntry[]>([]);
  const [interactionMode, setInteractionMode] = useState<'chat' | 'cinematic'>('chat');
  const [cinematicForm, setCinematicForm] = useState({
    projectId: '',
    title: '',
    summary: '',
    styleTags: '',
    assetIds: '',
    autoRender: true
  });

  const sessionId = useRef(defaultSessionId || `session-${Date.now()}`);
  const audioRef = useRef<HTMLAudioElement>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const speechRecognitionRef = useRef<SpeechRecognition | null>(null);
  const hasInitializedRef = useRef(false);
  const [speechRecognitionSupported, setSpeechRecognitionSupported] = useState(false);
  const [interimTranscript, setInterimTranscript] = useState('');

  // Initialize Nora on component mount
  useEffect(() => {
    if (!hasInitializedRef.current) {
      hasInitializedRef.current = true;
      void initializeNora();
    }
    setSpeechRecognitionSupported(getSpeechRecognitionConstructor() !== null);
  }, []);

  const initializeNora = async () => {
    setIsLoading(true);
    try {
      const response = await fetch('/api/nora/initialize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          config: {
            personality: {
              accentStrength: 0.8,
              formalityLevel: "professional",
              warmthLevel: "warm",
              proactiveCommunication: true,
              executiveVocabulary: true,
              britishExpressions: true,
              politenessLevel: "veryPolite"
            },
            voice: {
              tts: {
                provider: "elevenLabs",
                voiceId: "ZtcPZrt9K4w8e1OB9M6w",
                speed: 1.0,
                volume: 0.8,
                pitch: 0.0,
                quality: "high",
                britishVoicePreferences: ["ZtcPZrt9K4w8e1OB9M6w"],
                fallbackProviders: ["system"]
              },
              stt: {
                provider: "system",
                model: "system_stt",
                language: "en-GB",
                britishDialectSupport: true,
                executiveVocabulary: true,
                realTime: false,
                noiseReduction: true
              },
              audio: {
                sampleRate: 44100,
                channels: 2,
                bitDepth: 16,
                bufferSize: 1024,
                noiseSuppression: true,
                echoCancellation: true,
                autoGainControl: true
              },
              britishAccent: {
                accentStrength: 0.8,
                regionalVariant: "receivedPronunciation",
                formalityLevel: "professional",
                vocabularyPreferences: "executive"
              },
              executiveMode: {
                enabled: true,
                proactiveCommunication: true,
                executiveSummaryStyle: true,
                formalAddress: true,
                businessVocabulary: true
              }
            },
            executiveMode: true,
            proactiveNotifications: true,
            contextAwareness: true,
            multiAgentCoordination: true
          },
          activateImmediately: true
        })
      });

      if (response.ok) {
        const payload = (await response.json()) as { message?: string };
        setIsInitialized(true);
        if (payload?.message) {
          const welcomeMessage = payload.message;
          setConversationHistory(prev => {
            const alreadyWelcomed = prev.some(
              entry => entry.type === 'nora' && entry.content === welcomeMessage
            );
            if (alreadyWelcomed) {
              return prev;
            }

            return [
              ...prev,
              {
                type: 'nora',
                content: welcomeMessage,
                timestamp: new Date(),
              }
            ];
          });
        }
      }
    } catch (error) {
      console.error('Failed to initialize Nora:', error);
      addMessage('nora', 'I apologise, but I\'m having difficulty connecting at the moment. Please try again shortly.');
    } finally {
      setIsLoading(false);
    }
  };

  const addMessage = (type: 'user' | 'nora', content: string, response?: NoraResponse) => {
    setConversationHistory(prev => [...prev, {
      type,
      content,
      timestamp: new Date(),
      response
    }]);
  };

  const sendMessage = async (
    content: string,
    requestType: NoraRequestType = 'textInteraction',
    context?: Record<string, unknown> | null
  ) => {
    if (!content.trim() || isLoading) return;

    addMessage('user', content);
    setCurrentInput('');
    setInterimTranscript('');
    setIsLoading(true);

    const request = {
      message: content,
      sessionId: sessionId.current,
      requestType,
      voiceEnabled,
      priority: 'normal' as RequestPriority,
      context: context ?? null
    };

    try {
      const response = await fetch('/api/nora/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request)
      });

      if (response.ok) {
        const noraResponse: NoraResponse = await response.json();
        addMessage('nora', noraResponse.content, noraResponse);
        if (noraResponse.planId) {
          toast.info(`Nora initiated orchestration plan ${noraResponse.planId.slice(0, 8)}…`);
        }

        // Play voice response if available
        if (noraResponse.voiceResponse && voiceEnabled && audioRef.current) {
          const audioElement = audioRef.current;
          audioElement.src = `data:audio/mpeg;base64,${noraResponse.voiceResponse}`;
          audioElement.load();
          audioElement.play().catch(err => {
            console.error('Failed to play Nora voice response:', err);
          });
        }
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      addMessage('nora', 'I apologise, but I encountered an issue processing your request. Please try again.');
    } finally {
      setIsLoading(false);
    }
  };

  const startSpeechRecognition = async () => {
    if (!speechRecognitionSupported) {
      await startMediaRecorder();
      return;
    }

    try {
      const SpeechRecognitionCtor = getSpeechRecognitionConstructor();
      if (!SpeechRecognitionCtor) {
        await startMediaRecorder();
        return;
      }

      const recognition = new SpeechRecognitionCtor();
      recognition.lang = 'en-GB';
      recognition.continuous = false;
      recognition.interimResults = true;

      recognition.onresult = async (event: SpeechRecognitionEvent) => {
        let finalTranscript = '';
        let interim = '';

        for (let i = event.resultIndex; i < event.results.length; i += 1) {
          const result = event.results[i];
          if (!result) {
            continue;
          }
          const alternative = result[0];
          const transcript = alternative?.transcript ?? '';
          if (result.isFinal) {
            finalTranscript += transcript;
          } else {
            interim += transcript;
          }
        }

        if (interim) {
          setInterimTranscript(interim);
        }

        if (finalTranscript.trim()) {
          recognition.stop();
          speechRecognitionRef.current = null;
          setIsListening(false);
          setInterimTranscript('');
          await sendMessage(finalTranscript.trim(), 'voiceInteraction');
        }
      };

      recognition.onerror = async () => {
        speechRecognitionRef.current = null;
        setIsListening(false);
        setInterimTranscript('');
        await startMediaRecorder();
      };

      recognition.onend = () => {
        speechRecognitionRef.current = null;
        setIsListening(false);
        setInterimTranscript('');
      };

      recognition.onspeechend = () => {
        recognition.stop();
      };

      recognition.start();
      speechRecognitionRef.current = recognition;
      setIsListening(true);
    } catch (error) {
      console.error('Speech recognition failed, falling back to recorder:', error);
      await startMediaRecorder();
    }
  };

  const startMediaRecorder = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaRecorderRef.current = new MediaRecorder(stream);
      audioChunksRef.current = [];

      mediaRecorderRef.current.ondataavailable = (event: BlobEvent) => {
        audioChunksRef.current.push(event.data);
      };

      mediaRecorderRef.current.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/wav' });
        const base64Audio = await blobToBase64(audioBlob);

        try {
          const response = await fetch('/api/nora/voice/transcribe', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ audioData: base64Audio })
          });

          if (response.ok) {
            const { text } = (await response.json()) as { text?: string };
            const cleanedText = text?.trim();
            if (cleanedText && !cleanedText.startsWith('This is a dummy transcription')) {
              await sendMessage(cleanedText, 'voiceInteraction');
            } else {
              addMessage('nora', "I couldn't clearly capture that audio. Please try again with a clearer phrase.");
            }
          } else {
            addMessage('nora', 'I was unable to transcribe that audio clip. Let’s give it another go.');
          }
        } catch (error) {
          console.error('Transcription request failed:', error);
          addMessage('nora', 'I ran into an error while transcribing that clip. Could you repeat it for me?');
        }
      };

      mediaRecorderRef.current.start();
      setIsListening(true);
    } catch (error) {
      console.error('Failed to start voice recording:', error);
    }
  };

  const startVoiceRecording = async () => {
    await startSpeechRecognition();
  };

  const stopVoiceRecording = () => {
    if (speechRecognitionRef.current) {
      speechRecognitionRef.current.stop();
      speechRecognitionRef.current = null;
      setInterimTranscript('');
      return;
    }

    if (mediaRecorderRef.current && isListening) {
      mediaRecorderRef.current.stop();
      mediaRecorderRef.current.stream.getTracks().forEach(track => track.stop());
      setIsListening(false);
      setInterimTranscript('');
    }
  };

  const blobToBase64 = (blob: Blob): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        resolve(result.split(',')[1]); // Remove data:audio/wav;base64, prefix
      };
      reader.onerror = reject;
      reader.readAsDataURL(blob);
    });
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (interactionMode === 'cinematic') {
        void sendCinematicBrief();
      } else {
        void sendMessage(currentInput);
      }
    }
  };

  const parseList = (value: string) =>
    value
      .split(',')
      .map(entry => entry.trim())
      .filter(Boolean);

  const sendCinematicBrief = async () => {
    if (!cinematicForm.projectId.trim() || !cinematicForm.title.trim() || !cinematicForm.summary.trim()) {
      toast.error('Project ID, title, and summary are required for cinematic briefs.');
      return;
    }

    const contextPayload = {
      projectId: cinematicForm.projectId.trim(),
      title: cinematicForm.title.trim(),
      summary: cinematicForm.summary.trim(),
      script: currentInput.trim() || undefined,
      assetIds: parseList(cinematicForm.assetIds),
      styleTags: parseList(cinematicForm.styleTags),
      autoRender: cinematicForm.autoRender,
      requesterId: 'executive'
    };

    await sendMessage(
      `Cinematic brief: ${cinematicForm.title.trim()}`,
      'cinematicBrief',
      contextPayload
    );

    setCinematicForm(prev => ({
      ...prev,
      title: '',
      summary: '',
      styleTags: '',
      assetIds: ''
    }));
    setCurrentInput('');
  };

  const handleSend = () => {
    if (!canSend || isLoading) return;
    if (interactionMode === 'cinematic') {
      void sendCinematicBrief();
    } else {
      void sendMessage(currentInput);
    }
  };

  const canSend = interactionMode === 'cinematic'
    ? Boolean(
        cinematicForm.projectId.trim() &&
        cinematicForm.title.trim() &&
        cinematicForm.summary.trim()
      )
    : Boolean((currentInput || interimTranscript).trim());

  return (
    <Card className={`flex flex-col h-full ${className}`}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-lg font-semibold flex items-center gap-2">
          <Crown className="w-5 h-5 text-purple-600" />
          Nora - Executive Assistant
        </CardTitle>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setVoiceEnabled(!voiceEnabled)}
            className={voiceEnabled ? 'text-blue-600' : 'text-gray-400'}
          >
            {voiceEnabled ? <Volume2 className="w-4 h-4" /> : <VolumeX className="w-4 h-4" />}
          </Button>
          <Button variant="ghost" size="sm">
            <Settings className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>

      <CardContent className="flex flex-col flex-1 gap-4 p-4">
        {!isInitialized ? (
          <div className="flex items-center justify-center flex-1">
            <Loader message="Initializing Nora..." size={32} />
          </div>
        ) : (
          <>
            <div className="flex flex-col gap-3">
              <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
                <div className="flex items-center gap-2">
                  <span className="text-xs uppercase text-muted-foreground">Mode</span>
                  <select
                    value={interactionMode}
                    onChange={(event) => setInteractionMode(event.target.value as 'chat' | 'cinematic')}
                    className="border rounded-md px-2 py-1 text-sm focus:outline-none"
                  >
                    <option value="chat">Executive Chat</option>
                    <option value="cinematic">Cinematic Brief</option>
                  </select>
                </div>
                {interactionMode === 'cinematic' && (
                  <Badge variant="outline" className="text-purple-600 border-purple-300">
                    Master Cinematographer engaged
                  </Badge>
                )}
              </div>

              {interactionMode === 'cinematic' && (
                <div className="space-y-3 rounded-lg border border-purple-200/70 bg-purple-50/40 p-3 text-sm">
                  <div className="grid gap-3 md:grid-cols-2">
                    <div>
                      <Label htmlFor="cinematic-project">Project ID</Label>
                      <Input
                        id="cinematic-project"
                        placeholder="UUID for the target project"
                        value={cinematicForm.projectId}
                        onChange={(e) =>
                          setCinematicForm(prev => ({ ...prev, projectId: e.target.value }))
                        }
                      />
                    </div>
                    <div>
                      <Label htmlFor="cinematic-title">Production Title</Label>
                      <Input
                        id="cinematic-title"
                        placeholder="e.g. Neon Metropolis Flythrough"
                        value={cinematicForm.title}
                        onChange={(e) =>
                          setCinematicForm(prev => ({ ...prev, title: e.target.value }))
                        }
                      />
                    </div>
                  </div>

                  <div>
                    <Label htmlFor="cinematic-summary">Creative Summary</Label>
                    <Textarea
                      id="cinematic-summary"
                      rows={3}
                      placeholder="Key beats, tone, and deliverable goals"
                      value={cinematicForm.summary}
                      onChange={(e) =>
                        setCinematicForm(prev => ({ ...prev, summary: e.target.value }))
                      }
                    />
                  </div>

                  <div className="grid gap-3 md:grid-cols-2">
                    <div>
                      <Label htmlFor="cinematic-style">Style Tags</Label>
                      <Input
                        id="cinematic-style"
                        placeholder="cyberpunk, volumetric lighting, slow dolly"
                        value={cinematicForm.styleTags}
                        onChange={(e) =>
                          setCinematicForm(prev => ({ ...prev, styleTags: e.target.value }))
                        }
                      />
                    </div>
                    <div>
                      <Label htmlFor="cinematic-assets">Asset IDs</Label>
                      <Input
                        id="cinematic-assets"
                        placeholder="Comma separated project asset IDs"
                        value={cinematicForm.assetIds}
                        onChange={(e) =>
                          setCinematicForm(prev => ({ ...prev, assetIds: e.target.value }))
                        }
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-between">
                    <div>
                      <Label htmlFor="cinematic-auto">Auto-render via ComfyUI</Label>
                      <p className="text-xs text-muted-foreground">
                        Nora will immediately hand the brief to the Master Cinematographer
                      </p>
                    </div>
                    <Switch
                      id="cinematic-auto"
                      checked={cinematicForm.autoRender}
                      onCheckedChange={(checked) =>
                        setCinematicForm(prev => ({ ...prev, autoRender: checked }))
                      }
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Conversation History */}
            <div className="flex-1 overflow-y-auto space-y-4 min-h-0">
              {conversationHistory.map((message, index) => (
                <div
                  key={index}
                  className={`flex ${message.type === 'user' ? 'justify-end' : 'justify-start'}`}
                >
                  <div
                    className={`max-w-xs lg:max-w-md px-4 py-2 rounded-lg ${
                      message.type === 'user'
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-100 text-gray-800'
                    }`}
                  >
                    <div className="text-sm prose prose-sm max-w-none">
                      {message.type === 'nora' ? (
                        <ReactMarkdown
                          components={{
                            p: ({children}) => <p className="mb-2 last:mb-0">{children}</p>,
                            strong: ({children}) => <strong className="font-semibold">{children}</strong>,
                            ul: ({children}) => <ul className="list-disc ml-4 mb-2">{children}</ul>,
                            ol: ({children}) => <ol className="list-decimal ml-4 mb-2">{children}</ol>,
                            li: ({children}) => <li className="mb-1">{children}</li>,
                          }}
                        >
                          {message.content}
                        </ReactMarkdown>
                      ) : (
                        message.content
                      )}
                    </div>
                    {message.response && (
                      <div className="mt-2 space-y-1">
                        {/* Executive Actions */}
                        {message.response.actions.length > 0 && (
                          <div className="text-xs space-y-1">
                            {message.response.actions.map(action => (
                              <Badge
                                key={action.actionId}
                                variant="secondary"
                                className="text-xs"
                              >
                                {action.actionType}: {action.description}
                              </Badge>
                            ))}
                          </div>
                        )}
                        {/* Follow-up Suggestions */}
                        {message.response.followUpSuggestions.length > 0 && (
                          <div className="text-xs text-gray-600">
                            <span className="font-medium">Suggestions:</span>
                            <ul className="list-disc list-inside ml-2">
                              {message.response.followUpSuggestions.map((suggestion, i) => (
                                <li key={i} className="cursor-pointer hover:text-blue-600"
                                    onClick={() => setCurrentInput(suggestion)}>
                                  {suggestion}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}
                      </div>
                    )}
                    <div className="text-xs opacity-50 mt-1">
                      {message.timestamp.toLocaleTimeString()}
                    </div>
                  </div>
                </div>
              ))}
              {isLoading && (
                <div className="flex justify-start">
                  <div className="bg-gray-100 rounded-lg px-4 py-2">
                    <Loader message="Nora is thinking..." size={16} />
                  </div>
                </div>
              )}
            </div>

            {/* Input Area */}
            <div className="flex gap-2">
              <div className="flex-1">
                <Textarea
                  value={interactionMode === 'cinematic' ? currentInput : (currentInput || interimTranscript)}
                  onChange={(e) => setCurrentInput(e.target.value)}
                  onKeyPress={handleKeyPress}
                  placeholder={
                    interactionMode === 'cinematic'
                      ? 'Optional script or beat sheet for this cinematic brief (Shift+Enter for new line)'
                      : 'Ask Nora anything... (Press Enter to send, Shift+Enter for new line)'
                  }
                  className="min-h-[40px] max-h-[120px] resize-none"
                  disabled={isLoading}
                />
              </div>
              <div className="flex flex-col gap-2">
                <Button
                  onClick={isListening ? stopVoiceRecording : startVoiceRecording}
                  variant={isListening ? "destructive" : "secondary"}
                  size="icon"
                  disabled={isLoading}
                >
                  {isListening ? <MicOff className="w-4 h-4" /> : <Mic className="w-4 h-4" />}
                </Button>
                <Button
                  onClick={handleSend}
                  disabled={!canSend || isLoading}
                  size="icon"
                >
                  <Send className="w-4 h-4" />
                </Button>
              </div>
            </div>

            {/* Voice Conversation Controls */}
            <div className="border-t pt-4">
              <div className="flex items-center justify-between mb-3">
                <h4 className="font-medium text-sm">Voice Conversation</h4>
                <Badge variant={voiceEnabled ? "default" : "secondary"}>
                  {voiceEnabled ? "Voice On" : "Voice Off"}
                </Badge>
              </div>

              <div className="flex gap-3">
                <Button
                  onClick={() => setVoiceEnabled(!voiceEnabled)}
                  variant={voiceEnabled ? "default" : "outline"}
                  className="flex-1"
                >
                  {voiceEnabled ? <Volume2 className="w-4 h-4 mr-2" /> : <VolumeX className="w-4 h-4 mr-2" />}
                  {voiceEnabled ? "Voice Enabled" : "Enable Voice"}
                </Button>

                {voiceEnabled && (
                  <Button
                    onClick={isListening ? stopVoiceRecording : startVoiceRecording}
                    variant={isListening ? "destructive" : "default"}
                    size="lg"
                    className="px-6"
                  >
                    {isListening ? (
                      <>
                        <MicOff className="w-4 h-4 mr-2" />
                        End Call
                      </>
                    ) : (
                      <>
                        <Mic className="w-4 h-4 mr-2" />
                        Start Call
                      </>
                    )}
                  </Button>
                )}
              </div>

              {isListening && (
                <div className="mt-3 p-3 bg-green-50 border border-green-200 rounded-lg">
                  <div className="flex items-center">
                    <div className="w-2 h-2 bg-red-500 rounded-full animate-pulse mr-2"></div>
                    <span className="text-sm text-green-700">
                      Recording... Speak to Nora now. Click "End Call" when finished.
                    </span>
                  </div>
                </div>
              )}
            </div>

            {/* Quick Action Buttons */}
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => sendMessage('Please provide a strategic overview of current projects', 'strategyPlanning')}
              >
                Strategy Overview
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => sendMessage('Show me performance analytics for the team', 'performanceAnalysis')}
              >
                Performance Report
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => sendMessage('Coordinate tasks and priorities', 'taskCoordination')}
              >
                Task Coordination
              </Button>
            </div>
          </>
        )}
      </CardContent>

      {/* Hidden audio element for voice playback */}
      <audio ref={audioRef} style={{ display: 'none' }} />
    </Card>
  );
}
