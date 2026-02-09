#!/usr/bin/env python3
"""
Coqui TTS HTTP Server - Local British Female Voice

A fully local TTS server using Coqui TTS with British female voice.
No API keys required - completely sovereign stack.

Installation:
    pip install TTS fastapi uvicorn

Usage:
    python coqui_server.py

Environment Variables:
    COQUI_PORT: Port to run the server on (default: 8102)
    COQUI_MODEL: Model to use (default: tts_models/en/vctk/vits)
    COQUI_SPEAKER: Speaker ID (default: p225 - young British female)

API Endpoints:
    GET  /health    - Health check
    POST /tts       - Text-to-speech synthesis
"""

import io
import os
import sys
from pathlib import Path

try:
    from TTS.api import TTS
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import Response
    from pydantic import BaseModel
    import uvicorn
    import soundfile as sf
    import numpy as np
except ImportError as e:
    print(f"Missing dependency: {e}")
    print("\nInstall required packages with:")
    print("  pip install TTS fastapi uvicorn soundfile")
    sys.exit(1)

app = FastAPI(
    title="Coqui TTS Server",
    description="Fast local TTS with British female voice (VCTK)",
    version="1.0.0",
)

# Global TTS instance
tts = None
model_name = None
speaker_id = None


class TTSRequest(BaseModel):
    """Request body for TTS synthesis"""
    text: str
    speed: float = 1.0  # Speech speed multiplier
    speaker: str = None  # Optional speaker override


class HealthResponse(BaseModel):
    """Health check response"""
    status: str
    model_loaded: bool
    model_name: str
    speaker_id: str
    voice_type: str


@app.on_event("startup")
async def load_model():
    """Load the Coqui TTS model on server startup"""
    global tts, model_name, speaker_id

    # Get configuration from environment
    model_name = os.environ.get("COQUI_MODEL", "tts_models/en/vctk/vits")
    speaker_id = os.environ.get("COQUI_SPEAKER", "p225")  # p225 is a young British female

    print(f"Loading Coqui TTS model: {model_name}...")
    print(f"Speaker: {speaker_id} (young British female)")
    print("This may take a moment on first run (downloading model)...")

    try:
        tts = TTS(model_name, gpu=False)  # Coqui CPU inference is fast enough
        print(f"✓ Model loaded successfully")

        # List available speakers if it's a multi-speaker model
        if hasattr(tts, 'speakers') and tts.speakers:
            print(f"  Available speakers: {len(tts.speakers)}")
            # Show some British female speakers
            british_females = [s for s in tts.speakers if s.startswith('p2')][:5]
            print(f"  British female speakers: {', '.join(british_females)}")
    except Exception as e:
        print(f"ERROR: Failed to load model: {e}")
        sys.exit(1)


@app.get("/health", response_model=HealthResponse)
async def health():
    """Health check endpoint"""
    return HealthResponse(
        status="ok" if tts is not None else "loading",
        model_loaded=tts is not None,
        model_name=model_name if model_name else "unknown",
        speaker_id=speaker_id if speaker_id else "unknown",
        voice_type="British Female (VCTK p225)",
    )


@app.post("/tts")
async def synthesize(request: TTSRequest):
    """
    Synthesize speech from text using Coqui TTS

    Returns audio/wav format
    """
    if tts is None:
        raise HTTPException(status_code=503, detail="TTS model not loaded")

    if not request.text or not request.text.strip():
        raise HTTPException(status_code=400, detail="Text cannot be empty")

    try:
        # Use provided speaker or default
        speaker = request.speaker if request.speaker else speaker_id

        # Synthesize audio
        # VCTK model requires speaker_id parameter
        if hasattr(tts, 'speakers') and tts.speakers:
            wav = tts.tts(text=request.text, speaker=speaker)
        else:
            wav = tts.tts(text=request.text)

        # Apply speed adjustment if needed
        if request.speed != 1.0:
            # Simple time-stretching (not perfect but works)
            import librosa
            wav = librosa.effects.time_stretch(np.array(wav), rate=request.speed)

        # Convert to WAV bytes
        wav_buffer = io.BytesIO()
        sf.write(wav_buffer, wav, tts.synthesizer.output_sample_rate, format='WAV')
        wav_buffer.seek(0)

        return Response(
            content=wav_buffer.read(),
            media_type="audio/wav",
            headers={
                "Content-Disposition": "inline; filename=speech.wav",
                "X-Sample-Rate": str(tts.synthesizer.output_sample_rate),
            }
        )

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Synthesis failed: {str(e)}")


if __name__ == "__main__":
    port = int(os.environ.get("COQUI_PORT", "8102"))

    print("=" * 60)
    print("Coqui TTS Server - British Female Voice (VCTK p225)")
    print("=" * 60)
    print(f"Starting server on http://0.0.0.0:{port}")
    print()
    print("Environment variables:")
    print(f"  COQUI_PORT={port}")
    print(f"  COQUI_MODEL={os.environ.get('COQUI_MODEL', 'tts_models/en/vctk/vits')}")
    print(f"  COQUI_SPEAKER={os.environ.get('COQUI_SPEAKER', 'p225')}")
    print()
    print("To test:")
    print(f"  curl -X POST http://localhost:{port}/tts \\")
    print('    -H "Content-Type: application/json" \\')
    print('    -d \'{"text": "Hello, I am Nora, your British executive assistant."}\' \\')
    print("    --output test.wav")
    print("=" * 60)
    print()

    uvicorn.run(app, host="0.0.0.0", port=port, log_level="info")
