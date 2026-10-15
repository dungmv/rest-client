use crate::components::status_badge::MethodBadge;
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;

#[derive(IntoElement)]
pub struct SettingsModal {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
}

impl SettingsModal {
    pub fn new(
        workspace_state: Entity<WorkspaceState>,
        editor_state: Entity<EditorState>,
    ) -> Self {
        Self {
            workspace_state,
            editor_state,
        }
    }
}

impl RenderOnce for SettingsModal {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();

        let active_req = ed_clone.read(cx).active_request.clone();
        let settings = active_req
            .as_ref()
            .map(|r| r.settings.clone())
            .unwrap_or_default();

        let workspace_name = ws_clone
            .read(cx)
            .active_workspace
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "default".to_string());

        div()
            .id("settings-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .on_click({
                let ws_s = ws_clone.clone();
                move |_, _, cx| {
                    ws_s.update(cx, |state, cx| {
                        state.toggle_settings_modal(cx);
                    });
                }
            })
            .child(
                div()
                    .id("settings-modal-dialog")
                    .occlude()
                    .w(px(580.))
                    .max_h(px(580.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Modal Header
                    .child(
                        div()
                            .h(px(46.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(t.border)
                            .bg(t.bg_titlebar)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child("⚙")
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("Settings"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("• Request & network configuration"),
                                    ),
                            )
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("close-settings-modal")
                                    .cursor_pointer()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_color(t.text_muted)
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.toggle_settings_modal(cx);
                                        });
                                    })
                                    .child("✕")
                            }),
                    )
                    // Modal Body
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .p(px(20.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            // Active request badge indicator
                            .child({
                                if let Some(req) = &active_req {
                                    div()
                                        .p(px(10.))
                                        .rounded(px(6.))
                                        .bg(t.bg_app)
                                        .border_1()
                                        .border_color(t.border)
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(t.text_muted)
                                                .child("Active Request:"),
                                        )
                                        .child(MethodBadge::new(req.method))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(t.text_primary)
                                                .child(req.name.clone()),
                                        )
                                } else {
                                    div()
                                        .p(px(10.))
                                        .rounded(px(6.))
                                        .bg(t.bg_app)
                                        .border_1()
                                        .border_color(t.border)
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child("No active request selected. Default settings shown.")
                                }
                            })
                            // Network & Request section
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_secondary)
                                    .child("NETWORK & REQUEST EXECUTION"),
                            )
                            // 1. Timeout presets
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_primary)
                                                    .child("Request Timeout"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child(format!("Current: {}s", settings.timeout_secs)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("Maximum time to wait before terminating the HTTP request."),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(8.))
                                            .pt(px(2.))
                                            .children([10, 30, 60, 120].into_iter().map(|s| {
                                                let is_cur = settings.timeout_secs == s;
                                                let ed_set = ed_clone.clone();
                                                div()
                                                    .id(format!("timeout-{}s", s))
                                                    .px(px(12.))
                                                    .py(px(5.))
                                                    .rounded(px(4.))
                                                    .bg(if is_cur { t.primary } else { t.bg_input })
                                                    .border_1()
                                                    .border_color(if is_cur { t.primary } else { t.border })
                                                    .text_color(if is_cur {
                                                        t.primary_foreground
                                                    } else {
                                                        t.text_secondary
                                                    })
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .hover(|s| {
                                                        if !is_cur {
                                                            s.bg(t.bg_hover).text_color(t.text_primary)
                                                        } else {
                                                            s
                                                        }
                                                    })
                                                    .on_click(move |_, _, cx| {
                                                        ed_set.update(cx, |st, cx| {
                                                            st.set_timeout_secs(s, cx);
                                                        });
                                                    })
                                                    .child(format!("{}s", s))
                                            })),
                                    ),
                            )
                            // 2. Follow Redirects toggle
                            .child({
                                let cur_follow = settings.follow_redirects;
                                let ed_set = ed_clone.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_primary)
                                                    .child("Follow HTTP Redirects"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Automatically follow 301, 302, 307, and 308 redirects."),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("toggle-follow-redirects")
                                            .px(px(10.))
                                            .py(px(5.))
                                            .rounded(px(4.))
                                            .bg(if cur_follow { t.primary } else { t.bg_input })
                                            .border_1()
                                            .border_color(if cur_follow { t.primary } else { t.border })
                                            .text_color(if cur_follow {
                                                t.primary_foreground
                                            } else {
                                                t.text_muted
                                            })
                                            .cursor_pointer()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .hover(|s| {
                                                if !cur_follow {
                                                    s.bg(t.bg_hover).text_color(t.text_primary)
                                                } else {
                                                    s
                                                }
                                            })
                                            .on_click(move |_, _, cx| {
                                                ed_set.update(cx, |st, cx| {
                                                    st.set_follow_redirects(!cur_follow, cx);
                                                });
                                            })
                                            .child(if cur_follow { "✓ Enabled" } else { "Disabled" }),
                                    )
                            })
                            // 3. Validate TLS Certificates toggle
                            .child({
                                let cur_tls = settings.validate_tls;
                                let ed_set = ed_clone.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_primary)
                                                    .child("Validate SSL / TLS Certificates"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Verify SSL certificate chain. Disable for local development or self-signed certs."),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("toggle-validate-tls")
                                            .px(px(10.))
                                            .py(px(5.))
                                            .rounded(px(4.))
                                            .bg(if cur_tls { t.primary } else { t.bg_input })
                                            .border_1()
                                            .border_color(if cur_tls { t.primary } else { t.border })
                                            .text_color(if cur_tls {
                                                t.primary_foreground
                                            } else {
                                                t.text_muted
                                            })
                                            .cursor_pointer()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .hover(|s| {
                                                if !cur_tls {
                                                    s.bg(t.bg_hover).text_color(t.text_primary)
                                                } else {
                                                    s
                                                }
                                            })
                                            .on_click(move |_, _, cx| {
                                                ed_set.update(cx, |st, cx| {
                                                    st.set_validate_tls(!cur_tls, cx);
                                                });
                                            })
                                            .child(if cur_tls { "✓ Enabled" } else { "Disabled" }),
                                    )
                            })
                            // Separator
                            .child(div().h(px(1.)).bg(t.border))
                            // Workspace & Storage section
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_secondary)
                                    .child("WORKSPACE & ENVIRONMENT"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Active Workspace"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(t.text_primary)
                                                    .child(workspace_name),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Storage Location"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_secondary)
                                                    .child(".restclient/workspaces/"),
                                            ),
                                    ),
                            )
                            // Separator
                            .child(div().h(px(1.)).bg(t.border))
                            // Keyboard Shortcuts
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(t.text_secondary)
                                    .child("KEYBOARD SHORTCUTS"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Send Request"),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.5))
                                                    .rounded(px(3.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child("Ctrl+Enter / Cmd+Enter"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Command Palette"),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.5))
                                                    .rounded(px(3.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child("Ctrl+K / Cmd+K"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Toggle Sidebar"),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.5))
                                                    .rounded(px(3.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child("Ctrl+B / Cmd+B"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .child("Open Settings"),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.5))
                                                    .rounded(px(3.))
                                                    .bg(t.bg_input)
                                                    .border_1()
                                                    .border_color(t.border)
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(t.text_primary)
                                                    .child("Ctrl+, / Cmd+,"),
                                            ),
                                    ),
                            ),
                    )
                    // Modal Footer
                    .child(
                        div()
                            .h(px(48.))
                            .px(px(16.))
                            .border_t_1()
                            .border_color(t.border)
                            .bg(t.bg_titlebar)
                            .flex()
                            .items_center()
                            .justify_end()
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("btn-done-settings")
                                    .px(px(14.))
                                    .py(px(5.))
                                    .rounded(px(4.))
                                    .bg(t.primary)
                                    .text_color(t.primary_foreground)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.toggle_settings_modal(cx);
                                        });
                                    })
                                    .child("Done")
                            }),
                    ),
            )
    }
}
