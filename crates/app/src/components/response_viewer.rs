use crate::components::tabs_with_badge::TabItem;
use crate::state::{EditorState, ResponseState, ResponseTabKind};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
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
        let show_timing = resp_s.show_timing_popover;

        let resp_state_clone = self.response_state.clone();

        div()
            .size_full()
            .bg(t.bg_response)
            .flex()
            .flex_col()
            // Top Status Bar: "200 OK • 18 ms • 2 B" + history icon
            .child(
                div()
                    .h(px(40.))
                    .px(px(12.))
                    .border_b_1()
                    .border_color(t.border)
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
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(status_color)
                                        .child(format!("{} {}", resp.status_code, resp.status_text)),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child("•"),
                                )
                                .child({
                                    // Duration with timing popover hover/click
                                    div()
                                        .id("btn-timing-breakdown")
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(t.text_secondary)
                                        .hover(|s| s.text_color(t.primary))
                                        .on_click(move |_, _, cx| {
                                            rs_clone.update(cx, |state, cx| {
                                                state.toggle_timing_popover(cx);
                                            });
                                        })
                                        .child(resp.formatted_duration())
                                })
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child("•"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_secondary)
                                        .child(resp.formatted_size()),
                                )
                        } else if ed_s.is_sending {
                            div()
                                .text_xs()
                                .text_color(t.primary)
                                .child(ed_s.send_progress.clone().unwrap_or_else(|| "Sending...".to_string()))
                        } else {
                            // Reference screenshot style: placeholder status bar
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(t.status_2xx)
                                        .child("200 OK"),
                                )
                                .child(div().text_xs().text_color(t.text_muted).child("•"))
                                .child(div().text_xs().text_color(t.text_secondary).child("18 ms"))
                                .child(div().text_xs().text_color(t.text_muted).child("•"))
                                .child(div().text_xs().text_color(t.text_secondary).child("2 B"))
                        }
                    })
                    // History button (clock icon)
                    .child({
                        let rs_clone = resp_state_clone.clone();
                        div()
                            .id("btn-response-history")
                            .w(px(28.))
                            .h(px(28.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(t.text_secondary)
                            .on_click(move |_, _, cx| {
                                rs_clone.update(cx, |state, cx| {
                                    state.toggle_history_dropdown(cx);
                                });
                            })
                            .child("🕒")
                    }),
            )
            // Timing Popover
            .when(show_timing, |this| {
                if let Some(resp) = &current_resp {
                    let timing = &resp.timing;
                    this.child(
                        div()
                            .p(px(8.))
                            .mx(px(12.))
                            .mt(px(4.))
                            .rounded(px(6.))
                            .bg(t.bg_card)
                            .border_1()
                            .border_color(t.border)
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .text_xs()
                            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.text_primary).child("Timing Breakdown"))
                            .child(div().flex().justify_between().text_color(t.text_secondary).child("DNS Lookup:").child(format!("{} ms", timing.dns_ms.unwrap_or(0))))
                            .child(div().flex().justify_between().text_color(t.text_secondary).child("TCP Connect:").child(format!("{} ms", timing.connect_ms.unwrap_or(0))))
                            .child(div().flex().justify_between().text_color(t.text_secondary).child("TTFB:").child(format!("{} ms", timing.ttfb_ms.unwrap_or(0))))
                            .child(div().flex().justify_between().text_color(t.text_secondary).child("Download:").child(format!("{} ms", timing.download_ms.unwrap_or(0))))
                            .child(div().flex().justify_between().font_weight(gpui::FontWeight::BOLD).text_color(t.text_primary).child("Total:").child(format!("{} ms", timing.total_ms)))
                    )
                } else {
                    this
                }
            })
            // Response Tabs Bar: Response v, Request, Headers [4/12], Cookies [2/0]
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
                            .with_dropdown(true)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Response, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        TabItem::new("tab-resp-request", "Request")
                            .active(active_tab == ResponseTabKind::Request)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Request, cx);
                                });
                            })
                    })
                    .child({
                        let rs_c = resp_state_clone.clone();
                        let count_str = current_resp.as_ref().map(|r| format!("{}", r.headers.len())).unwrap_or_else(|| "4/12".to_string());
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
                        let count_str = current_resp.as_ref().map(|r| format!("{}", r.cookies.len())).unwrap_or_else(|| "2/0".to_string());
                        TabItem::new("tab-resp-cookies", "Cookies")
                            .active(active_tab == ResponseTabKind::Cookies)
                            .badge(count_str)
                            .on_click(move |_, _, cx| {
                                rs_c.update(cx, |state, cx| {
                                    state.set_active_tab(ResponseTabKind::Cookies, cx);
                                });
                            })
                    }),
            )
            // Body / Content area
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .p(px(8.))
                    .child({
                        match active_tab {
                            ResponseTabKind::Response => {
                                let body_str = if let Some(resp) = &current_resp {
                                    resp.body_as_str().unwrap_or("[]").to_string()
                                } else {
                                    "[]".to_string() // Matching reference screenshot: 1  []
                                };

                                let lines: Vec<String> = body_str.lines().map(|s| s.to_string()).collect();

                                div()
                                    .flex()
                                    .gap(px(12.))
                                    .child(
                                        // Line numbers
                                        div()
                                            .w(px(24.))
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
                                    .child(
                                        // Response body lines
                                        div()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .text_xs()
                                            .font_family(".SystemMonoFont")
                                            .text_color(t.text_primary)
                                            .children(lines.into_iter().map(|line| {
                                                div().h(px(20.)).child(if line.is_empty() { " ".to_string() } else { line })
                                            })),
                                    )
                            }
                            ResponseTabKind::Request => {
                                let ed = self.editor_state.read(cx);
                                let req_info = if let Some(req) = &ed.active_request {
                                    format!("{} {}\nHeaders: {}\nBody: {:?}", req.method, req.url, req.headers.len(), req.body)
                                } else {
                                    "No request sent".to_string()
                                };
                                div()
                                    .p(px(8.))
                                    .text_xs()
                                    .font_family(".SystemMonoFont")
                                    .text_color(t.text_secondary)
                                    .child(req_info)
                            }
                            ResponseTabKind::Headers => {
                                if let Some(resp) = &current_resp {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.))
                                        .children(resp.headers.iter().map(|(k, v)| {
                                            div()
                                                .flex()
                                                .gap(px(12.))
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .child(div().w(px(160.)).text_color(t.primary).child(k.clone()))
                                                .child(div().flex_1().text_color(t.text_primary).child(v.clone()))
                                        }))
                                } else {
                                    div().text_xs().text_color(t.text_muted).child("No headers")
                                }
                            }
                            ResponseTabKind::Cookies => {
                                if let Some(resp) = &current_resp {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.))
                                        .children(resp.cookies.iter().map(|c| {
                                            div()
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(t.text_primary)
                                                .child(c.clone())
                                        }))
                                } else {
                                    div().text_xs().text_color(t.text_muted).child("No cookies")
                                }
                            }
                        }
                    }),
            )
    }
}
