use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::agents::providers::team::{SpawnTeamRequest, TeamRuntimeContext};
use crate::agents::tool_runtime::runtime::ToolRuntime;

pub(crate) const DESCRIPTION: &str =
    "Spawn một team subagent runtime-only, cho họ trao đổi qua transcript chung rồi trả báo cáo về cho Kuromi.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "mission": {
                "type": "string",
                "description": "Mục tiêu chung mà team dynamic cần giải quyết"
            },
            "briefing": {
                "type": "string",
                "description": "Bổ sung ngắn từ Kuromi để các subagent bám vào"
            },
            "rounds": {
                "type": "integer",
                "minimum": 1,
                "maximum": 4,
                "description": "Số vòng thảo luận cho team, mặc định 2"
            },
            "report_instruction": {
                "type": "string",
                "description": "Định dạng hoặc yêu cầu riêng cho báo cáo cuối của team"
            },
            "members": {
                "type": "array",
                "description": "Danh sách subagent cần spawn cho mission này",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Tên hiển thị của subagent"
                        },
                        "responsibility": {
                            "type": "string",
                            "description": "Trọng trách hoặc góc phân tích chính của subagent"
                        },
                        "instructions": {
                            "type": "string",
                            "description": "Chỉ dẫn bổ sung riêng cho subagent này"
                        }
                    },
                    "required": ["name", "responsibility"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["mission", "members"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(runtime: &ToolRuntime, arguments: Value) -> Result<Value> {
    let request: SpawnTeamRequest =
        serde_json::from_value(arguments).context("payload của spawn_team không hợp lệ")?;
    let Some(team_orchestrator) = runtime.team_orchestrator.as_ref() else {
        bail!("spawn_team chưa được gắn team orchestrator ở runtime hiện tại");
    };

    let output = team_orchestrator
        .execute(
            request,
            TeamRuntimeContext {
                history: runtime.history.clone(),
                context_preview: runtime.context_preview.clone(),
            },
        )
        .await?;

    serde_json::to_value(output).context("không thể serialize kết quả spawn_team")
}
