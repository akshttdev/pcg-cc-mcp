#!/usr/bin/env python3
"""
Chatterbox TTS HTTP Server

A simple HTTP wrapper around the Chatterbox TTS library (resemble-ai/chatterbox)
for use with NORA's voice engine.

Installation:
    pip install chatterbox-tts fastapi uvicorn torch torchaudio

Usage:
    python chatterbox_server.py

    Or with custom port:
    CHATTERBOX_PORT=8100 python chatterbox_server.py

Environment Variables:
    CHATTERBOX_PORT: Port to run the server on (default: 8100)
    CHATTERBOX_DEVICE: Device to run model on - "cuda" or "cpu" (default: auto-detect)
    CHATTERBOX_VOICE_REF: Path to reference audio for voice cloning (optional)

API Endpoints:
    GET  /health    - Health check
    POST /tts       - Text-to-speech synthesis
"""

import io
import os
import sys
from pathlib import Path

try:
    import torch
    import torchaudio
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import Response
    from pydantic import BaseModel
except ImportError as e:
    print(f"Missing dependency: {e}")
    print("\nInstall required packages with:")
    print("  pip install chatterbox-tts fastapi uvicorn torch torchaudio")
    sys.exit(1)

app = FastAPI(
    title="Chatterbox TTS Server",
    description="HTTP wrapper for Chatterbox TTS (resemble-ai/chatterbox)",
    version="1.0.0",
)

# Global model instance
model = None
voice_reference = None


class TTSRequest(BaseModel):
    """Request body for TTS synthesis"""
    text: str
    voice: str = "british_female"  # Voice identifier (for future multi-voice support)
    speed: float = 1.0  # Speech speed multiplier
    exaggeration: float = 0.5  # Expression/emotion level (0.0-1.0)


class HealthResponse(BaseModel):
    """Health check response"""
    status: str
    model_loaded: bool
    device: str
    voice_reference_loaded: bool


@app.on_event("startup")
async def load_model():
    """Load the Chatterbox model on server startup"""
    global model, voice_reference

    try:
        from chatterbox.tts import ChatterboxTTS
    except ImportError:
        print("ERROR: chatterbox-tts not installed")
        print("Install with: pip install chatterbox-tts")
        sys.exit(1)

    # Determine device
    device = os.environ.get("CHATTERBOX_DEVICE")
    if device is None:
        device = "cuda" if torch.cuda.is_available() else "cpu"

    print(f"Loading Chatterbox model on {device}...")
    print("This may take a moment on first run (downloading model weights)...")

    try:
        model = ChatterboxTTS.from_pretrained(device=device)
        print(f"✓ Model loaded successfully on {device}")
    except Exception as e:
        print(f"ERROR: Failed to load model: {e}")
        sys.exit(1)

    # Load voice reference if provided
    voice_ref_path = os.environ.get("CHATTERBOX_VOICE_REF")
    if voice_ref_path and Path(voice_ref_path).exists():
        print(f"Loading voice reference from {voice_ref_path}...")
        try:
            voice_reference = voice_ref_path
            print(f"✓ Voice reference loaded")
        except Exception as e:
            print(f"WARNING: Failed to load voice reference: {e}")
            voice_reference = None
    else:
        print("No voice reference provided - using default voice")


@app.get("/health", response_model=HealthResponse)
async def health():
    """Health check endpoint"""
    device = "unknown"
    if model is not None:
        device = str(next(model.parameters()).device) if hasattr(model, 'parameters') else "cpu"

    return HealthResponse(
        status="ok" if model is not None else "loading",
        model_loaded=model is not None,
        device=device,
        voice_reference_loaded=voice_reference is not None,
    )


@app.post("/tts")
async def synthesize(request: TTSRequest):
    """
    Synthesize speech from text.

    Returns WAV audio data.
    """
    if model is None:
        raise HTTPException(
            status_code=503,
            detail="Model not loaded yet. Please wait for startup to complete.",
        )

    if not request.text.strip():
        raise HTTPException(
            status_code=400,
            detail="Text cannot be empty",
        )

    # Limit text length to prevent memory issues
    max_chars = 5000
    if len(request.text) > max_chars:
        raise HTTPException(
            status_code=400,
            detail=f"Text too long. Maximum {max_chars} characters allowed.",
        )

    try:
        print(f"Synthesizing: '{request.text[:50]}...' (voice={request.voice}, exaggeration={request.exaggeration})")

        # Generate audio
        # Chatterbox supports voice cloning with a reference audio
        generate_kwargs = {
            "text": request.text,
            "exaggeration": request.exaggeration,
        }

        # Add voice reference if available
        if voice_reference is not None:
            generate_kwargs["audio_prompt_path"] = voice_reference

        wav = model.generate(**generate_kwargs)

        # Convert to WAV bytes
        buffer = io.BytesIO()
        torchaudio.save(buffer, wav, model.sr, format="wav")
        buffer.seek(0)
        audio_bytes = buffer.read()

        print(f"✓ Generated {len(audio_bytes)} bytes of audio")

        return Response(
            content=audio_bytes,
            media_type="audio/wav",
            headers={
                "Content-Disposition": "inline; filename=speech.wav",
            },
        )

    except torch.cuda.OutOfMemoryError:
        raise HTTPException(
            status_code=503,
            detail="GPU out of memory. Try shorter text or restart server.",
        )
    except Exception as e:
        print(f"ERROR: TTS generation failed: {e}")
        raise HTTPException(
            status_code=500,
            detail=f"TTS generation failed: {str(e)}",
        )


@app.get("/")
async def root():
    """Root endpoint with API info"""
    return {
        "name": "Chatterbox TTS Server",
        "version": "1.0.0",
        "endpoints": {
            "GET /health": "Health check",
            "POST /tts": "Text-to-speech synthesis",
        },
        "example": {
            "endpoint": "POST /tts",
            "body": {
                "text": "Hello, this is a test.",
                "voice": "british_female",
                "exaggeration": 0.5,
            },
        },
    }


if __name__ == "__main__":
    import uvicorn

    port = int(os.environ.get("CHATTERBOX_PORT", "8100"))
    host = os.environ.get("CHATTERBOX_HOST", "0.0.0.0")

    print("=" * 60)
    print("Chatterbox TTS Server")
    print("=" * 60)
    print(f"Starting server on http://{host}:{port}")
    print()
    print("Environment variables:")
    print(f"  CHATTERBOX_PORT={port}")
    print(f"  CHATTERBOX_DEVICE={os.environ.get('CHATTERBOX_DEVICE', 'auto')}")
    print(f"  CHATTERBOX_VOICE_REF={os.environ.get('CHATTERBOX_VOICE_REF', 'not set')}")
    print()
    print("To test:")
    print(f"  curl -X POST http://localhost:{port}/tts \\")
    print('    -H "Content-Type: application/json" \\')
    print('    -d \'{"text": "Hello, this is a test."}\' \\')
    print("    --output test.wav")
    print("=" * 60)
    print()

    uvicorn.run(app, host=host, port=port)
