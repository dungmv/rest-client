# Implementation Plan: Replace Copy Buttons with Selectable TextView (No Blinking Cursor)

## Goal Description
Simplify the **Request Sent** tab UI by:
1. Removing all copy buttons (`Copy URL`, `cURL`, `Copy All`, per-header copy buttons, and `Copy Body`).
2. Replacing `Textarea` and `div`-based non-selectable text with **`gpui_component::text::TextView`** configured with `.selectable(true)`.
3. Eliminating the annoying blinking text cursor (`|`) that `Textarea`/`Input` creates, giving the inspector a clean, native, read-only document appearance while enabling mouse selection ("bôi đen") and standard keyboard copy (`Cmd+C` / `Ctrl+C`).

---

## Technical Rationale: Why `TextView` instead of `Textarea`?
- **No Blinking Cursor (No Caret)**: `Textarea` and `Editor` use `gpui_base::input::blink_cursor`, which paints a blinking vertical line whenever focused or clicked. `TextView` is a pure display element with zero carets.
- **Full Text Selection**: `TextView::markdown(id, content).selectable(true)` natively supports drag-to-select, word selection, and multi-line selection.
- **Monospace & Code Formatting**: Using markdown backticks and code blocks (`` `...` `` and ```` ```http ... ``` ````) provides syntax formatting and ensures URLs and HTTP headers display cleanly in monospace without word distortion.
- **Clipboard Integration**: `SelectionFormat::Plain` (default) ensures text copied via `Cmd+C` / `Ctrl+C` is pure, unadorned text.
- **Architectural Cleanup**: Eliminates redundant `request_url_textarea` from `InspectorDisplays`, reducing memory overhead and sync cycles.

---

## User Review Required

> [!NOTE]
> All copy buttons will be completely removed from the **Request Sent** tab.
> Content (URL, Headers, Body) will be rendered via `TextView` with `.selectable(true)`.
> There will be **no blinking cursor** when clicking anywhere in the tab.

---

## Proposed Changes

### `crates/app/src/components/response_inspector.rs`
#### [MODIFY] `crates/app/src/components/response_inspector.rs`
- Import `gpui_component::text::TextView`.
- In `render()` under `ResponseTabKind::Request`:
  - **URL Section**:
    - Remove `btn-copy-sent-url` ("Copy URL") and `btn-copy-sent-curl` ("cURL").
    - Render URL using `TextView::markdown("sent-request-url", format!("`{}`", sent.url)).selectable(true)`.
  - **Headers Sent Section**:
    - Remove `btn-copy-sent-headers` ("Copy All") and the `div`-based list with `copy-sent-hdr-{idx}` icon buttons.
    - Format headers into HTTP header block:
      ```rust
      let headers_text = sent.headers.iter()
          .map(|(k, v)| format!("{}: {}", k, v))
          .collect::<Vec<_>>()
          .join("\n");
      ```
    - Render via `TextView::markdown("sent-headers-view", format!("```http\n{}\n```", headers_text)).selectable(true)`.
  - **Body Sent Section**:
    - Remove `btn-copy-sent-body` ("Copy Body").
    - Keep section title `Body Sent (X bytes):` and render body preview with `TextView::markdown("sent-body-view", format!("```{}\n{}\n```", lang, body_text)).selectable(true)` (or `Editor::new(&self.request_body_editor)`).
  - Remove unused variables (`sent_url_copy`, `curl_cmd`, `headers_all_text`, `preview_copy`).
  - Remove unused `request_url_textarea` field from `ResponseInspector`.

---

### `crates/app/src/app/inspector_displays.rs` & `crates/app/src/app/mod.rs`
#### [MODIFY] `crates/app/src/app/inspector_displays.rs`
- Clean up unused `request_url_textarea` field and its sync logic.
#### [MODIFY] `crates/app/src/app/mod.rs`
- Update `ResponseInspector::new` call signature to match.

---

## Verification Plan

### Automated Tests
```bash
cargo test --package zest --bin zest
cargo clippy --all-targets -- -D warnings
```

### Manual Verification
1. Run application: `cargo run`.
2. Send any request and click on the **Request Sent** tab.
3. Verify that:
   - All copy buttons are gone.
   - Clicking on the URL, Headers, or Body does **NOT** show a blinking cursor (`|`).
   - Dragging the mouse highlights ("bôi đen") text smoothly.
   - Pressing `Cmd+C` / `Ctrl+C` copies the highlighted text to the system clipboard.
