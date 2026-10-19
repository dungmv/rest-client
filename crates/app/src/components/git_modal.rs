use crate::state::{GitActiveModal, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::git::GitFileStatus;

#[derive(IntoElement)]
pub struct GitModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for GitModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ws = self.workspace_state.read(cx);
        let active_modal = ws.active_git_modal;

        match active_modal {
            Some(GitActiveModal::Commit) => GitCommitModal::new(self.workspace_state.clone()).into_any_element(),
            Some(GitActiveModal::History) => GitHistoryModal::new(self.workspace_state.clone()).into_any_element(),
            Some(GitActiveModal::NewBranch) => GitBranchModal::new(self.workspace_state.clone()).into_any_element(),
            Some(GitActiveModal::Remotes) => GitRemotesModal::new(self.workspace_state.clone()).into_any_element(),
            None => div().into_any_element(),
        }
    }
}

// =========================================================================
// 1. GitCommitModal - Dedicated modal for reviewing changes and committing
// =========================================================================

#[derive(IntoElement)]
pub struct GitCommitModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitCommitModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for GitCommitModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ws_clone = self.workspace_state.clone();

        let is_repo = ws.git_status.as_ref().map(|s| s.is_repo).unwrap_or(false);
        let branch_name = ws.git_branch.clone().unwrap_or_else(|| "main".to_string());
        let status_opt = ws.git_status.clone();
        let selected_file = ws.git_selected_file.clone();
        let diff_content = ws.git_diff_content.clone();
        let git_error = ws.git_error.clone();

        let staged_count = status_opt.as_ref().map(|s| s.staged_files.len()).unwrap_or(0);
        let unstaged_count = status_opt.as_ref().map(|s| s.unstaged_files.len()).unwrap_or(0);
        let untracked_count = status_opt.as_ref().map(|s| s.untracked_files.len()).unwrap_or(0);
        let total_changes = staged_count + unstaged_count + untracked_count;

        let commit_input_holder = window.use_keyed_state(
            gpui::SharedString::from("git-commit-msg-input"),
            cx,
            |window, cx| {
                let inp = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("Commit message (e.g. feat: add login endpoint)")
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
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .when(is_repo, |this| {
                                        let ws_refresh = ws_clone.clone();
                                        this.child(
                                            div()
                                                .id("btn-git-refresh-status")
                                                .px(px(8.))
                                                .py(px(4.))
                                                .rounded(px(4.))
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(t.text_secondary)
                                                .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                                .on_click(move |_, _, cx| {
                                                    ws_refresh.update(cx, |ws, cx| {
                                                        ws.refresh_git_status(cx);
                                                    });
                                                })
                                                .child("⟳ Refresh"),
                                        )
                                    })
                                    .child(
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
                                                let ws_s = ws_clone.clone();
                                                move |_, _, cx| {
                                                    ws_s.update(cx, |state, cx| {
                                                        state.close_git_modal(cx);
                                                    });
                                                }
                                            })
                                            .child("✕"),
                                    ),
                            ),
                    )
                    // Error banner
                    .when_some(git_error, |this, err| {
                        this.child(render_error_banner(ws_clone.clone(), err))
                    })
                    // Body
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(if !is_repo {
                                render_not_a_repo_view(ws_clone.clone())
                            } else {
                                render_changes_view(
                                    ws_clone.clone(),
                                    status_opt,
                                    selected_file,
                                    diff_content,
                                    commit_input,
                                    view_mode,
                                    view_mode_entity,
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}

// =========================================================================
// 2. GitHistoryModal - Dedicated modal for inspecting commit log & diffs
// =========================================================================

#[derive(IntoElement)]
pub struct GitHistoryModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitHistoryModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for GitHistoryModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ws_clone = self.workspace_state.clone();

        let is_repo = ws.git_status.as_ref().map(|s| s.is_repo).unwrap_or(false);
        let branch_name = ws.git_branch.clone().unwrap_or_else(|| "main".to_string());
        let history = ws.git_history.clone();
        let selected_commit = ws.git_selected_commit.clone();
        let git_error = ws.git_error.clone();

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
            .id("git-history-modal-overlay")
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
            .child(
                div()
                    .id("git-history-modal-dialog")
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
                                    .child(
                                        div()
                                            .text_base()
                                            .text_color(t.primary)
                                            .child("◷"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("Git History"),
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
                                                .child(format!("({} commits)", history.len())),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .id("btn-git-history-close")
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
                            ),
                    )
                    // Error banner
                    .when_some(git_error, |this, err| {
                        this.child(render_error_banner(ws_clone.clone(), err))
                    })
                    // Body
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(if !is_repo {
                                render_not_a_repo_view(ws_clone.clone())
                            } else {
                                render_history_view(ws_clone.clone(), history, selected_commit, view_mode, view_mode_entity)
                            }),
                    ),
            )
            .into_any_element()
    }
}

// =========================================================================
// 3. GitBranchModal - Dedicated modal for creating and managing branches
// =========================================================================

#[derive(IntoElement)]
pub struct GitBranchModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitBranchModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
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

// =========================================================================
// 4. GitRemotesModal - Dedicated modal for viewing and managing remotes
// =========================================================================

#[derive(IntoElement)]
pub struct GitRemotesModal {
    workspace_state: Entity<WorkspaceState>,
}

impl GitRemotesModal {
    pub fn new(workspace_state: Entity<WorkspaceState>) -> Self {
        Self { workspace_state }
    }
}

impl RenderOnce for GitRemotesModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let ws = self.workspace_state.read(cx);
        let ws_clone = self.workspace_state.clone();

        let is_repo = ws.git_status.as_ref().map(|s| s.is_repo).unwrap_or(false);
        let remotes = ws.git_remotes.clone();
        let git_error = ws.git_error.clone();

        let remote_name_holder = window.use_keyed_state(
            gpui::SharedString::from("git-remote-name-input"),
            cx,
            |window, cx| {
                let inp = cx.new(|cx| {
                    let mut s = InputState::new(window, cx);
                    s.set_value("origin", window, cx);
                    s
                });
                (inp, ())
            },
        );
        let remote_name_input = remote_name_holder.read(cx).0.clone();

        let remote_url_holder = window.use_keyed_state(
            gpui::SharedString::from("git-remote-url-input"),
            cx,
            |window, cx| {
                let inp = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("https://github.com/owner/repository.git")
                });
                (inp, ())
            },
        );
        let remote_url_input = remote_url_holder.read(cx).0.clone();

        div()
            .id("git-remotes-modal-overlay")
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
            .child(
                div()
                    .id("git-remotes-modal-dialog")
                    .occlude()
                    .w(px(480.))
                    .h(px(340.))
                    .rounded(px(8.))
                    .bg(t.bg_card)
                    .border_1()
                    .border_color(t.border)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
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
                                            .child("⇄"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child("Manage Remotes"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("btn-git-remotes-close")
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
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(if !is_repo {
                                render_not_a_repo_view(ws_clone.clone())
                            } else {
                                render_remotes_view(
                                    ws_clone.clone(),
                                    remotes,
                                    remote_name_input,
                                    remote_url_input,
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}

// =========================================================================
// Reusable sub-views
// =========================================================================

fn render_error_banner(ws_clone: Entity<WorkspaceState>, err: String) -> gpui::AnyElement {
    let t = theme();
    let ws_err = ws_clone.clone();

    div()
        .px(px(16.))
        .py(px(8.))
        .bg(gpui::Hsla { h: 0.0, s: 0.7, l: 0.45, a: 0.2 })
        .border_b_1()
        .border_color(t.status_5xx)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_xs()
                .text_color(t.status_5xx)
                .child(format!("⚠️ Git: {}", err)),
        )
        .child(
            div()
                .id("btn-dismiss-git-error")
                .cursor_pointer()
                .text_xs()
                .text_color(t.text_muted)
                .hover(|s| s.text_color(t.text_primary))
                .on_click(move |_, _, cx| {
                    ws_err.update(cx, |ws, cx| {
                        ws.git_error = None;
                        cx.notify();
                    });
                })
                .child("✕"),
        )
        .into_any_element()
}

fn render_not_a_repo_view(ws_clone: Entity<WorkspaceState>) -> gpui::AnyElement {
    let t = theme();
    let ws_init = ws_clone.clone();

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(14.))
        .child(
            div()
                .text_2xl()
                .text_color(t.text_muted)
                .child("📦"),
        )
        .child(
            div()
                .text_base()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(t.text_primary)
                .child("Not a Git Repository"),
        )
        .child(
            div()
                .max_w(px(400.))
                .text_center()
                .text_xs()
                .text_color(t.text_muted)
                .child("This workspace directory does not have Git version control initialized yet."),
        )
        .child(
            div()
                .id("btn-git-init")
                .mt(px(8.))
                .px(px(16.))
                .py(px(8.))
                .rounded(px(6.))
                .bg(t.primary)
                .cursor_pointer()
                .text_xs()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(gpui::rgb(0xffffff))
                .hover(|s| s.opacity(0.9))
                .on_click(move |_, _, cx| {
                    ws_init.update(cx, |ws, cx| {
                        let _ = ws.git_init_repo(cx);
                    });
                })
                .child("✨ Initialize Git Repository"),
        )
        .into_any_element()
}

fn render_changes_view(
    ws_clone: Entity<WorkspaceState>,
    status_opt: Option<restclient_core::git::GitStatusResult>,
    selected_file: Option<(String, bool)>,
    diff_content: Option<String>,
    commit_input: Entity<InputState>,
    view_mode: DiffViewMode,
    mode_entity: Entity<DiffViewMode>,
) -> gpui::AnyElement {
    let t = theme();

    let staged_files = status_opt.as_ref().map(|s| s.staged_files.clone()).unwrap_or_default();
    let unstaged_files = status_opt.as_ref().map(|s| s.unstaged_files.clone()).unwrap_or_default();
    let untracked_files = status_opt.as_ref().map(|s| s.untracked_files.clone()).unwrap_or_default();

    let total_staged = staged_files.len();

    div()
        .size_full()
        .flex()
        // Left Column: Staging lists and Commit input (width: 350px)
        .child(
            div()
                .w(px(350.))
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
                        .child(
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
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(t.text_secondary)
                                                .child(format!("Staged ({})", total_staged)),
                                        )
                                        .when(total_staged > 0, |this| {
                                            let ws_unstage_all = ws_clone.clone();
                                            this.child(
                                                div()
                                                    .id("btn-unstage-all")
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(t.primary)
                                                    .hover(|s| s.underline())
                                                    .on_click(move |_, _, cx| {
                                                        ws_unstage_all.update(cx, |ws, cx| {
                                                            ws.git_unstage_all(cx);
                                                        });
                                                    })
                                                    .child("Unstage All"),
                                            )
                                        }),
                                )
                                .children(staged_files.into_iter().map(|item| {
                                    let ws_sel = ws_clone.clone();
                                    let ws_act = ws_clone.clone();
                                    let p = item.path.clone();
                                    let p_sel = item.path.clone();
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
                                            ws_sel.update(cx, |ws, cx| {
                                                ws.select_git_file(p_sel.clone(), true, cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .overflow_hidden()
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
                                            // Unstage button (-)
                                            div()
                                                .id(format!("btn-unstage-{}", item.path))
                                                .cursor_pointer()
                                                .p(px(2.))
                                                .rounded(px(3.))
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .hover(|s| s.bg(t.bg_card).text_color(t.text_primary))
                                                .on_click(move |_, _, cx| {
                                                    cx.stop_propagation();
                                                    ws_act.update(cx, |ws, cx| {
                                                        ws.git_unstage_file(&p, cx);
                                                    });
                                                })
                                                .child("⊖"),
                                        )
                                })),
                        )
                        // Working Changes Section
                        .child(
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
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(t.text_secondary)
                                                .child(format!("Changes ({})", unstaged_files.len() + untracked_files.len())),
                                        )
                                        .when(!unstaged_files.is_empty() || !untracked_files.is_empty(), |this| {
                                            let ws_stage_all = ws_clone.clone();
                                            this.child(
                                                div()
                                                    .id("btn-stage-all")
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(t.primary)
                                                    .hover(|s| s.underline())
                                                    .on_click(move |_, _, cx| {
                                                        ws_stage_all.update(cx, |ws, cx| {
                                                            ws.git_stage_all(cx);
                                                        });
                                                    })
                                                    .child("Stage All"),
                                            )
                                        }),
                                )
                                .children(unstaged_files.into_iter().chain(untracked_files).map(|item| {
                                    let ws_sel = ws_clone.clone();
                                    let ws_stage = ws_clone.clone();
                                    let ws_disc = ws_clone.clone();
                                    let p = item.path.clone();
                                    let p_disc = item.path.clone();
                                    let p_sel = item.path.clone();
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
                                            ws_sel.update(cx, |ws, cx| {
                                                ws.select_git_file(p_sel.clone(), false, cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .overflow_hidden()
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
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.))
                                                // Discard button (↺)
                                                .child(
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
                                                            ws_disc.update(cx, |ws, cx| {
                                                                ws.git_discard_file(&p_disc, cx);
                                                            });
                                                        })
                                                        .child("↺"),
                                                )
                                                // Stage button (+)
                                                .child(
                                                    div()
                                                        .id(format!("btn-stage-{}", item.path))
                                                        .cursor_pointer()
                                                        .p(px(2.))
                                                        .rounded(px(3.))
                                                        .text_xs()
                                                        .text_color(t.text_muted)
                                                        .hover(|s| s.bg(t.bg_card).text_color(t.primary))
                                                        .on_click(move |_, _, cx| {
                                                            cx.stop_propagation();
                                                            ws_stage.update(cx, |ws, cx| {
                                                                ws.git_stage_file(&p, cx);
                                                            });
                                                        })
                                                        .child("⊕"),
                                                ),
                                        )
                                })),
                        ),
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
                            let ws_commit = ws_clone.clone();
                            let inp = commit_input.clone();
                            let can_commit = total_staged > 0;

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
                                        .text_color(gpui::rgb(0xffffff))
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
                                    let ws_s = ws_commit.clone();
                                    let inp_clear = inp.clone();
                                    ws_s.update(cx, |ws, cx| {
                                        if ws.git_commit(&msg, cx).is_ok() {
                                            inp_clear.update(cx, |inp_st, cx| {
                                                inp_st.set_value("", window, cx);
                                            });
                                            ws.close_git_modal(cx);
                                        }
                                    });
                                })
                                .child(format!("✓ Commit Changes ({} staged)", total_staged))
                        }),
                ),
        )
        // Right Column: Diff Viewer (Flex-1)
        .child(
            div()
                .flex_1()
                .h_full()
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

fn render_history_view(
    ws_clone: Entity<WorkspaceState>,
    history: Vec<restclient_core::git::GitCommitInfo>,
    selected_commit: Option<restclient_core::git::GitCommitDetail>,
    view_mode: DiffViewMode,
    mode_entity: Entity<DiffViewMode>,
) -> gpui::AnyElement {
    let t = theme();

    div()
        .size_full()
        .flex()
        // Left Column: Commit Log (width: 350px)
        .child(
            div()
                .w(px(350.))
                .h_full()
                .border_r_1()
                .border_color(t.border)
                .bg(t.bg_sidebar)
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_1()
                        .overflow_y_scrollbar()
                        .children(history.into_iter().map(|item| {
                            let ws_sel = ws_clone.clone();
                            let h = item.hash.clone();
                            let is_selected = selected_commit.as_ref().map(|c| c.commit.hash == item.hash).unwrap_or(false);

                            div()
                                .id(format!("commit-item-{}", item.hash))
                                .px(px(12.))
                                .py(px(8.))
                                .border_b_1()
                                .border_color(t.border)
                                .cursor_pointer()
                                .when(is_selected, |s| s.bg(t.bg_hover))
                                .hover(|s| s.bg(t.bg_hover))
                                .on_click(move |_, _, cx| {
                                    ws_sel.update(cx, |ws, cx| {
                                        ws.select_git_commit(&h, cx);
                                    });
                                })
                                .flex()
                                .flex_col()
                                .gap(px(3.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .px(px(4.))
                                                .py(px(1.))
                                                .rounded(px(3.))
                                                .bg(t.bg_card)
                                                .border_1()
                                                .border_color(t.border)
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(t.primary)
                                                .child(item.short_hash),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .child(item.relative_date),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(t.text_primary)
                                        .line_clamp(2)
                                        .child(item.message),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child(format!("👤 {}", item.author_name)),
                                )
                        })),
                ),
        )
        // Right Column: Commit Details & Diff (Flex-1)
        .child({
            if let Some(commit_detail) = selected_commit {
                div()
                    .flex_1()
                    .h_full()
                    .bg(t.bg_app)
                    .flex()
                    .flex_col()
                    .child(
                        // Header details
                        div()
                            .p(px(14.))
                            .border_b_1()
                            .border_color(t.border)
                            .bg(t.bg_card)
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
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(t.text_primary)
                                            .child(commit_detail.commit.message.clone()),
                                    )
                                    .child(render_diff_view_toggle(view_mode, mode_entity.clone())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child(format!("Commit: {}", commit_detail.commit.hash))
                                    .child(format!("Author: {} <{}>", commit_detail.commit.author_name, commit_detail.commit.author_email))
                                    .child(format!("Date: {}", commit_detail.commit.relative_date)),
                            )
                            .when(!commit_detail.changed_files.is_empty(), |this| {
                                this.child(
                                    div()
                                        .mt(px(4.))
                                        .flex()
                                        .flex_wrap()
                                        .gap(px(6.))
                                        .children(commit_detail.changed_files.iter().map(|f| {
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(t.bg_hover)
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(t.text_secondary)
                                                .child(f.clone())
                                        })),
                                )
                            }),
                    )
                    .child(
                        // Commit diff
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .p(px(8.))
                            .child(render_diff_content(Some(commit_detail.diff), view_mode)),
                    )
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .h_full()
                    .bg(t.bg_app)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(t.text_muted)
                    .child("Select a commit from the list to view changes.")
                    .into_any_element()
            }
        })
        .into_any_element()
}

fn render_new_branch_view(
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

fn render_remotes_view(
    ws_clone: Entity<WorkspaceState>,
    remotes: Vec<restclient_core::git::GitRemote>,
    remote_name_input: Entity<InputState>,
    remote_url_input: Entity<InputState>,
) -> gpui::AnyElement {
    let t = theme();

    div()
        .size_full()
        .flex()
        .flex_col()
        // Top/Middle: Remotes List (Scrollable)
        .child(
            div()
                .flex_1()
                .overflow_y_scrollbar()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .when(remotes.is_empty(), |this| {
                    this.child(
                        div()
                            .py(px(24.))
                            .text_center()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child("No git remotes configured yet."),
                    )
                })
                .children(remotes.into_iter().map(|r| {
                    let ws_del = ws_clone.clone();
                    let r_name = r.name.clone();

                    div()
                        .id(format!("remote-row-{}", r.name))
                        .px(px(10.))
                        .py(px(8.))
                        .rounded(px(6.))
                        .bg(t.bg_sidebar)
                        .border_1()
                        .border_color(t.border)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .font_family(".SystemMonoFont")
                                        .text_color(t.primary)
                                        .child(r.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .font_family(".SystemMonoFont")
                                        .text_color(t.text_secondary)
                                        .truncate()
                                        .child(r.fetch_url.clone()),
                                ),
                        )
                        .child(
                            div()
                                .id(format!("btn-remove-remote-{}", r.name))
                                .px(px(8.))
                                .py(px(4.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .text_xs()
                                .text_color(t.text_muted)
                                .hover(|s| s.bg(t.bg_card).text_color(t.status_5xx))
                                .on_click(move |_, _, cx| {
                                    ws_del.update(cx, |ws, cx| {
                                        let _ = ws.git_remove_remote(&r_name, cx);
                                    });
                                })
                                .child("Remove"),
                        )
                })),
        )
        // Bottom: Add Remote Form
        .child(
            div()
                .border_t_1()
                .border_color(t.border)
                .p(px(12.))
                .bg(t.bg_sidebar)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .w(px(84.))
                        .bg(t.bg_card)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(6.))
                        .p(px(4.))
                        .child(Input::new(&remote_name_input)),
                )
                .child(
                    div()
                        .flex_1()
                        .bg(t.bg_card)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(6.))
                        .p(px(4.))
                        .child(Input::new(&remote_url_input)),
                )
                .child({
                    let ws_add = ws_clone.clone();
                    let inp_n = remote_name_input.clone();
                    let inp_u = remote_url_input.clone();
                    div()
                        .id("btn-add-remote-action")
                        .px(px(12.))
                        .h(px(30.))
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
                            let name = inp_n.read(cx).value().trim().to_string();
                            let url = inp_u.read(cx).value().trim().to_string();
                            if name.is_empty() || url.is_empty() {
                                return;
                            }
                            let ws_s = ws_add.clone();
                            let inp_u_clear = inp_u.clone();
                            ws_s.update(cx, |ws, cx| {
                                if ws.git_add_remote(&name, &url, cx).is_ok() {
                                    inp_u_clear.update(cx, |inp_st, cx| {
                                        inp_st.set_value("", window, cx);
                                    });
                                }
                            });
                        })
                        .child("+ Add")
                }),
        )
        .into_any_element()
}

fn render_status_badge(status: GitFileStatus) -> gpui::AnyElement {
    let color = match status {
        GitFileStatus::Modified => gpui::rgb(0xfacc15),
        GitFileStatus::Added | GitFileStatus::Untracked => gpui::rgb(0x4ade80),
        GitFileStatus::Deleted => gpui::rgb(0xef4444),
        _ => gpui::rgb(0x9ca3af),
    };

    div()
        .w(px(16.))
        .h(px(16.))
        .rounded(px(3.))
        .bg(gpui::Hsla { a: 0.15, ..color.into() })
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(color)
        .child(status.code())
        .into_any_element()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiffViewMode {
    Split,
    Unified,
}

#[derive(Clone, PartialEq, Eq)]
pub enum DiffLineKind {
    Normal,
    Added,
    Deleted,
}

#[derive(Clone)]
pub struct DiffSideLine {
    pub line_no: Option<usize>,
    pub content: String,
    pub kind: DiffLineKind,
}

#[derive(Clone)]
pub enum SideBySideRow {
    HunkHeader(String),
    Diff {
        left: Option<DiffSideLine>,
        right: Option<DiffSideLine>,
    },
}

fn parse_side_by_side_diff(diff: &str) -> Vec<SideBySideRow> {
    let mut rows = Vec::new();
    let mut old_line = 1usize;
    let mut new_line = 1usize;
    let mut pending_deleted: Vec<String> = Vec::new();
    let mut pending_added: Vec<String> = Vec::new();

    let flush_pending = |deleted: &mut Vec<String>, added: &mut Vec<String>, old_cur: &mut usize, new_cur: &mut usize, rows: &mut Vec<SideBySideRow>| {
        let max_len = deleted.len().max(added.len());
        for i in 0..max_len {
            let left = if i < deleted.len() {
                let ln = *old_cur;
                *old_cur += 1;
                Some(DiffSideLine {
                    line_no: Some(ln),
                    content: deleted[i].clone(),
                    kind: DiffLineKind::Deleted,
                })
            } else {
                None
            };

            let right = if i < added.len() {
                let ln = *new_cur;
                *new_cur += 1;
                Some(DiffSideLine {
                    line_no: Some(ln),
                    content: added[i].clone(),
                    kind: DiffLineKind::Added,
                })
            } else {
                None
            };

            rows.push(SideBySideRow::Diff { left, right });
        }
        deleted.clear();
        added.clear();
    };

    for line in diff.lines() {
        if line.starts_with("diff --git") || line.starts_with("index ") || line.starts_with("---") || line.starts_with("+++") {
            continue;
        }

        if line.starts_with("@@") {
            flush_pending(&mut pending_deleted, &mut pending_added, &mut old_line, &mut new_line, &mut rows);
            if let Some(hunk_parts) = line.split("@@").nth(1) {
                let parts: Vec<&str> = hunk_parts.split_whitespace().collect();
                for p in parts {
                    if p.starts_with('-') {
                        let num = p[1..].split(',').next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(1);
                        old_line = num;
                    } else if p.starts_with('+') {
                        let num = p[1..].split(',').next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(1);
                        new_line = num;
                    }
                }
            }
            rows.push(SideBySideRow::HunkHeader(line.to_string()));
        } else if line.starts_with('-') {
            pending_deleted.push(line[1..].to_string());
        } else if line.starts_with('+') {
            pending_added.push(line[1..].to_string());
        } else {
            flush_pending(&mut pending_deleted, &mut pending_added, &mut old_line, &mut new_line, &mut rows);
            let content = if line.starts_with(' ') { &line[1..] } else { line };
            let ln_old = old_line;
            let ln_new = new_line;
            old_line += 1;
            new_line += 1;
            rows.push(SideBySideRow::Diff {
                left: Some(DiffSideLine {
                    line_no: Some(ln_old),
                    content: content.to_string(),
                    kind: DiffLineKind::Normal,
                }),
                right: Some(DiffSideLine {
                    line_no: Some(ln_new),
                    content: content.to_string(),
                    kind: DiffLineKind::Normal,
                }),
            });
        }
    }

    flush_pending(&mut pending_deleted, &mut pending_added, &mut old_line, &mut new_line, &mut rows);
    rows
}

fn render_diff_view_toggle(view_mode: DiffViewMode, view_mode_entity: Entity<DiffViewMode>) -> gpui::AnyElement {
    let t = theme();
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .p(px(2.))
        .rounded(px(4.))
        .bg(t.bg_sidebar)
        .border_1()
        .border_color(t.border)
        .child({
            let st = view_mode_entity.clone();
            div()
                .id("btn-diff-split")
                .px(px(6.))
                .py(px(2.))
                .rounded(px(3.))
                .cursor_pointer()
                .text_xs()
                .when(view_mode == DiffViewMode::Split, |s| s.bg(t.bg_card).text_color(t.text_primary).font_weight(gpui::FontWeight::SEMIBOLD))
                .when(view_mode != DiffViewMode::Split, |s| s.text_color(t.text_muted).hover(|h| h.text_color(t.text_secondary)))
                .on_click(move |_, _, cx| {
                    st.update(cx, |m, cx| {
                        *m = DiffViewMode::Split;
                        cx.notify();
                    });
                })
                .child("◫ Split")
        })
        .child({
            let st = view_mode_entity.clone();
            div()
                .id("btn-diff-unified")
                .px(px(6.))
                .py(px(2.))
                .rounded(px(3.))
                .cursor_pointer()
                .text_xs()
                .when(view_mode == DiffViewMode::Unified, |s| s.bg(t.bg_card).text_color(t.text_primary).font_weight(gpui::FontWeight::SEMIBOLD))
                .when(view_mode != DiffViewMode::Unified, |s| s.text_color(t.text_muted).hover(|h| h.text_color(t.text_secondary)))
                .on_click(move |_, _, cx| {
                    st.update(cx, |m, cx| {
                        *m = DiffViewMode::Unified;
                        cx.notify();
                    });
                })
                .child("▤ Unified")
        })
        .into_any_element()
}

fn render_diff_content(diff_content: Option<String>, view_mode: DiffViewMode) -> gpui::AnyElement {
    let t = theme();

    match diff_content {
        Some(diff) if !diff.trim().is_empty() => {
            let green_bg = gpui::Hsla { h: 142.0 / 360.0, s: 0.7, l: 0.45, a: 0.15 };
            let green_text: gpui::Hsla = gpui::rgb(0x4ade80).into();
            let red_bg = gpui::Hsla { h: 0.0, s: 0.7, l: 0.45, a: 0.15 };
            let red_text: gpui::Hsla = gpui::rgb(0xef4444).into();
            let gray_text = t.text_muted;

            match view_mode {
                DiffViewMode::Split => {
                    let rows = parse_side_by_side_diff(&diff);
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .font_family(".SystemMonoFont")
                        .text_xs()
                        .children(rows.into_iter().map(|row| {
                            match row {
                                SideBySideRow::HunkHeader(hunk) => {
                                    div()
                                        .w_full()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .bg(t.bg_sidebar)
                                        .border_y_1()
                                        .border_color(t.border)
                                        .text_color(t.primary)
                                        .child(hunk)
                                }
                                SideBySideRow::Diff { left, right } => {
                                    div()
                                        .w_full()
                                        .flex()
                                        .border_b_1()
                                        .border_color(gpui::Hsla { a: 0.25, ..t.border })
                                        // Left column (Old)
                                        .child({
                                            let (bg, txt_color, line_num, txt) = match left {
                                                Some(l) => {
                                                    let (bg, col) = match l.kind {
                                                        DiffLineKind::Deleted => (Some(red_bg), red_text),
                                                        DiffLineKind::Added => (Some(green_bg), green_text),
                                                        DiffLineKind::Normal => (None, gray_text),
                                                    };
                                                    (bg, col, l.line_no.map(|n| n.to_string()).unwrap_or_default(), l.content)
                                                }
                                                None => (Some(gpui::Hsla { a: 0.15, ..t.bg_card }), gray_text, String::new(), String::new()),
                                            };

                                            div()
                                                .flex_1()
                                                .min_w(px(0.))
                                                .border_r_1()
                                                .border_color(t.border)
                                                .when_some(bg, |this, b| this.bg(b))
                                                .flex()
                                                .items_start()
                                                .overflow_hidden()
                                                .child(
                                                    div()
                                                        .w(px(34.))
                                                        .flex_shrink_0()
                                                        .px(px(4.))
                                                        .py(px(1.))
                                                        .text_right()
                                                        .text_color(t.text_muted)
                                                        .child(line_num),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w(px(0.))
                                                        .px(px(6.))
                                                        .py(px(1.))
                                                        .text_color(txt_color)
                                                        .overflow_hidden()
                                                        .child(txt),
                                                )
                                        })
                                        // Right column (New)
                                        .child({
                                            let (bg, txt_color, line_num, txt) = match right {
                                                Some(r) => {
                                                    let (bg, col) = match r.kind {
                                                        DiffLineKind::Added => (Some(green_bg), green_text),
                                                        DiffLineKind::Deleted => (Some(red_bg), red_text),
                                                        DiffLineKind::Normal => (None, t.text_primary),
                                                    };
                                                    (bg, col, r.line_no.map(|n| n.to_string()).unwrap_or_default(), r.content)
                                                }
                                                None => (Some(gpui::Hsla { a: 0.15, ..t.bg_card }), gray_text, String::new(), String::new()),
                                            };

                                            div()
                                                .flex_1()
                                                .min_w(px(0.))
                                                .when_some(bg, |this, b| this.bg(b))
                                                .flex()
                                                .items_start()
                                                .overflow_hidden()
                                                .child(
                                                    div()
                                                        .w(px(34.))
                                                        .flex_shrink_0()
                                                        .px(px(4.))
                                                        .py(px(1.))
                                                        .text_right()
                                                        .text_color(t.text_muted)
                                                        .child(line_num),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w(px(0.))
                                                        .px(px(6.))
                                                        .py(px(1.))
                                                        .text_color(txt_color)
                                                        .overflow_hidden()
                                                        .child(txt),
                                                )
                                        })
                                }
                            }
                        }))
                        .into_any_element()
                }
                DiffViewMode::Unified => {
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .font_family(".SystemMonoFont")
                        .text_xs()
                        .children(diff.lines().map(|line| {
                            let (bg, text_color) = if line.starts_with('+') && !line.starts_with("+++") {
                                (Some(green_bg), green_text)
                            } else if line.starts_with('-') && !line.starts_with("---") {
                                (Some(red_bg), red_text)
                            } else if line.starts_with("@@") {
                                (None, t.primary)
                            } else {
                                (None, gray_text)
                            };

                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .when_some(bg, |this, b| this.bg(b))
                                .text_color(text_color)
                                .child(line.to_string())
                        }))
                        .into_any_element()
                }
            }
        }
        Some(_) => {
            div()
                .p(px(12.))
                .text_xs()
                .text_color(t.text_muted)
                .child("File has no modifications compared to HEAD.")
                .into_any_element()
        }
        None => {
            div()
                .p(px(12.))
                .text_xs()
                .text_color(t.text_muted)
                .child("Select a modified or staged file to view its git diff.")
                .into_any_element()
        }
    }
}
