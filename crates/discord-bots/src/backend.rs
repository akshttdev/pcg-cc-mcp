use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client for calling the PCG CC MCP backend API
#[derive(Debug, Clone)]
pub struct BackendClient {
    client: reqwest::Client,
    base_url: String,
    auth_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoraChatResponse {
    // API returns "content" field; "message" is a fallback alias
    #[serde(alias = "message")]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopsiChatResponse {
    #[serde(alias = "message")]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    #[serde(default)]
    pub topology_changes: Vec<serde_json::Value>,
    pub topology_summary: Option<serde_json::Value>,
    #[serde(default)]
    pub issues: Vec<serde_json::Value>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

impl BackendClient {
    pub fn new(base_url: String, auth_token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            base_url,
            auth_token,
        }
    }

    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/api{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(token) = &self.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        req
    }

    /// Chat with Nora via the backend API
    pub async fn chat_nora(&self, message: &str, session_id: &str) -> Result<NoraChatResponse> {
        let request = ChatRequest {
            message: message.to_string(),
            session_id: Some(session_id.to_string()),
            project_id: None,
            request_type: Some("chat".to_string()),
        };

        let response = self
            .build_request(reqwest::Method::POST, "/nora/chat")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Nora chat failed ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    /// Chat with Topsi via the backend API
    pub async fn chat_topsi(&self, message: &str, session_id: &str) -> Result<TopsiChatResponse> {
        let request = ChatRequest {
            message: message.to_string(),
            session_id: Some(session_id.to_string()),
            project_id: None,
            request_type: Some("chat".to_string()),
        };

        let response = self
            .build_request(reqwest::Method::POST, "/topsi/chat")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Topsi chat failed ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    /// Check if the backend is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    // ========================================================================
    // Meeting API (Topsi meeting mode)
    // ========================================================================

    /// Start a meeting session
    pub async fn start_meeting(
        &self,
        project_id: &str,
        title: Option<&str>,
    ) -> Result<StartMeetingResponse> {
        let response = self
            .build_request(reqwest::Method::POST, "/topsi/meeting/start")
            .json(&serde_json::json!({
                "projectId": project_id,
                "title": title.unwrap_or("Discord Voice Meeting"),
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Start meeting failed ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    /// Send an audio chunk from a meeting for transcription + wake word detection
    pub async fn meeting_audio_chunk(
        &self,
        session_id: &str,
        audio_b64: &str,
        chunk_index: u32,
        duration_ms: u32,
    ) -> Result<MeetingAudioChunkResponse> {
        let response = self
            .build_request(reqwest::Method::POST, "/topsi/meeting/audio")
            .json(&serde_json::json!({
                "sessionId": session_id,
                "audioData": audio_b64,
                "chunkIndex": chunk_index,
                "durationMs": duration_ms,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Meeting audio chunk failed ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    /// Send a text message to a meeting (for logging text that was already transcribed)
    pub async fn meeting_text_message(
        &self,
        session_id: &str,
        text: &str,
        speaker_label: Option<&str>,
    ) -> Result<serde_json::Value> {
        let response = self
            .build_request(reqwest::Method::POST, "/topsi/meeting/message")
            .json(&serde_json::json!({
                "sessionId": session_id,
                "text": text,
                "speakerLabel": speaker_label,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Meeting text message failed ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    /// End a meeting session and optionally generate notes
    pub async fn end_meeting(
        &self,
        session_id: &str,
        generate_notes: bool,
    ) -> Result<EndMeetingResponse> {
        let response = self
            .build_request(reqwest::Method::POST, "/topsi/meeting/end")
            .json(&serde_json::json!({
                "sessionId": session_id,
                "generateNotes": generate_notes,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("End meeting failed ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    /// Send a heartbeat to keep a meeting session alive
    pub async fn meeting_heartbeat(&self, session_id: &str) -> Result<()> {
        let _ = self
            .build_request(reqwest::Method::POST, "/topsi/meeting/heartbeat")
            .json(&serde_json::json!({ "sessionId": session_id }))
            .send()
            .await?;
        Ok(())
    }
}

// ============================================================================
// Meeting types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartMeetingResponse {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAudioChunkResponse {
    pub session_id: String,
    pub chunk_index: u32,
    pub text: String,
    pub speaker_label: Option<String>,
    pub is_topsi_addressed: bool,
    pub topsi_response: Option<String>,
    pub topsi_audio_response: Option<String>,
    pub segment_index: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndMeetingResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: i32,
    pub participant_count: i32,
    pub segment_count: usize,
    pub notes: Option<serde_json::Value>,
}
