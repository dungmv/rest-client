# Đặc tả Kỹ thuật Giai đoạn 0: Nền Tảng Cốt Lõi — Gửi HTTP Request & Xem Phản Hồi (Core HTTP Foundation)

## 1. Mục Tiêu của Giai Đoạn 0
Giai đoạn 0 là nền móng quan trọng nhất của toàn bộ ứng dụng: xây dựng hoàn chỉnh kiến trúc gửi và nhận HTTP requests thực tế với hiệu năng cao, không bị giật lag (120 FPS), không bị block UI khi xử lý payload lớn, và lưu trữ dữ liệu an toàn dưới dạng file thuần trên đĩa cứng.

---

## 2. Kiến Trúc Bộ Soạn Thảo Request (Request Builder Engine)

### 2.1 Phương Thức HTTP (HTTP Methods)
Hỗ trợ đầy đủ các phương thức chuẩn với mã màu nhận diện trực quan:
- `GET` (Tím `#a855f7`)
- `POST` (Xanh lá `#22c55e`)
- `PUT` (Xanh dương `#3b82f6`)
- `DELETE` (Đỏ `#ef4444`)
- `PATCH` (Vàng `#eab308`)
- `HEAD` (Cyan `#06b6d4`), `OPTIONS` (Tím nhạt `#8b5cf6`)

### 2.2 Quản Lý Query Parameters
- Bảng key-value động: Cho phép thêm, sửa, xóa từng cặp tham số.
- Checkbox bật/tắt: Tạm thời vô hiệu hóa một tham số mà không cần xóa đi.
- Tự động URL encoding chuẩn RFC 3986 (xử lý khoảng trắng, ký tự đặc biệt, unicode UTF-8).
- Tự động đồng bộ hai chiều (Bi-directional sync): Sửa trên thanh URL bar thì bảng Params tự cập nhật, và ngược lại.

### 2.3 Quản Lý Headers
- Bảng Header key-value kèm checkbox bật/tắt.
- Gợi ý tự động (Autocomplete) các headers thông dụng: `Content-Type`, `Authorization`, `Accept`, `User-Agent`, `Cache-Control`, `X-Custom-Header`.
- Tự động tính toán các header hệ thống (`Content-Length`, `Host`) khi gửi.

### 2.4 Các Định Dạng Request Body
1. **None:** Trạng thái mặc định cho GET/HEAD requests.
2. **JSON:**
   - Trình soạn thảo văn bản có số dòng (line numbers).
   - Highlight cú pháp JSON (keys, strings, numbers, booleans, null).
   - Nút **Beautify / Format JSON** tự động căn chỉnh thụt dòng chuẩn 2 spaces.
   - Nút **Copy Body** ra clipboard.
3. **Form URL-Encoded:** Bảng key-value tự động mã hóa header `application/x-www-form-urlencoded`.
4. **Multipart Form-Data:** Hỗ trợ gửi text fields kết hợp đính kèm file nhị phân (`multipart/form-data`).
5. **Raw (Text / XML / HTML / Plain):** Gửi chuỗi thô kèm Content-Type tùy biến.
6. **Binary File:** Chọn một file bất kỳ từ ổ đĩa để upload trực tiếp qua luồng nhị phân (Octet-stream).

### 2.5 Cấu Hình Request (Request Settings)
- **Timeout:** Thời gian chờ tối đa (mặc định: 30s, cho phép chỉnh từ 1s đến 300s).
- **Follow Redirects:** Bật/tắt tự động chuyển hướng HTTP (301, 302, 307, 308), giới hạn số lần redirect tối đa (mặc định: 10).
- **Validate TLS Certificate:** Tùy chọn bỏ qua kiểm tra chứng chỉ SSL/TLS khi làm việc với API local/self-signed (`https://localhost`).

---

## 3. Kiến Trúc Thực Thi Mạng (Tokio & Reqwest Decoupled Executor)

### 3.1 Mô Hình Đa Luồng Cô Lập (Threading Isolation)
- **GPUI Main Thread:** Chỉ xử lý render giao diện đồ họa GPU, bắt sự kiện chuột/phím. Tuyệt đối không thực hiện I/O mạng trên luồng này.
- **Tokio Multi-threaded Runtime:** Một runtime chạy nền riêng biệt. Khi bấm **Send**:
  1. Trích xuất snapshot request hiện tại.
  2. Spawn một Tokio task chạy độc lập.
  3. Cung cấp một `tokio_util::sync::CancellationToken` để khi người dùng bấm nút **Cancel**, kết nối HTTP in-flight lập tức bị ngắt kết nối và giải phóng tài nguyên ngay lập tức.
  4. Bắn các sự kiện tiến trình (`HttpSendEvent::Progress`, `Success`, `Error`, `Cancelled`) qua channel `tokio::sync::mpsc::unbounded_channel()` về GPUI để cập nhật giao diện.

### 3.2 Phân Rã Thời Gian Chi Tiết Độ Chính Xác Cao (High-Precision Timing Breakdown)
Sử dụng các hook socket để đo lường chính xác từng giai đoạn của kết nối mạng:
- **DNS Lookup Time:** Thời gian phân giải tên miền IP (ms).
- **TCP Connection Time:** Thời gian bắt tay 3 bước TCP SYN/ACK (ms).
- **TLS Handshake Time:** Thời gian thiết lập mã hóa SSL/TLS (ms).
- **TTFB (Time to First Byte):** Khoảng thời gian từ lúc gửi xong byte request cuối cùng đến khi nhận được byte response đầu tiên từ server (đo lường tốc độ xử lý backend).
- **Download Time:** Thời gian tải toàn bộ payload body về client.
- **Total Duration:** Tổng thời gian từ lúc bấm Send đến khi hoàn tất.

---

## 4. Kiến Trúc Trình Xem Phản Hồi (Response Viewer & Inspector)

### 4.1 Thanh Trạng Thái Phản Hồi (Status Metric Bar)
- **Status Code Badge:**
  - `2xx`: Xanh lá (200 OK, 201 Created)
  - `3xx`: Xanh dương (301 Moved, 304 Not Modified)
  - `4xx`: Cam (400 Bad Request, 401 Unauthorized, 404 Not Found)
  - `5xx`: Đỏ (500 Internal Server Error, 502 Bad Gateway)
- **Thời Gian (Duration):** Hiển thị tổng số mili-giây (`48 ms`). Nhấp vào icon đồng hồ `🕒` mở popup xem biểu đồ thanh Timing Breakdown.
- **Kích Thước (Size):** Hiển thị dung lượng tải về tự động chuyển đơn vị (`512 B`, `24.5 KB`, `3.2 MB`).

### 4.2 Các Tab Thanh Kiểm Tra
1. **Response Body Tab:**
   - Dropdown chế độ:
     - `Pretty`: Tự động format JSON thụt dòng, đánh số dòng, tô màu cú pháp.
     - `Raw`: Chuỗi văn bản thô nguyên bản từ server.
   - Xử lý payload lớn (Large Body Handling): Không render toàn bộ chuỗi khổng lồ một lúc; xử lý dòng ảo hóa (virtualized line rendering) đảm bảo phản hồi 10MB+ không gây đơ ứng dụng.
   - Nút **Copy Body** và nút **Save to File** để lưu response xuống ổ đĩa.
2. **Request Sent Tab:** Hiển thị chính xác request thực tế đã gửi qua đường truyền (URL đã interpolate biến, headers thực tế kèm Authorization token, payload body thực tế).
3. **Headers Tab:** Bảng danh sách toàn bộ Response Headers kèm badge đếm số lượng (ví dụ: `Headers 12`).
4. **Cookies Tab:** Danh sách các cookies mà server gửi về qua header `Set-Cookie` kèm cờ bảo mật.

---

## 5. Lưu Trữ File Cục Bộ Git-Native (Storage Foundation)

### 5.1 Lưu Trữ Collection Dạng File Phẳng (Git-Friendly File Storage)
- Lưu trữ trong thư mục workspace dưới dạng các file plain text (YAML).
- Mỗi thư mục con là một Folder API.
- Mỗi request là một file YAML riêng biệt với các keys được sắp xếp ổn định.
- Dữ liệu hoàn toàn độc lập, dễ dàng commit vào Git repo của dự án.

### 5.2 Lưu Trữ Bộ Nhớ Đệm Phản Hồi (.loc Cache)
- File cache cục bộ `<slug>.loc` nằm cạnh file request (được gitignore).
- Tự động ghi lại: `sent_at`, `request`, `response` (status, duration_ms, size, headers, body, truncated, binary).
- Lưu response gần nhất của mỗi request để xem lại ngay khi mở request.
