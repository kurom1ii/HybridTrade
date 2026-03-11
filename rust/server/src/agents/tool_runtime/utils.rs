use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

pub(super) const MAX_TOOL_OUTPUT_CHARS: usize = 128000;
pub(super) const MAX_TOOL_PREVIEW_CHARS: usize = 32000;

pub(super) fn required_string_arg(arguments: &Value, field: &str) -> Result<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `{field}`"))
}

pub(super) fn required_raw_string_arg(arguments: &Value, field: &str) -> Result<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `{field}`"))
}

pub(super) fn optional_string_arg(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn optional_raw_string_arg(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub(super) fn optional_bool_arg(arguments: &Value, field: &str) -> Option<bool> {
    arguments.get(field).and_then(Value::as_bool)
}

pub(super) fn optional_usize_arg(arguments: &Value, field: &str) -> Option<usize> {
    arguments
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn optional_u64_arg(arguments: &Value, field: &str) -> Option<u64> {
    arguments.get(field).and_then(Value::as_u64)
}

pub(super) fn string_array_arg(arguments: &Value, field: &str) -> Vec<String> {
    arguments
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn render_tool_output_for_model(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return truncate_chars(text, MAX_TOOL_OUTPUT_CHARS);
    }

    if let Some(text) = extract_text_from_tool_result(value) {
        return truncate_chars(&collapse_whitespace(&text), MAX_TOOL_OUTPUT_CHARS);
    }

    let serialized = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    truncate_chars(&serialized, MAX_TOOL_OUTPUT_CHARS)
}

pub(super) fn tool_output_is_error(value: &Value) -> bool {
    value
        .get("isError")
        .and_then(Value::as_bool)
        .or_else(|| value.get("is_error").and_then(Value::as_bool))
        .unwrap_or(false)
}

pub(super) fn sanitize_tool_arguments(value: Value) -> Value {
    match value {
        Value::Null => json!({}),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .filter_map(|(key, value)| {
                    let value = sanitize_tool_arguments(value);
                    if value.is_null() {
                        None
                    } else {
                        Some((key, value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(sanitize_tool_arguments)
                .filter(|value| !value.is_null())
                .collect(),
        ),
        other => other,
    }
}

fn extract_text_from_tool_result(value: &Value) -> Option<String> {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return Some(text.to_string());
        }
    }

    let items = value.get("content")?.as_array()?;
    let text = items
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");

    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

pub(super) fn extract_html_title(body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let Some(tag_end) = lower[start..].find('>') else {
        return String::new();
    };
    let content_start = start + tag_end + 1;
    let Some(end) = lower[content_start..].find("</title>") else {
        return String::new();
    };

    collapse_whitespace(&body[content_start..content_start + end])
}

pub(super) fn strip_html_tags(body: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut inside_tag = false;

    for ch in body.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

pub(super) fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

pub(super) fn count_keyword_hits(haystack: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| haystack.contains(**keyword))
        .count()
}

pub(super) fn find_timeframes(lowered: &str) -> Vec<&'static str> {
    ["1m", "5m", "15m", "1h", "4h", "1d", "1w", "daily", "weekly"]
        .into_iter()
        .filter(|timeframe| lowered.contains(timeframe))
        .collect()
}

pub(super) fn collect_keywords(lowered: &str) -> Vec<&'static str> {
    [
        "support",
        "resistance",
        "breakout",
        "breakdown",
        "volume",
        "trend",
        "range",
        "retest",
    ]
    .into_iter()
    .filter(|keyword| lowered.contains(keyword))
    .collect()
}

pub(super) fn extract_candidate_numbers(text: &str) -> Vec<String> {
    let mut current = String::new();
    let mut values = Vec::new();

    for ch in text.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | ',' | '/') {
            current.push(ch);
        } else {
            push_number_candidate(&mut values, &mut current);
        }
    }
    push_number_candidate(&mut values, &mut current);

    values
}

fn push_number_candidate(values: &mut Vec<String>, current: &mut String) {
    let candidate = current.trim_matches(|ch: char| ch == '.' || ch == ',' || ch == '/');
    if candidate.chars().filter(|ch| ch.is_ascii_digit()).count() >= 3
        && !values.iter().any(|value| value == candidate)
    {
        values.push(candidate.to_string());
    }
    current.clear();
}

pub(super) fn extract_domain(value: &str) -> Option<String> {
    reqwest::Url::parse(value)
        .ok()
        .and_then(|url| url.domain().map(str::to_string))
}

pub(super) fn resolve_workspace_root() -> Result<PathBuf> {
    let base = env::var("HYBRIDTRADE_TOOL_ROOT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().context("không thể xác định current_dir cho native tools")?);

    base.canonicalize()
        .with_context(|| format!("không thể canonicalize workspace root `{}`", base.display()))
}

pub(super) fn best_effort_canonicalize(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

pub(super) fn normalize_path_for_workspace(candidate: &Path) -> Result<PathBuf> {
    let mut existing = candidate;
    let mut suffix = Vec::<OsString>::new();

    loop {
        if existing.exists() {
            let mut normalized = existing
                .canonicalize()
                .with_context(|| format!("không thể canonicalize path `{}`", existing.display()))?;
            for part in suffix.iter().rev() {
                normalized.push(part);
            }
            return Ok(normalized);
        }

        let Some(name) = existing.file_name() else {
            bail!("path `{}` không hợp lệ", candidate.display());
        };
        suffix.push(name.to_os_string());
        existing = existing
            .parent()
            .ok_or_else(|| anyhow!("path `{}` không hợp lệ", candidate.display()))?;
    }
}

pub(super) fn ensure_path_is_within_workspace(path: &Path, workspace_root: &Path) -> Result<()> {
    if path.starts_with(workspace_root) {
        Ok(())
    } else {
        bail!(
            "path `{}` nằm ngoài workspace `{}`",
            path.display(),
            workspace_root.display()
        )
    }
}

pub(super) fn resolve_requested_timeout(arguments: &Value, max_timeout: Duration) -> Duration {
    let max_ms = max_timeout.as_millis().min(u128::from(u64::MAX)) as u64;
    optional_u64_arg(arguments, "timeout_ms")
        .map(|value| value.max(1).min(max_ms))
        .map(Duration::from_millis)
        .unwrap_or(max_timeout)
}
