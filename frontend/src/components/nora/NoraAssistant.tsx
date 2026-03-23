import {
  Crown,
  Mic,
  MicOff,
  Send,
  Settings,
  Volume2,
  VolumeX} from 'lucide-react';
import React, { useEffect, useRef,useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Loader } from '@/components/ui/loader';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { resolveApiUrl } from '@/lib/api';

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

export function NoraAssistant({ className, defaultSessionId }: NoraAssistantProps) {
  const [isInitialized, setIsInitialized] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false); // Track when Nora is speaking
  const [voiceEnabled, setVoiceEnabled] = useState(true); // Enable voice responses
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
  const [continuousMode, setContinuousMode] = useState(false);

  const sessionId = useRef(defaultSessionId || `session-${Date.now()}`);
  const audioRef = useRef<HTMLAudioElement>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const speechRecognitionRef = useRef<SpeechRecognition | null>(null);
  const hasInitializedRef = useRef(false);
  const [interimTranscript, setInterimTranscript] = useState('');
  
  // VAD (Voice Activity Detection) refs
  const audioContextRef = useRef<AudioContext | null>(null);
  const silenceTimeoutRef = useRef<number | null>(null);
  const speechBufferRef = useRef<Blob[]>([]);
  const shouldContinueListeningRef = useRef(false);
  const autoStopTimerRef = useRef<number | null>(null);

  // Initialize Nora on component mount
  useEffect(() => {
    if (!hasInitializedRef.current) {
      hasInitializedRef.current = true;
      void initializeNora();
    }
  }, []);

  const initializeNora = async () => {
    setIsLoading(true);
    try {
      const response = await fetch(resolveApiUrl('/api/nora/initialize'), {
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
      // Stop listening while we process (prevents feedback)
      if (continuousMode && isListening) {
        stopVoiceRecording();
      }

      const response = await fetch(resolveApiUrl('/api/nora/chat'), {
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
          setIsSpeaking(true);
          
          // Set up handler to resume listening after Nora finishes speaking
          audioElement.onended = () => {
            setIsSpeaking(false);
            
            // In continuous mode, automatically resume listening
            if (continuousMode && shouldContinueListeningRef.current) {
              setTimeout(() => {
                void startMediaRecorder();
              }, 300); // Small delay to prevent picking up tail end of audio
            }
          };
          
          audioElement.onerror = () => {
            console.error('[Voice Conversation] Audio playback error');
            setIsSpeaking(false);
            // Resume listening even on error in continuous mode
            if (continuousMode && shouldContinueListeningRef.current) {
              setTimeout(() => void startMediaRecorder(), 300);
            }
          };
          
          audioElement.src = `data:audio/mpeg;base64,${noraResponse.voiceResponse}`;
          audioElement.load();
          audioElement.play().catch(err => {
            console.error('Failed to play Nora voice response:', err);
            setIsSpeaking(false);
            // Resume listening even on play error in continuous mode
            if (continuousMode && shouldContinueListeningRef.current) {
              setTimeout(() => void startMediaRecorder(), 300);
            }
          });
        } else {
          // No voice response - resume listening immediately in continuous mode
          if (continuousMode && shouldContinueListeningRef.current) {
            setTimeout(() => void startMediaRecorder(), 500);
          }
        }
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      addMessage('nora', 'I apologise, but I encountered an issue processing your request. Please try again.');
      // Resume listening even on error in continuous mode
      if (continuousMode && shouldContinueListeningRef.current) {
        setTimeout(() => void startMediaRecorder(), 300);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const startMediaRecorder = async () => {
    try {
      shouldContinueListeningRef.current = continuousMode;

      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });

      // Use a proper container format — MediaRecorder never produces raw WAV
      const mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
        ? 'audio/webm;codecs=opus'
        : MediaRecorder.isTypeSupported('audio/webm')
        ? 'audio/webm'
        : MediaRecorder.isTypeSupported('audio/ogg;codecs=opus')
        ? 'audio/ogg;codecs=opus'
        : '';

      mediaRecorderRef.current = mimeType
        ? new MediaRecorder(stream, { mimeType })
        : new MediaRecorder(stream);
      audioChunksRef.current = [];

      mediaRecorderRef.current.ondataavailable = (event: BlobEvent) => {
        if (event.data.size > 0) audioChunksRef.current.push(event.data);
      };

      mediaRecorderRef.current.onstop = async () => {
        if (autoStopTimerRef.current) {
          clearTimeout(autoStopTimerRef.current);
          autoStopTimerRef.current = null;
        }

        // Release microphone
        stream.getTracks().forEach(track => track.stop());

        if (audioChunksRef.current.length === 0) {
          toast.warning('No audio captured — please speak and try again');
          return;
        }

        const recordedMime = mediaRecorderRef.current?.mimeType || mimeType || 'audio/webm';
        const audioBlob = new Blob(audioChunksRef.current, { type: recordedMime });
        const base64Audio = await blobToBase64(audioBlob);

        toast.info('Sending to Nora...');
        setIsLoading(true);
        try {
          // Single-shot: Whisper STT → Nora → ElevenLabs TTS
          const response = await fetch(resolveApiUrl('/api/nora/voice/interaction'), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              interactionId: `interaction-${Date.now()}`,
              sessionId: sessionId.current,
              interactionType: 'speechInput',
              audioInput: base64Audio,
              responseText: '',
              processingTimeMs: 0,
              timestamp: new Date().toISOString(),
            }),
          });

          if (response.ok) {
            const result = (await response.json()) as {
              transcription?: string;
              responseText?: string;
              audioResponse?: string;
            };

            // Show transcription as user message
            const transcript = result.transcription?.trim();
            addMessage('user', transcript || '🎤 [Voice message]');

            // Show Nora's text response
            if (result.responseText) {
              addMessage('nora', result.responseText);
            }

            // Play ElevenLabs audio response
            if (result.audioResponse && voiceEnabled && audioRef.current) {
              const audioElement = audioRef.current;
              setIsSpeaking(true);
              audioElement.onended = () => {
                setIsSpeaking(false);
                if (continuousMode && shouldContinueListeningRef.current) {
                  setTimeout(() => void startMediaRecorder(), 300);
                }
              };
              audioElement.onerror = () => {
                setIsSpeaking(false);
                if (continuousMode && shouldContinueListeningRef.current) {
                  setTimeout(() => void startMediaRecorder(), 300);
                }
              };
              audioElement.src = `data:audio/mpeg;base64,${result.audioResponse}`;
              audioElement.load();
              audioElement.play().catch(err => {
                console.error('[NoraVoice] playback error:', err);
                setIsSpeaking(false);
              });
            }
          } else {
            console.error('[NoraVoice] voice/interaction failed:', response.status);
            addMessage('nora', "I couldn't process that audio. Please try again.");
          }
        } catch (error) {
          console.error('[NoraVoice] request failed:', error);
          addMessage('nora', 'I ran into an error while processing your voice message. Could you try again?');
        } finally {
          setIsLoading(false);
        }
      };

      mediaRecorderRef.current.start();
      setIsListening(true);
      toast.info('Recording — speak now, then click the mic again to send to Nora');

      // Auto-stop after 30s
      autoStopTimerRef.current = window.setTimeout(() => {
        if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
          mediaRecorderRef.current.stop();
          setIsListening(false);
        }
      }, 30000);
    } catch (error) {
      console.error('Failed to start voice recording:', error);
      toast.error('Could not access microphone. Please check browser permissions.');
    }
  };

  const startVoiceRecording = async () => {
    // Barge-in: Stop Nora's speech if user starts talking
    if (isSpeaking && audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
      setIsSpeaking(false);
    }
    // Use MediaRecorder+Whisper — Chrome SpeechRecognition needs Google servers
    // and shows browser-level "did not respond" errors when unreachable
    await startMediaRecorder();
  };

  const stopVoiceRecording = () => {
    // Signal that we should stop continuous listening
    shouldContinueListeningRef.current = false;
    
    // Clear any pending silence timeout
    if (silenceTimeoutRef.current) {
      clearTimeout(silenceTimeoutRef.current);
      silenceTimeoutRef.current = null;
    }

    // Clear auto-stop timer
    if (autoStopTimerRef.current) {
      clearTimeout(autoStopTimerRef.current);
      autoStopTimerRef.current = null;
    }

    // Stop speech recognition
    if (speechRecognitionRef.current) {
      try {
        speechRecognitionRef.current.stop();
      } catch (e) {
        console.error('Error stopping recognition:', e);
      }
      speechRecognitionRef.current = null;
      setInterimTranscript('');
    }

    // Stop MediaRecorder if active (triggers onstop → processes audio via Whisper + ElevenLabs)
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop(); // onstop callback releases tracks
    } else if (mediaRecorderRef.current?.stream) {
      mediaRecorderRef.current.stream.getTracks().forEach(track => track.stop());
    }
    
    setIsListening(false);

    // Cleanup audio context
    if (audioContextRef.current) {
      try {
        audioContextRef.current.close();
      } catch (e) {
        console.error('Error closing audio context:', e);
      }
      audioContextRef.current = null;
    }

    speechBufferRef.current = [];
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
          <Button
            variant={continuousMode ? 'default' : 'outline'}
            size="sm"
            onClick={() => {
              setContinuousMode(!continuousMode);
              if (isListening && !continuousMode) {
                // Switching to continuous mode while listening - restart recognition
                stopVoiceRecording();
                setTimeout(() => {
                  void startVoiceRecording();
                }, 100);
              }
            }}
            title={continuousMode ? 'Continuous Mode (Call)' : 'Push-to-Talk Mode'}
            className="text-xs"
          >
            {continuousMode ? '📞 Call' : '🎤 PTT'}
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
                  className="relative"
                  title={continuousMode ? (isListening ? 'End Call' : 'Start Call') : (isListening ? 'Stop Recording' : 'Start Recording')}
                >
                  {isListening ? <MicOff className="w-4 h-4" /> : <Mic className="w-4 h-4" />}
                  {continuousMode && isListening && (
                    <span className="absolute -top-1 -right-1 flex h-3 w-3">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-3 w-3 bg-red-500"></span>
                    </span>
                  )}
                </Button>
                <IconButton
                  onClick={handleSend}
                  disabled={!canSend || isLoading}
                  icon={Send}
                  label="Send message"
                />
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

              {isSpeaking && (
                <div className="mt-3 p-3 bg-blue-50 border border-blue-200 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center">
                      <div className="flex space-x-1 mr-2">
                        <div className="w-1 h-3 bg-blue-500 animate-pulse" style={{ animationDelay: '0ms' }}></div>
                        <div className="w-1 h-4 bg-blue-500 animate-pulse" style={{ animationDelay: '150ms' }}></div>
                        <div className="w-1 h-3 bg-blue-500 animate-pulse" style={{ animationDelay: '300ms' }}></div>
                      </div>
                      <span className="text-sm text-blue-700">
                        Nora is speaking... {continuousMode && "(Tap mic to interrupt)"}
                      </span>
                    </div>
                    {continuousMode && (
                      <Button
                        onClick={startVoiceRecording}
                        variant="outline"
                        size="sm"
                        className="text-blue-600 border-blue-300 hover:bg-blue-100"
                      >
                        <Mic className="w-3 h-3 mr-1" />
                        Interrupt
                      </Button>
                    )}
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
