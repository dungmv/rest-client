use crate::components::status_badge::{MethodBadge, StatusCodeBadge};
use crate::state::{EditorState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::menu::ContextMenuExt as _;
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::HttpRequestItem;

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
        let (git_branch, query, filter_query_raw, folders, requests, renaming_req_id, rename_input) = {
            let ws = self.workspace_state.read(cx);
            (
                ws.git_branch.clone().unwrap_or_else(|| "main".to_string()),
                ws.filter_query.to_lowercase(),
                ws.filter_query.clone(),
                ws.folders.clone(),
                ws.requests.clone(),
                ws.renaming_request_id.clone(),
                ws.rename_input.clone(),
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

        let render_req_item = |req: &HttpRequestItem, in_folder: bool| -> AnyElement {
            let is_selected = active_req_id.as_ref() == Some(&req.id);
            let is_renaming = renaming_req_id.as_deref() == Some(&req.id);
            let req_for_click = req.clone();
            let req_for_right = req.clone();
            let req_for_menu = req.clone();
            let req_id_del = req.id.clone();
            let ed_s = ed_clone.clone();
            let ed_right = ed_clone.clone();
            let ws_del = ws_clone.clone();
            let ed_del = ed_clone.clone();
            let ws_click = ws_clone.clone();

            let initial_name = {
                let eff = crate::state::effective_request_name(req);
                if !req.name.trim().is_empty() && req.name != "New Request" && req.name != "Untitled Request" {
                    req.name.clone()
                } else {
                    eff
                }
            };
            let initial_name_click = initial_name.clone();
            let initial_name_menu = initial_name.clone();

            div()
                .id(format!("req-item-{}", req.id))
                .when(in_folder, |s| s.ml(px(14.)))
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
                .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                    ed_right.update(cx, |state, cx| {
                        state.set_active_request(req_for_right.clone(), false, cx);
                    });
                })
                .on_click(move |ev, window, cx| {
                    if is_renaming {
                        return;
                    }
                    if ev.click_count() > 1 {
                        let r_id = req_for_click.id.clone();
                        let init_name = initial_name_click.clone();
                        ws_click.update(cx, |state, cx| {
                            state.start_renaming_request(r_id, init_name, window, cx);
                        });
                        ed_s.update(cx, |state, cx| {
                            state.set_active_request(req_for_click.clone(), false, cx);
                        });
                    } else {
                        ed_s.update(cx, |state, cx| {
                            state.set_active_request(req_for_click.clone(), false, cx);
                        });
                    }
                })
                .context_menu({
                    let ws_rename = ws_clone.clone();
                    let r_id_rename = req_for_menu.id.clone();
                    let init_name_rename = initial_name_menu.clone();

                    let ws_dup = ws_clone.clone();
                    let ed_dup = ed_clone.clone();
                    let r_id_dup = req_for_menu.id.clone();

                    let ws_del_ctx = ws_clone.clone();
                    let ed_del_ctx = ed_clone.clone();
                    let r_id_del_ctx = req_for_menu.id.clone();

                    move |menu, _window, _cx| {
                        let ws_r = ws_rename.clone();
                        let r_id_r = r_id_rename.clone();
                        let init_r = init_name_rename.clone();

                        let ws_d = ws_dup.clone();
                        let ed_d = ed_dup.clone();
                        let r_id_d = r_id_dup.clone();

                        let ws_del = ws_del_ctx.clone();
                        let ed_del = ed_del_ctx.clone();
                        let r_id_del = r_id_del_ctx.clone();

                        menu
                            .item(
                                gpui_component::menu::PopupMenuItem::new("Rename")
                                    .on_click(move |_, window, cx| {
                                        ws_r.update(cx, |state, cx| {
                                            state.start_renaming_request(r_id_r.clone(), init_r.clone(), window, cx);
                                        });
                                    }),
                            )
                            .item(
                                gpui_component::menu::PopupMenuItem::new("Duplicate")
                                    .on_click(move |_, _, cx| {
                                        if let Some(dup) = ws_d.update(cx, |state, cx| {
                                            state.duplicate_request(&r_id_d, cx)
                                        }) {
                                            ed_d.update(cx, |state, cx| {
                                                state.set_active_request(dup, false, cx);
                                            });
                                        }
                                    }),
                            )
                            .separator()
                            .item(
                                gpui_component::menu::PopupMenuItem::new("Delete")
                                    .on_click(move |_, _, cx| {
                                        ws_del.update(cx, |state, cx| {
                                            state.delete_request(&r_id_del, cx);
                                        });
                                        ed_del.update(cx, |state, cx| {
                                            state.close_tab(&r_id_del, cx);
                                        });
                                    }),
                            )
                    }
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .overflow_hidden()
                        .child(MethodBadge::new(req.method))
                        .child(if is_renaming && rename_input.is_some() {
                            let inp = rename_input.as_ref().unwrap();
                            let ws_esc = ws_clone.clone();
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .h(px(22.))
                                .px(px(4.))
                                .rounded(px(4.))
                                .bg(t.bg_input)
                                .border_1()
                                .border_color(t.border_focus)
                                .flex()
                                .items_center()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation();
                                })
                                .on_key_down(move |event, _, cx| {
                                    if event.keystroke.key.as_str() == "escape" {
                                        ws_esc.update(cx, |state, cx| {
                                            state.cancel_renaming(cx);
                                        });
                                    }
                                })
                                .child(
                                    gpui_component::input::Input::new(inp)
                                        .id(format!("rename-input-field-{}", req.id))
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false)
                                        .cleanable(false),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .text_xs()
                                .font_family(".SystemMonoFont")
                                .text_color(if is_selected { t.text_primary } else { t.text_secondary })
                                .truncate()
                                .child(crate::state::effective_request_name(req))
                                .into_any_element()
                        }),
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
                .into_any_element()
        };

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
                            .children(folder_requests.iter().map(|req| {
                                render_req_item(req, true)
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
                            .map(|req| render_req_item(req, false)),
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
