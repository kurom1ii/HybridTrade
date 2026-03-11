use super::models::{SpawnTeamReport, SpawnTeamRequest, SpawnTeamTranscriptEntry};
use crate::agents::models::ChatTurn;

const MAX_HISTORY_CHARS: usize = 2_400;
const MAX_TRANSCRIPT_CHARS: usize = 5_400;
const MAX_CONTEXT_PREVIEW_CHARS: usize = 8_000;

pub(super) fn build_subagent_system_prompt(
    common_markdown: &str,
    kuromi_markdown: &str,
    request: &SpawnTeamRequest,
    member_name: &str,
    responsibility: &str,
    instructions: Option<&str>,
) -> String {
    let common_skills = render_markdown_block(
        common_markdown,
        "# Skills chung\n\nChưa có file Markdown nào trong `.skills/common`.",
    );
    let kuromi_skills = render_markdown_block(
        kuromi_markdown,
        "# Skills Kuromi\n\n- Ưu tiên phối hợp tool đúng lúc và báo cáo rõ ràng cho Kuromi Finance.",
    );
    let instructions_block = instructions
        .map(|value| format!("\nChỉ dẫn thêm cho bạn:\n- {value}"))
        .unwrap_or_default();
    let report_block = request
        .report_instruction
        .as_deref()
        .map(|value| format!("\nYêu cầu báo cáo cho cả team:\n- {value}"))
        .unwrap_or_default();

    format!(
        r#"Bạn là subagent runtime-only trong team do Kuromi Finance spawn ra. Bạn không phải agent cố định hiển thị cho user.

Tên subagent: {member_name}
Phạm vi chính: {responsibility}{instructions_block}{report_block}

Mục tiêu team:
- {mission}

Skills chung của hệ thống:
{common_skills}

Skills của Kuromi:
{kuromi_skills}

Quy tắc:
- Trao đổi như một thành viên chuyên trách trong team, không tự xưng là Kuromi.
- Có thể đồng ý, phản biện, sửa giả thuyết hoặc yêu cầu thêm dữ liệu khi cần.
- Nếu cần tool hoặc MCP thật sự để xác minh, dùng tool hiện có ngay trong lượt của bạn.
- Giữ câu trả lời ngắn, rõ, có căn cứ và hướng về quyết định cho Kuromi.
- Ưu tiên nêu điểm mới so với transcript hiện tại, tránh lặp lại nguyên văn người khác.
- Kết thúc bằng một đoạn báo cáo ngắn mà Kuromi có thể tái sử dụng trực tiếp."#,
        mission = request.mission,
    )
}

pub(super) fn build_round_message(
    request: &SpawnTeamRequest,
    round: usize,
    total_rounds: usize,
    history: &[ChatTurn],
    transcript: &[SpawnTeamTranscriptEntry],
) -> String {
    let caller_history = render_history(history);
    let transcript = render_transcript(transcript);
    let briefing = request
        .briefing
        .as_deref()
        .map(|value| format!("\nBổ sung từ Kuromi:\n{value}\n"))
        .unwrap_or_default();

    format!(
        "Round {round}/{total_rounds} của team dynamic.\n\
Mục tiêu: {mission}\n{briefing}\n\
Lịch sử chat gần đây giữa user và Kuromi:\n{caller_history}\n\n\
Transcript team hiện tại:\n{transcript}\n\n\
Yêu cầu cho lượt này:\n\
- Đóng góp thêm góc nhìn mới, hoặc phản biện trực tiếp ý đã có nếu cần.\n\
- Nếu cần tool để kiểm chứng, dùng tool ngay rồi phản hồi theo kết quả thật.\n\
- Ưu tiên insight có thể chuyển thành báo cáo ngắn cho Kuromi.\n\
- Nếu không có gì mới, nói rõ điều đó thay vì lặp lại.",
        mission = request.mission,
    )
}

pub(super) fn build_runtime_context_preview(
    request: &SpawnTeamRequest,
    history: &[ChatTurn],
    context_preview: Option<&str>,
    transcript: &[SpawnTeamTranscriptEntry],
) -> Option<String> {
    let mut blocks = Vec::new();

    if let Some(context_preview) = context_preview
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        blocks.push(format!(
            "Ngữ cảnh backend hiện tại:\n{}",
            truncate_chars(context_preview, 2_400)
        ));
    }

    if !history.is_empty() {
        blocks.push(format!("Caller history:\n{}", render_history(history)));
    }

    if !transcript.is_empty() {
        blocks.push(format!(
            "Transcript team:\n{}",
            render_transcript(transcript)
        ));
    }

    blocks.push(format!("Mission team: {}", request.mission));

    if let Some(briefing) = request.briefing.as_deref() {
        blocks.push(format!("Briefing bổ sung: {briefing}"));
    }

    let preview = truncate_chars(&blocks.join("\n\n"), MAX_CONTEXT_PREVIEW_CHARS);
    if preview.trim().is_empty() {
        None
    } else {
        Some(preview)
    }
}

pub(super) fn build_kuromi_brief(
    request: &SpawnTeamRequest,
    reports: &[SpawnTeamReport],
) -> String {
    let mut lines = vec![format!("Mission: {}", request.mission)];

    if let Some(briefing) = request.briefing.as_deref() {
        lines.push(format!("Briefing: {briefing}"));
    }

    lines.push("Báo cáo team:".to_string());
    for report in reports {
        lines.push(format!(
            "- {} ({}) => {}",
            report.member,
            report.responsibility,
            truncate_chars(&collapse_whitespace(&report.report), 420)
        ));
    }

    if let Some(report_instruction) = request.report_instruction.as_deref() {
        lines.push(format!("Nhắc lại yêu cầu báo cáo: {report_instruction}"));
    }

    lines.join("\n")
}

fn render_markdown_block(markdown: &str, fallback: &str) -> String {
    let markdown = markdown.trim();
    if markdown.is_empty() {
        fallback.to_string()
    } else {
        markdown.to_string()
    }
}

fn render_history(history: &[ChatTurn]) -> String {
    if history.is_empty() {
        return "Chưa có history caller đáng kể.".to_string();
    }

    let mut lines = history
        .iter()
        .rev()
        .take(6)
        .map(|turn| {
            format!(
                "- {}: {}",
                role_label(&turn.role),
                truncate_chars(&collapse_whitespace(&turn.content), 320)
            )
        })
        .collect::<Vec<_>>();
    lines.reverse();

    truncate_chars(&lines.join("\n"), MAX_HISTORY_CHARS)
}

fn render_transcript(transcript: &[SpawnTeamTranscriptEntry]) -> String {
    if transcript.is_empty() {
        return "Chưa có ai phát biểu trong team.".to_string();
    }

    let mut lines = transcript
        .iter()
        .rev()
        .map(|entry| {
            format!(
                "- Round {} | {} ({}) => {}",
                entry.round,
                entry.speaker,
                entry.responsibility,
                truncate_chars(&collapse_whitespace(&entry.content), 320)
            )
        })
        .collect::<Vec<_>>();
    lines.reverse();

    truncate_chars(&lines.join("\n"), MAX_TRANSCRIPT_CHARS)
}

fn role_label(role: &str) -> &'static str {
    match role {
        "assistant" => "Kuromi",
        _ => "User",
    }
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
