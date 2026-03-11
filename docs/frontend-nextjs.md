# Tài liệu frontend Next.js

## 1. Phạm vi

Frontend nằm trong thư mục `frontend/`, dùng `Next.js App Router`, `React 19`, `TypeScript`, `Tailwind v4` và `motion`. Ở thời điểm hiện tại đây là một frontend lai giữa hai hướng:

- phần `landing` và nhiều màn `dashboard/*` vẫn mang hình thái trading platform mock, dùng dữ liệu tĩnh;
- phần `investigations`, `agents` và `analytics` đã kết nối thật với backend Rust intelligence.

Điều này giải thích vì sao trong cùng một project có cả những route bám API Rust và những route mới chỉ là khung UI.

## 2. Stack và entrypoint

- Framework: `Next.js 16`
- UI: React 19
- CSS: Tailwind CSS v4
- Motion: `motion`
- Theme: `next-themes`

Các entrypoint quan trọng:

- `frontend/app/layout.tsx`: layout gốc và metadata chung.
- `frontend/app/page.tsx`: landing page công khai.
- `frontend/app/dashboard/layout.tsx`: shell dashboard gồm sidebar và top bar.

## 3. Cấu trúc thư mục

- `frontend/app/`: route theo App Router.
- `frontend/components/landing/`: component cho trang chủ.
- `frontend/components/dashboard/`: component dùng trong shell và các màn dashboard.
- `frontend/hooks/`: polling và SSE hook.
- `frontend/lib/intelligence-types.ts`: contract TypeScript khớp với API Rust hiện có.
- `frontend/lib/intelligence-api.ts`: HTTP client cho backend.
- `frontend/lib/forex-intelligence.ts`: helper để dựng lớp intelligence theo forex pair, hiện chưa được route nào dùng thật.
- `frontend/lib/formatting.ts`: formatter ngày giờ, role title, confidence.

## 4. Route map hiện tại

### 4.1 Landing và shell chung

- `/`
  Landing page tĩnh, không gọi backend Rust.

- `/dashboard`
  Dashboard trading mock tĩnh, không gọi API.

### 4.2 Route đã nối backend Rust

- `/dashboard/investigations`
  - fetch danh sách bằng `GET /api/investigations`
  - tạo investigation bằng `POST /api/investigations`

- `/dashboard/investigations/[id]`
  - fetch chi tiết bằng `GET /api/investigations/:id`
  - nhận realtime update qua SSE `GET /api/investigations/:id/stream`

- `/dashboard/agents`
  - dùng `GET /api/dashboard`
  - hiển thị `agent_statuses`, `recent_findings`, `recent_investigations`

- `/dashboard/analytics`
  - dùng `GET /api/schedules`
  - hiển thị cron jobs hiện tại

### 4.3 Route còn là dữ liệu tĩnh

Các route sau chưa gọi backend nào:

- `/dashboard/markets`
- `/dashboard/positions`
- `/dashboard/orders`
- `/dashboard/news`
- `/dashboard/signals`

Những màn này đang render từ mảng dữ liệu hardcoded trong file page tương ứng.

## 5. Lớp dữ liệu hiện có

### 5.1 Contract type

`frontend/lib/intelligence-types.ts` định nghĩa các model chính:

- `InvestigationSummary`
- `InvestigationDetail`
- `SectionView`
- `MessageView`
- `FindingView`
- `SourceView`
- `HeartbeatView`
- `ScheduleView`
- `AgentStatusView`
- `DashboardResponse`
- payload type cho `createInvestigation`

Các type này đang khớp khá tốt với struct Rust trả về từ backend.

### 5.2 API client

`frontend/lib/intelligence-api.ts` hiện có các hàm:

- `fetchDashboard()`
- `fetchInvestigations()`
- `fetchInvestigation(id)`
- `createInvestigation(payload)`
- `fetchAgentStatuses()`
- `fetchHeartbeats()`
- `fetchSchedules()`
- `investigationStreamUrl(id)`

Base URL lấy từ `NEXT_PUBLIC_API_BASE_URL`, mặc định là `http://127.0.0.1:8080`.

Backend Rust mới phía sau các route này đã được giản lược mạnh: investigation chỉ còn là snapshot metadata/sections, còn nhánh debug agent là nơi dùng tool/MCP khi cần.

### 5.3 Polling và SSE

`frontend/hooks/use-polling-resource.ts`:

- polling mặc định 15 giây;
- hỗ trợ tắt mở bằng `enabled`;
- dùng `startTransition` để cập nhật state mượt hơn.

`frontend/hooks/use-investigation-stream.ts`:

- mở `EventSource` tới investigation stream;
- lắng nghe các event:
  - `investigation.updated`
  - `heartbeat`
  - `job.status`
- khi nhận event, page detail hiện tại chỉ `reload()` lại snapshot toàn bộ investigation.

## 6. Luồng UI đã hoạt động với backend

### 6.1 Investigation composer

Trang `/dashboard/investigations` cho phép:

1. nhập `topic`, `goal`, `tags`, `seed URLs`;
2. submit investigation mới;
3. được chuyển sang trang detail ngay sau khi backend tạo investigation thành công;
4. xem queue của các investigation đã tạo.

Payload gửi lên backend đã dùng đúng các trường snake_case mà Rust đang nhận:

- `source_scope`
- `priority`
- `seed_urls`

### 6.2 Investigation detail

Trang `/dashboard/investigations/[id]` hiển thị:

- metadata của investigation;
- section conclusions;
- stored summary;
- seed URLs;
- heartbeats;

Sau khi stream nhận được event, trang reload lại toàn bộ `InvestigationDetail`. Cách này đơn giản và đúng chức năng, nhưng chưa tối ưu vì chưa merge event cục bộ.

### 6.3 Agent console

Trang `/dashboard/agents` thực chất là command center của hệ intelligence, không phải màn quản lý trading agent theo nghĩa broker/trade execution.

Trang này đang dùng đúng những gì backend Rust có sẵn:

- `agent_statuses`
- `recent_findings`
- `recent_investigations`

### 6.4 Analytics / schedules

Trang `/dashboard/analytics` hiện đang là màn theo dõi schedule chứ chưa phải analytics trading. Route label hiện tại hơi lệch tên nhưng data contract với backend thì đúng.

## 7. Thành phần đang chuẩn bị nhưng chưa được dùng hết

`frontend/lib/forex-intelligence.ts` và `frontend/lib/forex-pairs.ts` đã chuẩn bị sẵn các helper như:

- map investigation/findings sang từng forex pair;
- tính bias, coverage, confidence;
- dựng default brief cho pair cụ thể;
- dựng subagent task list.

Tuy nhiên ở thời điểm hiện tại chưa có route dashboard nào dùng trực tiếp lớp này để render một forex command center hoàn chỉnh.

Nói ngắn gọn: frontend đã có nền cho intelligence dashboard, nhưng route mặc định `/dashboard` vẫn còn là trading mock cũ.

## 8. Các điểm lệch nội bộ trong frontend

Đây là những điểm quan trọng khi đọc code hoặc lên kế hoạch nối backend:

- `Sidebar` vẫn trỏ vào các màn trading mock như `markets`, `positions`, `analytics`; không có link trực tiếp tới `investigations` hoặc `agents`.
- `TopBar` hiển thị nhãn `TRADING LIVE`, trong khi backend thực tế là intelligence backend.
- `app/dashboard/page.tsx` không dùng `fetchDashboard()` mà render số liệu trading tĩnh.
- metadata toàn app vẫn nói về `AI-Powered Trading Platform`, chưa phản ánh đúng hệ điều phối intelligence.
- nhiều page như `markets`, `orders`, `signals`, `news`, `positions` chưa có lớp fetch data.

Các điểm trên là lý do chính khiến cảm giác sản phẩm hiện tại chưa đồng nhất, dù contract giữa Rust backend và một phần frontend đã tương đối rõ.

## 9. Cách chạy local

Chạy frontend:

```bash
cd frontend
NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:8080 npm run dev
```

Mở:

```text
http://127.0.0.1:3000
http://127.0.0.1:3000/dashboard/investigations
```

Nếu muốn kiểm tra các màn đã nối backend, nên đi trực tiếp vào:

- `/dashboard/investigations`
- `/dashboard/investigations/<id>`
- `/dashboard/agents`
- `/dashboard/analytics`

## 10. Kết luận thực trạng frontend

Frontend hiện không phải là một khối đồng nhất. Nó gồm:

- một lớp landing/trading UI cũ hoặc mock;
- một lớp intelligence UI mới đã bắt đầu bám backend Rust thật.

Vì vậy, nếu định hướng là "chỉ chỉnh Rust backend để khớp frontend", cần đọc thêm tài liệu [đồng bộ backend/frontend](./dong-bo-backend-frontend.md) để thấy phần nào backend đã đáp ứng được và phần nào frontend hiện vẫn chưa gọi API nên backend-only chưa thể làm màn hình đó sống thật.
