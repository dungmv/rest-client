use crate::state::{ConsoleFilter, ResponseState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use zest_core::models::{ConsoleLogEntry, ConsoleLogLevel, ConsoleLogSource, TestResult};

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
        let all_tests = resp_s.test_results.clone();
        let all_logs = resp_s.console_logs.clone();
        let current_filter = resp_s.console_filter;
        let rs = self.response_state.clone();

        let total_tests = all_tests.len();
        let passed_tests = all_tests.iter().filter(|r| r.passed).count();
        let failed_tests = total_tests.saturating_sub(passed_tests);

        let total_logs = all_logs.len();
        let error_logs = all_logs
            .iter()
            .filter(|l| l.level == ConsoleLogLevel::Error)
            .count();

        let total_items = total_tests + total_logs;
        let total_errors = failed_tests + error_logs;

        let filtered_tests: Vec<TestResult> = all_tests
            .into_iter()
            .filter(|r| match current_filter {
                ConsoleFilter::All | ConsoleFilter::Tests => true,
                ConsoleFilter::Logs => false,
                ConsoleFilter::Errors => !r.passed,
            })
            .collect();

        let filtered_logs: Vec<ConsoleLogEntry> = all_logs
            .into_iter()
            .filter(|l| match current_filter {
                ConsoleFilter::All | ConsoleFilter::Logs => true,
                ConsoleFilter::Tests => false,
                ConsoleFilter::Errors => l.level == ConsoleLogLevel::Error,
            })
            .collect();

        let has_filtered_items = !filtered_tests.is_empty() || !filtered_logs.is_empty();

        // Prepare copy text for visible tests and logs
        let mut copy_lines = Vec::new();
        for res in &filtered_tests {
            let status = if res.passed { "PASS" } else { "FAIL" };
            if let Some(err) = &res.error_message {
                copy_lines.push(format!(
                    "[TEST] [{}] {} ({}ms) - {}",
                    status, res.name, res.duration_ms, err
                ));
            } else {
                copy_lines.push(format!(
                    "[TEST] [{}] {} ({}ms)",
                    status, res.name, res.duration_ms
                ));
            }
        }
        for l in &filtered_logs {
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
            copy_lines.push(format!(
                "[{}] [{}] [{}] {}",
                l.timestamp, src, lvl, l.message
            ));
        }
        let all_copy_text = copy_lines.join("\n");

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
                    // Filter tabs + Pass/Fail summary on the left
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(render_filter_pill(
                                "filter-all",
                                format!("All ({})", total_items),
                                current_filter == ConsoleFilter::All,
                                ConsoleFilter::All,
                                rs.clone(),
                            ))
                            .child(render_filter_pill(
                                "filter-tests",
                                format!("Tests ({})", total_tests),
                                current_filter == ConsoleFilter::Tests,
                                ConsoleFilter::Tests,
                                rs.clone(),
                            ))
                            .child(render_filter_pill(
                                "filter-logs",
                                format!("Logs ({})", total_logs),
                                current_filter == ConsoleFilter::Logs,
                                ConsoleFilter::Logs,
                                rs.clone(),
                            ))
                            .when(
                                total_errors > 0 || current_filter == ConsoleFilter::Errors,
                                |this| {
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
                                            .child(format!("Errors ({})", total_errors)),
                                    )
                                },
                            )
                            // Pass/Fail summary badge when tests exist
                            .when(total_tests > 0, |this| {
                                let (badge_bg, badge_border, badge_text) = if failed_tests == 0 {
                                    (
                                        gpui::Hsla {
                                            a: 0.15,
                                            ..t.method_post
                                        },
                                        gpui::Hsla {
                                            a: 0.4,
                                            ..t.method_post
                                        },
                                        t.method_post,
                                    )
                                } else {
                                    (
                                        gpui::Hsla {
                                            a: 0.15,
                                            ..t.method_delete
                                        },
                                        gpui::Hsla {
                                            a: 0.4,
                                            ..t.method_delete
                                        },
                                        t.method_delete,
                                    )
                                };

                                this.child(
                                    div()
                                        .ml(px(4.))
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(10.))
                                        .bg(badge_bg)
                                        .border_1()
                                        .border_color(badge_border)
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(badge_text)
                                        .child(format!(
                                            "{} {}/{} Passed",
                                            if failed_tests == 0 { "✓" } else { "✗" },
                                            passed_tests,
                                            total_tests
                                        )),
                                )
                            }),
                    )
                    // Action buttons on the right
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .when(has_filtered_items, |this| {
                                let copy_str = all_copy_text.clone();
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
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                        .on_click(move |_, _, cx| {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                                copy_str.clone(),
                                            ));
                                        })
                                        .child(Icon::new(IconName::Copy).size(px(13.)))
                                        .child("Copy All"),
                                )
                            })
                            .when(total_items > 0, |this| {
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
            // Unified content list or Empty State
            .child(div().flex_1().overflow_y_scrollbar().p(px(12.)).child({
                if total_items == 0 {
                    render_empty_state()
                } else if !has_filtered_items {
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
                                .child("No tests or logs match the selected filter."),
                        )
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .children(filtered_tests.into_iter().map(render_test_card))
                        .children(
                            filtered_logs
                                .into_iter()
                                .enumerate()
                                .map(|(idx, entry)| render_log_card(idx, entry)),
                        )
                }
            }))
    }
}

fn render_filter_pill(
    id: &'static str,
    label: String,
    is_active: bool,
    filter: ConsoleFilter,
    rs: Entity<ResponseState>,
) -> impl IntoElement {
    let t = theme();
    div()
        .id(id)
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
            rs.update(cx, |s, cx| {
                s.set_console_filter(filter, cx);
            });
        })
        .child(label)
}

fn render_empty_state() -> gpui::Div {
    let t = theme();
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .py(px(40.))
        .gap(px(10.))
        .child(div().text_3xl().text_color(t.text_muted).child("📟"))
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.text_primary)
                .child("No Tests or Console Output"),
        )
        .child(div().text_xs().text_color(t.text_secondary).child(
            "Test assertions from z.test(...) and logs from console.log() will appear here.",
        ))
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
                .child("// In Pre-request or Post-response Script:")
                .child("console.log(\"Response status:\", z.response.status);")
                .child("z.test(\"Status is 200\", () => {")
                .child("    z.expect(z.response.status).toBe(200);")
                .child("});"),
        )
}

fn render_test_card(res: TestResult) -> gpui::Div {
    let t = theme();
    let status_color = if res.passed {
        t.method_post
    } else {
        t.method_delete
    };

    div()
        .p(px(8.))
        .rounded(px(6.))
        .bg(if res.passed {
            t.bg_card
        } else {
            gpui::Hsla {
                a: 0.08,
                ..status_color
            }
        })
        .border_1()
        .border_color(if res.passed {
            t.border
        } else {
            gpui::Hsla {
                a: 0.35,
                ..status_color
            }
        })
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
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(3.))
                                .bg(gpui::Hsla {
                                    a: 0.14,
                                    ..status_color
                                })
                                .border_1()
                                .border_color(gpui::Hsla {
                                    a: 0.3,
                                    ..status_color
                                })
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(status_color)
                                .child(if res.passed { "✓ PASS" } else { "✗ FAIL" }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(t.text_primary)
                                .child(res.name),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(".SystemMonoFont")
                        .text_color(t.text_muted)
                        .child(format!("{}ms", res.duration_ms)),
                ),
        )
        .when_some(res.error_message, |this, err| {
            this.child(
                div()
                    .pl(px(12.))
                    .pt(px(2.))
                    .text_xs()
                    .font_family(".SystemMonoFont")
                    .text_color(status_color)
                    .child(format!("↳ {}", err)),
            )
        })
}

fn render_log_card(idx: usize, entry: ConsoleLogEntry) -> gpui::Div {
    let t = theme();
    let is_err = entry.level == ConsoleLogLevel::Error;
    let is_warn = entry.level == ConsoleLogLevel::Warn;
    let is_info = entry.level == ConsoleLogLevel::Info;

    let msg_copy = entry.message.clone();

    let (src_label, src_color) = match entry.source {
        ConsoleLogSource::PreRequest => ("Pre-request", crate::theme::hex_to_hsla(0x06b6d4)),
        ConsoleLogSource::PostResponse => ("Post-response", crate::theme::hex_to_hsla(0xa855f7)),
        ConsoleLogSource::System => ("System", t.text_secondary),
    };

    let (lvl_label, lvl_color) = match entry.level {
        ConsoleLogLevel::Log => ("LOG", t.text_secondary),
        ConsoleLogLevel::Info => ("INFO", crate::theme::hex_to_hsla(0x3b82f6)),
        ConsoleLogLevel::Warn => ("WARN", crate::theme::hex_to_hsla(0xf59e0b)),
        ConsoleLogLevel::Error => ("ERROR", t.method_delete),
    };

    let card_border = if is_err {
        gpui::Hsla {
            a: 0.35,
            ..t.method_delete
        }
    } else if is_warn {
        gpui::Hsla {
            a: 0.35,
            ..crate::theme::hex_to_hsla(0xf59e0b)
        }
    } else {
        t.border
    };

    let card_bg = if is_err {
        gpui::Hsla {
            a: 0.08,
            ..t.method_delete
        }
    } else if is_warn {
        gpui::Hsla {
            a: 0.06,
            ..crate::theme::hex_to_hsla(0xf59e0b)
        }
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
                        .child(
                            div()
                                .text_xs()
                                .font_family(".SystemMonoFont")
                                .text_color(t.text_muted)
                                .child(entry.timestamp),
                        )
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(3.))
                                .bg(gpui::Hsla {
                                    a: 0.12,
                                    ..src_color
                                })
                                .border_1()
                                .border_color(gpui::Hsla {
                                    a: 0.25,
                                    ..src_color
                                })
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(src_color)
                                .child(src_label),
                        )
                        .child(
                            div()
                                .px(px(5.))
                                .py(px(1.))
                                .rounded(px(3.))
                                .bg(gpui::Hsla {
                                    a: 0.12,
                                    ..lvl_color
                                })
                                .border_1()
                                .border_color(gpui::Hsla {
                                    a: 0.25,
                                    ..lvl_color
                                })
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(lvl_color)
                                .child(lvl_label),
                        ),
                )
                .child(
                    div()
                        .id(format!("btn-copy-log-{}", idx))
                        .cursor_pointer()
                        .text_color(t.text_muted)
                        .hover(|s| s.text_color(t.text_primary))
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                msg_copy.clone(),
                            ));
                        })
                        .child(Icon::new(IconName::Copy).size(px(14.))),
                ),
        )
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
                .child(entry.message),
        )
}
