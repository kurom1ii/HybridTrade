use std::{
    collections::{BTreeMap, HashMap},
    env,
    path::PathBuf,
};

use reqwest::Client;
use serde_json::{json, Value};

use crate::agents::providers::team::TeamOrchestrator;
use crate::config::{McpServerConfig, NativeToolConfig};

use super::super::models::{ChatTurn, DebugToolCall};
use super::{
    mcp::McpSession,
    native::NativeToolKind,
    utils::{
        best_effort_canonicalize, render_tool_output_for_model, resolve_workspace_root,
        sanitize_tool_arguments, tool_output_is_error, truncate_chars, MAX_TOOL_PREVIEW_CHARS,
    },
};

pub(crate) struct ToolRuntime {
    pub(super) http_client: Client,
    pub(super) history: Vec<ChatTurn>,
    pub(super) context_preview: Option<String>,
    pub(super) workspace_root: PathBuf,
    pub(super) tools: BTreeMap<String, ToolDefinition>,
    pub(super) mcp_sessions: HashMap<String, McpSession>,
    pub(super) initialization_warnings: Vec<String>,
    pub(super) tool_calls: Vec<DebugToolCall>,
    pub(super) team_orchestrator: Option<TeamOrchestrator>,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolDefinition {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) input_schema: Value,
    pub(super) source_label: String,
    pub(super) executor: ToolExecutor,
}

#[derive(Debug, Clone)]
pub(super) enum ToolExecutor {
    Native {
        kind: NativeToolKind,
        timeout: std::time::Duration,
    },
    Mcp {
        server_name: String,
        tool_name: String,
    },
}

impl ToolRuntime {
    pub(crate) async fn bootstrap(
        mcp_servers: Vec<McpServerConfig>,
        native_tools: Vec<NativeToolConfig>,
        history: Vec<ChatTurn>,
        context_preview: Option<String>,
        http_client: Client,
    ) -> Self {
        let (workspace_root, workspace_warning) = match resolve_workspace_root() {
            Ok(path) => (path, None),
            Err(error) => {
                let fallback = best_effort_canonicalize(
                    env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                );
                let warning = format!(
                    "không thể resolve workspace root cho native tools: {}. Dùng fallback `{}`",
                    error,
                    fallback.display()
                );
                (fallback, Some(warning))
            }
        };

        let mut runtime = Self {
            http_client,
            history,
            context_preview,
            workspace_root,
            tools: BTreeMap::new(),
            mcp_sessions: HashMap::new(),
            initialization_warnings: Vec::new(),
            tool_calls: Vec::new(),
            team_orchestrator: None,
        };

        if let Some(warning) = workspace_warning {
            runtime.initialization_warnings.push(warning);
        }

        for tool in native_tools {
            runtime.register_native_tool(tool);
        }

        for server in mcp_servers {
            runtime.load_mcp_server(server).await;
        }

        runtime
    }

    pub(crate) fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().cloned().collect()
    }

    pub(crate) fn available_tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub(crate) fn initialization_warnings(&self) -> &[String] {
        &self.initialization_warnings
    }

    pub(crate) fn tool_calls(&self) -> &[DebugToolCall] {
        &self.tool_calls
    }

    pub(crate) fn prepare_turn(&mut self, history: &[ChatTurn], context_preview: Option<String>) {
        self.history = history.to_vec();
        self.context_preview = context_preview;
        self.tool_calls.clear();
        self.team_orchestrator = None;
    }

    pub(crate) fn attach_team_orchestrator(&mut self, team_orchestrator: TeamOrchestrator) {
        self.team_orchestrator = Some(team_orchestrator);
    }

    pub(crate) async fn execute(&mut self, name: &str, arguments: Value) -> String {
        let arguments = sanitize_tool_arguments(arguments);

        let Some(definition) = self.tools.get(name).cloned() else {
            let output = json!({
                "ok": false,
                "error": format!("tool `{name}` không tồn tại trong runtime hiện tại"),
            });
            let output_text = render_tool_output_for_model(&output);
            self.tool_calls.push(DebugToolCall {
                name: name.to_string(),
                source: "runtime".to_string(),
                status: "failed".to_string(),
                input: arguments,
                output_preview: truncate_chars(&output_text, MAX_TOOL_PREVIEW_CHARS),
            });
            return output_text;
        };

        let result = self
            .execute_inner(&definition.executor, arguments.clone())
            .await;

        match result {
            Ok(output) => {
                let output_text = render_tool_output_for_model(&output);
                let status = if tool_output_is_error(&output) {
                    "failed"
                } else {
                    "completed"
                };
                self.tool_calls.push(DebugToolCall {
                    name: definition.name,
                    source: definition.source_label,
                    status: status.to_string(),
                    input: arguments,
                    output_preview: truncate_chars(&output_text, MAX_TOOL_PREVIEW_CHARS),
                });
                output_text
            }
            Err(error) => {
                let output = json!({
                    "ok": false,
                    "error": error.to_string(),
                });
                let output_text = render_tool_output_for_model(&output);
                self.tool_calls.push(DebugToolCall {
                    name: definition.name,
                    source: definition.source_label,
                    status: "failed".to_string(),
                    input: arguments,
                    output_preview: truncate_chars(&output_text, MAX_TOOL_PREVIEW_CHARS),
                });
                output_text
            }
        }
    }

    pub(super) fn insert_definition(&mut self, definition: ToolDefinition) {
        if self.tools.contains_key(&definition.name) {
            self.initialization_warnings.push(format!(
                "tool `{}` bị trùng tên trong runtime, giữ lại bản đầu tiên",
                definition.name
            ));
            return;
        }

        self.tools.insert(definition.name.clone(), definition);
    }

    pub(crate) fn remove_definition(&mut self, name: &str) {
        self.tools.remove(name);
    }

    async fn execute_inner(
        &mut self,
        executor: &ToolExecutor,
        arguments: Value,
    ) -> anyhow::Result<Value> {
        match executor {
            ToolExecutor::Native { kind, timeout } => {
                self.execute_native_tool(*kind, *timeout, arguments).await
            }
            ToolExecutor::Mcp {
                server_name,
                tool_name,
            } => {
                self.execute_mcp_tool(server_name, tool_name, arguments)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;
    use serde_json::json;
    use std::{
        fs as std_fs,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "hybridtrade-tool-runtime-{label}-{}-{unique}",
            std::process::id()
        ));
        std_fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_runtime(workspace_root: PathBuf) -> ToolRuntime {
        ToolRuntime {
            http_client: Client::new(),
            history: Vec::new(),
            context_preview: None,
            workspace_root,
            tools: BTreeMap::new(),
            mcp_sessions: HashMap::new(),
            initialization_warnings: Vec::new(),
            tool_calls: Vec::new(),
            team_orchestrator: None,
        }
    }

    #[tokio::test]
    async fn read_tool_supports_line_windows() {
        let root = temp_dir("read");
        std_fs::write(root.join("notes.txt"), "line-1\nline-2\nline-3\n").unwrap();
        let runtime = test_runtime(root.clone());

        let result = runtime
            .read_path(json!({
                "path": "notes.txt",
                "start_line": 2,
                "line_count": 2,
            }))
            .await
            .unwrap();

        assert_eq!(result["content"], "line-2\nline-3");
        assert_eq!(result["start_line"], 2);
        assert_eq!(result["end_line"], 3);

        let _ = std_fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn write_tool_blocks_paths_outside_workspace() {
        let root = temp_dir("write");
        let runtime = test_runtime(root.clone());

        let error = runtime
            .write_path(json!({
                "path": "../escape.txt",
                "content": "boom",
            }))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("nằm ngoài workspace"));

        let _ = std_fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn exec_tool_runs_inside_workspace() {
        let root = temp_dir("exec");
        let runtime = test_runtime(root.clone());

        let result = runtime
            .exec_command(
                json!({
                    "command": "pwd",
                }),
                Duration::from_secs(2),
            )
            .await
            .unwrap();

        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains(&root.display().to_string()));

        let _ = std_fs::remove_dir_all(root);
    }
}
