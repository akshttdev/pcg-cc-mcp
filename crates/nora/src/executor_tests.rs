//! Tests for task executor

#[cfg(test)]
mod tests {
    use crate::executor::{TaskDefinition, TaskExecutor};
    use db::models::task::Priority;

    #[tokio::test]
    async fn test_executor_initialization() {
        // Test that TaskExecutor can be created
        // This is mainly a compilation test since we can't easily test DB operations
        // without full migrations in unit tests
        let _task_def = TaskDefinition {
            title: "Test Task".to_string(),
            description: Some("Test description".to_string()),
            priority: Some(Priority::High),
            tags: Some(vec!["test".to_string()]),
            assignee_id: None,
            board_id: None,
            pod_id: None,
        };
        
        // Verify the struct can be instantiated
        assert_eq!(_task_def.title, "Test Task");
    }

    #[test]
    fn test_task_definition_creation() {
        let task_def = TaskDefinition {
            title: "Integration Test".to_string(),
            description: Some("Integration test task".to_string()),
            priority: Some(Priority::Medium),
            tags: Some(vec!["integration".to_string(), "test".to_string()]),
            assignee_id: Some("user-123".to_string()),
            board_id: None,
            pod_id: None,
        };

        assert_eq!(task_def.title, "Integration Test");
        assert_eq!(task_def.priority, Some(Priority::Medium));
        assert_eq!(task_def.tags, Some(vec!["integration".to_string(), "test".to_string()]));
        assert_eq!(task_def.assignee_id, Some("user-123".to_string()));
    }

    #[test]
    fn test_task_definition_minimal() {
        let task_def = TaskDefinition {
            title: "Minimal Task".to_string(),
            description: None,
            priority: None,
            tags: None,
            assignee_id: None,
            board_id: None,
            pod_id: None,
        };

        assert_eq!(task_def.title, "Minimal Task");
        assert_eq!(task_def.description, None);
        assert_eq!(task_def.priority, None);
    }

    #[test]
    fn test_priority_enum() {
        let priorities = vec![
            Priority::Critical,
            Priority::High,
            Priority::Medium,
            Priority::Low,
        ];

        assert_eq!(priorities.len(), 4);
        assert!(matches!(priorities[0], Priority::Critical));
        assert!(matches!(priorities[1], Priority::High));
    }
}
