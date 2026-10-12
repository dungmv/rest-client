use crate::theme::theme;
use gpui::{
    div, px, App, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::RequestBody;
use std::rc::Rc;

#[derive(IntoElement)]
pub struct BodyEditor {
    body: RequestBody,
    on_change_json: Rc<dyn Fn(String, &mut Window, &mut App)>,
}

impl BodyEditor {
    pub fn new(body: RequestBody, on_change_json: impl Fn(String, &mut Window, &mut App) + 'static) -> Self {
        Self {
            body,
            on_change_json: Rc::new(on_change_json),
        }
    }
}

impl RenderOnce for BodyEditor {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();

        match &self.body {
            RequestBody::None => {
                // Empty state matching Yaak screenshot: centered italic/muted "No Body"
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
            }
            RequestBody::Json(raw_json) => {
                let lines: Vec<String> = if raw_json.is_empty() {
                    vec!["{}".to_string()]
                } else {
                    raw_json.lines().map(|s| s.to_string()).collect()
                };

                let on_change = self.on_change_json.clone();
                let current_json = raw_json.clone();

                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    // Top toolbar for JSON editor
                    .child(
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .border_b_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("JSON"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    // Format JSON button
                                    .child({
                                        let on_change_c = on_change.clone();
                                        let json_c = current_json.clone();
                                        div()
                                            .id("btn-format-json")
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(t.bg_card)
                                            .border_1()
                                            .border_color(t.border)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(t.bg_hover))
                                            .text_xs()
                                            .text_color(t.text_secondary)
                                            .on_click(move |_, window, cx| {
                                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_c) {
                                                    if let Ok(formatted) = serde_json::to_string_pretty(&val) {
                                                        on_change_c(formatted, window, cx);
                                                    }
                                                }
                                            })
                                            .child("Format")
                                    })
                                    // Copy JSON button
                                    .child({
                                        let json_c = current_json.clone();
                                        div()
                                            .id("btn-copy-json")
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(t.bg_card)
                                            .border_1()
                                            .border_color(t.border)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(t.bg_hover))
                                            .text_xs()
                                            .text_color(t.text_secondary)
                                            .on_click(move |_, _, cx| {
                                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(json_c.clone()));
                                            })
                                            .child("Copy")
                                    }),
                            ),
                    )
                    // Editor content with line numbers
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .p(px(8.))
                            .flex()
                            .gap(px(12.))
                            // Line numbers gutter
                            .child(
                                div()
                                    .w(px(28.))
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .text_xs()
                                    .font_family(".SystemMonoFont")
                                    .text_color(t.text_muted)
                                    .children((1..=lines.len()).map(|num| {
                                        div().h(px(20.)).child(format!("{}", num))
                                    })),
                            )
                            // Code text lines
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .text_xs()
                                    .font_family(".SystemMonoFont")
                                    .text_color(t.text_primary)
                                    .children(lines.into_iter().map(|line| {
                                        div()
                                            .h(px(20.))
                                            .child(if line.is_empty() { " ".to_string() } else { line })
                                    })),
                            ),
                    )
            }
            RequestBody::Raw { content, content_type } => {
                div()
                    .size_full()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child(format!("Type: {}", content_type)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .p(px(8.))
                            .bg(t.bg_input)
                            .rounded(px(6.))
                            .font_family(".SystemMonoFont")
                            .text_xs()
                            .text_color(t.text_primary)
                            .child(content.clone()),
                    )
            }
            _ => {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(t.text_muted)
                            .child("Form / Multipart / Binary Body"),
                    )
            }
        }
    }
}
