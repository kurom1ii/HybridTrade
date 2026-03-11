use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use futures_util::stream::{self, StreamExt};
use reqwest::{redirect::Policy, Client, StatusCode};

const SOURCE_URL: &str = "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks5.txt";
const HTTP_TARGET: &str = "http://fxstreet.com/";
const HTTPS_TARGET: &str = "https://fxstreet.com/";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";
const CONCURRENCY: usize = 64;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_REDIRECTS: usize = 10;

#[derive(Debug)]
struct ProxyScore {
    proxy: String,
    http_latency: Duration,
    https_latency: Duration,
}

impl ProxyScore {
    fn total_latency(&self) -> Duration {
        self.http_latency + self.https_latency
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let output_path = output_path()?;
    let fetch_client = Client::builder()
        .no_proxy()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(Duration::from_secs(20))
        .build()
        .context("không tạo được HTTP client để tải danh sách proxy")?;

    println!("Đang tải danh sách SOCKS5 từ {SOURCE_URL}");
    let proxy_list = fetch_proxy_list(&fetch_client).await?;

    if proxy_list.is_empty() {
        write_results(&output_path, &[]).await?;
        println!(
            "Không lấy được proxy hợp lệ nào. Đã tạo file rỗng tại {}",
            output_path.display()
        );
        return Ok(());
    }

    println!(
        "Bắt đầu test {} proxy với concurrency={} trên {} và {}",
        proxy_list.len(),
        CONCURRENCY,
        HTTP_TARGET,
        HTTPS_TARGET
    );

    let checked = Arc::new(AtomicUsize::new(0));
    let passed = Arc::new(AtomicUsize::new(0));
    let total = proxy_list.len();

    let mut scores: Vec<ProxyScore> = stream::iter(proxy_list.into_iter().map(|proxy| {
        let checked = Arc::clone(&checked);
        let passed = Arc::clone(&passed);

        async move {
            let result = test_proxy(proxy.clone()).await;
            let checked_now = checked.fetch_add(1, Ordering::Relaxed) + 1;

            if let Some(score) = result {
                let passed_now = passed.fetch_add(1, Ordering::Relaxed) + 1;
                println!(
                    "[OK {passed_now}] {} | http={}ms | https={}ms",
                    score.proxy,
                    score.http_latency.as_millis(),
                    score.https_latency.as_millis(),
                );
                if checked_now % 100 == 0 || checked_now == total {
                    println!("Tiến độ: {checked_now}/{total} | pass={passed_now}");
                }
                Some(score)
            } else {
                if checked_now % 100 == 0 || checked_now == total {
                    let passed_now = passed.load(Ordering::Relaxed);
                    println!("Tiến độ: {checked_now}/{total} | pass={passed_now}");
                }
                None
            }
        }
    }))
    .buffer_unordered(CONCURRENCY)
    .filter_map(async move |item| item)
    .collect()
    .await;

    scores.sort_by_key(ProxyScore::total_latency);
    write_results(&output_path, &scores).await?;

    println!(
        "Hoàn tất: {} proxy pass. Kết quả đã lưu tại {}",
        scores.len(),
        output_path.display()
    );

    if !scores.is_empty() {
        println!("Top proxy nhanh nhất:");
        for score in scores.iter().take(10) {
            println!(
                "- {} | tổng={}ms | http={}ms | https={}ms",
                score.proxy,
                score.total_latency().as_millis(),
                score.http_latency.as_millis(),
                score.https_latency.as_millis(),
            );
        }
    }

    Ok(())
}

async fn fetch_proxy_list(client: &Client) -> Result<Vec<String>> {
    let body = client
        .get(SOURCE_URL)
        .send()
        .await
        .context("không tải được danh sách proxy từ GitHub")?
        .error_for_status()
        .context("GitHub trả về lỗi khi tải danh sách proxy")?
        .text()
        .await
        .context("không đọc được nội dung danh sách proxy")?;

    let mut seen = HashSet::new();
    let mut proxies = Vec::new();

    for raw_line in body.lines() {
        let line = normalize_proxy_line(raw_line);
        if let Some(proxy) = line {
            if seen.insert(proxy.clone()) {
                proxies.push(proxy);
            }
        }
    }

    Ok(proxies)
}

fn normalize_proxy_line(raw_line: &str) -> Option<String> {
    let line = raw_line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let line = line
        .strip_prefix("socks5://")
        .or_else(|| line.strip_prefix("socks5h://"))
        .unwrap_or(line);

    let (host, port) = line.split_once(':')?;
    if host.trim().is_empty() {
        return None;
    }

    let port: u16 = port.trim().parse().ok()?;
    if port == 0 {
        return None;
    }

    Some(format!("{}:{}", host.trim(), port))
}

async fn test_proxy(proxy: String) -> Option<ProxyScore> {
    let (_, http_latency) = match request_via_proxy(&proxy, HTTP_TARGET, false).await {
        Ok(result) if is_http_pass(result.0) => result,
        _ => return None,
    };

    let (_, https_latency) = match request_via_proxy(&proxy, HTTPS_TARGET, true).await {
        Ok(result) if is_https_pass(result.0) => result,
        _ => return None,
    };

    Some(ProxyScore {
        proxy,
        http_latency,
        https_latency,
    })
}

async fn request_via_proxy(
    proxy: &str,
    url: &str,
    follow_redirects: bool,
) -> Result<(StatusCode, Duration)> {
    let policy = if follow_redirects {
        Policy::limited(MAX_REDIRECTS)
    } else {
        Policy::none()
    };

    let proxy_url = format!("socks5h://{proxy}");
    let client = Client::builder()
        .no_proxy()
        .proxy(reqwest::Proxy::all(&proxy_url).context("proxy URL không hợp lệ")?)
        .redirect(policy)
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("không tạo được HTTP client qua SOCKS5")?;

    let started_at = Instant::now();
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("request tới {url} qua proxy {proxy} thất bại"))?;
    let latency = started_at.elapsed();

    Ok((response.status(), latency))
}

fn is_http_pass(status: StatusCode) -> bool {
    status.is_success() || status.is_redirection()
}

fn is_https_pass(status: StatusCode) -> bool {
    status.is_success() || status.is_redirection()
}

async fn write_results(output_path: &PathBuf, scores: &[ProxyScore]) -> Result<()> {
    let parent = output_path
        .parent()
        .ok_or_else(|| anyhow!("đường dẫn output không có thư mục cha"))?;
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("không tạo được thư mục {}", parent.display()))?;

    let content = scores
        .iter()
        .map(|score| score.proxy.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    tokio::fs::write(output_path, content)
        .await
        .with_context(|| format!("không ghi được file {}", output_path.display()))?;

    Ok(())
}

fn output_path() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("không xác định được thư mục root của repo"))?;

    Ok(repo_root.join("proxy").join("sock5_work.txt"))
}
