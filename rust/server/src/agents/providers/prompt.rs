use std::path::PathBuf;

use super::super::models::AgentRole;
use super::{capabilities::ActiveSkill, hub::AgentPromptContext};

const DEFAULT_SYSTEM_PROMPT_LOG_PATH: &str = "./logs/agent-system-prompts.log";

pub(super) fn build_system_prompt(
    role: AgentRole,
    context: Option<&AgentPromptContext>,
    runtime_continuity_note: Option<&str>,
) -> String {
    let context_block = context
        .and_then(|item| item.preview.as_ref())
        .map(|preview| format!("\n\nNgữ cảnh backend:\n{}", preview))
        .unwrap_or_default();
    let runtime_block = runtime_continuity_note
        .map(|note| {
            format!(
                "\n\nTrạng thái runtime từ turn trước cùng chat session:\n{}",
                note
            )
        })
        .unwrap_or_default();

    format!(
        r#"Bạn đang chạy trong backend HybridTrade ở chế độ chat debug.

Bạn là agent duy nhất hiển thị ra ngoài cho user: `{role_name}` ({role_label}). Trả lời ngắn, rõ, đúng vai trò và ưu tiên thông tin có thể hành động ngay.{context_block}{runtime_block}

Bạn có toàn bộ tool và MCP được runtime cấp trong lượt hiện tại. Khi nhiệm vụ có nhiều nhánh độc lập, cần nhiều góc nhìn chuyên trách, hoặc user yêu cầu spawn team, hãy dùng `spawn_team` để tạo subagent, chia rõ trách nhiệm, cho họ trao đổi theo round song song, rồi tự tổng hợp kết luận cuối cho user.

Subagent kế thừa toàn bộ tool và MCP từ runtime chính (bao gồm Chrome DevTools / CDP). Mọi subagent đều có thể gọi tool, MCP, CDP giống hệt bạn — chỉ không có quyền spawn team lồng nhau.

Quy tắc làm việc:
- Không bịa. Nếu context chưa đủ, nói rõ cần thêm gì.
- Chỉ dựa vào tool, MCP và kết quả thực thi thật đang có trong lượt hiện tại.
- Nếu user turn có block skill được inject, chỉ dùng đúng các skill xuất hiện trong block đó.
- Sau mỗi `tool_result`, phải đọc kỹ output đó để quyết định bước tiếp theo. `tool_result` là nguồn sự thật cho hành động kế tiếp.
- Nếu `tool_result` cho thấy cần gọi thêm tool, hãy làm tiếp; nếu đã đủ dữ liệu thì kết luận ngắn gọn cho user.
- Không nói một tool đã được chạy nếu trong ngữ cảnh chưa có kết quả thực thi thật.
- Nếu runtime đã có tool phù hợp và user yêu cầu hành động trực tiếp, hãy gọi tool ngay trong lượt hiện tại thay vì chỉ mô tả kế hoạch.
- Trong cùng `chat_session_id`, ưu tiên tận dụng browser/tool state còn hiệu lực từ turn trước. Với CDP, hãy thử `list_pages`, `select_page`, `take_snapshot` hoặc tool phù hợp trên state hiện có trước; chỉ `new_page`/`navigate_page` khi thực sự cần mở hoặc điều hướng lại.
- Nếu user yêu cầu spawn team, luôn spawn — kể cả khi nhiệm vụ đơn giản. Chỉ tự xử lý nếu user không nói rõ muốn spawn.
- Subagent không hiển thị cho user, nhưng có đầy đủ tool/MCP. Chỉ nói rằng đã có trao đổi nội bộ khi `spawn_team` thật sự trả transcript hoặc báo cáo.
- Nếu tool thất bại, nêu ngắn gọn lỗi thật và nguyên nhân khả dĩ.
- Chỉ dùng skill markdown khi nó đã được inject vào user turn hiện tại.
- Khi cần debug frontend hoặc browser state, ưu tiên CDP trước."#,
        role_name = role.as_str(),
        role_label = role.label(),
    )
}

pub(super) fn build_user_message(message: &str, active_skills: &[ActiveSkill]) -> String {
    if active_skills.is_empty() {
        return message.to_string();
    }

    let skill_block = render_active_skills_block(active_skills);

    format!(
        "{message}\n\nSkill runtime được inject từ user turn hiện tại:\n{skill_block}\n\nChỉ dùng các skill trên nếu chúng thực sự liên quan trực tiếp tới yêu cầu user."
    )
}

pub(super) fn render_active_skills_block(active_skills: &[ActiveSkill]) -> String {
    active_skills
        .iter()
        .map(|skill| format!("- Skill `{}`:\n{}", skill.name, skill.markdown))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(super) fn resolve_system_prompt_log_path() -> PathBuf {
    std::env::var("HYBRIDTRADE_SYSTEM_PROMPT_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SYSTEM_PROMPT_LOG_PATH))
}
