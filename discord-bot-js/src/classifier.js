/**
 * classifier.js — Context-aware "should Nora speak?" classifier
 *
 * Uses a local Ollama instance (qwen2.5:7b) to decide whether an utterance is
 * addressed to the AI assistant. All inference stays on-machine — no data is
 * sent to any external service.
 *
 * Exports:
 *   createContextBuffer(maxTurns = 8) → { push, getHistory, clear }
 *   shouldNoraSpeak(utterance, speaker, contextHistory) → Promise<ClassifierResult>
 *
 * ClassifierResult: { decision: bool, confidence: 'high'|'medium'|'low', reasoning: string }
 *
 * Fail-safe: any error (Ollama unreachable, parse failure, timeout >2 s) returns
 *   { decision: false, confidence: 'low', reasoning: 'classifier unavailable' }
 * so that Nora never speaks unexpectedly when the classifier is down.
 */

const OLLAMA_BASE_URL = process.env.OLLAMA_BASE_URL ?? 'http://localhost:11434';
const OLLAMA_MODEL    = 'qwen2.5:7b';
const TIMEOUT_MS      = 2000;

// ─── Context buffer ──────────────────────────────────────────────────────────

/**
 * Creates a rolling conversation buffer that retains the last `maxTurns` turns.
 *
 * @param {number} maxTurns  Maximum number of speaker turns to keep (default 8).
 * @returns {{ push(speaker: string, text: string): void, getHistory(): Array<{speaker: string, text: string}>, clear(): void }}
 */
export function createContextBuffer(maxTurns = 8) {
  /** @type {Array<{speaker: string, text: string}>} */
  const history = [];

  return {
    push(speaker, text) {
      history.push({ speaker, text });
      if (history.length > maxTurns) {
        history.splice(0, history.length - maxTurns);
      }
    },
    getHistory() {
      return [...history];
    },
    clear() {
      history.length = 0;
    },
  };
}

// ─── Classifier ───────────────────────────────────────────────────────────────

/**
 * @typedef {{ decision: boolean, confidence: 'high'|'medium'|'low', reasoning: string }} ClassifierResult
 */

/** Fail-safe result returned whenever the classifier is unavailable. */
const UNAVAILABLE = { decision: false, confidence: 'low', reasoning: 'classifier unavailable' };

/**
 * Asks the local Ollama model whether the latest utterance is addressed to Nora.
 *
 * @param {string} utterance       The new speaker turn to classify.
 * @param {string} speaker         Display name of the speaker.
 * @param {Array<{speaker: string, text: string}>} contextHistory  Previous turns.
 * @returns {Promise<ClassifierResult>}
 */
export async function shouldNoraSpeak(utterance, speaker, contextHistory) {
  try {
    const contextLines = contextHistory
      .map((t) => `${t.speaker}: ${t.text}`)
      .join('\n');

    const prompt = [
      'You are monitoring a meeting. Given the conversation history and the latest utterance,',
      'decide if the AI assistant named Nora is being addressed or if the question should be answered by Nora.',
      'Reply with a JSON object only — no markdown, no explanation outside the JSON:',
      '{"speak": true/false, "confidence": "high"|"medium"|"low", "reason": "one sentence"}',
      '',
      contextLines ? `Conversation so far:\n${contextLines}\n` : '',
      `Latest utterance from ${speaker}: ${utterance}`,
    ].join('\n');

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);

    let raw;
    try {
      const res = await fetch(`${OLLAMA_BASE_URL}/api/generate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: OLLAMA_MODEL,
          prompt,
          stream: false,
          options: { temperature: 0.0 },
        }),
        signal: controller.signal,
      });
      if (!res.ok) return UNAVAILABLE;
      raw = await res.json();
    } finally {
      clearTimeout(timer);
    }

    const responseText = (raw?.response ?? '').trim();

    // Extract JSON — the model may wrap it in backticks or add surrounding text
    const jsonMatch = responseText.match(/\{[\s\S]*\}/);
    if (!jsonMatch) return UNAVAILABLE;

    const parsed = JSON.parse(jsonMatch[0]);

    const decision    = Boolean(parsed.speak);
    const confidence  = ['high', 'medium', 'low'].includes(parsed.confidence)
      ? parsed.confidence
      : 'low';
    const reasoning   = typeof parsed.reason === 'string' ? parsed.reason : '';

    return { decision, confidence, reasoning };
  } catch {
    // AbortError, network error, JSON parse error — all map to fail-safe
    return UNAVAILABLE;
  }
}
