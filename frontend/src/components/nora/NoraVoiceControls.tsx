import { useState, useEffect, useRef } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { Loader } from '@/components/ui/loader';
import {
  Mic,
  MicOff,
  Volume2,
  Crown,
  Languages
} from 'lucide-react';

// Type definitions for voice configuration
interface VoiceConfig {
  tts: {
    provider: string;
    voiceId: string;
    speed: number;
    volume: number;
    pitch: number;
    quality: string;
    britishVoicePreferences: string[];
  };
  stt: {
    provider: string;
    model: string;
    language: string;
    britishDialectSupport: boolean;
    executiveVocabulary: boolean;
  };
  britishAccent: {
    accentStrength: number;
    regionalVariant: string;
    formalityLevel: string;
    vocabularyPreferences: string;
  };
  executiveMode: {
    enabled: boolean;
    proactiveCommunication: boolean;
    executiveSummaryStyle: boolean;
    formalAddress: boolean;
    businessVocabulary: boolean;
  };
}

interface TranscriptionResult {
  text: string;
  confidence: number;
  language: string;
  processingTimeMs: number;
}

interface NoraVoiceControlsProps {
  className?: string;
  onConfigChange?: (config: VoiceConfig) => void;
}

export function NoraVoiceControls({ className, onConfigChange }: NoraVoiceControlsProps) {
  const [config, setConfig] = useState<VoiceConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRecording, setIsRecording] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<TranscriptionResult | null>(null);
  const [audioLevel, setAudioLevel] = useState(0);

  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const animationRef = useRef<number | null>(null);

  const mergeVoiceConfig = (base: VoiceConfig, partial: Partial<VoiceConfig>): VoiceConfig => ({
    ...base,
    ...partial,
    tts: { ...base.tts, ...(partial.tts ?? {}) },
    stt: { ...base.stt, ...(partial.stt ?? {}) },
    britishAccent: { ...base.britishAccent, ...(partial.britishAccent ?? {}) },
    executiveMode: { ...base.executiveMode, ...(partial.executiveMode ?? {}) },
  });

  useEffect(() => {
    void fetchVoiceConfig();
    return () => {
      if (animationRef.current !== null) {
        window.cancelAnimationFrame(animationRef.current);
        animationRef.current = null;
      }
    };
  }, []);

  const fetchVoiceConfig = async () => {
    try {
      setIsLoading(true);
      const response = await fetch('/api/nora/voice/config');
      if (response.ok) {
        const payload = (await response.json()) as { config: VoiceConfig };
        setConfig(payload.config);
      } else if (response.status === 404) {
        setConfig(null);
      }
    } catch (error) {
      console.error('Failed to fetch voice config:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const updateConfig = async (newConfig: Partial<VoiceConfig>) => {
    if (!config) return;

    const updatedConfig = mergeVoiceConfig(config, newConfig);
    setConfig(updatedConfig);

    try {
      const response = await fetch('/api/nora/voice/config', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ config: updatedConfig })
      });

      if (response.ok && onConfigChange) {
        const payload = (await response.json()) as { config: VoiceConfig };
        onConfigChange(payload.config);
      }
    } catch (error) {
      console.error('Failed to update voice config:', error);
    }
  };

  const startRecordingTest = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });

      // Set up audio context for level monitoring
      audioContextRef.current = new AudioContext();
      const source = audioContextRef.current.createMediaStreamSource(stream);
      analyserRef.current = audioContextRef.current.createAnalyser();
      analyserRef.current.fftSize = 256;
      source.connect(analyserRef.current);

      // Start monitoring audio levels
      monitorAudioLevel();

      // Set up media recorder
      mediaRecorderRef.current = new MediaRecorder(stream);
      audioChunksRef.current = [];

      mediaRecorderRef.current.ondataavailable = (event: BlobEvent) => {
        audioChunksRef.current.push(event.data);
      };

      mediaRecorderRef.current.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/wav' });
        const base64Audio = await blobToBase64(audioBlob);

        // Send for transcription
        try {
          setIsTesting(true);
          const response = await fetch('/api/nora/voice/transcribe', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ audioData: base64Audio })
          });

          if (response.ok) {
            const data = await response.json();
            setTestResult({
              text: data.text,
              confidence: data.confidence ?? 0,
              language: data.language ?? 'en-GB',
              processingTimeMs: data.processingTimeMs ?? 0
            });
          }
        } catch (error) {
          console.error('Transcription failed:', error);
        } finally {
          setIsTesting(false);
        }
      };

      mediaRecorderRef.current.start();
      setIsRecording(true);
      setTestResult(null);
    } catch (error) {
      console.error('Failed to start recording:', error);
    }
  };

  const stopRecordingTest = () => {
    if (mediaRecorderRef.current && isRecording) {
      mediaRecorderRef.current.stop();
      mediaRecorderRef.current.stream.getTracks().forEach(track => track.stop());
      setIsRecording(false);
      setAudioLevel(0);

      if (audioContextRef.current) {
        audioContextRef.current.close();
      }

      if (animationRef.current !== null) {
        window.cancelAnimationFrame(animationRef.current);
        animationRef.current = null;
      }
    }
  };

  const monitorAudioLevel = () => {
    if (!analyserRef.current) return;

    const bufferLength = analyserRef.current.frequencyBinCount;
    const dataArray = new Uint8Array(bufferLength);

    const updateLevel = () => {
      if (!analyserRef.current) return;

      analyserRef.current.getByteFrequencyData(dataArray);
      const average = dataArray.reduce((a, b) => a + b, 0) / Math.max(bufferLength, 1);
      setAudioLevel(average / 255);

      if (isRecording) {
        animationRef.current = window.requestAnimationFrame(updateLevel);
      }
    };

    updateLevel();
  };

  const testSynthesis = async () => {
    if (!config) return;

    setIsTesting(true);
    try {
      const isProfessional = config.britishAccent.formalityLevel.toLowerCase() === 'professional';
      const testText = isProfessional
        ? "Good afternoon. I trust this message demonstrates the quality of my British executive pronunciation."
        : "Hello! This is a test of my voice synthesis capabilities.";

      const response = await fetch('/api/nora/voice/synthesize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text: testText })
      });

      if (response.ok) {
        const speechResponse = (await response.json()) as { 
          audioData: string;
          durationMs: number;
          sampleRate: number;
          format: string;
          processingTimeMs: number;
        };
        
        // Decode base64 audio data
        const audioData = atob(speechResponse.audioData);
        const audioBuffer = new Uint8Array(audioData.length);
        for (let i = 0; i < audioData.length; i += 1) {
          audioBuffer[i] = audioData.charCodeAt(i);
        }
        
        // Determine MIME type based on format
        const mimeType = speechResponse.format === 'Mp3' ? 'audio/mpeg' : 'audio/wav';
        const audioBlob = new Blob([audioBuffer], { type: mimeType });
        const audioUrl = URL.createObjectURL(audioBlob);
        const audioElement = new Audio(audioUrl);
        await audioElement.play();
      }
    } catch (error) {
      console.error('Synthesis test failed:', error);
    } finally {
      setIsTesting(false);
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

  if (isLoading) {
    return (
      <Card className={className}>
        <CardContent className="flex items-center justify-center py-8">
          <Loader message="Loading voice configuration..." size={32} />
        </CardContent>
      </Card>
    );
  }

  if (!config) {
    return (
      <Card className={className}>
        <CardContent className="flex flex-col items-center justify-center gap-3 py-8 text-center text-gray-500">
          <div>Nora is not initialized yet.</div>
          <div className="text-sm">Open the Nora assistant to initialize her before adjusting voice settings.</div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className={`space-y-6 ${className}`}>
      {/* Voice Testing */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Crown className="w-5 h-5 text-purple-600" />
            Voice Testing
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Recording Test */}
          <div className="space-y-4">
            <div className="flex items-center gap-4">
              <Button
                onClick={isRecording ? stopRecordingTest : startRecordingTest}
                variant={isRecording ? "destructive" : "default"}
                disabled={isTesting}
              >
                {isRecording ? <MicOff className="w-4 h-4 mr-2" /> : <Mic className="w-4 h-4 mr-2" />}
                {isRecording ? 'Stop Recording' : 'Test Microphone'}
              </Button>

              <Button
                onClick={testSynthesis}
                variant="secondary"
                disabled={isTesting}
              >
                <Volume2 className="w-4 h-4 mr-2" />
                Test Voice Synthesis
              </Button>

              {isTesting && <Loader size={20} />}
            </div>

            {/* Audio Level Indicator */}
            {isRecording && (
              <div className="space-y-2">
                <div className="text-sm font-medium">Audio Level</div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-blue-500 h-2 rounded-full transition-all duration-100"
                    style={{ width: `${audioLevel * 100}%` }}
                  />
                </div>
              </div>
            )}

            {/* Transcription Result */}
            {testResult && (
              <div className="border rounded-lg p-4 bg-green-50">
                <div className="text-sm font-medium text-green-800 mb-2">Transcription Result</div>
                <div className="text-sm mb-2">"{testResult.text}"</div>
                <div className="flex gap-4 text-xs text-green-600">
                  <span>Confidence: {(testResult.confidence * 100).toFixed(1)}%</span>
                  <span>Language: {testResult.language}</span>
                  <span>Processing: {testResult.processingTimeMs}ms</span>
                </div>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* TTS Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Volume2 className="w-5 h-5 text-blue-600" />
            Text-to-Speech Settings
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Provider Selection */}
            <div className="space-y-2">
              <label className="text-sm font-medium">TTS Provider</label>
              <Select
                value={config.tts.provider}
                onValueChange={(value) => updateConfig({
                  tts: { ...config.tts, provider: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="elevenLabs">ElevenLabs</SelectItem>
                  <SelectItem value="azure">Azure</SelectItem>
                  <SelectItem value="openAI">OpenAI</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Voice ID */}
            <div className="space-y-2">
              <label className="text-sm font-medium">
                Voice
                {config.tts.provider === "openAI" && (
                  <span className="ml-2 text-xs text-muted-foreground">(British style)</span>
                )}
              </label>
              <Select
                value={config.tts.voiceId}
                onValueChange={(value) => updateConfig({
                  tts: { ...config.tts, voiceId: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {config.tts.provider === "openAI" ? (
                    <>
                      <SelectItem value="fable">Fable (British Female Executive)</SelectItem>
                      <SelectItem value="nova">Nova (Professional Female)</SelectItem>
                      <SelectItem value="echo">Echo (British Male Executive)</SelectItem>
                      <SelectItem value="onyx">Onyx (Professional Male)</SelectItem>
                      <SelectItem value="shimmer">Shimmer (Softer Female)</SelectItem>
                      <SelectItem value="alloy">Alloy (Neutral)</SelectItem>
                    </>
                  ) : config.tts.britishVoicePreferences.map((voice) => (
                    <SelectItem key={voice} value={voice}>
                      {voice}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Quality */}
            <div className="space-y-2">
              <label className="text-sm font-medium">Quality</label>
              <Select
                value={config.tts.quality}
                onValueChange={(value) => updateConfig({
                  tts: { ...config.tts, quality: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="low">Low</SelectItem>
                  <SelectItem value="medium">Medium</SelectItem>
                  <SelectItem value="high">High</SelectItem>
                  <SelectItem value="premium">Premium</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Voice Parameters */}
          <div className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Speed: {config.tts.speed}</label>
              <Slider
                value={[config.tts.speed]}
                onValueChange={([value]) => updateConfig({
                  tts: { ...config.tts, speed: value }
                })}
                min={0.5}
                max={2.0}
                step={0.1}
                className="w-full"
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">Volume: {config.tts.volume}</label>
              <Slider
                value={[config.tts.volume]}
                onValueChange={([value]) => updateConfig({
                  tts: { ...config.tts, volume: value }
                })}
                min={0.0}
                max={1.0}
                step={0.1}
                className="w-full"
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">Pitch: {config.tts.pitch}</label>
              <Slider
                value={[config.tts.pitch]}
                onValueChange={([value]) => updateConfig({
                  tts: { ...config.tts, pitch: value }
                })}
                min={0.5}
                max={2.0}
                step={0.1}
                className="w-full"
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* STT Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mic className="w-5 h-5 text-green-600" />
            Speech-to-Text Settings
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">STT Provider</label>
              <Select
                value={config.stt.provider}
                onValueChange={(value) => updateConfig({
                  stt: { ...config.stt, provider: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="whisper">Whisper</SelectItem>
                  <SelectItem value="azure">Azure</SelectItem>
                  <SelectItem value="google">Google</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">Language</label>
              <Select
                value={config.stt.language}
                onValueChange={(value) => updateConfig({
                  stt: { ...config.stt, language: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="en-GB">English (British)</SelectItem>
                  <SelectItem value="en-US">English (American)</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={config.stt.britishDialectSupport}
                onChange={(e) => updateConfig({
                  stt: { ...config.stt, britishDialectSupport: e.target.checked }
                })}
              />
              <span className="text-sm">British Dialect Support</span>
            </label>

            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={config.stt.executiveVocabulary}
                onChange={(e) => updateConfig({
                  stt: { ...config.stt, executiveVocabulary: e.target.checked }
                })}
              />
              <span className="text-sm">Executive Vocabulary</span>
            </label>
          </div>
        </CardContent>
      </Card>

      {/* British Accent Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Languages className="w-5 h-5 text-purple-600" />
            British Accent Settings
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium">
              Accent Strength: {config.britishAccent.accentStrength}
            </label>
            <Slider
              value={[config.britishAccent.accentStrength]}
              onValueChange={([value]) => updateConfig({
                britishAccent: { ...config.britishAccent, accentStrength: value }
              })}
              min={0.0}
              max={1.0}
              step={0.1}
              className="w-full"
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Regional Variant</label>
              <Select
                value={config.britishAccent.regionalVariant}
                onValueChange={(value) => updateConfig({
                  britishAccent: { ...config.britishAccent, regionalVariant: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ReceivedPronunciation">Received Pronunciation</SelectItem>
                  <SelectItem value="GeneralBritish">General British</SelectItem>
                  <SelectItem value="London">London</SelectItem>
                  <SelectItem value="Scottish">Scottish</SelectItem>
                  <SelectItem value="Welsh">Welsh</SelectItem>
                  <SelectItem value="NorthernEnglish">Northern English</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">Formality Level</label>
              <Select
                value={config.britishAccent.formalityLevel}
                onValueChange={(value) => updateConfig({
                  britishAccent: { ...config.britishAccent, formalityLevel: value }
                })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Casual">Casual</SelectItem>
                  <SelectItem value="Neutral">Neutral</SelectItem>
                  <SelectItem value="Professional">Professional</SelectItem>
                  <SelectItem value="VeryFormal">Very Formal</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Executive Mode */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Crown className="w-5 h-5 text-gold-600" />
            Executive Mode
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {Object.entries(config.executiveMode).map(([key, value]) => (
              <label key={key} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={value as boolean}
                  onChange={(e) => updateConfig({
                    executiveMode: {
                      ...config.executiveMode,
                      [key]: e.target.checked
                    }
                  })}
                />
                <span className="text-sm capitalize">
                  {key.replace(/([A-Z])/g, ' $1').toLowerCase()}
                </span>
              </label>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
