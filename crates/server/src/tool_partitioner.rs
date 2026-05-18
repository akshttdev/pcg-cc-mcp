//! Tool Call Partitioner
//!
//! Partitions tool calls into batches based on concurrency safety.
//! Mirrors demo_impl/src/services/tools/toolOrchestration.ts

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to execute
    pub name: String,
    /// Input arguments as JSON
    pub input: Value,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

/// Metadata about a tool's concurrency characteristics
#[derive(Debug, Clone, Default)]
pub struct ToolMetadata {
    /// Whether this tool can safely run in parallel with others
    pub is_concurrency_safe: bool,
    /// Whether this tool only reads state (no mutations)
    pub is_read_only: bool,
}

impl ToolMetadata {
    pub fn safe_read_only() -> Self {
        Self {
            is_concurrency_safe: true,
            is_read_only: true,
        }
    }

    pub fn safe_mutating() -> Self {
        Self {
            is_concurrency_safe: true,
            is_read_only: false,
        }
    }

    pub fn unsafe_mutating() -> Self {
        Self {
            is_concurrency_safe: false,
            is_read_only: false,
        }
    }
}

/// A batch of tool calls that can be executed together
#[derive(Debug, Clone)]
pub struct ToolBatch {
    /// If true, all calls in this batch can run concurrently
    pub is_concurrency_safe: bool,
    /// Tool calls in this batch
    pub calls: Vec<ToolCall>,
}

impl ToolBatch {
    fn new(is_concurrency_safe: bool) -> Self {
        Self {
            is_concurrency_safe,
            calls: Vec::new(),
        }
    }

    fn with_call(is_concurrency_safe: bool, call: ToolCall) -> Self {
        Self {
            is_concurrency_safe,
            calls: vec![call],
        }
    }
}

// ── Partitioning Logic ────────────────────────────────────────────────────────

/// Partition tool calls into batches:
/// - Consecutive safe calls are grouped together (can run in parallel)
/// - Each unsafe call gets its own batch (must run serially)
/// - Unknown tools are treated as unsafe (conservative default)
pub fn partition_tool_calls(
    calls: Vec<ToolCall>,
    tool_metadata: &HashMap<String, ToolMetadata>,
) -> Vec<ToolBatch> {
    if calls.is_empty() {
        return Vec::new();
    }

    calls.into_iter().fold(Vec::new(), |mut acc, call| {
        let is_safe = tool_metadata
            .get(&call.name)
            .map(|m| m.is_concurrency_safe)
            .unwrap_or(false); // Conservative: unknown tools are unsafe

        match acc.last_mut() {
            // Append to existing safe batch if current call is also safe
            Some(batch) if batch.is_concurrency_safe && is_safe => {
                batch.calls.push(call);
            }
            // Otherwise, create a new batch
            _ => {
                acc.push(ToolBatch::with_call(is_safe, call));
            }
        }
        acc
    })
}

// ── Default Tool Registry ─────────────────────────────────────────────────────

/// Build default tool metadata for PCG-CC-MCP's built-in tools
pub fn default_tool_metadata() -> HashMap<String, ToolMetadata> {
    let mut m = HashMap::new();

    // ── Read-only, concurrency-safe tools ────────────────────────────────────
    // These tools only read data and don't modify any state
    for name in [
        "get_deal_context",
        "get_task_context",
        "read_file",
        "glob",
        "grep",
        "list_tasks",
        "list_projects",
        "get_contact_info",
        "search_knowledge",
    ] {
        m.insert(name.to_string(), ToolMetadata::safe_read_only());
    }

    // ── Stateful but concurrency-safe tools ──────────────────────────────────
    // These modify state but operate on independent resources
    for name in [
        "save_artifact", // Each artifact is independent
        "create_task",   // Creating tasks is idempotent per unique ID
        "log_activity",  // Append-only logging
    ] {
        m.insert(name.to_string(), ToolMetadata::safe_mutating());
    }

    // ── Unsafe tools (must run serially) ─────────────────────────────────────
    // These modify shared state and could cause conflicts
    for name in [
        "update_deal_field",
        "update_task_notes",
        "write_file",
        "bash",
        "submit_response",
        "advance_pipeline_stage",
        "send_email",
        "send_sms",
        "update_contact",
    ] {
        m.insert(name.to_string(), ToolMetadata::unsafe_mutating());
    }

    m
}

/// Check if a specific tool is concurrency-safe
pub fn is_tool_safe(tool_name: &str, metadata: &HashMap<String, ToolMetadata>) -> bool {
    metadata
        .get(tool_name)
        .map(|m| m.is_concurrency_safe)
        .unwrap_or(false)
}

/// Count how many tools in a list are concurrency-safe
pub fn count_safe_tools(tool_names: &[&str], metadata: &HashMap<String, ToolMetadata>) -> usize {
    tool_names
        .iter()
        .filter(|name| is_tool_safe(name, metadata))
        .count()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn make_call(name: &str) -> ToolCall {
        ToolCall::new(format!("id-{}", name), name, json!({}))
    }

    fn test_metadata() -> HashMap<String, ToolMetadata> {
        let mut m = HashMap::new();
        m.insert("read".to_string(), ToolMetadata::safe_read_only());
        m.insert("glob".to_string(), ToolMetadata::safe_read_only());
        m.insert("grep".to_string(), ToolMetadata::safe_read_only());
        m.insert("write".to_string(), ToolMetadata::unsafe_mutating());
        m.insert("bash".to_string(), ToolMetadata::unsafe_mutating());
        m
    }

    #[test]
    fn test_partition_empty() {
        let batches = partition_tool_calls(vec![], &test_metadata());
        assert!(batches.is_empty());
    }

    #[test]
    fn test_partition_single_safe() {
        let batches = partition_tool_calls(vec![make_call("read")], &test_metadata());
        assert_eq!(batches.len(), 1);
        assert!(batches[0].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 1);
    }

    #[test]
    fn test_partition_single_unsafe() {
        let batches = partition_tool_calls(vec![make_call("write")], &test_metadata());
        assert_eq!(batches.len(), 1);
        assert!(!batches[0].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 1);
    }

    #[test]
    fn test_partition_all_safe() {
        let calls = vec![make_call("read"), make_call("glob"), make_call("grep")];
        let batches = partition_tool_calls(calls, &test_metadata());

        assert_eq!(batches.len(), 1);
        assert!(batches[0].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 3);
    }

    #[test]
    fn test_partition_all_unsafe() {
        let calls = vec![make_call("write"), make_call("bash")];
        let batches = partition_tool_calls(calls, &test_metadata());

        // Each unsafe call gets its own batch
        assert_eq!(batches.len(), 2);
        assert!(!batches[0].is_concurrency_safe);
        assert!(!batches[1].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 1);
        assert_eq!(batches[1].calls.len(), 1);
    }

    #[test]
    fn test_partition_mixed() {
        // safe, safe, unsafe, safe, safe
        let calls = vec![
            make_call("read"),
            make_call("glob"),
            make_call("write"), // unsafe - breaks batch
            make_call("read"),
            make_call("grep"),
        ];
        let batches = partition_tool_calls(calls, &test_metadata());

        assert_eq!(batches.len(), 3);

        // First batch: 2 safe calls
        assert!(batches[0].is_concurrency_safe);
        assert_eq!(batches[0].calls.len(), 2);

        // Second batch: 1 unsafe call
        assert!(!batches[1].is_concurrency_safe);
        assert_eq!(batches[1].calls.len(), 1);
        assert_eq!(batches[1].calls[0].name, "write");

        // Third batch: 2 safe calls
        assert!(batches[2].is_concurrency_safe);
        assert_eq!(batches[2].calls.len(), 2);
    }

    #[test]
    fn test_partition_unknown_tools_are_unsafe() {
        let calls = vec![make_call("unknown_tool")];
        let batches = partition_tool_calls(calls, &test_metadata());

        assert_eq!(batches.len(), 1);
        assert!(!batches[0].is_concurrency_safe);
    }

    #[test]
    fn test_partition_alternating() {
        // safe, unsafe, safe, unsafe
        let calls = vec![
            make_call("read"),
            make_call("write"),
            make_call("glob"),
            make_call("bash"),
        ];
        let batches = partition_tool_calls(calls, &test_metadata());

        assert_eq!(batches.len(), 4);
        assert!(batches[0].is_concurrency_safe);
        assert!(!batches[1].is_concurrency_safe);
        assert!(batches[2].is_concurrency_safe);
        assert!(!batches[3].is_concurrency_safe);
    }

    #[test]
    fn test_default_metadata_has_expected_tools() {
        let metadata = default_tool_metadata();

        // Read-only tools should be safe
        assert!(
            metadata
                .get("get_deal_context")
                .unwrap()
                .is_concurrency_safe
        );
        assert!(metadata.get("get_deal_context").unwrap().is_read_only);
        assert!(metadata.get("glob").unwrap().is_concurrency_safe);

        // Mutating but safe tools
        assert!(metadata.get("save_artifact").unwrap().is_concurrency_safe);
        assert!(!metadata.get("save_artifact").unwrap().is_read_only);

        // Unsafe tools
        assert!(
            !metadata
                .get("update_deal_field")
                .unwrap()
                .is_concurrency_safe
        );
        assert!(!metadata.get("write_file").unwrap().is_concurrency_safe);
        assert!(!metadata.get("bash").unwrap().is_concurrency_safe);
    }

    #[test]
    fn test_is_tool_safe() {
        let metadata = test_metadata();
        assert!(is_tool_safe("read", &metadata));
        assert!(is_tool_safe("glob", &metadata));
        assert!(!is_tool_safe("write", &metadata));
        assert!(!is_tool_safe("unknown", &metadata));
    }

    #[test]
    fn test_count_safe_tools() {
        let metadata = test_metadata();
        assert_eq!(count_safe_tools(&["read", "glob", "grep"], &metadata), 3);
        assert_eq!(count_safe_tools(&["read", "write", "glob"], &metadata), 2);
        assert_eq!(count_safe_tools(&["write", "bash"], &metadata), 0);
        assert_eq!(count_safe_tools(&["unknown"], &metadata), 0);
    }

    #[test]
    fn test_tool_call_creation() {
        let call = ToolCall::new("test-id", "read_file", json!({"path": "/tmp/test.txt"}));
        assert_eq!(call.id, "test-id");
        assert_eq!(call.name, "read_file");
        assert_eq!(call.input["path"], "/tmp/test.txt");
    }
}
