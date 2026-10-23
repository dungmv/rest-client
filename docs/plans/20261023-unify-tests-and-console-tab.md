# Unify `Tests` and `Console` Tabs in Response Inspector

## 1. Goal & Context

Currently, the Response Inspector separates script execution outputs into two distinct tabs: **`Tests`** (`ResponseTabKind::Tests` rendered via `TestResultsPanel`) and **`Console`** (`ResponseTabKind::Console` rendered via `ConsolePanel`). Because both tabs display output produced by the same pre-request and post-response scripting lifecycle (`z.test(...)` assertions and `console.*` logs), splitting them forces users to switch back and forth when debugging scripts.

This plan merges `Tests` and `Console` into a single **`Console`** tab featuring a unified filter bar (`All`, `Tests`, `Logs`, `Errors`), a Pass/Fail test summary badge, and a combined scrollable view of test results and console log entries.

---

## 2. Architectural Decisions

1. **Single `ResponseTabKind::Console` Variant**:
   - Remove `ResponseTabKind::Tests` from `crates/app/src/state/response_state.rs`.
   - Keep `ResponseTabKind::Console` as the unified tab for both test assertion results and console logs.
2. **Unified `ConsoleFilter` Enum**:
   - Update `ConsoleFilter` in `crates/app/src/state/response_state.rs` to filter across both `TestResult` and `ConsoleLogEntry` items:
     - `All`: Shows all test results and all console logs.
     - `Tests`: Shows only `TestResult` items (`z.test(...)`).
     - `Logs`: Shows only `ConsoleLogEntry` items (`console.log/info/warn/error`).
     - `Errors`: Shows failed `TestResult` items (`!res.passed`) and error-level `ConsoleLogEntry` items (`level == ConsoleLogLevel::Error`).
   - Individual console log rows still retain their `Pre-request` / `Post-response` source badge so script phase context is never lost.
3. **Component Consolidation (< 650 LOC limit)**:
   - Merge the test result card rendering from `crates/app/src/components/test_results_panel.rs` directly into `crates/app/src/components/console_panel.rs` (~380 LOC combined, well within the 650 LOC rule in `AGENTS.md`).
   - Delete `crates/app/src/components/test_results_panel.rs` and remove its export from `crates/app/src/components/mod.rs`.

---

## 3. Step-by-Step Implementation Tasks

### Step 1: Update `ResponseState` (`crates/app/src/state/response_state.rs`)
- Remove `Tests` from `ResponseTabKind`.
- Update `ConsoleFilter` variants to:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum ConsoleFilter {
      #[default]
      All,
      Tests,
      Logs,
      Errors,
  }
  ```
- Update `clear_console_logs` in `ResponseState` so clicking `Clear` in the unified `Console` tab clears both `console_logs` and `test_results` for the active session.
- Update unit tests `test_console_tab_kind` and `test_console_logs_filtering` in `response_state.rs`.

### Step 2: Update `ResponseInspector` (`crates/app/src/components/response_inspector.rs`)
- Remove `TestResultsPanel` import.
- In the tab bar (`response_inspector.rs`):
  - Remove `tab-resp-tests`.
  - Update `tab-resp-console` (`"Console"`) badge logic:
    - If `total_tests > 0 && total_logs > 0`: show `format!("{}/{} • {}", passed_tests, total_tests, total_logs)`.
    - If `total_tests > 0`: show `format!("{}/{}", passed_tests, total_tests)`.
    - If `total_logs > 0`: show `format!("{}", total_logs)`.
    - Otherwise: `None`.
- In the tab content matcher (`response_inspector.rs`):
  - Remove the `ResponseTabKind::Tests` arm.
  - In `ResponseTabKind::Console`, show the loading spinner when `is_sending_this_request` is true (`"Sending request... Scripts and tests will execute after response is received."`), otherwise render `ConsolePanel::new(resp_state_clone.clone())`.

### Step 3: Build Unified `ConsolePanel` (`crates/app/src/components/console_panel.rs`) & Remove `TestResultsPanel`
- **Top Toolbar**:
  - **Filter Buttons (Left)**:
    - `All (total_tests + total_logs)`
    - `Tests (total_tests)`
    - `Logs (total_logs)`
    - Conditional `Errors (failed_tests + error_logs)` (rendered with error styling when `failed_tests + error_logs > 0` or when `ConsoleFilter::Errors` is active).
  - **Pass/Fail Summary Pill**:
    - When `total_tests > 0`, display a compact status badge (`✓ X/Y Passed` in green when all pass, or `✗ X/Y Passed` in red when any fail).
  - **Actions (Right)**:
    - `Copy All`: Copies currently visible test results and/or console logs to the clipboard.
    - `Clear`: Calls `state.clear_console_logs(cx)` to clear current test results and console logs.
- **Unified Scrollable List**:
  - **Empty State**: When both `test_results` and `console_logs` are empty (or no items match the active filter), show a unified empty state with a brief snippet demonstrating both `z.test(...)` and `console.log(...)`.
  - **Test Results Section** (rendered when filter is `All`, `Tests`, or `Errors` and matching tests exist):
    - Render each matching `TestResult` card with pass/fail badge (`✓`/`✗`), test name, `duration_ms`, and `↳ error_message` on failure.
  - **Console Logs Section** (rendered when filter is `All`, `Logs`, or `Errors` and matching logs exist):
    - Render each matching `ConsoleLogEntry` row with timestamp, `Pre-request`/`Post-response` badge, `LOG`/`INFO`/`WARN`/`ERROR` level badge, hover `Copy` button, and monospace message.
- Delete `crates/app/src/components/test_results_panel.rs` and remove `pub mod test_results_panel;` / `pub use test_results_panel::*;` from `crates/app/src/components/mod.rs`.

---

## 4. Risk Mitigation
- **Pure Rendering (`PERF-006`)**: Filtering over `test_results` and `console_logs` in `ConsolePanel::render()` operates strictly on borrowed in-memory slices from `ResponseState` with zero state mutations or blocking IO.
- **File Size Constraint**: Merging the two panels yields ~380 lines in `console_panel.rs`, well below the 650 LOC ceiling.

---

## 5. Verification Steps
1. `cargo fmt --check` — Verify code formatting.
2. `cargo test` — Run all unit tests across `zest-core` and `zest` (including updated `response_state` tests).
3. `cargo clippy --all-targets -- -D warnings` — Ensure 0 warnings and no unused imports/dead code after removing `TestResultsPanel` and `ResponseTabKind::Tests`.
