import React, { useState, useEffect, useRef } from 'react';
import { resolveApiUrl } from '@/lib/api';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
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

import type {
  NoraAssistantProps,
  NoraResponse,
  NoraRequestType,
  RequestPriority,
  ConversationEntry,
  CinematicFormState,
  SpeechRecognition,
} from './types';
import { CinematicBriefForm } from './CinematicBriefForm';
import { ConversationHistory } from './ConversationHistory';
import { VoiceControls } from './VoiceControls';

export function NoraAssistant({ className, defaultSessionId }: NoraAssistantProps) {
  const [isInitialized, setIsInitialized] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [voiceEnabled, setVoiceEnabled] = useState(true);
  const [currentInput, setCurrentInput] = useState('');
  const [conversationHistory, setConversationHistory] = useState<ConversationEntry[]>([]);
  const [interactionMode, setInteractionMode] = useState<'chat' | 'cinematic'>('chat');
  const [cinematicForm, setCinematicForm] = useState<CinematicFormState>({
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
            if (alreadyWelcomed) return prev;
            return [
              ...prev,
              { type: 'nora', content: welcomeMessage, timestamp: new Date() }
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
      type, content, timestamp: new Date(), response
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
          playVoiceResponse(noraResponse.voiceResponse);
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
      if (continuousMode && shouldContinueListeningRef.current) {
        setTimeout(() => void startMediaRecorder(), 300);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const playVoiceResponse = (base64Audio: string) => {
    const audioElement = audioRef.current;
    if (!audioElement) return;

    setIsSpeaking(true);

    audioElement.onended = () => {
      setIsSpeaking(false);
      if (continuousMode && shouldContinueListeningRef.current) {
        setTimeout(() => void startMediaRecorder(), 300);
      }
    };

    audioElement.onerror = () => {
      console.error('[Voice Conversation] Audio playback error');
      setIsSpeaking(false);
      if (continuousMode && shouldContinueListeningRef.current) {
        setTimeout(() => void startMediaRecorder(), 300);
      }
    };

    audioElement.src = `data:audio/mpeg;base64,${base64Audio}`;
    audioElement.load();
    audioElement.play().catch(err => {
      console.error('Failed to play Nora voice response:', err);
      setIsSpeaking(false);
      if (continuousMode && shouldContinueListeningRef.current) {
        setTimeout(() => void startMediaRecorder(), 300);
      }
    });
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

            const transcript = result.transcription?.trim();
            addMessage('user', transcript || '🎤 [Voice message]');

            if (result.responseText) {
              addMessage('nora', result.responseText);
            }

            if (result.audioResponse && voiceEnabled && audioRef.current) {
              playVoiceResponse(result.audioResponse);
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
    await startMediaRecorder();
  };

  const stopVoiceRecording = () => {
    shouldContinueListeningRef.current = false;

    if (silenceTimeoutRef.current) {
      clearTimeout(silenceTimeoutRef.current);
      silenceTimeoutRef.current = null;
    }

    if (autoStopTimerRef.current) {
      clearTimeout(autoStopTimerRef.current);
      autoStopTimerRef.current = null;
    }

    if (speechRecognitionRef.current) {
      try {
        speechRecognitionRef.current.stop();
      } catch (e) {
        console.error('Error stopping recognition:', e);
      }
      speechRecognitionRef.current = null;
      setInterimTranscript('');
    }

    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop();
    } else if (mediaRecorderRef.current?.stream) {
      mediaRecorderRef.current.stream.getTracks().forEach(track => track.stop());
    }

    setIsListening(false);

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
        resolve(result.split(',')[1]);
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
    value.split(',').map(entry => entry.trim()).filter(Boolean);

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
            <CinematicBriefForm
              interactionMode={interactionMode}
              onModeChange={setInteractionMode}
              cinematicForm={cinematicForm}
              onFormChange={(updates) => setCinematicForm(prev => ({ ...prev, ...updates }))}
            />

            <ConversationHistory
              messages={conversationHistory}
              isLoading={isLoading}
              onSuggestionClick={setCurrentInput}
            />

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
                <Button
                  onClick={handleSend}
                  disabled={!canSend || isLoading}
                  size="icon"
                >
                  <Send className="w-4 h-4" />
                </Button>
              </div>
            </div>

            <VoiceControls
              voiceEnabled={voiceEnabled}
              onToggleVoice={() => setVoiceEnabled(!voiceEnabled)}
              isListening={isListening}
              isSpeaking={isSpeaking}
              continuousMode={continuousMode}
              onStartRecording={startVoiceRecording}
              onStopRecording={stopVoiceRecording}
            />

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
