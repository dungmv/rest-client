use crate::state::WorkspaceState;
use crate::theme::theme;
use gpui::{
    div, px, App, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::Input;
use gpui_component::Icon;
use gpui_kit::assets::IconName;

#[derive(IntoElement)]
pub struct RenameWorkspaceModal {
    workspace_state: Entity<WorkspaceState>,
}

impl RenameWorkspaceModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for RenameWorkspaceModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ws_s = self.workspace_state.clone();
        let input_entity = ws.rename_workspace_input.clone();

        div()
            .id("rename-workspace-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .on_click({
                let ws_c = ws_s.clone();
                move |_, _, cx| {
                    ws_c.update(cx, |state, cx| {
                        state.cancel_renaming_workspace(cx);
                    });
                }
            })
            .on_key_down({
                let ws_c = ws_s.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        ws_c.update(cx, |state, cx| {
                            state.cancel_renaming_workspace(cx);
                        });
                    }
                }
            })
            .child(
                div()
                    .id("rename-workspace-modal-dialog")
                    .occlude()
                    .w(px(380.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .p(px(16.))
                    .gap(px(14.))
                    .on_key_down({
                        let ws_c = ws_s.clone();
                        move |event, _, cx| {
                            if event.keystroke.key.as_str() == "escape" {
                                ws_c.update(cx, |state, cx| {
                                    state.cancel_renaming_workspace(cx);
                                });
                            }
                        }
                    })
                    // Header
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
                                    .child(Icon::new(IconName::Pencil).size(px(20.)))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("Rename Workspace"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-rename-workspace-modal")
                                    .cursor_pointer()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_color(t.text_muted)
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .on_click({
                                        let ws_c = ws_s.clone();
                                        move |_, _, cx| {
                                            ws_c.update(cx, |state, cx| {
                                                state.cancel_renaming_workspace(cx);
                                            });
                                        }
                                    })
                                    .child("✕"),
                            ),
                    )
                    // Description
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child("Enter a new name for this workspace:"),
                    )
                    // Input
                    .child({
                        let ws_c = ws_s.clone();
                        if let Some(inp) = input_entity.clone() {
                            div()
                                .w_full()
                                .on_key_down(move |event, _, cx| {
                                    if event.keystroke.key.as_str() == "escape" {
                                        ws_c.update(cx, |state, cx| {
                                            state.cancel_renaming_workspace(cx);
                                        });
                                    }
                                })
                                .child(Input::new(&inp))
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }
                    })
                    // Footer buttons: Cancel & Save
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.))
                            .pt(px(4.))
                            .child(
                                div()
                                    .id("btn-cancel-rename-ws")
                                    .px(px(12.))
                                    .py(px(5.))
                                    .rounded(px(4.))
                                    .bg(t.bg_input)
                                    .border_1()
                                    .border_color(t.border)
                                    .text_color(t.text_secondary)
                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .on_click({
                                        let ws_c = ws_s.clone();
                                        move |_, _, cx| {
                                            ws_c.update(cx, |state, cx| {
                                                state.cancel_renaming_workspace(cx);
                                            });
                                        }
                                    })
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("btn-save-rename-ws")
                                    .px(px(14.))
                                    .py(px(5.))
                                    .rounded(px(4.))
                                    .bg(t.primary)
                                    .text_color(t.primary_foreground)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .on_click({
                                        let ws_c = ws_s.clone();
                                        let inp_c = input_entity.clone();
                                        move |_, _, cx| {
                                            let val = inp_c
                                                .as_ref()
                                                .map(|i| i.read(cx).value().to_string())
                                                .unwrap_or_default();
                                            ws_c.update(cx, |state, cx| {
                                                state.finish_renaming_workspace(val, cx);
                                            });
                                        }
                                    })
                                    .child("Save"),
                            ),
                    ),
            )
    }
}
