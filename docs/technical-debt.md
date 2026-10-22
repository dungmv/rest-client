# Danh mục Nợ Kỹ Thuật Tập Trung (Technical Debt Registry)

> **Mục đích:** Tài liệu duy nhất và chính thức ghi nhận toàn diện các khoản nợ kỹ thuật (Technical Debt), các anti-pattern, điểm nghẽn kiến trúc và vấn đề hiệu năng/bộ nhớ tồn đọng trong codebase `restclient` (`restclient-core` và `restclient-app`), kèm mức độ ưu tiên và phương án khắc phục chi tiết. Mọi cập nhật về nợ kỹ thuật bắt buộc phải ghi nhận vào tài liệu này.

---

## 📊 Bảng tổng hợp trạng thái (Status Overview)

| Mức độ | Đã giải quyết | Còn lại | Chi tiết ưu tiên tiếp theo |
| :--- | :---: | :---: | :--- |
| 🔴 **CRITICAL** | 2 | 0 | Đã giải quyết toàn bộ (PERF-001, TD-01) |
| 🟠 **HIGH** | 12 | 2 | **PERF-005** (`suppress_path` thread spawn), **CC-002** (FsEvent cascade notification) |
| 🟡 **MEDIUM** | 5 | 8 | **PERF-007**, **TD-07/TD-005** (`unwrap` Mutex), **TD-08/TD-007**, **TD-006**, **PERF-008**, **PERF-009**, **PERF-010**, **CC-003** |
| 🔵 **LOW** | 0 | 7 | **TD-09**, **TD-014** (OpenAPI ref recursion), **TD-015**, **TD-017** (`body_editor` split), **TD-018**, **TD-019**, **SL-007** |
| **TỔNG** | **19** | **17** | **Tập trung xử lý Top 3: PERF-005, CC-002, TD-07/TD-005** |

---

## 📌 Bảng ma trận ưu tiên tổng thể (Priority Matrix)

| ID | Vấn đề / Khoản nợ | Khu vực ảnh hưởng | Mức độ | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :---: | :---: | :---: |
| **PERF-005** | `suppress_path()` spawn OS thread mới mỗi lần write file | `crates/core/src/storage/fs_store.rs` | **P1 (High)** | Thấp | 🔴 Chưa xử lý |
| **CC-002** | `FsEvent` reload cascade render notifications | `crates/app/src/state/persistence_state.rs` | **P1 (High)** | Trung bình | 🔴 Chưa xử lý |
| **TD-07 / TD-005** | Panics tiềm ẩn do `unwrap()` trên Mutex lock / Option | `crates/core/src/http/executor.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **PERF-007** | `refresh_formatted_body()` parse lại JSON mỗi lần đổi view mode | `crates/app/src/state/response_state.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **TD-08 / TD-007** | Doc comments & code comments tiếng Việt trong core | `crates/core/src/scripting/`, `executor.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **TD-006** | Hardcoded developer machine path trong test | `crates/core/src/storage/mod.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **PERF-008** | Clone chuỗi trong `RequestFile::to_item()` và `update_from_item()` | `crates/core/src/storage/models.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **PERF-009** | `interpolate_cow()` tạo `VariableInterpolator` mới mỗi iteration | `crates/core/src/interpolation.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **PERF-010** | Pre/post-request script chạy blocking trên UI thread | `crates/app/src/services/` | **P2 (Med)** | Trung bình | 🔴 Chưa xử lý |
| **CC-003** | Git operations (`pull`, `push`) đồng bộ trên UI path | `crates/app/src/state/git_ops.rs` | **P2 (Med)** | Trung bình | 🔴 Chưa xử lý |
| **TD-008** | `suppressed_paths` là `pub` — lộ implementation detail | `crates/core/src/storage/fs_store.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **TD-009** | `PostResponseOutput` không được box khi đi qua enum variant | `crates/app/src/services/runner.rs` | **P2 (Med)** | Thấp | 🔴 Chưa xử lý |
| **TD-011** | `#[allow(clippy::too_many_arguments)]` ở `modals.rs`, `auth_tab.rs` | `crates/app/src/app/` | **P2 (Med)** | Trung bình | 🔴 Chưa xử lý |
| **TD-09** | Struct Padding & Memory Alignment của `TimingBreakdown` | `crates/core/src/models.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| **TD-014** | `resolve_ref` trong OpenAPI importer đệ quy vô hạn (stack overflow) | `crates/core/src/importers/openapi.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| **TD-015** | `ModalState::default()` thực hiện I/O đọc file cấu hình | `crates/app/src/state/modal_state.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| **TD-017** | `body_editor.rs` giant component (>1.900 LOC) | `crates/app/src/components/body_editor.rs` | **P3 (Low)** | Trung bình | 🔴 Chưa xử lý |
| **TD-018** | `git_discard_file` gọi `std::fs::remove_file` trực tiếp | `crates/core/src/git/mod.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| **TD-019** | `run_git_cmd` dùng `std::process::Command` không timeout | `crates/core/src/git/mod.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| **SL-007** | `FolderNode` recursive Vec re-allocate toàn bộ tree | `crates/core/src/storage/models.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| **PERF-013** | `normalize_path_for_compare()` tạo String lowercase mỗi compare | `crates/core/src/storage/watcher.rs` | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |
| *SL-001* | `AppView` God Struct (32 fields) | `crates/app/src/app/` | **P0 (Critical)** | Cao | 🟢 Đã giải quyết |
| *SL-002* | `ResponseState` cache unbounded memory leak (TD-03) | `crates/app/src/state/response_state.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *SL-003* | `EditorState` own toàn bộ `HttpRequestItem` (18 fields) | `crates/app/src/state/` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *SL-004* | Lặp pattern inline-rename 5 lần (15 fields) | `crates/app/src/state/sidebar_state.rs` | **P2 (Med)** | Thấp | 🟢 Đã giải quyết |
| *SL-005* | `BTreeMap extra` trên 9 storage structs | `crates/core/src/storage/models.rs` | **P2 (Med)** | Thấp | 🟢 Đã giải quyết |
| *SL-006* | Boxing rồi dereference unbox `HttpResponseRecord` | `crates/core/src/models.rs` | **P2 (Med)** | Thấp | 🟢 Đã giải quyết |
| *PERF-001* | `reqwest::Client` khởi tạo mới per request | `crates/core/src/http/executor.rs` | **P0 (Critical)** | Thấp | 🟢 Đã giải quyết |
| *PERF-002* | Double TCP probe trước mỗi request | `crates/core/src/http/executor.rs` | **P1 (High)** | Thấp | 🟢 Đã giải quyết |
| *PERF-003* | Full filesystem scan sau mỗi CRUD operation (CC-001) | `crates/app/src/state/workspace_crud.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *PERF-004* | `effective_environment()` clone toàn bộ variables mỗi request | `crates/app/src/state/environment_state.rs` | **P1 (High)** | Thấp | 🟢 Đã giải quyết |
| *PERF-006* | `sync_*()` gọi mỗi frame render (60fps) | `crates/app/src/app/subscriptions.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *TD-01* | Clippy compiler error (`never_loop`) & warnings | `crates/core/src/importers/openapi.rs` | **P0 (Blocker)** | Thấp | 🟢 Đã giải quyết |
| *TD-02* | God Object State (`WorkspaceState` > 3.300 LOC) | `crates/app/src/state/` | **P0 (Critical)** | Cao | 🟢 Đã giải quyết |
| *TD-04* | Giant Component Files (`git_modal`, `sidebar_tree`, `app.rs`) | `crates/app/src/components/`, `app/` | **P1 (High)** | Cao | 🟢 Đã giải quyết |
| *TD-05* | Dual-State Sync & Race condition tự đổi màu modified | `crates/app/src/app/subscriptions.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *TD-06* | Lạm dụng `.clone()` trong HTTP Request Pipeline | `crates/app/src/services/` | **P2 (Med)** | Thấp | 🟢 Đã giải quyết |
| *TD-001* | Dead code `FsStorage` (JSON legacy) | `crates/core/src/storage/` | **P1 (High)** | Thấp | 🟢 Đã giải quyết |
| *TD-002* | Duplicate type hierarchy giữa domain và storage model | `crates/core/src/models.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *TD-003* | Git CLI operations blocking UI thread (CC-004) | `crates/app/src/state/git_ops.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết |
| *TD-004* | Global `#![allow(dead_code)]` ở `main.rs` | `crates/app/src/main.rs` | **P1 (High)** | Thấp | 🟢 Đã giải quyết |

---

## 🔍 Chi tiết các khoản nợ kỹ thuật còn tồn đọng (Unresolved Items)

### 1. PERF-005: `suppress_path()` spawn OS thread mới mỗi lần write file (🟠 High)
- **Vị trí:** [`crates/core/src/storage/fs_store.rs:37-42`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/fs_store.rs#L37-L42)
- **Vấn đề:** Mỗi lần ghi một file xuống đĩa, hàm gọi `std::thread::spawn` rồi `thread::sleep(1500ms)` để gỡ đường dẫn khỏi danh sách suppressed. Khi import hoặc lưu hàng loạt 50 files sẽ tạo 50 OS threads đồng thời.
- **Giải pháp:** Sử dụng async timer trong executor nền hoặc một hàng đợi ưu tiên (priority queue / timer wheel) chạy trên một single worker task duy nhất.

### 2. CC-002: `FsEvent` reload cascade render notifications (🟠 High)
- **Vị trí:** [`crates/app/src/state/persistence_state.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/persistence_state.rs)
- **Vấn đề:** Khi có nhiều thay đổi file bên ngoài, watcher bắn nhiều `FsEvent` riêng lẻ liên tiếp khiến `WorkspaceDataState` reload và gọi `cx.notify()`, gây cascade re-render liên tục.
- **Giải pháp:** Áp dụng cơ chế batching events (debounce 100-200ms gom nhóm các FsEvent) và chỉ phát ra duy nhất một thông báo cập nhật cây thư mục sau khi gom xong.

### 3. TD-07 / TD-005: Tiềm ẩn Panic do `unwrap()` trên Mutex / Option (🟡 Medium)
- **Vị trí:** [`crates/core/src/http/executor.rs:370-385`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs#L370-L385)
- **Vấn đề:** `self.cookie_store.lock().unwrap()` có thể hoảng loạn (panic) nếu Mutex bị poisoned do một thread trước đó gặp sự cố.
- **Giải pháp:** Thay bằng `.lock().unwrap_or_else(|poisoned| poisoned.into_inner())` để thu hồi guard an toàn.

### 4. PERF-007: `refresh_formatted_body()` parse lại JSON mỗi lần đổi view mode (🟡 Medium)
- **Vị trí:** [`crates/app/src/state/response_state.rs:219-253`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/response_state.rs#L219-L253)
- **Vấn đề:** `serde_json::from_str()` và `to_string_pretty()` chạy lại toàn bộ khi người dùng toggle view settings. Payload lớn (500KB - 5MB) gây giật lag 10-50ms trên UI.
- **Giải pháp:** Lưu cache chuỗi `formatted_body` trong `ResponseState`, chỉ định dạng lại khi raw body bytes thực sự thay đổi.

### 5. TD-08 / TD-007: Doc Comments & Code Comments tiếng Việt trong Core (🟡 Medium)
- **Vị trí:** [`crates/core/src/scripting/sandbox.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/scripting/sandbox.rs), [`crates/core/src/http/executor.rs:324`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs#L324)
- **Vấn đề:** Tồn tại các comment tiếng Việt trong mã nguồn lõi (`// Chạy Pre-request Script...`), làm giảm tính nhất quán quốc tế của codebase mã nguồn mở.
- **Giải pháp:** Chuẩn hoá 100% comments và docstrings trong file code sang tiếng Anh kỹ thuật; giữ tài liệu tiếng Việt trong thư mục `docs/`.

### 6. TD-006: Hardcoded Developer Path trong Test (🟡 Medium)
- **Vị trí:** [`crates/core/src/storage/mod.rs:459-485`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/mod.rs#L459-L485)
- **Vấn đề:** Đường dẫn tuyệt đối Windows `C:\Users\maiva\...` bị fix cứng khiến unit test âm thầm bị bỏ qua (skip) trên các máy khác hoặc CI.
- **Giải pháp:** Chuyển sang sử dụng `tempfile::tempdir()` hoặc fixture nội bộ từ `env!("CARGO_MANIFEST_DIR")`.

### 7. PERF-008 & PERF-009: Cấp phát chuỗi dư thừa trong `models` và `interpolation` (🟡 Medium)
- **Vị trí:** [`crates/core/src/storage/models.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/models.rs), [`crates/core/src/interpolation.rs:78-83`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/interpolation.rs#L78-L83)
- **Vấn đề:** `interpolate_cow()` tạo mới `VariableInterpolator` (cùng HashMap) ở mỗi header loop; `RequestFile::to_item()` clone từng trường chuỗi.
- **Giải pháp:** Tái sử dụng một `VariableInterpolator` duy nhất xuyên suốt quá trình resolve một request; tối ưu hoá move semantics cho `to_item()`.

### 8. TD-014: `resolve_ref` trong OpenAPI Importer đệ quy vô hạn (🔵 Low)
- **Vị trí:** [`crates/core/src/importers/openapi.rs:291-300`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/importers/openapi.rs#L291-L300)
- **Vấn đề:** File OpenAPI có circular `$ref` (Schema A trỏ đến Schema B trỏ lại A) sẽ khiến hàm đệ quy vô hạn gây tràn stack (stack overflow).
- **Giải pháp:** Bổ sung giới hạn độ sâu (ví dụ `depth > 32`) và `HashSet<String>` ghi nhận các ref đã duyệt qua.

### 9. TD-017: Giant Component `body_editor.rs` (> 1.900 LOC) (🔵 Low)
- **Vị trí:** [`crates/app/src/components/body_editor.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/components/body_editor.rs)
- **Vấn đề:** Xử lý toàn bộ các định dạng Raw, JSON, XML, Form-Urlencoded, Multipart trong một file duy nhất dài gần 2.000 dòng.
- **Giải pháp:** Tách thành package module `components/body_editor/` gồm `json.rs`, `xml.rs`, `form.rs`, `multipart.rs`, `raw.rs`.

---

## 🏆 Tóm tắt kết quả các cải tiến kiến trúc lớn đã hoàn thành

### SL-001 & TD-02: Phân rã God Struct `AppView` và `WorkspaceState`
- Xoá bỏ struct khổng lồ 3.300 dòng và gom 9 entities độc lập: `WorkspaceDataState`, `EnvironmentState`, `PersistenceState`, `SidebarState`, `ModalState`, `GitState`, `RequestSessionState`, `ResponseState`, `ToastState`.
- Tách `ComposerInputs` (14 handles) và `InspectorDisplays` (6 handles) ra khỏi `AppView`.
- Tách `HttpEngine` (độc lập, không phụ thuộc GPUI) và `Runner` chạy nền.

### SL-002: Bounded LRU Cache `ResponseCache` (SL-002 / TD-03)
- Triển khai `LruMemoryCache` kiểm soát kép: tối đa 100 entries hoặc 50MB RAM.
- Xóa bỏ dead state duplicate (`previous_responses`). Tích hợp nạp on-demand từ disk cache `.loc`.

### SL-003: Single Source of Truth cho Request (SL-003)
- `RequestSessionState` chỉ lưu `active_request_id: Option<String>`.
- Không sao chép toàn bộ request vào session state; mượn on-demand từ `WorkspaceDataState`.

### SL-005 & TD-002: Chuẩn hóa Schema và Serialization
- Áp dụng `#[serde(deny_unknown_fields)]` trên toàn bộ 9 storage models YAML.
- Hợp nhất domain model và storage model, loại bỏ chuyển đổi trung gian `From`/`Into`.

### PERF-001, PERF-002, PERF-006: Tối ưu mạng và render 120 FPS
- Khởi tạo và chia sẻ connection pool trong `ReqwestExecutor`, pre-warm connection khi mở ứng dụng.
- Bỏ hoàn toàn TCP probe rỗng.
- Triệt tiêu 100% các lệnh sync trong `AppView::render()`, chuyển hoàn toàn sang event subscriptions qua `cx.observe_in()`.

---

## 🛠️ Quy trình cập nhật nợ kỹ thuật (Maintenance Protocol)

Mọi lập trình viên và AI Agent khi làm việc trên dự án phải tuân thủ:
1. **Một nguồn chân lý duy nhất (Single Source of Truth):** Toàn bộ nợ kỹ thuật phát hiện mới hoặc cập nhật trạng thái đều phải ghi trực tiếp vào file này ([`docs/technical-debt.md`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/docs/technical-debt.md)). Tuyệt đối không tạo thêm các file đánh số phiên bản phụ (`technical-debt-02.md`, v.v.).
2. **Kèm Unit Test:** Khi xử lý xong bất kỳ mục nợ kỹ thuật nào, bắt buộc phải có ít nhất 1 unit test kiểm chứng chống hồi quy.
3. **Cập nhật ma trận:** Chuyển trạng thái từ `🔴 Chưa xử lý` sang `🟢 Đã giải quyết` và bổ sung tóm tắt giải pháp vào tài liệu này.
