import React, { useState, useEffect, useRef } from 'react';
import ReactMarkdown from 'react-markdown';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
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

interface SpeechRecognitionErrorEvent extends Event {
  error: string;
  message: string;
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
  | 'proactiveNotification';

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
  const [isSpeaking, setIsSpeaking] = useState(false); // Track when Nora is speaking
  const [voiceEnabled, setVoiceEnabled] = useState(true); // Enable voice responses
  const [currentInput, setCurrentInput] = useState('');
  const [conversationHistory, setConversationHistory] = useState<ConversationEntry[]>([]);
  const [continuousMode, setContinuousMode] = useState(false);

  const sessionId = useRef(defaultSessionId || `session-${Date.now()}`);
  const audioRef = useRef<HTMLAudioElement>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const speechRecognitionRef = useRef<SpeechRecognition | null>(null);
  const hasInitializedRef = useRef(false);
  const [speechRecognitionSupported, setSpeechRecognitionSupported] = useState(false);
  const [interimTranscript, setInterimTranscript] = useState('');
  
  // VAD (Voice Activity Detection) refs
  const audioContextRef = useRef<AudioContext | null>(null);
  const silenceTimeoutRef = useRef<number | null>(null);
  const speechBufferRef = useRef<Blob[]>([]);
  const shouldContinueListeningRef = useRef(false);
  const networkErrorRetryCount = useRef(0);
  const abortedErrorCount = useRef(0);
  const lastAbortedTime = useRef(0);

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

  const sendMessage = async (content: string, requestType: NoraRequestType = 'textInteraction') => {
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
      context: null
    };

    try {
      // Stop listening while we process (prevents feedback)
      if (continuousMode && isListening) {
        console.log('[Voice Conversation] Pausing listening while processing...');
        stopVoiceRecording();
      }

      const response = await fetch('/api/nora/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request)
      });

      if (response.ok) {
        const noraResponse: NoraResponse = await response.json();
        addMessage('nora', noraResponse.content, noraResponse);

        // Play voice response if available
        if (noraResponse.voiceResponse && voiceEnabled && audioRef.current) {
          const audioElement = audioRef.current;
          setIsSpeaking(true);
          
          // Set up handler to resume listening after Nora finishes speaking
          audioElement.onended = () => {
            console.log('[Voice Conversation] Nora finished speaking');
            setIsSpeaking(false);
            
            // In continuous mode, automatically resume listening
            if (continuousMode && shouldContinueListeningRef.current) {
              console.log('[Voice Conversation] Auto-resuming listening after Nora spoke');
              setTimeout(() => {
                void startSpeechRecognition();
              }, 300); // Small delay to prevent picking up tail end of audio
            }
          };
          
          audioElement.onerror = () => {
            console.error('[Voice Conversation] Audio playback error');
            setIsSpeaking(false);
            // Resume listening even on error in continuous mode
            if (continuousMode && shouldContinueListeningRef.current) {
              setTimeout(() => void startSpeechRecognition(), 300);
            }
          };
          
          audioElement.src = `data:audio/mpeg;base64,${noraResponse.voiceResponse}`;
          audioElement.load();
          audioElement.play().catch(err => {
            console.error('Failed to play Nora voice response:', err);
            setIsSpeaking(false);
            // Resume listening even on play error in continuous mode
            if (continuousMode && shouldContinueListeningRef.current) {
              setTimeout(() => void startSpeechRecognition(), 300);
            }
          });
        } else {
          // No voice response - resume listening immediately in continuous mode
          if (continuousMode && shouldContinueListeningRef.current) {
            console.log('[Voice Conversation] No voice response, resuming listening');
            setTimeout(() => void startSpeechRecognition(), 100);
          }
        }
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      addMessage('nora', 'I apologise, but I encountered an issue processing your request. Please try again.');
      // Resume listening even on error in continuous mode
      if (continuousMode && shouldContinueListeningRef.current) {
        setTimeout(() => void startSpeechRecognition(), 300);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const startSpeechRecognition = async () => {
    // Don't start if Nora is currently speaking (prevents feedback)
    if (isSpeaking) {
      console.log('[Voice Conversation] Skipping start - Nora is speaking');
      return;
    }
    
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
      recognition.continuous = continuousMode; // Enable continuous mode
      recognition.interimResults = true;

      // Track if we should continue listening (for continuous mode)
      shouldContinueListeningRef.current = continuousMode;
      networkErrorRetryCount.current = 0; // Reset retry counter on successful start
      abortedErrorCount.current = 0; // Reset aborted error counter

      let accumulatedTranscript = '';
      let lastUpdateTime = Date.now();

      recognition.onresult = async (event: SpeechRecognitionEvent) => {
        console.log('[Speech Recognition] onresult fired, resultIndex:', event.resultIndex, 'results.length:', event.results.length);
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

        const now = Date.now();
        lastUpdateTime = now;

        console.log('[Speech Recognition] Processed - final:', finalTranscript, 'interim:', interim);

        if (interim) {
          setInterimTranscript(accumulatedTranscript + ' ' + interim);
        }

        if (finalTranscript.trim()) {
          accumulatedTranscript += (accumulatedTranscript ? ' ' : '') + finalTranscript.trim();
          setInterimTranscript(accumulatedTranscript);
          
          // Update last speech time
          lastUpdateTime = Date.now();

          // In continuous mode, set a timer to send after pause
          if (continuousMode) {
            // Clear existing timeout
            if (silenceTimeoutRef.current) {
              clearTimeout(silenceTimeoutRef.current);
            }
            
            console.log('[Pause Detection] Setting timeout, accumulated:', accumulatedTranscript);
            
            // Monitor for silence - send after 1.5s of no new speech
            silenceTimeoutRef.current = window.setTimeout(async () => {
              const timeSinceLastUpdate = Date.now() - lastUpdateTime;
              console.log('[Pause Detection] Timeout fired, time since last update:', timeSinceLastUpdate, 'ms');
              console.log('[Pause Detection] Accumulated text:', accumulatedTranscript);
              
              // Check if enough time has passed since last update
              if (timeSinceLastUpdate >= 1400 && accumulatedTranscript.trim()) {
                const messageToSend = accumulatedTranscript.trim();
                console.log('[Pause Detection] Sending message:', messageToSend);
                accumulatedTranscript = '';
                setInterimTranscript('');
                await sendMessage(messageToSend, 'voiceInteraction');
              } else {
                console.log('[Pause Detection] Not sending - timeSinceLastUpdate:', timeSinceLastUpdate, 'accumulated:', accumulatedTranscript);
              }
            }, 1500);
          } else {
            // In push-to-talk mode, send immediately
            recognition.stop();
            speechRecognitionRef.current = null;
            setIsListening(false);
            setInterimTranscript('');
            await sendMessage(accumulatedTranscript.trim(), 'voiceInteraction');
            accumulatedTranscript = '';
          }
        }
      };

      recognition.onerror = async (event: Event) => {
        const errorEvent = event as SpeechRecognitionErrorEvent;
        console.log('[Speech Recognition] Error:', errorEvent.error);
        
        // In continuous mode, handle errors more gracefully
        if (shouldContinueListeningRef.current) {
          // Track rapid aborted errors - circuit breaker
          if (errorEvent.error === 'aborted') {
            const now = Date.now();
            // If getting aborted errors more than once per second, we have a problem
            if (now - lastAbortedTime.current < 1000) {
              abortedErrorCount.current += 1;
            } else {
              abortedErrorCount.current = 1;
            }
            lastAbortedTime.current = now;
            
            // Circuit breaker: if more than 10 aborted errors in quick succession
            if (abortedErrorCount.current > 10) {
              console.error('[Speech Recognition] Too many aborted errors, stopping continuous mode');
              addMessage('nora', 'Speech recognition is experiencing technical difficulties. Switching to Push-to-Talk mode (🎤).');
              setContinuousMode(false);
              shouldContinueListeningRef.current = false;
              speechRecognitionRef.current = null;
              setIsListening(false);
              return;
            }
            
            console.log('[Speech Recognition] Ignoring benign error in continuous mode:', errorEvent.error);
            return;
          }
          
          // Ignore no-speech errors - they're normal
          if (errorEvent.error === 'no-speech') {
            console.log('[Speech Recognition] Ignoring benign error in continuous mode:', errorEvent.error);
            return;
          }
          
          // Network errors - try to restart after a delay (max 3 retries)
          if (errorEvent.error === 'network') {
            networkErrorRetryCount.current += 1;
            
            if (networkErrorRetryCount.current <= 3) {
              console.log(`[Speech Recognition] Network error, retry ${networkErrorRetryCount.current}/3 in 2 seconds...`);
              setTimeout(() => {
                if (shouldContinueListeningRef.current) {
                  console.log('[Speech Recognition] Retrying after network error...');
                  void startSpeechRecognition();
                }
              }, 2000);
              return;
            } else {
              console.log('[Speech Recognition] Max network retries reached, stopping');
              addMessage('nora', 'I\'m unable to connect to the speech recognition service. This could be due to network issues or browser limitations. Please try using Push-to-Talk mode (🎤 button) instead, which works offline.');
              // Auto-switch to push-to-talk mode
              setContinuousMode(false);
            }
          }
          
          // Audio capture errors - might be temporary
          if (errorEvent.error === 'audio-capture' || errorEvent.error === 'not-allowed') {
            console.log('[Speech Recognition] Audio error:', errorEvent.error);
            addMessage('nora', `I'm having trouble accessing your microphone. Error: ${errorEvent.error}`);
          }
        }
        
        // Fatal error or push-to-talk mode - stop everything
        console.log('[Speech Recognition] Fatal error, stopping');
        shouldContinueListeningRef.current = false;
        speechRecognitionRef.current = null;
        setIsListening(false);
        setInterimTranscript('');
        
        if (!continuousMode) {
          await startMediaRecorder();
        }
      };

      recognition.onend = () => {
        console.log('[Speech Recognition] onend fired, shouldContinue:', shouldContinueListeningRef.current);
        console.log('[Speech Recognition] speechRecognitionRef.current exists:', !!speechRecognitionRef.current);
        
        // In continuous mode, restart recognition automatically
        if (shouldContinueListeningRef.current) {
          // Use longer delay to prevent rapid restart loops
          setTimeout(() => {
            console.log('[Speech Recognition] Timeout fired, attempting restart...');
            console.log('[Speech Recognition] Ref still exists:', !!speechRecognitionRef.current);
            console.log('[Speech Recognition] Should still continue:', shouldContinueListeningRef.current);
            
            if (speechRecognitionRef.current && shouldContinueListeningRef.current) {
              try {
                console.log('[Speech Recognition] Calling recognition.start()');
                recognition.start();
                console.log('[Speech Recognition] Successfully restarted');
              } catch (error) {
                console.error('[Speech Recognition] Error during restart:', error);
                // If already started, that's fine - ignore the error
                if (error instanceof Error && !error.message.includes('already started')) {
                  speechRecognitionRef.current = null;
                  setIsListening(false);
                  setInterimTranscript('');
                }
              }
            } else {
              console.log('[Speech Recognition] Skipping restart - ref or shouldContinue is false');
            }
          }, 300); // Increased from 100ms to 300ms to prevent rapid loops
        } else {
          console.log('[Speech Recognition] Not restarting, cleaning up');
          speechRecognitionRef.current = null;
          setIsListening(false);
          setInterimTranscript('');
        }
      };

      recognition.onspeechend = () => {
        console.log('[Speech Recognition] onspeechend fired');
        // In push-to-talk mode, stop on speech end
        // In continuous mode, keep listening
        if (!continuousMode) {
          recognition.stop();
        }
      };

      // Add additional event listeners for debugging (not in TypeScript types but exist in runtime)
      (recognition as any).onstart = () => {
        console.log('[Speech Recognition] onstart fired - recognition is now active');
      };

      (recognition as any).onsoundstart = () => {
        console.log('[Speech Recognition] onsoundstart - sound detected');
      };

      (recognition as any).onsoundend = () => {
        console.log('[Speech Recognition] onsoundend - sound ended');
      };

      (recognition as any).onspeechstart = () => {
        console.log('[Speech Recognition] onspeechstart - speech detected');
      };

      console.log('[Speech Recognition] Starting recognition in', continuousMode ? 'CONTINUOUS' : 'PUSH-TO-TALK', 'mode');
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
    // Barge-in: Stop Nora's speech if user starts talking
    if (isSpeaking && audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
      setIsSpeaking(false);
    }
    await startSpeechRecognition();
  };

  const stopVoiceRecording = () => {
    // Signal that we should stop continuous listening
    shouldContinueListeningRef.current = false;
    
    // Clear any pending silence timeout
    if (silenceTimeoutRef.current) {
      clearTimeout(silenceTimeoutRef.current);
      silenceTimeoutRef.current = null;
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

    // Stop media stream (used for VAD in continuous mode)
    if (mediaRecorderRef.current?.stream) {
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
      sendMessage(currentInput);
    }
  };

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
                  value={currentInput || interimTranscript}
                  onChange={(e) => setCurrentInput(e.target.value)}
                  onKeyPress={handleKeyPress}
                  placeholder="Ask Nora anything... (Press Enter to send, Shift+Enter for new line)"
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
                  onClick={() => sendMessage(currentInput)}
                  disabled={!currentInput.trim() || isLoading}
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
