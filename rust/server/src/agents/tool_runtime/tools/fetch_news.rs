use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::agents::tool_runtime::utils::{optional_bool_arg, optional_usize_arg, truncate_chars};

pub(crate) const DESCRIPTION: &str =
    "Lấy tin tức tài chính mới nhất từ hệ thống. Trả về danh sách bài viết với tiêu đề, nội dung, thời gian và mức độ quan trọng.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "count": {
                "type": "integer",
                "minimum": 1,
                "maximum": 50,
                "description": "Số lượng tin tức cần lấy, mặc định 50"
            },
            "important": {
                "type": "boolean",
                "description": "Nếu true chỉ lấy tin quan trọng"
            }
        },
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(arguments: Value) -> Result<Value> {
    let count = optional_usize_arg(&arguments, "count")
        .unwrap_or(20)
        .clamp(1, 50);
    let important = optional_bool_arg(&arguments, "important").unwrap_or(false);

    let frontend_url = std::env::var("HYBRIDTRADE_FRONTEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let url = format!(
        "{}/api/news?pageSize={}&checkImportant={}",
        frontend_url, count, important
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("không thể tạo HTTP client")?;

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("không thể gọi {url}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "fetch_news trả về status {status}: {}",
            truncate_chars(&body, 500)
        );
    }

    let data: Value = response
        .json()
        .await
        .context("không thể parse JSON từ /api/news")?;

    let items = data.get("items").cloned().unwrap_or(Value::Array(vec![]));
    let item_count = items.as_array().map(|a| a.len()).unwrap_or(0);

    Ok(json!({
        "ok": true,
        "count": item_count,
        "items": items,
    }))
}
