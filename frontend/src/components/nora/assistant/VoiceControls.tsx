import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Mic, MicOff, Volume2, VolumeX } from 'lucide-react';

interface VoiceControlsProps {
  voiceEnabled: boolean;
  onToggleVoice: () => void;
  isListening: boolean;
  isSpeaking: boolean;
  continuousMode: boolean;
  onStartRecording: () => void;
  onStopRecording: () => void;
}

export function VoiceControls({
  voiceEnabled,
  onToggleVoice,
  isListening,
  isSpeaking,
  continuousMode,
  onStartRecording,
  onStopRecording,
}: VoiceControlsProps) {
  return (
    <div className="border-t pt-4">
      <div className="flex items-center justify-between mb-3">
        <h4 className="font-medium text-sm">Voice Conversation</h4>
        <Badge variant={voiceEnabled ? "default" : "secondary"}>
          {voiceEnabled ? "Voice On" : "Voice Off"}
        </Badge>
      </div>

      <div className="flex gap-3">
        <Button
          onClick={onToggleVoice}
          variant={voiceEnabled ? "default" : "outline"}
          className="flex-1"
        >
          {voiceEnabled ? <Volume2 className="w-4 h-4 mr-2" /> : <VolumeX className="w-4 h-4 mr-2" />}
          {voiceEnabled ? "Voice Enabled" : "Enable Voice"}
        </Button>

        {voiceEnabled && (
          <Button
            onClick={isListening ? onStopRecording : onStartRecording}
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
                onClick={onStartRecording}
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
  );
}
