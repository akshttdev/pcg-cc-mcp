//! Tests for executive tools

#[cfg(test)]
mod tests {
    use super::super::*;
    #[tokio::test]
    async fn test_read_file_tool() {
        let tools = ExecutiveTools::new();

        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nora_test_read.txt");
        tokio::fs::write(&test_file, "Hello, Nora!").await.unwrap();

        let tool = NoraExecutiveTool::ReadFile {
            file_path: test_file.to_str().unwrap().to_string(),
            encoding: None,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["content"].as_str().unwrap(), "Hello, Nora!");

        // Cleanup
        tokio::fs::remove_file(&test_file).await.ok();
    }

    #[tokio::test]
    async fn test_write_file_tool() {
        let tools = ExecutiveTools::new();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nora_test_write.txt");

        let tool = NoraExecutiveTool::WriteFile {
            file_path: test_file.to_str().unwrap().to_string(),
            content: "Test content from Nora".to_string(),
            create_directories: true,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());

        // Verify file was created
        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "Test content from Nora");

        // Cleanup
        tokio::fs::remove_file(&test_file).await.ok();
    }

    #[tokio::test]
    async fn test_list_directory_tool() {
        let tools = ExecutiveTools::new();

        let temp_dir = std::env::temp_dir();

        let tool = NoraExecutiveTool::ListDirectory {
            directory_path: temp_dir.to_str().unwrap().to_string(),
            recursive: false,
            pattern: None,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["entries"].is_array());
    }

    #[tokio::test]
    async fn test_delete_file_tool() {
        let tools = ExecutiveTools::new();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nora_test_delete.txt");
        tokio::fs::write(&test_file, "Delete me").await.unwrap();

        let tool = NoraExecutiveTool::DeleteFile {
            file_path: test_file.to_str().unwrap().to_string(),
            confirm: true,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_analyze_code_quality_tool() {
        let tools = ExecutiveTools::new();

        let code = r#"
fn hello_world() {
    // This is a comment
    println!("Hello, world!");
}
"#;

        let tool = NoraExecutiveTool::AnalyzeCodeQuality {
            code: code.to_string(),
            language: CodeLanguage::Rust,
            check_security: false,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["metrics"]["line_count"].as_u64().unwrap() > 0);
        assert!(result["metrics"]["has_comments"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_send_email_tool() {
        let tools = ExecutiveTools::new();

        let tool = NoraExecutiveTool::SendEmail {
            recipients: vec!["test@example.com".to_string()],
            subject: "Test Email".to_string(),
            body: "This is a test".to_string(),
            priority: EmailPriority::Normal,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["message_id"].is_string());
    }

    #[tokio::test]
    async fn test_create_notification_tool() {
        let tools = ExecutiveTools::new();

        let tool = NoraExecutiveTool::CreateNotification {
            title: "Test Notification".to_string(),
            message: "This is a test notification".to_string(),
            notification_type: NotificationType::Info,
            recipients: vec!["user1".to_string()],
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["notification_id"].is_string());
    }

    #[test]
    fn test_run_visual_qc_tool_name() {
        let tools = ExecutiveTools::new();
        let tool = NoraExecutiveTool::RunVisualQc {
            batch_id: "test-batch-id".to_string(),
            candidates_per_clip: Some(5),
            min_composition_score: Some(0.6),
            target_aspect_ratio: Some("16:9".to_string()),
            project_id: None,
        };

        assert_eq!(tools.get_tool_name(&tool), "run_visual_qc");
    }

    #[test]
    fn test_run_visual_qc_tool_parse() {
        let _tools = ExecutiveTools::new();
        let args = serde_json::json!({
            "batch_id": "abc-123",
            "candidates_per_clip": 3,
            "min_composition_score": 0.7,
            "target_aspect_ratio": "16:9"
        });

        let parsed = ExecutiveTools::parse_tool_call("run_visual_qc", &args);
        assert!(parsed.is_some());

        if let Some(NoraExecutiveTool::RunVisualQc {
            batch_id,
            candidates_per_clip,
            min_composition_score,
            target_aspect_ratio,
            project_id,
        }) = parsed
        {
            assert_eq!(batch_id, "abc-123");
            assert_eq!(candidates_per_clip, Some(3));
            assert!((min_composition_score.unwrap() - 0.7).abs() < 0.001);
            assert_eq!(target_aspect_ratio.as_deref(), Some("16:9"));
            assert!(project_id.is_none());
        } else {
            panic!("Expected RunVisualQc variant");
        }
    }

    #[test]
    fn test_run_visual_qc_tool_in_definitions() {
        let tools = ExecutiveTools::new();
        assert!(
            tools.available_tools.contains_key("run_visual_qc"),
            "run_visual_qc should be in tool definitions"
        );
    }
}
