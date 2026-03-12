use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use mcp_client::{
    ClientCapabilities, ClientInfo, McpClient, McpClientTrait, McpService, StdioTransport,
    Transport,
};
use serde_json::{json, Value};
use tokio::sync::Mutex as AsyncMutex;

use crate::config::McpServerConfig;

use super::runtime::{ToolDefinition, ToolExecutor, ToolRuntime};

#[derive(Debug)]
struct McpToolInfo {
    name: String,
    description: String,
    input_schema: Value,
}

pub(super) struct McpSession {
    server_name: String,
    client: Box<dyn McpClientTrait>,
}

impl ToolRuntime {
    pub(super) async fn load_mcp_server(&mut self, config: McpServerConfig) {
        let server_name = config.name.clone();
        match McpSession::start(&config).await {
            Ok(session) => match session.list_tools().await {
                Ok(tools) => {
                    for tool in tools {
                        let provider_name = format_mcp_tool_name(&server_name, &tool.name);
                        let description = if tool.description.trim().is_empty() {
                            format!("MCP tool `{}` từ server `{}`", tool.name, server_name)
                        } else {
                            format!("[{}] {}", server_name, tool.description.trim())
                        };

                        self.insert_definition(ToolDefinition {
                            name: provider_name,
                            description,
                            input_schema: tool.input_schema,
                            source_label: format!("mcp:{}", server_name),
                            executor: ToolExecutor::Mcp {
                                server_name: server_name.clone(),
                                tool_name: tool.name,
                            },
                        });
                    }

                    self.mcp_sessions.insert(server_name.clone(), Arc::new(AsyncMutex::new(session)));
                }
                Err(error) => {
                    self.initialization_warnings.push(format!(
                        "không thể list tools từ MCP `{}`: {}",
                        server_name, error
                    ));
                }
            },
            Err(error) => {
                self.initialization_warnings.push(format!(
                    "không thể khởi tạo MCP `{}`: {}",
                    server_name, error
                ));
            }
        }
    }

    pub(super) async fn execute_mcp_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        let arc = self
            .mcp_sessions
            .get(server_name)
            .ok_or_else(|| anyhow!("MCP server `{server_name}` chưa sẵn sàng"))?;
        let session = arc.lock().await;
        session.call_tool(tool_name, arguments).await
    }
}

impl McpSession {
    async fn start(config: &McpServerConfig) -> Result<Self> {
        let transport = StdioTransport::new(
            config.command.clone(),
            config.args.clone(),
            HashMap::<String, String>::new(),
        );
        let handle = transport.start().await.with_context(|| {
            format!(
                "không thể spawn tiến trình MCP `{}` bằng `{}`",
                config.name, config.command
            )
        })?;
        let timeout = Duration::from_millis(config.timeout_ms.max(1_000));
        let service = McpService::with_timeout(handle, timeout);
        let mut client = McpClient::new(service);

        client
            .initialize(
                ClientInfo {
                    name: "hybridtrade-backend".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                ClientCapabilities::default(),
            )
            .await
            .with_context(|| format!("không thể initialize MCP `{}`", config.name))?;

        Ok(Self {
            server_name: config.name.clone(),
            client: Box::new(client),
        })
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        let result = self
            .client
            .list_tools(None)
            .await
            .with_context(|| format!("MCP `{}` không trả được tools/list", self.server_name))?;
        let value = serde_json::to_value(&result)
            .with_context(|| format!("không thể serialize tools/list từ `{}`", self.server_name))?;

        Ok(value
            .get("tools")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|tool| {
                let name = tool.get("name")?.as_str()?.trim().to_string();
                if name.is_empty() {
                    return None;
                }

                Some(McpToolInfo {
                    name,
                    description: tool
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    input_schema: tool.get("inputSchema").cloned().unwrap_or_else(|| {
                        json!({
                            "type": "object",
                            "properties": {},
                            "additionalProperties": true,
                        })
                    }),
                })
            })
            .collect())
    }

    async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let result = self
            .client
            .call_tool(tool_name, arguments)
            .await
            .with_context(|| {
                format!(
                    "MCP `{}` không thể gọi tool `{}`",
                    self.server_name, tool_name
                )
            })?;

        serde_json::to_value(&result).with_context(|| {
            format!(
                "không thể serialize kết quả tool `{}` từ MCP `{}`",
                tool_name, self.server_name
            )
        })
    }
}

fn format_mcp_tool_name(server_name: &str, tool_name: &str) -> String {
    format!(
        "{}__{}",
        sanitize_tool_name_segment(server_name),
        sanitize_tool_name_segment(tool_name)
    )
}

fn sanitize_tool_name_segment(value: &str) -> String {
    let mut sanitized = String::new();
    let mut previous_was_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            sanitized.push('_');
            previous_was_separator = true;
        }
    }

    sanitized.trim_matches('_').to_string()
}
