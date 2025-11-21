#!/bin/bash
# Voice Configuration Diagnostic Script
# Checks if your voice/TTS/STT setup is properly configured

set -e

echo "🎙️  NORA Voice Configuration Diagnostic"
echo "========================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check functions
check_pass() {
    echo -e "${GREEN}✅ $1${NC}"
}

check_fail() {
    echo -e "${RED}❌ $1${NC}"
}

check_warn() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

check_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check if running in Docker
if [ -f "/.dockerenv" ]; then
    check_info "Running inside Docker container"
    ENV_SOURCE="container environment"
else
    check_info "Running on host system"
    ENV_SOURCE=".env file or system environment"
fi

echo ""
echo "📋 Checking API Keys and Credentials"
echo "------------------------------------"

# Check OpenAI API Key
if [ -n "$OPENAI_API_KEY" ]; then
    check_pass "OpenAI API key is set"
    echo "     Key: ${OPENAI_API_KEY:0:10}...${OPENAI_API_KEY: -4}"
    echo "     ✅ Enables: LLM (GPT-4), OpenAI TTS, OpenAI Whisper STT"
else
    check_fail "OpenAI API key NOT set"
    echo "     ❌ Without this key:"
    echo "        - NORA's LLM responses won't work"
    echo "        - OpenAI TTS won't work"
    echo "        - OpenAI Whisper STT won't work (you'll get dummy transcriptions)"
    echo "     💡 Fix: Set OPENAI_API_KEY in $ENV_SOURCE"
fi

# Check ElevenLabs API Key
if [ -n "$ELEVENLABS_API_KEY" ]; then
    check_pass "ElevenLabs API key is set"
    echo "     Key: ${ELEVENLABS_API_KEY:0:10}..."
    echo "     ✅ Enables: Premium British TTS voices"
else
    check_warn "ElevenLabs API key not set (optional)"
    echo "     ℹ️  ElevenLabs provides premium British voices"
    echo "     💡 Get key from: https://elevenlabs.io/"
fi

# Check Azure Speech credentials
if [ -n "$AZURE_SPEECH_KEY" ]; then
    check_pass "Azure Speech key is set"
    echo "     Key: ${AZURE_SPEECH_KEY:0:10}..."
    echo "     Region: ${AZURE_SPEECH_REGION:-not set}"
    echo "     ✅ Enables: Azure TTS and STT"
    
    if [ -z "$AZURE_SPEECH_REGION" ]; then
        check_warn "Azure region not set, defaulting to 'eastus'"
    fi
else
    check_warn "Azure Speech credentials not set (optional)"
    echo "     ℹ️  Azure provides enterprise-grade TTS/STT"
    echo "     💡 Get from: Azure Portal > Cognitive Services > Speech"
fi

echo ""
echo "🔍 Voice Provider Status"
echo "------------------------"

# Determine which providers are available
TTS_AVAILABLE=()
STT_AVAILABLE=()

if [ -n "$ELEVENLABS_API_KEY" ]; then
    TTS_AVAILABLE+=("ElevenLabs (Premium)")
fi

if [ -n "$OPENAI_API_KEY" ]; then
    TTS_AVAILABLE+=("OpenAI")
    STT_AVAILABLE+=("OpenAI Whisper")
fi

if [ -n "$AZURE_SPEECH_KEY" ]; then
    TTS_AVAILABLE+=("Azure")
    STT_AVAILABLE+=("Azure")
fi

# Always available
TTS_AVAILABLE+=("System (Fallback)")
STT_AVAILABLE+=("System (Dummy)")

echo "Available TTS Providers:"
for provider in "${TTS_AVAILABLE[@]}"; do
    if [[ "$provider" == *"Fallback"* ]] || [[ "$provider" == *"Dummy"* ]]; then
        echo "  ⚠️  $provider"
    else
        echo "  ✅ $provider"
    fi
done

echo ""
echo "Available STT Providers:"
for provider in "${STT_AVAILABLE[@]}"; do
    if [[ "$provider" == *"Dummy"* ]]; then
        echo "  ⚠️  $provider (returns dummy transcriptions)"
    else
        echo "  ✅ $provider"
    fi
done

echo ""
echo "🎯 Configuration Recommendations"
echo "--------------------------------"

if [ -z "$OPENAI_API_KEY" ]; then
    echo "🔴 CRITICAL: Set OPENAI_API_KEY for basic functionality"
    echo "   - NORA's LLM won't work without this"
    echo "   - STT will return dummy transcriptions"
    echo "   - TTS will use low-quality system voice"
fi

if [ -n "$OPENAI_API_KEY" ] && [ -z "$ELEVENLABS_API_KEY" ]; then
    echo "🟡 RECOMMENDED: Add ElevenLabs for premium British voices"
    echo "   - Current: OpenAI TTS (good quality)"
    echo "   - Upgrade: ElevenLabs (premium British executive voices)"
fi

if [ -n "$OPENAI_API_KEY" ] && [ -n "$ELEVENLABS_API_KEY" ]; then
    check_pass "Optimal setup! You have access to premium features"
fi

echo ""
echo "📝 Current Voice Configuration"
echo "------------------------------"
echo "Default TTS Provider: $([ -n "$ELEVENLABS_API_KEY" ] && echo "ElevenLabs" || ([ -n "$OPENAI_API_KEY" ] && echo "OpenAI" || echo "System"))"
echo "Default STT Provider: $([ -n "$OPENAI_API_KEY" ] && echo "OpenAI Whisper" || echo "System (Dummy)")"

echo ""
echo "🧪 Quick Test Commands"
echo "----------------------"

if [ -n "$OPENAI_API_KEY" ]; then
    echo "Test STT (should work with OpenAI):"
    echo "  curl -X POST http://localhost:3001/nora/voice/transcribe \\"
    echo "    -H 'Content-Type: application/json' \\"
    echo "    -d '{\"audio_data\": \"<base64_audio>\"}'"
    echo ""
    echo "Test TTS (should work with OpenAI):"
    echo "  curl -X POST http://localhost:3001/nora/voice/synthesize \\"
    echo "    -H 'Content-Type: application/json' \\"
    echo "    -d '{\"text\": \"Hello from Nora\"}'"
else
    check_warn "Cannot test - OPENAI_API_KEY not set"
fi

echo ""
echo "📚 Documentation"
echo "----------------"
echo "Voice Status Report: NORA_VOICE_STATUS_REPORT.md"
echo "Quick Start Guide: NORA_DOCKER_QUICKSTART.md"
echo "Usage Guide: NORA_USAGE_GUIDE.md"

echo ""
echo "✅ Diagnostic Complete"
echo ""

# Exit with error if critical issues found
if [ -z "$OPENAI_API_KEY" ]; then
    echo "❌ Critical issues found - NORA won't work properly"
    exit 1
else
    echo "✅ Basic configuration OK"
    exit 0
fi
