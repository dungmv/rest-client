# Đặc tả Kỹ thuật: Đồng bộ Git & Cộng tác Nhóm không phụ thuộc Cloud (Git Sync & Collaboration)

## 1. Triết lý: Tại sao Local-First & Git-Native là Tương lai?
- **Khủng hoảng Cloud Lock-in của Postman/Insomnia:**
  - Postman bắt buộc lưu trữ dữ liệu API lên server của Postman. Nhiều công ty tài chính, y tế, an ninh mạng **nghiêm cấm** nhân viên sử dụng Postman vì lo ngại rò rỉ thông tin nội bộ (API Keys, Production Endpoints, PII Data).
  - Insomnia v8 yêu cầu login khiến nhiều tổ chức phải đóng băng phiên bản v7 cũ.
- **Giải pháp của RestClient:**
  - Sử dụng chính hạ tầng Git sẵn có của tổ chức (GitHub, GitLab, Bitbucket, hoặc Server nội bộ).
  - Mọi Workspace là một Git repository cục bộ.
  - Mỗi request là một file plain-text JSON có key sắp xếp ổn định (Stable Key Order), giúp `git diff` đọc được từng dòng, review pull request như code thông thường.

---

## 2. Cấu trúc Thư mục Git trong Workspace

```
my-api-project/
├── .git/
├── .restclient/
│   ├── workspace.json          # Metadata của workspace
│   ├── environments/
│   │   ├── local.json          # Biến môi trường local (commit vào git)
│   │   ├── staging.json        # Biến môi trường staging (commit vào git)
│   │   └── local.secret.json   # (Thêm vào .gitignore) Lưu token nhạy cảm
│   └── requests/
│       ├── auth/
│       │   ├── _folder.json    # Metadata của folder "auth"
│       │   ├── login.json      # File request POST /login
│       │   └── refresh.json    # File request POST /refresh
│       └── users/
│           ├── _folder.json
│           └── get_profile.json
└── README.md
```

### 2.1 Cơ chế Masking & Biến Bí Mật (`.secret.json`)
- Các biến thông thường (URL, non-sensitive headers) được lưu vào `staging.json` và commit lên Git để chia sẻ cho đồng đội.
- Các biến bí mật (Password, Secret Key, Private Token) được gắn cờ `is_secret: true`:
  - Trong UI: Luôn hiển thị dưới dạng dấu chấm `••••••••`.
  - Trên đĩa: Lưu riêng vào file `*.secret.json`. RestClient tự động thêm `*.secret.json` vào `.gitignore` khi khởi tạo workspace, ngăn chặn 100% nguy cơ vô tình commit lộ API key lên GitHub!

---

## 3. Tính năng Git Tích Hợp trong Giao diện UI

Thay vì bắt người dùng phải mở cửa sổ Terminal rời để gõ lệnh `git`, RestClient tích hợp một Git Drawer chuyên dụng:

1. **Thanh trạng thái Branch & Dirty Indicator:**
   - Góc trái titlebar và footer hiển thị branch hiện tại (ví dụ: `⎇ feature/auth-v2`).
   - Có chấm cam nhỏ nếu có thay đổi chưa commit (`3 modified, 1 untracked`).
2. **Visual Request Diff Viewer:**
   - Khi nhấp vào một request có thay đổi, hiển thị giao diện so sánh trước/sau (Side-by-side Diff):
     - Headers nào bị thêm/xóa.
     - Body JSON thay đổi ở trường nào (được tô màu xanh lá cho trường thêm mới, đỏ cho trường bị xóa).
3. **Thao tác 1-Click Git Operations:**
   - **Branch Picker:** Tạo branch mới, chuyển branch nhanh.
   - **Commit Dialog:** Nhập commit message và bấm **Commit Changes**.
   - **Sync:** Nút **Pull** (cập nhật từ remote) và **Push** (đẩy lên remote).
   - **Conflict Resolver:** Giao diện trực quan khi xảy ra merge conflict, cho phép chọn `Use Local`, `Use Remote`, hoặc `Merge Both`.
