use super::super::models::ChatTurn;

#[derive(Debug, Clone)]
pub(super) struct HistoryCompaction {
    pub(super) system_prompt: String,
    pub(super) history: Vec<ChatTurn>,
    pub(super) debug: HistoryCompactionDebugData,
}

#[derive(Debug, Clone)]
pub(super) struct HistoryCompactionDebugData {
    pub(super) compacted: bool,
    pub(super) original_history_count: usize,
    pub(super) retained_history_count: usize,
    pub(super) compacted_turns: usize,
    pub(super) estimated_chars_before: usize,
    pub(super) estimated_chars_after: usize,
    pub(super) compact_mode: Option<&'static str>,
    pub(super) compact_summary_preview: Option<String>,
}

impl HistoryCompaction {
    pub(super) fn unchanged(system_prompt: &str, history: &[ChatTurn], message: &str) -> Self {
        let estimated_chars = estimate_conversation_chars(system_prompt, history, message);

        Self {
            system_prompt: system_prompt.to_string(),
            history: history.to_vec(),
            debug: HistoryCompactionDebugData {
                compacted: false,
                original_history_count: history.len(),
                retained_history_count: history.len(),
                compacted_turns: 0,
                estimated_chars_before: estimated_chars,
                estimated_chars_after: estimated_chars,
                compact_mode: None,
                compact_summary_preview: None,
            },
        }
    }

    #[allow(dead_code)]
    pub(super) fn is_more_compact_than(&self, other: &Self) -> bool {
        self.debug.estimated_chars_after < other.debug.estimated_chars_after
            || self.debug.retained_history_count < other.debug.retained_history_count
    }
}

pub(super) fn normalize_history(history: &[ChatTurn]) -> Vec<ChatTurn> {
    history
        .iter()
        .filter_map(|turn| {
            let role = turn.role.trim().to_ascii_lowercase();
            if !(role == "user" || role == "assistant") {
                return None;
            }
            let content = turn.content.trim();
            if content.is_empty() {
                return None;
            }
            Some(ChatTurn {
                role,
                content: content.to_string(),
            })
        })
        .collect()
}

pub(super) fn normalize_chat_session_id(chat_session_id: Option<String>) -> Option<String> {
    chat_session_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn compact_history_for_retry(
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
    keep_recent_turns: usize,
    summary_char_budget: usize,
    attempt: usize,
) -> HistoryCompaction {
    if history.is_empty() {
        return HistoryCompaction::unchanged(system_prompt, history, message);
    }

    let (effective_keep, effective_budget, mode_label): (usize, usize, &'static str) =
        match attempt {
            1 => (
                keep_recent_turns.min(history.len()),
                summary_char_budget,
                "retry-1",
            ),
            _ => (
                (keep_recent_turns / 2).max(1).min(2).min(history.len()),
                (summary_char_budget / 2).max(400),
                "retry-2",
            ),
        };

    let split_index = history.len().saturating_sub(effective_keep);
    let older = &history[..split_index];
    let recent = history[split_index..].to_vec();

    let compact_summary = build_structured_summary(older, effective_budget);
    let compacted_system_prompt = if compact_summary.trim().is_empty() {
        system_prompt.to_string()
    } else {
        format!(
            "{}\n\n{}\n\nKhi cần tham chiếu các lượt trước, ưu tiên bám theo phần tóm tắt compact này.",
            system_prompt, compact_summary
        )
    };

    let estimated_before = estimate_conversation_chars(system_prompt, history, message);
    let estimated_after = estimate_conversation_chars(&compacted_system_prompt, &recent, message);

    HistoryCompaction {
        system_prompt: compacted_system_prompt,
        history: recent,
        debug: HistoryCompactionDebugData {
            compacted: true,
            original_history_count: history.len(),
            retained_history_count: effective_keep,
            compacted_turns: split_index,
            estimated_chars_before: estimated_before,
            estimated_chars_after: estimated_after,
            compact_mode: Some(mode_label),
            compact_summary_preview: (!compact_summary.is_empty())
                .then(|| truncate_chars(&compact_summary, 240)),
        },
    }
}

fn build_structured_summary(turns: &[ChatTurn], budget: usize) -> String {
    if turns.is_empty() || budget < 120 {
        return String::new();
    }

    let mut user_requests: Vec<String> = Vec::new();
    let mut tool_results: Vec<String> = Vec::new();
    let mut analysis: Vec<String> = Vec::new();
    let mut last_user_topics: Vec<String> = Vec::new();

    for turn in turns {
        if turn.role == "user" {
            let cleaned = collapse_whitespace(&turn.content);
            user_requests.push(truncate_chars(&cleaned, 120));
            last_user_topics.push(truncate_chars(&cleaned, 60));
        } else {
            let content = &turn.content;

            if content.contains("Kết quả tool thật") {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("- ") && line.contains('[') && line.contains("| output:") {
                        if let Some(bracket_start) = line.find('[') {
                            if let Some(bracket_end) = line[bracket_start..].find(']') {
                                let tool_name = line[2..bracket_start].trim();
                                let status =
                                    &line[bracket_start + 1..bracket_start + bracket_end];
                                let output_part = line
                                    .rfind("| output: ")
                                    .map(|pos| &line[pos + 10..])
                                    .unwrap_or("");
                                tool_results.push(format!(
                                    "- {} [{}]: {}",
                                    tool_name,
                                    status,
                                    truncate_chars(&collapse_whitespace(output_part), 100)
                                ));
                            }
                        }
                    }
                }
            }

            if content.contains("Phản hồi gửi user:") {
                if let Some(pos) = content.find("Phản hồi gửi user:") {
                    let response_text = &content[pos + "Phản hồi gửi user:".len()..];
                    let cleaned = collapse_whitespace(response_text.trim());
                    if !cleaned.is_empty() {
                        analysis.push(truncate_chars(&cleaned, 200));
                    }
                }
            } else if !content.contains("Kết quả tool thật") {
                let cleaned = collapse_whitespace(content.trim());
                if !cleaned.is_empty() {
                    analysis.push(truncate_chars(&cleaned, 200));
                }
            }
        }
    }

    // Build sections
    let mut sections: Vec<String> = Vec::new();

    if !user_requests.is_empty() {
        let mut section = "### Yêu cầu của user\n".to_string();
        for req in &user_requests {
            section.push_str(&format!("- {}\n", req));
        }
        sections.push(section);
    }

    if !tool_results.is_empty() {
        let mut section = "### Kết quả tool\n".to_string();
        for result in &tool_results {
            section.push_str(&format!("{}\n", result));
        }
        sections.push(section);
    }

    if !analysis.is_empty() {
        let mut section = "### Phân tích & kết luận\n".to_string();
        for item in &analysis {
            section.push_str(&format!("- {}\n", item));
        }
        sections.push(section);
    }

    // Continuity context from last user topics
    let continuity = if !last_user_topics.is_empty() {
        let recent: Vec<&str> = last_user_topics
            .iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|s| s.as_str())
            .collect();
        format!(
            "### Ngữ cảnh liên tục\n- Chủ đề: {}\n",
            recent.join(", ")
        )
    } else {
        String::new()
    };

    // Header
    let header = format!("## Ngữ cảnh compact ({} lượt)\n\n", turns.len());
    let mut result = header;

    // Budget enforcement: include all sections if within budget, otherwise drop
    // continuity first, then trim remaining sections
    let total_size: usize = result.len()
        + sections.iter().map(|s| s.len() + 1).sum::<usize>()
        + continuity.len();

    if total_size <= budget {
        for section in &sections {
            result.push_str(section);
            result.push('\n');
        }
        if !continuity.is_empty() {
            result.push_str(&continuity);
        }
    } else {
        // Drop continuity first
        let without_continuity: usize =
            result.len() + sections.iter().map(|s| s.len() + 1).sum::<usize>();
        if without_continuity <= budget {
            for section in &sections {
                result.push_str(section);
                result.push('\n');
            }
        } else {
            // Add what fits
            for section in &sections {
                if result.len() + section.len() + 1 <= budget {
                    result.push_str(section);
                    result.push('\n');
                }
            }
        }
    }

    truncate_chars(result.trim(), budget)
}

pub(super) fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
    if char_count(value) <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

fn estimate_conversation_chars(system_prompt: &str, history: &[ChatTurn], message: &str) -> usize {
    char_count(system_prompt)
        + char_count(message)
        + history
            .iter()
            .map(|turn| char_count(&turn.role) + char_count(&turn.content) + 24)
            .sum::<usize>()
        + 64
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::models::ChatTurn;

    fn make_turn(role: &str, content: &str) -> ChatTurn {
        ChatTurn {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn compact_history_for_retry_empty_history() {
        let result = compact_history_for_retry("system", &[], "msg", 6, 3200, 1);
        assert!(!result.debug.compacted);
        assert_eq!(result.history.len(), 0);
    }

    #[test]
    fn compact_history_for_retry_attempt1_keeps_recent() {
        let history: Vec<ChatTurn> = (0..10)
            .map(|i| {
                make_turn(
                    if i % 2 == 0 { "user" } else { "assistant" },
                    &format!("turn {i}"),
                )
            })
            .collect();

        let result = compact_history_for_retry("system", &history, "msg", 4, 3200, 1);
        assert!(result.debug.compacted);
        assert_eq!(result.debug.retained_history_count, 4);
        assert_eq!(result.debug.compacted_turns, 6);
        assert_eq!(result.history.len(), 4);
        assert_eq!(result.debug.compact_mode, Some("retry-1"));
    }

    #[test]
    fn compact_history_for_retry_attempt2_more_aggressive() {
        let history: Vec<ChatTurn> = (0..10)
            .map(|i| {
                make_turn(
                    if i % 2 == 0 { "user" } else { "assistant" },
                    &format!("turn {i}"),
                )
            })
            .collect();

        let result = compact_history_for_retry("system", &history, "msg", 6, 3200, 2);
        assert!(result.debug.compacted);
        assert!(result.debug.retained_history_count <= 2);
        assert_eq!(result.debug.compact_mode, Some("retry-2"));
    }

    #[test]
    fn build_structured_summary_categorizes_turns() {
        let turns = vec![
            make_turn("user", "Phân tích XAU/USD"),
            make_turn(
                "assistant",
                "Kết quả tool thật trong turn này:\n- fetch_news [completed] | input: {} | output: 3 tin tức về Fed\n\nPhản hồi gửi user:\nXAU/USD đang tăng mạnh, RSI=65",
            ),
            make_turn("user", "Kiểm tra lịch kinh tế"),
            make_turn("assistant", "Lịch kinh tế tuần này có NFP"),
        ];

        let summary = build_structured_summary(&turns, 3200);
        assert!(summary.contains("Yêu cầu của user"));
        assert!(summary.contains("Phân tích XAU/USD"));
        assert!(summary.contains("Kết quả tool"));
        assert!(summary.contains("fetch_news"));
        assert!(summary.contains("Phân tích & kết luận"));
        assert!(summary.contains("Ngữ cảnh liên tục"));
    }

    #[test]
    fn build_structured_summary_empty_turns() {
        assert!(build_structured_summary(&[], 3200).is_empty());
    }

    #[test]
    fn build_structured_summary_respects_budget() {
        let turns: Vec<ChatTurn> = (0..20)
            .map(|i| {
                make_turn(
                    if i % 2 == 0 { "user" } else { "assistant" },
                    &format!(
                        "This is a long turn content that repeats many times for turn number {i}"
                    ),
                )
            })
            .collect();

        let summary = build_structured_summary(&turns, 300);
        // truncate_chars adds "..." (3 chars) when truncating
        assert!(char_count(&summary) <= 303);
    }
}
