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
use tokio::sync::Mutex;
use tracing::warn;

use crate::config::{ProviderConfig, ProvidersConfig, ToolingConfig};

use super::super::{
    models::{
        AgentRole, ChatTurn, DebugAgentChatDebug, DebugAgentChatResponse, ProviderStatusView,
    },
    skills::SkillRegistry,
    tool_runtime::runtime::ToolRuntime,
};
use super::{
    capabilities::{AgentCapabilityProfile, CapabilityCatalog},
    history::{
        collapse_whitespace, compact_history_for_provider, looks_like_context_limit_error,
        normalize_chat_session_id, normalize_history, truncate_chars, CompactMode,
        HistoryCompaction,
    },
    prompt::{build_system_prompt, resolve_system_prompt_log_path},
    protocols::call_provider,
};

const CHAT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct ProviderHub {
    client: Client,
    config: ProvidersConfig,
    capabilities: CapabilityCatalog,
    chat_sessions: Arc<Mutex<HashMap<String, CachedChatRuntime>>>,
    system_prompt_log: Arc<std::sync::Mutex<File>>,
    system_prompt_log_path: PathBuf,
}

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
        if !provider_is_configured(&config) {
            bail!(
                "provider {} chưa được cấu hình đầy đủ, hãy kiểm tra base_url, model và API key nếu backend đó yêu cầu",
                provider.name()
            );
        }
        let api_key = provider_api_key(&config);

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

        let mut content = call_provider(
            &self.client,
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

                    match call_provider(
                        &self.client,
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
}

fn cached_tool_runtime_key(role: AgentRole, chat_session_id: &str) -> String {
    format!("{}::{chat_session_id}", role.as_str())
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
