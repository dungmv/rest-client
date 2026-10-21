# Danh mục Nợ Kỹ Thuật (Technical Debt Registry)

> **Mục đích:** Tài liệu này ghi nhận toàn diện các khoản nợ kỹ thuật (Technical Debt), các anti-pattern, điểm nghẽn kiến trúc và vấn đề hiệu năng/bộ nhớ tồn đọng trong codebase `restclient` (`restclient-core` và `restclient-app`), kèm mức độ ưu tiên và phương án khắc phục chi tiết.

---

## 📌 Bảng ma trận ưu tiên (Priority Matrix)

| ID | Vấn đề / Khoản nợ | Khu vực ảnh hưởng | Mức độ | Độ phức tạp | Trạng thái |
|:---|:---|:---|:---:|:---:|:---:|
| **TD-01** | Clippy compiler error (`never_loop`) & warnings | `crates/core/src/importers/openapi.rs` | **P0 (Blocker)** | Thấp | 🟢 Đã giải quyết |
| **TD-02** | God Object State (`WorkspaceState` > 3.300 LOC, 70+ fields) | `crates/app/src/state/` | **P0 (Critical)** | Cao | 🟢 Đã giải quyết (2026-10-21) |
| **TD-03** | Unbounded Memory Cache ở `ResponseState` (No LRU) | `crates/app/src/state/response_state.rs` | **P1 (High)** | Trung bình | 🔴 Chưa xử lý |
| **TD-04** | Giant Component Files (> 2.000 LOC / file) | `git_modal.rs`, `sidebar_tree.rs`, `app.rs` | **P1 (High)** | Cao | 🟢 Đã giải quyết (2026-10-21) |
| **TD-05** | Dual-State Sync & Race condition tiềm ẩn | `crates/app/src/app/`, `persistence_state.rs` | **P1 (High)** | Trung bình | 🟢 Đã giải quyết (2026-10-21) |
| **TD-06** | Lạm dụng `.clone()` trong HTTP Request Pipeline | `crates/app/src/pipeline.rs` | **P2 (Medium)** | Thấp | 🟢 Đã giải quyết |
| **TD-07** | Panics tiềm ẩn do `unwrap()` trên Mutex / Option | `crates/core/src/http/executor.rs` | **P2 (Medium)** | Thấp | 🔴 Chưa xử lý |
| **TD-08** | Inconsistent Code Comments & Documentation Language | `crates/core/src/http/executor.rs` | **P2 (Medium)** | Thấp | 🔴 Chưa xử lý |
| **TD-09** | Memory alignment & Padding của Data Structs | `crates/core/src/models.rs` (`TimingBreakdown`) | **P3 (Low)** | Thấp | 🔴 Chưa xử lý |

---

## 🔍 Chi tiết từng khoản nợ kỹ thuật

### TD-01: Lỗi Linter & Compiler Clippy (Blocker)

- **Mô tả:**
  - Lệnh `cargo clippy` đang thất bại ngay lập tức do rule `#[deny(clippy::never_loop)]` tại [`crates/core/src/importers/openapi.rs:396`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/importers/openapi.rs#L396).
  - Vòng lặp `for (media_type, media_obj) in content` thực hiện `return` ngay ở vòng lặp đầu tiên mà không bao giờ lặp sang phần tử kế tiếp.
  - Ngoài ra còn có các warning: `clippy::needless_borrow` (dòng 196), `clippy::manual_strip` (dòng 294) và `clippy::useless_vec` tại [`crates/core/src/http/executor.rs:709`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs#L709).
- **Rủi ro:** Khiến CI/CD pipelines chặn build nếu cấu hình fail on clippy warning; giảm tính nghiêm ngặt của static analysis.
- **Giải pháp:**
  - Thay vòng lặp bằng `if let Some((media_type, media_obj)) = content.into_iter().next()`.
  - Áp dụng `strip_prefix('#')` và loại bỏ mượn lặp `&resolved_param`.
  - Thay `vec![...]` thành slice/array literal `[...]` trong unit test.

---

### TD-02: God Object Anti-Pattern tại `WorkspaceState` (Critical) - 🟢 Đã giải quyết (2026-10-21)

- **Mô tả ban đầu:**
  - `WorkspaceState` phình to lên tới **3.337 dòng code** với hơn **70 fields**.
  - Struct này đảm nhiệm quá nhiều mối quan tâm (vi phạm Single Responsibility Principle):
    1. Dữ liệu nghiệp vụ cốt lõi: collections, requests, folders, environments, pins.
    2. Toàn bộ Git VCS Client UI State: commit message, staging files, diff content, branches, remotes, commit history, syncing status.
    3. Quản lý trạng thái hiển thị của 8 loại modals khác nhau (`show_env_modal`, `show_cookies_modal`, `show_git_modal`,...).
    4. Trạng thái điều khiển inline-rename của sidebar (input entity, subscription của request, folder, env, example, workspace).
    5. Bộ lọc tìm kiếm (`filter_query`, `env_filter_query`, `var_filter_query`).
- **Rủi ro:**
  - Mọi thao tác nhỏ (như gõ phím vào ô commit message hay toggle modal) đều trigger notify redraw trên `Entity<WorkspaceState>`, gây ra re-render không cần thiết cho các component quan sát nó.
  - Cực kỳ khó viết unit test độc lập cho từng module.
- **Kết quả giải quyết (Triệt để - Radical Multi-Entity Architecture):**
  - Xóa bỏ hoàn toàn God Object `WorkspaceState` cũ và không sử dụng bất kỳ monolithic facade/proxy ủy quyền nào.
  - Phân rã thành 6 GPUI Entity States độc lập với đơn trách nhiệm và ranh giới rõ ràng:
    1. **`WorkspaceDataState`** (`crates/app/src/state/workspace_data_state.rs` ~341 LOC & `workspace_crud.rs` ~289 LOC): Chỉ chứa pure domain data (requests, folders, pins, workspace metadata, CRUD operations, hierarchy resolution).
    2. **`EnvironmentState`** (`crates/app/src/state/environment_state.rs` ~218 LOC): Quản lý toàn bộ danh sách environments, global environment, active environment ID, corrupt secrets, effective environment fallback (`effective_environment()`), tạo/sao chép/đổi tên/xóa environment và lưu disk debounced.
    3. **`PersistenceState`** (`crates/app/src/state/persistence_state.rs` ~311 LOC): Quản lý persistence I/O, `WorkspaceStore`, background `WorkspaceWatcher`, nhận `FsEvent`, external conflicts, dirty unsaved requests tracking và debounced save background tasks.
    4. **`GitState`** (`crates/app/src/state/git_state.rs` ~424 LOC & `git_ops.rs` ~274 LOC): Đảm nhiệm toàn bộ VCS operations, staging, commit, diff view, branch switcher, remotes, history.
    5. **`ModalState`** (`crates/app/src/state/modal_state.rs` ~238 LOC): Quản lý cờ hiển thị của tất cả modals (`env`, `git`, `settings`, `import`, `cookies`, `scope_settings`, `rename_workspace`), theme preference, split layout vertical.
    6. **`SidebarState`** (`crates/app/src/state/sidebar_state.rs` ~396 LOC): Quản lý sidebar width, folder/request collapsed sets, inline renaming controls/subscriptions cho request/folder/env/example/workspace, navigation history (`record_navigation`, `navigate_back`, `navigate_forward`).
  - Xóa bỏ hoàn toàn các file ngụy trang god object trước đó (`workspace_env.rs`, `workspace_watcher.rs`).
  - Toàn bộ UI components và AppView chuyển thẳng sang cú pháp ngữ nghĩa tường minh (ví dụ: `modal_state.open_env_modal(...)`, `git_state.commit(...)`, `sidebar_state.start_renaming(...)`, `env_state.set_active_environment(...)`), triệt tiêu hoàn toàn re-render chéo giữa Git/Modals/Sidebar/Environments/Persistence và Data core.
  - Mọi file đều tuân thủ nghiêm ngặt giới hạn độ dài `< 650 LOC` (phần lớn `< 450 LOC`).
  - Toàn bộ 114 unit tests passed 100%, 0 clippy warnings với `-D warnings`.

---

### TD-03: Nguy cơ Memory Leak do Unbounded Cache tại `ResponseState` (High)

- **Mô tả:**
  - Trong [`crates/app/src/state/response_state.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/response_state.rs#L50-L51):
    ```rust
    pub session_responses: HashMap<String, HttpResponseRecord>,
    pub previous_responses: HashMap<String, HttpResponseRecord>,
    ```
  - Hai `HashMap` này lưu trữ toàn bộ lịch sử response trong phiên làm việc theo `request_id`.
  - [`HttpResponseRecord`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/models.rs#L507-L525) chứa trường `body: Vec<u8>`.
  - Không có giới hạn số lượng phần tử (capacity limit) hay thời gian hết hạn (TTL/LRU eviction).
- **Rủi ro:** Nếu người dùng thực hiện tải các file nhị phân (ảnh, video, PDF) hoặc thực hiện hàng trăm requests với payload JSON lớn (vài chục MB), bộ nhớ RAM của app sẽ tăng liên tục không giải phóng cho đến khi tắt ứng dụng.
- **Giải pháp:**
  - Thay thế `HashMap` bằng cấu trúc **LRU Cache** (ví dụ giới hạn lưu tối đa 50-100 response gần nhất hoặc tối đa 100MB tổng dung lượng).
  - Tránh lưu toàn bộ raw binary body trong RAM session; đối với payload > 5MB, ghi tạm ra file `.loc` hoặc cache disk và chỉ đọc theo luồng khi người dùng mở tab response.

---

### TD-04: Giant Component Files (> 2.000 LOC) (High) - 🟢 Đã giải quyết (2026-10-21)

- **Mô tả ban đầu:**
  - Các file UI component có kích thước quá lớn:
    - `git_modal.rs`: **2.508 dòng**.
    - `app.rs`: **2.271 dòng**.
    - `sidebar_tree.rs`: **2.196 dòng**.
  - Các file này trộn lẫn giữa:
    - Khởi tạo sub-view GPUI element.
    - Xử lý sự kiện bàn phím/chuột.
    - Logic tính toán diff / cây thư mục / trạng thái icon.
- **Rủi ro:** Giảm tính khả đọc, khó review PR, tăng khả năng conflict mã nguồn khi nhiều người cùng đóng góp.
- **Kết quả giải quyết:**
  - **`components/git/`**: Thay thế hoàn toàn `git_modal.rs` bằng 7 modules chuyên biệt (`branch_modal.rs`, `commit_modal.rs`, `common.rs`, `diff_view.rs`, `history_modal.rs`, `remotes_modal.rs`, `mod.rs`). Xoá bỏ file monolith cũ `git_modal.rs`.
  - **`components/sidebar/`**: Thay thế hoàn toàn `sidebar_tree.rs` bằng 8 modules (`drag_drop.rs`, `example_node.rs`, `folder_node.rs`, `footer.rs`, `git_status.rs`, `item_actions.rs`, `request_node.rs`, `mod.rs`). Xoá bỏ file monolith cũ `sidebar_tree.rs`.
  - **`app/`**: Tách `app.rs` nguyên khối thành package `crates/app/src/app/` gồm 8 modules (`actions.rs`, `auth_input.rs`, `auth_tab.rs`, `editor_pane.rs`, `modals.rs`, `subscriptions.rs`, `sync.rs`, `mod.rs`). Xoá bỏ file `app.rs`.
  - Mọi subfile đều dưới ~650 LOC, toàn bộ 109 unit tests vượt qua 100%, không phát sinh warning mới trên clippy.

---

### TD-05: Dual-State Synchronization & Race Condition (High) - 🟢 Đã giải quyết (2026-10-21)

- **Mô tả ban đầu:**
  - Request đang được mở nằm đồng thời ở nhiều nơi:
    1. Input buffer của `InputState` / `CodeEditorState` (trên UI thread).
    2. `EditorState.active_request` (trong entity state).
    3. Danh sách `WorkspaceDataState.requests` (danh sách nguồn từ disk).
  - Quá trình đồng bộ được thực hiện bán thủ công qua hàm `save_active_request_without_redraw` và cơ chế debounce.
- **Rủi ro:** Khi chuyển tab nhanh, đóng tab hoặc khi File Watcher từ bên ngoài báo thay đổi file trên ổ đĩa, có khả năng dữ liệu đang gõ dở bị ghi đè hoặc bị desync giữa UI buffer và disk storage. Đặc biệt xuất hiện lỗi khi vừa click chọn request trong sidebar, request đã lập tức bị đánh dấu modified (chuyển vàng) do scheduler tự kích hoạt dirty save.
- **Kết quả giải quyết:**
  - Tách riêng `PersistenceState` chuyên trách việc auto-save debounced, file watcher và conflict detection.
  - Sửa đổi cơ chế dirty tracking trong `subscriptions.rs`:
    - Khi chuyển request (`last_req_id != req_id`), observer trên `editor_state` chỉ nạp cache và cập nhật ID, tuyệt đối không lập lịch auto-save và không đưa vào `unsaved_requests`. Khắc phục triệt để lỗi request vừa click đã chuyển màu vàng và báo modified giả.
    - Cơ chế auto-save debounced chỉ kích hoạt khi đang ở cùng request và request thực sự có biến đổi (`is_dirty == true`).
    - Snapshot `req_snapshot` an toàn khi lập lịch lưu để đảm bảo nếu người dùng chuyển tab ngay sau khi gõ, dữ liệu đã gõ vẫn được flush xuống disk đúng đắn mà không bị desync.
    - Đặt lại `ed.is_dirty = false` ngay khi background task ghi file xuống disk thành công.
  - Xử lý external conflicts thông qua `conflicts` map của `PersistenceState`, ngăn chặn watcher ghi đè lên thay đổi chưa lưu của người dùng.

---

### TD-06: Lạm dụng `.clone()` trong Request Execution Pipeline (Medium)

- **Mô tả:**
  - Trong [`crates/app/src/pipeline.rs`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/pipeline.rs), quá trình giải quyết biến giả động (dynamic fake vars) và hierarchy resolution thực hiện nhiều lệnh clone liên tiếp:
    ```rust
    let Some(mut req_item) = ed.active_request.clone() else { return; };
    ...
    req_item.headers = resolved_headers; // clone
    ...
    ```
- **Rủi ro:** Tạo ra các heap allocations dư thừa trong chu kỳ gửi request tần suất cao (ví dụ khi chạy Collection Runner).
- **Giải pháp:**
  - Tái cấu trúc hàm resolve biến để sử dụng mượn tham chiếu (`&str`) hoặc `Cow<'a, str>`, chỉ cấp phát heap mới khi thực sự có placeholder `{{...}}` cần thay thế.

---

### TD-07: Tiềm ẩn Panic do `unwrap()` trên Mutex / Option (Medium)

- **Mô tả:**
  - Trong [`crates/core/src/http/executor.rs:370, 379`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs#L370-L380):
    ```rust
    let store = self.cookie_store.lock().unwrap();
    ```
  - Tại [`crates/app/src/state/editor_state.rs:77`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/app/src/state/editor_state.rs#L77):
    ```rust
    self.active_request.as_mut().unwrap()
    ```
- **Rủi ro:** Nếu một thread gặp panic khi đang giữ mutex của `CookieStore`, lock sẽ bị "poisoned", khiến tất cả các lần đọc cookie sau đó gây crash toàn bộ ứng dụng.
- **Giải pháp:**
  - Xử lý poisoned lock một cách an toàn:
    ```rust
    let store = self.cookie_store.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    ```
  - Thay thế các `unwrap()` tự định dạng bằng pattern matching hoặc `ok_or_else(...)` có thông báo lỗi chi tiết.

---

### TD-08: Ngôn ngữ chú thích (Doc Comments) thiếu nhất quán (Medium)

- **Mô tả:**
  - Phần lớn dự án sử dụng tiếng Anh chuẩn cho tài liệu kỹ thuật và code comments.
  - Tuy nhiên, xuất hiện một số đoạn doc comment tiếng Việt tại [`crates/core/src/http/executor.rs:324`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/http/executor.rs#L324):
    ```rust
    /// Áp dụng các thay đổi từ Pre-request Script (headers, body JSON override)
    pub fn apply_pre_script_mutations(...)
    ```
- **Rủi ro:** Giảm tính chuyên nghiệp của codebase mã nguồn mở khi cộng tác với các lập trình viên quốc tế.
- **Giải pháp:** Đồng bộ hóa 100% comment trong source code sang tiếng Anh, giữ tiếng Việt cho hệ thống tài liệu trong `docs/`.

---

### TD-09: Struct Padding & Memory Alignment (Low)

- **Mô tả:**
  - Struct [`TimingBreakdown`](file:///Users/dungmv/Documents/macos-native-apps/rest-client/crates/core/src/models.rs#L394-L403) khai báo 5 trường `Option<u64>` liên tiếp.
  - Trên hệ 64-bit, mỗi trường `Option<u64>` chiếm 16 bytes (8 bytes giá trị + 1 byte discriminant tag + 7 bytes alignment padding).
  - Kết hợp với `total_ms: u64` và `connection_reused: bool`, tổng kích thước struct lên tới 96 bytes.
- **Rủi ro:** Tốn cache line và băng thông bộ nhớ khi lưu trữ hàng ngàn bản ghi lịch sử test runner.
- **Giải pháp:**
  - Đóng gói các giá trị `Option<u64>` (vốn biểu thị thời gian ms hiếm khi vượt quá $2^{32}$) thành `Option<u32>` hoặc sử dụng sentinel value (ví dụ `u64::MAX` đại diện cho None) để nén struct về kích thước nhỏ gọn (~48 bytes).

---

## 🛠️ Lộ trình giải quyết khuyến nghị (Action Plan)

```mermaid
flowchart TD
    subgraph Sprint 1: Stability & Cleanliness
        TD01["TD-01: Fix Clippy Error & Warnings (Đã xong)"]
        TD08["TD-08: Standardize English Comments"]
        TD07["TD-07: Safe Mutex & Unwrap Handling"]
    end

    subgraph Sprint 2: Memory & Leak Prevention
        TD03["TD-03: Add LRU Eviction to Response Cache"]
        TD06["TD-06: Optimize Pipeline Allocations (Đã xong)"]
        TD09["TD-09: Compact Struct Layouts"]
    end

    subgraph Sprint 3: Structural Refactoring
        TD02["TD-02: Split WorkspaceState God Object (Đã xong)"]
        TD04["TD-04: Modularize Giant UI Components (Đã xong)"]
        TD05["TD-05: Robust Dirty-Tracking Sync (Đã xong)"]
    end

    Sprint 1 --> Sprint 2 --> Sprint 3
```
