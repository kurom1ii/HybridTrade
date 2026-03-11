use reqwest::Client;

use crate::config::{McpServerConfig, NativeToolConfig, ToolingConfig};

use super::super::{
    models::{AgentRole, ChatTurn, DebugMcpServerView, DebugToolView},
    skills::SkillRegistry,
    tool_runtime::runtime::ToolRuntime,
};

#[derive(Debug, Clone)]
pub struct AgentCapabilityProfile {
    pub common_skills: Vec<String>,
    pub agent_skills: Vec<String>,
    pub skill_tools: Vec<String>,
    pub mcp_servers: Vec<DebugMcpServerView>,
    pub native_tools: Vec<DebugToolView>,
}

#[derive(Debug, Clone)]
pub(super) struct AgentPromptProfile {
    pub(super) common_markdown: String,
    pub(super) agent_markdown: String,
}

#[derive(Clone)]
pub(super) struct CapabilityCatalog {
    skills: SkillRegistry,
    skill_tools: Vec<String>,
    mcp_servers: Vec<McpServerConfig>,
    native_tools: Vec<NativeToolConfig>,
}

impl CapabilityCatalog {
    pub(super) fn new(tooling: ToolingConfig, skills: SkillRegistry) -> Self {
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

    pub(super) fn profile_for(&self, role: AgentRole) -> AgentCapabilityProfile {
        AgentCapabilityProfile {
            common_skills: self.skills.common_titles(),
            agent_skills: self.skills.agent_titles(role),
            skill_tools: self.skill_tools.clone(),
            mcp_servers: self.mcp_servers_for(role),
            native_tools: self.native_tools_for(role),
        }
    }

    pub(super) fn prompt_profile_for(&self, role: AgentRole) -> AgentPromptProfile {
        AgentPromptProfile {
            common_markdown: self.skills.common_markdown(),
            agent_markdown: self.skills.agent_markdown(role),
        }
    }

    pub(super) async fn tool_runtime_for(
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

pub(super) fn mcp_server_allowed_for_role(server: &McpServerConfig, role: AgentRole) -> bool {
    if server.allowed_agents.is_empty() {
        return true;
    }

    server
        .allowed_agents
        .iter()
        .any(|agent| agent.trim().eq_ignore_ascii_case(role.as_str()))
}

pub(super) fn tool_allowed_for_role(tool: &NativeToolConfig, role: AgentRole) -> bool {
    if tool.allowed_agents.is_empty() {
        return true;
    }

    tool.allowed_agents
        .iter()
        .any(|agent| agent.trim().eq_ignore_ascii_case(role.as_str()))
}
