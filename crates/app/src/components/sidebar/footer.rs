use crate::state::{GitActiveModal, ToastState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement, StatefulInteractiveElement as _, Styled,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use restclient_core::git::GitStatusResult;

pub fn render_sidebar_footer(
    git_branch: &str,
    git_status: Option<&GitStatusResult>,
    is_syncing: bool,
    ws: &Entity<WorkspaceState>,
    toast: &Entity<ToastState>,
) -> AnyElement {
    let t = theme();
    let is_repo = git_status.map(|s| s.is_repo).unwrap_or(false);
    let behind = git_status.map(|s| s.behind).unwrap_or(0);
    let ahead = git_status.map(|s| s.ahead).unwrap_or(0);

    let ws_footer = ws.clone();
    let toast_footer = toast.clone();
    let current_branch_name = git_branch.to_string();

    div()
        .id("sidebar-footer-bar")
        .h(px(28.))
        .w_full()
        .bg(t.bg_sidebar)
        .border_t_1()
        .border_color(t.border)
        .px(px(8.))
        .flex()
        .items_center()
        .justify_between()
        .text_xs()
        // Branch chip & dropdown menu
        .child(if is_repo {
            let ws_menu = ws_footer.clone();
            let toast_menu = toast_footer.clone();
            let active_b = current_branch_name.clone();

            Button::new("sidebar-footer-branch-chip")
                .ghost()
                .compact()
                .flex_1()
                .min_w(px(0.))
                .px(px(4.))
                .py(px(2.))
                .h(px(24.))
                .rounded(px(4.))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .overflow_hidden()
                                .child(
                                    Icon::new(IconName::GitBranch)
                                        .size(px(12.))
                                        .text_color(t.primary),
                                )
                                .child(
                                    div()
                                        .text_color(t.text_secondary)
                                        .font_family(".SystemMonoFont")
                                        .max_w(px(140.))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(git_branch.to_string()),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .when(is_syncing, |s| {
                                    s.child(
                                        Icon::new(IconName::RefreshCw)
                                            .size(px(12.))
                                            .text_color(t.primary),
                                    )
                                })
                                .when(ahead > 0, |s| {
                                    s.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(1.))
                                            .text_color(t.text_muted)
                                            .child(
                                                Icon::new(IconName::ArrowUp).size(px(10.)),
                                            )
                                            .child(
                                                div().text_xs().child(format!("{}", ahead)),
                                            ),
                                    )
                                })
                                .when(behind > 0, |s| {
                                    s.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(1.))
                                            .text_color(t.primary)
                                            .child(
                                                Icon::new(IconName::ArrowDown)
                                                    .size(px(10.)),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(
                                                        gpui::FontWeight::BOLD,
                                                    )
                                                    .child(format!("{}", behind)),
                                            ),
                                    )
                                })
                                .child(
                                    Icon::new(IconName::ChevronDown)
                                        .size(px(10.))
                                        .text_color(t.text_muted),
                                ),
                        ),
                )
                .dropdown_menu(move |menu, window, cx| {
                    let ws_commit = ws_menu.clone();
                    let ws_history = ws_menu.clone();
                    let ws_new_branch = ws_menu.clone();
                    let ws_remotes = ws_menu.clone();
                    let ws_fetch = ws_menu.clone();
                    let ws_pull = ws_menu.clone();
                    let ws_push = ws_menu.clone();
                    let toast_fetch = toast_menu.clone();
                    let toast_pull = toast_menu.clone();
                    let toast_push = toast_menu.clone();

                    let ws_checkout_parent = ws_menu.clone();
                    let toast_checkout_parent = toast_menu.clone();
                    let (branches_list, current_b) = {
                        let ws = ws_menu.read(cx);
                        let branches = if !ws.git_branches.is_empty() {
                            ws.git_branches.clone()
                        } else {
                            restclient_core::git::git_branches(&ws.workspace_path)
                                .unwrap_or_default()
                        };
                        let current =
                            ws.git_branch.clone().unwrap_or_else(|| active_b.clone());
                        (branches, current)
                    };

                    let mut m = menu;

                    // 1. Commit...
                    m = m.item(
                        PopupMenuItem::new("Commit...")
                            .icon(IconName::GitCommitHorizontal)
                            .on_click(move |_, _, cx| {
                                ws_commit.update(cx, |ws, cx| {
                                    ws.open_git_modal(GitActiveModal::Commit, cx);
                                });
                            }),
                    );

                    // 2. History
                    m = m.item(
                        PopupMenuItem::new("History")
                            .icon(IconName::Clock)
                            .on_click(move |_, _, cx| {
                                ws_history.update(cx, |ws, cx| {
                                    ws.open_git_modal(GitActiveModal::History, cx);
                                });
                            }),
                    );

                    // 3. Branch submenu: named after the current branch
                    m = m.submenu(
                        current_b.clone(),
                        window,
                        cx,
                        move |sub, _w, _cx| {
                            let mut s = sub;
                            if branches_list.is_empty() {
                                s = s.item(
                                    PopupMenuItem::new(if current_b.is_empty() {
                                        "No branches".to_string()
                                    } else {
                                        current_b.clone()
                                    })
                                    .checked(true),
                                );
                            } else {
                                for b in &branches_list {
                                    let b_name = b.name.clone();
                                    let is_curr = b.is_current || b.name == current_b;
                                    let ws_b = ws_checkout_parent.clone();
                                    let toast_b = toast_checkout_parent.clone();
                                    let b_target = b_name.clone();

                                    s = s.item(
                                        PopupMenuItem::new(b_name)
                                            .checked(is_curr)
                                            .on_click(move |_, _, cx| {
                                                if !is_curr {
                                                    ws_b.update(cx, |ws, cx| {
                                                        match ws.git_checkout(&b_target, cx) {
                                                            Ok(_) => {
                                                                toast_b.update(cx, |ts, cx| {
                                                                    ts.push_success("Git Checkout", format!("Switched to branch '{}'", b_target), cx);
                                                                });
                                                            }
                                                            Err(err) => {
                                                                toast_b.update(cx, |ts, cx| {
                                                                    ts.push_error("Git Checkout Failed", err, cx);
                                                                });
                                                            }
                                                        }
                                                    });
                                                }
                                            }),
                                    );
                                }
                            }
                            s
                        },
                    );

                    // 4. New Branch...
                    m = m.item(
                        PopupMenuItem::new("New Branch...")
                            .icon(IconName::GitBranchPlus)
                            .on_click(move |_, _, cx| {
                                ws_new_branch.update(cx, |ws, cx| {
                                    ws.open_git_modal(GitActiveModal::NewBranch, cx);
                                });
                            }),
                    );

                    m = m.separator();

                    // 5. Fetch
                    m = m.item(
                        PopupMenuItem::new("Fetch")
                            .icon(IconName::CloudDownload)
                            .on_click(move |_, _, cx| {
                                let ws_s = ws_fetch.clone();
                                let toast_s = toast_fetch.clone();
                                ws_s.update(cx, |ws, cx| {
                                    match ws.git_fetch(cx) {
                                        Ok(msg) => {
                                            let trimmed = msg.trim();
                                            toast_s.update(cx, |ts, cx| {
                                                ts.push_success(
                                                    "Git Fetch",
                                                    if trimmed.is_empty() {
                                                        "Fetched latest references."
                                                            .to_string()
                                                    } else {
                                                        trimmed.to_string()
                                                    },
                                                    cx,
                                                );
                                            });
                                        }
                                        Err(e) => {
                                            toast_s.update(cx, |ts, cx| {
                                                ts.push_error(
                                                    "Git Fetch Failed",
                                                    e,
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                });
                            }),
                    );

                    // 6. Pull
                    m = m.item(
                        PopupMenuItem::new("Pull")
                            .icon(IconName::ArrowDown)
                            .on_click(move |_, _, cx| {
                                let ws_s = ws_pull.clone();
                                let toast_s = toast_pull.clone();
                                ws_s.update(cx, |ws, cx| {
                                    match ws.git_pull(cx) {
                                        Ok(msg) => {
                                            let trimmed = msg.trim();
                                            if trimmed.contains("Already up to date") {
                                                toast_s.update(cx, |ts, cx| {
                                                    ts.push_info(
                                                        "Git Sync",
                                                        "Already up to date.",
                                                        cx,
                                                    );
                                                });
                                            } else {
                                                toast_s.update(cx, |ts, cx| {
                                                    ts.push_success(
                                                        "Git Pull",
                                                        format!(
                                                            "Successfully pulled: {}",
                                                            trimmed
                                                        ),
                                                        cx,
                                                    );
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            toast_s.update(cx, |ts, cx| {
                                                ts.push_error(
                                                    "Git Pull Failed",
                                                    e,
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                });
                            }),
                    );

                    // 7. Push
                    m = m.item(
                        PopupMenuItem::new("Push")
                            .icon(IconName::ArrowUp)
                            .on_click(move |_, _, cx| {
                                let ws_s = ws_push.clone();
                                let toast_s = toast_push.clone();
                                ws_s.update(cx, |ws, cx| {
                                    match ws.git_push(cx) {
                                        Ok(msg) => {
                                            let trimmed = msg.trim();
                                            toast_s.update(cx, |ts, cx| {
                                                ts.push_success(
                                                    "Git Push",
                                                    if trimmed.is_empty() {
                                                        "Pushed commits to remote."
                                                            .to_string()
                                                    } else {
                                                        trimmed.to_string()
                                                    },
                                                    cx,
                                                );
                                            });
                                        }
                                        Err(e) => {
                                            toast_s.update(cx, |ts, cx| {
                                                ts.push_error(
                                                    "Git Push Failed",
                                                    e,
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                });
                            }),
                    );

                    m = m.separator();

                    // 8. Manage Remotes...
                    m = m.item(
                        PopupMenuItem::new("Manage Remotes...")
                            .icon(IconName::Globe)
                            .on_click(move |_, _, cx| {
                                ws_remotes.update(cx, |ws, cx| {
                                    ws.open_git_modal(GitActiveModal::Remotes, cx);
                                });
                            }),
                    );

                    m
                })
                .into_any_element()
        } else {
            let ws_init = ws_footer.clone();
            div()
                .id("sidebar-footer-no-git")
                .flex_1()
                .cursor_pointer()
                .px(px(4.))
                .py(px(2.))
                .rounded(px(4.))
                .hover(|s| s.bg(t.bg_hover))
                .flex()
                .items_center()
                .gap(px(4.))
                .on_click(move |_, _, cx| {
                    ws_init.update(cx, |ws, cx| {
                        ws.open_git_modal(GitActiveModal::Commit, cx);
                    });
                })
                .child(div().text_color(t.text_muted).child("No Git"))
                .into_any_element()
        })
        .into_any_element()
}
