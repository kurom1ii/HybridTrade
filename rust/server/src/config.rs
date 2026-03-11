use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ConfigBundle {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub heartbeat: HeartbeatConfig,
    pub orchestration: OrchestrationConfig,
    pub providers: ProvidersConfig,
    pub tooling: ToolingConfig,
    pub schedules: Vec<ScheduleSeed>,
}

impl ConfigBundle {
    pub fn load(base_dir: &Path) -> Result<Self> {
        let app: AppFile = read_toml(base_dir.join("app.toml"))?;
        let schedules = read_optional_toml::<ScheduleFile>(base_dir.join("schedules.toml"))?
            .unwrap_or_default()
            .schedules;
        let mcp = read_optional_toml::<McpFile>(base_dir.join("mcp.toml"))?.unwrap_or_default();
        let tools =
            read_optional_toml::<ToolsFile>(base_dir.join("tools.toml"))?.unwrap_or_default();

        Ok(Self {
            server: app.server,
            database: app.database,
            heartbeat: app.heartbeat,
            orchestration: app.orchestration,
            providers: app.providers,
            tooling: ToolingConfig {
                skill_tools: mcp.skill_tools,
                mcp_servers: mcp.mcp_servers,
                native_tools: tools.tools,
            },
            schedules,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AppFile {
    server: ServerConfig,
    database: DatabaseConfig,
    heartbeat: HeartbeatConfig,
    orchestration: OrchestrationConfig,
    #[serde(default)]
    providers: ProvidersConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatConfig {
    pub api_interval_seconds: u64,
    pub api_ttl_seconds: i64,
    pub scheduler_interval_seconds: u64,
    pub scheduler_ttl_seconds: i64,
    pub agent_ttl_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrchestrationConfig {
    pub default_source_scope: String,
    pub default_priority: String,
    pub default_goal: String,
    pub default_sections: Vec<String>,
    pub max_parallel_sources: usize,
    #[serde(default)]
    pub seed_urls: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub default: DefaultProviderConfig,
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub anthropic: ProviderConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ToolingConfig {
    pub skill_tools: Vec<String>,
    pub mcp_servers: Vec<McpServerConfig>,
    pub native_tools: Vec<NativeToolConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultProviderConfig {
    #[serde(default = "default_chat_provider")]
    pub chat: String,
}

impl Default for DefaultProviderConfig {
    fn default() -> Self {
        Self {
            chat: default_chat_provider(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_compact_threshold_chars")]
    pub compact_threshold_chars: usize,
    #[serde(default = "default_compact_target_chars")]
    pub compact_target_chars: usize,
    #[serde(default = "default_compact_summary_chars")]
    pub compact_summary_chars: usize,
    #[serde(default = "default_compact_keep_recent_turns")]
    pub compact_keep_recent_turns: usize,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct McpFile {
    #[serde(default)]
    skill_tools: Vec<String>,
    #[serde(default)]
    mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ToolsFile {
    #[serde(default)]
    tools: Vec<NativeToolConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allowed_agents: Vec<String>,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NativeToolConfig {
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allowed_agents: Vec<String>,
    #[serde(default)]
    pub timeout_ms: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            model: String::new(),
            api_key_env: String::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            compact_threshold_chars: default_compact_threshold_chars(),
            compact_target_chars: default_compact_target_chars(),
            compact_summary_chars: default_compact_summary_chars(),
            compact_keep_recent_turns: default_compact_keep_recent_turns(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ScheduleFile {
    #[serde(default)]
    schedules: Vec<ScheduleSeed>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleSeed {
    pub name: String,
    pub cron_expr: String,
    pub job_type: String,
    pub enabled: bool,
    #[serde(default)]
    pub payload: Value,
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .with_context(|| format!("cannot read config {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("invalid toml {}", path.display()))
}

fn read_optional_toml<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<Option<T>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("cannot read config {}", path.display()))?;
    let parsed =
        toml::from_str(&content).with_context(|| format!("invalid toml {}", path.display()))?;
    Ok(Some(parsed))
}

fn default_chat_provider() -> String {
    "openai".to_string()
}

fn default_max_tokens() -> u32 {
    1200
}

fn default_temperature() -> f32 {
    0.2
}

fn default_compact_threshold_chars() -> usize {
    24_000
}

fn default_compact_target_chars() -> usize {
    14_000
}

fn default_compact_summary_chars() -> usize {
    3_200
}

fn default_compact_keep_recent_turns() -> usize {
    6
}
