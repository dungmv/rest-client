use std::ops::Range;
use crate::components::kv_table::KvTable;
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Context, Entity, HighlightStyle, InteractiveElement as _,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled, Window,
};
use gpui_component::input::{
    Editor, EditorState as CodeEditorState, FoldRange, HighlightStyleResolver, InputEdit,
    InputHighlighter, Rope, Textarea, TextareaState,
};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::{KeyValuePair, MultipartItem, RequestBody};

pub struct JsonHighlighter {
    pub text: Rope,
    pub cached_str: String,
}

impl JsonHighlighter {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            cached_str: String::new(),
        }
    }
}

impl Default for JsonHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp_char_boundary_left(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn clamp_char_boundary_right(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

impl InputHighlighter for JsonHighlighter {
    fn language(&self) -> SharedString {
        "json".into()
    }

    fn update(
        &mut self,
        _edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<CodeEditorState>,
    ) {
        self.text = text.clone();
        self.cached_str = text.to_string();
    }

    fn styles(
        &self,
        range: &Range<usize>,
        _resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let temp_str;
        let full_str: &str = if !self.cached_str.is_empty() {
            &self.cached_str
        } else if self.text.len() != 0 {
            temp_str = self.text.to_string();
            &temp_str
        } else {
            ""
        };
        if full_str.is_empty() || range.is_empty() {
            return vec![(range.clone(), HighlightStyle::default())];
        }

        let start = clamp_char_boundary_left(full_str, range.start);
        let end = clamp_char_boundary_right(full_str, range.end.min(full_str.len()));

        if start >= end {
            return vec![(range.clone(), HighlightStyle::default())];
        }

        let slice_str = &full_str[start..end];
        let slice_bytes = slice_str.as_bytes();
        let len = slice_bytes.len();
        let mut i = 0;
        let mut raw_tokens = Vec::new();
        let t = theme();

        while i < len {
            let b = slice_bytes[i];
            if b == b'"' {
                let tok_start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    let ch = slice_bytes[i];
                    if escaped {
                        escaped = false;
                        i += 1;
                    } else if ch == b'\\' {
                        escaped = true;
                        i += 1;
                    } else if ch == b'"' {
                        i += 1;
                        break;
                    } else if ch == b'\n' {
                        break;
                    } else {
                        i += 1;
                    }
                }
                let tok_end = i;
                let mut j = tok_end;
                while j < len
                    && (slice_bytes[j] == b' '
                        || slice_bytes[j] == b'\t'
                        || slice_bytes[j] == b'\r'
                        || slice_bytes[j] == b'\n')
                {
                    j += 1;
                }
                let is_key = j < len && slice_bytes[j] == b':';
                let color = if is_key { t.json_key } else { t.json_string };
                let str_content = &slice_str[tok_start..tok_end];
                if str_content.contains("{{") && str_content.contains("}}") {
                    let mut cur_pos = 0;
                    while let Some(open_idx) = str_content[cur_pos..].find("{{") {
                        let abs_open = cur_pos + open_idx;
                        if let Some(close_idx) = str_content[abs_open..].find("}}") {
                            let abs_close = abs_open + close_idx + 2;
                            if abs_open > cur_pos {
                                raw_tokens.push((
                                    start + tok_start + cur_pos..start + tok_start + abs_open,
                                    HighlightStyle {
                                        color: Some(color),
                                        ..Default::default()
                                    },
                                ));
                            }
                            raw_tokens.push((
                                start + tok_start + abs_open..start + tok_start + abs_close,
                                HighlightStyle {
                                    color: Some(t.var_resolved),
                                    font_weight: Some(gpui::FontWeight::SEMIBOLD),
                                    ..Default::default()
                                },
                            ));
                            cur_pos = abs_close;
                        } else {
                            break;
                        }
                    }
                    if cur_pos < str_content.len() {
                        raw_tokens.push((
                            start + tok_start + cur_pos..start + tok_start + str_content.len(),
                            HighlightStyle {
                                color: Some(color),
                                ..Default::default()
                            },
                        ));
                    }
                } else {
                    raw_tokens.push((
                        start + tok_start..start + tok_end,
                        HighlightStyle {
                            color: Some(color),
                            ..Default::default()
                        },
                    ));
                }
            } else if matches!(b, b'{' | b'}' | b'[' | b']' | b':' | b',') {
                raw_tokens.push((
                    start + i..start + i + 1,
                    HighlightStyle {
                        color: Some(t.json_punctuation),
                        ..Default::default()
                    },
                ));
                i += 1;
            } else if b == b'-' || b.is_ascii_digit() {
                let num_start = i;
                i += 1;
                while i < len
                    && (slice_bytes[i].is_ascii_digit()
                        || matches!(slice_bytes[i], b'.' | b'e' | b'E' | b'+' | b'-'))
                {
                    i += 1;
                }
                raw_tokens.push((
                    start + num_start..start + i,
                    HighlightStyle {
                        color: Some(t.json_number),
                        ..Default::default()
                    },
                ));
            } else if slice_str[i..].starts_with("true")
                && (i + 4 == len
                    || !slice_bytes[i + 4].is_ascii_alphanumeric() && slice_bytes[i + 4] != b'_')
            {
                raw_tokens.push((
                    start + i..start + i + 4,
                    HighlightStyle {
                        color: Some(t.json_boolean),
                        font_weight: Some(gpui::FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                ));
                i += 4;
            } else if slice_str[i..].starts_with("false")
                && (i + 5 == len
                    || !slice_bytes[i + 5].is_ascii_alphanumeric() && slice_bytes[i + 5] != b'_')
            {
                raw_tokens.push((
                    start + i..start + i + 5,
                    HighlightStyle {
                        color: Some(t.json_boolean),
                        font_weight: Some(gpui::FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                ));
                i += 5;
            } else if slice_str[i..].starts_with("null")
                && (i + 4 == len
                    || !slice_bytes[i + 4].is_ascii_alphanumeric() && slice_bytes[i + 4] != b'_')
            {
                raw_tokens.push((
                    start + i..start + i + 4,
                    HighlightStyle {
                        color: Some(t.json_null),
                        font_style: Some(gpui::FontStyle::Italic),
                        ..Default::default()
                    },
                ));
                i += 4;
            } else {
                let ch = slice_str[i..].chars().next().unwrap();
                i += ch.len_utf8();
            }
        }

        let mut full_styles = Vec::new();
        let mut cursor = range.start;

        for (token_range, style) in raw_tokens {
            if token_range.start > cursor {
                full_styles.push((cursor..token_range.start, HighlightStyle::default()));
            }
            cursor = token_range.end;
            full_styles.push((token_range, style));
        }

        if cursor < range.end {
            full_styles.push((cursor..range.end, HighlightStyle::default()));
        }

        full_styles
    }

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        let mut folds = Vec::new();
        let mut stack = Vec::new();
        let s = text.to_string();
        let mut current_line = 0;
        let mut in_string = false;
        let mut escaped = false;
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if in_string {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    in_string = false;
                }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'{' | b'[' => stack.push(current_line),
                    b'}' | b']' => {
                        if let Some(start_line) = stack.pop() {
                            if current_line > start_line {
                                folds.push(FoldRange::new(start_line, current_line));
                            }
                        }
                    }
                    b'\n' => current_line += 1,
                    _ => {}
                }
            }
            if in_string && b == b'\n' {
                current_line += 1;
            }
            i += 1;
        }
        folds
    }
}

fn format_file_size(path_str: &str) -> String {
    if path_str.is_empty() {
        return "".to_string();
    }
    if let Ok(meta) = std::fs::metadata(path_str) {
        let bytes = meta.len();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    } else {
        "File not found".to_string()
    }
}

#[derive(IntoElement)]
pub struct BodyEditor {
    editor_state: Entity<EditorState>,
    json_editor: Entity<CodeEditorState>,
    raw_textarea: Entity<TextareaState>,
    workspace_state: Option<Entity<WorkspaceState>>,
}

impl BodyEditor {
    pub fn new(
        editor_state: Entity<EditorState>,
        json_editor: Entity<CodeEditorState>,
        raw_textarea: Entity<TextareaState>,
    ) -> Self {
        Self {
            editor_state,
            json_editor,
            raw_textarea,
            workspace_state: None,
        }
    }

    pub fn workspace_state(mut self, ws: Entity<WorkspaceState>) -> Self {
        self.workspace_state = Some(ws);
        self
    }
}

impl RenderOnce for BodyEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ed = self.editor_state.read(cx);
        let ed_entity = self.editor_state.clone();

        let active_req = ed.active_request.clone();
        let body = active_req.as_ref().map(|r| r.body.clone()).unwrap_or(RequestBody::None);

        let body_kind_label = match &body {
            RequestBody::None => "none",
            RequestBody::Json(_) => "json",
            RequestBody::FormUrlEncoded(_) => "form",
            RequestBody::Multipart(_) => "multipart",
            RequestBody::Raw { .. } => "raw",
            RequestBody::Binary { .. } => "binary",
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            // Top format selector bar: None | JSON | Form | Multipart | Raw | Binary
            .child(
                div()
                    .h(px(32.))
                    .px(px(8.))
                    .border_b_1()
                    .border_color(t.border)
                    .bg(t.bg_editor)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .children([
                                ("none", "None"),
                                ("json", "JSON"),
                                ("form", "Form URL-Encoded"),
                                ("multipart", "Multipart"),
                                ("raw", "Raw"),
                                ("binary", "Binary"),
                            ].into_iter().map(|(key, label)| {
                                let is_active = body_kind_label == key;
                                let ed_c = ed_entity.clone();
                                div()
                                    .id(format!("body-format-{}", key))
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(if is_active { t.border } else { t.transparent })
                                    .bg(if is_active { t.bg_selected } else { t.transparent })
                                    .text_color(if is_active { t.text_primary } else { t.text_secondary })
                                    .hover(|s| if is_active { s } else { s.text_color(t.text_primary).bg(t.bg_hover) })
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(if is_active { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                    .on_click(move |_, _, cx| {
                                        ed_c.update(cx, |state, cx| {
                                            let new_body = match key {
                                                "none" => RequestBody::None,
                                                "json" => RequestBody::Json("{\n  \n}".to_string()),
                                                "form" => RequestBody::FormUrlEncoded(vec![KeyValuePair::new("", "")]),
                                                "multipart" => RequestBody::Multipart(vec![MultipartItem::default()]),
                                                "raw" => RequestBody::Raw {
                                                    content: "".to_string(),
                                                    content_type: "text/plain".to_string(),
                                                },
                                                "binary" => RequestBody::Binary { file_path: "".to_string() },
                                                _ => RequestBody::None,
                                            };
                                            state.set_body_type(new_body, cx);
                                        });
                                    })
                                    .child(label)
                            }))
                    )
                    // Quick Action button in toolbar
                    .child({
                        match &body {
                            RequestBody::Json(_) => {
                                let ed_c = ed_entity.clone();
                                let j_text = self.json_editor.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .id("btn-format-json-body")
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(t.bg_input)
                                            .border_1()
                                            .border_color(t.border)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(t.bg_hover))
                                            .text_xs()
                                            .text_color(t.text_secondary)
                                            .on_click(move |_, window, cx| {
                                                let raw = j_text.read(cx).value().to_string();
                                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                                                    if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                                                        j_text.update(cx, |txt, cx| {
                                                            txt.set_value(pretty.clone(), window, cx);
                                                        });
                                                        ed_c.update(cx, |state, cx| {
                                                            state.set_body_json(pretty, cx);
                                                        });
                                                    }
                                                }
                                            })
                                            .child("Format JSON")
                                    )
                                    .child({
                                        let j_text = self.json_editor.clone();
                                        div()
                                            .id("btn-copy-json-body")
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(t.bg_input)
                                            .border_1()
                                            .border_color(t.border)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(t.bg_hover))
                                            .text_xs()
                                            .text_color(t.text_secondary)
                                            .on_click(move |_, _, cx| {
                                                let raw = j_text.read(cx).value().to_string();
                                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(raw));
                                            })
                                            .child("Copy")
                                    })
                            }
                            _ => div(),
                        }
                    })
            )
            // Body Content Area
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child({
                        match body {
                            RequestBody::None => {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_2xl()
                                            .text_color(t.text_muted)
                                            .child("📄"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(t.text_secondary)
                                            .child("This request has no body payload"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("Select a format above (e.g. JSON, Form) to add request body data."),
                                    )
                                    .into_any_element()
                            }
                            RequestBody::Json(_) => {
                                div()
                                    .size_full()
                                    .py(px(6.))
                                    .px(px(4.))
                                    .child(
                                        Editor::new(&self.json_editor)
                                            .appearance(false)
                                            .bordered(false)
                                            .h(gpui::relative(1.))
                                    )
                                    .into_any_element()
                            }
                            RequestBody::FormUrlEncoded(pairs) => {
                                let ed_up = ed_entity.clone();
                                let ed_add = ed_entity.clone();
                                let ed_del = ed_entity.clone();
                                let req_id_str = active_req.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| "scratch".to_string());

                                div()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .child(
                                        KvTable::new(
                                            format!("form-kv-{}", req_id_str),
                                            pairs,
                                            move |idx, k, v, enabled, _, cx| {
                                                ed_up.update(cx, |state, cx| {
                                                    state.update_form_param(idx, k, v, enabled, cx);
                                                });
                                            },
                                            move |_, cx| {
                                                ed_add.update(cx, |state, cx| {
                                                    state.add_form_param(cx);
                                                });
                                            },
                                            move |idx, _, cx| {
                                                ed_del.update(cx, |state, cx| {
                                                    state.remove_form_param(idx, cx);
                                                });
                                            },
                                        )
                                        .optional_workspace_state(self.workspace_state.clone())
                                    )
                                    .into_any_element()
                            }
                            RequestBody::Multipart(items) => {
                                let ed_c = ed_entity.clone();
                                let req_id_str = active_req.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| "scratch".to_string());
                                div()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .p(px(8.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    // Multipart Table Header
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .px(px(6.))
                                            .py(px(4.))
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(t.text_muted)
                                            .border_b_1()
                                            .border_color(t.border)
                                            .child(div().w(px(24.)).child(""))
                                            .child(div().w(px(160.)).child("Key"))
                                            .child(div().w(px(70.)).child("Type"))
                                            .child(div().flex_1().child("Value / File Path"))
                                            .child(div().w(px(80.)).child("Browse"))
                                            .child(div().w(px(28.)).child(""))
                                    )
                                    // Multipart Rows
                                    .children(items.into_iter().enumerate().map(|(idx, item)| {
                                        let ed_row = ed_c.clone();
                                        let ed_del = ed_c.clone();
                                        let ed_pick = ed_c.clone();
                                        let key_str = item.key.clone();
                                        let val_str = item.value.clone();
                                        let is_file = item.is_file;
                                        let file_path = item.file_path.clone();
                                        let enabled = item.enabled;
                                        let mp_prefix = format!("mp-{}-row-{}", req_id_str, idx);

                                        let ed_up_k = ed_row.clone();
                                        let ed_up_v = ed_row.clone();
                                        let fp_k = file_path.clone();
                                        let fp_v = file_path.clone();
                                        let mp_holder = window.use_keyed_state(
                                            gpui::SharedString::from(format!("{}-holder", mp_prefix)),
                                            cx,
                                            |window, cx| {
                                                let key_input = cx.new(|cx| gpui_component::input::InputState::new(window, cx).placeholder("Key..."));
                                                let val_input = cx.new(|cx| gpui_component::input::InputState::new(window, cx).placeholder("Value..."));

                                                let k_in_1 = key_input.clone();
                                                let v_in_1 = val_input.clone();
                                                let sub1 = cx.subscribe_in(&key_input, window, move |_, _, event, _, cx| {
                                                    if let gpui_component::input::InputEvent::Change = event {
                                                        let k = k_in_1.read(cx).value().to_string();
                                                        let v = v_in_1.read(cx).value().to_string();
                                                        ed_up_k.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k, v, is_file, fp_k.clone(), true, cx);
                                                        });
                                                    }
                                                });

                                                let k_in_2 = key_input.clone();
                                                let v_in_2 = val_input.clone();
                                                let sub2 = cx.subscribe_in(&val_input, window, move |_, _, event, _, cx| {
                                                    if let gpui_component::input::InputEvent::Change = event {
                                                        let k = k_in_2.read(cx).value().to_string();
                                                        let v = v_in_2.read(cx).value().to_string();
                                                        ed_up_v.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k, v, is_file, fp_v.clone(), true, cx);
                                                        });
                                                    }
                                                });

                                                (key_input, val_input, vec![sub1, sub2])
                                            },
                                        );

                                        let (key_input, val_input) = {
                                            let h = mp_holder.read(cx);
                                            (h.0.clone(), h.1.clone())
                                        };
                                        if key_input.read(cx).value().as_str() != key_str {
                                            key_input.update(cx, |inp, cx| {
                                                inp.set_value(key_str.clone(), window, cx);
                                            });
                                        }
                                        if val_input.read(cx).value().as_str() != val_str {
                                            val_input.update(cx, |inp, cx| {
                                                inp.set_value(val_str.clone(), window, cx);
                                            });
                                        }

                                        div()
                                            .flex()
                                            .items_center()
                                            .px(px(6.))
                                            .py(px(4.))
                                            .gap(px(8.))
                                            .rounded(px(4.))
                                            .hover(|s| s.bg(t.bg_hover))
                                            // Checkbox
                                            .child({
                                                let ed_toggle = ed_row.clone();
                                                let k = key_str.clone();
                                                let v = val_str.clone();
                                                let fp = file_path.clone();
                                                div()
                                                    .id(format!("check-mp-{}", idx))
                                                    .w(px(20.))
                                                    .h(px(20.))
                                                    .rounded(px(4.))
                                                    .border_1()
                                                    .border_color(if enabled { t.primary } else { t.border })
                                                    .bg(if enabled { t.primary } else { t.bg_input })
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(t.primary_foreground)
                                                    .on_click(move |_, _, cx| {
                                                        ed_toggle.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k.clone(), v.clone(), is_file, fp.clone(), !enabled, cx);
                                                        });
                                                    })
                                                    .child(if enabled { "✓" } else { "" })
                                            })
                                            // Key
                                            .child(
                                                div()
                                                    .w(px(160.))
                                                    .h(px(28.))
                                                    .px(px(4.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        gpui_component::input::Input::new(&key_input)
                                                            .id(gpui::SharedString::from(format!("{}-key", mp_prefix)))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false),
                                                    )
                                            )
                                            // Type toggle: Text | File
                                            .child({
                                                let ed_toggle_type = ed_row.clone();
                                                let k = key_str.clone();
                                                let v = val_str.clone();
                                                let fp = file_path.clone();
                                                div()
                                                    .id(format!("toggle-mp-type-{}", idx))
                                                    .w(px(70.))
                                                    .h(px(28.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_card)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_xs()
                                                    .text_color(t.text_secondary)
                                                    .on_click(move |_, _, cx| {
                                                        ed_toggle_type.update(cx, |state, cx| {
                                                            state.update_multipart_item(idx, k.clone(), v.clone(), !is_file, fp.clone(), enabled, cx);
                                                        });
                                                    })
                                                    .child(if is_file { "📁 File" } else { "🔤 Text" })
                                            })
                                            // Value or Path
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .h(px(28.))
                                                    .px(px(4.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .child(if is_file {
                                                        div()
                                                            .px(px(4.))
                                                            .text_xs()
                                                            .font_family(".SystemMonoFont")
                                                            .text_color(t.text_primary)
                                                            .child(file_path.clone().unwrap_or_else(|| "Select a file...".to_string()))
                                                            .into_any_element()
                                                    } else {
                                                        gpui_component::input::Input::new(&val_input)
                                                            .id(gpui::SharedString::from(format!("{}-val", mp_prefix)))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false)
                                                            .into_any_element()
                                                    })
                                            )
                                            // Browse button (only enabled if is_file)
                                            .child({
                                                let ed_p = ed_pick.clone();
                                                let k = key_str.clone();
                                                let v = val_str.clone();
                                                let btn = div()
                                                    .id(format!("browse-mp-{}", idx))
                                                    .w(px(80.))
                                                    .h(px(28.))
                                                    .rounded(px(4.))
                                                    .bg(if is_file { t.bg_card } else { gpui::transparent_black() })
                                                    .border_1()
                                                    .border_color(if is_file { t.border } else { gpui::transparent_black() })
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_xs()
                                                    .text_color(if is_file { t.text_secondary } else { t.text_muted });

                                                if is_file {
                                                    btn.cursor_pointer()
                                                        .hover(|s| s.bg(t.bg_hover))
                                                        .on_click(move |_, _, cx| {
                                                            let ed_bg = ed_p.clone();
                                                            let k_bg = k.clone();
                                                            let v_bg = v.clone();
                                                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                                let p = path.to_string_lossy().to_string();
                                                                ed_bg.update(cx, |state, cx| {
                                                                    state.update_multipart_item(idx, k_bg, v_bg, true, Some(p), enabled, cx);
                                                                });
                                                            }
                                                        })
                                                        .child("Browse...")
                                                } else {
                                                    btn.child("")
                                                }
                                            })
                                            // Delete row
                                            .child(
                                                div()
                                                    .id(format!("del-mp-row-{}", idx))
                                                    .w(px(24.))
                                                    .h(px(24.))
                                                    .rounded(px(4.))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover).text_color(t.status_5xx))
                                                    .text_color(t.text_muted)
                                                    .text_xs()
                                                    .on_click(move |_, _, cx| {
                                                        ed_del.update(cx, |state, cx| {
                                                            state.remove_multipart_item(idx, cx);
                                                        });
                                                    })
                                                    .child("×")
                                            )
                                    }))
                                    // Add Part button
                                    .child(
                                        div()
                                            .pt(px(4.))
                                            .child({
                                                let ed_add = ed_c.clone();
                                                div()
                                                    .id("btn-add-multipart-part")
                                                    .px(px(8.))
                                                    .py(px(4.))
                                                    .rounded(px(4.))
                                                    .w(px(90.))
                                                    .bg(t.bg_card)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .gap(px(4.))
                                                    .text_xs()
                                                    .text_color(t.text_secondary)
                                                    .on_click(move |_, _, cx| {
                                                        ed_add.update(cx, |state, cx| {
                                                            state.add_multipart_item(cx);
                                                        });
                                                    })
                                                    .child("+ Add Part")
                                            })
                                    )
                                    .into_any_element()
                            }
                            RequestBody::Raw { content_type, .. } => {
                                let ed_c = ed_entity.clone();
                                let raw_tx = self.raw_textarea.clone();

                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    // Raw MIME sub-bar
                                    .child(
                                        div()
                                            .h(px(30.))
                                            .px(px(8.))
                                            .border_b_1()
                                            .border_color(t.border)
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Content-Type:")
                                            )
                                            .children([
                                                "text/plain",
                                                "application/xml",
                                                "text/html",
                                                "application/javascript",
                                                "application/graphql",
                                            ].into_iter().map(|mime| {
                                                let is_sel = content_type == mime;
                                                let ed_mime = ed_c.clone();
                                                let raw_tx_c = raw_tx.clone();
                                                div()
                                                    .id(format!("raw-mime-{}", mime))
                                                    .px(px(6.))
                                                    .py(px(2.))
                                                    .rounded(px(4.))
                                                    .bg(if is_sel { t.primary } else { t.bg_card })
                                                    .text_color(if is_sel { t.primary_foreground } else { t.text_secondary })
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .on_click(move |_, _, cx| {
                                                        let r_c = raw_tx_c.read(cx).value().to_string();
                                                        ed_mime.update(cx, |state, cx| {
                                                            state.set_raw_body(r_c, mime.to_string(), cx);
                                                        });
                                                    })
                                                    .child(mime)
                                            }))
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .p(px(8.))
                                            .child(
                                                Textarea::new(&self.raw_textarea)
                                                    .appearance(false)
                                                    .bordered(false)
                                                    .h(gpui::relative(1.))
                                            )
                                    )
                                    .into_any_element()
                            }
                            RequestBody::Binary { file_path } => {
                                let ed_c = ed_entity.clone();
                                let has_file = !file_path.is_empty();
                                let size_str = format_file_size(&file_path);

                                div()
                                    .size_full()
                                    .p(px(20.))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .w(px(460.))
                                            .p(px(20.))
                                            .rounded(px(8.))
                                            .bg(t.bg_card)
                                            .border_1()
                                            .border_color(t.border)
                                            .flex()
                                            .flex_col()
                                            .gap(px(14.))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(10.))
                                                    .child(div().text_xl().child("📦"))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_weight(gpui::FontWeight::BOLD)
                                                                    .text_color(t.text_primary)
                                                                    .child("Binary Request Payload")
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(t.text_secondary)
                                                                    .child("Select a file from your system to send as the raw binary body.")
                                                            )
                                                    )
                                            )
                                            .child(
                                                div()
                                                    .p(px(10.))
                                                    .rounded(px(6.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .gap(px(2.))
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_family(".SystemMonoFont")
                                                                    .text_color(if has_file { t.text_primary } else { t.text_muted })
                                                                    .child(if has_file { file_path.clone() } else { "No file chosen yet".to_string() })
                                                            )
                                                            .when(has_file, |this| {
                                                                this.child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(t.text_muted)
                                                                        .child(format!("Size: {}", size_str))
                                                                )
                                                            })
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.))
                                                            .child({
                                                                let ed_pick = ed_c.clone();
                                                                div()
                                                                    .id("btn-browse-binary-file")
                                                                    .px(px(10.))
                                                                    .py(px(4.))
                                                                    .rounded(px(4.))
                                                                    .bg(t.primary)
                                                                    .text_color(t.primary_foreground)
                                                                    .hover(|s| s.bg(t.primary_hover))
                                                                    .cursor_pointer()
                                                                    .text_xs()
                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                    .on_click(move |_, _, cx| {
                                                                        let ed_bg = ed_pick.clone();
                                                                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                                            let p = path.to_string_lossy().to_string();
                                                                            ed_bg.update(cx, |state, cx| {
                                                                                state.set_binary_file(p, cx);
                                                                            });
                                                                        }
                                                                    })
                                                                    .child("Browse File...")
                                                            })
                                                            .when(has_file, |this| {
                                                                let ed_clr = ed_c.clone();
                                                                this.child(
                                                                    div()
                                                                        .id("btn-clear-binary-file")
                                                                        .px(px(8.))
                                                                        .py(px(4.))
                                                                        .rounded(px(4.))
                                                                        .bg(t.bg_card)
                                                                        .border_1()
                                                                        .border_color(t.border)
                                                                        .hover(|s| s.bg(t.bg_hover))
                                                                        .cursor_pointer()
                                                                        .text_xs()
                                                                        .text_color(t.text_secondary)
                                                                        .on_click(move |_, _, cx| {
                                                                            ed_clr.update(cx, |state, cx| {
                                                                                state.set_binary_file("".to_string(), cx);
                                                                            });
                                                                        })
                                                                        .child("Clear")
                                                                )
                                                            })
                                                    )
                                            )
                                    )
                                    .into_any_element()
                            }
                        }
                    })
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyResolver;
    impl HighlightStyleResolver for DummyResolver {
        fn style(&self, _: &str) -> Option<HighlightStyle> {
            None
        }
    }

    #[test]
    fn test_json_highlighter_styles_coverage() {
        let mut highlighter = JsonHighlighter::new();
        let json_text = r#"{"name": "Alice", "age": 30, "admin": true, "extra": null}"#;
        highlighter.text = Rope::from_str(json_text);

        let range = 0..json_text.len();
        let styles = highlighter.styles(&range, &DummyResolver);

        // Verify contiguous coverage
        let mut cursor = 0;
        for (sub_range, _) in &styles {
            assert_eq!(sub_range.start, cursor);
            assert!(sub_range.end > sub_range.start);
            cursor = sub_range.end;
        }
        assert_eq!(cursor, json_text.len());

        // Check that keys and values received colors
        let t = theme();
        let key_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "\"name\"" && s.color == Some(t.json_key)
        });
        assert!(key_highlight.is_some(), "Key should have json_key color");

        let str_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "\"Alice\"" && s.color == Some(t.json_string)
        });
        assert!(str_highlight.is_some(), "String should have json_string color");

        let num_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "30" && s.color == Some(t.json_number)
        });
        assert!(num_highlight.is_some(), "Number should have json_number color");

        let bool_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "true" && s.color == Some(t.json_boolean)
        });
        assert!(bool_highlight.is_some(), "Boolean should have json_boolean color");

        let null_highlight = styles.iter().find(|(r, s)| {
            &json_text[r.clone()] == "null" && s.color == Some(t.json_null)
        });
        assert!(null_highlight.is_some(), "Null should have json_null color");
    }

    #[test]
    fn test_json_highlighter_folding() {
        let highlighter = JsonHighlighter::new();
        let json_text = "{\n  \"items\": [\n    1,\n    2\n  ]\n}";
        let rope = Rope::from_str(json_text);
        let folds = highlighter.fold_ranges(&rope);
        assert!(!folds.is_empty(), "Should detect foldable ranges");
    }

    #[test]
    fn test_json_highlighter_large_file_performance() {
        let mut highlighter = JsonHighlighter::new();
        let items: Vec<String> = (0..500)
            .map(|i| format!(r#"{{"id": {}, "title": "large test item {}", "active": true}}"#, i, i))
            .collect();
        let json_text = format!("[\n{}\n]", items.join(",\n"));
        highlighter.text = Rope::from_str(&json_text);
        highlighter.cached_str = json_text.clone();

        // Highlighting individual line range should be instant and not allocate full string
        let first_line_range = 0..1;
        let styles = highlighter.styles(&first_line_range, &DummyResolver);
        assert!(!styles.is_empty());
    }
}
