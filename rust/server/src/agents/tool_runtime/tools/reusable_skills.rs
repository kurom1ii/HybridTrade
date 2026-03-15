use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::agents::tool_runtime::utils::required_string_arg;

const LEARNED_DIR: &str = ".skills/learned";

pub(crate) const DESCRIPTION: &str =
    "Quản lý learned skills (.md): save/load/list/delete/match. Agent tự tạo skill dạng Markdown cho mỗi website/workflow đã học và tái sử dụng ở lần sau.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["save", "load", "list", "delete", "match"],
                "description": "Hành động cần thực hiện:\n- save: Lưu/cập nhật skill (Markdown)\n- load: Đọc skill theo tên\n- list: Liệt kê tất cả learned skills\n- delete: Xoá skill theo tên\n- match: Tìm skill phù hợp nhất cho URL hoặc keyword"
            },
            "name": {
                "type": "string",
                "description": "Tên skill (dùng làm filename, ví dụ: fxstreet-gold-news, investing-technical). Dùng cho save/load/delete."
            },
            "url": {
                "type": "string",
                "description": "URL hoặc keyword để tìm skill phù hợp. Dùng cho action=match."
            },
            "content": {
                "type": "string",
                "description": "Nội dung Markdown của skill (action=save). Chứa mô tả, selectors, scripts, tips, workflows."
            }
        },
        "required": ["action"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(arguments: Value, workspace_root: &Path) -> Result<Value> {
    let action = required_string_arg(&arguments, "action")?;

    match action.as_str() {
        "save" => execute_save(&arguments, workspace_root).await,
        "load" => execute_load(&arguments, workspace_root).await,
        "list" => execute_list(workspace_root).await,
        "delete" => execute_delete(&arguments, workspace_root).await,
        "match" => execute_match(&arguments, workspace_root).await,
        _ => bail!("action `{action}` không hợp lệ, cần: save/load/list/delete/match"),
    }
}

fn learned_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(LEARNED_DIR)
}

async fn ensure_learned_dir(workspace_root: &Path) -> Result<PathBuf> {
    let dir = learned_dir(workspace_root);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .await
            .with_context(|| format!("không thể tạo thư mục {}", dir.display()))?;
    }
    Ok(dir)
}

fn name_to_filename(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "-")
        + ".md"
}

fn extract_domain_from_url(url: &str) -> Option<String> {
    let url = url.trim();
    let after_scheme = if let Some(idx) = url.find("://") {
        &url[idx + 3..]
    } else {
        url
    };
    let host = after_scheme.split('/').next()?;
    let host = host.split(':').next()?;
    let host = host.trim().to_ascii_lowercase();

    if host.is_empty() {
        return None;
    }

    let host = host.strip_prefix("www.").unwrap_or(&host);
    Some(host.to_string())
}

async fn execute_save(arguments: &Value, workspace_root: &Path) -> Result<Value> {
    let name = required_string_arg(arguments, "name")?;
    let content = arguments
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("thiếu `content` (Markdown) cho action=save"))?;

    let dir = ensure_learned_dir(workspace_root).await?;
    let filename = name_to_filename(&name);
    let filepath = dir.join(&filename);
    let existed = filepath.exists();

    fs::write(&filepath, content.as_bytes())
        .await
        .with_context(|| format!("không thể ghi file `{}`", filepath.display()))?;

    Ok(json!({
        "ok": true,
        "action": "save",
        "name": name,
        "file": filepath.display().to_string(),
        "created": !existed,
        "message": if existed {
            format!("Đã cập nhật skill `{name}`")
        } else {
            format!("Đã tạo skill mới `{name}`")
        }
    }))
}

async fn execute_load(arguments: &Value, workspace_root: &Path) -> Result<Value> {
    let name = required_string_arg(arguments, "name")?;
    let dir = ensure_learned_dir(workspace_root).await?;
    let filename = name_to_filename(&name);
    let filepath = dir.join(&filename);

    if !filepath.exists() {
        return Ok(json!({
            "ok": false,
            "action": "load",
            "name": name,
            "found": false,
            "message": format!("Chưa có learned skill `{name}`")
        }));
    }

    let content = fs::read_to_string(&filepath)
        .await
        .with_context(|| format!("không thể đọc file `{}`", filepath.display()))?;

    Ok(json!({
        "ok": true,
        "action": "load",
        "name": name,
        "found": true,
        "content": content,
    }))
}

async fn execute_list(workspace_root: &Path) -> Result<Value> {
    let dir = ensure_learned_dir(workspace_root).await?;
    let mut entries = Vec::new();

    let mut read_dir = fs::read_dir(&dir)
        .await
        .with_context(|| format!("không thể đọc thư mục `{}`", dir.display()))?;

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let metadata = fs::metadata(&path).await.ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        // Đọc dòng đầu làm summary
        let summary = match fs::read_to_string(&path).await {
            Ok(content) => {
                let first_line = content
                    .lines()
                    .find(|l| {
                        let trimmed = l.trim();
                        !trimmed.is_empty() && !trimmed.starts_with('#')
                    })
                    .unwrap_or("")
                    .trim();
                if first_line.len() > 120 {
                    format!("{}...", &first_line[..120])
                } else {
                    first_line.to_string()
                }
            }
            Err(_) => String::new(),
        };

        entries.push(json!({
            "name": name,
            "file": path.display().to_string(),
            "size_bytes": size,
            "summary": summary,
        }));
    }

    entries.sort_by(|a, b| {
        let na = a.get("name").and_then(Value::as_str).unwrap_or("");
        let nb = b.get("name").and_then(Value::as_str).unwrap_or("");
        na.cmp(nb)
    });

    Ok(json!({
        "ok": true,
        "action": "list",
        "count": entries.len(),
        "skills": entries,
    }))
}

async fn execute_delete(arguments: &Value, workspace_root: &Path) -> Result<Value> {
    let name = required_string_arg(arguments, "name")?;
    let dir = ensure_learned_dir(workspace_root).await?;
    let filename = name_to_filename(&name);
    let filepath = dir.join(&filename);

    if !filepath.exists() {
        return Ok(json!({
            "ok": false,
            "action": "delete",
            "name": name,
            "message": format!("Không tìm thấy skill `{name}` để xoá")
        }));
    }

    fs::remove_file(&filepath)
        .await
        .with_context(|| format!("không thể xoá file `{}`", filepath.display()))?;

    Ok(json!({
        "ok": true,
        "action": "delete",
        "name": name,
        "message": format!("Đã xoá skill `{name}`")
    }))
}

async fn execute_match(arguments: &Value, workspace_root: &Path) -> Result<Value> {
    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow::anyhow!("thiếu tham số `url` cho action=match"))?;

    let domain = extract_domain_from_url(url);
    let keyword = domain.as_deref().unwrap_or(url).to_ascii_lowercase();

    let dir = ensure_learned_dir(workspace_root).await?;
    let mut matches = Vec::new();

    let mut read_dir = fs::read_dir(&dir)
        .await
        .with_context(|| format!("không thể đọc thư mục `{}`", dir.display()))?;

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // Match by filename hoặc nội dung chứa domain/keyword
        let name_match = name.contains(&keyword) || keyword.contains(&name);

        let content_match = if !name_match {
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    let lower = content.to_ascii_lowercase();
                    lower.contains(&keyword) || lower.contains(url)
                }
                Err(_) => false,
            }
        } else {
            false
        };

        if name_match || content_match {
            let content = fs::read_to_string(&path).await.unwrap_or_default();
            matches.push(json!({
                "name": path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                "file": path.display().to_string(),
                "content": content,
                "match_type": if name_match { "name" } else { "content" },
            }));
        }
    }

    if matches.is_empty() {
        return Ok(json!({
            "ok": true,
            "action": "match",
            "url": url,
            "found": false,
            "message": format!("Chưa có learned skill phù hợp cho `{url}`. Hãy tạo skill mới sau khi xong.")
        }));
    }

    Ok(json!({
        "ok": true,
        "action": "match",
        "url": url,
        "found": true,
        "matches": matches,
        "message": format!("Tìm thấy {} skill phù hợp. Đọc content để tái sử dụng.", matches.len())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_domain_strips_www() {
        assert_eq!(
            extract_domain_from_url("https://www.forexfactory.com/news"),
            Some("forexfactory.com".to_string())
        );
    }

    #[test]
    fn extract_domain_no_scheme() {
        assert_eq!(
            extract_domain_from_url("investing.com/technical"),
            Some("investing.com".to_string())
        );
    }

    #[test]
    fn extract_domain_with_port() {
        assert_eq!(
            extract_domain_from_url("http://localhost:3000/api"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn domain_to_filename_sanitizes() {
        assert_eq!(name_to_filename("fxstreet-gold-news"), "fxstreet-gold-news.md");
        assert_eq!(name_to_filename("investing.com"), "investing.com.md");
        assert_eq!(name_to_filename("My Cool Skill"), "my-cool-skill.md");
    }
}
