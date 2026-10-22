use crate::state::{ModalState, WorkspaceDataState};
use crate::theme::theme;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;

#[derive(IntoElement)]
pub struct CookiesModal {
    workspace_state: Entity<WorkspaceDataState>,
    modal_state: Entity<ModalState>,
}

impl CookiesModal {
    pub fn new(
        workspace_state: Entity<WorkspaceDataState>,
        modal_state: Entity<ModalState>,
    ) -> Self {
        Self {
            workspace_state,
            modal_state,
        }
    }
}

impl RenderOnce for CookiesModal {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let t = theme();
        let ws_clone = self.workspace_state.clone();
        let modal_clone = self.modal_state.clone();

        div()
            .id("cookies-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .on_click({
                let modal_s = modal_clone.clone();
                move |_, _, cx| {
                    modal_s.update(cx, |state, cx| {
                        state.toggle_cookies_modal(cx);
                    });
                }
            })
            .on_key_down({
                let modal_s = modal_clone.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        modal_s.update(cx, |state, cx| {
                            state.toggle_cookies_modal(cx);
                        });
                    }
                }
            })
            .child(
                div()
                    .id("cookies-modal-dialog")
                    .occlude()
                    .w(px(520.))
                    .h(px(360.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_xl()
                    .flex()
                    .flex_col()
                    .on_key_down({
                        let modal_s = modal_clone.clone();
                        move |event, _, cx| {
                            if event.keystroke.key.as_str() == "escape" {
                                modal_s.update(cx, |state, cx| {
                                    state.toggle_cookies_modal(cx);
                                });
                            }
                        }
                    })
                    // Header
                    .child(
                        div()
                            .h(px(44.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(t.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div().flex().items_center().gap(px(6.)).child("🍪").child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(t.text_primary)
                                        .child("Workspace Cookie Jar"),
                                ),
                            )
                            .child({
                                let modal_s = modal_clone.clone();
                                div()
                                    .id("close-cookies-modal")
                                    .cursor_pointer()
                                    .text_color(t.text_muted)
                                    .hover(|s| s.text_color(t.text_primary))
                                    .on_click(move |_, _, cx| {
                                        modal_s.update(cx, |state, cx| {
                                            state.toggle_cookies_modal(cx);
                                        });
                                    })
                                    .child("✕")
                            }),
                    )
                    // Content
                    .child(
                        div()
                            .flex_1()
                            .p(px(16.))
                            .overflow_y_scrollbar()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("No cookies stored in the active session."),
                            ),
                    )
                    // Footer
                    .child(
                        div()
                            .h(px(44.))
                            .px(px(16.))
                            .border_t_1()
                            .border_color(t.border)
                            .bg(t.bg_titlebar)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child({
                                let ws_s = ws_clone.clone();
                                div()
                                    .id("btn-cookies-modal-clear")
                                    .px(px(10.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(t.bg_input)
                                    .border_1()
                                    .border_color(t.border)
                                    .text_color(t.text_secondary)
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .on_click(move |_, _, cx| {
                                        ws_s.update(cx, |state, cx| {
                                            state.clear_cookies(cx);
                                        });
                                    })
                                    .child("Clear Cookies")
                            })
                            .child({
                                let modal_s = modal_clone.clone();
                                div()
                                    .id("btn-cookies-modal-done")
                                    .px(px(12.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(t.primary)
                                    .text_color(t.primary_foreground)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .on_click(move |_, _, cx| {
                                        modal_s.update(cx, |state, cx| {
                                            state.toggle_cookies_modal(cx);
                                        });
                                    })
                                    .child("Done")
                            }),
                    ),
            )
    }
}
