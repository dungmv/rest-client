# Plan v2: Refactor kiến trúc, giải quyết SL-001 (và PERF-006)

> Tài liệu này dành cho agent thực thi (Antigravity). Đọc hết mục 0 và mục 1 trước khi sửa bất kỳ file nào.

## 0. Quy tắc làm việc (bắt buộc)

1. **Một bước = một commit.** Không gộp nhiều bước. Message commit dạng `refactor(sl-001): step N - <tên bước>`.
2. **Sau mỗi bước phải xanh:** `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`. Nếu đỏ, sửa trong cùng bước; không chuyển bước tiếp theo khi chưa xanh.
3. **Không đổi hành vi**, trừ ngoại lệ duy nhất là Bước 1 (gỡ bỏ hệ thống tab cũ). Các bước còn lại là refactor thuần cấu trúc: không thêm tính năng, không sửa bug ngoài phạm vi (nếu thấy bug, ghi lại ở cuối báo cáo, không sửa).
4. **Không thêm dependency mới** vào `Cargo.toml` nếu chưa có sẵn trong workspace. Nếu một bước cần dependency chưa có, dừng và hỏi.
5. **Không tạo code giữ chỗ:** không enum/struct/engine rỗng hay đoạn code bị comment cho tương lai (không `WebSocketEngine`, `GrpcEngine`, `ProtocolKind`...).
6. **Không giả định:** plan này được viết từ mô tả, chưa đối chiếu code thật. Nếu tên file, tên struct hay số field khác với plan, ưu tiên code thật và ghi lại chỗ khác biệt trong báo cáo.
7. **Quy tắc GPUI cần giữ:**
   - Không `update` một entity khi đang nằm trong `update` của chính nó (reentrancy sẽ panic). Dùng `emit` + `subscribe` hoặc `cx.defer`.
   - Task async chỉ giữ `WeakEntity`, không clone `Entity` vào task chạy dài.
   - Tên API (`observe_in`, `subscribe_in`, `Context<T>`, closure của `cx.spawn`...) phụ thuộc phiên bản GPUI đang dùng; kiểm tra phiên bản trong repo trước khi viết.
8. Khi gặp điểm mơ hồ có thể làm hỏng hành vi, **dừng và hỏi**, đừng đoán.

## 1. Mục tiêu và ngoài phạm vi

### Mục tiêu
1. Giải quyết **SL-001** (`AppView` God Struct, 32 field) trong `docs/technical-debt-02.md`.
2. Giải quyết **PERF-006**: không còn logic đồng bộ thủ công/polling trong `render()`.
3. Đổi tên các khái niệm bị gọi nhầm là "Editor".
4. Tách service thực thi HTTP khỏi UI và khỏi `AppState`, với giao diện sẵn sàng cho giao thức streaming sau này mà không thêm code cho giao thức chưa làm.
5. **Gỡ bỏ hệ thống tab cũ (legacy tabs) ngay từ đầu** để giảm state, field và nhánh render cần xử lý trong các bước tách sau đó.

### Ngoài phạm vi (không làm trong plan này)
- Không implement WebSocket, gRPC, GraphQL. Không thêm trường `protocol` vào định dạng yaml của request.
- Không đổi định dạng file lưu trên đĩa (yaml request, `.loc`, thư mục `.local/`).
- Không thiết kế tính năng pin mới; chỉ giữ hoặc di chuyển pin hiện có (xem Bước 1).

## 2. Kiến trúc đích

Quy tắc phụ thuộc, chỉ đi một chiều:

```
ui (views)  ->  state (Entity, ViewModel)  ->  core / services (không biết gpui, không biết state)
```

| Tầng | Chứa gì | Được import |
|---|---|---|
| Views | `AppView`, `RequestComposer`, `ResponseInspector`, `SidebarTree`... | state, controls, gpui |
| Controls (view-state) | Các handle widget: `InputState`, `CodeEditorState`, `TextareaState` | state, gpui |
| State | Các `Entity<...State>` | core, gpui |
| Service | `HttpEngine`, hàm `execute` | chỉ core/model, KHÔNG gpui, KHÔNG state |

Lý do: logic mạng test được bằng `cargo test` thường; việc thêm một giao thức sau này không đụng vào state hay view hiện có.

## 3. Bảng đổi tên

| Tên cũ | Tên mới | Bản chất |
|---|---|---|
| `EditorState` | `RequestSessionState` | State phiên làm việc: active request, cờ đang gửi, tiến độ, cancel (không còn tab) |
| `EditorPane` | `RequestComposer` | View: soạn thảo và cấu hình request |
| `ResponseViewer` | `ResponseInspector` | View: kiểm tra response (chỉ đọc) |
| `RequestEditors` | `ComposerInputs` | Controls: các handle input của form soạn thảo |
| `ResponseEditors` | `InspectorDisplays` | Controls: các handle hiển thị response (chỉ đọc) |
| `HttpService` / `pipeline.rs` | `HttpEngine` | Service thực thi HTTP |

Giữ nguyên `gpui_component::input::EditorState` (của thư viện), không đổi.

## 4. Các bước thực hiện

### Bước 0: Khảo sát (không sửa code)
- Chạy `cargo test`, ghi lại số test pass (kỳ vọng 126) và xác nhận `cargo clippy` sạch. Nếu baseline đã đỏ, dừng và báo.
- Liệt kê toàn bộ field của `AppView` (`crates/app/src/app/mod.rs`) và phân loại từng field vào 1 trong 4 nhóm: input handle (ghi), display handle (chỉ đọc), state entity, khác.
- Tìm mọi chỗ trong `render()` của `AppView` và các view con gọi hàm đồng bộ/cập nhật state (nguồn của PERF-006).
- Lập danh sách mọi thứ liên quan đến tab: field (`open_tabs`, `pinned_tabs`...), view thanh tab, action và phím tắt, chỗ lưu/nạp trạng thái tab trên đĩa, test phụ thuộc tab. Tìm xem pin đã tồn tại ở sidebar chưa.
- Xác nhận các file trong plan có tồn tại đúng đường dẫn không.
- **Đầu ra:** báo cáo ngắn trong phản hồi (không commit file mới). Nếu số liệu lệch nhiều so với plan, dừng và hỏi.

### Bước 1: Gỡ bỏ hệ thống tab cũ

Mục đích: giảm độ phức tạp trước khi tách `AppView`. Đây là thay đổi hành vi có chủ đích duy nhất của plan này. Mô hình mới: **mỗi thời điểm chỉ mở một request** (`active_request_id`); chọn request ở sidebar sẽ mở thẳng trong composer.

- Xóa thanh tab (view), `open_tabs`, các action và phím tắt liên quan tab (đóng tab, chuyển tab, mở tab mới...), logic xử lý tab trong session state và trong `AppView`.
- Xóa hoặc sửa các test chỉ kiểm tra hành vi tab; test còn lại phải giữ nguyên ý nghĩa.
- **Pin:** nếu sidebar đã có pin, xóa phần `pinned_tabs` trùng lặp. Nếu pin chỉ tồn tại dưới dạng `pinned_tabs`, chuyển sang `SidebarState` với cùng ngữ nghĩa. Nếu việc này buộc phải đổi định dạng file lưu trên đĩa, **dừng và hỏi**.
- **Dữ liệu tab cũ đã lưu** (nếu có, ví dụ trong `.local/`): khi nạp phải bỏ qua key cũ mà không báo lỗi, và không xóa file của người dùng.
- **Thay đổi chưa lưu:** giữ nguyên cơ chế auto-save/dirty hiện có khi chuyển request. Nếu có hành vi phụ thuộc tab (ví dụ giữ bản nháp riêng theo từng tab), liệt kê và hỏi trước khi quyết định.
- Dọn dead code: sau khi xóa, rà `grep -rniE "open_tabs|pinned_tabs|tab_bar|TabBar|close_tab|switch_tab"` (và các tên thực tế tìm thấy ở Bước 0), không để lại import, action hay field mồ côi.
- Không đổi tên `EditorState` ở bước này (đổi tên ở Bước 5).
- **Nghiệm thu:** không còn tham chiếu tới hệ thống tab; chọn request ở sidebar, pin (nếu có), gửi request, mở lại app đều hoạt động; test/clippy xanh; liệt kê trong báo cáo những field và file đã xóa.

### Bước 2: Tách `InspectorDisplays` (chỉ đọc)
- Tạo `crates/app/src/app/inspector_displays.rs` chứa các handle hiển thị response: `response_body_display`, `sent_url_display`, `sent_body_display` (và các handle display tương tự tìm thấy ở Bước 0), cùng các biến tracking `last_synced_*` hiện có.
- Chuyển phương thức đồng bộ response (hiện nằm trong `AppView`) sang struct mới, **giữ nguyên logic và nơi gọi**.
- `AppView` giữ một field `inspector_displays: InspectorDisplays` thay cho các field lẻ.
- **Nghiệm thu:** hành vi giống hệt; số field `AppView` giảm tương ứng; test/clippy xanh.

### Bước 3: Tách `ComposerInputs`
- Tạo `crates/app/src/app/composer_inputs.rs` chứa các handle input: URL, auth (bearer, basic user/pass, api key key/value), body (json, xml, raw/textarea, các editor khác nếu có), script (pre, test), cùng các biến tracking `last_synced_*`.
- Chuyển phương thức đồng bộ từ item sang input và phần đăng ký subscription liên quan sang struct mới, **giữ nguyên logic**.
- `AppView` giữ `composer_inputs: ComposerInputs`.
- **Nghiệm thu:** như Bước 2.

### Bước 4: Đồng bộ theo sự kiện, bỏ polling khỏi `render()` (PERF-006)
- Thay cơ chế "mỗi frame so version rồi đồng bộ" bằng đồng bộ **kích hoạt bởi sự kiện**: observe/subscribe vào entity liên quan (session cho request đang chọn, workspace cho dữ liệu request, response state cho response). Khi sự kiện xảy ra mới nạp lại vào các handle. Dùng biến thể có `window` (`observe_in`/`subscribe_in` hoặc tương đương theo phiên bản GPUI) vì việc nạp vào input cần `window`.
- Sau bước này `render()` của mọi view không được gọi hàm đồng bộ hay mutate state. Kiểm tra bằng cách rà soát thủ công danh sách đã lập ở Bước 0.
- Giữ cơ chế chống vòng lặp (nạp vào input không được kích hoạt ngược lại việc ghi vào state). Nếu cần, thêm cờ nội bộ trong controls.
- Các biến `last_synced_*_version` chỉ được xóa nếu không còn dùng; nếu còn cần để chống vòng lặp, giữ lại và ghi chú lý do.
- **Nghiệm thu:** `render()` thuần (chỉ đọc state, dựng UI); kiểm tra thủ công: đổi request ở sidebar, sửa URL/body/auth, gửi request, xem response, đều cập nhật đúng và không giật.

### Bước 5: Đổi tên (commit riêng, không đổi logic)
- Đổi theo bảng mục 3. Với `EditorState`, tạm giữ alias `pub type EditorState = RequestSessionState;` để test cũ không gãy.
- Đổi tên file tương ứng (`editor_state.rs` -> `request_session_state.rs`...).
- Không thay đổi cấu trúc field hay logic trong commit này.
- **Nghiệm thu:** `git diff --stat` chỉ gồm đổi tên/import; mọi thứ xanh.

### Bước 6: Tách `HttpEngine` và giao diện thực thi theo luồng sự kiện
- Chuyển nội dung `pipeline.rs` thành `HttpEngine` trong `crates/app/src/services/` (hoặc crate phù hợp nếu repo đã có crate core). Thành phần này **không được import gpui và không được import bất kỳ `*State` nào**.
- Định nghĩa giao diện:
  ```rust
  pub enum ExecEvent {
      Started,
      Progress(SendProgress),   // dùng kiểu tiến độ sẵn có
      Response(ResponseData),   // dùng kiểu response sẵn có
      Closed(Result<(), ExecError>),
  }
  // Trả về luồng sự kiện. Dùng futures::Stream nếu workspace đã có `futures`;
  // nếu chưa, dùng receiver của channel sẵn có (tokio/async channel) thay vì thêm dependency.
  pub fn execute(request: ResolvedRequest, ctx: ExecCtx) -> impl Stream<Item = ExecEvent>;
  ```
  Chỉ định nghĩa các variant mà HTTP hiện cần. Không thêm variant cho streaming khi chưa có giao thức dùng chúng.
- Engine nhận đầu vào là dữ liệu thuần (request đã resolve hierarchy, biến môi trường, script cần chạy) và trả về sự kiện. Pre-script, gửi, post-script nằm trong engine. Hành vi giống hệt hiện tại.
- Tạo `Runner` (entity hoặc đưa vào `ResponseState`, chọn cái ít xáo trộn hơn và ghi rõ lựa chọn): nhận luồng sự kiện và là nơi **duy nhất** thực hiện side-effect lên state: cập nhật `is_sending`/`send_progress`/response, ghi cache response (`.loc`), toast, persistence.
- Chuyển `send_current_request` khỏi `AppView` sang luồng: `AppView` (hoặc action) -> `Runner` -> `HttpEngine::execute`.
- Giữ cơ chế hủy hiện có (`cancel_tx` hoặc tương đương); việc drop/hủy luồng phải dừng request như trước.
- **Nghiệm thu:** `HttpEngine` compile được mà không cần gpui (thử tách test: ít nhất một test của `execute` với server giả hoặc mock, nếu repo đã có hạ tầng mock); hành vi gửi/hủy/response giữ nguyên.

### Bước 7: `AppState` chỉ là container khởi tạo
- Tạo `crates/app/src/state/app_state.rs` gom các entity (`workspace`, `env`, `persistence`, `git`, `session`, `response`, `sidebar`, `modal`, `toast`) và hàm `AppState::new(cx)` để khởi tạo một chỗ.
- **Không truyền `&AppState` vào view/component.** Mỗi component nhận đúng các entity nó dùng (ví dụ `SidebarTree::new(workspace, session, sidebar, cx)`). `AppState` chỉ dùng ở nơi khởi tạo/wiring (`app.rs`/`AppView::new`).
- Loại bỏ prop-drilling bằng cách truyền entity cụ thể hoặc nhóm nhỏ, không bằng một đối tượng truy cập tất cả.
- **Nghiệm thu:** không component UI nào có tham số kiểu `AppState`; `AppView` chỉ còn: các entity/controls/subscription/trạng thái layout. Ghi lại danh sách field còn lại và lý do tồn tại của từng field.

### Bước 8: Dọn dẹp và tài liệu
- Xóa alias chuyển tiếp `EditorState` nếu không còn dùng.
- Cập nhật `docs/technical-debt-02.md`: đánh dấu SL-001 và PERF-006 đã giải quyết, kèm tóm tắt thay đổi và các điểm lệch so với plan (nếu có).
- Chạy kiểm tra cuối (mục 6).

## 5. Quyết định còn mở (agent không tự quyết)

- **Pin:** nếu việc giữ pin buộc phải đổi định dạng dữ liệu lưu trên đĩa, hoặc có hành vi phụ thuộc tab chưa rõ (xem Bước 1), dừng và hỏi chủ dự án.
- Mọi điểm lệch khác so với plan: ghi lại trong báo cáo cuối.

## 6. Kiểm tra cuối

```bash
cargo build
cargo test            # số test pass bằng baseline ở Bước 0
cargo clippy --all-targets -- -D warnings
```

Kiểm tra cấu trúc:
- `grep -rn "use gpui" <thư mục HttpEngine>` không có kết quả.
- Không có view/component nào nhận tham số kiểu `AppState`.
- `render()` của các view không gọi hàm đồng bộ hay mutate state.

Kiểm tra thủ công:
1. Soạn request: URL, Params, Headers, Auth (Bearer, Basic, API Key), Body (JSON, XML, Form, Raw), Script; giá trị cập nhật tức thì, auto-save hoạt động.
2. Chuyển request ở sidebar: các input nạp đúng dữ liệu của request mới, không lẫn dữ liệu request trước.
3. Gửi bằng `Cmd+Enter`: status, thời gian, kích thước, tô màu cú pháp, sent preview, test assertion, console đều đúng; hủy request giữa chừng hoạt động.
4. Đóng/mở lại app: response cache nạp lại như trước.
5. Chuyển request liên tục, kéo thanh chia panel: không giật, không re-render dây chuyền.
6. Không còn thanh tab hay phím tắt tab; chọn request ở sidebar mở thẳng request đó; pin (nếu có) vẫn hoạt động; mở lại app với dữ liệu cũ không báo lỗi.

## 7. Báo cáo cuối cùng

Sau khi xong, trả lời ngắn gọn: (1) danh sách commit theo bước, (2) số field `AppView` trước/sau và lý do từng field còn lại, (3) các điểm lệch so với plan, (4) danh sách field/file/action/test đã xóa khi gỡ tab và cách xử lý pin, (5) bug hoặc nợ kỹ thuật phát hiện thêm nhưng chưa sửa.
