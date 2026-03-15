use std::{path::PathBuf, time::Duration};

use anyhow::{bail, Result};
use serde_json::Value;

use crate::config::NativeToolConfig;

use super::{
    runtime::{ToolDefinition, ToolExecutor, ToolRuntime},
    tools,
    utils::{ensure_path_is_within_workspace, normalize_path_for_workspace},
};

#[derive(Debug, Clone, Copy)]
pub(super) enum NativeToolKind {
    Read,
    Write,
    Exec,
    Bash,
    SpawnTeam,
    FetchNews,
    FetchCalendar,
    FetchDashboard,
    UpdateDashboard,
    ReusableSkills,
}

impl ToolRuntime {
    pub(super) fn register_native_tool(&mut self, config: NativeToolConfig) {
        let Some(kind) = native_tool_kind(&config.name) else {
            self.initialization_warnings.push(format!(
                "native tool `{}` chưa có executor Rust tương ứng, nên bị bỏ qua",
                config.name
            ));
            return;
        };

        let definition = ToolDefinition {
            name: config.name.clone(),
            description: native_tool_description(kind).to_string(),
            input_schema: native_tool_schema(kind),
            source_label: format!("native:{}", config.name),
            executor: ToolExecutor::Native {
                kind,
                timeout: Duration::from_millis(config.timeout_ms.max(1_000)),
            },
        };

        self.insert_definition(definition);
    }

    pub(super) async fn execute_native_tool(
        &self,
        kind: NativeToolKind,
        tool_timeout: Duration,
        arguments: Value,
    ) -> Result<Value> {
        match tokio::time::timeout(tool_timeout, async {
            match kind {
                NativeToolKind::Read => tools::read::execute(self, arguments).await,
                NativeToolKind::Write => tools::write::execute(self, arguments).await,
                NativeToolKind::Exec => tools::exec::execute(self, arguments, tool_timeout).await,
                NativeToolKind::Bash => tools::bash::execute(self, arguments, tool_timeout).await,
                NativeToolKind::SpawnTeam => tools::spawn_team::execute(self, arguments).await,
                NativeToolKind::FetchNews => tools::fetch_news::execute(arguments).await,
                NativeToolKind::FetchCalendar => tools::fetch_calendar::execute(arguments).await,
                NativeToolKind::FetchDashboard => tools::fetch_dashboard::execute(arguments).await,
                NativeToolKind::UpdateDashboard => {
                    tools::update_dashboard::execute(arguments).await
                }
                NativeToolKind::ReusableSkills => {
                    tools::reusable_skills::execute(arguments, &self.workspace_root).await
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => bail!(
                "native tool `{}` vượt quá timeout {}ms",
                native_tool_name(kind),
                tool_timeout.as_millis()
            ),
        }
    }

    pub(crate) fn resolve_workspace_path(&self, requested_path: &str) -> Result<PathBuf> {
        let candidate = PathBuf::from(requested_path);
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            self.workspace_root.join(candidate)
        };
        let normalized = normalize_path_for_workspace(&candidate)?;
        ensure_path_is_within_workspace(&normalized, &self.workspace_root)?;
        Ok(normalized)
    }

    pub(crate) fn resolve_command_cwd(&self, requested_cwd: Option<&str>) -> Result<PathBuf> {
        let cwd = match requested_cwd {
            Some(value) => self.resolve_workspace_path(value)?,
            None => self.workspace_root.clone(),
        };

        if !cwd.is_dir() {
            bail!("cwd `{}` không phải thư mục", cwd.display());
        }

        Ok(cwd)
    }
}

pub(super) fn native_tool_kind(name: &str) -> Option<NativeToolKind> {
    match name.trim().to_ascii_lowercase().as_str() {
        "read" => Some(NativeToolKind::Read),
        "write" => Some(NativeToolKind::Write),
        "exec" => Some(NativeToolKind::Exec),
        "bash" => Some(NativeToolKind::Bash),
        "spawn_team" => Some(NativeToolKind::SpawnTeam),
        "fetch_news" => Some(NativeToolKind::FetchNews),
        "fetch_calendar" => Some(NativeToolKind::FetchCalendar),
        "fetch_dashboard" => Some(NativeToolKind::FetchDashboard),
        "update_dashboard" => Some(NativeToolKind::UpdateDashboard),
        "reusable_skills" => Some(NativeToolKind::ReusableSkills),
        _ => None,
    }
}

fn native_tool_name(kind: NativeToolKind) -> &'static str {
    match kind {
        NativeToolKind::Read => "read",
        NativeToolKind::Write => "write",
        NativeToolKind::Exec => "exec",
        NativeToolKind::Bash => "bash",
        NativeToolKind::SpawnTeam => "spawn_team",
        NativeToolKind::FetchNews => "fetch_news",
        NativeToolKind::FetchCalendar => "fetch_calendar",
        NativeToolKind::FetchDashboard => "fetch_dashboard",
        NativeToolKind::UpdateDashboard => "update_dashboard",
        NativeToolKind::ReusableSkills => "reusable_skills",
    }
}

pub(super) fn native_tool_description(kind: NativeToolKind) -> &'static str {
    match kind {
        NativeToolKind::Read => tools::read::DESCRIPTION,
        NativeToolKind::Write => tools::write::DESCRIPTION,
        NativeToolKind::Exec => tools::exec::DESCRIPTION,
        NativeToolKind::Bash => tools::bash::DESCRIPTION,
        NativeToolKind::SpawnTeam => tools::spawn_team::DESCRIPTION,
        NativeToolKind::FetchNews => tools::fetch_news::DESCRIPTION,
        NativeToolKind::FetchCalendar => tools::fetch_calendar::DESCRIPTION,
        NativeToolKind::FetchDashboard => tools::fetch_dashboard::DESCRIPTION,
        NativeToolKind::UpdateDashboard => tools::update_dashboard::DESCRIPTION,
        NativeToolKind::ReusableSkills => tools::reusable_skills::DESCRIPTION,
    }
}

fn native_tool_schema(kind: NativeToolKind) -> Value {
    match kind {
        NativeToolKind::Read => tools::read::schema(),
        NativeToolKind::Write => tools::write::schema(),
        NativeToolKind::Exec => tools::exec::schema(),
        NativeToolKind::Bash => tools::bash::schema(),
        NativeToolKind::SpawnTeam => tools::spawn_team::schema(),
        NativeToolKind::FetchNews => tools::fetch_news::schema(),
        NativeToolKind::FetchCalendar => tools::fetch_calendar::schema(),
        NativeToolKind::FetchDashboard => tools::fetch_dashboard::schema(),
        NativeToolKind::UpdateDashboard => tools::update_dashboard::schema(),
        NativeToolKind::ReusableSkills => tools::reusable_skills::schema(),
    }
}
