use std::collections::HashSet;

use crate::config::{McpServerConfig, NativeToolConfig, ToolingConfig};

use super::super::{
    models::{AgentRole, ChatTurn, DebugMcpServerView, DebugToolView},
    skills::SkillRegistry,
    tool_runtime::runtime::ToolRuntime,
};

#[derive(Debug, Clone)]
pub struct AgentCapabilityProfile {
    pub available_commands: Vec<String>,
    pub mcp_servers: Vec<DebugMcpServerView>,
    pub native_tools: Vec<DebugToolView>,
}

#[derive(Debug, Clone)]
pub(super) struct ActiveSkill {
    pub(super) name: String,
    pub(super) markdown: String,
}

#[derive(Debug, Clone)]
pub(super) struct TurnSkillContext {
    pub(super) command: Option<String>,
    pub(super) clean_message: String,
    pub(super) active_skills: Vec<ActiveSkill>,
}

#[derive(Clone)]
pub(super) struct CapabilityCatalog {
    skills: SkillRegistry,
    mcp_servers: Vec<McpServerConfig>,
    native_tools: Vec<NativeToolConfig>,
}

impl CapabilityCatalog {
    pub(super) fn new(tooling: ToolingConfig, skills: SkillRegistry) -> Self {
        Self {
            skills,
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

    pub(super) fn profile_for(&self, _role: AgentRole) -> AgentCapabilityProfile {
        AgentCapabilityProfile {
            available_commands: self.skills.available_commands(),
            mcp_servers: self.mcp_servers_for(),
            native_tools: self.native_tools_for(),
        }
    }

    pub(super) fn resolve_turn_skills(&self, message: &str) -> TurnSkillContext {
        let trimmed = message.trim();
        let (command, clean_message) = if trimmed.starts_with('/') {
            let first_space = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            let candidate = &trimmed[1..first_space];
            let resolved_command = self.skills.resolve_command_name(candidate);

            if candidate.is_empty() || resolved_command.is_none() {
                (None, message.to_string())
            } else {
                let rest = trimmed[first_space..].trim();
                let clean_message = if rest.is_empty() {
                    resolved_command.clone().unwrap_or_default()
                } else {
                    rest.to_string()
                };
                (resolved_command, clean_message)
            }
        } else {
            (None, message.to_string())
        };

        let mut seen = HashSet::new();
        let mut active_skills = Vec::new();

        if let Some(command_name) = command.as_deref() {
            self.push_active_skill(command_name, &mut seen, &mut active_skills);
        }

        for skill_name in self.skills.mentioned_commands(&clean_message) {
            self.push_active_skill(&skill_name, &mut seen, &mut active_skills);
        }

        TurnSkillContext {
            command,
            clean_message,
            active_skills,
        }
    }

    pub(super) async fn tool_runtime_for(
        &self,
        history: &[ChatTurn],
        context_preview: Option<String>,
    ) -> ToolRuntime {
        ToolRuntime::bootstrap(
            self.mcp_servers.clone(),
            self.native_tools.clone(),
            history.to_vec(),
            context_preview,
        )
        .await
    }

    pub(super) async fn tool_runtime_native_only(
        &self,
        history: &[ChatTurn],
        context_preview: Option<String>,
    ) -> ToolRuntime {
        ToolRuntime::bootstrap(
            vec![],
            self.native_tools.clone(),
            history.to_vec(),
            context_preview,
        )
        .await
    }

    /// Tạo runtime đầy đủ (native + MCP) nhưng mỗi MCP server browser sẽ chạy
    /// với `--isolated` flag để tránh xung đột profile giữa các subagent.
    pub(super) async fn tool_runtime_for_isolated(
        &self,
        history: &[ChatTurn],
        context_preview: Option<String>,
    ) -> ToolRuntime {
        let isolated_mcp = self
            .mcp_servers
            .iter()
            .map(|server| {
                let mut config = server.clone();
                if config.name.eq_ignore_ascii_case("chrome-devtools")
                    && !config.args.iter().any(|a| a == "--isolated")
                {
                    config.args.push("--isolated".to_string());
                }
                config
            })
            .collect();

        ToolRuntime::bootstrap(
            isolated_mcp,
            self.native_tools.clone(),
            history.to_vec(),
            context_preview,
        )
        .await
    }

    fn mcp_servers_for(&self) -> Vec<DebugMcpServerView> {
        self.mcp_servers
            .iter()
            .map(|server| DebugMcpServerView {
                name: server.name.clone(),
                description: describe_mcp_server(server),
                timeout_ms: server.timeout_ms,
                command: server.command.clone(),
                args: server.args.clone(),
                shared: true,
            })
            .collect()
    }

    fn native_tools_for(&self) -> Vec<DebugToolView> {
        self.native_tools
            .iter()
            .map(|tool| DebugToolView {
                name: tool.name.clone(),
                kind: tool.kind.clone(),
                description: describe_native_tool(tool),
                timeout_ms: tool.timeout_ms,
                shared: true,
            })
            .collect()
    }

    fn push_active_skill(
        &self,
        name: &str,
        seen: &mut HashSet<String>,
        active_skills: &mut Vec<ActiveSkill>,
    ) {
        let normalized = name.trim().to_ascii_lowercase();
        if normalized.is_empty() || seen.contains(&normalized) {
            return;
        }

        let Some(markdown) = self.skills.command_markdown(name) else {
            return;
        };

        seen.insert(normalized);
        active_skills.push(ActiveSkill {
            name: name.trim().to_string(),
            markdown,
        });
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
        "extract_signals" => "Rút tín hiệu kỹ thuật từ dữ liệu đã có".to_string(),
        "memory_lookup" => "Tra lại ngữ cảnh/lịch sử để đối chiếu khi debug".to_string(),
        "summarize_sources" => "Tóm tắt danh sách nguồn và các điểm chính".to_string(),
        "read" => "Đọc file trong workspace backend theo path được kiểm soát".to_string(),
        "write" => "Ghi hoặc append file text trong workspace backend".to_string(),
        "exec" => "Chạy executable trực tiếp trong workspace backend".to_string(),
        "bash" => "Chạy lệnh bash trong workspace backend với timeout cấu hình".to_string(),
        "spawn_team" => {
            "Spawn team subagent động để trao đổi nội bộ rồi báo cáo lại cho Kuromi".to_string()
        }
        _ => format!("Native tool {}", tool.name),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_skills_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "hybridtrade-capabilities-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(dir.join("commands")).unwrap();
        dir
    }

    fn catalog_with_command(name: &str) -> CapabilityCatalog {
        let dir = temp_skills_dir("command");
        fs::write(
            dir.join("commands").join(format!("{name}.md")),
            "# test skill\n\nbody",
        )
        .unwrap();

        CapabilityCatalog::new(ToolingConfig::default(), SkillRegistry::load(&dir).unwrap())
    }

    #[test]
    fn resolves_slash_command_into_active_skill() {
        let catalog = catalog_with_command("chrome-devtools");
        let resolved = catalog.resolve_turn_skills("/chrome-devtools mở example.com");

        assert_eq!(resolved.command.as_deref(), Some("chrome-devtools"));
        assert_eq!(resolved.clean_message, "mở example.com");
        assert_eq!(resolved.active_skills.len(), 1);
        assert_eq!(resolved.active_skills[0].name, "chrome-devtools");
    }

    #[test]
    fn resolves_mentioned_skill_name_without_slash_command() {
        let catalog = catalog_with_command("chrome-devtools");
        let resolved = catalog.resolve_turn_skills("hãy dùng chrome devtools để kiểm tra");

        assert!(resolved.command.is_none());
        assert_eq!(
            resolved.clean_message,
            "hãy dùng chrome devtools để kiểm tra"
        );
        assert_eq!(resolved.active_skills.len(), 1);
        assert_eq!(resolved.active_skills[0].name, "chrome-devtools");
    }
}
