use crate::components::tabs_with_badge::TabItem;
use crate::state::{EditorState, ResponseState, ResponseTabKind, ResponseViewMode};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;

#[derive(IntoElement)]
pub struct ResponseViewer {
    editor_state: Entity<EditorState>,
    response_state: Entity<ResponseState>,
}

impl ResponseViewer {
    pub fn new(
        editor_state: Entity<EditorState>,
        response_state: Entity<ResponseState>,
    ) -> Self {
        Self {
            editor_state,
            response_state,
        }
    }
}

impl RenderOnce for ResponseViewer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let resp_s = self.response_state.read(cx);
        let ed_s = self.editor_state.read(cx);

        let current_resp = resp_s.current_response.clone();
        let active_tab = resp_s.active_tab;
        let view_mode = resp_s.view_mode;
        let show_timing = resp_s.show_timing_popover;
        let show_history = resp_s.show_history_dropdown;

        let resp_state_clone = self.response_state.clone();

        div()
            .size_full()
            .bg(t.bg_response)
            .flex()
            .flex_col()
            .relative()
            // Top Status Bar: "200 OK • 18 ms • 2 B" + history icon
            .child(
                div()
                    .h(px(40.))
                    .px(px(12.))
                    .border_b_1()
                    .border_color(t.border)
                    .bg(t.bg_editor)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child({
                        if let Some(resp) = &current_resp {
                            let status_color = t.status_color(resp.status_code);
                            let rs_clone = resp_state_clone.clone();

                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(gpui::Hsla { a: 0.12, ..status_color })
                                        .border_1()
                                        .border_color(gpui::Hsla { a: 0.3, ..status_color })
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(status_color)
                                        .child(format!("● {} {}", resp.status_code, resp.status_text)),
                                )
                                .child({
                                    // Duration with timing popover hover/click
                                    div()
                                        .id("btn-timing-breakdown")
                                        .cursor_pointer()
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(if show_timing { t.bg_hover } else { t.bg_card })
                                        .border_1()
                                        .border_color(t.border)
                                        .text_xs()
                                        .text_color(t.text_secondary)
                                        .hover(|s| s.text_color(t.text_primary).bg(t.bg_hover))
                                        .on_click(move |_, _, cx| {
                                            rs_clone.update(cx, |state, cx| {
                                                state.toggle_timing_popover(cx);
                                            });
                                        })
                                        .child(format!("⚡ {}", resp.formatted_duration()))
                                })
                                .child(
                                    div()
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(t.bg_card)
                                        .border_1()
                                        .border_color(t.border)
                                        .text_xs()
                                        .text_color(t.text_secondary)
                                        .child(format!("📦 {}", resp.formatted_size())),
                                )
                        } else if ed_s.is_sending {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(t.primary)
                                .child(ed_s.send_progress.clone().unwrap_or_else(|| "Sending...".to_string()))
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child("No response yet"),
                                )
                        }
                    })
                    // History button (clock icon)
                    .child({
                        let rs_clone = resp_state_clone.clone();
                        div()
                            .id("btn-response-history")
                            .px(px(8.))
                            .h(px(26.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .bg(if show_history { t.bg_hover } else { t.bg_card })
                            .border_1()
                            .border_color(t.border)
                            .hover(|s| s.bg(t.bg_hover))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_color(t.text_secondary)
                            .on_click(move |_, _, cx| {
                                rs_clone.update(cx, |state, cx| {
                                    state.toggle_history_dropdown(cx);
                                });
                            })
                            .child("🕒 History")
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child(format!("({})", resp_s.history.len())),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("▾"),
                            )
                    }),
            )
            // High-Precision Timing Popover
            .when(show_timing, |this| {
                if let Some(resp) = &current_resp {
                    let timing = &resp.timing;
                    let total_ms = timing.total_ms.max(1);

                    // Proportions for visual bar
                    let dns_pct = (timing.dns_ms.unwrap_or(0) as f32 / total_ms as f32 * 100.0).clamp(0.0, 100.0);
                    let tcp_pct = (timing.connect_ms.unwrap_or(0) as f32 / total_ms as f32 * 100.0).clamp(0.0, 100.0);
                    let tls_pct = (timing.tls_ms.unwrap_or(0) as f32 / total_ms as f32 * 100.0).clamp(0.0, 100.0);
                    let ttfb_pct = (timing.ttfb_ms.unwrap_or(0) as f32 / total_ms as f32 * 100.0).clamp(0.0, 100.0);
                    let dl_pct = (timing.download_ms.unwrap_or(0) as f32 / total_ms as f32 * 100.0).clamp(0.0, 100.0);

                    this.child(
                        div()
                            .p(px(10.))
                            .mx(px(12.))
                            .mt(px(4.))
                            .rounded(px(6.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .text_xs()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(t.text_primary)
                                            .child("High-Precision Timing Breakdown"),
                                    )
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.primary)
                                            .child(format!("Total: {} ms", timing.total_ms)),
                                    ),
                            )
                            // Visual Waterfall bar
                            .child(
                                div()
                                    .h(px(8.))
                                    .w_full()
                                    .rounded(px(4.))
                                    .bg(t.bg_input)
                                    .overflow_hidden()
                                    .flex()
                                    .child(div().w(px(dns_pct * 2.5)).bg(gpui::rgb(0x06b6d4))) // Cyan for DNS
                                    .child(div().w(px(tcp_pct * 2.5)).bg(gpui::rgb(0x3b82f6))) // Blue for TCP
                                    .child(div().w(px(tls_pct * 2.5)).bg(gpui::rgb(0xa855f7))) // Purple for TLS
                                    .child(div().w(px(ttfb_pct * 2.5)).bg(gpui::rgb(0xeab308))) // Yellow for TTFB
                                    .child(div().w(px(dl_pct * 2.5)).bg(gpui::rgb(0x22c55e))), // Green for Download
                            )
                            // Detail rows
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_color(t.text_secondary)
                                    .child(div().flex().gap(px(4.)).child("● DNS Lookup:").text_color(gpui::rgb(0x06b6d4)))
                                    .child(format!("{} ms", timing.dns_ms.unwrap_or(0))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_color(t.text_secondary)
                                    .child(div().flex().gap(px(4.)).child("● TCP Connect:").text_color(gpui::rgb(0x3b82f6)))
                                    .child(format!("{} ms", timing.connect_ms.unwrap_or(0))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_color(t.text_secondary)
                                    .child(div().flex().gap(px(4.)).child("● TLS Handshake:").text_color(gpui::rgb(0xa855f7)))
                                    .child(format!("{} ms", timing.tls_ms.unwrap_or(0))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_color(t.text_secondary)
                                    .child(div().flex().gap(px(4.)).child("● TTFB (Wait):").text_color(gpui::rgb(0xeab308)))
                                    .child(format!("{} ms", timing.ttfb_ms.unwrap_or(0))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_color(t.text_secondary)
                                    .child(div().flex().gap(px(4.)).child("● Content Download:").text_color(gpui::rgb(0x22c55e)))
                                    .child(format!("{} ms", timing.download_ms.unwrap_or(0))),
                            )
                            .when(timing.connection_reused, |this| {
                                this.child(
                                    div()
                                        .px(px(4.))
                                        .py(px(2.))
                                        .rounded(px(3.))
                                        .bg(t.primary.opacity(0.15))
                                        .text_color(t.primary)
                                        .child("⚡ Connection Reused (HTTP Keep-Alive)"),
                                )
                            }),
                    )
                } else {
                    this
                }
            })
            // Response Tabs Bar: Response, Request Sent, Headers [N], Cookies [N]
            .child(
                div()
                    .h(px(34.))
                    .px(px(8.))
                    .border_b_1()
                    .border_color(t.border)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child({
                        let rs_c = resp_state_clone.clone();
                        TabItem::new("tab-resp-response", "Response")
                            .active(active_tab == ResponseTabKind::Response)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Response, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        TabItem::new("tab-resp-request", "Request Sent")
                            .active(active_tab == ResponseTabKind::Request)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Request, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        let count_str = current_resp.as_ref().map(|r| format!("{}", r.headers.len())).unwrap_or_else(|| "0".to_string());
                        TabItem::new("tab-resp-headers", "Headers")
                            .active(active_tab == ResponseTabKind::Headers)
                            .badge(count_str)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Headers, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        let count_str = current_resp.as_ref().map(|r| format!("{}", r.cookies.len())).unwrap_or_else(|| "0".to_string());
                        TabItem::new("tab-resp-cookies", "Cookies")
                            .active(active_tab == ResponseTabKind::Cookies)
                            .badge(count_str)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Cookies, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        let total_tests = resp_s.test_results.len();
                        let passed_tests = resp_s.test_results.iter().filter(|r| r.passed).count();
                        let badge_str = if total_tests > 0 {
                            format!("{}/{}", passed_tests, total_tests)
                        } else {
                            "0".to_string()
                        };
                        TabItem::new("tab-resp-tests", "Tests")
                            .active(active_tab == ResponseTabKind::Tests)
                            .badge(badge_str)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Tests, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        let total_logs = resp_s.console_logs.len();
                        let badge_str = format!("{}", total_logs);
                        TabItem::new("tab-resp-console", "Console")
                            .active(active_tab == ResponseTabKind::Console)
                            .badge(badge_str)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Console, cx);
                                });
                            })
                    }),
            )
            // Body / Content area
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child({
                        match active_tab {
                            ResponseTabKind::Response => {
                                if current_resp.is_none() {
                                    div()
                                        .size_full()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(10.))
                                        .child(
                                            div()
                                                .text_3xl()
                                                .text_color(t.text_muted)
                                                .child("🌐"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(t.text_secondary)
                                                .child("Waiting for response..."),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .child("Press \"Send\" or Ctrl+Enter to execute this request"),
                                        )
                                        .into_any_element()
                                } else {
                                    let body_bytes = current_resp.as_ref().map(|r| r.body.clone()).unwrap_or_default();
                                let raw_str = String::from_utf8_lossy(&body_bytes).to_string();

                                let parsed_json = serde_json::from_str::<serde_json::Value>(&raw_str).ok();
                                let is_json = parsed_json.is_some()
                                    || current_resp.as_ref().map(|r| r.is_json()).unwrap_or(false)
                                    || raw_str.trim_start().starts_with('{')
                                    || raw_str.trim_start().starts_with('[');

                                // Format if Pretty mode and valid JSON
                                let display_text = if matches!(view_mode, ResponseViewMode::Pretty) {
                                    if let Some(parsed) = &parsed_json {
                                        serde_json::to_string_pretty(parsed).unwrap_or_else(|_| raw_str.clone())
                                    } else {
                                        raw_str.clone()
                                    }
                                } else {
                                    raw_str.clone()
                                };

                                let lines: Vec<&str> = display_text.lines().collect();
                                let total_lines = lines.len();
                                let is_large_payload = body_bytes.len() > 2 * 1024 * 1024 || total_lines > 5000;
                                let lines_to_render = if is_large_payload {
                                    &lines[..lines.len().min(3000)]
                                } else {
                                    &lines[..]
                                };

                                let rs_toggle = resp_state_clone.clone();
                                let raw_for_copy = raw_str.clone();
                                let bytes_for_save = body_bytes.clone();

                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    // Response sub-toolbar
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
                                                    .gap(px(4.))
                                                    // Pretty Button
                                                    .child({
                                                        let rs_c = rs_toggle.clone();
                                                        let is_pretty = matches!(view_mode, ResponseViewMode::Pretty);
                                                        div()
                                                            .id("btn-view-pretty")
                                                            .px(px(8.))
                                                            .py(px(2.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if is_pretty { t.border } else { t.transparent })
                                                            .bg(if is_pretty { t.bg_selected } else { t.transparent })
                                                            .text_color(if is_pretty { t.text_primary } else { t.text_secondary })
                                                            .text_xs()
                                                            .font_weight(if is_pretty { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                                            .cursor_pointer()
                                                            .hover(|s| if is_pretty { s } else { s.text_color(t.text_primary).bg(t.bg_hover) })
                                                            .on_click(move |_, _, cx| {
                                                                rs_c.update(cx, |state, cx| {
                                                                    state.set_view_mode(ResponseViewMode::Pretty, cx);
                                                                });
                                                            })
                                                            .child("Pretty")
                                                    })
                                                    // Raw Button
                                                    .child({
                                                        let rs_c = rs_toggle.clone();
                                                        let is_raw = matches!(view_mode, ResponseViewMode::Raw);
                                                        div()
                                                            .id("btn-view-raw")
                                                            .px(px(8.))
                                                            .py(px(2.))
                                                            .rounded(px(4.))
                                                            .border_1()
                                                            .border_color(if is_raw { t.border } else { t.transparent })
                                                            .bg(if is_raw { t.bg_selected } else { t.transparent })
                                                            .text_color(if is_raw { t.text_primary } else { t.text_secondary })
                                                            .text_xs()
                                                            .font_weight(if is_raw { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                                                            .cursor_pointer()
                                                            .hover(|s| if is_raw { s } else { s.text_color(t.text_primary).bg(t.bg_hover) })
                                                            .on_click(move |_, _, cx| {
                                                                rs_c.update(cx, |state, cx| {
                                                                    state.set_view_mode(ResponseViewMode::Raw, cx);
                                                                });
                                                            })
                                                            .child("Raw")
                                                    })
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    // Copy Button
                                                    .child({
                                                        let copy_text = raw_for_copy.clone();
                                                        div()
                                                            .id("btn-copy-response")
                                                            .px(px(6.))
                                                            .py(px(2.))
                                                            .rounded(px(4.))
                                                            .bg(t.bg_card)
                                                            .border_1()
                                                            .border_color(t.border)
                                                            .hover(|s| s.bg(t.bg_hover))
                                                            .text_xs()
                                                            .text_color(t.text_secondary)
                                                            .cursor_pointer()
                                                            .on_click(move |_, _, cx| {
                                                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_text.clone()));
                                                            })
                                                            .child("📋 Copy")
                                                    })
                                                    // Save to File Button
                                                    .child({
                                                        let save_bytes = bytes_for_save.clone();
                                                        div()
                                                            .id("btn-save-response")
                                                            .px(px(6.))
                                                            .py(px(2.))
                                                            .rounded(px(4.))
                                                            .bg(t.bg_card)
                                                            .border_1()
                                                            .border_color(t.border)
                                                            .hover(|s| s.bg(t.bg_hover))
                                                            .text_xs()
                                                            .text_color(t.text_secondary)
                                                            .cursor_pointer()
                                                            .on_click(move |_, _, _| {
                                                                if let Some(path) = rfd::FileDialog::new()
                                                                    .set_file_name("response.json")
                                                                    .save_file()
                                                                {
                                                                    let _ = std::fs::write(path, &save_bytes);
                                                                }
                                                            })
                                                            .child("💾 Save to File")
                                                    }),
                                            ),
                                    )
                                    // Large payload protection banner
                                    .when(is_large_payload, |this| {
                                        this.child(
                                            div()
                                                .px(px(12.))
                                                .py(px(6.))
                                                .bg(t.var_unresolved.opacity(0.12))
                                                .border_b_1()
                                                .border_color(t.var_unresolved.opacity(0.3))
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(t.var_unresolved)
                                                        .child(format!("⚠ Large payload detected ({} lines). Showing first 3,000 lines to ensure 120 FPS performance.", total_lines)),
                                                )
                                        )
                                    })
                                    // Response lines scrollable area
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_y_scrollbar()
                                            .p(px(8.))
                                            .flex()
                                            .gap(px(12.))
                                            .child(
                                                // Line numbers
                                                div()
                                                    .w(px(32.))
                                                    .flex()
                                                    .flex_col()
                                                    .items_end()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_muted)
                                                    .children((1..=lines_to_render.len()).map(|num| {
                                                        div().h(px(20.)).child(format!("{}", num))
                                                    })),
                                            )
                                            .child(
                                                // Response text lines
                                                div()
                                                    .flex_1()
                                                    .flex()
                                                    .flex_col()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .children(lines_to_render.iter().map(|line| {
                                                        div().h(px(20.)).child(if line.is_empty() {
                                                            div().child(" ")
                                                        } else if is_json {
                                                            div().child(highlight_json_line(line, t))
                                                        } else {
                                                            div().child(line.to_string())
                                                        })
                                                    })),
                                            ),
                                    )
                                    .into_any_element()
                                }
                            }
                            ResponseTabKind::Request => {
                                if let Some(resp) = &current_resp {
                                    if let Some(sent) = &resp.request_sent {
                                        div()
                                            .size_full()
                                            .overflow_y_scrollbar()
                                            .p(px(12.))
                                            .flex()
                                            .flex_col()
                                            .gap(px(12.))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .child(
                                                        div()
                                                            .font_weight(gpui::FontWeight::BOLD)
                                                            .text_color(t.method_color(sent.method))
                                                            .child(sent.method.as_str()),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family(".SystemMonoFont")
                                                            .text_xs()
                                                            .text_color(t.text_primary)
                                                            .child(sent.url.clone()),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_muted)
                                                    .child("Headers Sent:"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(4.))
                                                    .children(sent.headers.iter().map(|(k, v)| {
                                                        div()
                                                            .flex()
                                                            .gap(px(8.))
                                                            .text_xs()
                                                            .font_family(".SystemMonoFont")
                                                            .child(div().w(px(160.)).text_color(t.primary).child(k.clone()))
                                                            .child(div().flex_1().text_color(t.text_primary).child(v.clone()))
                                                    })),
                                            )
                                            .when_some(sent.body_preview.clone(), |this, preview| {
                                                let is_preview_json = preview.trim_start().starts_with('{') || preview.trim_start().starts_with('[');
                                                let preview_display = if is_preview_json {
                                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&preview) {
                                                        serde_json::to_string_pretty(&parsed).unwrap_or(preview.clone())
                                                    } else {
                                                        preview.clone()
                                                    }
                                                } else {
                                                    preview.clone()
                                                };
                                                let p_lines: Vec<&str> = preview_display.lines().collect();

                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .text_color(t.text_muted)
                                                        .child(format!("Body Sent ({} bytes):", sent.body_size_bytes)),
                                                )
                                                .child(
                                                    div()
                                                        .p(px(8.))
                                                        .bg(t.bg_card)
                                                        .rounded(px(6.))
                                                        .border_1()
                                                        .border_color(t.border)
                                                        .font_family(".SystemMonoFont")
                                                        .text_xs()
                                                        .flex()
                                                        .flex_col()
                                                        .children(p_lines.into_iter().map(|line| {
                                                            div().h(px(20.)).child(if line.is_empty() {
                                                                div().child(" ")
                                                            } else if is_preview_json {
                                                                div().child(highlight_json_line(line, t))
                                                            } else {
                                                                div().child(line.to_string())
                                                            })
                                                        })),
                                                )
                                            })
                                            .into_any_element()
                                    } else {
                                        div().p(px(12.)).text_xs().text_color(t.text_muted).child("No request sent info available.").into_any_element()
                                    }
                                } else {
                                    div().p(px(12.)).text_xs().text_color(t.text_muted).child("No request sent yet.").into_any_element()
                                }
                            }
                            ResponseTabKind::Headers => {
                                if let Some(resp) = &current_resp {
                                    div()
                                        .size_full()
                                        .overflow_y_scrollbar()
                                        .p(px(12.))
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .children(resp.headers.iter().enumerate().map(|(idx, (k, v))| {
                                            let val_copy = v.clone();
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(12.))
                                                .py(px(4.))
                                                .border_b_1()
                                                .border_color(t.border_subtle)
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .child(div().w(px(200.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.primary).child(k.clone()))
                                                .child(div().flex_1().text_color(t.text_primary).child(v.clone()))
                                                .child(
                                                    div()
                                                        .id(format!("copy-hdr-{}", idx))
                                                        .cursor_pointer()
                                                        .text_color(t.text_muted)
                                                        .hover(|s| s.text_color(t.text_primary))
                                                        .on_click(move |_, _, cx| {
                                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(val_copy.clone()));
                                                        })
                                                        .child("📋"),
                                                )
                                        }))
                                        .into_any_element()
                                } else {
                                    div().p(px(12.)).text_xs().text_color(t.text_muted).child("No headers received.").into_any_element()
                                }
                            }
                            ResponseTabKind::Cookies => {
                                if let Some(resp) = &current_resp {
                                    let parsed_cookies = resp.parsed_cookies();
                                    if parsed_cookies.is_empty() {
                                        div().p(px(12.)).text_xs().text_color(t.text_muted).child("No cookies received in this response.").into_any_element()
                                    } else {
                                        div()
                                            .size_full()
                                            .overflow_y_scrollbar()
                                            .p(px(12.))
                                            .flex()
                                            .flex_col()
                                            .gap(px(8.))
                                            .children(parsed_cookies.into_iter().map(|cookie| {
                                                div()
                                                    .p(px(8.))
                                                    .rounded(px(6.))
                                                    .bg(t.bg_card)
                                                    .border_1()
                                                    .border_color(t.border)
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
                                                                    .font_weight(gpui::FontWeight::BOLD)
                                                                    .text_color(t.primary)
                                                                    .child(cookie.name.clone()),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .gap(px(4.))
                                                                    .when(cookie.secure, |this| {
                                                                        this.child(div().px(px(4.)).rounded(px(2.)).bg(gpui::rgb(0x22c55e)).text_color(gpui::rgb(0xffffff)).text_xs().child("Secure"))
                                                                    })
                                                                    .when(cookie.http_only, |this| {
                                                                        this.child(div().px(px(4.)).rounded(px(2.)).bg(gpui::rgb(0x3b82f6)).text_color(gpui::rgb(0xffffff)).text_xs().child("HttpOnly"))
                                                                    }),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family(".SystemMonoFont")
                                                            .text_xs()
                                                            .text_color(t.text_primary)
                                                            .child(cookie.value.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .gap(px(12.))
                                                            .text_xs()
                                                            .text_color(t.text_muted)
                                                            .when_some(cookie.domain, |this, d| this.child(format!("Domain: {}", d)))
                                                            .when_some(cookie.path, |this, p| this.child(format!("Path: {}", p)))
                                                            .when_some(cookie.expires, |this, exp| this.child(format!("Expires: {}", exp))),
                                                    )
                                            }))
                                            .into_any_element()
                                    }
                                } else {
                                    div().p(px(12.)).text_xs().text_color(t.text_muted).child("No cookies received.").into_any_element()
                                }
                            }
                            ResponseTabKind::Tests => {
                                crate::components::TestResultsPanel::new(
                                    resp_s.test_results.clone(),
                                    resp_s.console_logs.clone(),
                                )
                                .into_any_element()
                            }
                            ResponseTabKind::Console => {
                                crate::components::ConsolePanel::new(
                                    resp_state_clone.clone(),
                                )
                                .into_any_element()
                            }
                        }
                    }),
            )
            // History Dropdown Menu (rendered last so it paints above tabs and response body)
            .when(show_history, |this| {
                let history = resp_s.history.clone();
                let rs_c = resp_state_clone.clone();

                this.child(
                    div()
                        .absolute()
                        .top(px(42.))
                        .right(px(12.))
                        .w(px(320.))
                        .max_h(px(340.))
                        .overflow_y_scrollbar()
                        .bg(t.bg_card)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(6.))
                        .shadow_xl()
                        .p(px(6.))
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(4.))
                                .border_b_1()
                                .border_color(t.border)
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(t.text_primary)
                                        .child("Past Responses (Max 20)"),
                                )
                                .child({
                                    let rs_close = rs_c.clone();
                                    div()
                                        .id("btn-close-hist-dropdown")
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .hover(|s| s.text_color(t.text_primary))
                                        .on_click(move |_, _, cx| {
                                            rs_close.update(cx, |state, cx| {
                                                state.toggle_history_dropdown(cx);
                                            });
                                        })
                                        .child("✕")
                                }),
                        )
                        .child(if history.is_empty() {
                            div()
                                .p(px(12.))
                                .text_xs()
                                .italic()
                                .text_color(t.text_muted)
                                .child("No previous runs recorded in SQLite history.")
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .flex_col()
                                .children(history.into_iter().map(|item| {
                                    let rec_id = item.id.clone();
                                    let rs_set = rs_c.clone();
                                    let status_color = t.status_color(item.status_code);
                                    div()
                                        .id(format!("hist-item-{}", rec_id))
                                        .px(px(8.))
                                        .py(px(6.))
                                        .rounded(px(4.))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(t.bg_hover))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .text_xs()
                                        .on_click(move |_, _, cx| {
                                            rs_set.update(cx, |state, cx| {
                                                state.select_history_record(&rec_id, cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .font_weight(gpui::FontWeight::BOLD)
                                                        .text_color(status_color)
                                                        .child(format!("{}", item.status_code)),
                                                )
                                                .child(
                                                    div()
                                                        .text_color(t.text_secondary)
                                                        .child(item.formatted_duration()),
                                                )
                                                .child(
                                                    div()
                                                        .text_color(t.text_muted)
                                                        .child(item.formatted_size()),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_color(t.text_muted)
                                                .child(item.created_at.format("%H:%M:%S").to_string()),
                                        )
                                }))
                                .into_any_element()
                        }),
                )
            })
    }
}

pub fn highlight_json_line(line: &str, t: &crate::theme::AppTheme) -> gpui::StyledText {
    let mut highlights = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'"' {
            let start = i;
            i += 1;
            while i < len {
                let ch = line[i..].chars().next().unwrap();
                let ch_len = ch.len_utf8();
                if ch == '\\' {
                    i += ch_len;
                    if i < len {
                        let next_ch = line[i..].chars().next().unwrap();
                        i += next_ch.len_utf8();
                    }
                } else if ch == '"' {
                    i += ch_len;
                    break;
                } else {
                    i += ch_len;
                }
            }
            let end = i;

            // Check if followed by colon (ignoring spaces) -> JSON object key
            let mut j = end;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            let is_key = j < len && bytes[j] == b':';
            let color = if is_key { t.json_key } else { t.json_string };
            highlights.push((
                start..end,
                gpui::HighlightStyle {
                    color: Some(color),
                    ..Default::default()
                },
            ));
        } else if matches!(b, b'{' | b'}' | b'[' | b']' | b':' | b',') {
            highlights.push((
                i..i + 1,
                gpui::HighlightStyle {
                    color: Some(t.json_punctuation),
                    ..Default::default()
                },
            ));
            i += 1;
        } else if b == b'-' || b.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_digit()
                    || matches!(bytes[i], b'.' | b'e' | b'E' | b'+' | b'-'))
            {
                i += 1;
            }
            highlights.push((
                start..i,
                gpui::HighlightStyle {
                    color: Some(t.json_number),
                    ..Default::default()
                },
            ));
        } else if line[i..].starts_with("true")
            && (i + 4 == len || !bytes[i + 4].is_ascii_alphanumeric())
        {
            highlights.push((
                i..i + 4,
                gpui::HighlightStyle {
                    color: Some(t.json_boolean),
                    font_weight: Some(gpui::FontWeight::SEMIBOLD),
                    ..Default::default()
                },
            ));
            i += 4;
        } else if line[i..].starts_with("false")
            && (i + 5 == len || !bytes[i + 5].is_ascii_alphanumeric())
        {
            highlights.push((
                i..i + 5,
                gpui::HighlightStyle {
                    color: Some(t.json_boolean),
                    font_weight: Some(gpui::FontWeight::SEMIBOLD),
                    ..Default::default()
                },
            ));
            i += 5;
        } else if line[i..].starts_with("null")
            && (i + 4 == len || !bytes[i + 4].is_ascii_alphanumeric())
        {
            highlights.push((
                i..i + 4,
                gpui::HighlightStyle {
                    color: Some(t.json_null),
                    font_style: Some(gpui::FontStyle::Italic),
                    ..Default::default()
                },
            ));
            i += 4;
        } else {
            let ch = line[i..].chars().next().unwrap();
            i += ch.len_utf8();
        }
    }

    gpui::StyledText::new(line.to_string()).with_highlights(highlights)
}

