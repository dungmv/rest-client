# Đặc tả Kỹ thuật: Nâng Cấp Giao Diện & Trải Nghiệm Người Dùng (UI / UX Enhancements)

## 1. Hệ Thống Quản Lý Tab Đa Tác Vụ (Multi-Tab Workspace Bar)

### 1.1 Cơ Chế Hoạt Động Của Tab Bar:
Để đáp ứng thói quen làm việc đa nhiệm của lập trình viên, RestClient cần một thanh tab chuyên nghiệp:
1. **Single-Click (Preview Mode):** Khi nhấp 1 lần vào request ở sidebar, tab mở ở chế độ xem trước (tiêu đề in nghiêng). Nếu nhấp sang request khác, tab này sẽ được thay thế nhằm tránh tràn ngập tab rác.
2. **Double-Click (Pinned Mode):** Khi nhấp đúp vào request hoặc bắt đầu chỉnh sửa dữ liệu, tab chuyển sang trạng thái ghim cố định (chữ đứng, có nút đóng `✕`).
3. **Dirty State Indicator:** Khi có thay đổi chưa lưu (sửa URL, params, headers, body), hiển thị chấm tròn nhỏ `●` trên tab.
4. **Context Menu trên Tab:**
   - Đóng tab (`Close Tab`)
   - Đóng các tab khác (`Close Other Tabs`)
   - Đóng các tab bên phải (`Close Tabs to the Right`)
   - Nhân bản tab (`Duplicate Tab`)
   - Sao chép cURL của tab hiện tại (`Copy as cURL`).

---

## 2. Các Trình Kết Xuất Xem Trước (Preview Renderers)

Trong tab Response, bên cạnh chế độ xem `Pretty` (JSON thụt lề có màu) và `Raw`, bổ sung dropdown chuyển đổi:

### 2.1 Preview Hình Ảnh (Image Preview)
- Tự động nhận diện `Content-Type: image/png`, `image/jpeg`, `image/svg+xml`, `image/webp`, `image/gif`.
- Hiển thị ảnh trực quan, kích thước pixel (`width x height`), dung lượng file byte.
- Hỗ trợ phóng to, thu nhỏ (Zoom & Pan), và nút **"Save Image to File"**.

### 2.2 Sandboxed HTML Preview
- Tự động nhận diện `Content-Type: text/html`.
- Render mã HTML an toàn (tắt JavaScript độc hại hoặc chạy trong sandbox an toàn) để xem nhanh giao diện trang web hoặc thông báo lỗi của máy chủ web (Apache/Nginx/IIS 500 error page).

### 2.3 Hex Viewer
- Dành cho các response dạng nhị phân (Binary payloads, protobuf, file nén zip, file âm thanh).
- Hiển thị theo cột chuẩn: Offset hex bên trái, Byte hex ở giữa, ký tự ASCII bên phải.

---

## 3. Quản Lý Cookie Nâng Cao (Advanced Cookie Manager)

### 3.1 Nâng Cấp Modal Cookies:
- Phân nhóm cookie theo Domain (ví dụ: `.google.com`, `localhost`, `api.github.com`).
- Hiển thị đầy đủ các thuộc tính của cookie RFC 6265:
  - Name, Value
  - Domain, Path
  - Expires / Max-Age
  - Cờ bảo mật: `HttpOnly`, `Secure`, `SameSite (Strict, Lax, None)`.
- Cung cấp tính năng thêm cookie mới bằng tay, sửa giá trị, xóa từng cookie hoặc **"Clear All Cookies for Domain"**.

---

## 4. Tùy Biến Giao Diện & Themes (Customization & Polish)

1. **Bảng màu Tùy Chọn:**
   - **Dark Navy (Mặc định):** Deep navy `#1b1b2a` phong cách Yaak/Zed.
   - **OLED Pure Black:** `#000000` tiết kiệm pin trên màn hình OLED.
   - **Catppuccin Mocha / Macchiato:** Bộ màu được cộng đồng lập trình viên yêu thích nhất.
   - **Light Mode:** Dành cho người dùng thích giao diện sáng khi làm việc ban ngày.
2. **Cỡ Chữ & Font Family:**
   - Cho phép người dùng tùy chọn Font chữ Monospace (JetBrains Mono, Fira Code, Cascadia Code, SF Mono) và kích thước font size từ 11px đến 18px trong Settings.
