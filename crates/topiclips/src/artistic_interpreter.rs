//! Artistic Interpreter - LLM-powered prompt generation
//!
//! Translates data stories into surreal visual prompts using an LLM
//! to generate creative, artistic interpretations of topology events.

use anyhow::{anyhow, Result};
use db::models::topiclip::TopiClipCapturedEvent;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::story_extractor::NarrativeStory;

/// Result of artistic interpretation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtisticInterpretation {
    /// The main prompt for video generation
    pub artistic_prompt: String,
    /// Negative prompt (what to avoid)
    pub negative_prompt: String,
    /// Mapping of events to their symbols used
    pub symbol_mapping: Value,
    /// LLM reasoning about the interpretation
    pub reasoning: String,
}

/// Artistic interpreter using LLM for creative prompt generation
pub struct ArtisticInterpreter {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    model: String,
    temperature: f64,
}

impl ArtisticInterpreter {
    pub fn new(
        endpoint: String,
        api_key: Option<String>,
        model: String,
        temperature: f64,
    ) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            api_key,
            model,
            temperature,
        }
    }

    /// Interpret a narrative story and captured events into artistic prompts
    pub async fn interpret(
        &self,
        story: &NarrativeStory,
        events: &[TopiClipCapturedEvent],
    ) -> Result<ArtisticInterpretation> {
        // If no API key, use fallback generation
        if self.api_key.is_none() {
            return Ok(self.fallback_interpret(story, events));
        }

        // Build the LLM prompt
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(story, events);

        // Call the LLM
        let response = self.call_llm(&system_prompt, &user_prompt).await?;

        // Parse the response
        self.parse_llm_response(&response, story, events)
    }

    fn build_system_prompt(&self) -> String {
        r#"You are an artistic interpreter for TopiClips, transforming data topology changes into surreal, symbolic video prompts in the style of Beeple's Everydays.

Your role is to create visual prompts that:
1. Use symbolism and metaphor rather than literal data visualization
2. Create emotionally resonant imagery
3. Maintain visual coherence suitable for AI video generation
4. Incorporate the assigned symbols for each event

Output format (JSON):
{
  "artistic_prompt": "A single cohesive visual description for video generation (2-3 sentences)",
  "negative_prompt": "Elements to avoid (lowres, text, distorted, etc.)",
  "reasoning": "Brief explanation of your creative choices"
}

Style guidelines:
- Surreal, dreamlike imagery
- Geometric and organic forms in tension/harmony
- Light as a key narrative element
- Abstract representations of connection, growth, loss
- Cinematic composition suitable for 4-second clips"#.to_string()
    }

    fn build_user_prompt(&self, story: &NarrativeStory, events: &[TopiClipCapturedEvent]) -> String {
        let mut prompt = format!(
            r#"Create an artistic prompt for the following topology story:

**Theme:** {}
**Emotional Arc:** {}
**Narrative:** {}

**Events with Assigned Symbols:**
"#,
            story.primary_theme, story.emotional_arc, story.narrative_summary
        );

        for event in events.iter().take(5) {
            // Limit to top 5 events
            let symbol = event
                .assigned_symbol
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let symbol_prompt = event.symbol_prompt.as_ref().map(|s| s.as_str()).unwrap_or("");

            prompt.push_str(&format!(
                "- {} (Symbol: {}) - Base visual: {}\n",
                event.event_type, symbol, symbol_prompt
            ));
        }

        prompt.push_str(
            r#"
Synthesize these symbols into a single cohesive 4-second video scene that captures the essence of the story. The prompt should work with Stable Video Diffusion for smooth, artistic animation."#,
        );

        prompt
    }

    async fn call_llm(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("API key not configured"))?;

        let payload = json!({
            "model": self.model,
            "max_tokens": 1024,
            "temperature": self.temperature,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        });

        let response = self
            .client
            .post(&self.endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("LLM API error {}: {}", status, text));
        }

        let response_json: Value = response.json().await?;

        // Extract content from Claude's response format
        let content = response_json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .ok_or_else(|| anyhow!("Invalid LLM response format"))?;

        Ok(content.to_string())
    }

    fn parse_llm_response(
        &self,
        response: &str,
        story: &NarrativeStory,
        events: &[TopiClipCapturedEvent],
    ) -> Result<ArtisticInterpretation> {
        // Try to parse as JSON
        if let Ok(parsed) = serde_json::from_str::<Value>(response) {
            let artistic_prompt = parsed["artistic_prompt"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let negative_prompt = parsed["negative_prompt"]
                .as_str()
                .unwrap_or("lowres, blurry, text, watermark, distorted")
                .to_string();
            let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

            if !artistic_prompt.is_empty() {
                return Ok(ArtisticInterpretation {
                    artistic_prompt,
                    negative_prompt,
                    symbol_mapping: self.build_symbol_mapping(events),
                    reasoning,
                });
            }
        }

        // Try to extract JSON from response if it contains other text
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                let json_str = &response[start..=end];
                if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                    let artistic_prompt = parsed["artistic_prompt"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let negative_prompt = parsed["negative_prompt"]
                        .as_str()
                        .unwrap_or("lowres, blurry, text, watermark, distorted")
                        .to_string();
                    let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

                    if !artistic_prompt.is_empty() {
                        return Ok(ArtisticInterpretation {
                            artistic_prompt,
                            negative_prompt,
                            symbol_mapping: self.build_symbol_mapping(events),
                            reasoning,
                        });
                    }
                }
            }
        }

        // Fallback: use the response as the prompt directly
        Ok(ArtisticInterpretation {
            artistic_prompt: response.trim().to_string(),
            negative_prompt: "lowres, blurry, text, watermark, distorted, deformed".to_string(),
            symbol_mapping: self.build_symbol_mapping(events),
            reasoning: "Direct LLM response used as prompt".to_string(),
        })
    }

    /// Fallback interpretation when LLM is not available
    fn fallback_interpret(
        &self,
        story: &NarrativeStory,
        events: &[TopiClipCapturedEvent],
    ) -> ArtisticInterpretation {
        // Collect symbol prompts from events
        let symbol_prompts: Vec<&str> = events
            .iter()
            .filter_map(|e| e.symbol_prompt.as_deref())
            .take(3)
            .collect();

        // Build prompt from theme and collected symbols
        let base_prompt = match story.primary_theme.as_str() {
            "growth" => "Luminous crystalline structures emerge from geometric void, ascending toward radiant light, forms multiplying and expanding",
            "struggle" => "Fractured geometric forms strain against invisible forces, cracks of light spreading through dark crystalline surfaces, tension manifest",
            "transformation" => "Metamorphic shapes shift between states, geometric to organic to ethereal, light cascading through transitional forms",
            "connection" => "Golden threads weave between floating geometric structures, each connection sparking with inner light, unity forming from fragments",
            "loss" => "Dissolving particle clouds drift through empty geometric galleries, fading luminescence leaving trails of memory in the void",
            _ => "Abstract geometric forms float in ethereal space, light and shadow dancing across crystalline surfaces",
        };

        // Combine base prompt with top symbol prompt if available
        let artistic_prompt = if !symbol_prompts.is_empty() {
            format!(
                "{}. {}",
                base_prompt,
                symbol_prompts[0]
                    .split('.')
                    .next()
                    .unwrap_or(symbol_prompts[0])
            )
        } else {
            base_prompt.to_string()
        };

        // Add emotional arc modifier
        let emotional_suffix = match story.emotional_arc.as_str() {
            "triumphant" => ", bathed in golden triumphant light",
            "melancholic" => ", suffused with cool blue melancholy",
            "tense" => ", sharp contrasts and urgent energy",
            "peaceful" => ", serene and harmonious",
            "chaotic" => ", swirling dynamic chaos",
            "hopeful" => ", warm light breaking through",
            _ => "",
        };

        let final_prompt = format!("{}{}", artistic_prompt, emotional_suffix);

        ArtisticInterpretation {
            artistic_prompt: final_prompt,
            negative_prompt: "lowres, blurry, text, watermark, distorted, deformed, ugly, bad anatomy, realistic human faces, photographic, mundane".to_string(),
            symbol_mapping: self.build_symbol_mapping(events),
            reasoning: "Fallback interpretation using predefined templates".to_string(),
        }
    }

    fn build_symbol_mapping(&self, events: &[TopiClipCapturedEvent]) -> Value {
        let mut mapping = serde_json::Map::new();

        for event in events {
            if let Some(symbol) = &event.assigned_symbol {
                mapping.insert(
                    event.event_type.clone(),
                    json!({
                        "symbol": symbol,
                        "significance": event.significance_score,
                        "role": event.narrative_role,
                    }),
                );
            }
        }

        Value::Object(mapping)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use sqlx::types::Json;
    use uuid::Uuid;

    fn create_test_story() -> NarrativeStory {
        NarrativeStory {
            events: vec![],
            primary_theme: "growth".to_string(),
            emotional_arc: "triumphant".to_string(),
            narrative_summary: "A period of expansion".to_string(),
            overall_significance: 0.7,
            event_roles: vec![],
        }
    }

    fn create_test_events() -> Vec<TopiClipCapturedEvent> {
        vec![TopiClipCapturedEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            event_type: "ClusterFormed".to_string(),
            event_data: Json(json!({})),
            narrative_role: Some("protagonist".to_string()),
            significance_score: Some(0.8),
            assigned_symbol: Some("Constellation".to_string()),
            symbol_prompt: Some("Luminous figures rise into brilliant constellation".to_string()),
            affected_node_ids: None,
            affected_edge_ids: None,
            occurred_at: Utc::now().to_rfc3339(),
            created_at: Utc::now(),
        }]
    }

    #[test]
    fn test_fallback_interpret() {
        let interpreter = ArtisticInterpreter::new(
            "https://api.anthropic.com/v1/messages".to_string(),
            None,
            "claude-sonnet-4-20250514".to_string(),
            0.8,
        );

        let story = create_test_story();
        let events = create_test_events();

        let result = interpreter.fallback_interpret(&story, &events);

        assert!(!result.artistic_prompt.is_empty());
        assert!(result.artistic_prompt.contains("triumphant"));
        assert!(!result.negative_prompt.is_empty());
    }

    #[test]
    fn test_build_system_prompt() {
        let interpreter = ArtisticInterpreter::new(
            "test".to_string(),
            None,
            "test".to_string(),
            0.8,
        );

        let prompt = interpreter.build_system_prompt();
        assert!(prompt.contains("Beeple"));
        assert!(prompt.contains("surreal"));
    }
}
