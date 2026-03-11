use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::warn;

use crate::{
    config::{McpServerConfig, NativeToolConfig, ProviderConfig, ProvidersConfig, ToolingConfig},
    models::{
        AgentRole, ChatTurn, DebugAgentChatDebug, DebugAgentChatResponse, DebugMcpServerView,
        DebugToolView, ProviderStatusView,
    },
    skills::SkillRegistry,
    tool_runtime::ToolRuntime,
};

#[derive(Clone)]
pub struct ProviderHub {
    client: Client,
    config: ProvidersConfig,
    capabilities: CapabilityCatalog,
    chat_sessions: Arc<Mutex<HashMap<String, CachedChatRuntime>>>,
    system_prompt_log: Arc<std::sync::Mutex<File>>,
    system_prompt_log_path: PathBuf,
}

const CHAT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);
const DEFAULT_SYSTEM_PROMPT_LOG_PATH: &str = "./logs/agent-system-prompts.log";

#[derive(Clone)]
struct CachedChatRuntime {
    runtime: Arc<Mutex<ToolRuntime>>,
    last_used: Instant,
}

#[derive(Debug, Clone)]
pub struct AgentPromptContext {
    pub investigation_id: Option<String>,
    pub preview: Option<String>,
}

pub struct AgentChatOptions {
    pub provider: Option<String>,
    pub chat_session_id: Option<String>,
    pub history: Vec<ChatTurn>,
    pub message: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub context: Option<AgentPromptContext>,
}

#[derive(Debug, Clone, Copy)]
enum ProviderKind {
    OpenAi,
    Anthropic,
}

#[derive(Debug, Clone, Copy)]
enum CompactMode {
    Normal,
    Aggressive,
}

#[derive(Debug, Clone, Copy)]
struct CompactSettings {
    threshold_chars: usize,
    target_chars: usize,
    summary_chars: usize,
    keep_recent_turns: usize,
}

#[derive(Debug, Clone)]
struct HistoryCompaction {
    system_prompt: String,
    history: Vec<ChatTurn>,
    debug: HistoryCompactionDebugData,
}

#[derive(Debug, Clone)]
struct HistoryCompactionDebugData {
    compacted: bool,
    original_history_count: usize,
    retained_history_count: usize,
    compacted_turns: usize,
    estimated_chars_before: usize,
    estimated_chars_after: usize,
    compact_mode: Option<&'static str>,
    compact_summary_preview: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentCapabilityProfile {
    pub common_skills: Vec<String>,
    pub agent_skills: Vec<String>,
    pub skill_tools: Vec<String>,
    pub mcp_servers: Vec<DebugMcpServerView>,
    pub native_tools: Vec<DebugToolView>,
}

#[derive(Debug, Clone)]
struct AgentPromptProfile {
    common_markdown: String,
    agent_markdown: String,
}

#[derive(Clone)]
struct CapabilityCatalog {
    skills: SkillRegistry,
    skill_tools: Vec<String>,
    mcp_servers: Vec<McpServerConfig>,
    native_tools: Vec<NativeToolConfig>,
}

impl ProviderKind {
    fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            other => bail!("provider không được hỗ trợ: {other}"),
        }
    }
}

impl CompactMode {
    fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Aggressive => "aggressive",
        }
    }
}

impl HistoryCompaction {
    fn unchanged(system_prompt: &str, history: &[ChatTurn], message: &str) -> Self {
        let estimated_chars = estimate_conversation_chars(system_prompt, history, message);

        Self {
            system_prompt: system_prompt.to_string(),
            history: history.to_vec(),
            debug: HistoryCompactionDebugData {
                compacted: false,
                original_history_count: history.len(),
                retained_history_count: history.len(),
                compacted_turns: 0,
                estimated_chars_before: estimated_chars,
                estimated_chars_after: estimated_chars,
                compact_mode: None,
                compact_summary_preview: None,
            },
        }
    }

    fn is_more_compact_than(&self, other: &Self) -> bool {
        self.debug.estimated_chars_after < other.debug.estimated_chars_after
            || self.debug.retained_history_count < other.debug.retained_history_count
    }
}

impl CapabilityCatalog {
    fn new(tooling: ToolingConfig, skills: SkillRegistry) -> Self {
        Self {
            skills,
            skill_tools: tooling.skill_tools,
            mcp_servers: tooling
                .mcp_servers
                .into_iter()
                .filter(|server| server.enabled)
                .collect(),
            native_tools: tooling
                .native_tools
                .into_iter()
                .filter(|tool| tool.enabled)
                .collect(),
        }
    }

    fn profile_for(&self, role: AgentRole) -> AgentCapabilityProfile {
        AgentCapabilityProfile {
            common_skills: self.skills.common_titles(),
            agent_skills: self.skills.agent_titles(role),
            skill_tools: self.skill_tools.clone(),
            mcp_servers: self.mcp_servers_for(role),
            native_tools: self.native_tools_for(role),
        }
    }

    fn prompt_profile_for(&self, role: AgentRole) -> AgentPromptProfile {
        AgentPromptProfile {
            common_markdown: self.skills.common_markdown(),
            agent_markdown: self.skills.agent_markdown(role),
        }
    }

    fn mcp_servers_for(&self, role: AgentRole) -> Vec<DebugMcpServerView> {
        self.mcp_servers
            .iter()
            .filter(|server| mcp_server_allowed_for_role(server, role))
            .map(|server| DebugMcpServerView {
                name: server.name.clone(),
                description: describe_mcp_server(server),
                timeout_ms: server.timeout_ms,
                command: server.command.clone(),
                args: server.args.clone(),
                shared: server.allowed_agents.is_empty(),
            })
            .collect()
    }

    fn native_tools_for(&self, role: AgentRole) -> Vec<DebugToolView> {
        self.native_tools
            .iter()
            .filter(|tool| tool_allowed_for_role(tool, role))
            .map(|tool| DebugToolView {
                name: tool.name.clone(),
                kind: tool.kind.clone(),
                description: describe_native_tool(tool),
                timeout_ms: tool.timeout_ms,
                shared: tool.allowed_agents.is_empty(),
            })
            .collect()
    }

    async fn tool_runtime_for(
        &self,
        role: AgentRole,
        history: &[ChatTurn],
        context_preview: Option<String>,
        http_client: Client,
    ) -> ToolRuntime {
        let mcp_servers = self
            .mcp_servers
            .iter()
            .filter(|server| mcp_server_allowed_for_role(server, role))
            .cloned()
            .collect();
        let native_tools = self
            .native_tools
            .iter()
            .filter(|tool| tool_allowed_for_role(tool, role))
            .cloned()
            .collect();

        ToolRuntime::bootstrap(
            mcp_servers,
            native_tools,
            history.to_vec(),
            context_preview,
            http_client,
        )
        .await
    }
}

impl ProviderHub {
    pub fn new(
        config: ProvidersConfig,
        tooling: ToolingConfig,
        skills: SkillRegistry,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .context("cannot build provider client")?;
        let system_prompt_log_path = resolve_system_prompt_log_path();
        if let Some(parent) = system_prompt_log_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("cannot create system prompt log directory {:?}", parent)
            })?;
        }
        let system_prompt_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&system_prompt_log_path)
            .with_context(|| {
                format!(
                    "cannot open system prompt log file {:?}",
                    system_prompt_log_path
                )
            })?;

        Ok(Self {
            client,
            config,
            capabilities: CapabilityCatalog::new(tooling, skills),
            chat_sessions: Arc::new(Mutex::new(HashMap::new())),
            system_prompt_log: Arc::new(std::sync::Mutex::new(system_prompt_log)),
            system_prompt_log_path,
        })
    }

    pub fn default_provider_name(&self) -> String {
        self.config.default.chat.to_ascii_lowercase()
    }

    pub fn available_provider_names(&self) -> Vec<String> {
        self.provider_statuses()
            .into_iter()
            .filter(|item| item.enabled && item.configured)
            .map(|item| item.name)
            .collect()
    }

    pub fn provider_statuses(&self) -> Vec<ProviderStatusView> {
        let default_name = self.default_provider_name();
        [
            (ProviderKind::OpenAi, &self.config.openai),
            (ProviderKind::Anthropic, &self.config.anthropic),
        ]
        .into_iter()
        .map(|(kind, cfg)| ProviderStatusView {
            name: kind.name().to_string(),
            enabled: cfg.enabled,
            configured: cfg.enabled && cfg.is_configured(),
            model: cfg.model.clone(),
            default_for_chat: default_name == kind.name(),
        })
        .collect()
    }

    pub fn agent_capabilities(&self, role: AgentRole) -> AgentCapabilityProfile {
        self.capabilities.profile_for(role)
    }

    fn log_system_prompt_attempt(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        investigation_id: Option<&str>,
        chat_session_id: Option<&str>,
        message: &str,
        compacted: &HistoryCompaction,
        attempt: &str,
    ) {
        if let Err(error) = self.append_system_prompt_log(
            agent_role,
            provider,
            model,
            investigation_id,
            chat_session_id,
            message,
            compacted,
            attempt,
        ) {
            warn!(
                error = %error,
                path = %self.system_prompt_log_path.display(),
                agent_role = agent_role.as_str(),
                provider = provider.name(),
                "không thể ghi system prompt ra file log"
            );
        }
    }

    fn append_system_prompt_log(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        investigation_id: Option<&str>,
        chat_session_id: Option<&str>,
        message: &str,
        compacted: &HistoryCompaction,
        attempt: &str,
    ) -> Result<()> {
        let timestamp = chrono::Local::now().to_rfc3339();
        let compact_mode = compacted.debug.compact_mode.unwrap_or("none");
        let message_preview = truncate_chars(&collapse_whitespace(message), 240);

        let mut file = self
            .system_prompt_log
            .lock()
            .map_err(|_| anyhow!("system prompt log mutex bị poisoned"))?;

        writeln!(file, "===== agent_system_prompt =====")?;
        writeln!(file, "timestamp: {timestamp}")?;
        writeln!(file, "agent_role: {}", agent_role.as_str())?;
        writeln!(file, "provider: {}", provider.name())?;
        writeln!(file, "model: {model}")?;
        writeln!(file, "attempt: {attempt}")?;
        writeln!(
            file,
            "investigation_id: {}",
            investigation_id.unwrap_or("-")
        )?;
        writeln!(file, "chat_session_id: {}", chat_session_id.unwrap_or("-"))?;
        writeln!(
            file,
            "history_count: {}",
            compacted.debug.retained_history_count
        )?;
        writeln!(file, "compacted: {}", compacted.debug.compacted)?;
        writeln!(file, "compact_mode: {compact_mode}")?;
        writeln!(
            file,
            "chars_before_after: {} -> {}",
            compacted.debug.estimated_chars_before, compacted.debug.estimated_chars_after,
        )?;
        writeln!(file, "message_preview: {message_preview}")?;
        writeln!(file, "system_prompt:")?;
        writeln!(file, "{}", compacted.system_prompt)?;
        writeln!(file, "===== end_agent_system_prompt =====")?;
        writeln!(file)?;
        file.flush()?;
        Ok(())
    }

    async fn cached_tool_runtime_for(
        &self,
        agent_role: AgentRole,
        chat_session_id: &str,
        history: &[ChatTurn],
        context_preview: Option<String>,
    ) -> Arc<Mutex<ToolRuntime>> {
        let now = Instant::now();
        let cache_key = cached_tool_runtime_key(agent_role, chat_session_id);

        {
            let mut sessions = self.chat_sessions.lock().await;
            sessions.retain(|_, entry| now.duration_since(entry.last_used) <= CHAT_SESSION_TTL);

            if let Some(entry) = sessions.get_mut(&cache_key) {
                entry.last_used = now;
                return entry.runtime.clone();
            }
        }

        let runtime = Arc::new(Mutex::new(
            self.capabilities
                .tool_runtime_for(agent_role, history, context_preview, self.client.clone())
                .await,
        ));

        let mut sessions = self.chat_sessions.lock().await;
        sessions.insert(
            cache_key,
            CachedChatRuntime {
                runtime: runtime.clone(),
                last_used: now,
            },
        );
        runtime
    }

    async fn run_chat_with_runtime(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        config: &ProviderConfig,
        api_key: Option<&str>,
        context: Option<&AgentPromptContext>,
        context_preview: Option<String>,
        investigation_id: Option<String>,
        chat_session_id: Option<String>,
        history: &[ChatTurn],
        message: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tool_runtime: &mut ToolRuntime,
    ) -> Result<DebugAgentChatResponse> {
        let prompt_profile = self.capabilities.prompt_profile_for(agent_role);
        let system_prompt = build_system_prompt(agent_role, context, &prompt_profile);
        let mut compacted = compact_history_for_provider(
            config,
            &system_prompt,
            history,
            message,
            CompactMode::Normal,
        );

        self.log_system_prompt_attempt(
            agent_role,
            provider,
            &config.model,
            investigation_id.as_deref(),
            chat_session_id.as_deref(),
            message,
            &compacted,
            "normal",
        );

        let mut content = self
            .call_provider(
                provider,
                config,
                api_key,
                &compacted.system_prompt,
                &compacted.history,
                message,
                max_tokens,
                temperature,
                tool_runtime,
            )
            .await;

        if let Err(error) = content {
            if looks_like_context_limit_error(&error.to_string()) {
                let retry_compacted = compact_history_for_provider(
                    config,
                    &system_prompt,
                    history,
                    message,
                    CompactMode::Aggressive,
                );

                if retry_compacted.is_more_compact_than(&compacted) {
                    self.log_system_prompt_attempt(
                        agent_role,
                        provider,
                        &config.model,
                        investigation_id.as_deref(),
                        chat_session_id.as_deref(),
                        message,
                        &retry_compacted,
                        "aggressive_retry",
                    );

                    match self
                        .call_provider(
                            provider,
                            config,
                            api_key,
                            &retry_compacted.system_prompt,
                            &retry_compacted.history,
                            message,
                            max_tokens,
                            temperature,
                            tool_runtime,
                        )
                        .await
                    {
                        Ok(value) => {
                            compacted = retry_compacted;
                            content = Ok(value);
                        }
                        Err(retry_error) => {
                            return Err(retry_error.context(
                                "provider vẫn lỗi sau khi backend auto-compact mạnh hơn một lần",
                            ));
                        }
                    }
                } else {
                    return Err(error);
                }
            } else {
                return Err(error);
            }
        }

        let content = content?;

        Ok(DebugAgentChatResponse {
            agent_role: agent_role.as_str().to_string(),
            provider: provider.name().to_string(),
            model: config.model.clone(),
            content,
            investigation_id,
            chat_session_id,
            debug: DebugAgentChatDebug {
                system_prompt: compacted.system_prompt,
                context_preview,
                history_count: compacted.debug.retained_history_count,
                compacted: compacted.debug.compacted,
                original_history_count: compacted.debug.original_history_count,
                retained_history_count: compacted.debug.retained_history_count,
                compacted_turns: compacted.debug.compacted_turns,
                estimated_chars_before: compacted.debug.estimated_chars_before,
                estimated_chars_after: compacted.debug.estimated_chars_after,
                compact_mode: compacted.debug.compact_mode.map(str::to_string),
                compact_summary_preview: compacted.debug.compact_summary_preview,
                available_tools: tool_runtime.available_tool_names(),
                tool_runtime_warnings: tool_runtime.initialization_warnings().to_vec(),
                tool_calls: tool_runtime.tool_calls().to_vec(),
            },
        })
    }

    pub async fn chat(
        &self,
        agent_role: AgentRole,
        options: AgentChatOptions,
    ) -> Result<DebugAgentChatResponse> {
        let AgentChatOptions {
            provider,
            chat_session_id,
            history,
            message,
            max_tokens,
            temperature,
            context,
        } = options;

        let provider = self.resolve_provider(provider.as_deref())?;
        let config = self.provider_config(provider).clone();
        if !config.enabled {
            bail!("provider {} hiện đang tắt", provider.name());
        }
        if !config.is_configured() {
            bail!(
                "provider {} chưa được cấu hình đầy đủ, hãy kiểm tra base_url, model và API key nếu backend đó yêu cầu",
                provider.name()
            );
        }
        let api_key = config.api_key();

        let investigation_id = context
            .as_ref()
            .and_then(|item| item.investigation_id.clone());
        let context_preview = context.as_ref().and_then(|item| item.preview.clone());
        let normalized_history = normalize_history(&history);
        let chat_session_id = normalize_chat_session_id(chat_session_id);

        if let Some(session_id) = chat_session_id.clone() {
            let runtime = self
                .cached_tool_runtime_for(
                    agent_role,
                    &session_id,
                    &normalized_history,
                    context_preview.clone(),
                )
                .await;
            let mut tool_runtime = runtime.lock().await;
            tool_runtime.prepare_turn(&normalized_history, context_preview.clone());

            return self
                .run_chat_with_runtime(
                    agent_role,
                    provider,
                    &config,
                    api_key.as_deref(),
                    context.as_ref(),
                    context_preview,
                    investigation_id,
                    Some(session_id),
                    &normalized_history,
                    &message,
                    max_tokens,
                    temperature,
                    &mut tool_runtime,
                )
                .await;
        }

        let mut tool_runtime = self
            .capabilities
            .tool_runtime_for(
                agent_role,
                &normalized_history,
                context_preview.clone(),
                self.client.clone(),
            )
            .await;

        self.run_chat_with_runtime(
            agent_role,
            provider,
            &config,
            api_key.as_deref(),
            context.as_ref(),
            context_preview,
            investigation_id,
            None,
            &normalized_history,
            &message,
            max_tokens,
            temperature,
            &mut tool_runtime,
        )
        .await
    }

    fn resolve_provider(&self, provider_name: Option<&str>) -> Result<ProviderKind> {
        match provider_name {
            Some(name) if !name.trim().is_empty() => ProviderKind::parse(name),
            _ => ProviderKind::parse(&self.config.default.chat),
        }
    }

    fn provider_config(&self, provider: ProviderKind) -> &ProviderConfig {
        match provider {
            ProviderKind::OpenAi => &self.config.openai,
            ProviderKind::Anthropic => &self.config.anthropic,
        }
    }

    async fn call_provider(
        &self,
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
                self.call_openai(
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
                self.call_anthropic(
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
        }
    }

    async fn call_openai(
        &self,
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

            let request = self
                .client
                .post(format!(
                    "{}/responses",
                    config.base_url.trim_end_matches('/')
                ))
                .json(&payload);

            let response = if let Some(api_key) = api_key {
                request.bearer_auth(api_key)
            } else {
                request
            }
            .send()
            .await
            .context("gọi OpenAI Responses API thất bại")?;

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
        &self,
        config: &ProviderConfig,
        api_key: Option<&str>,
        system_prompt: &str,
        history: &[ChatTurn],
        message: &str,
        max_tokens: Option<u32>,
        _temperature: Option<f32>,
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

            let response = send_anthropic_request(
                &self.client,
                config.base_url.trim_end_matches('/'),
                api_key,
                &payload,
            )
            .await?;

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
}

impl ProviderConfig {
    fn api_key(&self) -> Option<String> {
        if self.api_key_env.trim().is_empty() {
            return None;
        }
        std::env::var(&self.api_key_env)
            .ok()
            .filter(|value| !value.trim().is_empty())
    }

    fn is_configured(&self) -> bool {
        if self.base_url.trim().is_empty() || self.model.trim().is_empty() {
            return false;
        }

        if self.api_key_env.trim().is_empty() {
            return true;
        }

        self.api_key().is_some()
    }

    fn compact_settings(&self, mode: CompactMode) -> CompactSettings {
        let threshold_chars = self.compact_threshold_chars.max(4_000);
        let target_chars = self.compact_target_chars.max(2_000).min(threshold_chars);
        let summary_chars = self.compact_summary_chars.max(400);
        let keep_recent_turns = self.compact_keep_recent_turns.max(2);

        match mode {
            CompactMode::Normal => CompactSettings {
                threshold_chars,
                target_chars,
                summary_chars,
                keep_recent_turns,
            },
            CompactMode::Aggressive => CompactSettings {
                threshold_chars: 0,
                target_chars: (target_chars / 2).max(4_000),
                summary_chars: (summary_chars / 2).max(800),
                keep_recent_turns: (keep_recent_turns / 2).max(2),
            },
        }
    }
}

fn build_system_prompt(
    role: AgentRole,
    context: Option<&AgentPromptContext>,
    prompt_profile: &AgentPromptProfile,
) -> String {
    let context_block = context
        .and_then(|item| item.preview.as_ref())
        .map(|preview| format!("\n\nNgữ cảnh backend:\n{}", preview))
        .unwrap_or_default();

    let common_skills = render_markdown_block(
        &prompt_profile.common_markdown,
        "# Skills chung\n\nChưa có file Markdown nào trong `.skills/common`.",
    );
    let agent_skills = render_markdown_block(
        &prompt_profile.agent_markdown,
        &format!(
            "# Skill riêng {}\n\n- Bạn là {}. Trả lời ngắn, rõ và đúng vai trò.\n- Hiện chưa có file Markdown riêng trong `.skills/agents` cho role này.",
            role.as_str(),
            role.label()
        )
    );
    format!(
        r#"Bạn đang chạy trong backend HybridTrade ở chế độ chat debug.

Bạn là agent `{role_name}` ({role_label}). Trả lời ngắn, rõ, đúng vai trò và ưu tiên thông tin phục vụ debug.{context_block}

Tài liệu kỹ năng chung nạp từ `.skills/common`:
{common_skills}

Tài liệu kỹ năng riêng của agent nạp từ `.skills/agents`:
{agent_skills}

Quy tắc:
- Không bịa. Nếu context chưa đủ, nói rõ cần thêm gì.
- Chỉ dùng skill từ Markdown đã nạp và tool thực sự được runtime cấp riêng cho lượt hiện tại, không tự bịa skill nội bộ.
- Nếu user hỏi bạn có tool/MCP gì, chỉ trả lời theo capability thật sự đang được cấp ở runtime hiện tại.
- Nếu runtime đã nạp được tool phù hợp và user yêu cầu hành động trực tiếp như mở URL, xem DOM, network, console hoặc kiểm tra page, hãy gọi tool ngay trong lượt hiện tại thay vì chỉ mô tả kế hoạch.
- Bạn chỉ được nói một tool đã được chạy khi trong ngữ cảnh có kết quả thực thi thật.
- Nếu tool thất bại, nêu ngắn gọn lỗi thật và nguyên nhân khả dĩ thay vì xin xác nhận lại không cần thiết.
- Khi cần debug frontend hoặc browser state, ưu tiên đề xuất CDP trước."#,
        role_name = role.as_str(),
        role_label = role.label(),
    )
}

fn render_markdown_block(markdown: &str, fallback: &str) -> String {
    let markdown = markdown.trim();
    if markdown.is_empty() {
        fallback.to_string()
    } else {
        markdown.to_string()
    }
}

fn describe_mcp_server(server: &McpServerConfig) -> String {
    match server.name.trim().to_ascii_lowercase().as_str() {
        "chrome-devtools" => {
            "Chrome DevTools Protocol để xem DOM, console, network, performance và trạng thái trình duyệt".to_string()
        }
        _ => format!("MCP server {}", server.name),
    }
}

fn describe_native_tool(tool: &NativeToolConfig) -> String {
    match tool.name.trim().to_ascii_lowercase().as_str() {
        "fetch_page" => "Lấy nội dung page hoặc seed URL để phục vụ rà nguồn".to_string(),
        "extract_signals" => "Rút tín hiệu kỹ thuật từ dữ liệu đã có".to_string(),
        "memory_lookup" => "Tra lại ngữ cảnh/lịch sử để đối chiếu khi debug".to_string(),
        "summarize_sources" => "Tóm tắt danh sách nguồn và các điểm chính".to_string(),
        "read" => "Đọc file trong workspace backend theo path được kiểm soát".to_string(),
        "write" => "Ghi hoặc append file text trong workspace backend".to_string(),
        "exec" => "Chạy executable trực tiếp trong workspace backend".to_string(),
        "bash" => "Chạy lệnh bash trong workspace backend với timeout cấu hình".to_string(),
        _ => format!("Native tool {}", tool.name),
    }
}

fn mcp_server_allowed_for_role(server: &McpServerConfig, role: AgentRole) -> bool {
    if server.allowed_agents.is_empty() {
        return true;
    }

    server
        .allowed_agents
        .iter()
        .any(|agent| agent.trim().eq_ignore_ascii_case(role.as_str()))
}

fn tool_allowed_for_role(tool: &NativeToolConfig, role: AgentRole) -> bool {
    if tool.allowed_agents.is_empty() {
        return true;
    }

    tool.allowed_agents
        .iter()
        .any(|agent| agent.trim().eq_ignore_ascii_case(role.as_str()))
}

fn normalize_history(history: &[ChatTurn]) -> Vec<ChatTurn> {
    history
        .iter()
        .filter_map(|turn| {
            let role = turn.role.trim().to_ascii_lowercase();
            if !(role == "user" || role == "assistant") {
                return None;
            }
            let content = turn.content.trim();
            if content.is_empty() {
                return None;
            }
            Some(ChatTurn {
                role,
                content: content.to_string(),
            })
        })
        .collect()
}

fn normalize_chat_session_id(chat_session_id: Option<String>) -> Option<String> {
    chat_session_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn resolve_system_prompt_log_path() -> PathBuf {
    std::env::var("HYBRIDTRADE_SYSTEM_PROMPT_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SYSTEM_PROMPT_LOG_PATH))
}

fn cached_tool_runtime_key(role: AgentRole, chat_session_id: &str) -> String {
    format!("{}::{chat_session_id}", role.as_str())
}

fn compact_history_for_provider(
    config: &ProviderConfig,
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
    mode: CompactMode,
) -> HistoryCompaction {
    if history.is_empty() {
        return HistoryCompaction::unchanged(system_prompt, history, message);
    }

    let settings = config.compact_settings(mode);
    let estimated_before = estimate_conversation_chars(system_prompt, history, message);
    if estimated_before <= settings.threshold_chars {
        return HistoryCompaction::unchanged(system_prompt, history, message);
    }

    let max_keep_recent = history.len().min(settings.keep_recent_turns);
    let mut fallback = None;

    for keep_recent in (0..=max_keep_recent).rev() {
        let split_index = history.len().saturating_sub(keep_recent);
        let older = &history[..split_index];
        let recent = history[split_index..].to_vec();
        let base_chars = estimate_conversation_chars(system_prompt, &recent, message);
        let summary_budget = settings
            .target_chars
            .saturating_sub(base_chars)
            .min(settings.summary_chars);
        let compact_summary = summarize_compacted_turns(older, summary_budget);
        let compacted_system_prompt = append_compact_summary(system_prompt, &compact_summary);
        let estimated_after =
            estimate_conversation_chars(&compacted_system_prompt, &recent, message);

        let candidate = HistoryCompaction {
            system_prompt: compacted_system_prompt,
            history: recent,
            debug: HistoryCompactionDebugData {
                compacted: true,
                original_history_count: history.len(),
                retained_history_count: keep_recent,
                compacted_turns: history.len().saturating_sub(keep_recent),
                estimated_chars_before: estimated_before,
                estimated_chars_after: estimated_after,
                compact_mode: Some(mode.label()),
                compact_summary_preview: (!compact_summary.is_empty())
                    .then(|| truncate_chars(&compact_summary, 240)),
            },
        };

        if estimated_after <= settings.target_chars || keep_recent == 0 {
            return candidate;
        }

        fallback = Some(candidate);
    }

    fallback.unwrap_or_else(|| HistoryCompaction::unchanged(system_prompt, history, message))
}

fn estimate_conversation_chars(system_prompt: &str, history: &[ChatTurn], message: &str) -> usize {
    char_count(system_prompt)
        + char_count(message)
        + history
            .iter()
            .map(|turn| char_count(&turn.role) + char_count(&turn.content) + 24)
            .sum::<usize>()
        + 64
}

fn summarize_compacted_turns(turns: &[ChatTurn], max_chars: usize) -> String {
    if turns.is_empty() || max_chars < 80 {
        return String::new();
    }

    let mut summary = format!("{} lượt chat cũ đã được compact:\n", turns.len());
    let mut included = 0usize;

    for turn in turns {
        let role_label = if turn.role == "user" {
            "User"
        } else {
            "Assistant"
        };
        let cleaned = collapse_whitespace(&turn.content);
        let line = format!("- {}: {}\n", role_label, truncate_chars(&cleaned, 180));
        if char_count(&(summary.clone() + &line)) > max_chars {
            break;
        }
        summary.push_str(&line);
        included += 1;
    }

    if included < turns.len() {
        let line = format!(
            "- ... còn {} lượt cũ hơn đã được rút gọn thêm.\n",
            turns.len() - included
        );
        if char_count(&(summary.clone() + &line)) <= max_chars {
            summary.push_str(&line);
        }
    }

    truncate_chars(summary.trim(), max_chars)
}

fn append_compact_summary(system_prompt: &str, compact_summary: &str) -> String {
    if compact_summary.trim().is_empty() {
        return system_prompt.to_string();
    }

    format!(
        "{}\n\nNgữ cảnh hội thoại cũ đã được compact:\n{}\n\nKhi cần tham chiếu các lượt trước, ưu tiên bám theo phần tóm tắt compact này.",
        system_prompt, compact_summary
    )
}

fn looks_like_context_limit_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "context length",
        "maximum context",
        "context window",
        "too many tokens",
        "prompt is too long",
        "input is too long",
        "message is too long",
        "token limit",
        "too long",
        "max context",
        "prompt_tokens",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if char_count(value) <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

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
                    .collect::<std::collections::HashSet<_>>();

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
    base_url: &str,
    api_key: Option<&str>,
    payload: &Value,
) -> Result<reqwest::Response> {
    let primary_url = if base_url.ends_with("/v1") {
        format!("{}/messages", base_url)
    } else {
        format!("{}/v1/messages", base_url)
    };

    let response = anthropic_request_builder(client, primary_url, api_key, payload)
        .send()
        .await
        .context("gọi Anthropic thất bại")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        let fallback_base = base_url.trim_end_matches("/v1");
        return anthropic_request_builder(
            client,
            format!("{}/messages", fallback_base),
            api_key,
            payload,
        )
        .send()
        .await
        .context("gọi Anthropic fallback /messages thất bại");
    }

    Ok(response)
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
