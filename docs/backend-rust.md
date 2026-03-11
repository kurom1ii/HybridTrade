# Backend Rust

## Mục tiêu

Backend đã được viết lại theo hướng tối giản: chỉ giữ phần mà frontend đang dùng thật, bỏ lớp tool registry cũ, memory pipeline cũ và phần fetch/phân tích phức tạp không còn cần thiết.

Mục tiêu của bản mới:

- ít module;
- API dễ đọc;
- SQLite schema nhỏ;
- SSE và scheduler đủ dùng;
- dễ bảo trì và dễ mở rộng tiếp.

## Kiến trúc hiện tại

Mã nguồn backend nằm trong `rust/server/src/` với 11 file:

- `main.rs`: boot app, load config, migrate DB, start worker nền, serve HTTP.
- `api.rs`: toàn bộ HTTP route.
- `config.rs`: đọc `app.toml` và `schedules.toml`.
- `db.rs`: CRUD, mapping row sang API view, schedule và heartbeat.
- `events.rs`: event bus cho SSE.
- `models.rs`: request/response types và DB row types.
- `providers.rs`: định tuyến provider AI, build system prompt và gọi OpenAI hoặc Anthropic.
- `runner.rs`: pipeline investigation và follow-up.
- `scheduler.rs`: heartbeat service + cron loop.
- `skills.rs`: nạp Markdown skill chung và theo agent.
- `tool_runtime.rs`: đăng ký native tools, MCP tools và thực thi tool calls trong debug chat.

Không còn các module cũ như `tool_registry`, `orchestrator`, `analysis`, `MCP`.

## Contract API

Các route đang được giữ ổn định để frontend hiện tại tiếp tục dùng:

- `GET /health`
- `GET /api/dashboard`
- `GET /api/investigations`
- `POST /api/investigations`
- `GET /api/investigations/:id`
- `GET /api/investigations/:id/stream`
- `POST /api/investigations/:id/follow-ups`
- `GET /api/agents/status`
- `GET /api/heartbeats`
- `GET /api/schedules`
- `POST /api/schedules`
- `PATCH /api/schedules/:id`

Ngoài ra backend có thêm nhóm route debug dành cho CLI agent riêng:

- `GET /api/debug/providers`
- `GET /api/debug/agents`
- `POST /api/debug/agents/:role/chat`

Contract JSON vẫn khớp với các type trong `frontend/lib/intelligence-types.ts`.

Nhóm route debug này là additive, không phá contract frontend hiện có.

## Luồng investigation mới

Khi frontend gọi `POST /api/investigations`, backend sẽ:

1. tạo investigation và sections trong SQLite với trạng thái `queued`;
2. trả ngay `InvestigationDetail` ban đầu;
3. spawn background task để chạy pipeline;
4. publish SSE để frontend detail page tự reload.

Pipeline hiện tại cố ý đơn giản và deterministic:

- `Coordinator`: ghi plan message.
- `Source Scout`: chuẩn hóa seed URLs thành sources.
- `Technical Analyst`: suy ra bias, confidence, key levels bằng heuristic nhẹ từ topic/goal/seed URLs.
- `Evidence Verifier`: đánh dấu narrative đồng hướng hay mixed.
- `Report Synthesizer`: dựng final report ngắn gọn cho detail page.

Điểm quan trọng: pipeline investigation mặc định vẫn đơn giản và không phụ thuộc tool ngoài. Tuy nhiên nhánh debug chat của agent có thể nạp native tools hoặc MCP theo cấu hình để phục vụ đọc file, chạy lệnh hoặc debug trình duyệt.

## Follow-up

`POST /api/investigations/:id/follow-ups` hoạt động theo kiểu lightweight:

- ghi user message;
- đọc sections và findings hiện có;
- sinh coordinator reply ngắn;
- publish SSE `agent.message` và `investigation.updated`.

Không có sub-run riêng cho follow-up.

## Database

Schema hiện tại chỉ giữ các bảng cần cho frontend:

- `investigations`
- `investigation_sections`
- `agent_runs`
- `agent_messages`
- `findings`
- `source_documents`
- `heartbeats`
- `schedules`

Migration rewrite nằm ở `rust/server/migrations/0002_rewrite_schema.sql`.

Lưu ý: migration này sẽ reset schema cũ của backend trước đó. Nếu DB cũ đã có dữ liệu, dữ liệu cũ sẽ bị thay bằng schema mới sau lần boot đầu tiên với bản rewrite này.

## Heartbeat và scheduler

Worker nền hiện có đúng 2 việc:

- heartbeat `service/server`;
- heartbeat `service/scheduler` + cron loop.

Các `job_type` đang hỗ trợ:

- `heartbeat_sweep`
- `memory_compaction`
- `investigation_refresh`

`memory_compaction` trong bản mới là cleanup nhẹ cho dữ liệu lịch sử, không còn liên quan tới memory pipeline cũ.

## Cấu hình

Backend vẫn đọc:

- `rust/config/app.toml`
- `rust/config/schedules.toml`
- `rust/config/tools.toml`
- `rust/config/mcp.toml`

`tools.toml` dùng để bật native tools cho agent debug, gồm cả các tool như `read`, `write`, `exec`, `bash` nếu được cấu hình.

`mcp.toml` dùng để khai báo MCP server và danh sách `skill_tools` hiển thị trong capability debug.

### Provider AI

Backend hiện có hai provider chat:

- `openai`
- `anthropic`

Phần bật/tắt, model và base URL được cấu hình trong `rust/config/app.toml`.

API key không được hardcode trong source. Backend đọc trực tiếp từ môi trường:

```bash
export OPENAI_API_KEY=your_openai_key
export ANTHROPIC_API_KEY=your_anthropic_key
```

Nếu thiếu key, các endpoint debug agent chat vẫn sống nhưng sẽ trả lỗi cấu hình khi thực hiện chat.

### Native tools và workspace

Native tools chạy trong workspace hiện tại của process backend. Có thể override root này bằng biến môi trường:

```bash
export HYBRIDTRADE_TOOL_ROOT=/abs/path/to/workspace
```

`read` và `write` chỉ cho phép truy cập path nằm trong workspace root đó. `exec` và `bash` cũng chạy với `cwd` nằm trong workspace này và bị chặn bởi `timeout_ms` trong `tools.toml`.

### CLI debug agent

Workspace Rust hiện có thêm crate `rust/agent-cli/`. Đây là CLI riêng để chat với backend agents phục vụ debug. CLI này không gọi provider trực tiếp mà đi qua backend.

Tài liệu dùng CLI nằm ở [Agent CLI](./agent-cli.md).

## Chạy và kiểm tra

Chạy server:

```bash
cd rust
cargo run -p hybridtrade-server
```

Chạy CLI debug agent:

```bash
cd rust
cargo run -p hybridtrade-agent-cli -- providers
cargo run -p hybridtrade-agent-cli -- agents
cargo run -p hybridtrade-agent-cli -- chat --agent technical_analyst
```

Kiểm tra compile/test:

```bash
cd rust
cargo fmt --all
cargo check -p hybridtrade-server
cargo test -p hybridtrade-server
cargo build -p hybridtrade-server
cargo build -p hybridtrade-agent-cli
```

## Phạm vi hiện tại

Backend mới phục vụ tốt các màn frontend đang nối thật:

- `/dashboard/investigations`
- `/dashboard/investigations/[id]`
- `/dashboard/agents`
- `/dashboard/analytics`

Các màn trading mock như `/dashboard`, `/markets`, `/positions`, `/orders`, `/news`, `/signals` vẫn chưa dùng backend nào.
