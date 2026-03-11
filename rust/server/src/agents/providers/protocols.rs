use std::{collections::HashSet, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing::warn;

use crate::config::ProviderConfig;

use super::{
    super::models::ChatTurn, super::tool_runtime::runtime::ToolRuntime, hub::ProviderKind,
};

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

    let tools = openai_tool_specs(tool_runtime);

    for _ in 0..8 {
        let mut payload = json!({
            "model": config.model,
            "input": input.clone(),
            "max_output_tokens": max_tokens.unwrap_or(config.max_tokens),
            "temperature": temperature.unwrap_or(config.temperature),
        });

        if !tools.is_empty() {
            payload["tools"] = Value::Array(tools.clone());
            payload["tool_choice"] = json!("auto");
        }

        let response = send_openai_request(client, config, api_key, &payload).await?;

        let payload = parse_provider_response(response, "openai").await?;
        let function_calls = extract_openai_function_calls(&payload);
        if function_calls.is_empty() {
            return extract_openai_responses_content(&payload)
                .ok_or_else(|| anyhow!("phản hồi OpenAI Responses API không có nội dung"));
        }

        for function_call in function_calls {
            input.push(json!({
                "type": "function_call",
                "name": function_call.name,
                "call_id": function_call.call_id,
                "arguments": function_call.arguments,
            }));

            let output = tool_runtime
                .execute(
                    &function_call.name,
                    parse_json_arguments(&function_call.arguments),
                )
                .await;
            input.push(json!({
                "type": "function_call_output",
                "call_id": function_call.call_id,
                "output": output,
            }));
        }
    }

    bail!("OpenAI tool_calls vượt quá giới hạn vòng lặp cho phép")
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
) -> Result<String> {
    let mut messages = history
        .iter()
        .map(anthropic_message_from_turn)
        .collect::<Vec<_>>();
    messages.push(json!({
        "role": "user",
        "content": [{ "type": "text", "text": message }],
    }));

    let tools = anthropic_tool_specs(tool_runtime);

    for _ in 0..8 {
        let mut payload = json!({
            "model": config.model,
            "system": system_prompt,
            "messages": messages.clone(),
            "max_tokens": max_tokens.unwrap_or(config.max_tokens),
        });

        if !tools.is_empty() {
            payload["tools"] = Value::Array(tools.clone());
            payload["tool_choice"] = json!({ "type": "auto" });
        }

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

        let mut tool_results = Vec::new();
        for tool_use in tool_uses {
            let output = tool_runtime.execute(&tool_use.name, tool_use.input).await;
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

    bail!("Anthropic tool_calls vượt quá giới hạn vòng lặp cho phép")
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
        let response = build_request(client, current_url.clone(), api_key, payload)
            .send()
            .await;

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

#[cfg(test)]
mod tests {
    use crate::config::ProviderConfig;

    use super::{retry_delay, should_retry_status};

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
}
