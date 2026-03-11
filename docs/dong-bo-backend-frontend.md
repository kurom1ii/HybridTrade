# Đồng bộ backend Rust với frontend hiện tại

## 1. Mục tiêu của tài liệu này

Tài liệu này trả lời đúng bài toán: nếu chỉ muốn phát triển hoặc chỉnh `backend Rust` để phù hợp với frontend hiện có, thì cần nhìn phần nào đã khớp, phần nào chưa khớp, và phần nào về mặt kỹ thuật không thể giải quyết hoàn toàn nếu không đụng frontend.

## 2. Kết luận ngắn

Có hai nhóm màn hình rất khác nhau:

- nhóm intelligence đã có contract thật với Rust backend mới;
- nhóm trading mock chưa gọi API nào, nên backend-only chưa thể làm chúng tự động dùng dữ liệu thật.

Nói thẳng: chỉ sửa backend không thể làm các page trading mock trở nên động nếu frontend không hề fetch dữ liệu. Backend có thể chuẩn bị contract và dữ liệu, nhưng tới lúc frontend gọi thì các màn đó mới hiển thị được dữ liệu thật.

## 3. Những route đã khớp sẵn với Rust backend

### 3.1 Đã dùng được ngay

- `/dashboard/investigations`
  Khớp với:
  - `GET /api/investigations`
  - `POST /api/investigations`

- `/dashboard/investigations/[id]`
  Khớp với:
  - `GET /api/investigations/:id`
  - `POST /api/investigations/:id/follow-ups`
  - `GET /api/investigations/:id/stream`

- `/dashboard/agents`
  Khớp với:
  - `GET /api/dashboard`

- `/dashboard/analytics`
  Khớp với:
  - `GET /api/schedules`

### 3.2 Việc nên giữ nguyên ở backend

Các endpoint trên đã đủ rõ và frontend đang dùng thật. Bản rewrite backend mới cũng đã được giữ xoay quanh đúng nhóm route này, nên nếu tiếp tục phát triển Rust backend thì nên coi đây là contract nền và tránh phá vỡ shape dữ liệu hiện tại.

## 4. Những route frontend chưa có backend tương ứng

### 4.1 `/dashboard`

Trang này hiện đang hiển thị:

- balance;
- open P/L;
- win rate;
- AI signals;
- watchlist;
- recent trades.

Rust backend hiện không có domain nào cho các khối đó. `GET /api/dashboard` hiện là dashboard intelligence, không phải trading overview.

Nếu muốn backend Rust khớp trang này, cần thêm tối thiểu một endpoint mới, ví dụ:

- `GET /api/trading/overview`

Response nên chứa tối thiểu:

```json
{
  "portfolio": {
    "balance": 24850.4,
    "open_pnl": 506.3,
    "win_rate": 0.72,
    "live_signal_count": 4
  },
  "watchlist": [],
  "signals": [],
  "recent_trades": []
}
```

### 4.2 `/dashboard/markets`

Trang này cần danh sách instrument, category filter, top movers và bảng market.

Backend Rust cần tối thiểu:

- `GET /api/markets/instruments`
- `GET /api/markets/top-movers`

Data shape tối thiểu:

```json
{
  "pair": "EUR/USD",
  "name": "Euro",
  "price": "1.0847",
  "change": "+0.24%",
  "change_type": "profit",
  "spread": "0.8",
  "volume": "2.4B",
  "category": "FOREX"
}
```

### 4.3 `/dashboard/positions`

Trang này cần open positions và panel chi tiết vị thế.

Backend Rust cần ít nhất:

- `GET /api/positions`
- `GET /api/positions/:id`

Cơ sở dữ liệu cần thêm bảng kiểu:

- `positions`
- có thể thêm `position_events` nếu muốn audit thay đổi

### 4.4 `/dashboard/orders`

Trang này cần pending orders, historical orders và quick order form.

Backend Rust cần ít nhất:

- `GET /api/orders`
- `POST /api/orders`
- `PATCH /api/orders/:id`
- `DELETE /api/orders/:id` hoặc endpoint cancel riêng

Database cần thêm bảng:

- `orders`

### 4.5 `/dashboard/news`

Trang này cần:

- bài viết news feed;
- economic calendar;
- trending topics;
- bookmarked items.

Backend Rust cần ít nhất:

- `GET /api/news/feed`
- `GET /api/news/calendar`
- `GET /api/news/topics`

Database hoặc adapter ngoài cần thêm model cho:

- `news_articles`
- `economic_events`
- `news_topics`

### 4.6 `/dashboard/signals`

Trang này cần tín hiệu trading và thống kê performance.

Backend Rust cần ít nhất:

- `GET /api/signals`
- `GET /api/signals/performance`

Database cần thêm bảng:

- `signals`
- có thể thêm `signal_results` nếu muốn theo dõi accuracy lịch sử

## 5. Những việc backend-only làm được và không làm được

### 5.1 Backend-only làm được

- định nghĩa domain model mới trong Rust;
- thêm bảng SQLite mới;
- thêm endpoint mới có shape khớp với các card/bảng ở frontend;
- thêm scheduler để đồng bộ market/news feed;
- thêm seed data để local demo có dữ liệu ngay.

### 5.2 Backend-only không làm được

- không thể tự biến page static thành page động nếu page đó không gọi API;
- không thể thay đổi điều hướng sidebar/topbar để người dùng đi vào đúng màn intelligence;
- không thể làm `/dashboard` dùng `GET /api/trading/overview` nếu file page hiện chưa fetch endpoint đó.

Đây là giới hạn kỹ thuật thực tế, không phải vấn đề lựa chọn triển khai.

## 6. Đề xuất nếu vẫn giữ nguyên nguyên tắc “ưu tiên sửa Rust backend”

### 6.1 Cách tiếp cận ít rủi ro nhất

1. Giữ nguyên toàn bộ intelligence API đang dùng thật.
2. Bổ sung thêm một namespace API mới cho các màn trading mock.
3. Tạo seed data trong SQLite để local chạy lên là có dữ liệu demo.
4. Chỉ khi cần thiết mới nối frontend vào các endpoint mới, nhưng không cần đổi layout hay đổi UX hiện có.

### 6.2 Namespace API nên thêm trong Rust

Một cách đặt tên hợp lý:

- `/api/trading/overview`
- `/api/markets/*`
- `/api/positions/*`
- `/api/orders/*`
- `/api/news/*`
- `/api/signals/*`

Lợi ích của hướng này:

- không phá contract intelligence hiện có;
- tách rõ hai miền nghiệp vụ: `intelligence` và `trading presentation`;
- dễ seed dữ liệu demo hoặc thay dần bằng adapter thật sau này.

## 7. Thứ tự ưu tiên nếu sau này triển khai thật trong Rust

Nếu chỉ được đầu tư vào backend Rust trước, thứ tự hợp lý là:

1. Giữ ổn định investigation API và SSE hiện có.
2. Bổ sung `trading overview` để thay thế dashboard mock mặc định.
3. Bổ sung `markets` vì đây là màn dễ seed dữ liệu nhất.
4. Bổ sung `signals` vì có thể tái sử dụng một phần logic intelligence.
5. Sau đó mới làm `positions`, `orders`, `news` tùy phạm vi sản phẩm thực tế.

## 8. Kết luận

Hiện trạng không phải là "backend Rust sai hoàn toàn" hay "frontend sai hoàn toàn". Vấn đề là hai nửa của sản phẩm đang thuộc hai giai đoạn khác nhau:

- backend Rust và một phần frontend đã đi theo hướng intelligence dashboard;
- nhiều route dashboard khác vẫn là trading mock tĩnh.

Nếu giữ nguyên nguyên tắc chỉ ưu tiên Rust backend, hướng đúng là:

- không phá phần intelligence đang chạy;
- thêm domain/API mới trong Rust cho các màn mock;
- chấp nhận rằng bản thân frontend sẽ cần ít nhất lớp fetch tối thiểu ở giai đoạn nối dữ liệu, dù không cần đổi giao diện.
