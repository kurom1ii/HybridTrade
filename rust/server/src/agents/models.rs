use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    Connected,
    AgentThinking {
        model: String,
    },
    ThinkingStart,
    ThinkingDelta {
        text: String,
    },
    TextStart,
    TextDelta {
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
        session_id: String,
        mission: String,
        members: Vec<String>,
    },
    TeamMemberStarted {
        member: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Kuromi,
    User,
}

impl AgentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kuromi => "kuromi",
            Self::User => "user",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Kuromi => "Kuromi Finance",
            Self::User => "User",
        }
    }

    pub fn visible_agents() -> &'static [Self] {
        &[Self::Kuromi]
    }

    pub fn matches_stored_role(self, value: &str) -> bool {
        let normalized = value.trim().to_ascii_lowercase();

        match self {
            Self::Kuromi => matches!(
                normalized.as_str(),
                "kuromi" | "kuromi_finance" | "kuromi-finance" | "coordinator"
            ),
            Self::User => normalized == "user",
        }
    }
}

impl FromStr for AgentRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "kuromi" | "kuromi_finance" | "kuromi-finance" | "coordinator" => Ok(Self::Kuromi),
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
    pub available_commands: Vec<String>,
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
    pub chat_session_id: Option<String>,
    pub debug: DebugAgentChatDebug,
}

#[cfg(test)]
mod tests {
    use super::AgentRole;
    use std::str::FromStr;

    #[test]
    fn parses_kuromi_aliases() {
        assert_eq!(AgentRole::from_str("kuromi").unwrap(), AgentRole::Kuromi);
        assert_eq!(
            AgentRole::from_str("kuromi_finance").unwrap(),
            AgentRole::Kuromi
        );
        assert_eq!(
            AgentRole::from_str("coordinator").unwrap(),
            AgentRole::Kuromi
        );
    }

    #[test]
    fn matches_legacy_stored_role_names() {
        assert!(AgentRole::Kuromi.matches_stored_role("kuromi"));
        assert!(AgentRole::Kuromi.matches_stored_role("coordinator"));
        assert!(!AgentRole::Kuromi.matches_stored_role("source_scout"));
    }
}
