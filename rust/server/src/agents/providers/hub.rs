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
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

use crate::config::{ProviderConfig, ProvidersConfig, ToolingConfig};

use super::super::{
    models::{
        AgentRole, ChatStreamEvent, ChatTurn, DebugAgentChatDebug, DebugAgentChatResponse,
        DebugToolCall, ProviderStatusView,
    },
    skills::SkillRegistry,
    tool_runtime::runtime::ToolRuntime,
};
use super::{
    capabilities::{AgentCapabilityProfile, CapabilityCatalog, TurnSkillContext},
    history::{
        collapse_whitespace, compact_history_for_provider, looks_like_context_limit_error,
        normalize_chat_session_id, normalize_history, truncate_chars, CompactMode,
        HistoryCompaction,
    },
    prompt::{build_system_prompt, build_user_message, resolve_system_prompt_log_path},
    protocols::{call_provider, reset_provider_http_log_file},
    team::TeamOrchestrator,
};

const CHAT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct ProviderHub {
    client: Client,
    config: ProvidersConfig,
    capabilities: CapabilityCatalog,
    chat_sessions: Arc<Mutex<HashMap<String, CachedChatRuntime>>>,
    system_prompt_log_lock: Arc<std::sync::Mutex<()>>,
    system_prompt_log_path: PathBuf,
}

#[derive(Clone)]
struct CachedChatRuntime {
    runtime: Arc<Mutex<ToolRuntime>>,
    last_used: Instant,
}

#[derive(Debug, Clone)]
pub struct AgentPromptContext {
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
pub(super) enum ProviderKind {
    OpenAi,
    Anthropic,
}

impl ProviderKind {
    pub(super) fn name(self) -> &'static str {
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
        reset_log_file(&system_prompt_log_path, "system prompt")?;
        reset_provider_http_log_file()?;

        Ok(Self {
            client,
            config,
            capabilities: CapabilityCatalog::new(tooling, skills),
            chat_sessions: Arc::new(Mutex::new(HashMap::new())),
            system_prompt_log_lock: Arc::new(std::sync::Mutex::new(())),
            system_prompt_log_path,
        })
    }

    fn with_system_prompt_log<T>(&self, action: impl FnOnce(&mut File) -> Result<T>) -> Result<T> {
        let _guard = self
            .system_prompt_log_lock
            .lock()
            .map_err(|_| anyhow!("system prompt log mutex bị poisoned"))?;

        if let Some(parent) = self.system_prompt_log_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("cannot create system prompt log directory {:?}", parent)
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.system_prompt_log_path)
            .with_context(|| {
                format!(
                    "cannot open system prompt log file {:?}",
                    self.system_prompt_log_path
                )
            })?;

        action(&mut file)
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
            configured: cfg.enabled && provider_is_configured(cfg),
            model: cfg.model.clone(),
            default_for_chat: default_name == kind.name(),
        })
        .collect()
    }

    pub(crate) fn agent_capabilities(&self, role: AgentRole) -> AgentCapabilityProfile {
        self.capabilities.profile_for(role)
    }

    pub async fn chat(
        &self,
        agent_role: AgentRole,
        options: AgentChatOptions,
        stream_tx: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
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

        let turn_skills = self.capabilities.resolve_turn_skills(&message);

        let provider = self.resolve_provider(provider.as_deref())?;
        let config = self.provider_config(provider).clone();
        if !config.enabled {
            bail!("provider {} hiện đang tắt", provider.name());
        }
        if !provider_is_configured(&config) {
            bail!(
                "provider {} chưa được cấu hình đầy đủ, hãy kiểm tra base_url, model và API key nếu backend đó yêu cầu",
                provider.name()
            );
        }
        let api_key = provider_api_key(&config);

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

            return self
                .run_chat_with_runtime(
                    agent_role,
                    provider,
                    &config,
                    api_key.as_deref(),
                    context.as_ref(),
                    context_preview,
                    Some(session_id),
                    &turn_skills,
                    &normalized_history,
                    max_tokens,
                    temperature,
                    &mut tool_runtime,
                    stream_tx.clone(),
                )
                .await;
        }

        let mut tool_runtime = self
            .capabilities
            .tool_runtime_for(&normalized_history, context_preview.clone())
            .await;

        self.run_chat_with_runtime(
            agent_role,
            provider,
            &config,
            api_key.as_deref(),
            context.as_ref(),
            context_preview,
            None,
            &turn_skills,
            &normalized_history,
            max_tokens,
            temperature,
            &mut tool_runtime,
            stream_tx,
        )
        .await
    }

    fn log_system_prompt_attempt(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        message: &str,
        compacted: &HistoryCompaction,
        attempt: &str,
    ) {
        if let Err(error) = self.append_system_prompt_log(
            agent_role,
            provider,
            model,
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
        chat_session_id: Option<&str>,
        message: &str,
        compacted: &HistoryCompaction,
        attempt: &str,
    ) -> Result<()> {
        let timestamp = chrono::Local::now().to_rfc3339();
        let compact_mode = compacted.debug.compact_mode.unwrap_or("none");
        let message_preview = truncate_chars(&collapse_whitespace(message), 240);

        self.with_system_prompt_log(|file| {
            writeln!(file, "===== agent_system_prompt =====")?;
            writeln!(file, "timestamp: {timestamp}")?;
            writeln!(file, "agent_role: {}", agent_role.as_str())?;
            writeln!(file, "provider: {}", provider.name())?;
            writeln!(file, "model: {model}")?;
            writeln!(file, "attempt: {attempt}")?;
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
        })
    }

    fn log_user_message(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        message: &str,
        command: Option<&str>,
        active_skills: &[String],
    ) {
        if let Err(error) = self.append_user_message_log(
            agent_role,
            provider,
            model,
            chat_session_id,
            message,
            command,
            active_skills,
        ) {
            warn!(
                error = %error,
                path = %self.system_prompt_log_path.display(),
                "không thể ghi user message ra file log"
            );
        }
    }

    fn append_user_message_log(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        message: &str,
        command: Option<&str>,
        active_skills: &[String],
    ) -> Result<()> {
        let timestamp = chrono::Local::now().to_rfc3339();

        self.with_system_prompt_log(|file| {
            writeln!(file, "===== user_message =====")?;
            writeln!(file, "timestamp: {timestamp}")?;
            writeln!(file, "agent_role: {}", agent_role.as_str())?;
            writeln!(file, "provider: {}", provider.name())?;
            writeln!(file, "model: {model}")?;
            writeln!(file, "chat_session_id: {}", chat_session_id.unwrap_or("-"))?;
            writeln!(file, "command: {}", command.unwrap_or("-"))?;
            writeln!(file, "skills: [{}]", active_skills.join(", "))?;
            writeln!(file, "message:")?;
            writeln!(file, "{message}")?;
            writeln!(file, "===== end_user_message =====")?;
            writeln!(file)?;
            file.flush()?;
            Ok(())
        })
    }

    fn log_assistant_request(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        command: Option<&str>,
        active_skills: &[String],
        message: &str,
        compacted: &HistoryCompaction,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tool_runtime: &ToolRuntime,
        attempt: &str,
    ) {
        if let Err(error) = self.append_assistant_request_log(
            agent_role,
            provider,
            model,
            chat_session_id,
            command,
            active_skills,
            message,
            compacted,
            max_tokens,
            temperature,
            tool_runtime,
            attempt,
        ) {
            warn!(
                error = %error,
                path = %self.system_prompt_log_path.display(),
                "không thể ghi assistant request ra file log"
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_assistant_request_log(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        command: Option<&str>,
        active_skills: &[String],
        message: &str,
        compacted: &HistoryCompaction,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tool_runtime: &ToolRuntime,
        attempt: &str,
    ) -> Result<()> {
        let timestamp = chrono::Local::now().to_rfc3339();
        let available_tools = tool_runtime.available_tool_names();
        let rendered_history = render_history_for_log(&compacted.history);
        let compact_mode = compacted.debug.compact_mode.unwrap_or("none");

        self.with_system_prompt_log(|file| {
            writeln!(file, "===== assistant_request =====")?;
            writeln!(file, "timestamp: {timestamp}")?;
            writeln!(file, "agent_role: {}", agent_role.as_str())?;
            writeln!(file, "provider: {}", provider.name())?;
            writeln!(file, "model: {model}")?;
            writeln!(file, "attempt: {attempt}")?;
            writeln!(file, "chat_session_id: {}", chat_session_id.unwrap_or("-"))?;
            writeln!(file, "command: {}", command.unwrap_or("-"))?;
            writeln!(file, "skills: [{}]", active_skills.join(", "))?;
            writeln!(
                file,
                "max_tokens: {}",
                max_tokens
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string())
            )?;
            writeln!(
                file,
                "temperature: {}",
                temperature
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string())
            )?;
            writeln!(file, "history_count: {}", compacted.history.len())?;
            writeln!(
                file,
                "compacted: {} | mode: {} | compacted_turns: {} | retained: {}/{} | chars: {} -> {}",
                compacted.debug.compacted,
                compact_mode,
                compacted.debug.compacted_turns,
                compacted.debug.retained_history_count,
                compacted.debug.original_history_count,
                compacted.debug.estimated_chars_before,
                compacted.debug.estimated_chars_after,
            )?;
            writeln!(file, "available_tools_count: {}", available_tools.len())?;
            writeln!(file, "available_tools: [{}]", available_tools.join(", "))?;
            writeln!(file, "message:")?;
            writeln!(file, "{message}")?;
            writeln!(file, "history:")?;
            writeln!(file, "{rendered_history}")?;
            writeln!(file, "===== end_assistant_request =====")?;
            writeln!(file)?;
            file.flush()?;
            Ok(())
        })
    }

    fn log_assistant_response(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        content: &str,
        tool_runtime: &ToolRuntime,
    ) {
        if let Err(error) = self.append_assistant_response_log(
            agent_role,
            provider,
            model,
            chat_session_id,
            content,
            tool_runtime,
        ) {
            warn!(
                error = %error,
                path = %self.system_prompt_log_path.display(),
                "không thể ghi assistant response ra file log"
            );
        }
    }

    fn append_assistant_response_log(
        &self,
        agent_role: AgentRole,
        provider: ProviderKind,
        model: &str,
        chat_session_id: Option<&str>,
        content: &str,
        tool_runtime: &ToolRuntime,
    ) -> Result<()> {
        let timestamp = chrono::Local::now().to_rfc3339();
        let tool_calls = tool_runtime.tool_calls();
        let tool_names: Vec<&str> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
        let mcp_servers: Vec<&str> = tool_calls
            .iter()
            .filter(|tc| tc.source.starts_with("mcp:"))
            .map(|tc| tc.source.trim_start_matches("mcp:"))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        self.with_system_prompt_log(|file| {
            writeln!(file, "===== assistant_response =====")?;
            writeln!(file, "timestamp: {timestamp}")?;
            writeln!(file, "agent_role: {}", agent_role.as_str())?;
            writeln!(file, "provider: {}", provider.name())?;
            writeln!(file, "model: {model}")?;
            writeln!(file, "chat_session_id: {}", chat_session_id.unwrap_or("-"))?;
            writeln!(file, "tool_calls_count: {}", tool_calls.len())?;
            writeln!(file, "tool_calls: [{}]", tool_names.join(", "))?;
            writeln!(file, "mcp_servers_used: [{}]", mcp_servers.join(", "))?;
            writeln!(file, "content:")?;
            writeln!(file, "{content}")?;
            writeln!(file, "===== end_assistant_response =====")?;
            writeln!(file)?;
            file.flush()?;
            Ok(())
        })
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
                .tool_runtime_for(history, context_preview)
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
        chat_session_id: Option<String>,
        turn_skills: &TurnSkillContext,
        history: &[ChatTurn],
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tool_runtime: &mut ToolRuntime,
        stream_tx: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
    ) -> Result<DebugAgentChatResponse> {
        let provider_message =
            build_user_message(&turn_skills.clean_message, &turn_skills.active_skills);
        let active_skill_names = turn_skills
            .active_skills
            .iter()
            .map(|skill| skill.name.clone())
            .collect::<Vec<_>>();
        let effective_history = resolve_effective_history(tool_runtime.history(), history);
        let runtime_continuity_note = build_runtime_continuity_note(tool_runtime.tool_calls());

        tool_runtime.prepare_turn(&effective_history, context_preview.clone());

        // Wire stream to tool_runtime for agentic loop events
        if let Some(tx) = &stream_tx {
            tool_runtime.set_stream_sender(tx.clone());
        }

        // Team orchestrator — subagent dùng isolated browser instance riêng.
        tool_runtime.attach_team_orchestrator(self.team_orchestrator(
            provider,
            config,
            api_key,
            turn_skills.active_skills.clone(),
            stream_tx.clone(),
        ));

        let system_prompt =
            build_system_prompt(agent_role, context, runtime_continuity_note.as_deref());
        let mut compacted = compact_history_for_provider(
            config,
            &system_prompt,
            &effective_history,
            &provider_message,
            CompactMode::Normal,
        );

        self.log_system_prompt_attempt(
            agent_role,
            provider,
            &config.model,
            chat_session_id.as_deref(),
            &turn_skills.clean_message,
            &compacted,
            "normal",
        );

        self.log_user_message(
            agent_role,
            provider,
            &config.model,
            chat_session_id.as_deref(),
            &provider_message,
            turn_skills.command.as_deref(),
            &active_skill_names,
        );

        self.log_assistant_request(
            agent_role,
            provider,
            &config.model,
            chat_session_id.as_deref(),
            turn_skills.command.as_deref(),
            &active_skill_names,
            &turn_skills.clean_message,
            &compacted,
            max_tokens,
            temperature,
            tool_runtime,
            "normal",
        );

        // Emit thinking event — report main model (first call uses this)
        if let Some(tx) = &stream_tx {
            let _ = tx.send(ChatStreamEvent::AgentThinking {
                model: config.model.clone(),
            });
        }

        let mut call_result = call_provider(
            &self.client,
            provider,
            config,
            api_key,
            &compacted.system_prompt,
            &compacted.history,
            &provider_message,
            max_tokens,
            temperature,
            tool_runtime,
        )
        .await;

        if let Err(error) = call_result {
            if looks_like_context_limit_error(&error.to_string()) {
                let retry_compacted = compact_history_for_provider(
                    config,
                    &system_prompt,
                    &effective_history,
                    &provider_message,
                    CompactMode::Aggressive,
                );

                if retry_compacted.is_more_compact_than(&compacted) {
                    self.log_system_prompt_attempt(
                        agent_role,
                        provider,
                        &config.model,
                        chat_session_id.as_deref(),
                        &turn_skills.clean_message,
                        &retry_compacted,
                        "aggressive_retry",
                    );

                    self.log_assistant_request(
                        agent_role,
                        provider,
                        &config.model,
                        chat_session_id.as_deref(),
                        turn_skills.command.as_deref(),
                        &active_skill_names,
                        &turn_skills.clean_message,
                        &retry_compacted,
                        max_tokens,
                        temperature,
                        tool_runtime,
                        "aggressive_retry",
                    );

                    match call_provider(
                        &self.client,
                        provider,
                        config,
                        api_key,
                        &retry_compacted.system_prompt,
                        &retry_compacted.history,
                        &provider_message,
                        max_tokens,
                        temperature,
                        tool_runtime,
                    )
                    .await
                    {
                        Ok(value) => {
                            compacted = retry_compacted;
                            call_result = Ok(value);
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

        let content = call_result?;
        let assistant_history =
            build_assistant_history_content(&content, tool_runtime.tool_calls());
        tool_runtime.set_history(append_turn_to_history(
            &effective_history,
            &turn_skills.clean_message,
            &assistant_history,
        ));

        self.log_assistant_response(
            agent_role,
            provider,
            &config.model,
            chat_session_id.as_deref(),
            &content,
            tool_runtime,
        );

        tool_runtime.clear_stream_state();

        Ok(DebugAgentChatResponse {
            agent_role: agent_role.as_str().to_string(),
            provider: provider.name().to_string(),
            model: config.model.clone(),
            content,
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

    fn team_orchestrator(
        &self,
        provider: ProviderKind,
        config: &ProviderConfig,
        api_key: Option<&str>,
        active_skills: Vec<super::capabilities::ActiveSkill>,
        stream_tx: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
    ) -> TeamOrchestrator {
        TeamOrchestrator::new(
            self.client.clone(),
            self.capabilities.clone(),
            provider,
            config.clone(),
            api_key.map(str::to_string),
            active_skills,
            stream_tx,
        )
    }
}

fn reset_log_file(path: &PathBuf, label: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {label} log directory {:?}", parent))?;
    }

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("cannot reset {label} log file {:?}", path))?;

    Ok(())
}

fn cached_tool_runtime_key(role: AgentRole, chat_session_id: &str) -> String {
    format!("{}::{chat_session_id}", role.as_str())
}

fn render_history_for_log(history: &[ChatTurn]) -> String {
    if history.is_empty() {
        return "(empty)".to_string();
    }

    history
        .iter()
        .enumerate()
        .map(|(index, turn)| {
            format!(
                "{}. {}: {}",
                index + 1,
                turn.role,
                collapse_whitespace(&turn.content)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn resolve_effective_history(
    runtime_history: &[ChatTurn],
    request_history: &[ChatTurn],
) -> Vec<ChatTurn> {
    if runtime_history.is_empty() {
        request_history.to_vec()
    } else {
        runtime_history.to_vec()
    }
}

fn append_turn_to_history(
    history: &[ChatTurn],
    user_message: &str,
    assistant_message: &str,
) -> Vec<ChatTurn> {
    let mut updated = history.to_vec();
    updated.push(ChatTurn {
        role: "user".to_string(),
        content: user_message.trim().to_string(),
    });
    updated.push(ChatTurn {
        role: "assistant".to_string(),
        content: assistant_message.trim().to_string(),
    });
    updated
}

fn build_assistant_history_content(content: &str, tool_calls: &[DebugToolCall]) -> String {
    if tool_calls.is_empty() {
        return content.trim().to_string();
    }

    let tool_summary = tool_calls
        .iter()
        .map(|call| {
            let input =
                serde_json::to_string(&call.input).unwrap_or_else(|_| call.input.to_string());
            format!(
                "- {} [{}] | input: {} | output: {}",
                call.name,
                call.status,
                truncate_chars(&collapse_whitespace(&input), 180),
                truncate_chars(&collapse_whitespace(&call.output_preview), 320),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Kết quả tool thật trong turn này:\n{}\n\nPhản hồi gửi user:\n{}",
        truncate_chars(&tool_summary, 2_400),
        content.trim(),
    )
}

fn build_runtime_continuity_note(tool_calls: &[DebugToolCall]) -> Option<String> {
    if tool_calls.is_empty() {
        return None;
    }

    let start = tool_calls.len().saturating_sub(4);
    let lines = tool_calls[start..]
        .iter()
        .map(|call| {
            format!(
                "- {} [{}] => {}",
                call.name,
                call.status,
                truncate_chars(&collapse_whitespace(&call.output_preview), 220)
            )
        })
        .collect::<Vec<_>>();

    Some(
        "Đây là kết quả tool thật gần nhất còn lưu trong runtime; nếu user đang thao tác tiếp trên cùng flow thì nên tái sử dụng state này trước khi mở lại từ đầu:\n".to_string()
            + &lines.join("\n"),
    )
}

fn provider_api_key(config: &ProviderConfig) -> Option<String> {
    if config.api_key_env.trim().is_empty() {
        return None;
    }
    std::env::var(&config.api_key_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn provider_is_configured(config: &ProviderConfig) -> bool {
    if config.base_url.trim().is_empty() || config.model.trim().is_empty() {
        return false;
    }

    if config.api_key_env.trim().is_empty() {
        return true;
    }

    provider_api_key(config).is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        append_turn_to_history, build_assistant_history_content, resolve_effective_history,
    };
    use crate::agents::models::{ChatTurn, DebugToolCall};
    use serde_json::json;

    #[test]
    fn prefers_runtime_history_when_available() {
        let runtime_history = vec![ChatTurn {
            role: "assistant".to_string(),
            content: "tool-backed memory".to_string(),
        }];
        let request_history = vec![ChatTurn {
            role: "assistant".to_string(),
            content: "plain client history".to_string(),
        }];

        let effective = resolve_effective_history(&runtime_history, &request_history);

        assert_eq!(effective[0].content, "tool-backed memory");
    }

    #[test]
    fn assistant_history_content_embeds_tool_results() {
        let content = build_assistant_history_content(
            "Đã mở example.com.",
            &[DebugToolCall {
                name: "chrome_devtools__new_page".to_string(),
                source: "mcp:chrome-devtools".to_string(),
                status: "completed".to_string(),
                input: json!({ "url": "https://example.com" }),
                output_preview: "## Pages 1: about:blank 2: https://example.com/ [selected]"
                    .to_string(),
            }],
        );

        assert!(content.contains("Kết quả tool thật trong turn này"));
        assert!(content.contains("chrome_devtools__new_page"));
        assert!(content.contains("Đã mở example.com."));
    }

    #[test]
    fn append_turn_to_history_keeps_user_assistant_order() {
        let updated = append_turn_to_history(&[], "vào example.com", "Đã vào rồi.");

        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].role, "user");
        assert_eq!(updated[1].role, "assistant");
    }
}
