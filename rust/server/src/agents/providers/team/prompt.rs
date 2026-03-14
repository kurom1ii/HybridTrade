use super::models::{SpawnTeamReport, SpawnTeamRequest};
use super::session::{TeamMessageKind, TeamSession};
use crate::agents::models::ChatTurn;

use super::super::{capabilities::ActiveSkill, prompt::render_active_skills_block};

const MAX_HISTORY_CHARS: usize = 2_400;
const MAX_INBOX_CHARS: usize = 8_000;
const MAX_CONTEXT_PREVIEW_CHARS: usize = 8_000;

pub(super) fn build_subagent_system_prompt(
    request: &SpawnTeamRequest,
    member_name: &str,
    responsibility: &str,
    instructions: Option<&str>,
    roster: &[String],
    session_id: &str,
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
        r#"Bạn là subagent trong team do Kuromi Finance điều phối. Bạn không hiển thị cho user nhưng có đầy đủ quyền tool và MCP giống agent chính (bao gồm Chrome DevTools / CDP).

Team session: {session_id}
Leader: Kuromi (điều phối, ra directive, tổng hợp báo cáo)
Tên bạn: {member_name}
Phạm vi chính: {responsibility}{instructions_block}{report_block}

Thành viên team:
{roster_block}

Mục tiêu team:
- {mission}

Quy tắc:
- Bạn là thành viên trong team có leader là Kuromi. Kuromi gửi directive, bạn phản hồi.
- Bạn thấy được toàn bộ tin nhắn trước đó trong session: directive từ Kuromi, responses từ các thành viên khác.
- Members phản hồi tuần tự — bạn thấy response của member trước bạn trong cùng exchange.
- Có thể tham chiếu, đồng ý, phản biện, hoặc bổ sung ý kiến của thành viên khác.
- Bạn có toàn bộ tool, MCP, CDP — hãy dùng ngay khi cần xác minh hoặc hành động.
- Sau mỗi `tool_result`, phải đọc kỹ output để biết cần làm gì tiếp theo.
- Giữ trọng tâm vào trách nhiệm riêng nhưng phối hợp chặt chẽ qua messages.
- Giữ câu trả lời ngắn, rõ, có căn cứ và hướng về quyết định cho Kuromi.
- Ưu tiên nêu điểm mới, tránh lặp lại nguyên văn người khác.
- Kết thúc bằng đoạn báo cáo ngắn mà Kuromi có thể tái sử dụng trực tiếp."#,
        mission = request.mission,
    )
}

pub(super) fn build_member_inbox(
    member_name: &str,
    session: &TeamSession,
    active_skills: &[ActiveSkill],
) -> String {
    let visible = session.messages_visible_for(member_name);

    let mut lines = Vec::new();
    for msg in &visible {
        let sender_label = if msg.from == "system" {
            "system".to_string()
        } else if msg.from == "kuromi" {
            "kuromi (leader)".to_string()
        } else {
            msg.from.clone()
        };

        let target = if msg.to == "*" {
            "all".to_string()
        } else {
            msg.to.clone()
        };

        let kind_label = match msg.kind {
            TeamMessageKind::System => "system",
            TeamMessageKind::Directive => "directive",
            TeamMessageKind::Response => "response",
            TeamMessageKind::Discussion => "discussion",
        };

        let mut entry = format!(
            "[#{} {}→{} ({})] {}",
            msg.seq, sender_label, target, kind_label, msg.content
        );

        if !msg.tool_calls.is_empty() {
            let tool_names: Vec<String> = msg
                .tool_calls
                .iter()
                .map(|tc| format!("{} [{}]", tc.name, tc.status))
                .collect();
            entry.push_str(&format!("\n  Tools: {}", tool_names.join(", ")));
        }

        lines.push(entry);
    }

    let inbox = if lines.is_empty() {
        "Chưa có tin nhắn nào trong session.".to_string()
    } else {
        truncate_chars(&lines.join("\n\n"), MAX_INBOX_CHARS)
    };

    let active_skills_block = if active_skills.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nSkill runtime được inject:\n{}",
            render_active_skills_block(active_skills)
        )
    };

    format!(
        "Tin nhắn trong team session:\n{inbox}{active_skills_block}\n\n\
        Yêu cầu: Xử lý directive mới nhất từ Kuromi. Dùng tool nếu cần xác minh. \
        Kết thúc bằng báo cáo ngắn cho Kuromi."
    )
}

pub(super) fn build_directive_content(
    request: &SpawnTeamRequest,
    exchange: usize,
    is_last: bool,
    session: &TeamSession,
) -> String {
    let briefing = request
        .briefing
        .as_deref()
        .map(|v| format!("\nBổ sung: {v}"))
        .unwrap_or_default();

    if exchange == 0 {
        // First exchange: full mission brief
        format!(
            "Mission: {mission}{briefing}\n\n\
            Đây là exchange đầu tiên. Mỗi thành viên hãy phân tích theo trách nhiệm riêng, \
            dùng tool để thu thập dữ liệu và đưa ra nhận định ban đầu.",
            mission = request.mission,
        )
    } else if is_last {
        // Last exchange: ask for final reports
        let responses_summary = summarize_latest_responses(session);
        format!(
            "Exchange cuối cùng. Tóm tắt responses trước:\n{responses_summary}\n\n\
            Đây là lượt cuối — hãy chốt báo cáo cuối cùng. \
            Nếu có insight mới từ tool, bổ sung vào. \
            Nếu không, tổng hợp lại kết luận rõ ràng cho Kuromi."
        )
    } else {
        // Middle exchanges: continue discussion
        let responses_summary = summarize_latest_responses(session);
        format!(
            "Tiếp tục thảo luận. Tóm tắt responses trước:\n{responses_summary}\n\n\
            Hãy phản hồi, bổ sung hoặc phản biện ý kiến từ các thành viên khác. \
            Dùng tool nếu cần kiểm chứng thêm."
        )
    }
}

pub(super) fn build_runtime_context_preview(
    request: &SpawnTeamRequest,
    history: &[ChatTurn],
    context_preview: Option<&str>,
    session: &TeamSession,
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

    let msg_count = session.messages().len();
    if msg_count > 0 {
        blocks.push(format!(
            "Team session {} — {} messages logged",
            session.session_id(),
            msg_count
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

fn summarize_latest_responses(session: &TeamSession) -> String {
    let responses: Vec<String> = session
        .messages()
        .iter()
        .rev()
        .filter(|msg| msg.kind == TeamMessageKind::Response)
        .take(6)
        .map(|msg| {
            format!(
                "- {} => {}",
                msg.from,
                truncate_chars(&collapse_whitespace(&msg.content), 280)
            )
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if responses.is_empty() {
        "Chưa có response nào.".to_string()
    } else {
        responses.join("\n")
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
