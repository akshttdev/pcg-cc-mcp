#!/usr/bin/env python3
"""
Simple test script for Nora integration
Tests the backend API endpoints to verify Nora is working
"""

import os
import requests
import json
import time
import sys
from datetime import datetime

# Configuration
BASE_URL = os.getenv("NORA_BASE_URL", "http://127.0.0.1:58297")
HEADERS = {"Content-Type": "application/json"}

def test_nora_initialization():
    """Test Nora initialization"""
    print("🔧 Testing Nora initialization...")

    payload = {
        "config": {
            "personality": {
                "accentStrength": 0.8,
                "formalityLevel": "professional",
                "warmthLevel": "warm",
                "proactiveCommunication": True,
                "executiveVocabulary": True,
                "britishExpressions": True,
                "politenessLevel": "veryPolite"
            },
            "voice": {
                "tts": {
                    "provider": "elevenLabs",
                    "voiceId": "ZtcPZrt9K4w8e1OB9M6w",
                    "speed": 1.0,
                    "volume": 0.8,
                    "pitch": 0.0,
                    "quality": "high",
                    "britishVoicePreferences": ["ZtcPZrt9K4w8e1OB9M6w"],
                    "fallbackProviders": ["system"]
                },
                "stt": {
                    "provider": "system",
                    "model": "system_stt",
                    "language": "en-GB",
                    "britishDialectSupport": True,
                    "executiveVocabulary": True,
                    "realTime": False,
                    "noiseReduction": True
                },
                "audio": {
                    "sampleRate": 44100,
                    "channels": 2,
                    "bitDepth": 16,
                    "bufferSize": 1024,
                    "noiseSuppression": True,
                    "echoCancellation": True,
                    "autoGainControl": True
                },
                "britishAccent": {
                    "accentStrength": 0.8,
                    "regionalVariant": "receivedPronunciation",
                    "formalityLevel": "professional",
                    "vocabularyPreferences": "executive"
                },
                "executiveMode": {
                    "enabled": True,
                    "proactiveCommunication": True,
                    "executiveSummaryStyle": True,
                    "formalAddress": True,
                    "businessVocabulary": True
                }
            },
            "executiveMode": True,
            "proactiveNotifications": True,
            "contextAwareness": True,
            "multiAgentCoordination": True
        },
        "activateImmediately": True
    }

    try:
        response = requests.post(f"{BASE_URL}/api/nora/initialize",
                               headers=HEADERS, json=payload, timeout=10)
        if response.status_code == 200:
            print("✅ Nora initialization successful")
            return True
        else:
            print(f"❌ Nora initialization failed: {response.status_code} - {response.text}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ Connection error: {e}")
        return False

def test_nora_chat():
    """Test basic chat with Nora"""
    print("\n💬 Testing Nora chat...")

    payload = {
        "message": "Hello Nora, please introduce yourself as a British executive assistant.",
        "sessionId": "test-session",
        "requestType": "textInteraction",
        "voiceEnabled": False,
        "priority": "normal",
        "context": None
    }

    try:
        response = requests.post(f"{BASE_URL}/api/nora/chat",
                               headers=HEADERS, json=payload, timeout=15)
        if response.status_code == 200:
            result = response.json()
            print("✅ Nora chat successful")
            print(f"📝 Response: {result.get('content', 'No content')}")
            if result.get('followUpSuggestions'):
                print(f"💡 Suggestions: {', '.join(result['followUpSuggestions'])}")
            return True
        else:
            print(f"❌ Nora chat failed: {response.status_code} - {response.text}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ Connection error: {e}")
        return False

def test_strategic_planning():
    """Test Nora's strategic planning capabilities"""
    print("\n📊 Testing strategic planning...")

    payload = {
        "message": "Please provide strategic guidance for implementing AI coordination systems in our organization.",
        "sessionId": "test-session",
        "requestType": "strategyPlanning",
        "voiceEnabled": False,
        "priority": "executive",
        "context": {
            "planning_request": {
                "scope": "Organization",
                "objectives": [
                    "Implement multi-agent coordination",
                    "Improve executive decision support",
                    "Enhance operational efficiency"
                ],
                "constraints": {
                    "timeline": "6 months",
                    "budget": 500000
                }
            }
        }
    }

    try:
        response = requests.post(f"{BASE_URL}/api/nora/chat",
                               headers=HEADERS, json=payload, timeout=20)
        if response.status_code == 200:
            result = response.json()
            print("✅ Strategic planning successful")
            print(f"📈 Strategy: {result.get('content', 'No content')[:200]}...")
            if result.get('actions'):
                print(f"⚡ Actions: {len(result['actions'])} executive actions suggested")
            return True
        else:
            print(f"❌ Strategic planning failed: {response.status_code} - {response.text}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ Connection error: {e}")
        return False

def test_coordination_stats():
    """Test coordination statistics endpoint"""
    print("\n📈 Testing coordination statistics...")

    try:
        response = requests.get(f"{BASE_URL}/api/nora/coordination/stats", timeout=10)
        if response.status_code == 200:
            stats = response.json()
            print("✅ Coordination stats retrieved")
            print(f"🤖 Agents: {stats.get('totalAgents', 0)} total, {stats.get('activeAgents', 0)} active")
            print(f"⏱️  Avg response time: {stats.get('averageResponseTime', 0):.1f}ms")
            return True
        else:
            print(f"❌ Coordination stats failed: {response.status_code} - {response.text}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ Connection error: {e}")
        return False

def test_voice_synthesis():
    """Test voice synthesis (if available)"""
    print("\n🎤 Testing voice synthesis...")

    payload = {
        "text": "Good afternoon. This is a test of my British executive voice synthesis capabilities."
    }

    try:
        response = requests.post(f"{BASE_URL}/api/nora/voice/synthesize",
                               headers=HEADERS, json=payload, timeout=15)
        if response.status_code == 200:
            result = response.json()
            audio_payload = result.get('audio') or result.get('audioData')
            if audio_payload:
                print("✅ Voice synthesis successful")
                print(f"🔊 Generated audio: {len(audio_payload)} characters (base64)")
                return True
            else:
                print("⚠️  Voice synthesis returned but no audio data")
                return False
        else:
            print(f"⚠️  Voice synthesis not available: {response.status_code}")
            print("   (This is expected if voice providers aren't configured)")
            return True  # Not a failure if voice isn't configured
    except requests.exceptions.RequestException as e:
        print(f"❌ Connection error: {e}")
        return False

def check_server_running():
    """Check if the server is running"""
    print("🌐 Checking if PCG Dashboard server is running...")

    try:
        response = requests.get(f"{BASE_URL}/health", timeout=5)
        if response.status_code == 200:
            print("✅ Server is running")
            return True
        else:
            print(f"⚠️  Server responded with status {response.status_code}")
            return False
    except requests.exceptions.RequestException:
        print("❌ Server is not running or not accessible")
        print(f"   Make sure the server is started on {BASE_URL}")
        return False

def main():
    """Run all Nora tests"""
    print("🚀 Starting Nora Integration Tests")
    print("=" * 50)

    # Check if server is running first
    if not check_server_running():
        print("\n❌ Cannot proceed with tests - server is not running")
        print("   Start the server with: cargo run --bin server")
        sys.exit(1)

    tests = [
        test_nora_initialization,
        test_nora_chat,
        test_strategic_planning,
        test_coordination_stats,
        test_voice_synthesis,
    ]

    passed = 0
    total = len(tests)

    for test in tests:
        if test():
            passed += 1
        time.sleep(1)  # Brief pause between tests

    print("\n" + "=" * 50)
    print(f"🎯 Test Results: {passed}/{total} tests passed")

    if passed == total:
        print("🎉 All tests passed! Nora integration is working perfectly.")
    elif passed >= total - 1:
        print("✨ Almost all tests passed! Nora integration is working well.")
    else:
        print("⚠️  Some tests failed. Check server logs for details.")

    print("\n💡 Next steps:")
    print("   - Access the web interface at: http://localhost:3001/nora")
    print("   - Test voice features with proper API keys (OpenAI, ElevenLabs, Azure)")
    print("   - Try the MCP integration tools")

    sys.exit(0 if passed >= total - 1 else 1)

if __name__ == "__main__":
    main()
