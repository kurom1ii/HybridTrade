---
name: chrome-devtools
description: Sử dụng Chrome DevTools qua MCP để debug, xử lý sự cố và tự động hóa trình duyệt hiệu quả. Dùng khi cần debug trang web, tự động hóa thao tác trình duyệt, phân tích hiệu năng hoặc kiểm tra network request.
---

## Khái niệm cốt lõi

**Vòng đời trình duyệt**: Trình duyệt sẽ tự khởi động ở lần gọi tool đầu tiên bằng một Chrome profile persistent. Có thể cấu hình qua CLI args trong MCP server config: `npx chrome-devtools-mcp@latest --help`.

**Chọn page**: Các tool hoạt động trên page đang được chọn hiện tại. Dùng `list_pages` để xem danh sách page, sau đó `select_page` để chuyển context.

**Tương tác phần tử**: Dùng `take_snapshot` để lấy cấu trúc trang cùng các `uid` của phần tử. Mỗi phần tử có một `uid` duy nhất để tương tác. Nếu không tìm thấy phần tử, hãy chụp snapshot mới vì phần tử có thể đã bị xoá hoặc trang đã thay đổi.

## Workflow gợi ý

### Trước khi tương tác với một page

1. Điều hướng: `navigate_page` hoặc `new_page`
2. Chờ: `wait_for` để chắc nội dung đã tải xong nếu bạn biết mình đang chờ gì.
3. Chụp snapshot: `take_snapshot` để hiểu cấu trúc trang
4. Tương tác: dùng `uid` lấy từ snapshot cho `click`, `fill`, v.v.

### Lấy dữ liệu hiệu quả

- Dùng tham số `filePath` cho các output lớn như screenshot, snapshot, trace
- Dùng phân trang (`pageIdx`, `pageSize`) và filter (`types`) để giảm dữ liệu trả về
- Đặt `includeSnapshot: false` cho các action nhập liệu/truy cập nếu bạn không cần trạng thái trang mới nhất

### Chọn tool đúng mục đích

- **Tự động hóa / tương tác**: `take_snapshot` (dạng text, nhanh hơn, phù hợp cho automation)
- **Kiểm tra trực quan**: `take_screenshot` (khi cần nhìn trạng thái hiển thị thực tế)
- **Lấy chi tiết bổ sung**: `evaluate_script` cho dữ liệu không có trong accessibility tree

### Thực thi song song

Bạn có thể gửi nhiều tool call song song, nhưng vẫn phải giữ đúng thứ tự logic: navigate -> wait -> snapshot -> interact.

## Xử lý sự cố

Nếu có lỗi khi khởi động `chrome-devtools-mcp` hoặc Chrome, xem thêm tại https://github.com/ChromeDevTools/chrome-devtools-mcp/blob/main/docs/troubleshooting.md.

