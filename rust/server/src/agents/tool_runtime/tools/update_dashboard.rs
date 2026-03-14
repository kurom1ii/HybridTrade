use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::agents::tool_runtime::utils::{optional_string_arg, required_string_arg, truncate_chars};

pub(crate) const DESCRIPTION: &str =
    "Cập nhật thông tin instrument trên dashboard. Cho phép cập nhật phân tích, giá, xu hướng, độ tin cậy và các mức giá quan trọng cho một cặp tiền hoặc asset.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "symbol": {
                "type": "string",
                "description": "Mã instrument cần cập nhật, ví dụ 'XAUUSD', 'EURUSD'"
            },
            "name": {
                "type": "string",
                "description": "Tên hiển thị, ví dụ 'Gold / US Dollar'"
            },
            "category": {
                "type": "string",
                "description": "Loại: 'forex', 'commodity', 'crypto', 'index'"
            },
            "direction": {
                "type": "string",
                "description": "Xu hướng: 'bullish', 'bearish', 'neutral'"
            },
            "confidence": {
                "type": "number",
                "minimum": 0,
                "maximum": 100,
                "description": "Độ tin cậy của phân tích (0-100)"
            },
            "price": {
                "type": "number",
                "description": "Giá hiện tại"
            },
            "change_pct": {
                "type": "number",
                "description": "% thay đổi giá"
            },
            "timeframe": {
                "type": "string",
                "description": "Khung thời gian phân tích, ví dụ 'H1', 'H4', 'D1'"
            },
            "analysis": {
                "type": "string",
                "description": "Tóm tắt phân tích kỹ thuật và cơ bản"
            },
            "key_levels": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "price": { "type": "number" },
                        "label": { "type": "string" },
                        "type": { "type": "string", "description": "support / resistance / pivot" }
                    }
                },
                "description": "Các mức giá quan trọng (hỗ trợ, kháng cự, pivot)"
            }
        },
        "required": ["symbol"],
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(arguments: Value) -> Result<Value> {
    let symbol = required_string_arg(&arguments, "symbol")?;

    let backend_url = std::env::var("HYBRIDTRADE_BACKEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    let url = format!("{}/api/instruments/{}", backend_url, symbol);

    let mut payload = serde_json::Map::new();
    if let Some(v) = optional_string_arg(&arguments, "name") {
        payload.insert("name".to_string(), Value::String(v));
    }
    if let Some(v) = optional_string_arg(&arguments, "category") {
        payload.insert("category".to_string(), Value::String(v));
    }
    if let Some(v) = optional_string_arg(&arguments, "direction") {
        payload.insert("direction".to_string(), Value::String(v));
    }
    if let Some(v) = arguments.get("confidence").and_then(|v| v.as_f64()) {
        payload.insert("confidence".to_string(), json!(v));
    }
    if let Some(v) = arguments.get("price").and_then(|v| v.as_f64()) {
        payload.insert("price".to_string(), json!(v));
    }
    if let Some(v) = arguments.get("change_pct").and_then(|v| v.as_f64()) {
        payload.insert("change_pct".to_string(), json!(v));
    }
    if let Some(v) = optional_string_arg(&arguments, "timeframe") {
        payload.insert("timeframe".to_string(), Value::String(v));
    }
    if let Some(v) = optional_string_arg(&arguments, "analysis") {
        payload.insert("analysis".to_string(), Value::String(v));
    }
    if let Some(v) = arguments.get("key_levels").cloned() {
        payload.insert("key_levels".to_string(), v);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("không thể tạo HTTP client")?;

    let response = client
        .put(&url)
        .json(&Value::Object(payload))
        .send()
        .await
        .with_context(|| format!("không thể gọi PUT {url}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "update_dashboard trả về status {status}: {}",
            truncate_chars(&body, 500)
        );
    }

    let instrument: Value = response
        .json()
        .await
        .context("không thể parse JSON response từ update_dashboard")?;

    Ok(json!({
        "ok": true,
        "instrument": instrument,
    }))
}
