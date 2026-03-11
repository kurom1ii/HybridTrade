use crate::config::ProviderConfig;

use super::super::models::{AgentRole, ChatTurn};

#[derive(Debug, Clone, Copy)]
pub(super) enum CompactMode {
    Normal,
    Aggressive,
}

#[derive(Debug, Clone, Copy)]
struct CompactSettings {
    threshold_chars: usize,
    target_chars: usize,
    summary_chars: usize,
    keep_recent_turns: usize,
}

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

impl CompactMode {
    fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Aggressive => "aggressive",
        }
    }
}

impl HistoryCompaction {
    fn unchanged(system_prompt: &str, history: &[ChatTurn], message: &str) -> Self {
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

pub(super) fn compact_history_for_provider(
    config: &ProviderConfig,
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
    mode: CompactMode,
) -> HistoryCompaction {
    if history.is_empty() {
        return HistoryCompaction::unchanged(system_prompt, history, message);
    }

    let settings = compact_settings(config, mode);
    let estimated_before = estimate_conversation_chars(system_prompt, history, message);
    if estimated_before <= settings.threshold_chars {
        return HistoryCompaction::unchanged(system_prompt, history, message);
    }

    let max_keep_recent = history.len().min(settings.keep_recent_turns);
    let mut fallback = None;

    for keep_recent in (0..=max_keep_recent).rev() {
        let split_index = history.len().saturating_sub(keep_recent);
        let older = &history[..split_index];
        let recent = history[split_index..].to_vec();
        let base_chars = estimate_conversation_chars(system_prompt, &recent, message);
        let summary_budget = settings
            .target_chars
            .saturating_sub(base_chars)
            .min(settings.summary_chars);
        let compact_summary = summarize_compacted_turns(older, summary_budget);
        let compacted_system_prompt = append_compact_summary(system_prompt, &compact_summary);
        let estimated_after =
            estimate_conversation_chars(&compacted_system_prompt, &recent, message);

        let candidate = HistoryCompaction {
            system_prompt: compacted_system_prompt,
            history: recent,
            debug: HistoryCompactionDebugData {
                compacted: true,
                original_history_count: history.len(),
                retained_history_count: keep_recent,
                compacted_turns: history.len().saturating_sub(keep_recent),
                estimated_chars_before: estimated_before,
                estimated_chars_after: estimated_after,
                compact_mode: Some(mode.label()),
                compact_summary_preview: (!compact_summary.is_empty())
                    .then(|| truncate_chars(&compact_summary, 240)),
            },
        };

        if estimated_after <= settings.target_chars || keep_recent == 0 {
            return candidate;
        }

        fallback = Some(candidate);
    }

    fallback.unwrap_or_else(|| HistoryCompaction::unchanged(system_prompt, history, message))
}

pub(super) fn looks_like_context_limit_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "context length",
        "maximum context",
        "context window",
        "too many tokens",
        "prompt is too long",
        "input is too long",
        "message is too long",
        "token limit",
        "too long",
        "max context",
        "prompt_tokens",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
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

fn compact_settings(config: &ProviderConfig, mode: CompactMode) -> CompactSettings {
    let threshold_chars = config.compact_threshold_chars.max(4_000);
    let target_chars = config.compact_target_chars.max(2_000).min(threshold_chars);
    let summary_chars = config.compact_summary_chars.max(400);
    let keep_recent_turns = config.compact_keep_recent_turns.max(2);

    match mode {
        CompactMode::Normal => CompactSettings {
            threshold_chars,
            target_chars,
            summary_chars,
            keep_recent_turns,
        },
        CompactMode::Aggressive => CompactSettings {
            threshold_chars: 0,
            target_chars: (target_chars / 2).max(4_000),
            summary_chars: (summary_chars / 2).max(800),
            keep_recent_turns: (keep_recent_turns / 2).max(2),
        },
    }
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

fn summarize_compacted_turns(turns: &[ChatTurn], max_chars: usize) -> String {
    if turns.is_empty() || max_chars < 80 {
        return String::new();
    }

    let mut summary = format!("{} lượt chat cũ đã được compact:\n", turns.len());
    let mut included = 0usize;

    for turn in turns {
        let role_label = if turn.role == "user" {
            "User"
        } else {
            "Assistant"
        };
        let cleaned = collapse_whitespace(&turn.content);
        let line = format!("- {}: {}\n", role_label, truncate_chars(&cleaned, 180));
        if char_count(&(summary.clone() + &line)) > max_chars {
            break;
        }
        summary.push_str(&line);
        included += 1;
    }

    if included < turns.len() {
        let line = format!(
            "- ... còn {} lượt cũ hơn đã được rút gọn thêm.\n",
            turns.len() - included
        );
        if char_count(&(summary.clone() + &line)) <= max_chars {
            summary.push_str(&line);
        }
    }

    truncate_chars(summary.trim(), max_chars)
}

fn append_compact_summary(system_prompt: &str, compact_summary: &str) -> String {
    if compact_summary.trim().is_empty() {
        return system_prompt.to_string();
    }

    format!(
        "{}\n\nNgữ cảnh hội thoại cũ đã được compact:\n{}\n\nKhi cần tham chiếu các lượt trước, ưu tiên bám theo phần tóm tắt compact này.",
        system_prompt, compact_summary
    )
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

#[allow(dead_code)]
fn _cached_tool_runtime_key(role: AgentRole, chat_session_id: &str) -> String {
    format!("{}::{chat_session_id}", role.as_str())
}
