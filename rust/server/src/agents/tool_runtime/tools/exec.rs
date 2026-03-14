use std::{process::Stdio, time::Duration};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use tokio::{process::Command, time::timeout};

use crate::agents::tool_runtime::runtime::ToolRuntime;
use crate::agents::tool_runtime::utils::{
    optional_string_arg, required_string_arg, resolve_requested_timeout, string_array_arg,
    truncate_chars,
};

pub(crate) const DESCRIPTION: &str =
    "Chạy một executable trực tiếp trong workspace mà không qua shell.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "Tên executable cần chạy"
            },
            "args": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Danh sách argument truyền vào executable"
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
        "required": ["command"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(
    runtime: &ToolRuntime,
    arguments: Value,
    tool_timeout: Duration,
) -> Result<Value> {
    let command = required_string_arg(&arguments, "command")?;
    let args = string_array_arg(&arguments, "args");
    let cwd = runtime.resolve_command_cwd(optional_string_arg(&arguments, "cwd").as_deref())?;
    let effective_timeout = resolve_requested_timeout(&arguments, tool_timeout);

    let mut process = Command::new(&command);
    process
        .args(&args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let output = match timeout(effective_timeout, process.output()).await {
        Ok(result) => result.with_context(|| format!("không thể chạy lệnh `{command}`"))?,
        Err(_) => {
            bail!(
                "exec `{command}` vượt quá timeout {}ms",
                effective_timeout.as_millis()
            )
        }
    };

    Ok(json!({
        "ok": output.status.success(),
        "command": command,
        "args": args,
        "cwd": cwd.display().to_string(),
        "timeout_ms": effective_timeout.as_millis(),
        "exit_code": output.status.code(),
        "success": output.status.success(),
        "stdout": truncate_chars(&String::from_utf8_lossy(&output.stdout), 3_000),
        "stderr": truncate_chars(&String::from_utf8_lossy(&output.stderr), 3_000),
    }))
}
