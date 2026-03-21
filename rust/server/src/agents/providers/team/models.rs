use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::agents::models::DebugToolCall;

const MAX_TEAM_MEMBERS: usize = 6;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SpawnTeamRequest {
    pub(crate) mission: String,
    #[serde(default)]
    pub(crate) briefing: Option<String>,
    #[serde(default)]
    pub(crate) members: Vec<SpawnTeamMemberSpec>,
    /// Kept for backward compat — ignored in execution (parallel blackboard pattern).
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) rounds: Option<usize>,
    #[serde(default)]
    pub(crate) report_instruction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SpawnTeamMemberSpec {
    pub(crate) name: String,
    pub(crate) responsibility: String,
    #[serde(default)]
    pub(crate) instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SpawnTeamResult {
    pub(crate) ok: bool,
    pub(crate) mission: String,
    pub(crate) briefing: Option<String>,
    pub(crate) duration_ms: u64,
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) members: Vec<SpawnTeamMemberView>,
    pub(crate) reports: Vec<SpawnTeamReport>,
    pub(crate) kuromi_brief: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SpawnTeamMemberView {
    pub(crate) name: String,
    pub(crate) responsibility: String,
    pub(crate) instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SpawnTeamReport {
    pub(crate) member: String,
    pub(crate) responsibility: String,
    pub(crate) report: String,
}

/// Entry in the shared blackboard DashMap.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SubagentFindings {
    pub(crate) member: String,
    pub(crate) responsibility: String,
    pub(crate) content: String,
    pub(crate) tool_calls: Vec<DebugToolCall>,
    pub(crate) status: SubagentStatus,
    pub(crate) completed_at: String,
}

#[derive(Debug, Clone)]
pub(crate) enum SubagentStatus {
    Completed,
    Failed(String),
}

impl SpawnTeamRequest {
    pub(crate) fn validate(mut self) -> Result<Self> {
        self.mission = self.mission.trim().to_string();
        self.briefing = normalize_optional(self.briefing);
        self.report_instruction = normalize_optional(self.report_instruction);

        if self.mission.is_empty() {
            bail!("spawn_team cần `mission`");
        }

        if self.members.is_empty() {
            bail!("spawn_team cần ít nhất một member");
        }

        if self.members.len() > MAX_TEAM_MEMBERS {
            bail!("spawn_team chỉ hỗ trợ tối đa {MAX_TEAM_MEMBERS} member mỗi lượt");
        }

        let mut names = Vec::new();
        for member in &mut self.members {
            member.name = member.name.trim().to_string();
            member.responsibility = member.responsibility.trim().to_string();
            member.instructions = normalize_optional(member.instructions.take());

            if member.name.is_empty() {
                bail!("mỗi member trong spawn_team phải có `name`");
            }

            if member.responsibility.is_empty() {
                bail!(
                    "member `{}` trong spawn_team phải có `responsibility`",
                    member.name
                );
            }

            let lowered = member.name.to_ascii_lowercase();
            if names.iter().any(|value| value == &lowered) {
                bail!("tên member trong spawn_team bị trùng: {}", member.name);
            }
            names.push(lowered);
        }

        Ok(self)
    }
}

impl From<&SpawnTeamMemberSpec> for SpawnTeamMemberView {
    fn from(value: &SpawnTeamMemberSpec) -> Self {
        Self {
            name: value.name.clone(),
            responsibility: value.responsibility.clone(),
            instructions: value.instructions.clone(),
        }
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{SpawnTeamMemberSpec, SpawnTeamRequest};

    #[test]
    fn validate_rejects_duplicate_member_names() {
        let request = SpawnTeamRequest {
            mission: "check market".to_string(),
            briefing: None,
            members: vec![
                SpawnTeamMemberSpec {
                    name: "Analyst".to_string(),
                    responsibility: "tech".to_string(),
                    instructions: None,
                },
                SpawnTeamMemberSpec {
                    name: "analyst".to_string(),
                    responsibility: "macro".to_string(),
                    instructions: None,
                },
            ],
            rounds: Some(2),
            report_instruction: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn validate_trims_optional_fields() {
        let request = SpawnTeamRequest {
            mission: "  assess eurusd  ".to_string(),
            briefing: Some("  ".to_string()),
            members: vec![SpawnTeamMemberSpec {
                name: "  Macro  ".to_string(),
                responsibility: "  calendar  ".to_string(),
                instructions: Some("  focus on forecasts  ".to_string()),
            }],
            rounds: None,
            report_instruction: Some("  concise  ".to_string()),
        }
        .validate()
        .unwrap();

        assert_eq!(request.mission, "assess eurusd");
        assert!(request.briefing.is_none());
        assert_eq!(request.members[0].name, "Macro");
        assert_eq!(request.members[0].responsibility, "calendar");
        assert_eq!(
            request.members[0].instructions.as_deref(),
            Some("focus on forecasts")
        );
        assert_eq!(request.report_instruction.as_deref(), Some("concise"));
    }
}
