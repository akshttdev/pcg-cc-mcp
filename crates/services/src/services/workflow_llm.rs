//! Workflow LLM Service — tool-calling-aware LLM routing for the workflow engine.
//!
//! Extends the PCG Router's provider dispatch with structured tool-calling support
//! for all providers (Anthropic, OpenAI-compat, Gemini). The workflow engine uses
//! this instead of calling `pcg_router::route_completion()` directly, gaining:
//!
//!   - Tool definitions sent to the provider in provider-native format
//!   - Parsed tool-call responses (`LLMResponse::ToolCalls`)
//!   - Structured output extraction via single-tool-call pattern
//!   - Provider-agnostic message helpers for multi-turn tool conversations

use db::models::pcg_router_model::PcgRouterModel;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

// ── Public types ────────────────────────────────────────────────────────────

/// A tool the LLM can call, described as a JSON Schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema describing the tool's parameters.
    pub parameters: Value,
}

/// A single tool invocation requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Provider-assigned call ID (used to correlate results).
    pub id: String,
    pub name: String,
    /// Parsed arguments object.
    pub arguments: Value,
}

/// Token usage counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// The two possible shapes of an LLM response.
#[derive(Debug, Clone)]
pub enum LLMResponse {
    /// Plain text completion.
    Text { content: String, usage: TokenUsage },
    /// One or more tool calls the caller should execute.
    ToolCalls {
        calls: Vec<ToolCallRequest>,
        usage: TokenUsage,
    },
}

impl LLMResponse {
    pub fn usage(&self) -> &TokenUsage {
        match self {
            LLMResponse::Text { usage, .. } => usage,
            LLMResponse::ToolCalls { usage, .. } => usage,
        }
    }
}

/// Routing metadata — mirrors `pcg_router::RoutingMetadata` but is owned by
/// this module so callers don't need to depend on the server crate.
#[derive(Debug, Clone, Serialize)]
pub struct RoutingMetadata {
    pub model_used: String,
    pub provider: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_micros: Option<i64>,
}

// ── Service ─────────────────────────────────────────────────────────────────

/// Stateless LLM service for workflow tool-calling completions.
///
/// All methods are async and take a `&SqlitePool` so the caller controls the
/// connection lifetime. An HTTP client is created per call (with 120 s timeout)
/// to avoid holding long-lived connections across await points.
pub struct WorkflowLLMService;

impl WorkflowLLMService {
    // ── Message constructors ────────────────────────────────────────────

    pub fn system_message(content: &str) -> Value {
        json!({ "role": "system", "content": content })
    }

    pub fn user_message(content: &str) -> Value {
        json!({ "role": "user", "content": content })
    }

    pub fn assistant_message(content: &str) -> Value {
        json!({ "role": "assistant", "content": content })
    }

    /// Build an assistant message that contains tool calls (for continuing a
    /// multi-turn tool conversation).
    pub fn assistant_tool_calls_message(calls: &[ToolCallRequest]) -> Value {
        let tool_calls: Vec<Value> = calls
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "type": "function",
                    "function": {
                        "name": c.name,
                        "arguments": serde_json::to_string(&c.arguments).unwrap_or_default(),
                    }
                })
            })
            .collect();

        json!({
            "role": "assistant",
            "tool_calls": tool_calls,
        })
    }

    /// Build a tool-result message to feed back into the conversation.
    pub fn tool_result_message(tool_call_id: &str, content: &str) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": content,
        })
    }

    // ── Simple text completion ──────────────────────────────────────────

    /// Drop-in replacement for `pcg_router::route_completion()` that returns a
    /// plain `String` instead of raw `Value`.
    ///
    /// Iterates candidates in priority order with automatic fallback.
    pub async fn completion(
        pool: &SqlitePool,
        messages: Vec<Value>,
        model_hint: Option<&str>,
        max_tokens: Option<i64>,
        temperature: Option<f64>,
    ) -> anyhow::Result<(String, RoutingMetadata)> {
        let (response, meta) =
            Self::completion_with_tools(pool, messages, &[], model_hint, max_tokens, temperature)
                .await?;

        let text = match response {
            LLMResponse::Text { content, .. } => content,
            LLMResponse::ToolCalls { .. } => {
                anyhow::bail!("Expected text response but got tool calls");
            }
        };

        Ok((text, meta))
    }

    // ── Tool-calling completion ─────────────────────────────────────────

    /// Send a completion request with tool definitions.
    ///
    /// Returns `LLMResponse::ToolCalls` when the model invokes one or more
    /// tools, or `LLMResponse::Text` for a plain text reply.  Iterates
    /// candidates in priority order with automatic fallback on provider errors.
    pub async fn completion_with_tools(
        pool: &SqlitePool,
        messages: Vec<Value>,
        tools: &[ToolDefinition],
        model_hint: Option<&str>,
        max_tokens: Option<i64>,
        temperature: Option<f64>,
    ) -> anyhow::Result<(LLMResponse, RoutingMetadata)> {
        let needs_tools = !tools.is_empty();
        let candidates = Self::resolve_candidates(pool, model_hint, needs_tools).await?;

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        for model in &candidates {
            let Some(api_key) = model.resolve_api_key() else {
                tracing::warn!(
                    "[WORKFLOW_LLM] No API key for model '{}', skipping",
                    model.name
                );
                continue;
            };

            let result = forward_to_provider(
                &http,
                model,
                &messages,
                tools,
                &api_key,
                max_tokens,
                temperature,
            )
            .await;

            match result {
                Ok((response, usage)) => {
                    tracing::info!(
                        "[WORKFLOW_LLM] Routed to '{}' ({}) — {:?}",
                        model.name,
                        model.provider,
                        match &response {
                            LLMResponse::Text { .. } => "text",
                            LLMResponse::ToolCalls { .. } => "tool_calls",
                        }
                    );

                    let cost = calculate_cost(model, &usage);
                    let metadata = RoutingMetadata {
                        model_used: model.model_id.clone(),
                        provider: model.provider.clone(),
                        input_tokens: usage.input_tokens,
                        output_tokens: usage.output_tokens,
                        estimated_cost_micros: cost,
                    };

                    return Ok((response, metadata));
                }
                Err(e) => {
                    tracing::warn!(
                        "[WORKFLOW_LLM] '{}' failed: {}, trying next model",
                        model.name,
                        e
                    );
                }
            }
        }

        anyhow::bail!("All PCG Router models failed for workflow LLM request")
    }

    // ── Structured output extraction ────────────────────────────────────

    /// Extract structured JSON output by wrapping the desired schema as a
    /// single tool and parsing the tool-call arguments.
    ///
    /// The model is instructed to call a tool named `extract_data` whose
    /// parameters match the provided `schema`.  The extracted `Value` is
    /// the parsed arguments of that tool call.
    pub async fn completion_with_structured_output(
        pool: &SqlitePool,
        messages: Vec<Value>,
        schema: Value,
        model_hint: Option<&str>,
    ) -> anyhow::Result<(Value, RoutingMetadata)> {
        let tool = ToolDefinition {
            name: "extract_data".to_string(),
            description: "Extract the requested structured data from the conversation.".to_string(),
            parameters: schema,
        };

        let (response, meta) =
            Self::completion_with_tools(pool, messages, &[tool], model_hint, None, None).await?;

        match response {
            LLMResponse::ToolCalls { calls, .. } => {
                let call = calls
                    .into_iter()
                    .find(|c| c.name == "extract_data")
                    .ok_or_else(|| {
                        anyhow::anyhow!("Model called unexpected tool instead of extract_data")
                    })?;
                Ok((call.arguments, meta))
            }
            LLMResponse::Text { content, .. } => {
                // Some models may respond with JSON text instead of a tool call.
                // Try to parse it as JSON; bail if it's not valid.
                let parsed: Value = serde_json::from_str(&content).map_err(|_| {
                    let preview: String = content.chars().take(200).collect();
                    anyhow::anyhow!(
                        "Expected structured tool call but got plain text: {}",
                        preview
                    )
                })?;
                Ok((parsed, meta))
            }
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Build the candidate list (shared by completion methods).
    async fn resolve_candidates(
        pool: &SqlitePool,
        model_hint: Option<&str>,
        needs_tools: bool,
    ) -> anyhow::Result<Vec<PcgRouterModel>> {
        let mut candidates = if let Some(hint) = model_hint {
            match PcgRouterModel::get_by_model_id(pool, hint).await? {
                Some(m) => vec![m],
                None => {
                    tracing::warn!(
                        "[WORKFLOW_LLM] Requested model '{}' not found, falling back to priority list",
                        hint
                    );
                    PcgRouterModel::list_enabled(pool).await?
                }
            }
        } else {
            PcgRouterModel::list_enabled(pool).await?
        };

        if needs_tools {
            candidates.retain(|m| m.supports_tools);
        }

        if candidates.is_empty() {
            if needs_tools {
                anyhow::bail!("No enabled models with tool support in PCG Router");
            } else {
                anyhow::bail!("No enabled models in PCG Router");
            }
        }

        Ok(candidates)
    }
}

// ── Provider dispatch ───────────────────────────────────────────────────────

#[allow(clippy::type_complexity)] // TODO: extract type alias for pinned boxed future
fn forward_to_provider<'a>(
    http: &'a reqwest::Client,
    model: &'a PcgRouterModel,
    messages: &'a [Value],
    tools: &'a [ToolDefinition],
    api_key: &'a str,
    max_tokens: Option<i64>,
    temperature: Option<f64>,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = anyhow::Result<(LLMResponse, TokenUsage)>> + Send + 'a>,
> {
    Box::pin(async move {
        match model.provider.as_str() {
            "anthropic" => {
                forward_to_anthropic(
                    http,
                    model,
                    messages,
                    tools,
                    api_key,
                    max_tokens,
                    temperature,
                )
                .await
            }
            "openai" | "openrouter" | "mistral" | "xai" | "deepseek" | "groq" | "cohere"
            | "qwen" => {
                forward_openai_compat(
                    http,
                    model,
                    messages,
                    tools,
                    api_key,
                    max_tokens,
                    temperature,
                )
                .await
            }
            "gemini" => {
                forward_to_gemini(
                    http,
                    model,
                    messages,
                    tools,
                    api_key,
                    max_tokens,
                    temperature,
                )
                .await
            }
            other => anyhow::bail!("Unknown provider: {}", other),
        }
    })
}

// ── Anthropic ───────────────────────────────────────────────────────────────

async fn forward_to_anthropic(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    messages: &[Value],
    tools: &[ToolDefinition],
    api_key: &str,
    max_tokens: Option<i64>,
    temperature: Option<f64>,
) -> anyhow::Result<(LLMResponse, TokenUsage)> {
    let mut system_parts: Vec<String> = Vec::new();
    let mut anthropic_messages: Vec<Value> = Vec::new();

    for msg in messages {
        let role = msg["role"].as_str().unwrap_or("user");

        match role {
            "system" => {
                let text = match &msg["content"] {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                system_parts.push(text);
            }
            "assistant" => {
                // Check if this is a tool-calls message
                if let Some(tool_calls) = msg["tool_calls"].as_array() {
                    // Convert OpenAI-format tool_calls to Anthropic content blocks
                    let mut content_blocks: Vec<Value> = Vec::new();

                    // Include any text content if present
                    if let Some(text) = msg["content"].as_str() {
                        if !text.is_empty() {
                            content_blocks.push(json!({ "type": "text", "text": text }));
                        }
                    }

                    for tc in tool_calls {
                        let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                        let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                        content_blocks.push(json!({
                            "type": "tool_use",
                            "id": tc["id"],
                            "name": tc["function"]["name"],
                            "input": args,
                        }));
                    }

                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": content_blocks,
                    }));
                } else {
                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": msg["content"],
                    }));
                }
            }
            "tool" => {
                // Convert OpenAI tool-result to Anthropic tool_result content block.
                // Anthropic expects tool results inside a "user" message.
                let tool_call_id = msg["tool_call_id"].as_str().unwrap_or("unknown");
                let content = match &msg["content"] {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };

                // Check if the last message is already a user message with tool_result
                // blocks — if so, append to it (Anthropic requires consecutive tool
                // results in a single user turn).
                let should_merge = anthropic_messages
                    .last()
                    .and_then(|m| {
                        if m["role"].as_str() == Some("user") {
                            m["content"].as_array().and_then(|arr| {
                                arr.first()
                                    .and_then(|b| b["type"].as_str())
                                    .filter(|t| *t == "tool_result")
                            })
                        } else {
                            None
                        }
                    })
                    .is_some();

                let block = json!({
                    "type": "tool_result",
                    "tool_use_id": tool_call_id,
                    "content": content,
                });

                if should_merge {
                    if let Some(last) = anthropic_messages.last_mut() {
                        if let Some(arr) = last["content"].as_array_mut() {
                            arr.push(block);
                        }
                    }
                } else {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": [block],
                    }));
                }
            }
            _ => {
                // "user" and anything else — pass through
                anthropic_messages.push(json!({
                    "role": role,
                    "content": msg["content"],
                }));
            }
        }
    }

    let mut payload = json!({
        "model": model.model_id,
        "max_tokens": max_tokens.unwrap_or(model.max_output_tokens.unwrap_or(4096)),
        "messages": anthropic_messages,
    });

    if !system_parts.is_empty() {
        payload["system"] = json!(system_parts.join("\n\n"));
    }
    if let Some(temp) = temperature {
        payload["temperature"] = json!(temp);
    }

    // Add tools in Anthropic format
    if !tools.is_empty() {
        let anthropic_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect();
        payload["tools"] = json!(anthropic_tools);
        payload["tool_choice"] = json!({ "type": "auto" });
    }

    let url = format!("{}/v1/messages", model.base_url());
    let resp = http
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic {}: {}", status, text);
    }

    let body: Value = resp.json().await?;

    let usage = TokenUsage {
        input_tokens: body["usage"]["input_tokens"].as_i64(),
        output_tokens: body["usage"]["output_tokens"].as_i64(),
    };

    // Check for tool_use in content blocks
    let stop_reason = body["stop_reason"].as_str().unwrap_or("");
    let content_blocks = body["content"].as_array();

    if stop_reason == "tool_use" {
        if let Some(blocks) = content_blocks {
            let calls: Vec<ToolCallRequest> = blocks
                .iter()
                .filter(|b| b["type"].as_str() == Some("tool_use"))
                .map(|b| ToolCallRequest {
                    id: b["id"].as_str().unwrap_or("").to_string(),
                    name: b["name"].as_str().unwrap_or("").to_string(),
                    arguments: b["input"].clone(),
                })
                .collect();

            if !calls.is_empty() {
                return Ok((
                    LLMResponse::ToolCalls {
                        calls,
                        usage: usage.clone(),
                    },
                    usage,
                ));
            }
        }
    }

    // Extract text content
    let text = content_blocks
        .and_then(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"].as_str() == Some("text"))
                .map(|b| b["text"].as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .first()
                .copied()
                .map(String::from)
        })
        .unwrap_or_default();

    Ok((
        LLMResponse::Text {
            content: text,
            usage: usage.clone(),
        },
        usage,
    ))
}

// ── OpenAI-compatible ───────────────────────────────────────────────────────

async fn forward_openai_compat(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    messages: &[Value],
    tools: &[ToolDefinition],
    api_key: &str,
    max_tokens: Option<i64>,
    temperature: Option<f64>,
) -> anyhow::Result<(LLMResponse, TokenUsage)> {
    let mut payload = json!({
        "model": model.model_id,
        "messages": messages,
        "max_tokens": max_tokens.unwrap_or(model.max_output_tokens.unwrap_or(4096)),
        "stream": false,
    });

    if let Some(temp) = temperature {
        payload["temperature"] = json!(temp);
    }

    if !tools.is_empty() {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        payload["tools"] = json!(openai_tools);
        payload["tool_choice"] = json!("auto");
    }

    let url = format!("{}/v1/chat/completions", model.base_url());
    let resp = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("{} {}: {}", model.provider, status, text);
    }

    let body: Value = resp.json().await?;
    parse_openai_response(&body)
}

// ── Gemini (OpenAI-compat with different URL) ───────────────────────────────

async fn forward_to_gemini(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    messages: &[Value],
    tools: &[ToolDefinition],
    api_key: &str,
    max_tokens: Option<i64>,
    temperature: Option<f64>,
) -> anyhow::Result<(LLMResponse, TokenUsage)> {
    let mut payload = json!({
        "model": model.model_id,
        "messages": messages,
        "max_tokens": max_tokens.unwrap_or(model.max_output_tokens.unwrap_or(4096)),
        "stream": false,
    });

    if let Some(temp) = temperature {
        payload["temperature"] = json!(temp);
    }

    if !tools.is_empty() {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        payload["tools"] = json!(openai_tools);
        payload["tool_choice"] = json!("auto");
    }

    // Gemini's OpenAI-compat layer: no `/v1` prefix — version is in the base URL.
    let url = format!("{}/chat/completions", model.base_url());
    let resp = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini {}: {}", status, text);
    }

    let body: Value = resp.json().await?;
    parse_openai_response(&body)
}

// ── Shared OpenAI response parser ───────────────────────────────────────────

/// Parse an OpenAI-format response body into `LLMResponse` + `TokenUsage`.
fn parse_openai_response(body: &Value) -> anyhow::Result<(LLMResponse, TokenUsage)> {
    let usage = TokenUsage {
        input_tokens: body["usage"]["prompt_tokens"].as_i64(),
        output_tokens: body["usage"]["completion_tokens"].as_i64(),
    };

    let message = &body["choices"][0]["message"];

    // Check for tool calls
    if let Some(tool_calls) = message["tool_calls"].as_array() {
        if !tool_calls.is_empty() {
            let calls: Vec<ToolCallRequest> = tool_calls
                .iter()
                .filter_map(|tc| {
                    let id = tc["id"].as_str()?.to_string();
                    let name = tc["function"]["name"].as_str()?.to_string();
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let arguments: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                    Some(ToolCallRequest {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect();

            if !calls.is_empty() {
                return Ok((
                    LLMResponse::ToolCalls {
                        calls,
                        usage: usage.clone(),
                    },
                    usage,
                ));
            }
        }
    }

    // Plain text response
    let content = message["content"].as_str().unwrap_or("").to_string();

    Ok((
        LLMResponse::Text {
            content,
            usage: usage.clone(),
        },
        usage,
    ))
}

// ── Cost calculation ────────────────────────────────────────────────────────

fn calculate_cost(model: &PcgRouterModel, usage: &TokenUsage) -> Option<i64> {
    match (usage.input_tokens, usage.output_tokens) {
        (Some(inp), Some(out)) => {
            let input_cost = (inp as f64 / 1_000_000.0) * model.cost_per_million_input as f64;
            let output_cost = (out as f64 / 1_000_000.0) * model.cost_per_million_output as f64;
            Some(((input_cost + output_cost) * 1_000_000.0) as i64)
        }
        _ => None,
    }
}
