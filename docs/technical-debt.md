# Danh mục Nợ Kỹ Thuật Tập Trung (Technical Debt Registry)

> **Mục đích:** Tài liệu duy nhất và chính thức ghi nhận toàn diện các khoản nợ kỹ thuật (Technical Debt), các anti-pattern, điểm nghẽn kiến trúc và vấn đề hiệu năng/bộ nhớ tồn đọng trong codebase `restclient` (`restclient-core` và `restclient-app`), kèm mức độ ưu tiên và phương án khắc phục chi tiết. Mọi cập nhật về nợ kỹ thuật bắt buộc phải ghi nhận vào tài liệu này.

---

## 📊 Bảng tổng hợp trạng thái (Status Overview)

| Mức độ | Đã giải quyết | Còn lại | Chi tiết ưu tiên tiếp theo |
| :--- | :---: | :---: | :--- |
| 🔴 **CRITICAL** | 2 | 0 | Đã giải quyết toàn bộ (PERF-001, TD-01) |
| 🟠 **HIGH** | 14 | 0 | Đã giải quyết toàn bộ (**PERF-005**, **CC-002**, TD-001, TD-002, TD-003, TD-004, PERF-002, PERF-003, PERF-004, PERF-006, SL-002, SL-003, TD-04, TD-05) |
| 🟡 **MEDIUM** | 8 | 0 | Đã giải quyết toàn bộ (**TD-08/TD-007**, **TD-006**, **PERF-009**, **PERF-010**, **CC-003**, **PERF-007**, **TD-07/TD-005**, **PERF-008**, **TD-008**, **TD-009**, **TD-011**) |
| 🔵 **LOW** | 7 | 0 | Đã giải quyết toàn bộ (**TD-09**, **TD-014**, **TD-015**, **TD-017**, **TD-018**, **TD-019**, **SL-007**, **PERF-013**) |
| **TỔNG** | **31** | **0** | **ĐÃ HOÀN THÀNH 100% TOÀN BỘ KHOẢN NỢ KỸ THUẬT TỒN ĐỌNG** |

---

## 📌 Bảng ma trận ưu tiên tổng thể (Priority Matrix)

| ID | Vấn đề / Khoản nợ | Khu vực ảnh hưởng | Mức độ | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :---: | :---: | :---: |
| **PERF-005** | `suppress_path()` spawn OS thread mới mỗi lần write file | `crates/core/src/storage/fs_store.rs` | **P1 (High)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **CC-002** | `FsEvent` reload cascade render notifications | `crates/app/src/state/persistence_state.rs` | **P1 (High)** | Trung bình | ✅ RESOLVED (2026-10-22) |
| **TD-07 / TD-005** | Panics tiềm ẩn do `unwrap()` trên Mutex lock / Option | `crates/core/src/http/executor.rs`, `cache/mod.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **PERF-007** | `refresh_formatted_body()` parse lại JSON mỗi lần đổi view mode | `crates/app/src/state/response_state.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-08 / TD-007** | Doc comments & code comments tiếng Việt trong core | `crates/core/src/scripting/`, `executor.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-006** | Hardcoded developer machine path trong test | `crates/core/src/storage/mod.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **PERF-008** | Clone chuỗi trong `RequestFile::to_item()` và `update_from_item()` | `crates/core/src/storage/models.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **PERF-009** | `interpolate_cow()` tạo `VariableInterpolator` mới mỗi iteration | `crates/core/src/interpolation.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **PERF-010** | Pre/post-request script chạy blocking trên UI thread | `crates/app/src/services/` | **P2 (Med)** | Trung bình | ✅ RESOLVED (2026-10-22) |
| **CC-003** | Git operations (`pull`, `push`) đồng bộ trên UI path | `crates/app/src/state/git_ops.rs` | **P2 (Med)** | Trung bình | ✅ RESOLVED (2026-10-22) |
| **TD-008** | `suppressed_paths` là `pub` — lộ implementation detail | `crates/core/src/storage/fs_store.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-009** | `PostResponseOutput` không được box khi đi qua enum variant | `crates/app/src/services/runner.rs` | **P2 (Med)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-011** | `#[allow(clippy::too_many_arguments)]` ở `modals.rs`, `auth_tab.rs` | `crates/app/src/app/` | **P2 (Med)** | Trung bình | ✅ RESOLVED (2026-10-22) |
| **TD-09** | Struct Padding & Memory Alignment của `TimingBreakdown` | `crates/core/src/models.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-014** | `resolve_ref` trong OpenAPI importer đệ quy vô hạn (stack overflow) | `crates/core/src/importers/openapi.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-015** | `ModalState::default()` thực hiện I/O đọc file cấu hình | `crates/app/src/state/modal_state.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-017** | `body_editor.rs` giant component (>1.900 LOC) | `crates/app/src/components/body_editor.rs` | **P3 (Low)** | Trung bình | ✅ RESOLVED (2026-10-22) |
| **TD-018** | `git_discard_file` gọi `std::fs::remove_file` trực tiếp | `crates/core/src/git/mod.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **TD-019** | `run_git_cmd` dùng `std::process::Command` không timeout | `crates/core/src/git/mod.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **SL-007** | `FolderNode` recursive Vec re-allocate toàn bộ tree | `crates/core/src/storage/models.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
| **PERF-013** | `normalize_path_for_compare()` tạo String lowercase mỗi compare | `crates/core/src/storage/watcher.rs` | **P3 (Low)** | Thấp | ✅ RESOLVED (2026-10-22) |
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

## 🔍 Chi tiết các khoản nợ kỹ thuật và giải pháp đã thực hiện

### 1. PERF-005: `suppress_path()` spawn OS thread mới mỗi lần write file (🟠 High) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/storage/fs_store.rs:37-42`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/fs_store.rs#L37-L42)
- **Giải pháp:** Sử dụng một background worker thread duy nhất (`workspace-suppress-cleaner`) với channel và queue timeout để giải phóng đường dẫn theo deadline, loại bỏ hoàn toàn việc tạo OS thread mới trên mỗi lần ghi file.

### 2. CC-002: `FsEvent` reload cascade render notifications (🟠 High) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/state/persistence_state.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/persistence_state.rs)
- **Giải pháp:** Triển khai `handle_fs_events_batch()` với debounce gom nhóm (100ms) qua channel `try_recv()`: gộp `TreeChanged` reload toàn bộ một lần duy nhất, gộp cập nhật nhiều requests trong một lần mutate state và chỉ gọi `cx.notify()` một lần ở cuối batch.

### 3. TD-07 / TD-005: Tiềm ẩn Panic do `unwrap()` trên Mutex / Option (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/http/executor.rs:370-385`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs#L370-L385), [`crates/core/src/cache/mod.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/cache/mod.rs), [`crates/app/src/components/response_inspector.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/components/response_inspector.rs)
- **Giải pháp:** Sử dụng `.lock().unwrap_or_else(|poisoned| poisoned.into_inner())` để thu hồi guard an toàn ngay cả khi mutex bị poisoned; thay thế các unwrap Option thành `if let Some` / `when_some` an toàn tuyệt đối.

### 4. PERF-007: `refresh_formatted_body()` parse lại JSON mỗi lần đổi view mode (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/state/response_state.rs:219-253`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/response_state.rs#L219-L253)
- **Giải pháp:** Lưu cache cấu trúc `CachedResponseFormat` (raw text + lines, pretty text + lines) keyed theo con trỏ `Arc<HttpResponseRecord>` bên trong `FormattedResponseBody`. Khi người dùng chuyển qua lại giữa Pretty và Raw, hệ thống hoán đổi Arc trong thời gian $O(1)$ mà không cần parse hay serialize lại JSON.

### 5. TD-08 / TD-007: Doc Comments & Code Comments tiếng Việt trong Core (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/scripting/`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/scripting/), [`crates/core/src/http/executor.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs)
- **Giải pháp:** Dịch và chuẩn hoá toàn bộ 19 comment và docstring tiếng Việt trong core sang tiếng Anh kỹ thuật theo tiêu chuẩn của `AGENTS.md`.

### 6. TD-006: Hardcoded Developer Path trong Test (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/storage/mod.rs:459-485`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/mod.rs#L459-L485)
- **Giải pháp:** Thay thế đường dẫn cục bộ Windows fix cứng `C:\Users\maiva\...` bằng in-memory synthetic request collection và tempdir benchmark an toàn trên mọi máy và CI.

### 7. PERF-008 & PERF-009: Cấp phát chuỗi dư thừa trong `models` và `interpolation` (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/storage/models.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/models.rs), [`crates/core/src/curl/mod.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/curl/mod.rs), [`crates/core/src/http/executor.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs)
- **Giải pháp:** Bổ sung move semantics `into_item(self)` và `from_item_owned(item)`. Tái sử dụng một `VariableInterpolator` duy nhất xuyên suốt quá trình interpolate cURL commands và executor thay vì khởi tạo mới per field/header.

### 8. TD-008: `suppressed_paths` rò rỉ implementation detail (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/storage/fs_store.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/fs_store.rs)
- **Giải pháp:** Đóng gói trường `suppressed_paths` thành private và cung cấp accessor thread-safe `suppressed_paths(&self) -> Arc<Mutex<HashSet<PathBuf>>>` trên trait `WorkspaceStore`.

### 9. TD-009: Enum message bloat do enum payload lớn (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/services/http_engine.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/services/http_engine.rs)
- **Giải pháp:** Đóng hộp (Box) các payload lớn `pre_output: Box<PreRequestOutput>` và `post_output: Box<PostResponseOutput>` trong `ExecEvent::Success`, thu nhỏ kích thước message trao đổi qua crossbeam / mpsc channel.

### 10. TD-011: `#[allow(clippy::too_many_arguments)]` (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/app/auth_tab.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/app/auth_tab.rs), [`crates/app/src/app/modals.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/app/modals.rs)
- **Giải pháp:** Gom 8 entities rời rạc thành các struct ngữ cảnh chuyên dụng `AuthTabContext<'a>` và `ModalOverlayEntities<'a>`, xóa bỏ toàn bộ `#[allow(clippy::too_many_arguments)]`.

### 11. PERF-010 & CC-003: Script và Git chạy blocking trên UI thread (🟡 Medium) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/services/http_engine.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/services/http_engine.rs), [`crates/app/src/state/git_ops.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/git_ops.rs)
- **Giải pháp:** Chuyển toàn bộ pre-request script execution, dynamic fake variable resolution, và validation logic vào background Tokio task (`tokio_rt.spawn`); toàn bộ Git CLI operations đã được chạy qua background thread pool không bao giờ làm đông cứng UI.

### 12. TD-09: Struct Padding & Memory Alignment của `TimingBreakdown` (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/models.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/models.rs)
- **Giải pháp:** Đưa trường `total_ms: u64` (8 bytes) lên đầu struct `TimingBreakdown` trước các trường `u32` (4 bytes) để căn chỉnh bộ nhớ tự nhiên và loại bỏ 4-byte padding thừa.

### 13. TD-014: `resolve_ref` trong OpenAPI Importer đệ quy vô hạn (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/importers/openapi.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/importers/openapi.rs)
- **Giải pháp:** Thay thế đệ quy bằng vòng lặp an toàn chu kỳ (`visited: HashSet` + giới hạn độ sâu `depth < 32`), bổ sung unit test `test_openapi_circular_ref_safety`.

### 14. TD-015: `ModalState::default()` thực hiện I/O đọc file cấu hình (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/state/modal_state.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/modal_state.rs)
- **Giải pháp:** Loại bỏ `AppSettings::load()` đồng bộ khỏi `Default::default()`, giữ `default()` hoàn toàn pure in-memory, và chuyển nạp cấu hình tường minh sang `ModalState::new()`.

### 15. TD-017: Giant Component `body_editor.rs` (> 2.100 LOC) (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/app/src/components/body_editor.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/components/body_editor.rs)
- **Giải pháp:** Tách hoàn toàn thành module package `crates/app/src/components/body_editor/`:
  - `json.rs`: `JsonHighlighter`, tokenizer & tests (385 LOC)
  - `xml.rs`: `XmlHighlighter`, `format_xml` & tests (621 LOC)
  - `form.rs`: Form URL-encoded table renderer (81 LOC)
  - `multipart.rs`: Multipart table & row picker renderer (507 LOC)
  - `binary.rs`: Binary stream file picker renderer & file size (217 LOC)
  - `mod.rs`: `BodyEditor` struct, dropdown toolbar & delegates (649 LOC)
  Tất cả các file đều tuân thủ nghiêm ngặt quy định `< 650 LOC` của `AGENTS.md`.

### 16. TD-018 & TD-019: Bảo mật và Timeout cho Git CLI (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/git/mod.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/git/mod.rs)
- **Giải pháp:** Thêm timeout 30 giây cho mọi Git subprocess thông qua `run_git_cmd_timeout` chống treo vĩnh viễn; gia cố `git_discard_file` dùng `git clean -f` và kiểm tra an toàn thư mục gốc trước khi xóa file untracked.

### 17. SL-007: Cấp phát lại bộ nhớ cây thư mục `FolderNode` (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/storage/models.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/models.rs), [`crates/core/src/storage/fs_store.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/fs_store.rs), [`crates/app/src/state/workspace_data_state.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/workspace_data_state.rs)
- **Giải pháp:** Thêm phương thức đếm tổng số phần tử `count_requests()` và `count_folders()` vào `FolderNode` và `WorkspaceTree`; sử dụng `Vec::with_capacity()` và `HashSet::with_capacity()` khi scan và reload tree để loại bỏ việc cấp phát lại bộ nhớ liên tục.

### 18. PERF-013: Cấp phát chuỗi trong so sánh đường dẫn watcher (🔵 Low) — ✅ RESOLVED (2026-10-22)
- **Vị trí:** [`crates/core/src/storage/watcher.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/storage/watcher.rs)
- **Giải pháp:** Loại bỏ việc phân bổ chuỗi mới `.replace('/', "\\").to_lowercase()`, thay thế bằng iterator comparison zero-allocation `t1.chars().map(normalize_ch).eq(t2.chars().map(normalize_ch))`.

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
