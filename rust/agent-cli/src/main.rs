use std::{
    collections::BTreeMap,
    collections::BTreeSet,
    io::{self, Write},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_BACKEND_URL: &str = "http://127.0.0.1:8080";

#[derive(Debug, Parser)]
#[command(
    name = "hybridtrade-agent-cli",
    version,
    about = "CLI riêng để chat với backend agents của HybridTrade"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        env = "HYBRIDTRADE_BACKEND_URL",
        default_value = DEFAULT_BACKEND_URL,
        help = "Base URL của backend HybridTrade"
    )]
    backend_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Liệt kê provider mà backend đang bật")]
    Providers,
    #[command(about = "Liệt kê backend agents có thể chat")]
    Agents,
    #[command(about = "Chat với một backend agent")]
    Chat(ChatArgs),
}

#[derive(Debug, Clone, Args)]
struct ChatArgs {
    #[arg(long, short = 'a', help = "Tên agent, mặc định là kuromi")]
    agent: Option<String>,

    #[arg(
        long,
        short = 'p',
        help = "Provider muốn ép dùng, ví dụ openai hoặc anthropic"
    )]
    provider: Option<String>,

    #[arg(long, help = "ID chat session để giữ MCP/CDP state xuyên nhiều lượt")]
    chat_session_id: Option<String>,

    #[arg(long, short = 'm', help = "Tin nhắn một lần. Nếu bỏ trống sẽ vào REPL")]
    message: Option<String>,

    #[arg(long, help = "Max tokens gửi sang backend")]
    max_tokens: Option<u32>,

    #[arg(long, help = "Temperature gửi sang backend")]
    temperature: Option<f32>,

    #[arg(long, help = "Không nhúng ngữ cảnh investigation từ backend")]
    no_backend_context: bool,

    #[arg(long, help = "In thêm prompt hệ thống và context preview backend")]
    show_debug: bool,
}

#[derive(Debug, Deserialize)]
struct ProviderStatusView {
    name: String,
    enabled: bool,
    configured: bool,
    model: String,
    default_for_chat: bool,
}

#[derive(Debug, Deserialize)]
struct DebugAgentView {
    role: String,
    label: String,
    status: String,
    providers: Vec<String>,
    default_provider: String,
    #[serde(default)]
    available_commands: Vec<String>,
    #[serde(default)]
    mcp_servers: Vec<DebugMcpServerView>,
    #[serde(default)]
    native_tools: Vec<DebugToolView>,
}

#[derive(Debug, Deserialize)]
struct DebugMcpServerView {
    name: String,
}

#[derive(Debug, Deserialize)]
struct DebugToolView {
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatTurn {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct DebugAgentChatRequest {
    message: String,
    provider: Option<String>,
    chat_session_id: Option<String>,
    history: Vec<ChatTurn>,
    include_backend_context: Option<bool>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct DebugAgentChatDebug {
    system_prompt: String,
    context_preview: Option<String>,
    history_count: usize,
    #[serde(default)]
    compacted: bool,
    #[serde(default)]
    original_history_count: usize,
    #[serde(default)]
    retained_history_count: usize,
    #[serde(default)]
    compacted_turns: usize,
    #[serde(default)]
    estimated_chars_before: usize,
    #[serde(default)]
    estimated_chars_after: usize,
    #[serde(default)]
    compact_mode: Option<String>,
    #[serde(default)]
    compact_summary_preview: Option<String>,
    #[serde(default)]
    available_tools: Vec<String>,
    #[serde(default)]
    tool_runtime_warnings: Vec<String>,
    #[serde(default)]
    tool_calls: Vec<DebugToolCall>,
}

#[derive(Debug, Deserialize)]
struct DebugToolCall {
    name: String,
    source: String,
    status: String,
    #[serde(default)]
    input: serde_json::Value,
    output_preview: String,
}

#[derive(Debug, Deserialize)]
struct DebugAgentChatResponse {
    agent_role: String,
    provider: String,
    model: String,
    content: String,
    chat_session_id: Option<String>,
    debug: DebugAgentChatDebug,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum ChatStreamEvent {
    Connected,
    AgentThinking {
        model: String,
    },
    ThinkingStart,
    ThinkingDelta {
        #[serde(default)]
        text: String,
    },
    TextStart,
    TextDelta {
        #[serde(default)]
        text: String,
    },
    AgentToolCall {
        tool: String,
        input_preview: String,
    },
    AgentToolResult {
        tool: String,
        status: String,
        output_preview: String,
    },
    TeamStarted {
        #[serde(default)]
        session_id: String,
        mission: String,
        members: Vec<String>,
    },
    TeamRound {
        round: usize,
        total: usize,
        #[serde(default)]
        phase: String,
    },
    TeamDirective {
        #[serde(default)]
        session_id: String,
        #[serde(default)]
        seq: usize,
        #[serde(default)]
        to: String,
        #[serde(default)]
        content_preview: String,
    },
    TeamToolCall {
        member: String,
        tool: String,
        status: String,
        output_preview: String,
    },
    TeamMemberResponse {
        member: String,
        round: usize,
        content: String,
        #[serde(default)]
        tool_calls: Vec<DebugToolCall>,
    },
    TeamCompleted,
    Response {
        data: Box<DebugAgentChatResponse>,
    },
    Error {
        message: String,
    },
}

struct BackendClient {
    base_url: String,
    client: Client,
}

impl BackendClient {
    fn new(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .context("không thể khởi tạo HTTP client")?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    async fn providers(&self) -> Result<Vec<ProviderStatusView>> {
        self.get_json("/api/debug/providers").await
    }

    async fn agents(&self) -> Result<Vec<DebugAgentView>> {
        self.get_json("/api/debug/agents").await
    }

    async fn chat(
        &self,
        agent: &str,
        request: &DebugAgentChatRequest,
    ) -> Result<(DebugAgentChatResponse, bool)> {
        let url = self.url(&format!("/api/debug/agents/{agent}/chat"));
        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .with_context(|| format!("không thể gọi backend POST {url}"))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .context("không thể đọc phản hồi backend")?;
            if let Ok(error) = serde_json::from_str::<ApiError>(&text) {
                bail!("backend lỗi {}: {}", status, error.error);
            }
            bail!("backend lỗi {}: {}", status, text.trim());
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut final_response: Option<DebugAgentChatResponse> = None;
        let mut has_tool_activity = false;
        let mut status_line_visible = false;
        let mut text_was_streamed = false;
        let chunk_timeout = Duration::from_secs(300); // 5 min per chunk

        loop {
            let chunk = match tokio::time::timeout(chunk_timeout, stream.next()).await {
                Ok(Some(chunk)) => chunk.context("lỗi đọc SSE stream")?,
                Ok(None) => break, // stream ended
                Err(_) => bail!("SSE stream timeout: không nhận được dữ liệu trong 5 phút"),
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(event) = extract_sse_event(&mut buffer) {
                match serde_json::from_str::<ChatStreamEvent>(&event) {
                    Ok(ChatStreamEvent::Connected) => {}
                    Ok(ChatStreamEvent::AgentThinking { model }) => {
                        print!("\x1b[2m⟡ Thinking ({})\x1b[0m", model);
                        io::stdout().flush().ok();
                        status_line_visible = true;
                    }
                    Ok(ChatStreamEvent::ThinkingStart) => {
                        // AgentThinking status line is already showing
                    }
                    Ok(ChatStreamEvent::ThinkingDelta { .. }) => {
                        // Suppress thinking stream for clean CLI output
                    }
                    Ok(ChatStreamEvent::TextStart) => {
                        if status_line_visible {
                            print!("\r\x1b[2K");
                            status_line_visible = false;
                        }
                        if has_tool_activity {
                            println!();
                        }
                    }
                    Ok(ChatStreamEvent::TextDelta { text }) => {
                        print!("{}", text);
                        io::stdout().flush().ok();
                        text_was_streamed = true;
                    }
                    Ok(ChatStreamEvent::AgentToolCall { tool, input_preview }) => {
                        if text_was_streamed {
                            println!();
                            println!();
                            text_was_streamed = false;
                        }
                        // Clear thinking status line on first tool call
                        if !has_tool_activity && status_line_visible {
                            print!("\r\x1b[2K");
                            status_line_visible = false;
                        }
                        has_tool_activity = true;
                        let preview = single_line_preview(&input_preview, 80);
                        println!(
                            "\x1b[33m  → {}\x1b[2m({})\x1b[0m",
                            tool, preview
                        );
                    }
                    Ok(ChatStreamEvent::AgentToolResult {
                        tool,
                        status,
                        output_preview,
                    }) => {
                        let preview = single_line_preview(&output_preview, 100);
                        let color = if status == "completed" { "32" } else { "31" };
                        println!(
                            "\x1b[{}m  ← {} [{}]\x1b[2m {}\x1b[0m",
                            color, tool, status, preview,
                        );
                    }
                    Ok(ChatStreamEvent::TeamStarted { session_id, mission, members }) => {
                        if !has_tool_activity {
                            print!("\r\x1b[2K");
                            has_tool_activity = true;
                        }
                        println!();
                        println!("=== Team [{}]: {} ===", session_id, mission);
                        println!(
                            "Members: {}",
                            members.join(", ")
                        );
                    }
                    Ok(ChatStreamEvent::TeamDirective { seq, to, content_preview, .. }) => {
                        let target = if to == "*" { "all".to_string() } else { to };
                        println!("  [kuromi→{}] #{}: {}", target, seq, truncate_preview(&content_preview, 150));
                    }
                    Ok(ChatStreamEvent::TeamRound { round, total, phase }) => {
                        let phase_label = if phase.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", phase)
                        };
                        println!();
                        println!("--- Round {}/{}{} ---", round, total, phase_label);
                    }
                    Ok(ChatStreamEvent::TeamToolCall {
                        member,
                        tool,
                        status,
                        output_preview,
                    }) => {
                        let preview = single_line_preview(&output_preview, 120);
                        println!(
                            "[{}] tool: {} -> {} [{}]",
                            member, tool, preview, status
                        );
                    }
                    Ok(ChatStreamEvent::TeamMemberResponse {
                        member,
                        round: _,
                        content,
                        ..
                    }) => {
                        println!("[{}] {}", member, truncate_preview(&content, 300));
                    }
                    Ok(ChatStreamEvent::TeamCompleted) => {
                        println!("=== Team Complete ===");
                        println!();
                    }
                    Ok(ChatStreamEvent::Response { data }) => {
                        if text_was_streamed {
                            println!(); // newline after last streamed chunk
                        } else if !has_tool_activity {
                            // Clear thinking status line if no tool calls happened
                            print!("\r\x1b[2K");
                        } else {
                            println!();
                        }
                        final_response = Some(*data);
                    }
                    Ok(ChatStreamEvent::Error { message }) => {
                        bail!("server error: {}", message);
                    }
                    Err(_) => {}
                }
            }
        }

        final_response
            .map(|r| (r, text_was_streamed))
            .context("backend không trả về response event")
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .get(self.url(path))
            .send()
            .await
            .with_context(|| format!("không thể gọi backend GET {path}"))?;

        decode_response(response).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let backend = BackendClient::new(cli.backend_url)?;

    match cli.command {
        Commands::Providers => run_providers(&backend).await,
        Commands::Agents => run_agents(&backend).await,
        Commands::Chat(args) => run_chat(&backend, args).await,
    }
}

async fn run_providers(backend: &BackendClient) -> Result<()> {
    let providers = backend.providers().await?;
    if providers.is_empty() {
        println!("Backend chưa khai báo provider nào.");
        return Ok(());
    }

    for provider in providers {
        println!(
            "- {} | bật: {} | cấu hình: {} | mặc định chat: {} | model: {}",
            provider.name,
            yes_no(provider.enabled),
            yes_no(provider.configured),
            yes_no(provider.default_for_chat),
            provider.model
        );
    }

    Ok(())
}

async fn run_agents(backend: &BackendClient) -> Result<()> {
    let agents = backend.agents().await?;
    if agents.is_empty() {
        println!("Backend chưa có agent nào khả dụng.");
        return Ok(());
    }

    let commands = agents
        .iter()
        .flat_map(|agent| {
            agent
                .available_commands
                .iter()
                .map(|name| format!("/{name}"))
        })
        .collect::<BTreeSet<_>>();
    let mcp_servers = agents
        .iter()
        .flat_map(|agent| agent.mcp_servers.iter().map(|item| item.name.clone()))
        .collect::<BTreeSet<_>>();
    let native_tools = agents
        .iter()
        .flat_map(|agent| agent.native_tools.iter().map(|item| item.name.clone()))
        .collect::<BTreeSet<_>>();

    println!("Commands: {}", join_set(&commands));
    println!("MCP chung: {}", join_set(&mcp_servers));
    println!("Tools chung: {}", join_set(&native_tools));
    println!();

    for agent in agents {
        let providers = if agent.providers.is_empty() {
            "không có".to_string()
        } else {
            agent.providers.join(", ")
        };
        let commands = if agent.available_commands.is_empty() {
            "không có".to_string()
        } else {
            agent
                .available_commands
                .iter()
                .map(|name| format!("/{name}"))
                .collect::<Vec<_>>()
                .join(", ")
        };

        println!(
            "- {} ({}) | trạng thái: {} | provider mặc định: {} | khả dụng: {}",
            agent.role, agent.label, agent.status, agent.default_provider, providers,
        );
        println!("  commands: {}", commands);
    }

    Ok(())
}

fn join_set(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        "không có".to_string()
    } else {
        values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

async fn run_chat(backend: &BackendClient, args: ChatArgs) -> Result<()> {
    let agent = selected_agent(&args);

    if let Some(message) = args.message.clone() {
        let request = build_chat_request(&args, message, Vec::new(), args.chat_session_id.clone());
        let (response, streamed) = backend.chat(&agent, &request).await?;
        print_chat_response(&response, args.show_debug, streamed);
        return Ok(());
    }

    run_repl(backend, args).await
}

async fn run_repl(backend: &BackendClient, args: ChatArgs) -> Result<()> {
    let agent = selected_agent(&args);

    println!(
        "Đang chat với agent `{}` qua backend {}",
        agent, backend.base_url
    );
    println!("Lệnh hỗ trợ: /exit, /quit, /clear, /debug");

    let stdin = io::stdin();
    let mut history = Vec::new();
    let mut show_debug = args.show_debug;
    let mut chat_session_id = args
        .chat_session_id
        .clone()
        .unwrap_or_else(new_chat_session_id);

    loop {
        print!("bạn> ");
        io::stdout().flush().context("không thể flush stdout")?;

        let mut line = String::new();
        let read = stdin.read_line(&mut line).context("không thể đọc stdin")?;
        if read == 0 {
            break;
        }

        let message = line.trim();
        if message.is_empty() {
            continue;
        }

        match message {
            "/exit" | "/quit" => break,
            "/clear" => {
                history.clear();
                chat_session_id = new_chat_session_id();
                println!("Đã xoá lịch sử chat cục bộ và tạo chat session mới.");
                continue;
            }
            "/debug" => {
                show_debug = !show_debug;
                println!(
                    "Chế độ debug hiện là: {}",
                    if show_debug { "bật" } else { "tắt" }
                );
                continue;
            }
            _ => {}
        }

        let request = build_chat_request(
            &args,
            message.to_string(),
            history.clone(),
            Some(chat_session_id.clone()),
        );
        let (response, streamed) = backend.chat(&agent, &request).await?;
        println!();
        print_chat_response(&response, show_debug, streamed);
        println!();

        history.push(ChatTurn {
            role: "user".to_string(),
            content: message.to_string(),
        });
        history.push(ChatTurn {
            role: "assistant".to_string(),
            content: response.content,
        });
    }

    Ok(())
}

fn selected_agent(args: &ChatArgs) -> String {
    args.agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("kuromi")
        .to_string()
}

fn build_chat_request(
    args: &ChatArgs,
    message: String,
    history: Vec<ChatTurn>,
    chat_session_id: Option<String>,
) -> DebugAgentChatRequest {
    DebugAgentChatRequest {
        message,
        provider: args.provider.clone(),
        chat_session_id,
        history,
        include_backend_context: Some(!args.no_backend_context),
        max_tokens: args.max_tokens,
        temperature: args.temperature,
    }
}

fn print_chat_response(response: &DebugAgentChatResponse, show_debug: bool, content_already_streamed: bool) {
    println!(
        "\x1b[2m{} | {} | {}\x1b[0m",
        agent_label(&response.agent_role),
        response.provider,
        response.model,
    );

    if !content_already_streamed {
        let assistant_content = render_assistant_content(&response.content);
        println!();
        println!("{}", assistant_content);
    }

    print_tool_call_summary(&response.debug.tool_calls);

    if show_debug {
        println!();
        println!("=== Debug ===");
        println!(
            "chat_session_id={} | history_count={}",
            response.chat_session_id.as_deref().unwrap_or("không có"),
            response.debug.history_count,
        );
        println!(
            "compact={} | mode={} | compacted_turns={} | retained={}/{} | chars={} -> {}",
            yes_no(response.debug.compacted),
            response.debug.compact_mode.as_deref().unwrap_or("không"),
            response.debug.compacted_turns,
            response.debug.retained_history_count,
            response.debug.original_history_count,
            response.debug.estimated_chars_before,
            response.debug.estimated_chars_after,
        );
        println!(
            "available_tools ({}) = {}",
            response.debug.available_tools.len(),
            if response.debug.available_tools.is_empty() {
                "không có".to_string()
            } else {
                response.debug.available_tools.join(", ")
            }
        );

        if !response.debug.tool_runtime_warnings.is_empty() {
            println!();
            println!("tool_runtime_warnings:");
            for warning in &response.debug.tool_runtime_warnings {
                println!("- {}", warning);
            }
        }

        if !response.debug.tool_calls.is_empty() {
            println!();
            println!("tool_calls:");
            for (index, call) in response.debug.tool_calls.iter().enumerate() {
                println!(
                    "{}. {} | status={} | source={}",
                    index + 1,
                    call.name,
                    call.status,
                    call.source,
                );
                println!("   input: {}", format_json_preview(&call.input, 220));
                println!(
                    "   output: {}",
                    single_line_preview(&call.output_preview, 220)
                );
            }
        }

        println!();
        println!("system_prompt:");
        println!("{}", response.debug.system_prompt);

        if let Some(context_preview) = response.debug.context_preview.as_deref() {
            println!();
            println!("context_preview:");
            println!("{}", context_preview);
        }

        if let Some(compact_summary_preview) = response.debug.compact_summary_preview.as_deref() {
            println!();
            println!("compact_summary_preview:");
            println!("{}", compact_summary_preview);
        }
    }
}

fn agent_label(role: &str) -> &str {
    match role.trim().to_ascii_lowercase().as_str() {
        "kuromi" | "kuromi_finance" | "kuromi-finance" | "coordinator" => "Kuromi Finance",
        "user" => "User",
        _ => role,
    }
}

fn render_assistant_content(content: &str) -> &str {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        "[assistant không trả nội dung text; hãy xem mục Tools hoặc Debug bên dưới]"
    } else {
        trimmed
    }
}

fn print_tool_call_summary(tool_calls: &[DebugToolCall]) {
    if tool_calls.is_empty() {
        return;
    }

    let mut summaries = BTreeMap::<String, (usize, bool)>::new();
    for call in tool_calls {
        let entry = summaries.entry(call.name.clone()).or_insert((0, false));
        entry.0 += 1;
        entry.1 |= call.status != "completed";
    }

    println!();
    println!("=== Tools Used ({}) ===", tool_calls.len());
    for (name, (count, has_failure)) in summaries {
        let status = if has_failure { "warning" } else { "ok" };
        if count == 1 {
            println!("- {} [{}]", name, status);
        } else {
            println!("- {} x{} [{}]", name, count, status);
        }
    }
}

fn format_json_preview(value: &serde_json::Value, max_chars: usize) -> String {
    let rendered = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    truncate_preview(&rendered, max_chars)
}

fn single_line_preview(value: &str, max_chars: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_preview(&collapsed, max_chars)
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }

    chars.into_iter().take(max_chars).collect::<String>() + "..."
}

fn new_chat_session_id() -> String {
    Uuid::new_v4().to_string()
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "có"
    } else {
        "không"
    }
}

async fn decode_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let text = response
        .text()
        .await
        .context("không thể đọc phản hồi backend")?;

    if !status.is_success() {
        if let Ok(error) = serde_json::from_str::<ApiError>(&text) {
            bail!("backend lỗi {}: {}", status, error.error);
        }

        let body = text.trim();
        if body.is_empty() {
            bail!("backend lỗi {}", status);
        }

        bail!("backend lỗi {}: {}", status, body);
    }

    serde_json::from_str(&text).with_context(|| {
        format!(
            "không thể parse JSON từ backend: {}",
            truncate_for_error(&text)
        )
    })
}

fn truncate_for_error(value: &str) -> String {
    const MAX_CHARS: usize = 240;
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= MAX_CHARS {
        return value.to_string();
    }

    chars.into_iter().take(MAX_CHARS).collect::<String>() + "..."
}

fn extract_sse_event(buffer: &mut String) -> Option<String> {
    loop {
        let double_newline = buffer.find("\n\n")?;
        let block = buffer[..double_newline].to_string();
        *buffer = buffer[double_newline + 2..].to_string();

        for line in block.lines() {
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if !data.is_empty() {
                    return Some(data.to_string());
                }
            }
        }
    }
}
