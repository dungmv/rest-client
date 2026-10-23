use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, InteractiveElement as _, IntoElement, ParentElement, RenderOnce, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;
use zest_core::models::{ConsoleLogEntry, ConsoleLogSource, TestResult};

#[derive(IntoElement)]
pub struct TestResultsPanel {
    results: Vec<TestResult>,
    console_logs: Vec<ConsoleLogEntry>,
}

impl TestResultsPanel {
    pub fn new(results: Vec<TestResult>, console_logs: Vec<ConsoleLogEntry>) -> Self {
        Self {
            results,
            console_logs,
        }
    }
}

impl RenderOnce for TestResultsPanel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = total.saturating_sub(passed);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.bg_response)
            .child(
                // Header summary bar
                div()
                    .h(px(40.))
                    .px(px(16.))
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
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_primary)
                                    .child("Test Results"),
                            )
                            .child({
                                let (badge_bg, badge_border, badge_text) = if total == 0 {
                                    (t.border, t.border, t.text_secondary)
                                } else if failed == 0 {
                                    (
                                        gpui::Hsla { a: 0.15, ..t.method_post },
                                        gpui::Hsla { a: 0.4, ..t.method_post },
                                        t.method_post,
                                    )
                                } else {
                                    (
                                        gpui::Hsla { a: 0.15, ..t.method_delete },
                                        gpui::Hsla { a: 0.4, ..t.method_delete },
                                        t.method_delete,
                                    )
                                };

                                div()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(10.))
                                    .bg(badge_bg)
                                    .border_1()
                                    .border_color(badge_border)
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(badge_text)
                                    .child(if total == 0 {
                                        "No tests ran".to_string()
                                    } else {
                                        format!("{}/{} Passed", passed, total)
                                    })
                            }),
                    ),
            )
            .child(
                // Test items list
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child({
                        if self.results.is_empty() {
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py(px(40.))
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_color(t.text_secondary)
                                        .text_sm()
                                        .child("No test script was executed for this response."),
                                )
                                .child(
                                    div()
                                        .text_color(t.text_secondary)
                                        .text_xs()
                                        .child("Add tests in the \"Tests\" tab of the request editor using z.test(...)"),
                                )
                        } else {
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .children(self.results.into_iter().map(|res| {
                                    let status_color = if res.passed {
                                        t.method_post // green
                                    } else {
                                        t.method_delete // red
                                    };

                                    div()
                                        .p(px(10.))
                                        .rounded(px(6.))
                                        .bg(t.bg_panel)
                                        .border_1()
                                        .border_color(if res.passed {
                                            t.border
                                        } else {
                                            gpui::Hsla { a: 0.3, ..status_color }
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
                                                                .text_sm()
                                                                .font_weight(gpui::FontWeight::BOLD)
                                                                .text_color(status_color)
                                                                .child(if res.passed { "✓" } else { "✗" }),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(t.text_primary)
                                                                .child(res.name),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_family("Consolas, monospace")
                                                        .text_color(t.text_secondary)
                                                        .child(format!("{}ms", res.duration_ms)),
                                                ),
                                        )
                                        .when_some(res.error_message, |this, err| {
                                            this.child(
                                                div()
                                                    .pl(px(20.))
                                                    .pt(px(4.))
                                                    .text_xs()
                                                    .font_family("Consolas, monospace")
                                                    .text_color(status_color)
                                                    .child(format!("↳ {}", err)),
                                            )
                                        })
                                }))
                        }
                    })
                    // Console Output Section
                    .when(!self.console_logs.is_empty(), |this| {
                        this.child(
                            div()
                                .mt(px(16.))
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(t.text_secondary)
                                        .child("Console Output:"),
                                )
                                .child(
                                    div()
                                        .p(px(10.))
                                        .rounded(px(6.))
                                        .bg(t.bg_editor)
                                        .border_1()
                                        .border_color(t.border)
                                        .font_family("Consolas, monospace")
                                        .text_xs()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.))
                                        .children(self.console_logs.into_iter().map(|log| {
                                            let prefix = match log.source {
                                                ConsoleLogSource::PreRequest => "[Pre-request]",
                                                ConsoleLogSource::PostResponse => "[Post-response]",
                                                ConsoleLogSource::System => "[System]",
                                            };
                                            div()
                                                .text_color(t.text_primary)
                                                .child(format!("> {} {}", prefix, log.message))
                                        })),
                                ),
                        )
                    }),
            )
    }
}
