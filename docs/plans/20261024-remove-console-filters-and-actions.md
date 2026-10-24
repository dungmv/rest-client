# Remove Filters, Copy All, and Clear Actions from Console Tab

## 1. Goal & Context
The user requested:
1. *"trong tab console bỏ hết fillter và copy all và clear đi."*
2. *"tab console badge hãy sửa lại badge chỉ hiển thị tổng số dòng log không cần chi tiết test như hiện tại"*

In the unified `Console` tab of the Response Inspector, remove the filter controls (`All`, `Tests`, `Logs`, `Errors`), the `Copy All` button, and the `Clear` button.
Update the tab badge on `"Console"` to display only the total count of console log lines (`resp_s.console_logs.len()`), removing test pass/fail details from the tab badge.
All test assertions and console logs will be rendered cleanly and directly without filtering or clutter. Individual console log entries retain their per-row copy actions, and test assertions retain their status badges and details.

---

## 2. Architectural Decisions
1. **Remove Filter Toolbar from `ConsolePanel`**:
   - The toolbar in `crates/app/src/components/console_panel.rs` previously held the filter pills (`All`, `Tests`, `Logs`, `Errors`) on the left and the action buttons (`Copy All`, `Clear`) on the right.
   - Removing these controls eliminates the redundant 38px toolbar completely.
   - Tests and logs will stream directly into the scrollable list container.
2. **Direct Unfiltered Display**:
   - `ConsolePanel::render` will directly render all `test_results` (via `render_test_card`) followed by all `console_logs` (via `render_log_card`).
   - If both are empty (`total_items == 0`), `render_empty_state()` is rendered.
   - Per-row copy functionality on `render_log_card` is retained so users can copy individual messages on demand.
3. **Console Tab Badge Simplification**:
   - In `ResponseInspector`, update `tab-resp-console` badge logic to `format!("{}", resp_s.console_logs.len())` (or `"-"` while sending), matching other tabs like Headers and Cookies.
4. **No Unused Code & Clippy Compliance**:
   - Remove unused imports and helper `render_filter_pill` from `console_panel.rs`.
   - Ensure 0 compiler and clippy warnings.

---

## 3. Implementation Steps
1. **Update `ConsolePanel` (`crates/app/src/components/console_panel.rs`)**:
   - Remove filter calculations, toolbar header `div().h(px(38.))...`, and `render_filter_pill`.
   - Render `all_tests` and `all_logs` directly in the scrollable view.
2. **Update `ResponseInspector` (`crates/app/src/components/response_inspector.rs`)**:
   - Simplify `"tab-resp-console"` badge to display `resp_s.console_logs.len()`.
3. **Verification**:
   - Run `cargo fmt --check`.
   - Run `cargo test`.
   - Run `cargo clippy --all-targets -- -D warnings`.
