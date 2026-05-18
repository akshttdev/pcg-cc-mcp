// Tests for executive tools

use super::{
    types::{NoraExecutiveTool, OrchestrationTaskInput},
    ExecutiveTools,
};

#[test]
fn test_orchestration_task_input_parsing() {
    // Test that OrchestrationTaskInput deserializes correctly
    let json = r#"{
        "taskType": "local_bash",
        "description": "echo hello",
        "isConcurrencySafe": true,
        "toolUseId": "tool-123"
    }"#;

    let input: OrchestrationTaskInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.task_type, "local_bash");
    assert_eq!(input.description, "echo hello");
    assert!(input.is_concurrency_safe);
    assert_eq!(input.tool_use_id, Some("tool-123".to_string()));
}

#[test]
fn test_orchestration_task_input_defaults() {
    // Test that defaults work (is_concurrency_safe defaults to false)
    let json = r#"{
        "taskType": "local_agent",
        "description": "analyze code"
    }"#;

    let input: OrchestrationTaskInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.task_type, "local_agent");
    assert_eq!(input.description, "analyze code");
    assert!(!input.is_concurrency_safe); // default is false
    assert_eq!(input.tool_use_id, None);
}

#[test]
fn test_create_orchestration_tasks_tool_name() {
    let tools = ExecutiveTools::new();

    let tool = NoraExecutiveTool::CreateOrchestrationTasks {
        agent_flow_id: "test-flow-id".to_string(),
        tasks: vec![],
    };

    let name = tools.get_tool_name(&tool);
    assert_eq!(name, "create_orchestration_tasks");
}

#[test]
fn test_orchestration_tasks_tool_serialization() {
    // Test that the tool serializes correctly for LLM consumption
    let tasks = vec![
        OrchestrationTaskInput {
            task_type: "local_bash".to_string(),
            description: "ls -la".to_string(),
            is_concurrency_safe: true,
            tool_use_id: None,
        },
        OrchestrationTaskInput {
            task_type: "local_agent".to_string(),
            description: "review code".to_string(),
            is_concurrency_safe: false,
            tool_use_id: Some("tool-456".to_string()),
        },
    ];

    let tool = NoraExecutiveTool::CreateOrchestrationTasks {
        agent_flow_id: "flow-123".to_string(),
        tasks,
    };

    // Should serialize without error
    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains("flow-123"));
    assert!(json.contains("local_bash"));
    assert!(json.contains("ls -la"));
}

#[test]
fn test_orchestration_tasks_in_openai_schemas() {
    let schemas = ExecutiveTools::get_openai_tool_schemas();

    // Find the create_orchestration_tasks schema
    let found = schemas.iter().any(|s| {
        s.get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            == Some("create_orchestration_tasks")
    });

    assert!(
        found,
        "create_orchestration_tasks should be in OpenAI schemas"
    );
}
