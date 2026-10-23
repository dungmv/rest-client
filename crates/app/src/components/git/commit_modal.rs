use super::common::{
    render_checkbox, render_error_banner, render_not_a_repo_view, render_status_badge,
};
use super::diff_view::{render_diff_content, render_diff_view_toggle, DiffViewMode};
use crate::state::{GitState, WorkspaceDataState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use std::collections::HashSet;

#[derive(IntoElement)]
pub struct GitCommitModal {
    git_state: Entity<GitState>,
    workspace_state: Entity<WorkspaceDataState>,
}

impl GitCommitModal {
    pub fn new(git_state: Entity<GitState>, workspace_state: Entity<WorkspaceDataState>) -> Self {
        Self {
            git_state,
            workspace_state,
        }
    }
}

pub struct ChangesViewProps {
    pub git_clone: Entity<GitState>,
    pub ws_clone: Entity<WorkspaceDataState>,
    pub status_opt: Option<zest_core::git::GitStatusResult>,
    pub selected_file: Option<(String, bool)>,
    pub diff_content: Option<String>,
    pub commit_input: Entity<InputState>,
    pub view_mode: DiffViewMode,
    pub mode_entity: Entity<DiffViewMode>,
    pub selected_staged: HashSet<String>,
    pub selected_unstaged: HashSet<String>,
    pub is_loading: bool,
}

pub fn render_changes_view(props: ChangesViewProps) -> gpui::AnyElement {
    let t = theme();
    let git_clone = props.git_clone;
    let ws_clone = props.ws_clone;
    let selected_file = props.selected_file;
    let diff_content = props.diff_content;
    let commit_input = props.commit_input;
    let view_mode = props.view_mode;
    let mode_entity = props.mode_entity;
    let selected_staged = props.selected_staged;
    let selected_unstaged = props.selected_unstaged;
    let is_loading = props.is_loading;

    let staged_files = props
        .status_opt
        .as_ref()
        .map(|s| s.staged_files.clone())
        .unwrap_or_default();
    let unstaged_files = props
        .status_opt
        .as_ref()
        .map(|s| s.unstaged_files.clone())
        .unwrap_or_default();
    let untracked_files = props
        .status_opt
        .as_ref()
        .map(|s| s.untracked_files.clone())
        .unwrap_or_default();

    let total_staged = staged_files.len();

    div()
        .size_full()
        .flex()
        // Left Column: Staging lists and Commit input (width: 350px)
        .child(
            div()
                .w(px(350.))
                .min_w(px(350.))
                .max_w(px(350.))
                .flex_shrink_0()
                .h_full()
                .border_r_1()
                .border_color(t.border)
                .bg(t.bg_sidebar)
                .flex()
                .flex_col()
                // 1. Staged & Working Files Lists (Scrollable top area)
                .child(
                    div()
                        .flex_1()
                        .overflow_y_scrollbar()
                        .p(px(10.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        // Staged Changes Section
                        .child({
                            let staged_selected_count = staged_files.iter().filter(|f| selected_staged.contains(&f.path)).count();
                            let all_staged_selected = total_staged > 0 && staged_selected_count == total_staged;
                            let partial_staged_selected = staged_selected_count > 0 && !all_staged_selected;
                            let git_toggle_all_staged = git_clone.clone();

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .px(px(4.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .when(total_staged > 0, |this| {
                                                    this.child(render_checkbox(
                                                        "chk-select-all-staged",
                                                        all_staged_selected,
                                                        partial_staged_selected,
                                                        move |_, _, cx| {
                                                            let select = !all_staged_selected;
                                                            git_toggle_all_staged.update(cx, |git, cx| {
                                                                git.toggle_all_staged_selection(select, cx);
                                                            });
                                                        },
                                                    ))
                                                })
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .text_color(t.text_secondary)
                                                        .child(format!("Staged ({})", total_staged)),
                                                ),
                                        )
                                        .when(total_staged > 0, |this| {
                                            let git_unstage = git_clone.clone();
                                            let has_sel = staged_selected_count > 0;
                                            let btn_label = if has_sel {
                                                format!("Unstage Selected ({})", staged_selected_count)
                                            } else {
                                                "Unstage Selected".to_string()
                                            };
                                            this.child(
                                                div()
                                                    .id("btn-unstage-selected")
                                                    .text_xs()
                                                    .when(has_sel, |s| {
                                                        s.cursor_pointer()
                                                            .text_color(t.primary)
                                                            .hover(|h| h.underline())
                                                            .on_click(move |_, _, cx| {
                                                                git_unstage.update(cx, |git, cx| {
                                                                    git.unstage_selected(cx);
                                                                });
                                                            })
                                                    })
                                                    .when(!has_sel, |s| {
                                                        s.text_color(t.text_muted)
                                                    })
                                                    .child(btn_label),
                                            )
                                        }),
                                )
                                .children(staged_files.into_iter().map(|item| {
                                    let git_sel = git_clone.clone();
                                    let git_chk = git_clone.clone();
                                    let p_chk = item.path.clone();
                                    let p_sel = item.path.clone();
                                    let is_checked = selected_staged.contains(&item.path);
                                    let is_selected = selected_file.as_ref().map(|(sel, st)| sel == &item.path && *st).unwrap_or(false);

                                    div()
                                        .id(format!("staged-item-{}", item.path))
                                        .px(px(6.))
                                        .py(px(4.))
                                        .rounded(px(4.))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .cursor_pointer()
                                        .when(is_selected, |s| s.bg(t.bg_hover))
                                        .hover(|s| s.bg(t.bg_hover))
                                        .on_click(move |_, _, cx| {
                                            git_sel.update(cx, |git, cx| {
                                                git.select_file(p_sel.clone(), true, cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .overflow_hidden()
                                                .child(render_checkbox(
                                                    &format!("chk-staged-{}", item.path),
                                                    is_checked,
                                                    false,
                                                    move |_, _, cx| {
                                                        git_chk.update(cx, |git, cx| {
                                                            git.toggle_staged_selection(&p_chk, cx);
                                                        });
                                                    },
                                                ))
                                                .child(render_status_badge(item.status))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_family(".SystemMonoFont")
                                                        .text_color(t.text_primary)
                                                        .truncate()
                                                        .child(item.path.clone()),
                                                ),
                                        )
                                }))
                        })
                        // Working Changes Section
                        .child({
                            let all_changes: Vec<_> = unstaged_files.into_iter().chain(untracked_files).collect();
                            let total_changes = all_changes.len();
                            let changes_selected_count = all_changes.iter().filter(|f| selected_unstaged.contains(&f.path)).count();
                            let all_changes_selected = total_changes > 0 && changes_selected_count == total_changes;
                            let partial_changes_selected = changes_selected_count > 0 && !all_changes_selected;
                            let git_toggle_all_unstaged = git_clone.clone();

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .px(px(4.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .when(total_changes > 0, |this| {
                                                    this.child(render_checkbox(
                                                        "chk-select-all-changes",
                                                        all_changes_selected,
                                                        partial_changes_selected,
                                                        move |_, _, cx| {
                                                            let select = !all_changes_selected;
                                                            git_toggle_all_unstaged.update(cx, |git, cx| {
                                                                git.toggle_all_unstaged_selection(select, cx);
                                                            });
                                                        },
                                                    ))
                                                })
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .text_color(t.text_secondary)
                                                        .child(format!("Changes ({})", total_changes)),
                                                ),
                                        )
                                        .when(total_changes > 0, |this| {
                                            let git_stage = git_clone.clone();
                                            let has_sel = changes_selected_count > 0;
                                            let btn_label = if has_sel {
                                                format!("Stage Selected ({})", changes_selected_count)
                                            } else {
                                                "Stage Selected".to_string()
                                            };
                                            this.child(
                                                div()
                                                    .id("btn-stage-selected")
                                                    .text_xs()
                                                    .when(has_sel, |s| {
                                                        s.cursor_pointer()
                                                            .text_color(t.primary)
                                                            .hover(|h| h.underline())
                                                            .on_click(move |_, _, cx| {
                                                                git_stage.update(cx, |git, cx| {
                                                                    git.stage_selected(cx);
                                                                });
                                                            })
                                                    })
                                                    .when(!has_sel, |s| {
                                                        s.text_color(t.text_muted)
                                                    })
                                                    .child(btn_label),
                                            )
                                        }),
                                )
                                .children(all_changes.into_iter().map(|item| {
                                    let git_sel = git_clone.clone();
                                    let git_chk = git_clone.clone();
                                    let git_disc = git_clone.clone();
                                    let ws_disc = ws_clone.clone();
                                    let p_chk = item.path.clone();
                                    let p_disc = item.path.clone();
                                    let p_sel = item.path.clone();
                                    let is_checked = selected_unstaged.contains(&item.path);
                                    let is_selected = selected_file.as_ref().map(|(sel, st)| sel == &item.path && !*st).unwrap_or(false);

                                    div()
                                        .id(format!("unstaged-item-{}", item.path))
                                        .px(px(6.))
                                        .py(px(4.))
                                        .rounded(px(4.))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .cursor_pointer()
                                        .when(is_selected, |s| s.bg(t.bg_hover))
                                        .hover(|s| s.bg(t.bg_hover))
                                        .on_click(move |_, _, cx| {
                                            git_sel.update(cx, |git, cx| {
                                                git.select_file(p_sel.clone(), false, cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .overflow_hidden()
                                                .child(render_checkbox(
                                                    &format!("chk-unstaged-{}", item.path),
                                                    is_checked,
                                                    false,
                                                    move |_, _, cx| {
                                                        git_chk.update(cx, |git, cx| {
                                                            git.toggle_unstaged_selection(&p_chk, cx);
                                                        });
                                                    },
                                                ))
                                                .child(render_status_badge(item.status))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_family(".SystemMonoFont")
                                                        .text_color(t.text_primary)
                                                        .truncate()
                                                        .child(item.path.clone()),
                                                ),
                                        )
                                        .child(
                                            // Discard button (↺)
                                            div()
                                                .id(format!("btn-discard-{}", item.path))
                                                .cursor_pointer()
                                                .p(px(2.))
                                                .rounded(px(3.))
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .hover(|s| s.bg(t.bg_card).text_color(t.status_5xx))
                                                .on_click(move |_, _, cx| {
                                                    cx.stop_propagation();
                                                    git_disc.update(cx, |git, cx| {
                                                        git.discard_file(&p_disc, cx);
                                                    });
                                                    ws_disc.update(cx, |ws, cx| {
                                                        ws.reload_all();
                                                        cx.notify();
                                                    });
                                                })
                                                .child("↺"),
                                        )
                                }))
                        }),
                )
                // 2. Commit Section (Moved to bottom with taller input)
                .child(
                    div()
                        .p(px(10.))
                        .border_t_1()
                        .border_color(t.border)
                        .bg(t.bg_card)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .bg(t.bg_app)
                                .border_1()
                                .border_color(t.border)
                                .rounded(px(6.))
                                .p(px(6.))
                                .h(px(68.))
                                .flex()
                                .flex_col()
                                .child(
                                    Input::new(&commit_input)
                                        .appearance(false)
                                        .bordered(false)
                                ),
                        )
                        .child({
                            let git_commit = git_clone.clone();
                            let inp = commit_input.clone();
                            let can_commit = total_staged > 0 && !is_loading;

                            div()
                                .id("btn-commit-action")
                                .h(px(32.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(can_commit, |s| {
                                    s.bg(t.primary)
                                        .cursor_pointer()
                                        .text_color(t.primary_foreground)
                                        .hover(|h| h.opacity(0.9))
                                })
                                .when(!can_commit, |s| {
                                    s.bg(t.bg_hover)
                                        .text_color(t.text_muted)
                                })
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .on_click(move |_, window, cx| {
                                    if !can_commit {
                                        return;
                                    }
                                    let msg = inp.read(cx).value().to_string();
                                    if msg.trim().is_empty() {
                                        return;
                                    }
                                    let inp_clear = inp.clone();
                                    inp_clear.update(cx, |inp_st, cx| {
                                        inp_st.set_value("", window, cx);
                                    });
                                    let git_s = git_commit.clone();
                                    git_s.update(cx, |git, cx| {
                                        git.commit(&msg, cx);
                                    });
                                })
                                .when(is_loading, |s| {
                                    s.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(Icon::new(IconName::RefreshCw).size(px(14.)))
                                            .child("Committing..."),
                                    )
                                })
                                .when(!is_loading, |s| {
                                    s.child(format!("✓ Commit Changes ({} staged)", total_staged))
                                })
                        }),
                ),
        )
        // Right Column: Diff Viewer (Flex-1)
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .overflow_hidden()
                .bg(t.bg_app)
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(40.))
                        .px(px(14.))
                        .border_b_1()
                        .border_color(t.border)
                        .bg(t.bg_card)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .when_some(selected_file.as_ref(), |this, (p, staged)| {
                                    this.child(
                                        div()
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(t.bg_hover)
                                            .border_1()
                                            .border_color(t.border)
                                            .text_xs()
                                            .text_color(if *staged { t.primary } else { t.text_secondary })
                                            .child(if *staged { "staged" } else { "working tree" }),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family(".SystemMonoFont")
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(t.text_primary)
                                            .child(p.clone()),
                                    )
                                })
                                .when(selected_file.is_none(), |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(t.text_muted)
                                            .child("No file selected"),
                                    )
                                }),
                        )
                        .child(render_diff_view_toggle(view_mode, mode_entity.clone())),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_y_scrollbar()
                        .p(px(8.))
                        .child(render_diff_content(diff_content, view_mode)),
                ),
        )
        .into_any_element()
}

impl RenderOnce for GitCommitModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let git_clone = self.git_state.clone();
        let ws_clone = self.workspace_state.clone();

        let (
            is_repo,
            branch_name,
            status_opt,
            selected_file,
            diff_content,
            git_error,
            selected_unstaged,
            selected_staged,
            is_loading,
        ) = {
            let git = self.git_state.read(cx);
            (
                git.status.as_ref().map(|s| s.is_repo).unwrap_or(false),
                git.branch.clone().unwrap_or_else(|| "main".to_string()),
                git.status.clone(),
                git.selected_file.clone(),
                git.diff_content.clone(),
                git.error.clone(),
                git.selected_unstaged_files.clone(),
                git.selected_staged_files.clone(),
                git.loading,
            )
        };

        let staged_count = status_opt
            .as_ref()
            .map(|s| s.staged_files.len())
            .unwrap_or(0);
        let unstaged_count = status_opt
            .as_ref()
            .map(|s| s.unstaged_files.len())
            .unwrap_or(0);
        let untracked_count = status_opt
            .as_ref()
            .map(|s| s.untracked_files.len())
            .unwrap_or(0);
        let total_changes = staged_count + unstaged_count + untracked_count;

        let commit_input_holder = window.use_keyed_state(
            gpui::SharedString::from("git-commit-msg-input"),
            cx,
            |window, cx| {
                let inp = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("Commit message (e.g. feat: add login endpoint)")
                });
                (inp, ())
            },
        );
        let commit_input = commit_input_holder.read(cx).0.clone();

        let view_mode_holder = window.use_keyed_state(
            gpui::SharedString::from("git-diff-view-mode"),
            cx,
            |_window, cx| {
                let st = cx.new(|_cx| DiffViewMode::Split);
                (st, ())
            },
        );
        let view_mode_entity = view_mode_holder.read(cx).0.clone();
        let view_mode = *view_mode_entity.read(cx);

        div()
            .id("git-commit-modal-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla {
                a: 0.65,
                ..t.bg_app
            })
            .flex()
            .items_center()
            .justify_center()
            .on_click({
                let git_s = git_clone.clone();
                move |_, _, cx| {
                    git_s.update(cx, |state, cx| {
                        state.close_modal(cx);
                    });
                }
            })
            .on_key_down({
                let git_s = git_clone.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        git_s.update(cx, |state, cx| {
                            state.close_modal(cx);
                        });
                    }
                }
            })
            .child(
                div()
                    .id("git-commit-modal-dialog")
                    .occlude()
                    .w(px(1060.))
                    .h(px(680.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_key_down({
                        let git_s = git_clone.clone();
                        move |event, _, cx| {
                            if event.keystroke.key.as_str() == "escape" {
                                git_s.update(cx, |state, cx| {
                                    state.close_modal(cx);
                                });
                            }
                        }
                    })
                    // Header
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
                                    .gap(px(10.))
                                    .child(div().text_base().text_color(t.primary).child("⎇"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("Git Commit"),
                                    )
                                    .when(is_repo, |this| {
                                        this.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(t.bg_hover)
                                                .border_1()
                                                .border_color(t.border)
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(t.text_secondary)
                                                .child(branch_name),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .child(format!("({} changes)", total_changes)),
                                        )
                                    }),
                            )
                            .child(
                                div().flex().items_center().gap(px(8.)).child(
                                    div()
                                        .id("btn-git-commit-close")
                                        .px(px(8.))
                                        .py(px(4.))
                                        .rounded(px(4.))
                                        .cursor_pointer()
                                        .text_sm()
                                        .text_color(t.text_muted)
                                        .hover(|s| s.text_color(t.text_primary))
                                        .on_click({
                                            let git_s = git_clone.clone();
                                            move |_, _, cx| {
                                                git_s.update(cx, |state, cx| {
                                                    state.close_modal(cx);
                                                });
                                            }
                                        })
                                        .child("✕"),
                                ),
                            ),
                    )
                    // Error banner
                    .when_some(git_error, |this, err| {
                        this.child(render_error_banner(git_clone.clone(), err))
                    })
                    // Body
                    .child(div().flex_1().overflow_hidden().child(if !is_repo {
                        render_not_a_repo_view(git_clone.clone(), is_loading)
                    } else {
                        render_changes_view(ChangesViewProps {
                            git_clone: git_clone.clone(),
                            ws_clone: ws_clone.clone(),
                            status_opt,
                            selected_file,
                            diff_content,
                            commit_input,
                            view_mode,
                            mode_entity: view_mode_entity,
                            selected_staged,
                            selected_unstaged,
                            is_loading,
                        })
                    })),
            )
            .into_any_element()
    }
}
