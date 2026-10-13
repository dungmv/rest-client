use crate::components::kv_table::KvTable;
use crate::state::EditorState;
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Textarea, TextareaState};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::{KeyValuePair, MultipartItem, RequestBody};

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
    json_textarea: Entity<TextareaState>,
    raw_textarea: Entity<TextareaState>,
}

impl BodyEditor {
    pub fn new(
        editor_state: Entity<EditorState>,
        json_textarea: Entity<TextareaState>,
        raw_textarea: Entity<TextareaState>,
    ) -> Self {
        Self {
            editor_state,
            json_textarea,
            raw_textarea,
        }
    }
}

impl RenderOnce for BodyEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
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
                    .bg(t.bg_card)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
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
                                    .bg(if is_active { t.primary } else { gpui::transparent_black() })
                                    .text_color(if is_active { t.primary_foreground } else { t.text_secondary })
                                    .hover(|s| if is_active { s } else { s.bg(t.bg_hover) })
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
                                let j_text = self.json_textarea.clone();
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
                                        let j_text = self.json_textarea.clone();
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
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .italic()
                                            .text_color(t.text_muted)
                                            .child("No Body"),
                                    )
                                    .into_any_element()
                            }
                            RequestBody::Json(_) => {
                                div()
                                    .size_full()
                                    .p(px(8.))
                                    .child(Textarea::new(&self.json_textarea))
                                    .into_any_element()
                            }
                            RequestBody::FormUrlEncoded(pairs) => {
                                let ed_up = ed_entity.clone();
                                let ed_add = ed_entity.clone();
                                let ed_del = ed_entity.clone();

                                div()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .child(
                                        KvTable::new(
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
                                    )
                                    .into_any_element()
                            }
                            RequestBody::Multipart(items) => {
                                let ed_c = ed_entity.clone();
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
                                                    .px(px(6.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child(if key_str.is_empty() { "Key...".to_string() } else { key_str.clone() })
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
                                                    .px(px(6.))
                                                    .rounded(px(4.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .flex()
                                                    .items_center()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child(if is_file {
                                                        file_path.clone().unwrap_or_else(|| "Select a file...".to_string())
                                                    } else if val_str.is_empty() {
                                                        "Value...".to_string()
                                                    } else {
                                                        val_str.clone()
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
                                let raw_content = self.raw_textarea.read(cx).value().to_string();

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
                                                let r_c = raw_content.clone();
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
                                                        ed_mime.update(cx, |state, cx| {
                                                            state.set_raw_body(r_c.clone(), mime.to_string(), cx);
                                                        });
                                                    })
                                                    .child(mime)
                                            }))
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .p(px(8.))
                                            .child(Textarea::new(&self.raw_textarea))
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
