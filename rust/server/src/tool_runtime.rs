use std::{
    collections::{BTreeMap, HashMap},
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use mcp_client::{
    ClientCapabilities, ClientInfo, McpClient, McpClientTrait, McpService, StdioTransport,
    Transport,
};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::{fs, io::AsyncWriteExt, process::Command, time::timeout};

use crate::{
    config::{McpServerConfig, NativeToolConfig},
    models::{ChatTurn, DebugToolCall},
};

const MAX_TOOL_OUTPUT_CHARS: usize = 128000;
const MAX_TOOL_PREVIEW_CHARS: usize = 32000;

pub struct ToolRuntime {
    http_client: Client,
    history: Vec<ChatTurn>,
    context_preview: Option<String>,
    workspace_root: PathBuf,
    tools: BTreeMap<String, ToolDefinition>,
    mcp_sessions: HashMap<String, McpSession>,
    initialization_warnings: Vec<String>,
    tool_calls: Vec<DebugToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub source_label: String,
    executor: ToolExecutor,
}

#[derive(Debug, Clone)]
enum ToolExecutor {
    Native {
        kind: NativeToolKind,
        timeout: Duration,
    },
    Mcp {
        server_name: String,
        tool_name: String,
    },
}

#[derive(Debug, Clone, Copy)]
enum NativeToolKind {
    FetchPage,
    ExtractSignals,
    MemoryLookup,
    SummarizeSources,
    Read,
    Write,
    Exec,
    Bash,
}

#[derive(Debug)]
struct McpToolInfo {
    name: String,
    description: String,
    input_schema: Value,
}

struct McpSession {
    server_name: String,
    client: Box<dyn McpClientTrait>,
}

impl ToolRuntime {
    pub async fn bootstrap(
        mcp_servers: Vec<McpServerConfig>,
        native_tools: Vec<NativeToolConfig>,
        history: Vec<ChatTurn>,
        context_preview: Option<String>,
        http_client: Client,
    ) -> Self {
        let (workspace_root, workspace_warning) = match resolve_workspace_root() {
            Ok(path) => (path, None),
            Err(error) => {
                let fallback = best_effort_canonicalize(
                    env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                );
                let warning = format!(
                    "không thể resolve workspace root cho native tools: {}. Dùng fallback `{}`",
                    error,
                    fallback.display()
                );
                (fallback, Some(warning))
            }
        };

        let mut runtime = Self {
            http_client,
            history,
            context_preview,
            workspace_root,
            tools: BTreeMap::new(),
            mcp_sessions: HashMap::new(),
            initialization_warnings: Vec::new(),
            tool_calls: Vec::new(),
        };

        if let Some(warning) = workspace_warning {
            runtime.initialization_warnings.push(warning);
        }

        for tool in native_tools {
            runtime.register_native_tool(tool);
        }

        for server in mcp_servers {
            runtime.load_mcp_server(server).await;
        }

        runtime
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().cloned().collect()
    }

    pub fn available_tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn initialization_warnings(&self) -> &[String] {
        &self.initialization_warnings
    }

    pub fn tool_calls(&self) -> &[DebugToolCall] {
        &self.tool_calls
    }

    pub fn prepare_turn(&mut self, history: &[ChatTurn], context_preview: Option<String>) {
        self.history = history.to_vec();
        self.context_preview = context_preview;
        self.tool_calls.clear();
    }

    pub async fn execute(&mut self, name: &str, arguments: Value) -> String {
        let arguments = sanitize_tool_arguments(arguments);

        let Some(definition) = self.tools.get(name).cloned() else {
            let output = json!({
                "ok": false,
                "error": format!("tool `{name}` không tồn tại trong runtime hiện tại"),
            });
            let output_text = render_tool_output_for_model(&output);
            self.tool_calls.push(DebugToolCall {
                name: name.to_string(),
                source: "runtime".to_string(),
                status: "failed".to_string(),
                input: arguments,
                output_preview: truncate_chars(&output_text, MAX_TOOL_PREVIEW_CHARS),
            });
            return output_text;
        };

        let result = self
            .execute_inner(&definition.executor, arguments.clone())
            .await;

        match result {
            Ok(output) => {
                let output_text = render_tool_output_for_model(&output);
                let status = if tool_output_is_error(&output) {
                    "failed"
                } else {
                    "completed"
                };
                self.tool_calls.push(DebugToolCall {
                    name: definition.name,
                    source: definition.source_label,
                    status: status.to_string(),
                    input: arguments,
                    output_preview: truncate_chars(&output_text, MAX_TOOL_PREVIEW_CHARS),
                });
                output_text
            }
            Err(error) => {
                let output = json!({
                    "ok": false,
                    "error": error.to_string(),
                });
                let output_text = render_tool_output_for_model(&output);
                self.tool_calls.push(DebugToolCall {
                    name: definition.name,
                    source: definition.source_label,
                    status: "failed".to_string(),
                    input: arguments,
                    output_preview: truncate_chars(&output_text, MAX_TOOL_PREVIEW_CHARS),
                });
                output_text
            }
        }
    }

    fn register_native_tool(&mut self, config: NativeToolConfig) {
        let Some(kind) = native_tool_kind(&config.name) else {
            self.initialization_warnings.push(format!(
                "native tool `{}` chưa có executor Rust tương ứng, nên bị bỏ qua",
                config.name
            ));
            return;
        };

        let definition = ToolDefinition {
            name: config.name.clone(),
            description: native_tool_description(kind).to_string(),
            input_schema: native_tool_schema(kind),
            source_label: format!("native:{}", config.name),
            executor: ToolExecutor::Native {
                kind,
                timeout: Duration::from_millis(config.timeout_ms.max(1_000)),
            },
        };

        self.insert_definition(definition);
    }

    async fn load_mcp_server(&mut self, config: McpServerConfig) {
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

                    self.mcp_sessions.insert(server_name.clone(), session);
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

    fn insert_definition(&mut self, definition: ToolDefinition) {
        if self.tools.contains_key(&definition.name) {
            self.initialization_warnings.push(format!(
                "tool `{}` bị trùng tên trong runtime, giữ lại bản đầu tiên",
                definition.name
            ));
            return;
        }

        self.tools.insert(definition.name.clone(), definition);
    }

    async fn execute_inner(&mut self, executor: &ToolExecutor, arguments: Value) -> Result<Value> {
        match executor {
            ToolExecutor::Native { kind, timeout } => {
                self.execute_native_tool(*kind, *timeout, arguments).await
            }
            ToolExecutor::Mcp {
                server_name,
                tool_name,
            } => {
                self.execute_mcp_tool(server_name, tool_name, arguments)
                    .await
            }
        }
    }

    async fn execute_native_tool(
        &self,
        kind: NativeToolKind,
        tool_timeout: Duration,
        arguments: Value,
    ) -> Result<Value> {
        match timeout(tool_timeout, async {
            match kind {
                NativeToolKind::FetchPage => self.fetch_page(arguments).await,
                NativeToolKind::ExtractSignals => self.extract_signals(arguments),
                NativeToolKind::MemoryLookup => self.memory_lookup(arguments),
                NativeToolKind::SummarizeSources => self.summarize_sources(arguments),
                NativeToolKind::Read => self.read_path(arguments).await,
                NativeToolKind::Write => self.write_path(arguments).await,
                NativeToolKind::Exec => self.exec_command(arguments, tool_timeout).await,
                NativeToolKind::Bash => self.run_bash(arguments, tool_timeout).await,
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => bail!(
                "native tool `{}` vượt quá timeout {}ms",
                native_tool_name(kind),
                tool_timeout.as_millis()
            ),
        }
    }

    async fn execute_mcp_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        let session = self
            .mcp_sessions
            .get(server_name)
            .ok_or_else(|| anyhow!("MCP server `{server_name}` chưa sẵn sàng"))?;
        session.call_tool(tool_name, arguments).await
    }

    async fn fetch_page(&self, arguments: Value) -> Result<Value> {
        let url = required_string_arg(&arguments, "url")?;
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("không thể tải URL `{url}`"))?;

        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = response
            .text()
            .await
            .context("không thể đọc body từ phản hồi page")?;
        let title = extract_html_title(&body);
        let excerpt = truncate_chars(&collapse_whitespace(&strip_html_tags(&body)), 1_200);

        Ok(json!({
            "ok": status < 400,
            "requested_url": url,
            "final_url": final_url,
            "status": status,
            "content_type": content_type,
            "title": title,
            "excerpt": excerpt,
        }))
    }

    fn extract_signals(&self, arguments: Value) -> Result<Value> {
        let text = required_string_arg(&arguments, "text")?;
        let lowered = text.to_ascii_lowercase();

        let bullish_hits = count_keyword_hits(&lowered, &["bull", "bullish", "breakout", "long"]);
        let bearish_hits = count_keyword_hits(&lowered, &["bear", "bearish", "breakdown", "short"]);

        let bias = match bullish_hits.cmp(&bearish_hits) {
            std::cmp::Ordering::Greater => "bullish",
            std::cmp::Ordering::Less => "bearish",
            std::cmp::Ordering::Equal => "neutral",
        };

        Ok(json!({
            "ok": true,
            "bias": bias,
            "timeframes": find_timeframes(&lowered),
            "levels": extract_candidate_numbers(&text),
            "keywords": collect_keywords(&lowered),
        }))
    }

    fn memory_lookup(&self, arguments: Value) -> Result<Value> {
        let query = optional_string_arg(&arguments, "query").unwrap_or_default();
        let lowered_query = query.to_ascii_lowercase();
        let tokens = lowered_query
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();

        let mut corpus = Vec::new();
        if let Some(context_preview) = self.context_preview.as_deref() {
            for line in context_preview
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                corpus.push(format!("context: {line}"));
            }
        }

        for turn in &self.history {
            corpus.push(format!(
                "{}: {}",
                turn.role,
                collapse_whitespace(&turn.content)
            ));
        }

        let matches = corpus
            .into_iter()
            .filter(|line| {
                if tokens.is_empty() {
                    return true;
                }

                let lowered = line.to_ascii_lowercase();
                tokens.iter().all(|token| lowered.contains(token))
            })
            .take(8)
            .collect::<Vec<_>>();

        Ok(json!({
            "ok": true,
            "query": query,
            "matches": matches,
            "history_turns": self.history.len(),
            "has_backend_context": self.context_preview.is_some(),
        }))
    }

    fn summarize_sources(&self, arguments: Value) -> Result<Value> {
        let mut items = string_array_arg(&arguments, "urls");
        if items.is_empty() {
            items = string_array_arg(&arguments, "items");
        }
        if items.is_empty() {
            if let Some(text) = optional_string_arg(&arguments, "text") {
                items = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect();
            }
        }

        if items.is_empty() {
            bail!("summarize_sources cần `urls`, `items` hoặc `text`");
        }

        let mut domains = Vec::new();
        for item in &items {
            if let Some(domain) = extract_domain(item) {
                if !domains.iter().any(|entry| entry == &domain) {
                    domains.push(domain);
                }
            }
        }

        Ok(json!({
            "ok": true,
            "count": items.len(),
            "domains": domains,
            "items": items.into_iter().take(10).collect::<Vec<_>>(),
        }))
    }

    async fn read_path(&self, arguments: Value) -> Result<Value> {
        let requested_path = required_string_arg(&arguments, "path")?;
        let path = self.resolve_workspace_path(&requested_path)?;
        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("không thể đọc metadata `{}`", path.display()))?;

        if metadata.is_dir() {
            bail!(
                "`{}` là thư mục, tool `read` chỉ hỗ trợ file",
                path.display()
            );
        }

        let bytes = fs::read(&path)
            .await
            .with_context(|| format!("không thể đọc file `{}`", path.display()))?;
        let lossy_utf8 = std::str::from_utf8(&bytes).is_err();
        let content = String::from_utf8_lossy(&bytes).into_owned();
        let start_line = optional_usize_arg(&arguments, "start_line")
            .unwrap_or(1)
            .max(1);
        let line_count = optional_usize_arg(&arguments, "line_count")
            .unwrap_or(200)
            .clamp(1, 2_000);
        let max_chars = optional_usize_arg(&arguments, "max_chars")
            .unwrap_or(6_000)
            .clamp(1, MAX_TOOL_OUTPUT_CHARS);

        let lines = content.lines().collect::<Vec<_>>();
        let start_index = start_line.saturating_sub(1).min(lines.len());
        let end_index = (start_index + line_count).min(lines.len());
        let mut snippet = lines[start_index..end_index].join("\n");
        let truncated = end_index < lines.len() || snippet.chars().count() > max_chars;

        if snippet.chars().count() > max_chars {
            snippet = truncate_chars(&snippet, max_chars);
        }

        Ok(json!({
            "ok": true,
            "path": requested_path,
            "resolved_path": path.display().to_string(),
            "size_bytes": metadata.len(),
            "total_lines": lines.len(),
            "start_line": if lines.is_empty() { 0 } else { start_index + 1 },
            "end_line": end_index,
            "truncated": truncated,
            "lossy_utf8": lossy_utf8,
            "content": snippet,
        }))
    }

    async fn write_path(&self, arguments: Value) -> Result<Value> {
        let requested_path = required_string_arg(&arguments, "path")?;
        let content = required_raw_string_arg(&arguments, "content")?;
        let append = optional_bool_arg(&arguments, "append").unwrap_or(false);
        let create_parent_dirs =
            optional_bool_arg(&arguments, "create_parent_dirs").unwrap_or(true);
        let path = self.resolve_workspace_path(&requested_path)?;

        let Some(parent) = path.parent() else {
            bail!("không thể xác định thư mục cha cho `{}`", path.display());
        };

        if !parent.exists() {
            if create_parent_dirs {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("không thể tạo thư mục cha `{}`", parent.display()))?;
            } else {
                bail!("thư mục cha `{}` chưa tồn tại", parent.display());
            }
        }

        let existed = path.exists();
        if append {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .with_context(|| format!("không thể mở file `{}` để append", path.display()))?;
            file.write_all(content.as_bytes())
                .await
                .with_context(|| format!("không thể append vào `{}`", path.display()))?;
            file.flush().await.ok();
        } else {
            fs::write(&path, content.as_bytes())
                .await
                .with_context(|| format!("không thể ghi file `{}`", path.display()))?;
        }

        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("không thể đọc metadata `{}` sau khi ghi", path.display()))?;

        Ok(json!({
            "ok": true,
            "path": requested_path,
            "resolved_path": path.display().to_string(),
            "mode": if append { "append" } else { "write" },
            "created": !existed,
            "bytes_written": content.as_bytes().len(),
            "size_bytes": metadata.len(),
        }))
    }

    async fn exec_command(&self, arguments: Value, tool_timeout: Duration) -> Result<Value> {
        let command = required_string_arg(&arguments, "command")?;
        let args = string_array_arg(&arguments, "args");
        let cwd = self.resolve_command_cwd(optional_string_arg(&arguments, "cwd").as_deref())?;
        let effective_timeout = resolve_requested_timeout(&arguments, tool_timeout);

        let mut process = Command::new(&command);
        process
            .args(&args)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = match timeout(effective_timeout, process.output()).await {
            Ok(result) => result.with_context(|| format!("không thể chạy lệnh `{command}`"))?,
            Err(_) => {
                bail!(
                    "exec `{command}` vượt quá timeout {}ms",
                    effective_timeout.as_millis()
                )
            }
        };

        Ok(json!({
            "ok": output.status.success(),
            "command": command,
            "args": args,
            "cwd": cwd.display().to_string(),
            "timeout_ms": effective_timeout.as_millis(),
            "exit_code": output.status.code(),
            "success": output.status.success(),
            "stdout": truncate_chars(&String::from_utf8_lossy(&output.stdout), 3_000),
            "stderr": truncate_chars(&String::from_utf8_lossy(&output.stderr), 3_000),
        }))
    }

    async fn run_bash(&self, arguments: Value, tool_timeout: Duration) -> Result<Value> {
        let script = optional_raw_string_arg(&arguments, "script")
            .or_else(|| optional_raw_string_arg(&arguments, "command"))
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `script`"))?;
        let cwd = self.resolve_command_cwd(optional_string_arg(&arguments, "cwd").as_deref())?;
        let effective_timeout = resolve_requested_timeout(&arguments, tool_timeout);

        let mut process = Command::new("bash");
        process
            .arg("-lc")
            .arg(&script)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = match timeout(effective_timeout, process.output()).await {
            Ok(result) => result.context("không thể chạy bash process")?,
            Err(_) => {
                bail!("bash vượt quá timeout {}ms", effective_timeout.as_millis())
            }
        };

        Ok(json!({
            "ok": output.status.success(),
            "script": script,
            "cwd": cwd.display().to_string(),
            "timeout_ms": effective_timeout.as_millis(),
            "exit_code": output.status.code(),
            "success": output.status.success(),
            "stdout": truncate_chars(&String::from_utf8_lossy(&output.stdout), 3_000),
            "stderr": truncate_chars(&String::from_utf8_lossy(&output.stderr), 3_000),
        }))
    }

    fn resolve_workspace_path(&self, requested_path: &str) -> Result<PathBuf> {
        let candidate = PathBuf::from(requested_path);
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            self.workspace_root.join(candidate)
        };
        let normalized = normalize_path_for_workspace(&candidate)?;
        ensure_path_is_within_workspace(&normalized, &self.workspace_root)?;
        Ok(normalized)
    }

    fn resolve_command_cwd(&self, requested_cwd: Option<&str>) -> Result<PathBuf> {
        let cwd = match requested_cwd {
            Some(value) => self.resolve_workspace_path(value)?,
            None => self.workspace_root.clone(),
        };

        if !cwd.is_dir() {
            bail!("cwd `{}` không phải thư mục", cwd.display());
        }

        Ok(cwd)
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

fn native_tool_kind(name: &str) -> Option<NativeToolKind> {
    match name.trim().to_ascii_lowercase().as_str() {
        "fetch_page" => Some(NativeToolKind::FetchPage),
        "extract_signals" => Some(NativeToolKind::ExtractSignals),
        "memory_lookup" => Some(NativeToolKind::MemoryLookup),
        "summarize_sources" => Some(NativeToolKind::SummarizeSources),
        "read" => Some(NativeToolKind::Read),
        "write" => Some(NativeToolKind::Write),
        "exec" => Some(NativeToolKind::Exec),
        "bash" => Some(NativeToolKind::Bash),
        _ => None,
    }
}

fn native_tool_name(kind: NativeToolKind) -> &'static str {
    match kind {
        NativeToolKind::FetchPage => "fetch_page",
        NativeToolKind::ExtractSignals => "extract_signals",
        NativeToolKind::MemoryLookup => "memory_lookup",
        NativeToolKind::SummarizeSources => "summarize_sources",
        NativeToolKind::Read => "read",
        NativeToolKind::Write => "write",
        NativeToolKind::Exec => "exec",
        NativeToolKind::Bash => "bash",
    }
}

fn native_tool_description(kind: NativeToolKind) -> &'static str {
    match kind {
        NativeToolKind::FetchPage => {
            "Tải một URL và trả về status, final URL, title và excerpt để rà nguồn nhanh."
        }
        NativeToolKind::ExtractSignals => {
            "Rút bias, timeframe, keywords và các mức giá ứng viên từ raw text kỹ thuật."
        }
        NativeToolKind::MemoryLookup => {
            "Tra nhanh backend context preview và lịch sử chat hiện tại theo từ khoá."
        }
        NativeToolKind::SummarizeSources => {
            "Tóm tắt danh sách URL hoặc item nguồn thành số lượng và domain chính."
        }
        NativeToolKind::Read => {
            "Đọc file trong workspace của backend với cửa sổ dòng và giới hạn ký tự."
        }
        NativeToolKind::Write => {
            "Ghi hoặc append nội dung text vào file trong workspace của backend."
        }
        NativeToolKind::Exec => "Chạy một executable trực tiếp trong workspace mà không qua shell.",
        NativeToolKind::Bash => {
            "Chạy một lệnh bash ngắn trong workspace để debug hoặc thao tác nhanh."
        }
    }
}

fn native_tool_schema(kind: NativeToolKind) -> Value {
    match kind {
        NativeToolKind::FetchPage => json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL cần tải"
                }
            },
            "required": ["url"],
            "additionalProperties": false,
        }),
        NativeToolKind::ExtractSignals => json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Đoạn text kỹ thuật cần rút tín hiệu"
                }
            },
            "required": ["text"],
            "additionalProperties": false,
        }),
        NativeToolKind::MemoryLookup => json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Từ khoá cần tra trong backend context và history"
                }
            },
            "additionalProperties": false,
        }),
        NativeToolKind::SummarizeSources => json!({
            "type": "object",
            "properties": {
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Danh sách URL nguồn"
                },
                "items": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Danh sách item nguồn bất kỳ"
                },
                "text": {
                    "type": "string",
                    "description": "Raw text chứa danh sách nguồn, mỗi dòng một item"
                }
            },
            "additionalProperties": false,
        }),
        NativeToolKind::Read => json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path file cần đọc, relative theo workspace tool"
                },
                "start_line": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Dòng bắt đầu, mặc định 1"
                },
                "line_count": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Số dòng cần đọc, mặc định 200"
                },
                "max_chars": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Giới hạn ký tự trả về"
                }
            },
            "required": ["path"],
            "additionalProperties": false,
        }),
        NativeToolKind::Write => json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path file cần ghi, relative theo workspace tool"
                },
                "content": {
                    "type": "string",
                    "description": "Nội dung text cần ghi"
                },
                "append": {
                    "type": "boolean",
                    "description": "Nếu true sẽ append thay vì overwrite"
                },
                "create_parent_dirs": {
                    "type": "boolean",
                    "description": "Nếu true sẽ tự tạo thư mục cha còn thiếu"
                }
            },
            "required": ["path", "content"],
            "additionalProperties": false,
        }),
        NativeToolKind::Exec => json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Tên executable cần chạy"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Danh sách argument truyền vào executable"
                },
                "cwd": {
                    "type": "string",
                    "description": "Thư mục làm việc, relative theo workspace tool"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Timeout mong muốn, không vượt quá timeout của tool"
                }
            },
            "required": ["command"],
            "additionalProperties": false,
        }),
        NativeToolKind::Bash => json!({
            "type": "object",
            "properties": {
                "script": {
                    "type": "string",
                    "description": "Đoạn lệnh bash cần chạy"
                },
                "cwd": {
                    "type": "string",
                    "description": "Thư mục làm việc, relative theo workspace tool"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Timeout mong muốn, không vượt quá timeout của tool"
                }
            },
            "required": ["script"],
            "additionalProperties": false,
        }),
    }
}

fn required_string_arg(arguments: &Value, field: &str) -> Result<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `{field}`"))
}

fn required_raw_string_arg(arguments: &Value, field: &str) -> Result<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `{field}`"))
}

fn optional_string_arg(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn optional_raw_string_arg(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn optional_bool_arg(arguments: &Value, field: &str) -> Option<bool> {
    arguments.get(field).and_then(Value::as_bool)
}

fn optional_usize_arg(arguments: &Value, field: &str) -> Option<usize> {
    arguments
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn optional_u64_arg(arguments: &Value, field: &str) -> Option<u64> {
    arguments.get(field).and_then(Value::as_u64)
}

fn string_array_arg(arguments: &Value, field: &str) -> Vec<String> {
    arguments
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn render_tool_output_for_model(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return truncate_chars(text, MAX_TOOL_OUTPUT_CHARS);
    }

    if let Some(text) = extract_text_from_tool_result(value) {
        return truncate_chars(&collapse_whitespace(&text), MAX_TOOL_OUTPUT_CHARS);
    }

    let serialized = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    truncate_chars(&serialized, MAX_TOOL_OUTPUT_CHARS)
}

fn extract_text_from_tool_result(value: &Value) -> Option<String> {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return Some(text.to_string());
        }
    }

    let items = value.get("content")?.as_array()?;
    let text = items
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");

    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn tool_output_is_error(value: &Value) -> bool {
    value
        .get("isError")
        .and_then(Value::as_bool)
        .or_else(|| value.get("is_error").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn sanitize_tool_arguments(value: Value) -> Value {
    match value {
        Value::Null => json!({}),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .filter_map(|(key, value)| {
                    let value = sanitize_tool_arguments(value);
                    if value.is_null() {
                        None
                    } else {
                        Some((key, value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(sanitize_tool_arguments)
                .filter(|value| !value.is_null())
                .collect(),
        ),
        other => other,
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

fn extract_html_title(body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let Some(tag_end) = lower[start..].find('>') else {
        return String::new();
    };
    let content_start = start + tag_end + 1;
    let Some(end) = lower[content_start..].find("</title>") else {
        return String::new();
    };

    collapse_whitespace(&body[content_start..content_start + end])
}

fn strip_html_tags(body: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut inside_tag = false;

    for ch in body.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

fn count_keyword_hits(haystack: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| haystack.contains(**keyword))
        .count()
}

fn find_timeframes(lowered: &str) -> Vec<&'static str> {
    ["1m", "5m", "15m", "1h", "4h", "1d", "1w", "daily", "weekly"]
        .into_iter()
        .filter(|timeframe| lowered.contains(timeframe))
        .collect()
}

fn collect_keywords(lowered: &str) -> Vec<&'static str> {
    [
        "support",
        "resistance",
        "breakout",
        "breakdown",
        "volume",
        "trend",
        "range",
        "retest",
    ]
    .into_iter()
    .filter(|keyword| lowered.contains(keyword))
    .collect()
}

fn extract_candidate_numbers(text: &str) -> Vec<String> {
    let mut current = String::new();
    let mut values = Vec::new();

    for ch in text.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | ',' | '/') {
            current.push(ch);
        } else {
            push_number_candidate(&mut values, &mut current);
        }
    }
    push_number_candidate(&mut values, &mut current);

    values
}

fn push_number_candidate(values: &mut Vec<String>, current: &mut String) {
    let candidate = current.trim_matches(|ch: char| ch == '.' || ch == ',' || ch == '/');
    if candidate.chars().filter(|ch| ch.is_ascii_digit()).count() >= 3
        && !values.iter().any(|value| value == candidate)
    {
        values.push(candidate.to_string());
    }
    current.clear();
}

fn extract_domain(value: &str) -> Option<String> {
    reqwest::Url::parse(value)
        .ok()
        .and_then(|url| url.domain().map(str::to_string))
}

fn resolve_workspace_root() -> Result<PathBuf> {
    let base = env::var("HYBRIDTRADE_TOOL_ROOT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().context("không thể xác định current_dir cho native tools")?);

    base.canonicalize()
        .with_context(|| format!("không thể canonicalize workspace root `{}`", base.display()))
}

fn best_effort_canonicalize(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn normalize_path_for_workspace(candidate: &Path) -> Result<PathBuf> {
    let mut existing = candidate;
    let mut suffix = Vec::<OsString>::new();

    loop {
        if existing.exists() {
            let mut normalized = existing
                .canonicalize()
                .with_context(|| format!("không thể canonicalize path `{}`", existing.display()))?;
            for part in suffix.iter().rev() {
                normalized.push(part);
            }
            return Ok(normalized);
        }

        let Some(name) = existing.file_name() else {
            bail!("path `{}` không hợp lệ", candidate.display());
        };
        suffix.push(name.to_os_string());
        existing = existing
            .parent()
            .ok_or_else(|| anyhow!("path `{}` không hợp lệ", candidate.display()))?;
    }
}

fn ensure_path_is_within_workspace(path: &Path, workspace_root: &Path) -> Result<()> {
    if path.starts_with(workspace_root) {
        Ok(())
    } else {
        bail!(
            "path `{}` nằm ngoài workspace `{}`",
            path.display(),
            workspace_root.display()
        )
    }
}

fn resolve_requested_timeout(arguments: &Value, max_timeout: Duration) -> Duration {
    let max_ms = max_timeout.as_millis().min(u128::from(u64::MAX)) as u64;
    optional_u64_arg(arguments, "timeout_ms")
        .map(|value| value.max(1).min(max_ms))
        .map(Duration::from_millis)
        .unwrap_or(max_timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs as std_fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "hybridtrade-tool-runtime-{label}-{}-{unique}",
            std::process::id()
        ));
        std_fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_runtime(workspace_root: PathBuf) -> ToolRuntime {
        ToolRuntime {
            http_client: Client::new(),
            history: Vec::new(),
            context_preview: None,
            workspace_root,
            tools: BTreeMap::new(),
            mcp_sessions: HashMap::new(),
            initialization_warnings: Vec::new(),
            tool_calls: Vec::new(),
        }
    }

    #[tokio::test]
    async fn read_tool_supports_line_windows() {
        let root = temp_dir("read");
        std_fs::write(root.join("notes.txt"), "line-1\nline-2\nline-3\n").unwrap();
        let runtime = test_runtime(root.clone());

        let result = runtime
            .read_path(json!({
                "path": "notes.txt",
                "start_line": 2,
                "line_count": 2,
            }))
            .await
            .unwrap();

        assert_eq!(result["content"], "line-2\nline-3");
        assert_eq!(result["start_line"], 2);
        assert_eq!(result["end_line"], 3);

        let _ = std_fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn write_tool_blocks_paths_outside_workspace() {
        let root = temp_dir("write");
        let runtime = test_runtime(root.clone());

        let error = runtime
            .write_path(json!({
                "path": "../escape.txt",
                "content": "boom",
            }))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("nằm ngoài workspace"));

        let _ = std_fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn exec_tool_runs_inside_workspace() {
        let root = temp_dir("exec");
        let runtime = test_runtime(root.clone());

        let result = runtime
            .exec_command(
                json!({
                    "command": "pwd",
                }),
                Duration::from_secs(2),
            )
            .await
            .unwrap();

        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains(&root.display().to_string()));

        let _ = std_fs::remove_dir_all(root);
    }
}
