use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::agents::tool_runtime::utils::{optional_string_arg, truncate_chars};

pub(crate) const DESCRIPTION: &str =
    "Lấy lịch kinh tế (economic calendar) với các sự kiện theo ngày và mức độ quan trọng. Dùng để theo dõi sự kiện ảnh hưởng thị trường.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "date": {
                "type": "string",
                "description": "Ngày cần xem lịch kinh tế, format YYYY-MM-DD. Mặc định là hôm nay."
            },
            "importance": {
                "type": "string",
                "description": "Lọc theo mức độ quan trọng: 'high', 'medium', 'low'. Bỏ trống để lấy tất cả."
            }
        },
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(arguments: Value) -> Result<Value> {
    let date = optional_string_arg(&arguments, "date").unwrap_or_default();
    let importance = optional_string_arg(&arguments, "importance").unwrap_or_default();

    let frontend_url = std::env::var("HYBRIDTRADE_FRONTEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let mut url = format!("{}/api/calendar", frontend_url);
    let mut params = Vec::new();
    if !date.is_empty() {
        params.push(format!("date={date}"));
    }
    if !importance.is_empty() {
        params.push(format!("importance={importance}"));
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

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
            "fetch_calendar trả về status {status}: {}",
            truncate_chars(&body, 500)
        );
    }

    let data: Value = response
        .json()
        .await
        .context("không thể parse JSON từ /api/calendar")?;

    let events = data.get("events").cloned().unwrap_or(Value::Array(vec![]));
    let event_count = events.as_array().map(|a| a.len()).unwrap_or(0);

    Ok(json!({
        "ok": true,
        "count": event_count,
        "events": events,
    }))
}
