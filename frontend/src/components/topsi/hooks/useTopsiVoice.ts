import { useState, useRef, useCallback } from 'react';
import { makeRequest } from '@/lib/api';
import { toast } from 'sonner';

interface VoiceInputResult {
  transcription?: string;
  responseText: string;
  audioResponse?: string;
  action?: string;
  actionProjectId?: string;
}

interface UseTopsiVoiceOptions {
  sessionId: string;
  onUserMessage: (content: string) => void;
  onAssistantMessage: (content: string, hasAudio?: boolean) => void;
  onActionComplete: (responseText: string) => void;
  onMeetingAction?: (projectId?: string) => void;
}

export interface TopsiVoiceState {
  isRecording: boolean;
  isInCall: boolean;
  isMuted: boolean;
  isSpeakerOn: boolean;
  isProcessingVoice: boolean;
  audioLevel: number;
}

export interface TopsiVoiceActions {
  startRecording: () => Promise<void>;
  stopRecording: () => void;
  startCall: () => Promise<void>;
  endCall: () => void;
  toggleMute: () => void;
  toggleSpeaker: () => void;
  handlePushToTalkStart: () => void;
  handlePushToTalkEnd: () => void;
  cleanup: () => void;
}

export function useTopsiVoice({
  sessionId,
  onUserMessage,
  onAssistantMessage,
  onActionComplete,
  onMeetingAction,
}: UseTopsiVoiceOptions): [TopsiVoiceState, TopsiVoiceActions] {
  // Voice state
  const [isRecording, setIsRecording] = useState(false);
  const [isInCall, setIsInCall] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isSpeakerOn, setIsSpeakerOn] = useState(true);
  const [isProcessingVoice, setIsProcessingVoice] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);

  // Refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const animationRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const currentAudioRef = useRef<HTMLAudioElement | null>(null);
  const isInCallRef = useRef(false);
  const silenceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hasSpokenRef = useRef(false);

  // Audio level monitoring with silence detection
  const monitorAudioLevel = () => {
    if (!analyserRef.current) return;

    const bufferLength = analyserRef.current.frequencyBinCount;
    const dataArray = new Uint8Array(bufferLength);

    const updateLevel = () => {
      if (!analyserRef.current) return;

      analyserRef.current.getByteFrequencyData(dataArray);
      const average = dataArray.reduce((a, b) => a + b, 0) / bufferLength;
      if (mediaRecorderRef.current?.state === 'recording') {
        setAudioLevel(average / 255);
      }

      // Silence detection in call mode
      if (isInCallRef.current && mediaRecorderRef.current?.state === 'recording') {
        if (average > 15) {
          hasSpokenRef.current = true;
          if (silenceTimerRef.current) {
            clearTimeout(silenceTimerRef.current);
            silenceTimerRef.current = null;
          }
        } else if (hasSpokenRef.current && !silenceTimerRef.current) {
          silenceTimerRef.current = setTimeout(() => {
            silenceTimerRef.current = null;
            hasSpokenRef.current = false;
            if (mediaRecorderRef.current?.state === 'recording') {
              mediaRecorderRef.current.stop();
              setIsRecording(false);
              setAudioLevel(0);
            }
          }, 1500);
        }
      }

      animationRef.current = requestAnimationFrame(updateLevel);
    };

    updateLevel();
  };

  // Convert blob to base64
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

  // Play base64 audio
  const playAudio = async (base64Audio: string, onEnded?: () => void) => {
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
        onEnded?.();
      };
    } catch (error) {
      console.error('Failed to play audio:', error);
      onEnded?.();
    }
  };

  // Process voice input through Topsi API
  const processVoiceInput = async (audioBlob: Blob) => {
    setIsProcessingVoice(true);

    try {
      const base64Audio = await blobToBase64(audioBlob);

      const res = await makeRequest('/api/topsi/voice/interaction', {
        method: 'POST',
        body: JSON.stringify({
          sessionId,
          audioInput: base64Audio,
        }),
      });

      if (res.ok) {
        const data = await res.json();
        const responseData: VoiceInputResult = data.data || data;

        if (responseData.transcription) {
          onUserMessage(responseData.transcription);
        }

        const responseText = responseData.responseText || 'I received your message.';
        const hasAudio = !!(responseData.audioResponse && responseData.audioResponse.length > 100);
        onAssistantMessage(responseText, hasAudio);

        // Play audio in push-to-talk mode (not call mode)
        if (hasAudio && isSpeakerOn && !isInCallRef.current) {
          await playAudio(responseData.audioResponse!);
        }

        // Handle meeting action
        if (responseData.action === 'start_meeting') {
          if (isInCall) {
            setIsInCall(false);
            stopRecordingInternal();
          }
          onMeetingAction?.(responseData.actionProjectId);
          return;
        }

        onActionComplete(responseText);

        // Call mode: restart listening after response
        if (isInCallRef.current && !isMuted) {
          hasSpokenRef.current = false;
          if (hasAudio && isSpeakerOn) {
            await playAudio(responseData.audioResponse!, () => {
              if (isInCallRef.current && !isMuted) startCallRecorder();
            });
          } else {
            setTimeout(() => {
              if (isInCallRef.current && !isMuted) startCallRecorder();
            }, 300);
          }
        }
      }
    } catch (error) {
      console.error('Failed to process voice:', error);
      toast.error('Voice processing failed');
    } finally {
      setIsProcessingVoice(false);
    }
  };

  // Start call recorder on existing stream (no getUserMedia)
  const startCallRecorder = () => {
    if (!streamRef.current || !isInCallRef.current) return;
    audioChunksRef.current = [];
    mediaRecorderRef.current = new MediaRecorder(streamRef.current);

    mediaRecorderRef.current.ondataavailable = (event) => {
      audioChunksRef.current.push(event.data);
    };

    mediaRecorderRef.current.onstop = async () => {
      const blob = new Blob(audioChunksRef.current, { type: 'audio/wav' });
      if (blob.size > 2000 && isInCallRef.current) {
        await processVoiceInput(blob);
      } else if (isInCallRef.current) {
        hasSpokenRef.current = false;
        startCallRecorder();
      }
    };

    mediaRecorderRef.current.start();
    setIsRecording(true);
  };

  // Internal stop recording (without callback dep)
  const stopRecordingInternal = () => {
    if (mediaRecorderRef.current?.state === 'recording') {
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

  // Public start recording (push-to-talk)
  const startRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      audioContextRef.current = new AudioContext();
      const source = audioContextRef.current.createMediaStreamSource(stream);
      analyserRef.current = audioContextRef.current.createAnalyser();
      analyserRef.current.fftSize = 256;
      source.connect(analyserRef.current);
      monitorAudioLevel();

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

  const stopRecording = useCallback(() => {
    stopRecordingInternal();
  }, []);

  // Start a persistent voice call
  const startCall = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      audioContextRef.current = new AudioContext();
      const source = audioContextRef.current.createMediaStreamSource(stream);
      analyserRef.current = audioContextRef.current.createAnalyser();
      analyserRef.current.fftSize = 256;
      source.connect(analyserRef.current);

      isInCallRef.current = true;
      hasSpokenRef.current = false;
      setIsInCall(true);
      onAssistantMessage("I'm listening. Speak when you're ready.");
      monitorAudioLevel();
      startCallRecorder();
    } catch (error) {
      console.error('Failed to start call:', error);
      toast.error('Could not access microphone');
    }
  };

  const endCall = () => {
    isInCallRef.current = false;
    hasSpokenRef.current = false;
    if (silenceTimerRef.current) {
      clearTimeout(silenceTimerRef.current);
      silenceTimerRef.current = null;
    }
    if (mediaRecorderRef.current?.state === 'recording') {
      mediaRecorderRef.current.onstop = null;
      mediaRecorderRef.current.ondataavailable = null;
      mediaRecorderRef.current.stop();
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach(t => t.stop());
      streamRef.current = null;
    }
    analyserRef.current = null;
    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
    if (currentAudioRef.current) {
      currentAudioRef.current.pause();
    }
    setIsInCall(false);
    setIsRecording(false);
    setAudioLevel(0);
    onAssistantMessage("Call ended. Feel free to start another call or type a message.");
  };

  const toggleMute = () => {
    if (isMuted) {
      setIsMuted(false);
      if (isInCallRef.current) {
        hasSpokenRef.current = false;
        startCallRecorder();
      }
    } else {
      setIsMuted(true);
      if (isInCallRef.current) {
        if (mediaRecorderRef.current?.state === 'recording') {
          const rec = mediaRecorderRef.current;
          rec.onstop = null;
          rec.ondataavailable = null;
          rec.stop();
          setIsRecording(false);
        }
      } else {
        stopRecordingInternal();
      }
    }
  };

  const toggleSpeaker = () => setIsSpeakerOn(!isSpeakerOn);

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

  const cleanup = () => {
    stopRecordingInternal();
    if (currentAudioRef.current) {
      currentAudioRef.current.pause();
    }
  };

  const state: TopsiVoiceState = {
    isRecording,
    isInCall,
    isMuted,
    isSpeakerOn,
    isProcessingVoice,
    audioLevel,
  };

  const actions: TopsiVoiceActions = {
    startRecording,
    stopRecording,
    startCall,
    endCall,
    toggleMute,
    toggleSpeaker,
    handlePushToTalkStart,
    handlePushToTalkEnd,
    cleanup,
  };

  return [state, actions];
}
