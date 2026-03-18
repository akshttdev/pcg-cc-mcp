#!/bin/bash
# Test script for NORA Twilio phone integration
# Simulates a phone call without actually calling

BASE_URL="${1:-http://localhost:3002}"
CALL_SID="TEST-$(date +%s)"

echo "🔵 Testing NORA Phone Integration"
echo "   Base URL: $BASE_URL"
echo "   Call SID: $CALL_SID"
echo ""

# Step 1: Check health
echo "1️⃣  Checking Twilio health..."
HEALTH=$(curl -s "$BASE_URL/api/twilio/health")
echo "   Response: $HEALTH"
echo ""

# Step 2: Simulate incoming call
echo "2️⃣  Simulating incoming call..."
GREETING=$(curl -s -X POST "$BASE_URL/api/twilio/voice" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "CallSid=$CALL_SID&AccountSid=AC123&From=+919876543210&To=+14053008311&CallStatus=ringing")
echo "   NORA says:"
echo "$GREETING" | grep -oP '(?<=<Say[^>]*>)[^<]+' | head -1 | sed 's/^/   📢 /'
echo ""

# Step 3: Simulate user speaking
echo "3️⃣  User says: 'Hello Nora, what can you help me with?'"
RESPONSE1=$(curl -s -X POST "$BASE_URL/api/twilio/speech?call_sid=$CALL_SID" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "CallSid=$CALL_SID&SpeechResult=Hello%20Nora%20what%20can%20you%20help%20me%20with&Confidence=0.95")
echo "   NORA responds:"
echo "$RESPONSE1" | grep -oP '(?<=<Say[^>]*>)[^<]+' | head -1 | sed 's/^/   📢 /'
echo ""

# Step 4: Another user message
echo "4️⃣  User says: 'Tell me about the current projects'"
RESPONSE2=$(curl -s -X POST "$BASE_URL/api/twilio/speech?call_sid=$CALL_SID" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "CallSid=$CALL_SID&SpeechResult=Tell%20me%20about%20the%20current%20projects&Confidence=0.92")
echo "   NORA responds:"
echo "$RESPONSE2" | grep -oP '(?<=<Say[^>]*>)[^<]+' | head -1 | sed 's/^/   📢 /'
echo ""

# Step 5: End call
echo "5️⃣  User says: 'Thank you, goodbye'"
GOODBYE=$(curl -s -X POST "$BASE_URL/api/twilio/speech?call_sid=$CALL_SID" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "CallSid=$CALL_SID&SpeechResult=Thank%20you%20goodbye&Confidence=0.98")
echo "   NORA responds:"
echo "$GOODBYE" | grep -oP '(?<=<Say[^>]*>)[^<]+' | head -1 | sed 's/^/   📢 /'

# Check for hangup
if echo "$GOODBYE" | grep -q "<Hangup/>"; then
  echo "   📞 Call ended"
fi
echo ""

echo "✅ Test complete!"
