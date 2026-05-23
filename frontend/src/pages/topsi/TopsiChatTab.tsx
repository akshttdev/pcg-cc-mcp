import {
  AlertTriangle,
  FolderOpen,
  Loader2,
  Mic,
  MicOff,
  RefreshCw,
  Send,
  Volume2,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import {
  ChatAttachmentButton,
  ChatAttachmentList,
  useChatAttachments,
} from '@/components/chat/ChatAttachments';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import type { TopologyOverview, TopsiStatusResponse } from '@/lib/api';
import { topsiApi } from '@/lib/api';
import { cn } from '@/lib/utils';

import type { TopsiChatMessage } from './types';

interface TopsiChatTabProps {
  status: TopsiStatusResponse | null;
  topology: TopologyOverview | null;
  sessionId: string;
  fetchTopology: () => void;
  fetchIssues: () => void;
  fetchProjects: () => void;
  /** Project ID for file uploads - required for attachment functionality */
  projectId?: string;
}

export function TopsiChatTab({
  status,
  topology,
  sessionId,
  fetchTopology,
  fetchIssues,
  fetchProjects,
  projectId,
}: TopsiChatTabProps) {
  const [messages, setMessages] = useState<TopsiChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hello! I'm Topsi, the PCG Platform Intelligence Agent. I manage projects, coordinate agents, and ensure data isolation between clients. How can I help you today?",
      timestamp: new Date(),
    },
  ]);
  const [inputMessage, setInputMessage] = useState('');
  const [isSending, setIsSending] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Voice state
  const [isRecording, setIsRecording] = useState(false);
  const [isProcessingVoice, setIsProcessingVoice] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, unused-imports/no-unused-vars
  const [isSpeakerOn, _setIsSpeakerOn] = useState(true);

  // Attachment support - requires projectId prop
  const {
    attachments,
    attachmentIds,
    isUploading,
    addFiles,
    removeAttachment,
    clearAttachments,
  } = useChatAttachments({
    projectId: projectId || 'default',
  });

  // Voice refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const animationRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const currentAudioRef = useRef<HTMLAudioElement | null>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Send message to Topsi
  const sendMessage = useCallback(async () => {
    if (!inputMessage.trim() || isSending) return;

    const userMessage: TopsiChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: inputMessage,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    const messageToSend = inputMessage;
    setInputMessage('');
    setIsSending(true);

    // Create abort controller with 2 minute timeout (Ollama fallback can be slow)
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 120000);

    try {
      const data = await topsiApi.chat(
        {
          message: messageToSend,
          sessionId,
          attachmentIds: attachmentIds.length > 0 ? attachmentIds : undefined,
        },
        controller.signal
      );
      clearTimeout(timeoutId);

      // Clear attachments after successful send
      clearAttachments();

      const assistantMessage: TopsiChatMessage = {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        content: data.message ?? '',
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
    } catch (error) {
      clearTimeout(timeoutId);
      console.error('Failed to send message:', error);
      if (error instanceof Error && error.name === 'AbortError') {
        toast.error('Request timed out - Topsi may be busy');
      } else {
        toast.error('Failed to send message');
      }
    } finally {
      setIsSending(false);
    }
  }, [inputMessage, isSending, sessionId, attachmentIds, clearAttachments]);

  // Voice recording functions
  const startRecording = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      // Audio level monitoring
      audioContextRef.current = new AudioContext();
      const source = audioContextRef.current.createMediaStreamSource(stream);
      analyserRef.current = audioContextRef.current.createAnalyser();
      analyserRef.current.fftSize = 256;
      source.connect(analyserRef.current);

      const bufferLength = analyserRef.current.frequencyBinCount;
      const dataArray = new Uint8Array(bufferLength);

      const updateLevel = () => {
        if (!analyserRef.current || !isRecording) return;
        analyserRef.current.getByteFrequencyData(dataArray);
        const average = dataArray.reduce((a, b) => a + b, 0) / bufferLength;
        setAudioLevel(average / 255);
        animationRef.current = requestAnimationFrame(updateLevel);
      };

      // Media recorder
      mediaRecorderRef.current = new MediaRecorder(stream);
      audioChunksRef.current = [];

      mediaRecorderRef.current.ondataavailable = (event) => {
        audioChunksRef.current.push(event.data);
      };

      mediaRecorderRef.current.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, {
          type: 'audio/wav',
        });
        await processVoiceInput(audioBlob);
      };

      mediaRecorderRef.current.start();
      setIsRecording(true);
      updateLevel();
    } catch (error) {
      console.error('Failed to start recording:', error);
      toast.error('Could not access microphone');
    }
  }, []);

  const stopRecording = useCallback(() => {
    if (mediaRecorderRef.current && isRecording) {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
      setAudioLevel(0);
    }

    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
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
  }, [isRecording]);

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

  const playAudio = async (base64Audio: string) => {
    if (!isSpeakerOn) return;
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

      audio.onended = () => {
        URL.revokeObjectURL(audioUrl);
      };
    } catch (error) {
      console.error('Failed to play audio:', error);
    }
  };

  const processVoiceInput = async (audioBlob: Blob) => {
    setIsProcessingVoice(true);

    try {
      const base64Audio = await blobToBase64(audioBlob);

      const voiceResult = await topsiApi.voiceInteraction(
        sessionId,
        base64Audio
      );
      const responseData = voiceResult.data || voiceResult;

      // Add user's transcribed message
      if (responseData.transcription) {
        const userMessage: TopsiChatMessage = {
          id: `user-${Date.now()}`,
          role: 'user',
          content: responseData.transcription,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, userMessage]);
      }

      // Add Topsi's response
      const responseText =
        responseData.responseText || 'I received your message.';
      const hasAudio = !!(
        responseData.audioResponse && responseData.audioResponse.length > 100
      );
      const assistantMessage: TopsiChatMessage = {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        content: responseText,
        timestamp: new Date(),
        hasAudio,
      };
      setMessages((prev) => [...prev, assistantMessage]);

      // Play audio response
      if (hasAudio) {
        await playAudio(responseData.audioResponse!);
      }
    } catch (error) {
      console.error('Failed to process voice:', error);
      toast.error('Voice processing failed');
    } finally {
      setIsProcessingVoice(false);
    }
  };

  // Push-to-talk handlers
  const handlePushToTalkStart = useCallback(() => {
    if (!isProcessingVoice && !isSending) {
      startRecording();
    }
  }, [isProcessingVoice, isSending, startRecording]);

  const handlePushToTalkEnd = useCallback(() => {
    if (isRecording) {
      stopRecording();
    }
  }, [isRecording, stopRecording]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopRecording();
      if (currentAudioRef.current) {
        currentAudioRef.current.pause();
      }
    };
  }, [stopRecording]);

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-full min-h-[600px]">
      {/* Chat Interface */}
      <div className="lg:col-span-2 flex flex-col h-full min-h-0">
        <Card className="flex-1 flex flex-col min-h-0 h-full">
          <CardHeader className="flex-shrink-0">
            <CardTitle className="text-lg">Chat with Topsi</CardTitle>
            <CardDescription>
              Ask questions, get insights, or manage your projects
            </CardDescription>
          </CardHeader>
          <CardContent className="flex-1 flex flex-col min-h-0 overflow-hidden p-4">
            <ScrollArea className="flex-1 min-h-0 h-[400px] lg:h-[500px] pr-4">
              <div className="space-y-4 pb-4">
                {messages.map((msg) => (
                  <div
                    key={msg.id}
                    className={cn(
                      'flex',
                      msg.role === 'user' ? 'justify-end' : 'justify-start'
                    )}
                  >
                    <div
                      className={cn(
                        'max-w-[80%] rounded-lg px-4 py-2',
                        msg.role === 'user'
                          ? 'bg-cyan-600 text-white'
                          : 'bg-muted'
                      )}
                    >
                      <p className="text-sm whitespace-pre-wrap">
                        {msg.content}
                      </p>
                      <div
                        className={cn(
                          'flex items-center gap-2 text-xs mt-1',
                          msg.role === 'user'
                            ? 'text-cyan-100'
                            : 'text-muted-foreground'
                        )}
                      >
                        <span>{msg.timestamp.toLocaleTimeString()}</span>
                        {msg.hasAudio && msg.role === 'assistant' && (
                          <Volume2 className="w-3 h-3" />
                        )}
                      </div>
                    </div>
                  </div>
                ))}
                <div ref={messagesEndRef} />
              </div>
            </ScrollArea>
            {/* Audio level indicator when recording */}
            {isRecording && (
              <div className="mb-2">
                <div className="h-1 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-cyan-500 transition-all duration-100"
                    style={{ width: `${audioLevel * 100}%` }}
                  />
                </div>
                <p className="text-xs text-center text-muted-foreground mt-1">
                  Listening...
                </p>
              </div>
            )}
            {isProcessingVoice && (
              <div className="mb-2 flex items-center justify-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="w-4 h-4 animate-spin" />
                Processing voice...
              </div>
            )}
            {/* Attachment Previews */}
            {attachments.length > 0 && (
              <ChatAttachmentList
                attachments={attachments}
                onRemove={removeAttachment}
                disabled={isSending || isUploading}
                className="mb-2"
              />
            )}
            <div className="flex gap-2 mt-4 flex-shrink-0">
              {/* Attachment button - only show when project is available */}
              {projectId && (
                <ChatAttachmentButton
                  onFilesSelected={addFiles}
                  disabled={isSending || isUploading}
                />
              )}
              {/* Push-to-talk button */}
              <Button
                variant={isRecording ? 'destructive' : 'outline'}
                size="icon"
                className="shrink-0"
                onMouseDown={handlePushToTalkStart}
                onMouseUp={handlePushToTalkEnd}
                onMouseLeave={handlePushToTalkEnd}
                onTouchStart={handlePushToTalkStart}
                onTouchEnd={handlePushToTalkEnd}
                disabled={isProcessingVoice || isSending}
                title="Hold to talk"
              >
                {isRecording ? (
                  <MicOff className="w-4 h-4" />
                ) : (
                  <Mic className="w-4 h-4" />
                )}
              </Button>
              <Input
                placeholder="Ask Topsi anything..."
                value={inputMessage}
                onChange={(e) => setInputMessage(e.target.value)}
                onKeyDown={(e) =>
                  e.key === 'Enter' && !e.shiftKey && sendMessage()
                }
                disabled={isSending || isRecording}
              />
              <Button
                onClick={sendMessage}
                disabled={isSending || !inputMessage.trim()}
                className="bg-cyan-600 hover:bg-cyan-700"
              >
                {isSending ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Send className="w-4 h-4" />
                )}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground mt-2 text-center">
              Hold the mic button to speak, or type your message
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Quick Stats */}
      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Platform Status</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                Access Level
              </span>
              <Badge variant="outline" className="capitalize">
                {status?.accessScope || 'unknown'}
              </Badge>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                Projects Visible
              </span>
              <span className="font-medium">
                {status?.projectsVisible || 0}
              </span>
            </div>
            {topology && (
              <>
                <Separator />
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Total Nodes
                  </span>
                  <span className="font-medium">{topology.totalNodes}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Total Edges
                  </span>
                  <span className="font-medium">{topology.totalEdges}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Clusters
                  </span>
                  <span className="font-medium">{topology.totalClusters}</span>
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Quick Actions</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <Button
              variant="outline"
              className="w-full justify-start"
              onClick={fetchTopology}
            >
              <RefreshCw className="w-4 h-4 mr-2" />
              Refresh Topology
            </Button>
            <Button
              variant="outline"
              className="w-full justify-start"
              onClick={fetchIssues}
            >
              <AlertTriangle className="w-4 h-4 mr-2" />
              Detect Issues
            </Button>
            <Button
              variant="outline"
              className="w-full justify-start"
              onClick={fetchProjects}
            >
              <FolderOpen className="w-4 h-4 mr-2" />
              Refresh Projects
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
