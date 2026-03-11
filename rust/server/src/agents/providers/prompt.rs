use std::path::PathBuf;

use super::super::models::AgentRole;
use super::{capabilities::AgentPromptProfile, hub::AgentPromptContext};

const DEFAULT_SYSTEM_PROMPT_LOG_PATH: &str = "./logs/agent-system-prompts.log";

pub(super) fn build_system_prompt(
    role: AgentRole,
    context: Option<&AgentPromptContext>,
    prompt_profile: &AgentPromptProfile,
) -> String {
    let context_block = context
        .and_then(|item| item.preview.as_ref())
        .map(|preview| format!("\n\nNgữ cảnh backend:\n{}", preview))
        .unwrap_or_default();

    let common_skills = render_markdown_block(
        &prompt_profile.common_markdown,
        "# Skills chung\n\nChưa có file Markdown nào trong `.skills/common`.",
    );
    let agent_skills = render_markdown_block(
        &prompt_profile.agent_markdown,
        &format!(
            "# Skill riêng {}\n\n- Bạn là {}. Trả lời ngắn, rõ và đúng vai trò.\n- Hiện chưa có file Markdown riêng trong `.skills/agents` cho role này.",
            role.as_str(),
            role.label()
        ),
    );
    format!(
        r#"Bạn đang chạy trong backend HybridTrade ở chế độ chat debug.

Bạn là agent duy nhất hiển thị ra ngoài cho user: `{role_name}` ({role_label}). Trả lời ngắn, rõ, đúng vai trò và ưu tiên thông tin phục vụ debug.{context_block}

Bạn có toàn bộ tool, MCP và skills đang được runtime cấp trong lượt này. Khi cần nhiều góc nhìn chuyên trách, hãy dùng tool `spawn_team` để tự tạo một team subagent động, cho họ trao đổi với nhau, rồi dựa trên báo cáo trả về để kết luận cho user.

Tài liệu kỹ năng chung nạp từ `.skills/common`:
{common_skills}

Tài liệu kỹ năng riêng của Kuromi nạp từ `.skills/agents`:
{agent_skills}

Quy tắc:
- Không bịa. Nếu context chưa đủ, nói rõ cần thêm gì.
- Chỉ dùng skill từ Markdown đã nạp và tool thực sự được runtime cấp riêng cho lượt hiện tại, không tự bịa skill nội bộ.
- Team con được spawn là runtime-only. Chỉ nói rằng đã có trao đổi nội bộ khi trong tool output thật sự có transcript/báo cáo từ `spawn_team`.
- Nếu user hỏi bạn có tool/MCP gì, chỉ trả lời theo capability thật sự đang được cấp ở runtime hiện tại.
- Nếu runtime đã nạp được tool phù hợp và user yêu cầu hành động trực tiếp như mở URL, xem DOM, network, console hoặc kiểm tra page, hãy gọi tool ngay trong lượt hiện tại thay vì chỉ mô tả kế hoạch.
- Bạn chỉ được nói một tool đã được chạy khi trong ngữ cảnh có kết quả thực thi thật.
- Nếu tool thất bại, nêu ngắn gọn lỗi thật và nguyên nhân khả dĩ thay vì xin xác nhận lại không cần thiết.
- Khi cần debug frontend hoặc browser state, ưu tiên đề xuất CDP trước."#,
        role_name = role.as_str(),
        role_label = role.label(),
    )
}

pub(super) fn resolve_system_prompt_log_path() -> PathBuf {
    std::env::var("HYBRIDTRADE_SYSTEM_PROMPT_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SYSTEM_PROMPT_LOG_PATH))
}

fn render_markdown_block(markdown: &str, fallback: &str) -> String {
    let markdown = markdown.trim();
    if markdown.is_empty() {
        fallback.to_string()
    } else {
        markdown.to_string()
    }
}
