#!/usr/bin/env python3
import sys
from piper.voice import PiperVoice
import wave
import io

test_text = "Hello, I am Nora, your British executive assistant. How may I assist you today?"

voices = [
    ("alba", "/home/pythia/pcg-cc-mcp/piper_voices/en_GB-alba-medium.onnx"),
    ("cori", "/home/pythia/pcg-cc-mcp/piper_voices/en_GB-cori-high.onnx"),
    ("semaine", "/home/pythia/pcg-cc-mcp/piper_voices/en_GB-semaine-medium.onnx"),
]

for name, model_path in voices:
    try:
        print(f"Generating sample for {name}...")
        voice = PiperVoice.load(model_path)
        audio_bytes = voice.synthesize(test_text)

        # Write WAV file
        output_path = f"/home/pythia/pcg-cc-mcp/voice_samples/piper_{name}.wav"
        wav_buffer = io.BytesIO()
        with wave.open(wav_buffer, 'wb') as wav_file:
            wav_file.setnchannels(1)
            wav_file.setsampwidth(2)
            wav_file.setframerate(voice.config.sample_rate)
            wav_file.writeframes(audio_bytes)

        with open(output_path, 'wb') as f:
            f.write(wav_buffer.getvalue())

        print(f"✓ Generated {name} sample")
    except Exception as e:
        print(f"✗ Failed {name}: {e}")
