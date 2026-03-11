# HybridTrade Intelligence Dashboard

HybridTrade hiện dùng một backend Rust đã được viết lại theo hướng gọn: phục vụ investigation workflow, heartbeat, schedules và SSE cho phần frontend intelligence đang chạy thật.

## Tổng quan

- `rust/`: backend Rust dạng monolith, dùng `Axum + Tokio + SQLx + SQLite + SSE`.
- `rust/agent-cli/`: CLI Rust tách riêng để chat với backend agents qua HTTP debug API.
- `frontend/`: frontend Next.js dạng hybrid, gồm một phần intelligence UI đã nối backend và một phần trading UI còn là mock tĩnh.
- `docs/`: bộ tài liệu tiếng Việt cho backend, frontend và độ lệch giữa hai phía.

## Tính năng v1

- Tạo `investigation` mới với topic, goal và danh sách URL công khai.
- Chạy pipeline agent gọn gồm `Coordinator`, `Source Scout`, `Technical Analyst`, `Evidence Verifier`, `Report Synthesizer`.
- Sinh `sources`, `findings`, `transcript`, `final_report` và đẩy cập nhật lên frontend theo thời gian thực.
- Lưu `investigations`, `sections`, `messages`, `findings`, `sources`, `heartbeats`, `schedules` trong SQLite.
- Stream sự kiện lên frontend qua `SSE`.
- Hỗ trợ `cron jobs`, `heartbeat sweep`, `history compaction` và `follow-up question`.

## Lưu ý phạm vi

Hệ thống này không dùng để đặt lệnh trade. Trong v1:

- Không có broker integration.
- Không có order execution.
- Không có risk engine để giao dịch thật.
- Chỉ làm việc với nguồn `public web`.

## Chạy nhanh

Backend:

```bash
cd rust
export OPENAI_API_KEY=your_openai_key
export ANTHROPIC_API_KEY=your_anthropic_key
cargo run -p hybridtrade-server
```

CLI debug agent:

```bash
cd rust
cargo run -p hybridtrade-agent-cli -- providers
cargo run -p hybridtrade-agent-cli -- agents
cargo run -p hybridtrade-agent-cli -- chat --agent technical_analyst
```

Frontend:

```bash
cd frontend
NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:8080 npm run dev
```

Sau khi backend và frontend cùng chạy, nên mở trực tiếp màn đã nối backend thật:

```text
http://127.0.0.1:3000/dashboard/investigations
```

`/dashboard` hiện vẫn là màn trading mock tĩnh.

## Kiểm tra

Backend:

```bash
cd rust
cargo test -p hybridtrade-server
cargo check -p hybridtrade-server
cargo build -p hybridtrade-server
cargo build -p hybridtrade-agent-cli
```

Frontend:

```bash
cd frontend
npm run build
```

## Tài liệu

- [Backend Rust](./docs/backend-rust.md)
- [Agent CLI](./docs/agent-cli.md)
- [Frontend Next.js](./docs/frontend-nextjs.md)
- [Đồng bộ backend và frontend](./docs/dong-bo-backend-frontend.md)
