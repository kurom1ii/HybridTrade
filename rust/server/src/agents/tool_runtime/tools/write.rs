use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use tokio::{fs, io::AsyncWriteExt};

use crate::agents::tool_runtime::runtime::ToolRuntime;
use crate::agents::tool_runtime::utils::{
    optional_bool_arg, required_raw_string_arg, required_string_arg,
};

pub(crate) const DESCRIPTION: &str =
    "Ghi hoặc append nội dung text vào file trong workspace của backend.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Path file cần ghi, relative theo workspace tool"
            },
            "content": {
                "type": "string",
                "description": "Nội dung text cần ghi"
            },
            "append": {
                "type": "boolean",
                "description": "Nếu true sẽ append thay vì overwrite"
            },
            "create_parent_dirs": {
                "type": "boolean",
                "description": "Nếu true sẽ tự tạo thư mục cha còn thiếu"
            }
        },
        "required": ["path", "content"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(runtime: &ToolRuntime, arguments: Value) -> Result<Value> {
    let requested_path = required_string_arg(&arguments, "path")?;
    let content = required_raw_string_arg(&arguments, "content")?;
    let append = optional_bool_arg(&arguments, "append").unwrap_or(false);
    let create_parent_dirs = optional_bool_arg(&arguments, "create_parent_dirs").unwrap_or(true);
    let path = runtime.resolve_workspace_path(&requested_path)?;

    let Some(parent) = path.parent() else {
        bail!("không thể xác định thư mục cha cho `{}`", path.display());
    };

    if !parent.exists() {
        if create_parent_dirs {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("không thể tạo thư mục cha `{}`", parent.display()))?;
        } else {
            bail!("thư mục cha `{}` chưa tồn tại", parent.display());
        }
    }

    let existed = path.exists();
    if append {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .with_context(|| format!("không thể mở file `{}` để append", path.display()))?;
        file.write_all(content.as_bytes())
            .await
            .with_context(|| format!("không thể append vào `{}`", path.display()))?;
        file.flush().await.ok();
    } else {
        fs::write(&path, content.as_bytes())
            .await
            .with_context(|| format!("không thể ghi file `{}`", path.display()))?;
    }

    let metadata = fs::metadata(&path)
        .await
        .with_context(|| format!("không thể đọc metadata `{}` sau khi ghi", path.display()))?;

    Ok(json!({
        "ok": true,
        "path": requested_path,
        "resolved_path": path.display().to_string(),
        "mode": if append { "append" } else { "write" },
        "created": !existed,
        "bytes_written": content.len(),
        "size_bytes": metadata.len(),
    }))
}
