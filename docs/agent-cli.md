# Agent CLI

## Mục tiêu

`rust/agent-cli/` là một CLI Rust tách riêng hoàn toàn với backend server. CLI này chỉ nói chuyện với backend qua HTTP để phục vụ debug agent sâu hơn, không gọi OpenAI hay Anthropic trực tiếp.

Điểm quan trọng:

- provider, prompt hệ thống và ngữ cảnh investigation nằm ở backend;
- CLI chỉ là lớp điều khiển và hiển thị phản hồi;
- có thể dùng CLI này để debug từng agent mà không đụng frontend.

## Luồng hoạt động

CLI gọi đúng ba endpoint debug của backend:

- `GET /api/debug/providers`
- `GET /api/debug/agents`
- `POST /api/debug/agents/:role/chat`

Vì vậy muốn dùng CLI thì backend phải chạy trước.

## Cấu hình provider

Backend đã bật sẵn hai provider trong `rust/config/app.toml`:

- `openai`
- `anthropic`

Nhưng backend chỉ thực sự gọi được provider khi môi trường có API key tương ứng:

```bash
export OPENAI_API_KEY=your_openai_key
export ANTHROPIC_API_KEY=your_anthropic_key
```

Nếu thiếu key, CLI vẫn gọi được backend nhưng lệnh chat sẽ báo lỗi rõ ràng từ backend.

## Chạy CLI

Build:

```bash
cd rust
cargo build -p hybridtrade-agent-cli
```

Liệt kê provider:

```bash
cd rust
./target/debug/hybridtrade-agent-cli providers
```

Liệt kê agent:

```bash
cd rust
./target/debug/hybridtrade-agent-cli agents
```

Chat một lần:

```bash
cd rust
./target/debug/hybridtrade-agent-cli chat \
  --agent technical_analyst \
  --provider openai \
  --message "Phân tích giúp tôi bias hiện tại"
```

Chat tương tác:

```bash
cd rust
./target/debug/hybridtrade-agent-cli chat --agent coordinator
```

## Tuỳ chọn hữu ích

- `--agent`: tên agent backend, ví dụ `coordinator`, `technical_analyst`
- `--provider`: ép backend dùng `openai` hoặc `anthropic`
- `--investigation-id`: nhúng ngữ cảnh từ một investigation cụ thể
- `--show-debug`: in thêm system prompt và context preview mà backend đã dùng
- `--no-backend-context`: tắt việc backend tự nạp context investigation
- `--message`: gửi một tin nhắn một lần; nếu bỏ trống sẽ vào REPL

CLI cũng đọc biến môi trường `HYBRIDTRADE_BACKEND_URL`. Nếu không đặt, mặc định là `http://127.0.0.1:8080`.

## Lệnh trong REPL

Khi chạy chế độ tương tác, CLI hỗ trợ:

- `/exit` hoặc `/quit`: thoát
- `/clear`: xoá lịch sử chat cục bộ của CLI
- `/debug`: bật hoặc tắt phần in debug

Lịch sử này chỉ nằm ở CLI để gửi lại cho backend trong các lượt chat sau. Backend không lưu riêng phiên chat debug này.
