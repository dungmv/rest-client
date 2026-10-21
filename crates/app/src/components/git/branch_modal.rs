use super::common::{render_error_banner, render_not_a_repo_view};
use crate::state::WorkspaceState;
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};

#[derive(IntoElement)]
pub struct GitBranchModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitBranchModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

pub fn render_new_branch_view(
    ws_clone: Entity<WorkspaceState>,
    current_branch: String,
    branch_input: Entity<InputState>,
) -> gpui::AnyElement {
    let t = theme();
    let ws_create = ws_clone.clone();
    let ws_cancel = ws_clone.clone();
    let inp = branch_input.clone();
    let inp_val = branch_input.clone();

    div()
        .p(px(20.))
        .flex()
        .flex_col()
        .gap(px(14.))
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
                                .child("Branch name"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(t.text_muted)
                                .child(format!("Based on: {}", current_branch)),
                        ),
                )
                .child(
                    div()
                        .bg(t.bg_sidebar)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(6.))
                        .p(px(4.))
                        .child(Input::new(&branch_input)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(t.text_muted)
                        .child("The new branch will be created and checked out automatically."),
                ),
        )
        // Buttons
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .pt(px(6.))
                .child(
                    div()
                        .id("btn-cancel-branch")
                        .px(px(14.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(t.border)
                        .cursor_pointer()
                        .text_xs()
                        .text_color(t.text_secondary)
                        .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                        .on_click(move |_, _, cx| {
                            ws_cancel.update(cx, |ws, cx| {
                                ws.close_git_modal(cx);
                            });
                        })
                        .child("Cancel"),
                )
                .child(
                    div()
                        .id("btn-create-branch-action")
                        .px(px(14.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .bg(t.primary)
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(gpui::rgb(0xffffff))
                        .hover(|s| s.opacity(0.9))
                        .on_click(move |_, window, cx| {
                            let name = inp_val.read(cx).value().trim().to_string();
                            if name.is_empty() {
                                return;
                            }
                            let ws_s = ws_create.clone();
                            let inp_clear = inp.clone();
                            ws_s.update(cx, |ws, cx| {
                                if ws.git_create_branch(&name, cx).is_ok() {
                                    inp_clear.update(cx, |inp_st, cx| {
                                        inp_st.set_value("", window, cx);
                                    });
                                    ws.close_git_modal(cx);
                                }
                            });
                        })
                        .child("Create Branch"),
                ),
        )
        .into_any_element()
}

impl RenderOnce for GitBranchModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ws_clone = self.workspace_state.clone();

        let is_repo = ws.git_status.as_ref().map(|s| s.is_repo).unwrap_or(false);
        let current_branch = ws.git_branch.clone().unwrap_or_else(|| "main".to_string());
        let git_error = ws.git_error.clone();

        let branch_input_holder = window.use_keyed_state(
            gpui::SharedString::from("git-new-branch-input"),
            cx,
            |window, cx| {
                let inp = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("New branch name (e.g. feature/auth)")
                });
                (inp, ())
            },
        );
        let branch_input = branch_input_holder.read(cx).0.clone();

        div()
            .id("git-branch-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla { a: 0.65, ..t.bg_app })
            .flex()
            .items_center()
            .justify_center()
            .on_click({
                let ws_s = ws_clone.clone();
                move |_, _, cx| {
                    ws_s.update(cx, |state, cx| {
                        state.close_git_modal(cx);
                    });
                }
            })
            .on_key_down({
                let ws_s = ws_clone.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        ws_s.update(cx, |state, cx| {
                            state.close_git_modal(cx);
                        });
                    }
                }
            })
            .child(
                div()
                    .id("git-branch-modal-dialog")
                    .occlude()
                    .w(px(420.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_key_down({
                        let ws_s = ws_clone.clone();
                        move |event, _, cx| {
                            if event.keystroke.key.as_str() == "escape" {
                                ws_s.update(cx, |state, cx| {
                                    state.close_git_modal(cx);
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
                            .bg(t.bg_titlebar)
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
                                            .text_base()
                                            .text_color(t.primary)
                                            .child("⎇"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("New Branch"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("btn-git-branch-close")
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(t.text_muted)
                                    .hover(|s| s.text_color(t.text_primary))
                                    .on_click({
                                        let ws_s = ws_clone.clone();
                                        move |_, _, cx| {
                                            ws_s.update(cx, |state, cx| {
                                                state.close_git_modal(cx);
                                            });
                                        }
                                    })
                                    .child("✕"),
                            ),
                    )
                    // Error banner
                    .when_some(git_error, |this, err| {
                        this.child(render_error_banner(ws_clone.clone(), err))
                    })
                    // Body
                    .child(
                        if !is_repo {
                            render_not_a_repo_view(ws_clone.clone())
                        } else {
                            render_new_branch_view(ws_clone.clone(), current_branch, branch_input)
                        },
                    ),
            )
            .into_any_element()
    }
}
