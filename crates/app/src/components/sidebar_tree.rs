use crate::components::status_badge::{MethodBadge, StatusCodeBadge};
use crate::state::{EditorState, ResponseState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, Render, RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::menu::ContextMenuExt as _;
use gpui_component::scroll::ScrollableElement as _;
use restclient_core::models::{HttpMethod, HttpRequestItem};
use restclient_core::storage::models::RequestFile;
use restclient_core::storage::serialize::to_yaml_string;

#[derive(Clone, Debug)]
pub struct DragPinnedItem {
    pub id: String,
    pub index: usize,
}

impl Render for DragPinnedItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(6.))
            .bg(t.bg_card)
            .border_1()
            .border_color(t.border_focus)
            .shadow_md()
            .text_xs()
            .text_color(t.text_primary)
            .child(format!("📌 {}", self.id))
    }
}

#[derive(Clone, Debug)]
pub struct DragRequestItem {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
}

impl Render for DragRequestItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(6.))
            .bg(t.bg_card)
            .border_1()
            .border_color(t.border_focus)
            .shadow_md()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(MethodBadge::new(self.method))
            .child(
                div()
                    .text_xs()
                    .text_color(t.text_primary)
                    .child(self.name.clone()),
            )
    }
}

#[derive(Clone, Debug)]
pub struct DragFolderItem {
    pub id: String,
    pub name: String,
}

impl Render for DragFolderItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(6.))
            .bg(t.bg_card)
            .border_1()
            .border_color(t.border_focus)
            .shadow_md()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(div().child("📁"))
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.text_primary)
                    .child(self.name.clone()),
            )
    }
}

#[derive(IntoElement)]
pub struct SidebarTree {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
    response_state: Entity<ResponseState>,
}

impl SidebarTree {
    pub fn new(
        workspace_state: Entity<WorkspaceState>,
        editor_state: Entity<EditorState>,
        response_state: Entity<ResponseState>,
    ) -> Self {
        Self {
            workspace_state,
            editor_state,
            response_state,
        }
    }
}

impl RenderOnce for SidebarTree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let (git_branch, query, filter_query_raw, folders, requests, renaming_req_id, rename_input, renaming_folder_id, rename_folder_input, collapsed_folders, collapsed_requests, pins, parse_errors, unsaved_requests, running_request_id) = {
            let ws = self.workspace_state.read(cx);
            (
                ws.git_branch.clone().unwrap_or_else(|| "main".to_string()),
                ws.filter_query.to_lowercase(),
                ws.filter_query.clone(),
                ws.folders.clone(),
                ws.requests.clone(),
                ws.renaming_request_id.clone(),
                ws.rename_input.clone(),
                ws.renaming_folder_id.clone(),
                ws.rename_folder_input.clone(),
                ws.collapsed_folders.clone(),
                ws.collapsed_requests.clone(),
                ws.pins.clone(),
                ws.parse_errors.clone(),
                ws.unsaved_requests.clone(),
                ws.running_request_id.clone(),
            )
        };
        let active_req_id = self.editor_state.read(cx).active_request.as_ref().map(|r| r.id.clone());
        let active_example_name = self.response_state.read(cx).active_example_name.clone();
        let has_nothing = folders.is_empty() && requests.is_empty();

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();
        let resp_clone = self.response_state.clone();

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
            let is_req_active = active_req_id.as_ref() == Some(&req.id);
            let is_viewing_example = is_req_active && active_example_name.is_some();
            let is_selected = is_req_active && !is_viewing_example;
            let is_renaming = renaming_req_id.as_deref() == Some(&req.id);
            let has_examples = !req.examples.is_empty();
            let is_req_collapsed = collapsed_requests.contains(&req.id) && query.is_empty();
            let req_for_click = req.clone();
            let req_for_right = req.clone();
            let req_for_menu = req.clone();
            let ed_s = ed_clone.clone();
            let ed_right = ed_clone.clone();
            let ws_click = ws_clone.clone();
            let resp_click = resp_clone.clone();

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

            let drag_req_payload = DragRequestItem {
                id: req.id.clone(),
                name: initial_name.clone(),
                method: req.method,
            };
            let ws_req_drop = ws_clone.clone();
            let target_req_id = req.id.clone();

            let parent_row = div()
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
                } else if is_viewing_example {
                    t.bg_card
                } else {
                    t.transparent
                })
                .hover(|s| s.bg(t.bg_hover))
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_between()
                .when(!is_renaming, |this| {
                    this.on_drag(drag_req_payload, |item, _, _, cx| {
                        cx.new(|_| item.clone())
                    })
                })
                .drag_over::<DragRequestItem>(|style, _, _, _| {
                    let t = theme();
                    style.bg(t.bg_hover).border_color(t.border_focus)
                })
                .on_drop(move |drag: &DragRequestItem, _, cx| {
                    ws_req_drop.update(cx, |state, cx| {
                        state.move_request(&drag.id, &target_req_id, cx);
                    });
                })
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
                        let r_id = req_for_click.id.clone();
                        ws_click.update(cx, |state, cx| {
                            state.record_navigation(&r_id, cx);
                        });
                        ed_s.update(cx, |state, cx| {
                            state.set_active_request(req_for_click.clone(), false, cx);
                        });
                        let ws_store = ws_click.read(cx).store.clone();
                        resp_click.update(cx, |state, cx| {
                            state.load_for_request(&r_id, ws_store.as_ref(), cx);
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

                    let ws_pin = ws_clone.clone();
                    let r_id_pin = req_for_menu.id.clone();
                    let is_pinned = pins.contains(&r_id_pin);

                    let req_yaml = req_for_menu.clone();
                    let req_curl = req_for_menu.clone();

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

                        let ws_p = ws_pin.clone();
                        let r_id_p = r_id_pin.clone();

                        let r_yaml = req_yaml.clone();
                        let r_curl = req_curl.clone();

                        menu
                            .item(
                                gpui_component::menu::PopupMenuItem::new("Copy as YAML")
                                    .on_click(move |_, _, cx| {
                                        let rf = RequestFile::from_item(&r_yaml);
                                        if let Ok(yaml) = to_yaml_string(&rf) {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(yaml));
                                        }
                                    }),
                            )
                            .item(
                                gpui_component::menu::PopupMenuItem::new("Copy as cURL")
                                    .on_click(move |_, _, cx| {
                                        let curl = restclient_core::curl::request_to_curl(&r_curl, None);
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(curl));
                                    }),
                            )
                            .separator()
                            .item(
                                gpui_component::menu::PopupMenuItem::new(if is_pinned { "Unpin" } else { "Pin to top" })
                                    .on_click(move |_, _, cx| {
                                        ws_p.update(cx, |state, cx| {
                                            state.toggle_pin(&r_id_p, cx);
                                        });
                                    }),
                            )
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
                                        delete_request_item(&ws_del, &ed_del, &r_id_del, cx);
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
                        .child(if has_examples {
                            let ws_toggle = ws_clone.clone();
                            let r_id = req.id.clone();
                            div()
                                .id(format!("req-toggle-{}", r_id))
                                .w(px(10.))
                                .text_center()
                                .cursor_pointer()
                                .hover(|s| s.text_color(t.text_primary))
                                .text_color(t.text_muted)
                                .child(if is_req_collapsed { "›" } else { "⌄" })
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    ws_toggle.update(cx, |ws, cx| {
                                        ws.toggle_request_collapsed(&r_id, cx);
                                    });
                                })
                                .into_any_element()
                        } else {
                            div().w(px(10.)).into_any_element()
                        })
                        .child(MethodBadge::new(req.method))
                        .child(if let (true, Some(inp)) = (is_renaming, rename_input.as_ref()) {
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
                                .text_color(if is_selected || is_viewing_example { t.text_primary } else { t.text_secondary })
                                .truncate()
                                .child(crate::state::effective_request_name(req))
                                .into_any_element()
                        }),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .when(parse_errors.contains_key(&req.id), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(t.status_5xx)
                                    .child("⚠"),
                            )
                        })
                        .when(running_request_id.as_deref() == Some(&req.id), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(t.primary)
                                    .child("⟳"),
                            )
                        })
                        .when(unsaved_requests.contains(&req.id), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(t.primary)
                                    .child("●"),
                            )
                        })
                        .when_some(req.last_status_code, |this, code| {
                            this.child(StatusCodeBadge::new(code))
                        }),
                );

            if !has_examples {
                parent_row.into_any_element()
            } else {
                let examples_list = req.examples.clone();
                let parent_id = req.id.clone();
                let req_for_children = req.clone();

                div()
                    .id(format!("req-group-{}", parent_id))
                    .flex()
                    .flex_col()
                    .child(parent_row)
                    .when(!is_req_collapsed, |this| {
                        this.children(examples_list.into_iter().enumerate().map(|(idx, ex)| {
                            let is_this_ex_active = is_req_active && active_example_name.as_deref() == Some(&ex.name);
                            let ed_ex_click = ed_clone.clone();
                            let resp_ex_click = resp_clone.clone();
                            let ws_ex_click = ws_clone.clone();
                            let req_for_click = req_for_children.clone();
                            let ex_for_click = ex.clone();
                            let r_id_for_click = parent_id.clone();

                            div()
                                .id(format!("req-{}-ex-{}", parent_id, idx))
                                .when(in_folder, |s| s.ml(px(14.)))
                                .ml(px(14.))
                                .mr(px(2.))
                                .my(px(1.))
                                .px(px(6.))
                                .py(px(3.))
                                .rounded(px(5.))
                                .border_1()
                                .border_color(if is_this_ex_active {
                                    t.border
                                } else {
                                    t.transparent
                                })
                                .bg(if is_this_ex_active {
                                    t.bg_selected
                                } else {
                                    t.transparent
                                })
                                .hover(|s| s.bg(t.bg_hover))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .on_click(move |_, _window, cx| {
                                    ws_ex_click.update(cx, |state, cx| {
                                        state.record_navigation(&r_id_for_click, cx);
                                    });
                                    ed_ex_click.update(cx, |state, cx| {
                                        state.set_active_request(req_for_click.clone(), false, cx);
                                    });
                                    resp_ex_click.update(cx, |state, cx| {
                                        state.view_example(&ex_for_click, cx);
                                    });
                                })
                                .context_menu({
                                    let ws_ctx = ws_clone.clone();
                                    let resp_ctx = resp_clone.clone();
                                    let r_id_ctx = parent_id.clone();
                                    let ex_ctx = ex.clone();
                                    let total_ex = req_for_children.examples.len();

                                    move |menu, _window, _cx| {
                                        let ws_dup = ws_ctx.clone();
                                        let r_id_dup = r_id_ctx.clone();
                                        let ex_dup = ex_ctx.clone();

                                        let ws_del = ws_ctx.clone();
                                        let resp_del = resp_ctx.clone();
                                        let r_id_del = r_id_ctx.clone();
                                        let ex_del_name = ex_ctx.name.clone();

                                        let mut m = menu
                                            .item(
                                                gpui_component::menu::PopupMenuItem::new("Duplicate")
                                                    .on_click(move |_, _, cx| {
                                                        ws_dup.update(cx, |ws, cx| {
                                                            if let Some(mut rf) = ws.get_request_file(&r_id_dup) {
                                                                let base_name = ex_dup.name.clone();
                                                                let mut name = format!("{}-copy", base_name);
                                                                let mut i = 2;
                                                                while rf.examples.iter().any(|e| e.name == name) {
                                                                    name = format!("{}-copy-{}", base_name, i);
                                                                    i += 1;
                                                                }
                                                                let mut new_ex = ex_dup.clone();
                                                                new_ex.name = name;
                                                                rf.examples.push(new_ex);
                                                                ws.save_request_file(&r_id_dup, &rf, cx);
                                                            }
                                                        });
                                                    }),
                                            );

                                        if idx > 0 {
                                            let ws_up = ws_ctx.clone();
                                            let r_id_up = r_id_ctx.clone();
                                            m = m.item(
                                                gpui_component::menu::PopupMenuItem::new("Move Up")
                                                    .on_click(move |_, _, cx| {
                                                        ws_up.update(cx, |ws, cx| {
                                                            if let Some(mut rf) = ws.get_request_file(&r_id_up) {
                                                                if idx < rf.examples.len() {
                                                                    rf.examples.swap(idx, idx - 1);
                                                                    ws.save_request_file(&r_id_up, &rf, cx);
                                                                }
                                                            }
                                                        });
                                                    }),
                                            );
                                        }

                                        if idx + 1 < total_ex {
                                            let ws_down = ws_ctx.clone();
                                            let r_id_down = r_id_ctx.clone();
                                            m = m.item(
                                                gpui_component::menu::PopupMenuItem::new("Move Down")
                                                    .on_click(move |_, _, cx| {
                                                        ws_down.update(cx, |ws, cx| {
                                                            if let Some(mut rf) = ws.get_request_file(&r_id_down) {
                                                                if idx + 1 < rf.examples.len() {
                                                                    rf.examples.swap(idx, idx + 1);
                                                                    ws.save_request_file(&r_id_down, &rf, cx);
                                                                }
                                                            }
                                                        });
                                                    }),
                                            );
                                        }

                                        m.separator()
                                            .item(
                                                gpui_component::menu::PopupMenuItem::new("Delete")
                                                    .on_click(move |_, _, cx| {
                                                        ws_del.update(cx, |ws, cx| {
                                                            if let Some(mut rf) = ws.get_request_file(&r_id_del) {
                                                                rf.examples.retain(|e| e.name != ex_del_name);
                                                                ws.save_request_file(&r_id_del, &rf, cx);
                                                            }
                                                            if is_this_ex_active {
                                                                let store = ws.store.clone();
                                                                resp_del.update(cx, |s, cx| {
                                                                    s.load_for_request(&r_id_del, store.as_ref(), cx);
                                                                });
                                                            }
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
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(t.text_muted)
                                                .child("↳"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_family(".SystemMonoFont")
                                                .text_color(if is_this_ex_active {
                                                    t.text_primary
                                                } else {
                                                    t.text_secondary
                                                })
                                                .truncate()
                                                .child(ex.name.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .child(StatusCodeBadge::new(ex.status)),
                                )
                        }))
                    })
                    .into_any_element()
            }
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
                    .when(!pins.is_empty(), |this| {
                        let ws_pin_reorder = ws_clone.clone();
                        let pinned_requests: Vec<_> = pins
                            .iter()
                            .filter_map(|pin_id| requests.iter().find(|r| &r.id == pin_id).cloned())
                            .collect();

                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .mb(px(4.))
                                .child(
                                    div()
                                        .px(px(6.))
                                        .py(px(2.))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(t.text_muted)
                                        .child("📌 PINNED"),
                                )
                                .children(pinned_requests.into_iter().enumerate().map(|(idx, req)| {
                                    let p_id = req.id.clone();
                                    let drag_pin = DragPinnedItem {
                                        id: p_id.clone(),
                                        index: idx,
                                    };
                                    let ws_p_drop = ws_pin_reorder.clone();
                                    let target_idx = idx;

                                    div()
                                        .id(format!("pinned-row-{}", p_id))
                                        .on_drag(drag_pin, |item, _, _, cx| {
                                            cx.new(|_| item.clone())
                                        })
                                        .drag_over::<DragPinnedItem>(|style, _, _, _| {
                                            let t = theme();
                                            style.bg(t.bg_hover).border_t_2().border_color(t.border_focus)
                                        })
                                        .on_drop(move |drag: &DragPinnedItem, _, cx| {
                                            let from_idx = drag.index;
                                            let to_idx = target_idx;
                                            ws_p_drop.update(cx, |state, cx| {
                                                if from_idx < state.pins.len() && to_idx < state.pins.len() {
                                                    let item = state.pins.remove(from_idx);
                                                    state.pins.insert(to_idx, item);
                                                    let _ = state.store.save_pins(&state.pins);
                                                    cx.notify();
                                                }
                                            });
                                        })
                                        .child(render_req_item(&req, false))
                                }))
                        )
                    })
                    .children(folders.iter().map(|folder| {
                        let f_id = folder.id.clone();
                        let is_renaming_folder = renaming_folder_id.as_deref() == Some(&f_id);
                        let is_collapsed = collapsed_folders.contains(&f_id);
                        let folder_name = folder.name.clone();

                        let folder_requests: Vec<_> = requests
                            .iter()
                            .filter(|r| {
                                r.folder_id.as_deref() == Some(&f_id)
                                    && (query.is_empty()
                                        || r.name.to_lowercase().contains(&query)
                                        || r.url.to_lowercase().contains(&query)
                                        || r.examples.iter().any(|e| e.name.to_lowercase().contains(&query)))
                            })
                            .cloned()
                            .collect();

                        let drag_folder_payload = DragFolderItem {
                            id: f_id.clone(),
                            name: folder_name.clone(),
                        };
                        let ws_drop_req = ws_clone.clone();
                        let f_id_target_req = f_id.clone();
                        let ws_drop_folder = ws_clone.clone();
                        let f_id_target_folder = f_id.clone();

                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            // Folder header
                            .child(
                                div()
                                    .id(format!("folder-header-{}", f_id))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .text_color(t.text_secondary)
                                    .text_xs()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(t.bg_hover))
                                    .when(!is_renaming_folder, move |this| {
                                        this.on_drag(drag_folder_payload, |item, _, _, cx| {
                                            cx.new(|_| item.clone())
                                        })
                                    })
                                    .drag_over::<DragRequestItem>(|style, _, _, _| {
                                        let t = theme();
                                        style.bg(t.bg_hover).border_1().border_color(t.border_focus)
                                    })
                                    .on_drop(move |drag: &DragRequestItem, _, cx| {
                                        ws_drop_req.update(cx, |state, cx| {
                                            state.move_request_to_folder(&drag.id, Some(f_id_target_req.clone()), cx);
                                        });
                                    })
                                    .drag_over::<DragFolderItem>(|style, _, _, _| {
                                        let t = theme();
                                        style.bg(t.bg_hover).border_t_2().border_color(t.border_focus)
                                    })
                                    .on_drop(move |drag: &DragFolderItem, _, cx| {
                                        ws_drop_folder.update(cx, |state, cx| {
                                            state.move_folder(&drag.id, &f_id_target_folder, cx);
                                        });
                                    })
                                    .on_click({
                                        let ws_f_click = ws_clone.clone();
                                        let f_id_click = f_id.clone();
                                        let f_name_click = folder_name.clone();
                                        move |ev, window, cx| {
                                            if is_renaming_folder {
                                                return;
                                            }
                                            if ev.click_count() > 1 {
                                                ws_f_click.update(cx, |state, cx| {
                                                    state.start_renaming_folder(f_id_click.clone(), f_name_click.clone(), window, cx);
                                                });
                                            } else {
                                                ws_f_click.update(cx, |state, cx| {
                                                    state.toggle_folder_collapsed(&f_id_click, cx);
                                                });
                                            }
                                        }
                                    })
                                    .context_menu({
                                        let ws_new = ws_clone.clone();
                                        let ed_new = ed_clone.clone();
                                        let f_id_new = f_id.clone();

                                        let ws_ren = ws_clone.clone();
                                        let f_id_ren = f_id.clone();
                                        let f_name_ren = folder_name.clone();

                                        let ws_del = ws_clone.clone();
                                        let f_id_del = f_id.clone();

                                        move |menu, _window, _cx| {
                                            let ws_n = ws_new.clone();
                                            let ed_n = ed_new.clone();
                                            let f_id_n = f_id_new.clone();

                                            let ws_r = ws_ren.clone();
                                            let f_id_r = f_id_ren.clone();
                                            let f_name_r = f_name_ren.clone();

                                            let ws_d = ws_del.clone();
                                            let f_id_d = f_id_del.clone();

                                            menu
                                                .item(
                                                    gpui_component::menu::PopupMenuItem::new("New Request")
                                                        .on_click(move |_, _, cx| {
                                                            let req = ws_n.update(cx, |state, cx| {
                                                                state.create_request(Some(f_id_n.clone()), cx)
                                                            });
                                                            ed_n.update(cx, |state, cx| {
                                                                state.set_active_request(req, true, cx);
                                                            });
                                                        }),
                                                )
                                                .item(
                                                    gpui_component::menu::PopupMenuItem::new("Rename")
                                                        .on_click(move |_, window, cx| {
                                                            ws_r.update(cx, |state, cx| {
                                                                state.start_renaming_folder(f_id_r.clone(), f_name_r.clone(), window, cx);
                                                            });
                                                        }),
                                                )
                                                .separator()
                                                .item(
                                                    gpui_component::menu::PopupMenuItem::new("Delete")
                                                        .on_click(move |_, _, cx| {
                                                            ws_d.update(cx, |state, cx| {
                                                                state.delete_folder(&f_id_d, cx);
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
                                            .child(
                                                div()
                                                    .w(px(10.))
                                                    .text_center()
                                                    .child(if is_collapsed { "›" } else { "⌄" }),
                                            )
                                            .child(div().child("📁"))
                                            .child(if let (true, Some(inp)) = (is_renaming_folder, rename_folder_input.as_ref()) {
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
                                                                state.cancel_renaming_folder(cx);
                                                            });
                                                        }
                                                    })
                                                    .child(
                                                        gpui_component::input::Input::new(inp)
                                                            .id(format!("rename-folder-field-{}", folder.id))
                                                            .appearance(false)
                                                            .bordered(false)
                                                            .focus_bordered(false)
                                                            .cleanable(false),
                                                    )
                                                    .into_any_element()
                                            } else {
                                                div()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(t.text_primary)
                                                    .truncate()
                                                    .child(folder.name.clone())
                                                    .into_any_element()
                                            }),
                                    ),
                            )
                            // Folder items
                            .when(!is_collapsed, |this| {
                                this.children(folder_requests.iter().map(|req| {
                                    render_req_item(req, true)
                                }))
                            })
                    }))
                    // Top-level requests without folder
                    .children(
                        requests
                            .iter()
                            .filter(|r| {
                                r.folder_id.is_none()
                                    && (query.is_empty()
                                        || r.name.to_lowercase().contains(&query)
                                        || r.url.to_lowercase().contains(&query)
                                        || r.examples.iter().any(|e| e.name.to_lowercase().contains(&query)))
                            })
                            .map(|req| render_req_item(req, false)),
                    )
                    // Drop zone at bottom to drag request to root level
                    .child({
                        let ws_root = ws_clone.clone();
                        div()
                            .id("sidebar-root-dropzone")
                            .min_h(px(24.))
                            .flex_1()
                            .rounded(px(4.))
                            .drag_over::<DragRequestItem>(|style, _, _, _| {
                                let t = theme();
                                style.bg(t.bg_hover).border_1().border_color(t.border_focus)
                            })
                            .on_drop(move |drag: &DragRequestItem, _, cx| {
                                ws_root.update(cx, |state, cx| {
                                    state.move_request_to_folder(&drag.id, None, cx);
                                });
                            })
                    }),
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

fn delete_request_item(
    ws_entity: &Entity<WorkspaceState>,
    ed_entity: &Entity<EditorState>,
    req_id: &str,
    cx: &mut App,
) {
    let is_active = ed_entity
        .read(cx)
        .active_request
        .as_ref()
        .map(|r| r.id.as_str() == req_id)
        .unwrap_or(false);

    let (remaining_requests, deleted_idx) = ws_entity.update(cx, |ws, cx| {
        let idx = ws.requests.iter().position(|r| r.id == req_id);
        ws.delete_request(req_id, cx);
        (ws.requests.clone(), idx)
    });

    let next_tab_id = ed_entity.update(cx, |ed, cx| ed.close_tab(req_id, cx));

    if is_active {
        let next_req = next_tab_id
            .and_then(|id| remaining_requests.iter().find(|r| r.id == id).cloned())
            .or_else(|| {
                if let Some(idx) = deleted_idx {
                    if !remaining_requests.is_empty() {
                        let new_idx = idx.min(remaining_requests.len() - 1);
                        return Some(remaining_requests[new_idx].clone());
                    }
                }
                remaining_requests.first().cloned()
            });

        if let Some(req) = next_req {
            ed_entity.update(cx, |ed, cx| {
                ed.set_active_request(req, false, cx);
            });
        } else {
            let new_req = ws_entity.update(cx, |ws, cx| ws.create_request(None, cx));
            ed_entity.update(cx, |ed, cx| {
                ed.set_active_request(new_req, true, cx);
            });
        }
    }
}

