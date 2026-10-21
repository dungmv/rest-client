use crate::state::{SidebarState, WorkspaceDataState};
use crate::theme::theme;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::Input;
use gpui_component::Icon;
use gpui_kit::assets::IconName;

#[derive(IntoElement)]
pub struct RenameWorkspaceModal {
    workspace_state: Entity<WorkspaceDataState>,
    sidebar_state: Entity<SidebarState>,
}

impl RenameWorkspaceModal {
    pub fn new(
        workspace_state: Entity<WorkspaceDataState>,
        sidebar_state: Entity<SidebarState>,
    ) -> Self {
        Self {
            workspace_state,
            sidebar_state,
        }
    }
}

impl RenderOnce for RenameWorkspaceModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws_s = self.workspace_state.clone();
        let sb_s = self.sidebar_state.clone();
        let input_entity = self.sidebar_state.read(cx).rename_workspace_input.clone();

        div()
            .id("rename-workspace-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.6, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .on_click({
                let sb_c = sb_s.clone();
                move |_, _, cx| {
                    sb_c.update(cx, |state, cx| {
                        state.cancel_renaming_workspace(cx);
                    });
                }
            })
            .on_key_down({
                let sb_c = sb_s.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        sb_c.update(cx, |state, cx| {
                            state.cancel_renaming_workspace(cx);
                        });
                    }
                }
            })
            .child(
                div()
                    .id("rename-workspace-modal-card")
                    .occlude()
                    .w(px(420.))
                    .p(px(20.))
                    .rounded(px(10.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .on_key_down({
                        let sb_c = sb_s.clone();
                        move |event, _, cx| {
                            if event.keystroke.key.as_str() == "escape" {
                                sb_c.update(cx, |state, cx| {
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
                                    .child(
                                        Icon::new(IconName::Pencil)
                                            .size(px(16.))
                                            .text_color(t.primary),
                                    )
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
                                    .id("btn-close-rename-workspace")
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(t.text_muted)
                                    .hover(|s| s.text_color(t.text_primary))
                                    .on_click({
                                        let sb_c = sb_s.clone();
                                        move |_, _, cx| {
                                            sb_c.update(cx, |state, cx| {
                                                state.cancel_renaming_workspace(cx);
                                            });
                                        }
                                    })
                                    .child("✕"),
                            ),
                    )
                    // Input Body
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_secondary)
                                    .child("Enter a new display name for this workspace:"),
                            )
                            .child(
                                div()
                                    .h(px(32.))
                                    .bg(t.bg_input)
                                    .border_1()
                                    .border_color(t.border)
                                    .rounded(px(6.))
                                    .px(px(8.))
                                    .flex()
                                    .items_center()
                                    .child(if let Some(inp) = &input_entity {
                                        Input::new(inp)
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .into_any_element()
                                    } else {
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("No input")
                                            .into_any_element()
                                    }),
                            ),
                    )
                    // Actions Footer
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("btn-cancel-rename-workspace")
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(px(6.))
                                    .bg(t.bg_sidebar)
                                    .border_1()
                                    .border_color(t.border)
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(t.text_secondary)
                                    .hover(|s| s.bg(t.bg_hover))
                                    .on_click({
                                        let sb_c = sb_s.clone();
                                        move |_, _, cx| {
                                            sb_c.update(cx, |state, cx| {
                                                state.cancel_renaming_workspace(cx);
                                            });
                                        }
                                    })
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("btn-save-rename-workspace")
                                    .px(px(14.))
                                    .py(px(6.))
                                    .rounded(px(6.))
                                    .bg(t.primary)
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(t.primary_foreground)
                                    .hover(|s| s.opacity(0.9))
                                    .on_click({
                                        let ws_c = ws_s.clone();
                                        let sb_c = sb_s.clone();
                                        let inp_c = input_entity.clone();
                                        move |_, _, cx| {
                                            let val = inp_c
                                                .as_ref()
                                                .map(|i| i.read(cx).value().to_string())
                                                .unwrap_or_default();
                                            sb_c.update(cx, |sb, cx| {
                                                sb.cancel_renaming_workspace(cx);
                                            });
                                            let trimmed = val.trim();
                                            if !trimmed.is_empty() {
                                                ws_c.update(cx, |ws, cx| {
                                                    ws.rename_workspace(trimmed.to_string(), cx);
                                                });
                                            }
                                        }
                                    })
                                    .child("Save"),
                            ),
                    ),
            )
    }
}
