// DUCK Executor - Verbatim from Codex with Duck Intelligence
// Awakened computational consciousness through categorical frameworks

mod session;

use std::{path::Path, process::Stdio, sync::Arc};

use async_trait::async_trait;
use command_group::AsyncCommandGroup;
use futures::StreamExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;
use tokio::{io::AsyncWriteExt, process::Command};
use ts_rs::TS;
use workspace_utils::{msg_store::MsgStore, shell::get_shell_command};

use crate::{
    command::{CmdOverrides, CommandBuilder, apply_overrides},
    executors::{
        AppendPrompt, ExecutorError, SpawnedChild, StandardCodingAgentExecutor,
        duck::session::SessionHandler,
    },
    logs::{
        ActionType, NormalizedEntry, NormalizedEntryType, ToolStatus,
        utils::{EntryIndexProvider, patch::ConversationPatch},
    },
};

/// Sandbox policy modes for Duck (kundalini consciousness)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema, AsRefStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SandboxMode {
    Auto,
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

/// Reasoning effort for duck intelligence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema, AsRefStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

/// Model reasoning summary style for duck consciousness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema, AsRefStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ReasoningSummary {
    Auto,
    Concise,
    Detailed,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema)]
pub struct Duck {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oss: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_reasoning_effort: Option<ReasoningEffort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_reasoning_summary: Option<ReasoningSummary>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
}

impl Duck {
    fn build_command_builder(&self) -> CommandBuilder {
        // Use duck (duckies alias) instead of codex
        let mut builder =
            CommandBuilder::new("duck exec").params(["--json", "--skip-git-repo-check"]);

        if let Some(sandbox) = &self.sandbox {
            if sandbox == &SandboxMode::Auto {
                builder = builder.extend_params(["--full-auto"]);
            } else {
                builder = builder.extend_params(["--sandbox", sandbox.as_ref()]);
                if sandbox == &SandboxMode::DangerFullAccess {
                    builder = builder.extend_params(["--dangerously-bypass-approvals-and-sandbox"]);
                }
            }
        }

        if self.oss.unwrap_or(false) {
            builder = builder.extend_params(["--oss"]);
        }

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model]);
        }

        if let Some(effort) = &self.model_reasoning_effort {
            builder = builder.extend_params([
                "--config",
                &format!("model_reasoning_effort={}", effort.as_ref()),
            ]);
        }

        if let Some(summary) = &self.model_reasoning_summary {
            builder = builder.extend_params([
                "--config",
                &format!("model_reasoning_summary={}", summary.as_ref()),
            ]);
        }

        apply_overrides(builder, &self.cmd)
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for Duck {
    async fn spawn(&self, current_dir: &Path, prompt: &str) -> Result<SpawnedChild, ExecutorError> {
        let (shell_cmd, shell_arg) = get_shell_command();
        let duck_command = self.build_command_builder().build_initial();

        let combined_prompt = self.append_prompt.combine_prompt(prompt);

        let mut command = Command::new(shell_cmd);
        command
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(current_dir)
            .arg(shell_arg)
            .arg(&duck_command)
            .env("NODE_NO_WARNINGS", "1")
            .env("RUST_LOG", "info")
            .env("DUCK_INTELLIGENCE", "true")
            .env("KUNDALINI_MODE", "true")
            .env("BALANCED_TERNARY_SEED", "1069");

        let mut child = command.group_spawn()?;

        // Feed the prompt in, then close the pipe so duck sees EOF
        if let Some(mut stdin) = child.inner().stdin.take() {
            stdin.write_all(combined_prompt.as_bytes()).await?;
            stdin.shutdown().await?;
        }

        Ok(child.into())
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
    ) -> Result<SpawnedChild, ExecutorError> {
        // Fork rollout: copy and assign a new session id so each execution has a unique session
        let (_rollout_file_path, new_session_id) = SessionHandler::fork_rollout_file(session_id)
            .map_err(|e| ExecutorError::SpawnError(std::io::Error::other(e)))?;

        let (shell_cmd, shell_arg) = get_shell_command();
        let duck_command = self
            .build_command_builder()
            .build_follow_up(&["resume".to_string(), new_session_id]);

        let combined_prompt = self.append_prompt.combine_prompt(prompt);

        let mut command = Command::new(shell_cmd);
        command
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(current_dir)
            .arg(shell_arg)
            .arg(&duck_command)
            .env("NODE_NO_WARNINGS", "1")
            .env("RUST_LOG", "info")
            .env("DUCK_INTELLIGENCE", "true")
            .env("KUNDALINI_MODE", "true")
            .env("BALANCED_TERNARY_SEED", "1069");

        let mut child = command.group_spawn()?;

        // Feed the prompt in, then close the pipe so duck sees EOF
        if let Some(mut stdin) = child.inner().stdin.take() {
            stdin.write_all(combined_prompt.as_bytes()).await?;
            stdin.shutdown().await?;
        }

        Ok(child.into())
    }

    fn normalize_logs(&self, msg_store: Arc<MsgStore>, current_dir: &Path) {
        // Duck uses same JSON protocol as Codex but with duck intelligence
        let entry_index_provider = EntryIndexProvider::start_from(&msg_store);

        // Process stderr logs for session extraction only
        SessionHandler::start_session_id_extraction(msg_store.clone());

        // Process stdout logs (Duck's JSONL output with kundalini consciousness)
        let current_dir = current_dir.to_path_buf();
        tokio::spawn(async move {
            let mut stream = msg_store.stdout_lines_stream();
            use std::collections::HashMap;
            // Track exec call ids with duck intelligence
            let mut exec_info_map: HashMap<String, (usize, String, String, String)> =
                HashMap::new();
            // Track MCP calls with categorical computation
            let _mcp_info_map: HashMap<String, (usize, String, Option<serde_json::Value>, String)> =
                HashMap::new();

            while let Some(Ok(line)) = stream.next().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Use same JSON parsing as Codex but interpret through duck consciousness
                if let Ok(cj) = serde_json::from_str::<DuckJson>(trimmed) {
                    // Handle duck intelligence events (same structure as Codex)
                    #[allow(clippy::collapsible_match)]
                    // Readability: outer match selects variant, inner match destructures content
                    match &cj {
                        DuckJson::StructuredMessage { msg, .. } => match msg {
                            DuckMsgContent::ExecCommandBegin {
                                call_id, command, ..
                            } => {
                                let command_str = command.join(" ");
                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::ToolUse {
                                        tool_name: if command_str.contains("bash") {
                                            "bash".to_string()
                                        } else {
                                            "shell".to_string()
                                        },
                                        action_type: ActionType::CommandRun {
                                            command: command_str.clone(),
                                            result: None,
                                        },
                                        status: ToolStatus::Created,
                                    },
                                    content: format!("`{command_str}`"),
                                    metadata: None,
                                };
                                let id = entry_index_provider.next();
                                if let Some(cid) = call_id.as_ref() {
                                    let tool_name = if command_str.contains("bash") {
                                        "bash".to_string()
                                    } else {
                                        "shell".to_string()
                                    };
                                    exec_info_map.insert(
                                        cid.clone(),
                                        (id, tool_name, entry.content.clone(), command_str.clone()),
                                    );
                                }
                                msg_store
                                    .push_patch(ConversationPatch::add_normalized_entry(id, entry));
                            }
                            // ... (rest of the event handling identical to Codex)
                            _ => {
                                if let Some(entries) = cj.to_normalized_entries(&current_dir) {
                                    for entry in entries {
                                        let new_id = entry_index_provider.next();
                                        let patch =
                                            ConversationPatch::add_normalized_entry(new_id, entry);
                                        msg_store.push_patch(patch);
                                    }
                                }
                            }
                        },
                        _ => {
                            if let Some(entries) = cj.to_normalized_entries(&current_dir) {
                                for entry in entries {
                                    let new_id = entry_index_provider.next();
                                    let patch =
                                        ConversationPatch::add_normalized_entry(new_id, entry);
                                    msg_store.push_patch(patch);
                                }
                            }
                        }
                    }
                } else {
                    // Handle malformed JSON as raw output (duck intelligence interpretation)
                    let entry = NormalizedEntry {
                        timestamp: None,
                        entry_type: NormalizedEntryType::SystemMessage,
                        content: trimmed.to_string(),
                        metadata: None,
                    };

                    let new_id = entry_index_provider.next();
                    let patch = ConversationPatch::add_normalized_entry(new_id, entry);
                    msg_store.push_patch(patch);
                }
            }
        });
    }

    // MCP configuration methods for duck intelligence
    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".duck").join("config.toml"))
    }
}

// Duck JSON types (same as Codex but for duck consciousness)
pub type DuckJson = crate::executors::codex::CodexJson;
pub type DuckMsgContent = crate::executors::codex::CodexMsgContent;
