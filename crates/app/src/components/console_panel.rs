use crate::state::{ConsoleFilter, ResponseState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::{ConsoleLogLevel, ConsoleLogSource};

#[derive(IntoElement)]
pub struct ConsolePanel {
    response_state: Entity<ResponseState>,
}

impl ConsolePanel {
    pub fn new(response_state: Entity<ResponseState>) -> Self {
        Self { response_state }
    }
}

impl RenderOnce for ConsolePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let resp_s = self.response_state.read(cx);
        let all_logs = resp_s.console_logs.clone();
        let current_filter = resp_s.console_filter;
        let rs = self.response_state.clone();

        let total_count = all_logs.len();
        let pre_count = all_logs
            .iter()
            .filter(|l| l.source == ConsoleLogSource::PreRequest)
            .count();
        let post_count = all_logs
            .iter()
            .filter(|l| l.source == ConsoleLogSource::PostResponse)
            .count();
        let error_count = all_logs
            .iter()
            .filter(|l| l.level == ConsoleLogLevel::Error)
            .count();

        let filtered_logs: Vec<_> = all_logs
            .into_iter()
            .filter(|l| match current_filter {
                ConsoleFilter::All => true,
                ConsoleFilter::PreRequest => l.source == ConsoleLogSource::PreRequest,
                ConsoleFilter::PostResponse => l.source == ConsoleLogSource::PostResponse,
                ConsoleFilter::Errors => l.level == ConsoleLogLevel::Error,
            })
            .collect();

        // Prepare copy text for all logs
        let all_logs_text = filtered_logs
            .iter()
            .map(|l| {
                let src = match l.source {
                    ConsoleLogSource::PreRequest => "Pre-request",
                    ConsoleLogSource::PostResponse => "Post-response",
                    ConsoleLogSource::System => "System",
                };
                let lvl = match l.level {
                    ConsoleLogLevel::Log => "LOG",
                    ConsoleLogLevel::Info => "INFO",
                    ConsoleLogLevel::Warn => "WARN",
                    ConsoleLogLevel::Error => "ERROR",
                };
                format!("[{}] [{}] [{}] {}", l.timestamp, src, lvl, l.message)
            })
            .collect::<Vec<_>>()
            .join("\n");

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.bg_response)
            // Header / Filter toolbar
            .child(
                div()
                    .h(px(38.))
                    .px(px(12.))
                    .border_b_1()
                    .border_color(t.border)
                    .bg(t.bg_editor)
                    .flex()
                    .items_center()
                    .justify_between()
                    // Filter tabs on the left
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            // All filter
                            .child({
                                let rs_c = rs.clone();
                                let is_active = current_filter == ConsoleFilter::All;
                                div()
                                    .id("filter-all")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(if is_active { t.bg_hover } else { t.transparent })
                                    .border_1()
                                    .border_color(if is_active { t.primary } else { t.transparent })
                                    .text_xs()
                                    .font_weight(if is_active {
                                        gpui::FontWeight::SEMIBOLD
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_active {
                                        t.text_primary
                                    } else {
                                        t.text_secondary
                                    })
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        rs_c.update(cx, |s, cx| {
                                            s.set_console_filter(ConsoleFilter::All, cx);
                                        });
                                    })
                                    .child(format!("All ({})", total_count))
                            })
                            // Pre-request filter
                            .child({
                                let rs_c = rs.clone();
                                let is_active = current_filter == ConsoleFilter::PreRequest;
                                div()
                                    .id("filter-pre-request")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(if is_active { t.bg_hover } else { t.transparent })
                                    .border_1()
                                    .border_color(if is_active { t.primary } else { t.transparent })
                                    .text_xs()
                                    .font_weight(if is_active {
                                        gpui::FontWeight::SEMIBOLD
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_active {
                                        t.text_primary
                                    } else {
                                        t.text_secondary
                                    })
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        rs_c.update(cx, |s, cx| {
                                            s.set_console_filter(ConsoleFilter::PreRequest, cx);
                                        });
                                    })
                                    .child(format!("Pre-request ({})", pre_count))
                            })
                            // Post-response filter
                            .child({
                                let rs_c = rs.clone();
                                let is_active = current_filter == ConsoleFilter::PostResponse;
                                div()
                                    .id("filter-post-response")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(if is_active { t.bg_hover } else { t.transparent })
                                    .border_1()
                                    .border_color(if is_active { t.primary } else { t.transparent })
                                    .text_xs()
                                    .font_weight(if is_active {
                                        gpui::FontWeight::SEMIBOLD
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_active {
                                        t.text_primary
                                    } else {
                                        t.text_secondary
                                    })
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        rs_c.update(cx, |s, cx| {
                                            s.set_console_filter(ConsoleFilter::PostResponse, cx);
                                        });
                                    })
                                    .child(format!("Post-response ({})", post_count))
                            })
                            // Errors filter
                            .when(error_count > 0, |this| {
                                let rs_c = rs.clone();
                                let is_active = current_filter == ConsoleFilter::Errors;
                                this.child(
                                    div()
                                        .id("filter-errors")
                                        .cursor_pointer()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(if is_active {
                                            gpui::Hsla {
                                                a: 0.18,
                                                ..t.method_delete
                                            }
                                        } else {
                                            t.transparent
                                        })
                                        .border_1()
                                        .border_color(if is_active {
                                            t.method_delete
                                        } else {
                                            t.transparent
                                        })
                                        .text_xs()
                                        .font_weight(if is_active {
                                            gpui::FontWeight::SEMIBOLD
                                        } else {
                                            gpui::FontWeight::NORMAL
                                        })
                                        .text_color(t.method_delete)
                                        .hover(|s| {
                                            s.bg(gpui::Hsla {
                                                a: 0.15,
                                                ..t.method_delete
                                            })
                                        })
                                        .on_click(move |_, _, cx| {
                                            rs_c.update(cx, |s, cx| {
                                                s.set_console_filter(ConsoleFilter::Errors, cx);
                                            });
                                        })
                                        .child(format!("Errors ({})", error_count)),
                                )
                            }),
                    )
                    // Action buttons on the right
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            // Copy All
                            .when(!filtered_logs.is_empty(), |this| {
                                let copy_str = all_logs_text.clone();
                                this.child(
                                    div()
                                        .id("btn-copy-console-logs")
                                        .cursor_pointer()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(t.bg_card)
                                        .border_1()
                                        .border_color(t.border)
                                        .text_xs()
                                        .text_color(t.text_secondary)
                                        .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                        .on_click(move |_, _, cx| {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                                copy_str.clone(),
                                            ));
                                        })
                                        .child("📋 Copy All"),
                                )
                            })
                            // Clear
                            .when(total_count > 0, |this| {
                                let rs_c = rs.clone();
                                this.child(
                                    div()
                                        .id("btn-clear-console-logs")
                                        .cursor_pointer()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(t.bg_card)
                                        .border_1()
                                        .border_color(t.border)
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .hover(|s| {
                                            s.bg(t.bg_hover)
                                                .text_color(t.method_delete)
                                                .border_color(t.method_delete)
                                        })
                                        .on_click(move |_, _, cx| {
                                            rs_c.update(cx, |s, cx| {
                                                s.clear_console_logs(cx);
                                            });
                                        })
                                        .child("🗑 Clear"),
                                )
                            }),
                    ),
            )
            // Log items list or Empty State
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .p(px(12.))
                    .child({
                        if total_count == 0 {
                            // Empty state: no logs at all
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py(px(40.))
                                .gap(px(10.))
                                .child(
                                    div()
                                        .text_3xl()
                                        .text_color(t.text_muted)
                                        .child("📟"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(t.text_primary)
                                        .child("No Console Output"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_secondary)
                                        .child("Output from console.log(), console.info(), console.warn(), or console.error() will appear here."),
                                )
                                .child(
                                    div()
                                        .mt(px(8.))
                                        .p(px(12.))
                                        .rounded(px(6.))
                                        .bg(t.bg_card)
                                        .border_1()
                                        .border_color(t.border)
                                        .font_family(".SystemMonoFont")
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.))
                                        .child("// In Pre-request Script:")
                                        .child("console.log(\"Requesting endpoint with auth token\");")
                                        .child("")
                                        .child("// In Tests / Post-response Script:")
                                        .child("console.log(\"Response status:\", client.response.status);")
                                        .child("console.log(\"Data:\", client.response.json());"),
                                )
                        } else if filtered_logs.is_empty() {
                            // Filter returned no results
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py(px(40.))
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(t.text_secondary)
                                        .child("No logs match the selected filter."),
                                )
                        } else {
                            // Render list of logs
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .children(filtered_logs.into_iter().enumerate().map(|(idx, entry)| {
                                    let is_err = entry.level == ConsoleLogLevel::Error;
                                    let is_warn = entry.level == ConsoleLogLevel::Warn;
                                    let is_info = entry.level == ConsoleLogLevel::Info;

                                    let msg_copy = entry.message.clone();

                                    // Source badge colors
                                    let (src_label, src_color) = match entry.source {
                                        ConsoleLogSource::PreRequest => ("Pre-request", crate::theme::hex_to_hsla(0x06b6d4)), // Cyan
                                        ConsoleLogSource::PostResponse => ("Post-response", crate::theme::hex_to_hsla(0xa855f7)), // Purple
                                        ConsoleLogSource::System => ("System", t.text_secondary),
                                    };

                                    // Level badge colors
                                    let (lvl_label, lvl_color) = match entry.level {
                                        ConsoleLogLevel::Log => ("LOG", t.text_secondary),
                                        ConsoleLogLevel::Info => ("INFO", crate::theme::hex_to_hsla(0x3b82f6)), // Blue
                                        ConsoleLogLevel::Warn => ("WARN", crate::theme::hex_to_hsla(0xf59e0b)), // Amber
                                        ConsoleLogLevel::Error => ("ERROR", t.method_delete), // Red
                                    };

                                    let card_border = if is_err {
                                        gpui::Hsla { a: 0.35, ..t.method_delete }
                                    } else if is_warn {
                                        gpui::Hsla { a: 0.35, ..crate::theme::hex_to_hsla(0xf59e0b) }
                                    } else {
                                        t.border
                                    };

                                    let card_bg = if is_err {
                                        gpui::Hsla { a: 0.08, ..t.method_delete }
                                    } else if is_warn {
                                        gpui::Hsla { a: 0.06, ..crate::theme::hex_to_hsla(0xf59e0b) }
                                    } else {
                                        t.bg_card
                                    };

                                    div()
                                        .p(px(8.))
                                        .rounded(px(6.))
                                        .bg(card_bg)
                                        .border_1()
                                        .border_color(card_border)
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(8.))
                                                        // Timestamp
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .font_family(".SystemMonoFont")
                                                                .text_color(t.text_muted)
                                                                .child(entry.timestamp.clone()),
                                                        )
                                                        // Source badge
                                                        .child(
                                                            div()
                                                                .px(px(6.))
                                                                .py(px(1.))
                                                                .rounded(px(3.))
                                                                .bg(gpui::Hsla { a: 0.12, ..src_color })
                                                                .border_1()
                                                                .border_color(gpui::Hsla { a: 0.25, ..src_color })
                                                                .text_xs()
                                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                .text_color(src_color)
                                                                .child(src_label),
                                                        )
                                                        // Level badge
                                                        .child(
                                                            div()
                                                                .px(px(5.))
                                                                .py(px(1.))
                                                                .rounded(px(3.))
                                                                .bg(gpui::Hsla { a: 0.12, ..lvl_color })
                                                                .border_1()
                                                                .border_color(gpui::Hsla { a: 0.25, ..lvl_color })
                                                                .text_xs()
                                                                .font_weight(gpui::FontWeight::BOLD)
                                                                .text_color(lvl_color)
                                                                .child(lvl_label),
                                                        ),
                                                )
                                                // Copy single message button
                                                .child(
                                                    div()
                                                        .id(format!("btn-copy-log-{}", idx))
                                                        .cursor_pointer()
                                                        .text_color(t.text_muted)
                                                        .hover(|s| s.text_color(t.text_primary))
                                                        .on_click(move |_, _, cx| {
                                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(msg_copy.clone()));
                                                        })
                                                        .child("📋"),
                                                ),
                                        )
                                        // Log message body
                                        .child(
                                            div()
                                                .pl(px(4.))
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(if is_err {
                                                    t.method_delete
                                                } else if is_warn {
                                                    crate::theme::hex_to_hsla(0xf59e0b)
                                                } else if is_info {
                                                    crate::theme::hex_to_hsla(0x38bdf8)
                                                } else {
                                                    t.text_primary
                                                })
                                                .child(entry.message.clone()),
                                        )
                                }))
                        }
                    }),
            )
    }
}
