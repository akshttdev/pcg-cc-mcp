#!/usr/bin/env python3
"""
Local Whisper STT HTTP Server

A simple HTTP wrapper around OpenAI's Whisper for local speech-to-text.
This runs entirely locally - no API keys or cloud services needed.

Installation:
    pip install openai-whisper fastapi uvicorn python-multipart torch

Usage:
    python whisper_server.py

    Or with custom port:
    WHISPER_PORT=8101 python whisper_server.py

Environment Variables:
    WHISPER_PORT: Port to run the server on (default: 8101)
    WHISPER_MODEL: Model size - tiny, base, small, medium, large (default: base)
    WHISPER_DEVICE: Device to run model on - "cuda" or "cpu" (default: auto-detect)

API Endpoints:
    GET  /health       - Health check
    POST /transcribe   - Speech-to-text transcription
"""

import io
import os
import sys
import base64
import tempfile
from pathlib import Path

try:
    import torch
    import whisper
    from fastapi import FastAPI, HTTPException, UploadFile, File
    from fastapi.responses import JSONResponse
    from pydantic import BaseModel
    import uvicorn
except ImportError as e:
    print(f"Missing dependency: {e}")
    print("\nInstall required packages with:")
    print("  pip install openai-whisper fastapi uvicorn python-multipart torch")
    sys.exit(1)

app = FastAPI(
    title="Local Whisper STT Server",
    description="HTTP wrapper for OpenAI Whisper (local speech-to-text)",
    version="1.0.0",
)

# Global model instance
model = None
model_name = None


class TranscribeRequest(BaseModel):
    """Request body for transcription with base64 audio"""
    audio_b64: str
    language: str = "en"  # Language code (en, es, fr, etc.) or "auto" for detection


class TranscribeResponse(BaseModel):
    """Response from transcription"""
    text: str
    language: str
    confidence: float
    duration_seconds: float
    processing_time_ms: int


@app.on_event("startup")
async def load_model():
    """Load Whisper model on startup"""
    global model, model_name

    model_name = os.environ.get("WHISPER_MODEL", "base")
    device = os.environ.get("WHISPER_DEVICE", "cuda" if torch.cuda.is_available() else "cpu")

    print(f"Loading Whisper model '{model_name}' on {device}...")
    print("This may take a moment on first run (downloading model)...")

    try:
        model = whisper.load_model(model_name, device=device)
        print(f"Whisper model loaded successfully!")
        print(f"  Model: {model_name}")
        print(f"  Device: {device}")
        if device == "cuda":
            print(f"  GPU: {torch.cuda.get_device_name(0)}")
    except Exception as e:
        print(f"Failed to load Whisper model: {e}")
        print("The server will start but transcription will fail.")


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy" if model is not None else "degraded",
        "model": model_name,
        "model_loaded": model is not None,
        "device": "cuda" if torch.cuda.is_available() and model is not None else "cpu",
    }


@app.post("/transcribe", response_model=TranscribeResponse)
async def transcribe_audio(request: TranscribeRequest):
    """Transcribe audio from base64 encoded data"""
    import time

    if model is None:
        raise HTTPException(status_code=503, detail="Whisper model not loaded")

    start_time = time.time()

    try:
        # Decode base64 audio
        audio_data = base64.b64decode(request.audio_b64)

        # Write to temporary file (Whisper needs a file path)
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp_file:
            tmp_file.write(audio_data)
            tmp_path = tmp_file.name

        try:
            # Transcribe
            language = None if request.language == "auto" else request.language
            result = model.transcribe(
                tmp_path,
                language=language,
                fp16=torch.cuda.is_available(),
            )

            # Get audio duration
            import whisper.audio
            audio = whisper.audio.load_audio(tmp_path)
            duration = len(audio) / whisper.audio.SAMPLE_RATE

        finally:
            # Clean up temp file
            os.unlink(tmp_path)

        processing_time = int((time.time() - start_time) * 1000)

        # Calculate average confidence from segments
        confidence = 1.0
        if result.get("segments"):
            avg_no_speech = sum(s.get("no_speech_prob", 0) for s in result["segments"]) / len(result["segments"])
            confidence = 1.0 - avg_no_speech

        return TranscribeResponse(
            text=result["text"].strip(),
            language=result.get("language", request.language),
            confidence=confidence,
            duration_seconds=duration,
            processing_time_ms=processing_time,
        )

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Transcription failed: {str(e)}")


@app.post("/transcribe/file")
async def transcribe_file(file: UploadFile = File(...), language: str = "en"):
    """Transcribe audio from uploaded file"""
    import time

    if model is None:
        raise HTTPException(status_code=503, detail="Whisper model not loaded")

    start_time = time.time()

    try:
        # Save uploaded file
        content = await file.read()

        with tempfile.NamedTemporaryFile(suffix=Path(file.filename).suffix or ".wav", delete=False) as tmp_file:
            tmp_file.write(content)
            tmp_path = tmp_file.name

        try:
            # Transcribe
            lang = None if language == "auto" else language
            result = model.transcribe(
                tmp_path,
                language=lang,
                fp16=torch.cuda.is_available(),
            )

            # Get audio duration
            import whisper.audio
            audio = whisper.audio.load_audio(tmp_path)
            duration = len(audio) / whisper.audio.SAMPLE_RATE

        finally:
            os.unlink(tmp_path)

        processing_time = int((time.time() - start_time) * 1000)

        confidence = 1.0
        if result.get("segments"):
            avg_no_speech = sum(s.get("no_speech_prob", 0) for s in result["segments"]) / len(result["segments"])
            confidence = 1.0 - avg_no_speech

        return {
            "text": result["text"].strip(),
            "language": result.get("language", language),
            "confidence": confidence,
            "duration_seconds": duration,
            "processing_time_ms": processing_time,
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Transcription failed: {str(e)}")


if __name__ == "__main__":
    port = int(os.environ.get("WHISPER_PORT", "8101"))
    print(f"\nStarting Local Whisper STT Server on port {port}")
    print("=" * 50)
    uvicorn.run(app, host="0.0.0.0", port=port)
