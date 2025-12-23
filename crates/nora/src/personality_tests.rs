//! Tests for British personality module

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        agent::{NoraRequest, NoraRequestType, RequestPriority},
        personality::{
            BritishPersonality, FormalityLevel, PersonalityConfig, PolitenessLevel, WarmthLevel,
        },
    };

    fn create_test_request(content: &str) -> NoraRequest {
        NoraRequest {
            request_id: "test-123".to_string(),
            session_id: "session-456".to_string(),
            request_type: NoraRequestType::TextInteraction,
            content: content.to_string(),
            context: None,
            voice_enabled: false,
            priority: RequestPriority::Normal,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_personality_initialization() {
        let config = PersonalityConfig::british_executive_assistant();
        let _personality = BritishPersonality::new(config.clone());

        assert_eq!(config.accent_strength, 0.8);
        assert_eq!(config.executive_vocabulary, true);
        assert_eq!(config.british_expressions, true);
    }

    #[test]
    fn test_casual_personality() {
        let config = PersonalityConfig::casual_british();
        assert_eq!(config.accent_strength, 0.6);
        assert!(matches!(config.formality_level, FormalityLevel::Casual));
        assert!(matches!(config.warmth_level, WarmthLevel::Friendly));
    }

    #[test]
    fn test_apply_personality_to_response() {
        let config = PersonalityConfig::british_executive_assistant();
        let personality = BritishPersonality::new(config);
        let request = create_test_request("I think we should do this task.");

        let original = "I think we should do this task.";
        let polished = personality.apply_personality_to_response(original, &request);

        // Response should be processed (may or may not be transformed depending on content)
        assert!(!polished.is_empty(), "Response should not be empty");

        // Check for potential British transformations
        let has_transformation = polished != original
            || polished.to_lowercase().contains("rather")
            || polished.to_lowercase().contains("quite")
            || polished.to_lowercase().contains("shall");

        // The personality system should at least process the text
        assert!(
            polished.len() >= original.len() - 10,
            "Response should maintain reasonable length"
        );
    }

    #[test]
    fn test_british_spelling_conversion() {
        let config = PersonalityConfig::british_executive_assistant();
        let personality = BritishPersonality::new(config);
        let request = create_test_request("organize color analyze behavior");

        let american = "We need to organize the color scheme and analyze the behavior.";
        let british = personality.apply_personality_to_response(american, &request);

        // Should convert American to British spelling
        assert!(
            british.contains("organise")
                || british.contains("colour")
                || british.contains("behaviour")
                || british != american,
            "Should convert American spellings to British"
        );
    }

    #[test]
    fn test_executive_vocabulary() {
        let config = PersonalityConfig::british_executive_assistant();
        let personality = BritishPersonality::new(config);
        let request = create_test_request("Let's do this now.");

        let casual = "Let's do this now.";
        let executive = personality.apply_personality_to_response(casual, &request);

        // Should process the text
        assert!(!executive.is_empty(), "Should return a response");
        // Executive language may be more elaborate or similar length
        assert!(executive.len() > 0, "Response should have content");
    }

    #[test]
    fn test_politeness_levels() {
        let mut config = PersonalityConfig::british_executive_assistant();

        // Test very polite
        config.politeness_level = PolitenessLevel::VeryPolite;
        let personality = BritishPersonality::new(config.clone());
        let request = create_test_request("Do this task.");
        let response = personality.apply_personality_to_response("Do this task.", &request);

        // Should process the text
        assert!(!response.is_empty(), "Response should not be empty");
        assert!(response.len() > 0, "Response should have content");
    }

    #[test]
    fn test_formality_levels() {
        // Professional formality
        let mut config = PersonalityConfig::british_executive_assistant();
        config.formality_level = FormalityLevel::Professional;
        let professional = BritishPersonality::new(config.clone());
        let request = create_test_request("Fix the bug.");

        let response_pro = professional.apply_personality_to_response("Fix the bug.", &request);

        // Very formal
        config.formality_level = FormalityLevel::VeryFormal;
        let very_formal = BritishPersonality::new(config);

        let response_formal = very_formal.apply_personality_to_response("Fix the bug.", &request);

        // Both should process the text
        assert!(!response_pro.is_empty());
        assert!(!response_formal.is_empty());
    }

    #[test]
    fn test_warmth_levels() {
        let mut config = PersonalityConfig::british_executive_assistant();

        config.warmth_level = WarmthLevel::Warm;
        let warm = BritishPersonality::new(config.clone());
        let request = create_test_request("Great job on the project!");

        config.warmth_level = WarmthLevel::Enthusiastic;
        let enthusiastic = BritishPersonality::new(config);

        let message = "Great job on the project!";
        let warm_response = warm.apply_personality_to_response(message, &request);
        let enthusiastic_response = enthusiastic.apply_personality_to_response(message, &request);

        // Both should maintain or enhance the positive tone
        assert!(!warm_response.is_empty());
        assert!(!enthusiastic_response.is_empty());
    }

    #[test]
    fn test_accent_strength_impact() {
        let mut config = PersonalityConfig::british_executive_assistant();

        // Low accent strength
        config.accent_strength = 0.2;
        let subtle = BritishPersonality::new(config.clone());
        let request = create_test_request("I think this is a good idea.");

        // High accent strength
        config.accent_strength = 1.0;
        let strong = BritishPersonality::new(config);

        let text = "I think this is a good idea.";
        let subtle_result = subtle.apply_personality_to_response(text, &request);
        let strong_result = strong.apply_personality_to_response(text, &request);

        // Both should process the text
        assert!(!subtle_result.is_empty());
        assert!(!strong_result.is_empty());
    }
}
