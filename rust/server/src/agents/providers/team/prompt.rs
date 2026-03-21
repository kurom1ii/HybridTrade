use super::models::{SpawnTeamRequest, SubagentFindings, SubagentStatus};

const MAX_CONTEXT_PREVIEW_CHARS: usize = 8_000;

pub(super) fn build_subagent_system_prompt(
    request: &SpawnTeamRequest,
    member_name: &str,
    responsibility: &str,
    instructions: Option<&str>,
    roster: &[String],
) -> String {
    let instructions_block = instructions
        .map(|value| format!("\nChỉ dẫn thêm cho bạn:\n- {value}"))
        .unwrap_or_default();
    let report_block = request
        .report_instruction
        .as_deref()
        .map(|value| format!("\nYêu cầu báo cáo cho cả team:\n- {value}"))
        .unwrap_or_default();

    let roster_block = roster
        .iter()
        .map(|r| format!("  - {r}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"Bạn là subagent độc lập trong team do Kuromi Finance điều phối. Bạn không hiển thị cho user nhưng có đầy đủ quyền tool và MCP giống agent chính (bao gồm Chrome DevTools / CDP).

Leader: Kuromi (điều phối, tổng hợp báo cáo)
Tên bạn: {member_name}
Phạm vi chính: {responsibility}{instructions_block}{report_block}

Thành viên team:
{roster_block}

Mục tiêu team:
- {mission}

Quy tắc:
- Bạn chạy độc lập và song song với các subagent khác — không thấy response của họ.
- Tập trung hoàn thành trách nhiệm riêng, dùng tool để thu thập dữ liệu và xác minh.
- Sau mỗi `tool_result`, phải đọc kỹ output để biết cần làm gì tiếp theo.
- Bạn có toàn bộ tool, MCP, CDP — hãy dùng ngay khi cần xác minh hoặc hành động.
- Giữ câu trả lời ngắn, rõ, có căn cứ và hướng về quyết định cho Kuromi.
- Kết thúc bằng đoạn báo cáo ngắn mà Kuromi có thể tái sử dụng trực tiếp."#,
        mission = request.mission,
    )
}

pub(super) fn build_subagent_task_message(
    request: &SpawnTeamRequest,
    member_name: &str,
    responsibility: &str,
    instructions: Option<&str>,
    context_preview: Option<&str>,
) -> String {
    let mut blocks = Vec::new();

    blocks.push(format!("Mission: {}", request.mission));

    if let Some(briefing) = request.briefing.as_deref() {
        blocks.push(format!("Bổ sung: {briefing}"));
    }

    blocks.push(format!(
        "Bạn là {member_name}, phạm vi: {responsibility}."
    ));

    if let Some(instructions) = instructions {
        blocks.push(format!("Chỉ dẫn riêng: {instructions}"));
    }

    if let Some(preview) = context_preview
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        blocks.push(format!(
            "Ngữ cảnh backend hiện tại:\n{}",
            truncate_chars(preview, MAX_CONTEXT_PREVIEW_CHARS)
        ));
    }

    blocks.push(
        "Hãy thực hiện phân tích theo trách nhiệm riêng, dùng tool nếu cần. \
         Kết thúc bằng báo cáo ngắn cho Kuromi."
            .to_string(),
    );

    blocks.join("\n\n")
}

pub(super) fn build_kuromi_brief(
    request: &SpawnTeamRequest,
    findings: &[SubagentFindings],
) -> String {
    let mut lines = vec![format!("Mission: {}", request.mission)];

    if let Some(briefing) = request.briefing.as_deref() {
        lines.push(format!("Briefing: {briefing}"));
    }

    lines.push("Báo cáo team:".to_string());
    for finding in findings {
        let status_label = match &finding.status {
            SubagentStatus::Completed => "",
            SubagentStatus::Failed(err) => &format!(" [FAILED: {err}]"),
        };
        lines.push(format!(
            "- {} ({}){} => {}",
            finding.member,
            finding.responsibility,
            status_label,
            truncate_chars(&collapse_whitespace(&finding.content), 420)
        ));
    }

    if let Some(report_instruction) = request.report_instruction.as_deref() {
        lines.push(format!("Nhắc lại yêu cầu báo cáo: {report_instruction}"));
    }

    lines.join("\n")
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}
