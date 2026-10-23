# Đặc tả Kỹ thuật: Bộ Công Cụ Developer — Sinh Mã Nguồn, So Sánh Response & Biến Ảo (Developer Tools Specification)

## 1. Trình Sinh Mã Nguồn Đa Ngôn Ngữ (Code Snippet Generator)

### 1.1 Mục Tiêu
Giúp kỹ sư sau khi test thành công một API trên Zest có thể sao chép ngay đoạn mã tương ứng để dán trực tiếp vào dự án backend hoặc frontend của họ mà không phải viết lại từ đầu.

### 1.2 Danh Sách Ngôn Ngữ & Framework Hỗ Trợ (16 Targets)

| Ngôn ngữ / Công cụ | Thư viện / Cú pháp | Ví dụ hiển thị |
|---|---|---|
| **cURL** | Shell CLI | `curl -X POST "https://api.com/v1" -H "..." -d '...'` |
| **Rust** | `reqwest` (async) | `let res = reqwest::Client::new().post(...).send().await?;` |
| **Rust** | `ureq` (blocking) | `let res = ureq::post("...").set("...", "...").send_json(...)?;` |
| **JavaScript** | `fetch` (ES6 native) | `const res = await fetch("...", { method: "POST", ... });` |
| **JavaScript / TS** | `axios` | `const res = await axios.post("...", data, { headers });` |
| **Node.js** | `http` (native) | `const req = http.request(options, (res) => { ... });` |
| **Python** | `requests` | `response = requests.post("...", json=data, headers=headers)` |
| **Python** | `httpx` (async) | `async with httpx.AsyncClient() as client: r = await client.post(...)` |
| **Go** | `net/http` | `req, err := http.NewRequestWithContext(ctx, "POST", url, body)` |
| **Java** | `java.net.http.HttpClient` | `HttpRequest.newBuilder().uri(...).POST(...).build();` |
| **Java / Kotlin** | `OkHttp` | `val request = Request.Builder().url(...).post(body).build()` |
| **C# / .NET** | `HttpClient` | `var response = await client.PostAsync("...", content);` |
| **PHP** | `cURL` | `$ch = curl_init(); curl_setopt_array($ch, [...]);` |
| **PHP** | `Guzzle` | `$res = $client->request('POST', '...', ['json' => $data]);` |
| **Swift** | `URLSession` | `let task = URLSession.shared.dataTask(with: request) { ... }` |
| **Dart / Flutter** | `http` | `final response = await http.post(url, headers: h, body: b);` |

---

## 2. Dynamic Fake Variables (Macro Biến Động `$random*`)

### 2.1 Cú pháp Sử Dụng
Tương thích hoàn toàn với Postman, hỗ trợ chèn trực tiếp trong URL, Headers, hoặc JSON Body:

- **ID & Thời gian:**
  - `{{$guid}}` hoặc `{{$uuid}}`: Sinh ngẫu nhiên UUID v4 (ví dụ: `8f7b3a41-2b99-4d1e-8e33-728b9c6142e0`).
  - `{{$timestamp}}`: Unix timestamp (ví dụ: `1791827400`).
  - `{{$isoTimestamp}}`: Thời gian chuẩn ISO 8601 (ví dụ: `2026-10-13T08:30:00Z`).
- **Dữ liệu giả lập (Faker):**
  - `{{$randomInt}}`: Số nguyên ngẫu nhiên từ 1 đến 1000.
  - `{{$randomEmail}}`: Email ngẫu nhiên (ví dụ: `user_8492@example.org`).
  - `{{$randomFirstName}}`, `{{$randomLastName}}`, `{{$randomFullName}}`.
  - `{{$randomPhoneNumber}}`.
  - `{{$randomIpv4}}`, `{{$randomCity}}`, `{{$randomCountry}}`.

---

## 3. Lọc & Tìm Kiếm Response Nâng Cao (JSONPath Filter Bar)

### 3.1 Vấn đề
Khi server trả về một mảng JSON khổng lồ (vài nghìn phần tử hoặc vài megabytes), việc cuộn và tìm đúng trường dữ liệu rất tốn thời gian.

### 3.2 Giải pháp
Phía trên cửa sổ Response Body, bổ sung một thanh nhập liệu **JSONPath Filter Bar**:
- Nhập cú pháp: `$.items[*].id` hoặc `$.users[?(@.age > 20)].email`.
- Ứng dụng lọc dữ liệu ngay lập tức trong bộ nhớ và chỉ hiển thị tập kết quả khớp.
- Có nút **"Copy Filtered JSON"** để sao chép kết quả đã lọc ra clipboard.

---

## 4. So Sánh Response (Response Diff Viewer)

- Người dùng có thể chọn bất kỳ 2 lần gửi trong **Response History** để mở màn hình so sánh Diff.
- Phân tích sự khác biệt ở:
  1. Status Code (ví dụ: `200 OK` vs `404 Not Found`).
  2. Headers (ví dụ: thay đổi `ETag`, `Set-Cookie`).
  3. Body JSON (hiển thị giao diện 2 cột với highlight màu xanh lá cho trường được thêm và màu đỏ cho trường bị xóa).

---

## 5. Trình Tạo Tài Liệu API Tự Động (API Documentation Generator)

- Nút **"Generate Documentation"** ở cấp độ Collection hoặc Workspace.
- Chế độ xem trang đơn (Single-page view) bao gồm:
  - Tên và mô tả API.
  - Mục lục danh sách endpoint theo từng thư mục.
  - Chi tiết từng endpoint: Method, URL, bảng Parameters, Headers, ví dụ Request Body và ví dụ Response JSON mẫu tương ứng.
  - Cho phép xuất ra tệp `API_DOCS.md` hoặc xuất tệp HTML hoàn chỉnh có thể mở trực tiếp trong trình duyệt web.
