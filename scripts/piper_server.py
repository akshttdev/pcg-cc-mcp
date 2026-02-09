#!/usr/bin/env python3
"""
Piper TTS HTTP Server for Nora Voice System
Serves the semaine British female voice (en_GB-semaine-medium)
"""
import os
import subprocess
import tempfile
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel
import uvicorn

app = FastAPI(title="Piper TTS Server")

# Configuration
PIPER_MODEL = "/home/pythia/pcg-cc-mcp/piper_voices/en_GB-semaine-medium.onnx"
PIPER_BIN = "/home/pythia/.local/bin/piper"

class TTSRequest(BaseModel):
    text: str
    speaker_id: str = None  # Speaker ID (piper format)
    voice: str = None       # Voice name (backend format)
    speed: float = 1.0      # Speech speed
    exaggeration: float = 0.5  # Voice exaggeration (ignored for Piper)

@app.get("/health")
async def health():
    return {"status": "healthy", "model": "en_GB-semaine-medium", "voice": "British female (semaine)"}

@app.post("/tts")
@app.post("/synthesize")
async def synthesize(request: TTSRequest):
    """Generate speech from text using Piper TTS"""
    try:
        # Map speaker names to IDs (from the semaine model config)
        # Also support voice names from backend ("british_female", etc.)
        speaker_map = {
            "prudence": 0,
            "spike": 1,
            "obadiah": 2,
            "poppy": 3,
            "british_female": 0,  # Map to prudence (female)
            "british_male": 1,     # Map to spike (male)
        }

        # Accept either speaker_id or voice parameter
        voice_param = request.speaker_id or request.voice or "prudence"
        speaker_id = speaker_map.get(voice_param.lower(), 0)

        # Create temporary file for output
        with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as tmp_file:
            output_path = tmp_file.name

        # Run piper to generate speech
        cmd = [
            PIPER_BIN,
            "--model", PIPER_MODEL,
            "--speaker", str(speaker_id),
            "--output_file", output_path
        ]

        # Pass text via stdin
        process = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )

        stdout, stderr = process.communicate(input=request.text.encode('utf-8'))

        if process.returncode != 0:
            raise HTTPException(
                status_code=500,
                detail=f"Piper TTS failed: {stderr.decode()}"
            )

        # Read the generated audio
        with open(output_path, 'rb') as f:
            audio_data = f.read()

        # Clean up
        os.unlink(output_path)

        # Return WAV audio
        return Response(
            content=audio_data,
            media_type="audio/wav",
            headers={
                "Content-Disposition": "attachment; filename=speech.wav"
            }
        )

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/")
async def root():
    return {
        "service": "Piper TTS Server",
        "model": "en_GB-semaine-medium",
        "voice": "British female (semaine)",
        "speakers": ["prudence", "spike", "obadiah", "poppy"],
        "default_speaker": "prudence"
    }

if __name__ == "__main__":
    port = int(os.environ.get("PIPER_PORT", "8102"))
    print(f"Starting Piper TTS server on port {port}")
    print(f"Model: {PIPER_MODEL}")
    print(f"Speakers: prudence (default), spike, obadiah, poppy")
    uvicorn.run(app, host="0.0.0.0", port=port)
