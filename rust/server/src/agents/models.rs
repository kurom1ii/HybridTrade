use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Coordinator,
    SourceScout,
    TechnicalAnalyst,
    EvidenceVerifier,
    ReportSynthesizer,
    User,
}

impl AgentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Coordinator => "coordinator",
            Self::SourceScout => "source_scout",
            Self::TechnicalAnalyst => "technical_analyst",
            Self::EvidenceVerifier => "evidence_verifier",
            Self::ReportSynthesizer => "report_synthesizer",
            Self::User => "user",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Coordinator => "Kuromi Finance",
            Self::SourceScout => "Agent 1",
            Self::TechnicalAnalyst => "Agent 2",
            Self::EvidenceVerifier => "Agent 3",
            Self::ReportSynthesizer => "Agent 4",
            Self::User => "User",
        }
    }

    pub fn team() -> &'static [Self] {
        &[
            Self::Coordinator,
            Self::SourceScout,
            Self::TechnicalAnalyst,
            Self::EvidenceVerifier,
            Self::ReportSynthesizer,
        ]
    }
}

impl FromStr for AgentRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "coordinator" => Ok(Self::Coordinator),
            "source_scout" => Ok(Self::SourceScout),
            "technical_analyst" => Ok(Self::TechnicalAnalyst),
            "evidence_verifier" => Ok(Self::EvidenceVerifier),
            "report_synthesizer" => Ok(Self::ReportSynthesizer),
            "user" => Ok(Self::User),
            other => Err(format!("vai trò agent không hợp lệ: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusView {
    pub name: String,
    pub enabled: bool,
    pub configured: bool,
    pub model: String,
    pub default_for_chat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAgentView {
    pub role: String,
    pub label: String,
    pub status: String,
    pub providers: Vec<String>,
    pub default_provider: String,
    pub common_skills: Vec<String>,
    pub agent_skills: Vec<String>,
    pub skill_tools: Vec<String>,
    pub mcp_servers: Vec<DebugMcpServerView>,
    pub native_tools: Vec<DebugToolView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugMcpServerView {
    pub name: String,
    pub description: String,
    pub timeout_ms: u64,
    pub command: String,
    pub args: Vec<String>,
    pub shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugToolView {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub timeout_ms: u64,
    pub shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAgentChatRequest {
    pub message: String,
    pub provider: Option<String>,
    pub investigation_id: Option<String>,
    pub chat_session_id: Option<String>,
    #[serde(default)]
    pub history: Vec<ChatTurn>,
    pub include_backend_context: Option<bool>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAgentChatDebug {
    pub system_prompt: String,
    pub context_preview: Option<String>,
    pub history_count: usize,
    pub compacted: bool,
    pub original_history_count: usize,
    pub retained_history_count: usize,
    pub compacted_turns: usize,
    pub estimated_chars_before: usize,
    pub estimated_chars_after: usize,
    pub compact_mode: Option<String>,
    pub compact_summary_preview: Option<String>,
    #[serde(default)]
    pub available_tools: Vec<String>,
    #[serde(default)]
    pub tool_runtime_warnings: Vec<String>,
    #[serde(default)]
    pub tool_calls: Vec<DebugToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugToolCall {
    pub name: String,
    pub source: String,
    pub status: String,
    #[serde(default)]
    pub input: Value,
    pub output_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAgentChatResponse {
    pub agent_role: String,
    pub provider: String,
    pub model: String,
    pub content: String,
    pub investigation_id: Option<String>,
    pub chat_session_id: Option<String>,
    pub debug: DebugAgentChatDebug,
}
