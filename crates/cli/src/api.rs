//! API Client for PCG Dashboard
//!
//! Handles all HTTP communication with the PCG backend server.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API client for ORCHA Dashboard
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Login with username/password and store session cookie
    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        let resp = self
            .client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .context("Failed to connect to server")?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            let user = &body["data"]["user"]["full_name"];
            Ok(user.as_str().unwrap_or("Unknown").to_string())
        } else {
            anyhow::bail!("Login failed: invalid credentials")
        }
    }

    // ============ Projects ============

    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        let resp = self
            .client
            .get(format!("{}/api/projects", self.base_url))
            .send()
            .await
            .context("Failed to fetch projects")?;

        if resp.status().is_success() {
            let text = resp.text().await?;
            // Check if response is HTML (frontend) instead of JSON
            if text.starts_with("<!DOCTYPE") || text.starts_with("<html") {
                anyhow::bail!("Authentication required. Run: orcha config --set server.username=YOUR_USER");
            }
            // Try parsing as wrapped response {"success": true, "data": [...]}
            if let Ok(wrapped) = serde_json::from_str::<ApiResponse<Vec<Project>>>(&text) {
                return Ok(wrapped.data.unwrap_or_default());
            }
            // Fallback: try parsing as raw array
            let projects: Vec<Project> = serde_json::from_str(&text)
                .context("Failed to parse projects response")?;
            Ok(projects)
        } else if resp.status().as_u16() == 401 {
            anyhow::bail!("Authentication required. Run: orcha config --set server.username=YOUR_USER");
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch projects: {} - {}", status, text);
        }
    }

    pub async fn get_project(&self, id: Uuid) -> Result<Option<Project>> {
        let resp = self
            .client
            .get(format!("{}/api/projects/{}", self.base_url, id))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(Some(resp.json().await?))
        } else if resp.status().as_u16() == 404 {
            Ok(None)
        } else {
            anyhow::bail!("Failed to fetch project: {}", resp.status())
        }
    }

    pub async fn find_project_by_name(&self, name: &str) -> Result<Option<Project>> {
        let projects = self.list_projects().await?;
        Ok(projects
            .into_iter()
            .find(|p| p.name.to_lowercase() == name.to_lowercase()))
    }

    pub async fn create_project(&self, name: &str, git_repo_path: &str, description: Option<&str>) -> Result<Project> {
        let request = serde_json::json!({
            "name": name,
            "gitRepoPath": git_repo_path,
            "useExistingRepo": false,
        });

        let resp = self
            .client
            .post(format!("{}/api/projects", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to create project")?;

        if resp.status().is_success() {
            let text = resp.text().await?;
            if let Ok(wrapped) = serde_json::from_str::<ApiResponse<Project>>(&text) {
                if let Some(project) = wrapped.data {
                    return Ok(project);
                }
                anyhow::bail!("Server returned success but no project data");
            }
            let project: Project = serde_json::from_str(&text)
                .context("Failed to parse create project response")?;
            Ok(project)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create project: {} - {}", status, text)
        }
    }

    // ============ Tasks ============

    pub async fn list_tasks(
        &self,
        project_id: Uuid,
        status: Option<&str>,
    ) -> Result<Vec<Task>> {
        let mut url = format!("{}/api/projects/{}/tasks", self.base_url, project_id);
        if let Some(s) = status {
            url.push_str(&format!("?status={}", s));
        }

        let resp = self.client.get(&url).send().await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            anyhow::bail!("Failed to fetch tasks: {}", resp.status())
        }
    }

    pub async fn create_task(&self, project_id: Uuid, board_id: Option<Uuid>, request: &CreateTaskRequest) -> Result<Task> {
        // Use NORA endpoint (no auth required)
        // If no board_id provided, use a well-known default board ID for PCG project
        // TODO: Fetch boards from API when auth is available
        let board_id_str = match board_id {
            Some(b) => b.to_string(),
            None => {
                // Development History board for PCG project
                // This is a temporary workaround until we have proper board discovery
                "a62c5b26-c253-4ca2-ab5d-7db4e0b089a4".to_string()
            }
        };

        let nora_request = serde_json::json!({
            "projectId": project_id.to_string(),
            "boardId": board_id_str,
            "title": request.title,
            "description": request.description,
        });

        let resp = self
            .client
            .post(format!("{}/api/nora/task/create", self.base_url))
            .json(&nora_request)
            .send()
            .await?;

        if resp.status().is_success() {
            let nora_resp: NoraTaskResponse = resp.json().await?;
            Ok(Task {
                id: Uuid::parse_str(&nora_resp.task_id)?,
                project_id,
                title: nora_resp.title,
                description: request.description.clone(),
                status: nora_resp.status,
                created_at: Some(nora_resp.created_at),
                updated_at: None,
            })
        } else {
            let err_text = resp.text().await?;
            anyhow::bail!("Failed to create task: {}", err_text)
        }
    }

    pub async fn update_task(&self, task_id: Uuid, update: &UpdateTaskRequest) -> Result<Task> {
        let resp = self
            .client
            .patch(format!("{}/api/tasks/{}", self.base_url, task_id))
            .json(&update)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            let err_text = resp.text().await?;
            anyhow::bail!("Failed to update task: {}", err_text)
        }
    }

    // ============ Development Sessions ============

    pub async fn start_session(&self, request: &StartSessionRequest) -> Result<DevSession> {
        let resp = self
            .client
            .post(format!("{}/api/dev-sessions/start", self.base_url))
            .json(&request)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            // Session endpoints may not exist yet, create a local session
            Ok(DevSession {
                id: Uuid::new_v4(),
                project_id: request.project_id,
                title: request.title.clone(),
                status: "active".to_string(),
                started_at: chrono::Utc::now(),
                ended_at: None,
                total_tokens_used: 0,
                total_vibe_cost: 0,
                tasks_created: 0,
                tasks_completed: 0,
            })
        }
    }

    pub async fn complete_session(&self, session_id: Uuid) -> Result<SessionReport> {
        let resp = self
            .client
            .post(format!(
                "{}/api/dev-sessions/{}/complete",
                self.base_url, session_id
            ))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            // Return a basic report if endpoint doesn't exist
            Ok(SessionReport {
                session_id,
                duration_minutes: 0,
                total_tokens: 0,
                total_vibe_cost: 0,
                tasks_created: 0,
                tasks_completed: 0,
                files_changed: 0,
                lines_added: 0,
                lines_removed: 0,
            })
        }
    }

    pub async fn get_active_session(&self, project_id: Uuid) -> Result<Option<DevSession>> {
        let resp = self
            .client
            .get(format!(
                "{}/api/dev-sessions/active?project_id={}",
                self.base_url, project_id
            ))
            .send()
            .await?;

        if resp.status().is_success() {
            let sessions: Vec<DevSession> = resp.json().await?;
            Ok(sessions.into_iter().next())
        } else {
            Ok(None)
        }
    }

    // ============ Agent Chat ============

    pub async fn chat_with_agent(
        &self,
        agent_id: Uuid,
        message: &str,
        session_id: &str,
        project_id: Option<Uuid>,
        model: Option<&str>,
        provider: Option<&str>,
    ) -> Result<AgentChatResponse> {
        let request = AgentChatRequest {
            message: message.to_string(),
            session_id: session_id.to_string(),
            project_id,
            context: None,
            stream: false,
            model: model.map(|s| s.to_string()),
            provider: provider.map(|s| s.to_string()),
        };

        let resp = self
            .client
            .post(format!("{}/api/agents/{}/chat", self.base_url, agent_id))
            .json(&request)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            let err_text = resp.text().await?;
            anyhow::bail!("Agent chat failed: {}", err_text)
        }
    }

    /// Chat directly with Topsi platform agent via dedicated endpoint
    pub async fn chat_with_topsi(
        &self,
        message: &str,
        session_id: &str,
        project_id: Option<Uuid>,
        context: Option<serde_json::Value>,
    ) -> Result<TopsiChatResponse> {
        let request = serde_json::json!({
            "message": message,
            "sessionId": session_id,
            "projectId": project_id,
            "context": context,
        });

        let resp = self
            .client
            .post(format!("{}/api/topsi/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;

            // Extract tool names from toolCalls array
            let tool_calls = body["toolCalls"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tc| tc["toolName"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            Ok(TopsiChatResponse {
                content: body["message"].as_str().unwrap_or("").to_string(),
                input_tokens: body["inputTokens"].as_i64(),
                output_tokens: body["outputTokens"].as_i64(),
                tool_calls,
            })
        } else {
            let err_text = resp.text().await?;
            anyhow::bail!("Topsi chat failed: {}", err_text)
        }
    }

    // ============ Agents ============

    pub async fn list_agents(&self) -> Result<Vec<Agent>> {
        let resp = self
            .client
            .get(format!("{}/api/agents", self.base_url))
            .send()
            .await?;

        if resp.status().is_success() {
            let text = resp.text().await?;
            // Try wrapped response first
            if let Ok(wrapped) = serde_json::from_str::<ApiResponse<Vec<Agent>>>(&text) {
                return Ok(wrapped.data.unwrap_or_default());
            }
            // Try raw array
            let agents: Vec<Agent> = serde_json::from_str(&text).unwrap_or_default();
            Ok(agents)
        } else {
            tracing::debug!("Agents endpoint returned {}", resp.status());
            Ok(vec![])
        }
    }

    pub async fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>> {
        let agents = self.list_agents().await?;
        Ok(agents
            .into_iter()
            .find(|a| a.short_name.to_lowercase() == name.to_lowercase()))
    }

    // ============ Health Check ============

    pub async fn health_check(&self) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/api/health", self.base_url))
            .send()
            .await;

        match resp {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

// ============ API Response Wrapper ============

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    #[allow(dead_code)]
    pub success: bool,
    pub data: Option<T>,
}

// ============ Data Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub git_repo_path: Option<String>,
    #[serde(default)]
    pub vibe_budget_limit: Option<i64>,
    #[serde(default)]
    pub vibe_spent_amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_created_by")]
    pub created_by: String,
}

fn default_created_by() -> String {
    "pcg-cli".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub total_tokens_used: i64,
    #[serde(default)]
    pub total_vibe_cost: i64,
    #[serde(default)]
    pub tasks_created: i32,
    #[serde(default)]
    pub tasks_completed: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    pub project_id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_start_sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReport {
    pub session_id: Uuid,
    pub duration_minutes: i64,
    pub total_tokens: i64,
    pub total_vibe_cost: i64,
    pub tasks_created: i32,
    pub tasks_completed: i32,
    pub files_changed: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatRequest {
    pub message: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    /// Optional model override (e.g. "llama3.2:3b", "gpt-4o", "claude-sonnet-4")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional provider override ("ollama", "openai", "anthropic")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatResponse {
    pub content: String,
    pub conversation_id: Uuid,
    pub agent_name: String,
    pub agent_designation: String,
    #[serde(default)]
    pub input_tokens: Option<i64>,
    #[serde(default)]
    pub output_tokens: Option<i64>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub latency_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub short_name: String,
    pub designation: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopsiChatResponse {
    pub content: String,
    #[serde(default)]
    pub input_tokens: Option<i64>,
    #[serde(default)]
    pub output_tokens: Option<i64>,
    /// Names of tools Topsi invoked while processing the request
    #[serde(default)]
    pub tool_calls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoraTaskResponse {
    pub task_id: String,
    pub project_id: String,
    pub board_id: Option<String>,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
}
