use std::{process::Stdio, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use tokio::{process::Command, time::timeout};

use crate::agents::tool_runtime::runtime::ToolRuntime;
use crate::agents::tool_runtime::utils::{
    optional_raw_string_arg, optional_string_arg, resolve_requested_timeout, truncate_chars,
};

pub(crate) const DESCRIPTION: &str =
    "Chạy một lệnh bash ngắn trong workspace để debug hoặc thao tác nhanh.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "script": {
                "type": "string",
                "description": "Đoạn lệnh bash cần chạy"
            },
            "cwd": {
                "type": "string",
                "description": "Thư mục làm việc, relative theo workspace tool"
            },
            "timeout_ms": {
                "type": "integer",
                "minimum": 1,
                "description": "Timeout mong muốn, không vượt quá timeout của tool"
            }
        },
        "required": ["script"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(
    runtime: &ToolRuntime,
    arguments: Value,
    tool_timeout: Duration,
) -> Result<Value> {
    let script = optional_raw_string_arg(&arguments, "script")
        .or_else(|| optional_raw_string_arg(&arguments, "command"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `script`"))?;
    let cwd = runtime.resolve_command_cwd(optional_string_arg(&arguments, "cwd").as_deref())?;
    let effective_timeout = resolve_requested_timeout(&arguments, tool_timeout);

    let mut process = Command::new("bash");
    process
        .arg("-lc")
        .arg(&script)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let output = match timeout(effective_timeout, process.output()).await {
        Ok(result) => result.context("không thể chạy bash process")?,
        Err(_) => {
            bail!("bash vượt quá timeout {}ms", effective_timeout.as_millis())
        }
    };

    Ok(json!({
        "ok": output.status.success(),
        "script": script,
        "cwd": cwd.display().to_string(),
        "timeout_ms": effective_timeout.as_millis(),
        "exit_code": output.status.code(),
        "success": output.status.success(),
        "stdout": truncate_chars(&String::from_utf8_lossy(&output.stdout), 3_000),
        "stderr": truncate_chars(&String::from_utf8_lossy(&output.stderr), 3_000),
    }))
}
