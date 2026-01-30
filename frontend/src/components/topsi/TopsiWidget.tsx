import { useState, useEffect, useRef } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import {
  Network,
  X,
  Send,
  Mic,
  MicOff,
  Phone,
  PhoneOff,
  Volume2,
  VolumeX,
  Loader2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  hasAudio?: boolean;
}

interface TopsiWidgetProps {
  className?: string;
}

type WidgetState = 'collapsed' | 'chat' | 'call';

export function TopsiWidget({ className }: TopsiWidgetProps) {
  // Widget state
  const [widgetState, setWidgetState] = useState<WidgetState>('collapsed');
  const [isInitialized, setIsInitialized] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);

  // Chat state
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputMessage, setInputMessage] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [sessionId] = useState(() => `topsi-widget-${Date.now()}`);

  // Voice state
  const [isRecording, setIsRecording] = useState(false);
  const [isInCall, setIsInCall] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isSpeakerOn, setIsSpeakerOn] = useState(true);
  const [isProcessingVoice, setIsProcessingVoice] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const animationRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const currentAudioRef = useRef<HTMLAudioElement | null>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Check Topsi status on mount
  useEffect(() => {
    checkTopsiStatus();
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopRecording();
      if (currentAudioRef.current) {
        currentAudioRef.current.pause();
      }
    };
  }, []);

  const checkTopsiStatus = async () => {
    try {
      const res = await fetch('/api/topsi/status');
      if (res.ok) {
        const data = await res.json();
        setIsInitialized(data.isActive);
      }
    } catch (error) {
      console.error('Failed to check Topsi status:', error);
    }
  };

  const initializeTopsi = async () => {
    setIsInitializing(true);
    try {
      const res = await fetch('/api/topsi/initialize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ activateImmediately: true }),
      });
      if (res.ok) {
        setIsInitialized(true);
        addMessage('assistant', "Hello! I'm Topsi, your platform orchestrator. How can I help you today?");
      } else {
        toast.error('Failed to initialize Topsi');
      }
    } catch (error) {
      console.error('Failed to initialize Topsi:', error);
      toast.error('Failed to connect to Topsi');
    } finally {
      setIsInitializing(false);
    }
  };

  const addMessage = (role: 'user' | 'assistant', content: string, hasAudio?: boolean) => {
    const message: ChatMessage = {
      id: `${role}-${Date.now()}`,
      role,
      content,
      timestamp: new Date(),
      hasAudio,
    };
    setMessages(prev => [...prev, message]);
  };

  // Text chat
  const sendTextMessage = async () => {
    if (!inputMessage.trim() || isSending) return;

    const userMessage = inputMessage.trim();
    setInputMessage('');
    addMessage('user', userMessage);
    setIsSending(true);

    try {
      const res = await fetch('/api/topsi/voice/interaction', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sessionId,
          textInput: userMessage,
        }),
      });

      if (res.ok) {
        const data = await res.json();
        const responseData = data.data || data;
        const responseText = responseData.responseText || 'I received your message.';
        const hasAudio = responseData.audioResponse && responseData.audioResponse.length > 100;

        addMessage('assistant', responseText, hasAudio);

        // Play audio if available and speaker is on
        if (hasAudio && isSpeakerOn) {
          playAudio(responseData.audioResponse);
        }
      } else {
        addMessage('assistant', 'Sorry, I encountered an error processing your request.');
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      addMessage('assistant', 'Sorry, I lost connection. Please try again.');
    } finally {
      setIsSending(false);
    }
  };

  // Voice recording
  const startRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      // Audio level monitoring
      audioContextRef.current = new AudioContext();
      const source = audioContextRef.current.createMediaStreamSource(stream);
      analyserRef.current = audioContextRef.current.createAnalyser();
      analyserRef.current.fftSize = 256;
      source.connect(analyserRef.current);
      monitorAudioLevel();

      // Media recorder
      mediaRecorderRef.current = new MediaRecorder(stream);
      audioChunksRef.current = [];

      mediaRecorderRef.current.ondataavailable = (event) => {
        audioChunksRef.current.push(event.data);
      };

      mediaRecorderRef.current.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/wav' });
        await processVoiceInput(audioBlob);
      };

      mediaRecorderRef.current.start();
      setIsRecording(true);
    } catch (error) {
      console.error('Failed to start recording:', error);
      toast.error('Could not access microphone');
    }
  };

  const stopRecording = () => {
    if (mediaRecorderRef.current && isRecording) {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
      setAudioLevel(0);
    }

    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop());
      streamRef.current = null;
    }

    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }

    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
  };

  const monitorAudioLevel = () => {
    if (!analyserRef.current) return;

    const bufferLength = analyserRef.current.frequencyBinCount;
    const dataArray = new Uint8Array(bufferLength);

    const updateLevel = () => {
      if (!analyserRef.current || !isRecording) return;

      analyserRef.current.getByteFrequencyData(dataArray);
      const average = dataArray.reduce((a, b) => a + b, 0) / bufferLength;
      setAudioLevel(average / 255);

      animationRef.current = requestAnimationFrame(updateLevel);
    };

    updateLevel();
  };

  const processVoiceInput = async (audioBlob: Blob) => {
    setIsProcessingVoice(true);

    try {
      const base64Audio = await blobToBase64(audioBlob);

      const res = await fetch('/api/topsi/voice/interaction', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sessionId,
          audioInput: base64Audio,
        }),
      });

      if (res.ok) {
        const data = await res.json();
        const responseData = data.data || data;

        // Add user's transcribed message
        if (responseData.transcription) {
          addMessage('user', responseData.transcription);
        }

        // Add Topsi's response
        const responseText = responseData.responseText || 'I received your message.';
        const hasAudio = responseData.audioResponse && responseData.audioResponse.length > 100;
        addMessage('assistant', responseText, hasAudio);

        // Play audio response
        if (hasAudio && isSpeakerOn) {
          await playAudio(responseData.audioResponse);
        }

        // If in call mode, continue listening after response
        if (isInCall && !isMuted) {
          setTimeout(() => startRecording(), 500);
        }
      }
    } catch (error) {
      console.error('Failed to process voice:', error);
      toast.error('Voice processing failed');
    } finally {
      setIsProcessingVoice(false);
    }
  };

  const playAudio = async (base64Audio: string) => {
    try {
      const audioData = atob(base64Audio);
      const audioBuffer = new Uint8Array(audioData.length);
      for (let i = 0; i < audioData.length; i++) {
        audioBuffer[i] = audioData.charCodeAt(i);
      }

      const audioBlob = new Blob([audioBuffer], { type: 'audio/wav' });
      const audioUrl = URL.createObjectURL(audioBlob);

      if (currentAudioRef.current) {
        currentAudioRef.current.pause();
      }

      const audio = new Audio(audioUrl);
      currentAudioRef.current = audio;

      await audio.play();

      // Cleanup after playing
      audio.onended = () => {
        URL.revokeObjectURL(audioUrl);
      };
    } catch (error) {
      console.error('Failed to play audio:', error);
    }
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

  // Call mode
  const startCall = async () => {
    setIsInCall(true);
    setWidgetState('call');
    addMessage('assistant', "I'm listening. Speak when you're ready.");
    await startRecording();
  };

  const endCall = () => {
    setIsInCall(false);
    stopRecording();
    if (currentAudioRef.current) {
      currentAudioRef.current.pause();
    }
    addMessage('assistant', "Call ended. Feel free to start another call or type a message.");
  };

  const toggleMute = () => {
    if (isMuted) {
      setIsMuted(false);
      if (isInCall) startRecording();
    } else {
      setIsMuted(true);
      stopRecording();
    }
  };

  // Push-to-talk handlers
  const handlePushToTalkStart = () => {
    if (!isInCall) {
      startRecording();
    }
  };

  const handlePushToTalkEnd = () => {
    if (!isInCall && isRecording) {
      stopRecording();
    }
  };

  const openWidget = () => {
    if (!isInitialized) {
      initializeTopsi();
    }
    setWidgetState('chat');
  };

  // Collapsed state - just the floating button
  if (widgetState === 'collapsed') {
    return (
      <div className={cn("fixed bottom-6 right-6 z-50", className)}>
        <Button
          onClick={openWidget}
          className="h-14 w-14 rounded-full bg-cyan-600 hover:bg-cyan-700 shadow-lg hover:shadow-xl transition-all"
          disabled={isInitializing}
        >
          {isInitializing ? (
            <Loader2 className="h-6 w-6 animate-spin" />
          ) : (
            <Network className="h-6 w-6" />
          )}
        </Button>
        <span className="absolute -top-1 -right-1 flex h-4 w-4">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-cyan-400 opacity-75"></span>
          <span className="relative inline-flex rounded-full h-4 w-4 bg-cyan-500"></span>
        </span>
      </div>
    );
  }

  // Expanded widget
  return (
    <div className={cn(
      "fixed bottom-6 right-6 z-50 flex flex-col bg-background border rounded-xl shadow-2xl transition-all",
      widgetState === 'call' ? "w-80 h-96" : "w-96 h-[32rem]",
      className
    )}>
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b bg-cyan-600 text-white rounded-t-xl">
        <div className="flex items-center gap-2">
          <Network className="h-5 w-5" />
          <span className="font-semibold">Topsi</span>
          {isInCall && (
            <Badge variant="secondary" className="bg-green-500 text-white text-xs">
              On Call
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-1">
          {widgetState === 'chat' && (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 text-white hover:bg-cyan-700"
              onClick={startCall}
              title="Start voice call"
            >
              <Phone className="h-4 w-4" />
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-white hover:bg-cyan-700"
            onClick={() => setWidgetState('collapsed')}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Call Mode UI */}
      {widgetState === 'call' && (
        <div className="flex-1 flex flex-col items-center justify-center p-6 gap-6">
          {/* Audio visualization */}
          <div className="relative">
            <div className={cn(
              "w-24 h-24 rounded-full bg-cyan-100 dark:bg-cyan-900 flex items-center justify-center transition-all",
              isRecording && "ring-4 ring-cyan-400 ring-opacity-50"
            )}
            style={{
              transform: `scale(${1 + audioLevel * 0.3})`,
            }}>
              <Network className="h-12 w-12 text-cyan-600" />
            </div>
            {isProcessingVoice && (
              <div className="absolute inset-0 flex items-center justify-center">
                <Loader2 className="h-8 w-8 animate-spin text-cyan-600" />
              </div>
            )}
          </div>

          <div className="text-center">
            <p className="text-sm text-muted-foreground">
              {isProcessingVoice ? 'Processing...' : isRecording ? 'Listening...' : isMuted ? 'Muted' : 'Ready'}
            </p>
          </div>

          {/* Call controls */}
          <div className="flex items-center gap-4">
            <Button
              variant={isMuted ? "destructive" : "outline"}
              size="icon"
              className="h-12 w-12 rounded-full"
              onClick={toggleMute}
            >
              {isMuted ? <MicOff className="h-5 w-5" /> : <Mic className="h-5 w-5" />}
            </Button>

            <Button
              variant="destructive"
              size="icon"
              className="h-14 w-14 rounded-full"
              onClick={endCall}
            >
              <PhoneOff className="h-6 w-6" />
            </Button>

            <Button
              variant={isSpeakerOn ? "outline" : "secondary"}
              size="icon"
              className="h-12 w-12 rounded-full"
              onClick={() => setIsSpeakerOn(!isSpeakerOn)}
            >
              {isSpeakerOn ? <Volume2 className="h-5 w-5" /> : <VolumeX className="h-5 w-5" />}
            </Button>
          </div>

          {/* Switch to chat */}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setWidgetState('chat')}
            className="text-muted-foreground"
          >
            Switch to text chat
          </Button>
        </div>
      )}

      {/* Chat Mode UI */}
      {widgetState === 'chat' && (
        <>
          {/* Messages */}
          <ScrollArea className="flex-1 p-3">
            <div className="space-y-3">
              {messages.length === 0 && (
                <div className="text-center text-muted-foreground py-8">
                  <Network className="h-12 w-12 mx-auto mb-3 opacity-30" />
                  <p className="text-sm">Start a conversation with Topsi</p>
                </div>
              )}
              {messages.map((msg) => (
                <div
                  key={msg.id}
                  className={cn(
                    "flex",
                    msg.role === 'user' ? "justify-end" : "justify-start"
                  )}
                >
                  <div
                    className={cn(
                      "max-w-[85%] rounded-lg px-3 py-2 text-sm",
                      msg.role === 'user'
                        ? "bg-cyan-600 text-white"
                        : "bg-muted"
                    )}
                  >
                    <p className="whitespace-pre-wrap">{msg.content}</p>
                    <div className={cn(
                      "flex items-center gap-2 mt-1 text-xs",
                      msg.role === 'user' ? "text-cyan-100" : "text-muted-foreground"
                    )}>
                      <span>{msg.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                      {msg.hasAudio && msg.role === 'assistant' && (
                        <Volume2 className="h-3 w-3" />
                      )}
                    </div>
                  </div>
                </div>
              ))}
              {(isSending || isProcessingVoice) && (
                <div className="flex justify-start">
                  <div className="bg-muted rounded-lg px-3 py-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                  </div>
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>
          </ScrollArea>

          {/* Input area */}
          <div className="p-3 border-t">
            {/* Audio level indicator when recording */}
            {isRecording && (
              <div className="mb-2">
                <div className="h-1 bg-gray-200 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-cyan-500 transition-all duration-100"
                    style={{ width: `${audioLevel * 100}%` }}
                  />
                </div>
              </div>
            )}

            <div className="flex items-center gap-2">
              {/* Push-to-talk button */}
              <Button
                variant={isRecording ? "destructive" : "outline"}
                size="icon"
                className="h-10 w-10 shrink-0"
                onMouseDown={handlePushToTalkStart}
                onMouseUp={handlePushToTalkEnd}
                onMouseLeave={handlePushToTalkEnd}
                onTouchStart={handlePushToTalkStart}
                onTouchEnd={handlePushToTalkEnd}
                disabled={isProcessingVoice}
                title="Hold to talk"
              >
                {isRecording ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
              </Button>

              {/* Text input */}
              <Input
                placeholder="Type a message..."
                value={inputMessage}
                onChange={(e) => setInputMessage(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && sendTextMessage()}
                disabled={isSending || isRecording}
                className="flex-1"
              />

              {/* Send button */}
              <Button
                size="icon"
                className="h-10 w-10 shrink-0 bg-cyan-600 hover:bg-cyan-700"
                onClick={sendTextMessage}
                disabled={isSending || !inputMessage.trim()}
              >
                {isSending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Send className="h-4 w-4" />
                )}
              </Button>
            </div>

            {/* Quick actions */}
            <div className="flex items-center justify-between mt-2 text-xs text-muted-foreground">
              <span>Hold mic for push-to-talk</span>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 text-xs"
                onClick={startCall}
              >
                <Phone className="h-3 w-3 mr-1" />
                Start call
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

export default TopsiWidget;
