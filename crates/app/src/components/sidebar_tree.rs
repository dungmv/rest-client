use crate::components::status_badge::{MethodBadge, StatusCodeBadge};
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, App, AppContext as _, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::scroll::ScrollableElement as _;

#[derive(IntoElement)]
pub struct SidebarTree {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
}

impl SidebarTree {
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

impl RenderOnce for SidebarTree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let (git_branch, query, filter_query_raw, folders, requests) = {
            let ws = self.workspace_state.read(cx);
            (
                ws.git_branch.clone().unwrap_or_else(|| "main".to_string()),
                ws.filter_query.to_lowercase(),
                ws.filter_query.clone(),
                ws.folders.clone(),
                ws.requests.clone(),
            )
        };
        let active_req_id = self.editor_state.read(cx).active_request.as_ref().map(|r| r.id.clone());
        let has_nothing = folders.is_empty() && requests.is_empty();

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();

        let ws_search = ws_clone.clone();
        let search_holder = window.use_keyed_state(
            gpui::SharedString::from("sidebar-search-input-holder"),
            cx,
            |window, cx| {
                let search_input = cx.new(|cx| {
                    gpui_component::input::InputState::new(window, cx).placeholder("Search...")
                });
                let s_in = search_input.clone();
                let sub = cx.subscribe_in(&search_input, window, move |_, _, event, _, cx| {
                    if let gpui_component::input::InputEvent::Change = event {
                        let q = s_in.read(cx).value().to_string();
                        ws_search.update(cx, |state, cx| {
                            state.set_filter_query(q, cx);
                        });
                    }
                });
                (search_input, sub)
            },
        );

        let search_input = search_holder.read(cx).0.clone();
        if search_input.read(cx).value().as_str() != filter_query_raw {
            search_input.update(cx, |inp, cx| {
                inp.set_value(filter_query_raw.clone(), window, cx);
            });
        }

        div()
            .w_full()
            .h_full()
            .bg(t.bg_sidebar)
            .border_r_1()
            .border_color(t.border)
            .flex()
            .flex_col()
            .justify_between()
            // Top section: search bar + overflow menu
            .child(
                div()
                    .p(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex_1()
                            .h(px(28.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .bg(t.bg_input)
                            .border_1()
                            .border_color(t.border_subtle)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t.text_muted)
                                    .child("🔍"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(
                                        gpui_component::input::Input::new(&search_input)
                                            .id("sidebar-search-input")
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .cleanable(true),
                                    ),
                            ),
                    )
                    // Overflow menu (⋮) - create folder
                    .child({
                        let ws_s = ws_clone.clone();
                        div()
                            .id("btn-sidebar-overflow")
                            .w(px(28.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(t.bg_hover))
                            .text_color(t.text_secondary)
                            .on_click(move |_, _, cx| {
                                ws_s.update(cx, |state, cx| {
                                    state.create_folder("new-folder".to_string(), cx);
                                });
                            })
                            .child("📁+")
                    }),
            )
            // Middle section: Folders & Requests tree
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .px(px(6.))
                    .py(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .when(has_nothing, |this| {
                        let ws_new = ws_clone.clone();
                        let ed_new = ed_clone.clone();
                        this.child(
                            div()
                                .py(px(32.))
                                .px(px(12.))
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .child("No saved requests yet"),
                                )
                                .child(
                                    div()
                                        .id("btn-sidebar-empty-new-req")
                                        .px(px(10.))
                                        .py(px(5.))
                                        .rounded(px(5.))
                                        .bg(t.bg_card)
                                        .border_1()
                                        .border_color(t.border)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(t.text_secondary)
                                        .on_click(move |_, _, cx| {
                                            let req = ws_new.update(cx, |state, cx| {
                                                state.create_request(None, cx)
                                            });
                                            ed_new.update(cx, |state, cx| {
                                                state.set_active_request(req, true, cx);
                                            });
                                        })
                                        .child("+ New Request"),
                                ),
                        )
                    })
                    .children(folders.iter().map(|folder| {
                        let f_id = folder.id.clone();
                        let f_id_add = folder.id.clone();
                        let f_id_del = folder.id.clone();
                        let ws_f_add = ws_clone.clone();
                        let ed_f_add = ed_clone.clone();
                        let ws_f_del = ws_clone.clone();
                        let folder_requests: Vec<_> = requests
                            .iter()
                            .filter(|r| {
                                r.folder_id.as_deref() == Some(&f_id)
                                    && (query.is_empty()
                                        || r.name.to_lowercase().contains(&query)
                                        || r.url.to_lowercase().contains(&query))
                            })
                            .cloned()
                            .collect();

                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            // Folder header
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .text_color(t.text_secondary)
                                    .text_xs()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(div().child("⌄"))
                                            .child(div().child("📁"))
                                            .child(
                                                div()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_primary)
                                                    .child(folder.name.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .id(format!("folder-add-{}", f_id))
                                                    .px(px(4.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover).text_color(t.text_primary))
                                                    .text_color(t.text_muted)
                                                    .on_click(move |_, _, cx| {
                                                        let req = ws_f_add.update(cx, |state, cx| {
                                                            state.create_request(Some(f_id_add.clone()), cx)
                                                        });
                                                        ed_f_add.update(cx, |state, cx| {
                                                            state.set_active_request(req, true, cx);
                                                        });
                                                    })
                                                    .child("+"),
                                            )
                                            .child(
                                                div()
                                                    .id(format!("folder-del-{}", f_id))
                                                    .px(px(4.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(t.bg_hover).text_color(t.status_4xx))
                                                    .text_color(t.text_muted)
                                                    .on_click(move |_, _, cx| {
                                                        ws_f_del.update(cx, |state, cx| {
                                                            state.delete_folder(&f_id_del, cx);
                                                        });
                                                    })
                                                    .child("×"),
                                            ),
                                    ),
                            )
                            // Folder items
                            .children(folder_requests.into_iter().map(|req| {
                                let is_selected = active_req_id.as_ref() == Some(&req.id);
                                let req_for_click = req.clone();
                                let req_id_del = req.id.clone();
                                let ed_s = ed_clone.clone();
                                let ws_del = ws_clone.clone();
                                let ed_del = ed_clone.clone();

                                div()
                                    .id(format!("req-item-{}", req.id))
                                    .ml(px(14.))
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(if is_selected {
                                        t.border
                                    } else {
                                        t.transparent
                                    })
                                    .bg(if is_selected {
                                        t.bg_selected
                                    } else {
                                        t.transparent
                                    })
                                    .hover(|s| s.bg(t.bg_hover))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .on_click(move |ev, _, cx| {
                                        let pin = ev.click_count() > 1;
                                        ed_s.update(cx, |state, cx| {
                                            state.set_active_request(req_for_click.clone(), pin, cx);
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .overflow_hidden()
                                            .child(MethodBadge::new(req.method))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(if is_selected { t.text_primary } else { t.text_secondary })
                                                    .truncate()
                                                    .child(crate::state::effective_request_name(&req)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .when_some(req.last_status_code, |this, code| {
                                                this.child(StatusCodeBadge::new(code))
                                            })
                                            .child(
                                                div()
                                                    .id(format!("req-del-{}", req.id))
                                                    .px(px(4.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .hover(|s| s.bg(t.bg_card).text_color(t.status_4xx))
                                                    .on_click(move |_, _, cx| {
                                                        ws_del.update(cx, |state, cx| {
                                                            state.delete_request(&req_id_del, cx);
                                                        });
                                                        ed_del.update(cx, |state, cx| {
                                                            state.close_tab(&req_id_del, cx);
                                                        });
                                                    })
                                                    .child("×"),
                                            ),
                                    )
                            }))
                    }))
                    // Top-level requests without folder
                    .children(
                        requests
                            .iter()
                            .filter(|r| {
                                r.folder_id.is_none()
                                    && (query.is_empty()
                                        || r.name.to_lowercase().contains(&query)
                                        || r.url.to_lowercase().contains(&query))
                            })
                            .map(|req| {
                                let is_selected = active_req_id.as_ref() == Some(&req.id);
                                let req_for_click = req.clone();
                                let req_id_del = req.id.clone();
                                let ed_s = ed_clone.clone();
                                let ws_del = ws_clone.clone();
                                let ed_del = ed_clone.clone();

                                div()
                                    .id(format!("req-top-{}", req.id))
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(if is_selected {
                                        t.border
                                    } else {
                                        t.transparent
                                    })
                                    .bg(if is_selected {
                                        t.bg_selected
                                    } else {
                                        t.transparent
                                    })
                                    .hover(|s| s.bg(t.bg_hover))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .on_click(move |ev, _, cx| {
                                        let pin = ev.click_count() > 1;
                                        ed_s.update(cx, |state, cx| {
                                            state.set_active_request(req_for_click.clone(), pin, cx);
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .overflow_hidden()
                                            .child(MethodBadge::new(req.method))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(".SystemMonoFont")
                                                    .text_color(if is_selected { t.text_primary } else { t.text_secondary })
                                                    .truncate()
                                                    .child(crate::state::effective_request_name(&req)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .when_some(req.last_status_code, |this, code| {
                                                this.child(StatusCodeBadge::new(code))
                                            })
                                            .child(
                                                div()
                                                    .id(format!("req-top-del-{}", req.id))
                                                    .px(px(4.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .text_xs()
                                                    .text_color(t.text_muted)
                                                    .hover(|s| s.bg(t.bg_card).text_color(t.status_4xx))
                                                    .on_click(move |_, _, cx| {
                                                        ws_del.update(cx, |state, cx| {
                                                            state.delete_request(&req_id_del, cx);
                                                        });
                                                        ed_del.update(cx, |state, cx| {
                                                            state.close_tab(&req_id_del, cx);
                                                        });
                                                    })
                                                    .child("×"),
                                            ),
                                    )
                            }),
                    ),
            )
            // Footer: git branch chip
            .child(
                div()
                    .h(px(32.))
                    .px(px(10.))
                    .border_t_1()
                    .border_color(t.border)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_xs()
                    .text_color(t.text_muted)
                    .child("⎇")
                    .child(
                        div()
                            .text_color(t.text_secondary)
                            .font_family(".SystemMonoFont")
                            .child(git_branch),
                    ),
            )
    }
}
