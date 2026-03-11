use std::{
    collections::BTreeSet,
    io::{self, Write},
};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
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

    #[arg(long, short = 'i', help = "ID investigation để backend nhúng ngữ cảnh")]
    investigation_id: Option<String>,

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
    mcp_servers: Vec<DebugMcpServerView>,
    #[serde(default)]
    native_tools: Vec<DebugToolView>,
    #[serde(default)]
    common_skills: Vec<String>,
    #[serde(default)]
    agent_skills: Vec<String>,
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
    investigation_id: Option<String>,
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
    investigation_id: Option<String>,
    chat_session_id: Option<String>,
    debug: DebugAgentChatDebug,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: String,
}

struct BackendClient {
    base_url: String,
    client: Client,
}

impl BackendClient {
    fn new(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
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
    ) -> Result<DebugAgentChatResponse> {
        self.post_json(&format!("/api/debug/agents/{agent}/chat"), request)
            .await
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

    async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let response = self
            .client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .with_context(|| format!("không thể gọi backend POST {path}"))?;

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

    let common_skills = agents
        .iter()
        .flat_map(|agent| agent.common_skills.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mcp_servers = agents
        .iter()
        .flat_map(|agent| agent.mcp_servers.iter().map(|item| item.name.clone()))
        .collect::<BTreeSet<_>>();
    let native_tools = agents
        .iter()
        .flat_map(|agent| agent.native_tools.iter().map(|item| item.name.clone()))
        .collect::<BTreeSet<_>>();

    println!("Skills chung: {}", join_set(&common_skills));
    println!("MCP chung: {}", join_set(&mcp_servers));
    println!("Tools chung: {}", join_set(&native_tools));
    println!();

    for agent in agents {
        let providers = if agent.providers.is_empty() {
            "không có".to_string()
        } else {
            agent.providers.join(", ")
        };
        let agent_skills = if agent.agent_skills.is_empty() {
            "không có".to_string()
        } else {
            agent.agent_skills.join(", ")
        };

        println!(
            "- {} ({}) | trạng thái: {} | provider mặc định: {} | khả dụng: {}",
            agent.role, agent.label, agent.status, agent.default_provider, providers,
        );
        println!("  skills riêng: {}", agent_skills);
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
        let response = backend.chat(&agent, &request).await?;
        print_chat_response(&response, args.show_debug);
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
        let response = backend.chat(&agent, &request).await?;
        println!();
        print_chat_response(&response, show_debug);
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
        investigation_id: args.investigation_id.clone(),
        chat_session_id,
        history,
        include_backend_context: Some(!args.no_backend_context),
        max_tokens: args.max_tokens,
        temperature: args.temperature,
    }
}

fn print_chat_response(response: &DebugAgentChatResponse, show_debug: bool) {
    println!(
        "[agent: {} | provider: {} | model: {}]",
        response.agent_role, response.provider, response.model
    );
    println!("{}", response.content.trim());

    if show_debug {
        println!();
        println!(
            "[debug] investigation_id: {}",
            response.investigation_id.as_deref().unwrap_or("không có")
        );
        println!(
            "[debug] chat_session_id: {}",
            response.chat_session_id.as_deref().unwrap_or("không có")
        );
        println!("[debug] history_count: {}", response.debug.history_count);
        println!(
            "[debug] compacted: {} | mode: {} | compacted_turns: {} | retained: {}/{} | chars: {} -> {}",
            yes_no(response.debug.compacted),
            response.debug.compact_mode.as_deref().unwrap_or("không"),
            response.debug.compacted_turns,
            response.debug.retained_history_count,
            response.debug.original_history_count,
            response.debug.estimated_chars_before,
            response.debug.estimated_chars_after,
        );
        println!(
            "[debug] available_tools ({}): {}",
            response.debug.available_tools.len(),
            if response.debug.available_tools.is_empty() {
                "không có".to_string()
            } else {
                response.debug.available_tools.join(", ")
            }
        );
        if !response.debug.tool_runtime_warnings.is_empty() {
            println!("[debug] tool_runtime_warnings:");
            for warning in &response.debug.tool_runtime_warnings {
                println!("- {}", warning);
            }
        }
        if !response.debug.tool_calls.is_empty() {
            println!("[debug] tool_calls:");
            for call in &response.debug.tool_calls {
                println!(
                    "- {} | source={} | status={} | input={} | output={} ",
                    call.name, call.source, call.status, call.input, call.output_preview,
                );
            }
        }
        println!("[debug] system_prompt:\n{}", response.debug.system_prompt);
        if let Some(context_preview) = response.debug.context_preview.as_deref() {
            println!("[debug] context_preview:\n{}", context_preview);
        }
        if let Some(compact_summary_preview) = response.debug.compact_summary_preview.as_deref() {
            println!(
                "[debug] compact_summary_preview:\n{}",
                compact_summary_preview
            );
        }
    }
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
