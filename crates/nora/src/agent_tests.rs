//! Unit tests for Nora agent

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        agent::{NoraAgent, NoraRequest, NoraRequestType, RequestPriority},
        memory::ConversationMemory,
        personality::PersonalityConfig,
        NoraConfig,
    };

    /// Test basic agent initialization
    #[tokio::test]
    async fn test_agent_initialization() {
        let config = NoraConfig::default();
        let agent = NoraAgent::new(config.clone()).await;

        assert!(agent.is_ok(), "Agent should initialize successfully");
        let agent = agent.unwrap();

        assert_eq!(agent.config.executive_mode, true);
        assert_eq!(agent.config.proactive_notifications, true);
        assert!(agent.is_active.read().await.clone());
    }

    /// Test agent initialization with custom config
    #[tokio::test]
    async fn test_agent_custom_config() {
        let mut config = NoraConfig::default();
        config.executive_mode = false;
        config.personality.accent_strength = 0.5;

        let agent = NoraAgent::new(config.clone()).await;
        assert!(agent.is_ok());
        let agent = agent.unwrap();

        assert_eq!(agent.config.executive_mode, false);
        assert_eq!(agent.config.personality.accent_strength, 0.5);
    }

    /// Test processing a simple text request
    #[tokio::test]
    async fn test_process_text_request() {
        let config = NoraConfig::default();
        let agent = NoraAgent::new(config).await.unwrap();

        let request = NoraRequest {
            request_id: "test-123".to_string(),
            session_id: "session-456".to_string(),
            request_type: NoraRequestType::TextInteraction,
            content: "Hello Nora".to_string(),
            context: None,
            voice_enabled: false,
            priority: RequestPriority::Normal,
            timestamp: Utc::now(),
        };

        let response = agent.process_request(request).await;
        assert!(response.is_ok(), "Request should be processed successfully");

        let response = response.unwrap();
        assert_eq!(response.request_id, "test-123");
        assert_eq!(response.session_id, "session-456");
        assert!(
            !response.content.is_empty(),
            "Response should contain content"
        );
    }

    /// Test task coordination request
    #[tokio::test]
    async fn test_task_coordination_request() {
        let config = NoraConfig::default();
        let agent = NoraAgent::new(config).await.unwrap();

        let request = NoraRequest {
            request_id: "task-123".to_string(),
            session_id: "session-789".to_string(),
            request_type: NoraRequestType::TaskCoordination,
            content: "What are our active projects?".to_string(),
            context: None,
            voice_enabled: false,
            priority: RequestPriority::High,
            timestamp: Utc::now(),
        };

        let response = agent.process_request(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(!response.content.is_empty());
        // Should have context updates for task coordination
        assert!(
            !response.context_updates.is_empty() || response.content.contains("project"),
            "Response should reference projects or have context updates"
        );
    }

    /// Test conversation memory
    #[tokio::test]
    async fn test_conversation_memory() {
        let memory = ConversationMemory::new();

        // Memory functionality is tested through the agent
        // This ensures the struct can be instantiated
        assert!(memory.recent_interactions(10).is_empty());
    }

    /// Test agent lifecycle
    #[tokio::test]
    async fn test_agent_lifecycle() {
        let config = NoraConfig::default();
        let agent = NoraAgent::new(config).await.unwrap();

        assert!(*agent.is_active.read().await);

        // Manually deactivate
        *agent.is_active.write().await = false;
        assert!(!*agent.is_active.read().await);
    }

    /// Test priority handling
    #[tokio::test]
    async fn test_priority_request_handling() {
        let config = NoraConfig::default();
        let agent = NoraAgent::new(config).await.unwrap();

        let urgent_request = NoraRequest {
            request_id: "urgent-1".to_string(),
            session_id: "session-urgent".to_string(),
            request_type: NoraRequestType::DecisionSupport,
            content: "Critical decision needed".to_string(),
            context: None,
            voice_enabled: false,
            priority: RequestPriority::Urgent,
            timestamp: Utc::now(),
        };

        let response = agent.process_request(urgent_request).await;
        assert!(
            response.is_ok(),
            "Urgent request should be processed successfully"
        );

        let response = response.unwrap();
        // Response should acknowledge the request
        assert!(!response.content.is_empty(), "Should have response content");
    }
}
