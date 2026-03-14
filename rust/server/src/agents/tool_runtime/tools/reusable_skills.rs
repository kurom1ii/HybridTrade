use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use tokio::fs;

use crate::agents::tool_runtime::utils::required_string_arg;

const SKILLS_DIR: &str = ".skills/learned";

pub(crate) const DESCRIPTION: &str =
    "Quản lý site-specific skills tự học: save/load/list/delete/match. Agent tự build skill cho mỗi website đã ghé thăm (selectors, workflows, scripts) và tự động tái sử dụng ở lần sau.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["save", "load", "list", "delete", "match"],
                "description": "Hành động cần thực hiện:\n- save: Lưu skill mới cho domain\n- load: Tải skill theo domain\n- list: Liệt kê tất cả learned skills\n- delete: Xoá skill theo domain\n- match: Tìm skill phù hợp nhất cho URL"
            },
            "domain": {
                "type": "string",
                "description": "Domain của website (ví dụ: forexfactory.com, investing.com). Dùng cho save/load/delete."
            },
            "url": {
                "type": "string",
                "description": "URL đầy đủ để tìm skill phù hợp. Dùng cho action=match."
            },
            "skill_data": {
                "type": "object",
                "description": "Dữ liệu skill cần lưu (action=save). Chứa selectors, workflows, scripts, tips.",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Tên mô tả ngắn cho skill"
                    },
                    "description": {
                        "type": "string",
                        "description": "Mô tả chi tiết skill làm gì"
                    },
                    "pages": {
                        "type": "object",
                        "description": "Map từ page_type (news, calendar, technical, article, search) tới config riêng",
                        "additionalProperties": {
                            "type": "object",
                            "properties": {
                                "url_pattern": {
                                    "type": "string",
                                    "description": "URL pattern hoặc path cho page type này"
                                },
                                "selectors": {
                                    "type": "object",
                                    "description": "CSS selectors cho các phần tử quan trọng",
                                    "additionalProperties": { "type": "string" }
                                },
                                "extract_script": {
                                    "type": "string",
                                    "description": "JavaScript function body cho evaluate_script"
                                },
                                "wait_for": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Các text cần chờ xuất hiện trước khi tương tác"
                                },
                                "cookie_dismiss": {
                                    "type": "string",
                                    "description": "Selector hoặc uid để dismiss popup cookie consent"
                                }
                            }
                        }
                    },
                    "tips": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Mẹo/lưu ý khi tương tác với website này"
                    },
                    "last_verified": {
                        "type": "string",
                        "description": "Ngày lần cuối kiểm tra skill này hoạt động (YYYY-MM-DD)"
                    }
                }
            }
        },
        "required": ["action"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(arguments: Value) -> Result<Value> {
    let action = required_string_arg(&arguments, "action")?;

    match action.as_str() {
        "save" => execute_save(&arguments).await,
        "load" => execute_load(&arguments).await,
        "list" => execute_list().await,
        "delete" => execute_delete(&arguments).await,
        "match" => execute_match(&arguments).await,
        _ => bail!("action `{action}` không hợp lệ, cần: save/load/list/delete/match"),
    }
}

async fn ensure_skills_dir() -> Result<std::path::PathBuf> {
    let dir = std::path::PathBuf::from(SKILLS_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .await
            .with_context(|| format!("không thể tạo thư mục {SKILLS_DIR}"))?;
    }
    Ok(dir)
}

fn domain_to_filename(domain: &str) -> String {
    domain
        .trim()
        .to_ascii_lowercase()
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        + ".json"
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

async fn execute_save(arguments: &Value) -> Result<Value> {
    let domain = required_string_arg(arguments, "domain")?;
    let skill_data = arguments
        .get("skill_data")
        .ok_or_else(|| anyhow::anyhow!("thiếu skill_data cho action=save"))?;

    let dir = ensure_skills_dir().await?;
    let filename = domain_to_filename(&domain);
    let filepath = dir.join(&filename);
    let existed = filepath.exists();

    let mut data = skill_data.clone();
    if let Value::Object(ref mut map) = data {
        map.insert("domain".to_string(), Value::String(domain.clone()));
        if !map.contains_key("last_verified") {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            map.insert("last_verified".to_string(), Value::String(today));
        }
    }

    let content = serde_json::to_string_pretty(&data)
        .context("không thể serialize skill_data")?;

    fs::write(&filepath, content.as_bytes())
        .await
        .with_context(|| format!("không thể ghi file `{}`", filepath.display()))?;

    Ok(json!({
        "ok": true,
        "action": "save",
        "domain": domain,
        "file": filepath.display().to_string(),
        "created": !existed,
        "message": if existed {
            format!("Đã cập nhật skill cho {domain}")
        } else {
            format!("Đã tạo skill mới cho {domain}")
        }
    }))
}

async fn execute_load(arguments: &Value) -> Result<Value> {
    let domain = required_string_arg(arguments, "domain")?;
    let dir = ensure_skills_dir().await?;
    let filename = domain_to_filename(&domain);
    let filepath = dir.join(&filename);

    if !filepath.exists() {
        return Ok(json!({
            "ok": false,
            "action": "load",
            "domain": domain,
            "found": false,
            "message": format!("Chưa có learned skill cho {domain}")
        }));
    }

    let content = fs::read_to_string(&filepath)
        .await
        .with_context(|| format!("không thể đọc file `{}`", filepath.display()))?;
    let data: Value = serde_json::from_str(&content)
        .with_context(|| format!("file `{}` không phải JSON hợp lệ", filepath.display()))?;

    Ok(json!({
        "ok": true,
        "action": "load",
        "domain": domain,
        "found": true,
        "skill": data,
    }))
}

async fn execute_list() -> Result<Value> {
    let dir = ensure_skills_dir().await?;
    let mut entries = Vec::new();

    let mut read_dir = fs::read_dir(&dir)
        .await
        .with_context(|| format!("không thể đọc thư mục `{}`", dir.display()))?;

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let domain = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let metadata = fs::metadata(&path).await.ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        let summary = match fs::read_to_string(&path).await {
            Ok(content) => serde_json::from_str::<Value>(&content)
                .ok()
                .and_then(|v| {
                    let name = v.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                    let desc = v.get("description").and_then(Value::as_str).unwrap_or("").to_string();
                    let last_verified = v
                        .get("last_verified")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let pages_count = v
                        .get("pages")
                        .and_then(Value::as_object)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    Some(json!({
                        "name": name,
                        "description": desc,
                        "last_verified": last_verified,
                        "pages_count": pages_count,
                    }))
                }),
            Err(_) => None,
        };

        entries.push(json!({
            "domain": domain,
            "file": path.display().to_string(),
            "size_bytes": size,
            "summary": summary,
        }));
    }

    entries.sort_by(|a, b| {
        let da = a.get("domain").and_then(Value::as_str).unwrap_or("");
        let db = b.get("domain").and_then(Value::as_str).unwrap_or("");
        da.cmp(db)
    });

    Ok(json!({
        "ok": true,
        "action": "list",
        "count": entries.len(),
        "skills": entries,
    }))
}

async fn execute_delete(arguments: &Value) -> Result<Value> {
    let domain = required_string_arg(arguments, "domain")?;
    let dir = ensure_skills_dir().await?;
    let filename = domain_to_filename(&domain);
    let filepath = dir.join(&filename);

    if !filepath.exists() {
        return Ok(json!({
            "ok": false,
            "action": "delete",
            "domain": domain,
            "message": format!("Không tìm thấy skill cho {domain} để xoá")
        }));
    }

    fs::remove_file(&filepath)
        .await
        .with_context(|| format!("không thể xoá file `{}`", filepath.display()))?;

    Ok(json!({
        "ok": true,
        "action": "delete",
        "domain": domain,
        "message": format!("Đã xoá skill cho {domain}")
    }))
}

async fn execute_match(arguments: &Value) -> Result<Value> {
    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow::anyhow!("thiếu tham số `url` cho action=match"))?;

    let domain = match extract_domain_from_url(url) {
        Some(d) => d,
        None => {
            return Ok(json!({
                "ok": false,
                "action": "match",
                "url": url,
                "found": false,
                "message": "Không thể trích xuất domain từ URL"
            }));
        }
    };

    let dir = ensure_skills_dir().await?;
    let filename = domain_to_filename(&domain);
    let filepath = dir.join(&filename);

    if !filepath.exists() {
        return Ok(json!({
            "ok": true,
            "action": "match",
            "url": url,
            "domain": domain,
            "found": false,
            "message": format!("Chưa có learned skill cho {domain}. Hãy khám phá website và save skill mới sau khi xong.")
        }));
    }

    let content = fs::read_to_string(&filepath)
        .await
        .with_context(|| format!("không thể đọc file `{}`", filepath.display()))?;
    let data: Value = serde_json::from_str(&content)
        .with_context(|| format!("file `{}` không phải JSON hợp lệ", filepath.display()))?;

    // Tìm page config phù hợp nhất dựa trên URL path
    let url_path = url
        .find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url)
        .split('/')
        .skip(1)
        .collect::<Vec<_>>()
        .join("/");

    let matched_page = data
        .get("pages")
        .and_then(Value::as_object)
        .and_then(|pages| {
            pages.iter().find(|(_, config)| {
                config
                    .get("url_pattern")
                    .and_then(Value::as_str)
                    .map(|pattern| url_path.contains(pattern) || url.contains(pattern))
                    .unwrap_or(false)
            })
        })
        .map(|(key, config)| {
            json!({
                "page_type": key,
                "config": config,
            })
        });

    Ok(json!({
        "ok": true,
        "action": "match",
        "url": url,
        "domain": domain,
        "found": true,
        "skill": data,
        "matched_page": matched_page,
        "message": format!("Tìm thấy learned skill cho {domain}. Dùng selectors và scripts trong skill để tương tác hiệu quả.")
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
        assert_eq!(domain_to_filename("forexfactory.com"), "forexfactory.com.json");
        assert_eq!(domain_to_filename("investing.com"), "investing.com.json");
    }
}
