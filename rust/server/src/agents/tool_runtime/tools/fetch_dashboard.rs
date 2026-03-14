use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::agents::tool_runtime::utils::truncate_chars;

pub(crate) const DESCRIPTION: &str =
    "Lấy danh sách các instrument card đang hiển thị trên dashboard (symbol, tên, giá, % thay đổi, xu hướng, độ tin cậy, phân tích, key levels). Dùng để xem lại thông tin agent đã cập nhật trước đó hoặc kiểm tra trạng thái hiện tại của các cặp tiền trên dashboard.";

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "symbol": {
                "type": "string",
                "description": "Lọc theo mã instrument cụ thể (VD: 'XAUUSD'). Nếu không truyền sẽ trả về tất cả."
            }
        },
        "additionalProperties": false,
    })
}

pub(crate) async fn execute(arguments: Value) -> Result<Value> {
    let backend_url = std::env::var("HYBRIDTRADE_BACKEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("không thể tạo HTTP client")?;

    let symbol = arguments
        .get("symbol")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let url = match symbol {
        Some(sym) => format!("{}/api/instruments/{}", backend_url, sym),
        None => format!("{}/api/instruments", backend_url),
    };

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("không thể gọi GET {url}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "fetch_dashboard trả về status {status}: {}",
            truncate_chars(&body, 500)
        );
    }

    let data: Value = response
        .json()
        .await
        .context("không thể parse JSON response từ fetch_dashboard")?;

    Ok(json!({
        "ok": true,
        "instruments": data,
    }))
}
