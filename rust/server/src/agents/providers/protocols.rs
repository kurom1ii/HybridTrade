use std::{
    collections::{BTreeMap, HashSet},
    fmt,
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::warn;

use crate::agents::models::ChatStreamEvent;
use crate::config::ProviderConfig;

use super::{
    super::models::ChatTurn, super::tool_runtime::runtime::ToolRuntime, hub::ProviderKind,
};

const DEFAULT_PROVIDER_HTTP_LOG_PATH: &str = "./logs/provider-http-requests.json";

#[derive(Debug)]
pub(super) struct ContextLimitExceeded {
    pub provider: String,
    pub detail: String,
}

impl fmt::Display for ContextLimitExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "context limit exceeded for provider {}: {}",
            self.provider, self.detail
        )
    }
}

impl std::error::Error for ContextLimitExceeded {}

static PROVIDER_HTTP_LOG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone)]
struct OpenAiFunctionCall {
    name: String,
    call_id: String,
    arguments: String,
}

#[derive(Debug, Clone)]
struct AnthropicToolUse {
    id: String,
    name: String,
    input: Value,
}

// SSE streaming deserialization types (Anthropic only)

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicSseEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicSseDelta>,
    #[serde(default)]
    content_block: Option<AnthropicSseContentBlock>,
    #[serde(default)]
    message: Option<AnthropicSseMessage>,
    #[serde(default)]
    usage: Option<AnthropicSseUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicSseDelta {
    #[serde(rename = "type", default)]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicSseContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicSseMessage {
    #[serde(default)]
    usage: Option<AnthropicSseUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicSseUsage {
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
}

struct AnthropicStreamResult {
    text: String,
    content_blocks: Vec<Value>,
    tool_calls: Vec<AnthropicToolUse>,
    stop_reason: String,
}

pub(super) async fn call_provider(
    client: &Client,
    provider: ProviderKind,
    config: &ProviderConfig,
    api_key: Option<&str>,
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    tool_runtime: &mut ToolRuntime,
    stream_tx: Option<&mpsc::UnboundedSender<ChatStreamEvent>>,
) -> Result<String> {
    match provider {
        ProviderKind::OpenAi => {
            call_openai(
                client,
                config,
                api_key,
                system_prompt,
                history,
                message,
                max_tokens,
                temperature,
                tool_runtime,
            )
            .await
        }
        ProviderKind::Anthropic => {
            call_anthropic(
                client,
                config,
                api_key,
                system_prompt,
                history,
                message,
                max_tokens,
                tool_runtime,
                stream_tx,
            )
            .await
        }
    }
}

async fn call_openai(
    client: &Client,
    config: &ProviderConfig,
    api_key: Option<&str>,
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    tool_runtime: &mut ToolRuntime,
) -> Result<String> {
    let mut input = vec![openai_response_input("system", system_prompt)];
    input.extend(
        history
            .iter()
            .map(|turn| openai_response_input(&turn.role, &turn.content)),
    );
    input.push(openai_response_input("user", message));

    let tools_value: Option<Value> = {
        let tools = openai_tool_specs(tool_runtime);
        if tools.is_empty() {
            None
        } else {
            Some(Value::Array(tools))
        }
    };

    loop {
        let mut payload = json!({
            "model": config.model,
            "input": input.clone(),
            "max_output_tokens": max_tokens.unwrap_or(config.max_tokens),
            "temperature": temperature.unwrap_or(config.temperature),
        });

        if let Some(tools) = &tools_value {
            payload["tools"] = tools.clone();
            payload["tool_choice"] = json!("auto");
        }

        let response = send_openai_request(client, config, api_key, &payload).await?;

        let payload = parse_provider_response(response, "openai").await?;
        let function_calls = extract_openai_function_calls(&payload);
        if function_calls.is_empty() {
            return extract_openai_responses_content(&payload)
                .ok_or_else(|| anyhow!("phản hồi OpenAI Responses API không có nội dung"));
        }

        // Push all function_call entries into input first
        for fc in &function_calls {
            input.push(json!({
                "type": "function_call",
                "name": fc.name,
                "call_id": fc.call_id,
                "arguments": fc.arguments,
            }));
        }

        // Execute tools — concurrent when multiple, sequential when single
        let calls: Vec<(String, Value)> = function_calls
            .iter()
            .map(|fc| (fc.name.clone(), parse_json_arguments(&fc.arguments)))
            .collect();
        let outputs = tool_runtime.execute_concurrent(calls).await;

        // Pair each output with its call_id
        for (fc, output) in function_calls.iter().zip(outputs) {
            input.push(json!({
                "type": "function_call_output",
                "call_id": fc.call_id,
                "output": output,
            }));
        }
    }
}

async fn call_anthropic(
    client: &Client,
    config: &ProviderConfig,
    api_key: Option<&str>,
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
    max_tokens: Option<u32>,
    tool_runtime: &mut ToolRuntime,
    stream_tx: Option<&mpsc::UnboundedSender<ChatStreamEvent>>,
) -> Result<String> {
    let mut messages = history
        .iter()
        .map(anthropic_message_from_turn)
        .collect::<Vec<_>>();
    messages.push(json!({
        "role": "user",
        "content": [{ "type": "text", "text": message }],
    }));

    let tools_value: Option<Value> = {
        let tools = anthropic_tool_specs(tool_runtime);
        if tools.is_empty() {
            None
        } else {
            Some(Value::Array(tools))
        }
    };

    // Agentic loop with dual-model:
    // - First call: main model (Opus) for deep reasoning + tool decisions
    // - Subsequent calls: light model (Sonnet) with thinking for quality tool processing
    let tool_model = if config.light_model.trim().is_empty() {
        config.model.clone()
    } else {
        config.light_model.clone()
    };

    let mut is_first_call = true;

    loop {
        let active_model = if is_first_call {
            &config.model
        } else {
            &tool_model
        };

        let effective_max = max_tokens.unwrap_or(config.max_tokens);
        let mut payload = json!({
            "model": active_model,
            "system": system_prompt,
            "messages": messages.clone(),
            "max_tokens": effective_max,
        });

        // Enable adaptive thinking for Claude models (opus-4.6, sonnet-4.6).
        // Adaptive mode lets the model decide when and how much to think.
        if config.thinking {
            payload["thinking"] = json!({
                "type": "adaptive",
            });
        }

        if let Some(tools) = &tools_value {
            payload["tools"] = tools.clone();
            payload["tool_choice"] = json!({ "type": "auto" });
        }

        is_first_call = false;

        // Streaming path: parse SSE events in real-time
        if let Some(tx) = stream_tx {
            payload["stream"] = json!(true);

            let response = send_anthropic_request(client, config, api_key, &payload).await?;
            let result = stream_anthropic_response(response, tx).await?;

            if result.tool_calls.is_empty() || result.stop_reason != "tool_use" {
                return Ok(result.text);
            }

            // Push assistant message with all content blocks
            messages.push(json!({
                "role": "assistant",
                "content": result.content_blocks,
            }));

            // Execute tools — concurrent when multiple, sequential when single
            let calls: Vec<(String, Value)> = result
                .tool_calls
                .iter()
                .map(|tu| (tu.name.clone(), tu.input.clone()))
                .collect();
            let outputs = tool_runtime.execute_concurrent(calls).await;

            let mut tool_results = Vec::new();
            for (tool_use, output) in result.tool_calls.iter().zip(outputs) {
                tool_results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "content": [{ "type": "text", "text": output }],
                }));
            }
            messages.push(json!({
                "role": "user",
                "content": tool_results,
            }));
            continue;
        }

        // Non-streaming path: existing behavior unchanged
        let response = send_anthropic_request(client, config, api_key, &payload).await?;

        let payload = parse_provider_response(response, "anthropic").await?;
        let tool_uses = extract_anthropic_tool_uses(&payload);
        if tool_uses.is_empty() {
            return extract_anthropic_content(&payload)
                .ok_or_else(|| anyhow!("phản hồi Anthropic không có nội dung assistant"));
        }

        messages.push(json!({
            "role": "assistant",
            "content": payload
                .get("content")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        }));

        // Execute tools — concurrent when multiple, sequential when single
        let calls: Vec<(String, Value)> = tool_uses
            .iter()
            .map(|tu| (tu.name.clone(), tu.input.clone()))
            .collect();
        let outputs = tool_runtime.execute_concurrent(calls).await;

        let mut tool_results = Vec::new();
        for (tool_use, output) in tool_uses.iter().zip(outputs) {
            tool_results.push(json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "content": [{ "type": "text", "text": output }],
            }));
        }
        messages.push(json!({
            "role": "user",
            "content": tool_results,
        }));
    }
}

fn openai_tool_specs(tool_runtime: &ToolRuntime) -> Vec<Value> {
    tool_runtime
        .definitions()
        .into_iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": normalize_openai_tool_schema(&tool.input_schema),
                "strict": true,
            })
        })
        .collect()
}

fn anthropic_tool_specs(tool_runtime: &ToolRuntime) -> Vec<Value> {
    tool_runtime
        .definitions()
        .into_iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema,
            })
        })
        .collect()
}

fn parse_json_arguments(arguments: &str) -> Value {
    serde_json::from_str(arguments).unwrap_or_else(|_| json!({ "raw": arguments }))
}

fn normalize_openai_tool_schema(schema: &Value) -> Value {
    match schema {
        Value::Object(object) => {
            let mut normalized = object.clone();

            if let Some(items) = normalized.get("items").cloned() {
                normalized.insert("items".to_string(), normalize_openai_tool_schema(&items));
            }

            if normalized
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "object")
            {
                let originally_required = normalized
                    .get("required")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<HashSet<_>>();

                if let Some(properties) = normalized.get("properties").and_then(Value::as_object) {
                    let mut rewritten = serde_json::Map::new();
                    let mut required = Vec::new();

                    for (key, property) in properties {
                        let mut property = normalize_openai_tool_schema(property);
                        if !originally_required.contains(key) {
                            property = make_schema_nullable(property);
                        }
                        rewritten.insert(key.clone(), property);
                        required.push(Value::String(key.clone()));
                    }

                    normalized.insert("properties".to_string(), Value::Object(rewritten));
                    normalized.insert("required".to_string(), Value::Array(required));
                    normalized
                        .entry("additionalProperties".to_string())
                        .or_insert(Value::Bool(false));
                }
            }

            Value::Object(normalized)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(normalize_openai_tool_schema)
                .collect::<Vec<_>>(),
        ),
        _ => schema.clone(),
    }
}

fn make_schema_nullable(schema: Value) -> Value {
    let mut schema = match schema {
        Value::Object(object) => object,
        other => return other,
    };

    match schema.get("type").cloned() {
        Some(Value::String(kind)) if kind != "null" => {
            schema.insert(
                "type".to_string(),
                Value::Array(vec![Value::String(kind), Value::String("null".to_string())]),
            );
            Value::Object(schema)
        }
        Some(Value::Array(mut kinds)) => {
            if !kinds.iter().any(|kind| kind.as_str() == Some("null")) {
                kinds.push(Value::String("null".to_string()));
            }
            schema.insert("type".to_string(), Value::Array(kinds));
            Value::Object(schema)
        }
        _ => Value::Object(schema),
    }
}

fn extract_openai_function_calls(payload: &Value) -> Vec<OpenAiFunctionCall> {
    payload
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let call_id = item.get("call_id")?.as_str()?.to_string();
            let arguments = item
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}")
                .to_string();
            Some(OpenAiFunctionCall {
                name,
                call_id,
                arguments,
            })
        })
        .collect()
}

fn anthropic_message_from_turn(turn: &ChatTurn) -> Value {
    json!({
        "role": turn.role,
        "content": [{ "type": "text", "text": turn.content }],
    })
}

fn extract_anthropic_tool_uses(payload: &Value) -> Vec<AnthropicToolUse> {
    payload
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("tool_use"))
        .filter_map(|item| {
            Some(AnthropicToolUse {
                id: item.get("id")?.as_str()?.to_string(),
                name: item.get("name")?.as_str()?.to_string(),
                input: item.get("input").cloned().unwrap_or_else(|| json!({})),
            })
        })
        .collect()
}

async fn stream_anthropic_response(
    response: reqwest::Response,
    tx: &mpsc::UnboundedSender<ChatStreamEvent>,
) -> Result<AnthropicStreamResult> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    // Per-block accumulation state
    let mut cur_block_type = String::new();
    let mut cur_text = String::new();
    let mut cur_thinking = String::new();
    let mut cur_tool_id = String::new();
    let mut cur_tool_name = String::new();
    let mut cur_tool_json = String::new();

    // Accumulated result
    let mut content_blocks: Vec<Value> = Vec::new();
    let mut tool_calls: Vec<AnthropicToolUse> = Vec::new();
    let mut stop_reason = String::new();
    let mut all_text = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("lỗi đọc SSE stream từ provider")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim_end().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            let data = match line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) {
                Some(d) => d.trim(),
                None => continue,
            };

            if data == "[DONE]" {
                continue;
            }

            let event: AnthropicSseEvent = match serde_json::from_str(data) {
                Ok(e) => e,
                Err(_) => continue,
            };

            match event.event_type.as_str() {
                "message_start" => {
                    // Nothing to emit — AgentThinking already sent by hub.rs
                }
                "content_block_start" => {
                    if let Some(block) = &event.content_block {
                        cur_block_type = block.block_type.clone();
                        match cur_block_type.as_str() {
                            "thinking" => {
                                cur_thinking.clear();
                                let _ = tx.send(ChatStreamEvent::ThinkingStart);
                            }
                            "text" => {
                                cur_text.clear();
                                let _ = tx.send(ChatStreamEvent::TextStart);
                            }
                            "tool_use" => {
                                cur_tool_id = block.id.clone().unwrap_or_default();
                                cur_tool_name = block.name.clone().unwrap_or_default();
                                cur_tool_json.clear();
                            }
                            _ => {}
                        }
                    }
                }
                "content_block_delta" => {
                    if let Some(delta) = &event.delta {
                        match cur_block_type.as_str() {
                            "thinking" => {
                                if let Some(t) = &delta.thinking {
                                    cur_thinking.push_str(t);
                                    let _ = tx.send(ChatStreamEvent::ThinkingDelta {
                                        text: t.clone(),
                                    });
                                }
                            }
                            "text" => {
                                if let Some(t) = &delta.text {
                                    cur_text.push_str(t);
                                    let _ = tx.send(ChatStreamEvent::TextDelta {
                                        text: t.clone(),
                                    });
                                }
                            }
                            "tool_use" => {
                                if let Some(pj) = &delta.partial_json {
                                    cur_tool_json.push_str(pj);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "content_block_stop" => {
                    match cur_block_type.as_str() {
                        "thinking" => {
                            content_blocks.push(json!({
                                "type": "thinking",
                                "thinking": cur_thinking,
                            }));
                        }
                        "text" => {
                            all_text.push_str(&cur_text);
                            content_blocks.push(json!({
                                "type": "text",
                                "text": cur_text,
                            }));
                            cur_text = String::new();
                        }
                        "tool_use" => {
                            let input: Value = serde_json::from_str(&cur_tool_json)
                                .unwrap_or_else(|_| json!({}));
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": cur_tool_id,
                                "name": cur_tool_name,
                                "input": input,
                            }));
                            tool_calls.push(AnthropicToolUse {
                                id: cur_tool_id.clone(),
                                name: cur_tool_name.clone(),
                                input,
                            });
                        }
                        _ => {}
                    }
                    cur_block_type.clear();
                }
                "message_delta" => {
                    if let Some(delta) = &event.delta {
                        if let Some(sr) = &delta.stop_reason {
                            stop_reason = sr.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(AnthropicStreamResult {
        text: all_text,
        content_blocks,
        tool_calls,
        stop_reason,
    })
}

async fn send_anthropic_request(
    client: &Client,
    config: &ProviderConfig,
    api_key: Option<&str>,
    payload: &Value,
) -> Result<reqwest::Response> {
    let base_url = config.base_url.trim_end_matches('/');
    let primary_url = if base_url.ends_with("/v1") {
        format!("{}/messages", base_url)
    } else {
        format!("{}/v1/messages", base_url)
    };
    let fallback_base = base_url.trim_end_matches("/v1");
    let fallback_url = (fallback_base != base_url
        || primary_url != format!("{}/messages", fallback_base))
    .then(|| format!("{}/messages", fallback_base));

    send_provider_request_with_retry(
        client,
        "anthropic",
        config,
        api_key,
        payload,
        primary_url,
        fallback_url,
        anthropic_request_builder,
    )
    .await
}

async fn send_openai_request(
    client: &Client,
    config: &ProviderConfig,
    api_key: Option<&str>,
    payload: &Value,
) -> Result<reqwest::Response> {
    let base_url = config.base_url.trim_end_matches('/');
    let primary_url = if base_url.ends_with("/v1") {
        format!("{}/responses", base_url)
    } else {
        format!("{}/v1/responses", base_url)
    };
    let fallback_base = base_url.trim_end_matches("/v1");
    let fallback_url = (fallback_base != base_url
        || primary_url != format!("{}/responses", fallback_base))
    .then(|| format!("{}/responses", fallback_base));

    send_provider_request_with_retry(
        client,
        "openai",
        config,
        api_key,
        payload,
        primary_url,
        fallback_url,
        openai_request_builder,
    )
    .await
}

async fn send_provider_request_with_retry(
    client: &Client,
    provider: &str,
    config: &ProviderConfig,
    api_key: Option<&str>,
    payload: &Value,
    primary_url: String,
    fallback_url: Option<String>,
    build_request: ProviderRequestBuilder,
) -> Result<reqwest::Response> {
    let max_retries = config.request_retries;
    let mut current_url = primary_url;
    let mut current_path_label = "primary";
    let mut used_fallback = false;
    let mut attempt = 0usize;

    loop {
        let request = build_request(client, current_url.clone(), api_key, payload)
            .build()
            .with_context(|| format!("không thể build request cho provider {provider}"))?;

        log_provider_http_request(&request, payload);

        let response = client.execute(request).await;

        match response {
            Ok(response) => {
                let status = response.status();

                if status == reqwest::StatusCode::NOT_FOUND {
                    if let Some(fallback_url) = fallback_url.as_ref().filter(|_| !used_fallback) {
                        used_fallback = true;
                        current_url = fallback_url.clone();
                        current_path_label = "fallback";
                        attempt = 0;
                        continue;
                    }
                }

                if status.is_success() {
                    return Ok(response);
                }

                let body = response.text().await.unwrap_or_default();
                let body = body.trim();
                let error_text = if body.is_empty() {
                    format!("provider {provider} trả về {status}")
                } else {
                    format!("provider {provider} trả về {status}: {body}")
                };

                if should_retry_status(status) && attempt < max_retries {
                    let delay = retry_delay(config, attempt);
                    warn!(
                        provider,
                        status = %status,
                        attempt = attempt + 1,
                        max_retries,
                        path = current_path_label,
                        url = %current_url,
                        delay_ms = delay.as_millis(),
                        "provider request failed with retryable status, retrying"
                    );
                    sleep(delay).await;
                    attempt += 1;
                    continue;
                }

                // Detect context limit errors on 400 Bad Request
                if status == reqwest::StatusCode::BAD_REQUEST && detect_context_limit(body) {
                    return Err(ContextLimitExceeded {
                        provider: provider.to_string(),
                        detail: body.to_string(),
                    }
                    .into());
                }

                bail!(error_text);
            }
            Err(error) => {
                if should_retry_transport_error(&error) && attempt < max_retries {
                    let delay = retry_delay(config, attempt);
                    warn!(
                        provider,
                        attempt = attempt + 1,
                        max_retries,
                        path = current_path_label,
                        url = %current_url,
                        delay_ms = delay.as_millis(),
                        error = %error,
                        "provider request transport error, retrying"
                    );
                    sleep(delay).await;
                    attempt += 1;
                    continue;
                }

                return Err(error).with_context(|| format!("gọi {provider} thất bại"));
            }
        }
    }
}

fn resolve_provider_http_log_path() -> PathBuf {
    std::env::var("HYBRIDTRADE_PROVIDER_HTTP_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_PROVIDER_HTTP_LOG_PATH))
}

pub(super) fn reset_provider_http_log_file() -> Result<()> {
    let path = resolve_provider_http_log_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create provider http log directory {:?}", parent))?;
    }

    fs::write(&path, "[]\n")
        .with_context(|| format!("cannot reset provider http log file {:?}", path))?;

    Ok(())
}

fn with_provider_http_log<T>(action: impl FnOnce(&PathBuf) -> Result<T>) -> Result<T> {
    let lock = PROVIDER_HTTP_LOG_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .map_err(|_| anyhow!("provider http log mutex bị poisoned"))?;

    let path = resolve_provider_http_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create provider http log directory {:?}", parent))?;
    }

    action(&path)
}

fn log_provider_http_request(request: &reqwest::Request, payload: &Value) {
    if let Err(error) = append_provider_http_request_log(request, payload) {
        let log_path = resolve_provider_http_log_path();
        warn!(
            error = %error,
            path = %log_path.display(),
            url = %request.url(),
            "không thể ghi provider HTTP request ra file log"
        );
    }
}

fn append_provider_http_request_log(request: &reqwest::Request, payload: &Value) -> Result<()> {
    let headers = render_request_headers(request.headers());
    let body = render_request_body(request, payload);
    let entry = json!({
        "headers": headers,
        "body": body,
    });

    with_provider_http_log(|path| {
        let existing = fs::read_to_string(path).unwrap_or_default();
        let mut entries = serde_json::from_str::<Vec<Value>>(&existing).unwrap_or_default();
        entries.push(entry);

        let rendered = serde_json::to_string_pretty(&entries)?;
        fs::write(path, format!("{rendered}\n"))?;
        Ok(())
    })
}

fn render_request_headers(headers: &reqwest::header::HeaderMap) -> Value {
    let mut grouped = BTreeMap::<String, Vec<String>>::new();

    for (name, value) in headers.iter() {
        let value = value
            .to_str()
            .map(str::to_string)
            .unwrap_or_else(|_| format!("<non-utf8:{} bytes>", value.as_bytes().len()));
        grouped
            .entry(name.as_str().to_string())
            .or_default()
            .push(value);
    }

    let headers = grouped
        .into_iter()
        .map(|(name, values)| {
            let value = values.into_iter().map(Value::String).collect::<Vec<_>>();

            let value = match value.as_slice() {
                [single] => single.clone(),
                _ => Value::Array(value),
            };

            (name, value)
        })
        .collect::<serde_json::Map<String, Value>>();

    Value::Object(headers)
}

fn render_request_body(request: &reqwest::Request, payload: &Value) -> Value {
    if let Some(bytes) = request.body().and_then(|body| body.as_bytes()) {
        if let Ok(json) = serde_json::from_slice::<Value>(bytes) {
            return json;
        }

        return String::from_utf8(bytes.to_vec())
            .map(Value::String)
            .unwrap_or_else(|_| Value::String(format!("<non-utf8-body:{} bytes>", bytes.len())));
    }

    payload.clone()
}

fn openai_request_builder<'a>(
    client: &'a Client,
    url: String,
    api_key: Option<&'a str>,
    payload: &'a Value,
) -> reqwest::RequestBuilder {
    let request = client.post(url).json(payload);

    if let Some(api_key) = api_key {
        request.bearer_auth(api_key)
    } else {
        request
    }
}

fn anthropic_request_builder<'a>(
    client: &'a Client,
    url: String,
    api_key: Option<&'a str>,
    payload: &'a Value,
) -> reqwest::RequestBuilder {
    let request = client
        .post(url)
        .header("anthropic-version", "2023-06-01")
        .json(payload);

    if let Some(api_key) = api_key {
        request.header("x-api-key", api_key)
    } else {
        request
    }
}

async fn parse_provider_response(response: reqwest::Response, provider: &str) -> Result<Value> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!("provider {} trả về {}: {}", provider, status, text);
    }
    serde_json::from_str(&text).with_context(|| format!("phản hồi {provider} không hợp lệ"))
}

type ProviderRequestBuilder =
    for<'a> fn(&'a Client, String, Option<&'a str>, &'a Value) -> reqwest::RequestBuilder;

fn should_retry_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn should_retry_transport_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn retry_delay(config: &ProviderConfig, attempt: usize) -> Duration {
    let base_ms = config.retry_backoff_ms.max(100);
    let exponent = attempt.min(6) as u32;
    Duration::from_millis(base_ms.saturating_mul(2_u64.saturating_pow(exponent)))
}

fn extract_openai_responses_content(payload: &Value) -> Option<String> {
    if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return Some(text.to_string());
        }
    }

    let text = payload
        .get("output")?
        .as_array()?
        .iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<String>();

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn openai_response_input(role: &str, text: &str) -> Value {
    let content_type = if role.eq_ignore_ascii_case("assistant") {
        "output_text"
    } else {
        "input_text"
    };

    json!({
        "type": "message",
        "role": role,
        "content": [
            {
                "type": content_type,
                "text": text,
            }
        ]
    })
}

fn extract_anthropic_content(payload: &Value) -> Option<String> {
    let items = payload.get("content")?.as_array()?;
    let text = items
        .iter()
        .filter_map(|item| item.get("text"))
        .filter_map(Value::as_str)
        .collect::<String>();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Detect context limit errors from provider error responses.
///
/// Handles both direct provider errors and double-wrapped errors from copilot-api proxy.
fn detect_context_limit(body: &str) -> bool {
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(error) = parsed.get("error") {
        // Check for proxy double-wrapping: error.message is a JSON string containing
        // the actual provider error (copilot-api proxy wraps upstream errors this way)
        if let Some(message_str) = error.get("message").and_then(Value::as_str) {
            if message_str.trim_start().starts_with('{') {
                if let Ok(inner_parsed) = serde_json::from_str::<Value>(message_str) {
                    if let Some(inner_error) = inner_parsed.get("error") {
                        if is_context_limit_error(inner_error) {
                            return true;
                        }
                    }
                    if is_context_limit_error(&inner_parsed) {
                        return true;
                    }
                }
            }
        }

        // Check the direct error object
        if is_context_limit_error(error) {
            return true;
        }
    }

    false
}

fn is_context_limit_error(error: &Value) -> bool {
    // OpenAI: error.code == "context_length_exceeded"
    if error.get("code").and_then(Value::as_str) == Some("context_length_exceeded") {
        return true;
    }

    // Anthropic: error.type == "invalid_request_error" + message about tokens/prompt length
    if error.get("type").and_then(Value::as_str) == Some("invalid_request_error") {
        if let Some(message) = error.get("message").and_then(Value::as_str) {
            let lower = message.to_ascii_lowercase();
            if (lower.contains("tokens") && lower.contains("maximum"))
                || lower.contains("prompt is too long")
            {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use crate::config::ProviderConfig;

    use super::{detect_context_limit, retry_delay, should_retry_status};

    #[test]
    fn retry_delay_grows_exponentially() {
        let config = ProviderConfig {
            retry_backoff_ms: 500,
            ..ProviderConfig::default()
        };

        assert_eq!(retry_delay(&config, 0).as_millis(), 500);
        assert_eq!(retry_delay(&config, 1).as_millis(), 1_000);
        assert_eq!(retry_delay(&config, 2).as_millis(), 2_000);
    }

    #[test]
    fn retries_only_for_transient_statuses() {
        assert!(should_retry_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(should_retry_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(!should_retry_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!should_retry_status(reqwest::StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn detect_openai_context_limit() {
        let body = r#"{"error":{"code":"context_length_exceeded","message":"This model maximum context length is 128000 tokens.","type":"invalid_request_error"}}"#;
        assert!(detect_context_limit(body));
    }

    #[test]
    fn detect_anthropic_context_limit_tokens() {
        let body = r#"{"error":{"type":"invalid_request_error","message":"prompt is too long: 210000 tokens > 200000 maximum"}}"#;
        assert!(detect_context_limit(body));
    }

    #[test]
    fn detect_anthropic_context_limit_prompt_too_long() {
        let body = r#"{"error":{"type":"invalid_request_error","message":"prompt is too long"}}"#;
        assert!(detect_context_limit(body));
    }

    #[test]
    fn detect_proxy_wrapped_context_limit() {
        // copilot-api proxy wraps upstream errors: error.message is a JSON string
        let inner = r#"{"error":{"type":"invalid_request_error","message":"prompt is too long"}}"#;
        let body = format!(
            r#"{{"error":{{"message":"{}","type":"error"}}}}"#,
            inner.replace('"', "\\\"")
        );
        assert!(detect_context_limit(&body));
    }

    #[test]
    fn non_context_400_not_detected() {
        let body = r#"{"error":{"type":"invalid_request_error","message":"Missing required field: model"}}"#;
        assert!(!detect_context_limit(body));
    }

    #[test]
    fn empty_body_not_detected() {
        assert!(!detect_context_limit(""));
    }

    #[test]
    fn non_json_body_not_detected() {
        assert!(!detect_context_limit("Bad Request"));
    }
}
