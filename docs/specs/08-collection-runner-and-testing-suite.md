# Đặc tả Kỹ thuật: Bộ Công Cụ Kiểm Thử & Tự Động Hóa Nâng Cao (Testing & Automation Suite Specification)

## 1. Bối cảnh & Giá trị Cốt Lõi
Trong quy trình phát triển phần mềm chuyên nghiệp, việc gửi từng request thủ công chỉ là bước khởi đầu. Giá trị lớn nhất mà các đội ngũ kỹ thuật phụ thuộc vào Postman nằm ở:
1. **Chạy kiểm thử hồi quy (Regression Testing):** Bấm 1 nút chạy toàn bộ 100 requests theo thứ tự logic nghiệp vụ để xác nhận hệ thống không bị lỗi sau khi deploy.
2. **Kiểm thử theo dữ liệu (Data-Driven Testing):** Chạy 1 kịch bản kiểm thử với 50 tài khoản khác nhau trong file CSV để kiểm tra quyền hạn (Role-based access).
3. **Tích hợp CI/CD (Continuous Integration):** Chạy bộ test API tự động mỗi khi có Pull Request được tạo trên GitHub/GitLab (thông qua Newman CLI).
4. **Mock Server:** Giả lập API cho Frontend / Mobile developer khi Backend chưa kịp triển khai.

Tài liệu này đặc tả chi tiết kiến trúc và triển khai cho **Giai đoạn 6 (Phase 6)**.

---

## 2. Kiến trúc Module (`zest-core::runner` & `crates/cli`)

```
zest/
├── crates/
│   ├── core/
│   │   ├── src/
│   │   │   ├── runner/
│   │   │   │   ├── mod.rs           # Runner trait & orchestrator
│   │   │   │   ├── data_source.rs   # CSV / JSON data file parser
│   │   │   │   ├── reporter.rs      # CLI, JSON, HTML report generators
│   │   │   │   └── benchmark.rs     # Latency histograms & throughput metrics
│   │   │   └── mock/
│   │   │       ├── mod.rs           # Embedded Axum/Hyper mock server
│   │   │       └── router.rs        # Route matching dựa trên URL, method, headers
│   ├── app/                         # GPUI Desktop App (chứa Runner UI & Mock Server UI)
│   └── cli/                         # Headless binary `zest-cli` cho CI/CD
```

---

## 3. Collection Runner UI (Trình Chạy Bộ Kiểm Thử Trên Desktop)

### 3.1 Cấu Hình Lần Chạy (Run Configuration)
Người dùng nhấp chuột phải vào một Folder hoặc Collection và chọn **"Run Collection"**:
- **Execution Order:** Danh sách các request sẽ chạy (có thể kéo thả để đổi thứ tự, bật/tắt checkbox để bỏ qua request).
- **Iterations:** Số lần lặp lại kịch bản (mặc định: `1`).
- **Delay:** Thời gian nghỉ giữa các request (mặc định: `0 ms`, tùy chỉnh `100ms - 5000ms` để tránh bị rate limit).
- **Data File:** Chọn file `data.csv` hoặc `data.json`.
- **Stop Run If An Error Occurs:** Tùy chọn dừng chạy ngay khi có test thất bại.
- **Keep Variable State:** Tùy chọn lưu lại các biến môi trường thay đổi sau khi runner kết thúc.

### 3.2 Giao Diện Giám Sát Tiến Độ (Live Progress Monitor)
- **Header:** Hiển thị Progress bar (`Running request 14/50...`), tổng thời gian chạy (`1.8s`), số bài kiểm tra đã đỗ (`38 passed`) / trượt (`1 failed`).
- **Bảng Kết Quả Chi Tiết:**
  - Cột 1: Thứ tự & Trạng thái (`✓ 200 OK` xanh lá hoặc `✗ 500 Internal Error` đỏ).
  - Cột 2: Tên Request (ví dụ: `Auth > POST /login`).
  - Cột 3: Thời gian phản hồi (`42 ms`).
  - Cột 4: Kết quả Tests (`Pass 3/3`, nhấp vào để bung rộng xem chi tiết từng assertion).

---

## 4. Data-Driven Testing (Kiểm Thử Theo Dữ Liệu)

### 4.1 Cấu trúc File Dữ liệu (`users.csv`):
```csv
username,password,expected_role,expected_status
admin@test.com,Secret123!,ADMIN,200
user1@test.com,UserPass123,MEMBER,200
banned@test.com,Pass1234!,BANNED,403
invalid_user,wrong_pass,NONE,401
```

### 4.2 Ánh Xạ Biến Tự Động:
1. **Trong URL & Body:** Các biến `{{username}}`, `{{password}}` tự động nhận giá trị của từng dòng CSV trong iteration tương ứng.
2. **Trong Test Script:**
   ```javascript
   const expectedStatus = parseInt(client.iterationData.get("expected_status"));
   const expectedRole = client.iterationData.get("expected_role");

   client.test(`Verify status is ${expectedStatus}`, () => {
       client.assert(client.response.status === expectedStatus);
   });

   if (client.response.status === 200) {
       const json = client.response.json();
       client.test(`Verify role is ${expectedRole}`, () => {
           client.assert(json.user.role === expectedRole);
       });
   }
   ```

---

## 5. Headless CLI Runner (`crates/cli` — Newman Alternative)

### 5.1 Khởi Tạo Sub-crate `crates/cli`
Xây dựng file nhị phân độc lập `zest-cli` không phụ thuộc vào thư viện GUI GPUI, có kích thước cực nhỏ (< 15MB) và chạy được trên môi trường Linux Alpine/Ubuntu server không có màn hình.

### 5.2 Cú Pháp Lệnh CLI:
```bash
# Chạy bộ test cơ bản
zest-cli run ./my-collection --env staging

# Chạy với dữ liệu CSV và xuất báo cáo HTML
zest-cli run ./my-collection \
  --folder "E2E Checkout Flow" \
  --env production \
  --data ./test-cases.csv \
  --delay 50 \
  --reporters cli,html \
  --output-dir ./test-results/

# Tích hợp GitHub Actions Workflow:
# - name: Run API Integration Tests
#   run: |
#     ./zest-cli run ./.zest/workspaces/api --env ci-test
```

### 5.3 Mã Trả Về (Exit Codes):
- `0`: Tất cả assertions và requests hoàn thành thành công.
- `1`: Có ít nhất 1 bài kiểm tra test assertion thất bại.
- `2`: Lỗi kết nối mạng hoặc lỗi cấu hình không thể gửi request.

---

## 6. Trình Giả Lập API Cục Bộ (Local Mock Server)

### 6.1 Cơ Chế Hoạt Động
- Sử dụng crate **`axum`** nhúng bên trong ứng dụng.
- Lắng nghe tại `http://127.0.0.1:port` (người dùng chọn cổng tùy ý hoặc random port).
- Phân tích cây request trong collection:
  - Nếu request có lưu **Response Examples** (ví dụ: `200 Success`, `400 Bad Request`), Mock Server tự động trả về example tương ứng.
  - Hỗ trợ Mock theo Header (ví dụ: gửi header `X-Mock-Status: 404` sẽ trả về mock response 404).
  - Tự động thêm CORS headers (`Access-Control-Allow-Origin: *`) để ứng dụng React/Vue/Flutter chạy trên trình duyệt gọi trực tiếp không bị chặn.

---

## 7. Stress & Benchmark Testing (Đo Lường Hiệu Năng)

- **Cấu hình Benchmark:**
  - Concurrency: `1..50` luồng gửi đồng thời (Tokio async workers).
  - Duration: `10s..60s` hoặc Total Requests: `100..10,000`.
- **Thống kê phân phối độ trễ (Latency Distribution):**
  - Minimum, Maximum, Average (Mean).
  - Percentiles: `P50` (Median), `P90`, `P95`, `P99`.
  - Thông lượng: Số requests xử lý thành công mỗi giây (Requests/sec - RPS).
  - Tỷ lệ lỗi mạng & HTTP 5xx.
