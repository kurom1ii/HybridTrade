use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use tokio::fs;

use crate::agents::tool_runtime::runtime::ToolRuntime;
use crate::agents::tool_runtime::utils::{
    optional_usize_arg, required_string_arg, truncate_chars, MAX_TOOL_OUTPUT_CHARS,
};

pub(crate) const DESCRIPTION: &str =
    "Đọc file trong workspace của backend với cửa sổ dòng và giới hạn ký tự.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Path file cần đọc, relative theo workspace tool"
            },
            "start_line": {
                "type": "integer",
                "minimum": 1,
                "description": "Dòng bắt đầu, mặc định 1"
            },
            "line_count": {
                "type": "integer",
                "minimum": 1,
                "description": "Số dòng cần đọc, mặc định 200"
            },
            "max_chars": {
                "type": "integer",
                "minimum": 1,
                "description": "Giới hạn ký tự trả về"
            }
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(runtime: &ToolRuntime, arguments: Value) -> Result<Value> {
    let requested_path = required_string_arg(&arguments, "path")?;
    let path = runtime.resolve_workspace_path(&requested_path)?;
    let metadata = fs::metadata(&path)
        .await
        .with_context(|| format!("không thể đọc metadata `{}`", path.display()))?;

    if metadata.is_dir() {
        bail!(
            "`{}` là thư mục, tool `read` chỉ hỗ trợ file",
            path.display()
        );
    }

    let bytes = fs::read(&path)
        .await
        .with_context(|| format!("không thể đọc file `{}`", path.display()))?;
    let lossy_utf8 = std::str::from_utf8(&bytes).is_err();
    let content = String::from_utf8_lossy(&bytes).into_owned();
    let start_line = optional_usize_arg(&arguments, "start_line")
        .unwrap_or(1)
        .max(1);
    let line_count = optional_usize_arg(&arguments, "line_count")
        .unwrap_or(200)
        .clamp(1, 2_000);
    let max_chars = optional_usize_arg(&arguments, "max_chars")
        .unwrap_or(6_000)
        .clamp(1, MAX_TOOL_OUTPUT_CHARS);

    let lines = content.lines().collect::<Vec<_>>();
    let start_index = start_line.saturating_sub(1).min(lines.len());
    let end_index = (start_index + line_count).min(lines.len());
    let mut snippet = lines[start_index..end_index].join("\n");
    let truncated = end_index < lines.len() || snippet.chars().count() > max_chars;

    if snippet.chars().count() > max_chars {
        snippet = truncate_chars(&snippet, max_chars);
    }

    Ok(json!({
        "ok": true,
        "path": requested_path,
        "resolved_path": path.display().to_string(),
        "size_bytes": metadata.len(),
        "total_lines": lines.len(),
        "start_line": if lines.is_empty() { 0 } else { start_index + 1 },
        "end_line": end_index,
        "truncated": truncated,
        "lossy_utf8": lossy_utf8,
        "content": snippet,
    }))
}
