use crate::components::status_badge::{MethodBadge, StatusCodeBadge};
use crate::state::{EditorState, GitActiveModal, ResponseState, ToastState, WorkspaceState};
use crate::theme::theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, Render, RenderOnce, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::Icon;
use gpui_kit::assets::IconName;
use restclient_core::models::{Folder, HttpMethod, HttpRequestItem};
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
            .child(
                Icon::new(IconName::Folder)
                    .size(px(14.))
                    .text_color(t.text_secondary),
            )
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.text_primary)
                    .child(self.name.clone()),
            )
    }
}

#[derive(Clone, Debug)]
pub struct SidebarExampleItem {
    pub name: String,
    pub status: u16,
}

#[derive(Clone, Debug)]
pub struct SidebarReqItem {
    pub id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub display_name: String,
    pub method: HttpMethod,
    pub url: String,
    pub last_status_code: Option<u16>,
    pub examples: Vec<SidebarExampleItem>,
}

#[derive(IntoElement)]
pub struct SidebarTree {
    workspace_state: Entity<WorkspaceState>,
    editor_state: Entity<EditorState>,
    response_state: Entity<ResponseState>,
    toast_state: Entity<ToastState>,
}

impl SidebarTree {
    pub fn new(
        workspace_state: Entity<WorkspaceState>,
        editor_state: Entity<EditorState>,
        response_state: Entity<ResponseState>,
        toast_state: Entity<ToastState>,
    ) -> Self {
        Self {
            workspace_state,
            editor_state,
            response_state,
            toast_state,
        }
    }
}

impl RenderOnce for SidebarTree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme();
        let (git_branch, git_status, git_is_syncing, query, filter_query_raw, folders, requests, renaming_req_id, rename_input, renaming_folder_id, rename_folder_input, renaming_example, rename_example_input, collapsed_folders, collapsed_requests, pins, parse_errors, unsaved_requests, running_request_id) = {
            let ws = self.workspace_state.read(cx);
            let req_items: Vec<SidebarReqItem> = ws.requests.iter().map(|r| {
                let initial_name = {
                    let eff = crate::state::effective_request_name(r);
                    if !r.name.trim().is_empty() && r.name != "New Request" && r.name != "Untitled Request" {
                        r.name.clone()
                    } else {
                        eff
                    }
                };
                let examples = r.examples.iter().map(|e| SidebarExampleItem {
                    name: e.name.clone(),
                    status: e.status,
                }).collect();

                SidebarReqItem {
                    id: r.id.clone(),
                    folder_id: r.folder_id.clone(),
                    name: r.name.clone(),
                    display_name: initial_name,
                    method: r.method,
                    url: r.url.clone(),
                    last_status_code: r.last_status_code,
                    examples,
                }
            }).collect();

            (
                ws.git_branch.clone().unwrap_or_else(|| "main".to_string()),
                ws.git_status.clone(),
                ws.git_is_syncing,
                ws.filter_query.to_lowercase(),
                ws.filter_query.clone(),
                ws.folders.clone(),
                req_items,
                ws.renaming_request_id.clone(),
                ws.rename_input.clone(),
                ws.renaming_folder_id.clone(),
                ws.rename_folder_input.clone(),
                ws.renaming_example.clone(),
                ws.rename_example_input.clone(),
                ws.collapsed_folders.clone(),
                ws.collapsed_requests.clone(),
                ws.pins.clone(),
                ws.parse_errors.clone(),
                ws.unsaved_requests.clone(),
                ws.running_request_id.clone(),
            )
        };
        let is_repo = git_status.as_ref().map(|s| s.is_repo).unwrap_or(false);
        let behind = git_status.as_ref().map(|s| s.behind).unwrap_or(0);
        let ahead = git_status.as_ref().map(|s| s.ahead).unwrap_or(0);
        let is_syncing = git_is_syncing;

        let active_req_id = self.editor_state.read(cx).active_request.as_ref().map(|r| r.id.clone());
        let active_example_name = self.response_state.read(cx).active_example_name.clone();
        let has_nothing = folders.is_empty() && requests.is_empty();

        let ws_clone = self.workspace_state.clone();
        let ed_clone = self.editor_state.clone();
        let resp_clone = self.response_state.clone();
        let toast_clone = self.toast_state.clone();

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

        let render_req_item = |req: &SidebarReqItem, in_folder: bool| -> AnyElement {
            let is_req_active = active_req_id.as_ref() == Some(&req.id);
            let is_viewing_example = is_req_active && active_example_name.is_some();
            let is_selected = is_req_active && !is_viewing_example;
            let is_renaming = renaming_req_id.as_deref() == Some(&req.id);
            let has_examples = !req.examples.is_empty();
            let is_req_collapsed = collapsed_requests.contains(&req.id) && query.is_empty();
            let r_id_click = req.id.clone();
            let r_id_right = req.id.clone();
            let r_id_menu = req.id.clone();
            let ed_s = ed_clone.clone();
            let ed_right = ed_clone.clone();
            let ws_click = ws_clone.clone();
            let ws_right = ws_clone.clone();
            let resp_click = resp_clone.clone();

            let initial_name = req.display_name.clone();
            let initial_name_click = initial_name.clone();
            let initial_name_menu = initial_name.clone();

            let drag_req_payload = DragRequestItem {
                id: req.id.clone(),
                name: initial_name.clone(),
                method: req.method,
            };
            let ws_req_drop = ws_clone.clone();
            let target_req_id = req.id.clone();

            let git_green: gpui::Hsla = gpui::rgb(0x4ade80).into();
            let git_yellow: gpui::Hsla = gpui::rgb(0xfacc15).into();
            let is_unsaved = unsaved_requests.contains(&req.id);
            let req_git_status = calculate_request_git_status(&req.id, git_status.as_ref(), is_unsaved);

            let req_name_color = match req_git_status {
                SidebarGitStatus::New => git_green,
                SidebarGitStatus::Modified => git_yellow,
                SidebarGitStatus::Clean => t.text_primary,
            };

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
                    if let Some(r) = ws_right.read(cx).find_request(&r_id_right) {
                        let r = r.clone();
                        ed_right.update(cx, |state, cx| {
                            state.set_active_request(r, false, cx);
                        });
                    }
                })
                .on_click(move |ev, window, cx| {
                    if is_renaming {
                        return;
                    }
                    if let Some(req_item) = ws_click.read(cx).find_request(&r_id_click).cloned() {
                        if ev.click_count() > 1 {
                            let r_id = r_id_click.clone();
                            let init_name = initial_name_click.clone();
                            ws_click.update(cx, |state, cx| {
                                state.start_renaming_request(r_id, init_name, window, cx);
                            });
                            ed_s.update(cx, |state, cx| {
                                state.set_active_request(req_item, false, cx);
                            });
                        } else {
                            let r_id = r_id_click.clone();
                            ws_click.update(cx, |state, cx| {
                                state.record_navigation(&r_id, cx);
                            });
                            ed_s.update(cx, |state, cx| {
                                state.set_active_request(req_item, false, cx);
                            });
                            let ws_store = ws_click.read(cx).store.clone();
                            resp_click.update(cx, |state, cx| {
                                state.load_for_request(&r_id, ws_store.as_ref(), cx);
                            });
                        }
                    }
                })
                .context_menu({
                    let ws_menu = ws_clone.clone();
                    let r_id_menu = r_id_menu.clone();
                    let init_name_menu = initial_name_menu.clone();
                    let ed_menu = ed_clone.clone();
                    let toast_menu = toast_clone.clone();
                    let is_pinned = pins.contains(&r_id_menu);
                    let is_dirty_git = req_git_status != SidebarGitStatus::Clean;

                    move |menu, _window, _cx| {
                        let ws_r = ws_menu.clone();
                        let r_id_r = r_id_menu.clone();
                        let init_r = init_name_menu.clone();

                        let ws_d = ws_menu.clone();
                        let ed_d = ed_menu.clone();
                        let r_id_d = r_id_menu.clone();

                        let ws_del = ws_menu.clone();
                        let ed_del = ed_menu.clone();
                        let r_id_del = r_id_menu.clone();

                        let ws_p = ws_menu.clone();
                        let r_id_p = r_id_menu.clone();

                        let ws_yaml = ws_menu.clone();
                        let r_id_yaml = r_id_menu.clone();
                        let ws_curl = ws_menu.clone();
                        let r_id_curl = r_id_menu.clone();

                        let mut m = menu
                            .item(
                                PopupMenuItem::new("Copy as YAML")
                                    .on_click(move |_, _, cx| {
                                        if let Some(r) = ws_yaml.read(cx).find_request(&r_id_yaml) {
                                            let rf = RequestFile::from_item(r);
                                            if let Ok(yaml) = to_yaml_string(&rf) {
                                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(yaml));
                                            }
                                        }
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Copy as cURL")
                                    .on_click(move |_, _, cx| {
                                        if let Some(r) = ws_curl.read(cx).find_request(&r_id_curl) {
                                            let mut r_clone = r.clone();
                                            let (resolved_h, resolved_a) = ws_curl.read(cx).resolve_request_hierarchy(&r_clone);
                                            r_clone.headers = resolved_h;
                                            r_clone.auth = resolved_a;
                                            let curl = restclient_core::curl::request_to_curl(&r_clone, None);
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(curl));
                                        }
                                    }),
                            )
                            .separator()
                            .item(
                                PopupMenuItem::new(if is_pinned { "Unpin" } else { "Pin to top" })
                                    .on_click(move |_, _, cx| {
                                        ws_p.update(cx, |state, cx| {
                                            state.toggle_pin(&r_id_p, cx);
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Rename")
                                    .on_click(move |_, window, cx| {
                                        ws_r.update(cx, |state, cx| {
                                            state.start_renaming_request(r_id_r.clone(), init_r.clone(), window, cx);
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Duplicate")
                                    .on_click(move |_, _, cx| {
                                        if let Some(dup) = ws_d.update(cx, |state, cx| {
                                            state.duplicate_request(&r_id_d, cx)
                                        }) {
                                            ed_d.update(cx, |state, cx| {
                                                state.set_active_request(dup, false, cx);
                                            });
                                        }
                                    }),
                            );

                        if is_dirty_git {
                            let ws_disc = ws_menu.clone();
                            let ed_disc = ed_menu.clone();
                            let r_id_disc = r_id_menu.clone();
                            let toast_disc = toast_menu.clone();
                            let r_name_disc = init_name_menu.clone();

                            m = m.separator().item(
                                PopupMenuItem::new("Discard Changes")
                                    .on_click(move |_, _, cx| {
                                        let req_name = r_name_disc.clone();
                                        ws_disc.update(cx, |ws, cx| {
                                            ws.discard_request_changes(&r_id_disc, &ed_disc, cx);
                                        });
                                        toast_disc.update(cx, |ts, cx| {
                                            ts.push_info("Git Discard", format!("Discarded changes in '{}'", req_name), cx);
                                        });
                                    }),
                            );
                        }

                        m.separator().item(
                            PopupMenuItem::new("Delete")
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
                                .w(px(14.))
                                .h(px(14.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|s| s.text_color(t.text_primary))
                                .text_color(t.text_muted)
                                .child(
                                    Icon::new(if is_req_collapsed {
                                        IconName::ChevronRight
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .size(px(11.))
                                )
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    ws_toggle.update(cx, |ws, cx| {
                                        ws.toggle_request_collapsed(&r_id, cx);
                                    });
                                })
                                .into_any_element()
                        } else {
                            div().w(px(14.)).h(px(14.)).flex_shrink_0().into_any_element()
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
                                .font_weight(if req_git_status != SidebarGitStatus::Clean {
                                    gpui::FontWeight::MEDIUM
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .text_color(req_name_color)
                                .truncate()
                                .child(req.display_name.clone())
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
                                    .child(Icon::new(IconName::RefreshCw).size(px(12.))),
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
                        .when(req_git_status != SidebarGitStatus::Clean, |this| {
                            match req_git_status {
                                SidebarGitStatus::New => this.child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(git_green)
                                        .child("U"),
                                ),
                                SidebarGitStatus::Modified => this.child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(git_yellow)
                                        .child("M"),
                                ),
                                SidebarGitStatus::Clean => this,
                            }
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

                div()
                    .id(format!("req-group-{}", parent_id))
                    .flex()
                    .flex_col()
                    .child(parent_row)
                    .when(!is_req_collapsed, |this| {
                        this.children(examples_list.into_iter().enumerate().map(|(idx, ex)| {
                            let is_this_ex_active = is_req_active && active_example_name.as_deref() == Some(&ex.name);
                            let is_renaming_this_ex = renaming_example.as_ref().map(|(r_id, ex_name)| r_id == &parent_id && ex_name == &ex.name).unwrap_or(false);
                            let ed_ex_click = ed_clone.clone();
                            let resp_ex_click = resp_clone.clone();
                            let ws_ex_click = ws_clone.clone();
                            let r_id_for_click = parent_id.clone();
                            let ex_name_for_click = ex.name.clone();

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
                                .on_click(move |ev, window, cx| {
                                    if is_renaming_this_ex {
                                        return;
                                    }
                                    let r_id = r_id_for_click.clone();
                                    let ex_name = ex_name_for_click.clone();
                                    if ev.click_count() > 1 {
                                        ws_ex_click.update(cx, |state, cx| {
                                            state.start_renaming_example(r_id.clone(), ex_name.clone(), window, cx);
                                        });
                                        if let Some(r) = ws_ex_click.read(cx).find_request(&r_id).cloned() {
                                            ed_ex_click.update(cx, |state, cx| {
                                                state.set_active_request(r, false, cx);
                                            });
                                        }
                                        if let Some(rf) = ws_ex_click.read(cx).get_request_file(&r_id) {
                                            if let Some(ex_file) = rf.examples.iter().find(|e| e.name == ex_name) {
                                                resp_ex_click.update(cx, |state, cx| {
                                                    state.view_example(ex_file, cx);
                                                });
                                            }
                                        }
                                    } else {
                                        ws_ex_click.update(cx, |state, cx| {
                                            state.record_navigation(&r_id, cx);
                                        });
                                        if let Some(r) = ws_ex_click.read(cx).find_request(&r_id).cloned() {
                                            ed_ex_click.update(cx, |state, cx| {
                                                state.set_active_request(r, false, cx);
                                            });
                                        }
                                        if let Some(rf) = ws_ex_click.read(cx).get_request_file(&r_id) {
                                            if let Some(ex_file) = rf.examples.iter().find(|e| e.name == ex_name) {
                                                resp_ex_click.update(cx, |state, cx| {
                                                    state.view_example(ex_file, cx);
                                                });
                                            }
                                        }
                                    }
                                })
                                .context_menu({
                                    let ws_ctx = ws_clone.clone();
                                    let resp_ctx = resp_clone.clone();
                                    let r_id_ctx = parent_id.clone();
                                    let ex_name_ctx = ex.name.clone();
                                    let total_ex = req.examples.len();

                                    move |menu, _window, _cx| {
                                        let ws_ren = ws_ctx.clone();
                                        let r_id_ren = r_id_ctx.clone();
                                        let ex_ren_name = ex_name_ctx.clone();

                                        let ws_dup = ws_ctx.clone();
                                        let r_id_dup = r_id_ctx.clone();
                                        let ex_dup_name = ex_name_ctx.clone();

                                        let ws_del = ws_ctx.clone();
                                        let resp_del = resp_ctx.clone();
                                        let r_id_del = r_id_ctx.clone();
                                        let ex_del_name = ex_name_ctx.clone();

                                        let mut m = menu
                                            .item(
                                                gpui_component::menu::PopupMenuItem::new("Rename")
                                                    .on_click(move |_, window, cx| {
                                                        ws_ren.update(cx, |state, cx| {
                                                            state.start_renaming_example(r_id_ren.clone(), ex_ren_name.clone(), window, cx);
                                                        });
                                                    }),
                                            )
                                            .item(
                                                gpui_component::menu::PopupMenuItem::new("Duplicate")
                                                    .on_click(move |_, _, cx| {
                                                        ws_dup.update(cx, |ws, cx| {
                                                            if let Some(mut rf) = ws.get_request_file(&r_id_dup) {
                                                                if let Some(orig_ex) = rf.examples.iter().find(|e| e.name == ex_dup_name).cloned() {
                                                                    let base_name = orig_ex.name.clone();
                                                                    let mut name = format!("{}-copy", base_name);
                                                                    let mut i = 2;
                                                                    while rf.examples.iter().any(|e| e.name == name) {
                                                                        name = format!("{}-copy-{}", base_name, i);
                                                                        i += 1;
                                                                    }
                                                                    let mut new_ex = orig_ex;
                                                                    new_ex.name = name;
                                                                    rf.examples.push(new_ex);
                                                                    ws.save_request_file(&r_id_dup, &rf, cx);
                                                                }
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
                                        .child(if let (true, Some(inp)) = (is_renaming_this_ex, rename_example_input.as_ref()) {
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
                                                            state.cancel_renaming_example(cx);
                                                        });
                                                    }
                                                })
                                                .child(
                                                    gpui_component::input::Input::new(inp)
                                                        .id(format!("rename-ex-input-{}-{}", parent_id, idx))
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
                                                .text_color(if is_this_ex_active {
                                                    t.text_primary
                                                } else {
                                                    t.text_secondary
                                                })
                                                .truncate()
                                                .child(ex.name.clone())
                                                .into_any_element()
                                        }),
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

        let res = div()
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
                        let ws_menu = ws_clone.clone();
                        let ed_menu = ed_clone.clone();
                        Button::new("btn-sidebar-new")
                            .ghost()
                            .compact()
                            .w(px(28.))
                            .h(px(28.))
                            .child(
                                Icon::new(IconName::Plus)
                                    .size(px(15.))
                                    .text_color(t.text_secondary),
                            )
                            .dropdown_menu_with_anchor(gpui::Anchor::TopRight, move |menu, _window, _cx| {
                                let ws_req = ws_menu.clone();
                                let ed_req = ed_menu.clone();
                                let ws_folder = ws_menu.clone();

                                menu
                                    .item(
                                        PopupMenuItem::new("New Request")
                                            .icon(Icon::new(IconName::FileText))
                                            .on_click(move |_, _, cx| {
                                                let req = ws_req.update(cx, |state, cx| {
                                                    state.create_request(None, cx)
                                                });
                                                ed_req.update(cx, |state, cx| {
                                                    state.set_active_request(req, true, cx);
                                                });
                                            }),
                                    )
                                    .item(
                                        PopupMenuItem::new("New Folder")
                                            .icon(Icon::new(IconName::Folder))
                                            .on_click(move |_, window, cx| {
                                                let created = ws_folder.update(cx, |state, cx| {
                                                    state.create_folder(None, "new-folder".to_string(), cx)
                                                });
                                                if let Some(new_id) = created {
                                                    ws_folder.update(cx, |state, cx| {
                                                        state.start_renaming_folder(new_id, "new-folder".to_string(), window, cx);
                                                    });
                                                }
                                            }),
                                    )
                            })
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
                    .children({
                        let root_folders: Vec<_> = folders
                            .iter()
                            .filter(|f| f.parent_id.is_none() || !folders.iter().any(|p| Some(&p.id) == f.parent_id.as_ref()))
                            .collect();
                        root_folders.into_iter().filter_map(|folder| {
                            render_folder_node(
                                folder,
                                &folders,
                                &requests,
                                &query,
                                renaming_folder_id.as_deref(),
                                &rename_folder_input,
                                &collapsed_folders,
                                &ws_clone,
                                &ed_clone,
                                &render_req_item,
                                git_status.as_ref(),
                                &unsaved_requests,
                            )
                        })
                    })
                    // Top-level requests without folder
                    .children(
                        requests
                            .iter()
                            .filter(|r| {
                                (r.folder_id.is_none() || !folders.iter().any(|f| Some(&f.id) == r.folder_id.as_ref()))
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
                    })
            )
            // Footer at bottom of sidebar: Branch info on left, Pull button on right
            .child({
                let ws_footer = ws_clone.clone();
                let toast_footer = toast_clone.clone();
                let current_branch_name = git_branch.clone();

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
                                                    .child(git_branch),
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
                                                        .child(Icon::new(IconName::ArrowUp).size(px(10.)))
                                                        .child(div().text_xs().child(format!("{}", ahead))),
                                                )
                                            })
                                            .when(behind > 0, |s| {
                                                s.child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(1.))
                                                        .text_color(t.primary)
                                                        .child(Icon::new(IconName::ArrowDown).size(px(10.)))
                                                        .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child(format!("{}", behind))),
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
                                        restclient_core::git::git_branches(&ws.workspace_path).unwrap_or_default()
                                    };
                                    let current = ws.git_branch.clone().unwrap_or_else(|| active_b.clone());
                                    (branches, current)
                                };

                                let mut m = menu;

                                // 1. Commit...
                                m = m.item(
                                    PopupMenuItem::new("Commit...").icon(IconName::GitCommitHorizontal)
                                        .on_click(move |_, _, cx| {
                                            ws_commit.update(cx, |ws, cx| {
                                                ws.open_git_modal(GitActiveModal::Commit, cx);
                                            });
                                        })
                                );

                                // 2. History
                                m = m.item(
                                    PopupMenuItem::new("History").icon(IconName::Clock)
                                        .on_click(move |_, _, cx| {
                                            ws_history.update(cx, |ws, cx| {
                                                ws.open_git_modal(GitActiveModal::History, cx);
                                            });
                                        })
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
                                                PopupMenuItem::new(if current_b.is_empty() { "No branches".to_string() } else { current_b.clone() })
                                                    .checked(true)
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
                                                        })
                                                );
                                            }
                                        }
                                        s
                                    }
                                );

                                // 4. New Branch...
                                m = m.item(
                                    PopupMenuItem::new("New Branch...").icon(IconName::GitBranchPlus)
                                        .on_click(move |_, _, cx| {
                                            ws_new_branch.update(cx, |ws, cx| {
                                                ws.open_git_modal(GitActiveModal::NewBranch, cx);
                                            });
                                        })
                                );

                                m = m.separator();

                                // 5. Fetch
                                m = m.item(
                                    PopupMenuItem::new("Fetch").icon(IconName::CloudDownload)
                                        .on_click(move |_, _, cx| {
                                            let ws_s = ws_fetch.clone();
                                            let toast_s = toast_fetch.clone();
                                            ws_s.update(cx, |ws, cx| {
                                                match ws.git_fetch(cx) {
                                                    Ok(msg) => {
                                                        let trimmed = msg.trim();
                                                        toast_s.update(cx, |ts, cx| {
                                                            ts.push_success("Git Fetch", if trimmed.is_empty() { "Fetched latest references.".to_string() } else { trimmed.to_string() }, cx);
                                                        });
                                                    }
                                                    Err(e) => {
                                                        toast_s.update(cx, |ts, cx| {
                                                            ts.push_error("Git Fetch Failed", e, cx);
                                                        });
                                                    }
                                                }
                                            });
                                        })
                                );

                                // 6. Pull
                                m = m.item(
                                    PopupMenuItem::new("Pull").icon(IconName::ArrowDown)
                                        .on_click(move |_, _, cx| {
                                            let ws_s = ws_pull.clone();
                                            let toast_s = toast_pull.clone();
                                            ws_s.update(cx, |ws, cx| {
                                                match ws.git_pull(cx) {
                                                    Ok(msg) => {
                                                        let trimmed = msg.trim();
                                                        if trimmed.contains("Already up to date") {
                                                            toast_s.update(cx, |ts, cx| {
                                                                ts.push_info("Git Sync", "Already up to date.", cx);
                                                            });
                                                        } else {
                                                            toast_s.update(cx, |ts, cx| {
                                                                ts.push_success("Git Pull", format!("Successfully pulled: {}", trimmed), cx);
                                                            });
                                                        }
                                                    }
                                                    Err(e) => {
                                                        toast_s.update(cx, |ts, cx| {
                                                            ts.push_error("Git Pull Failed", e, cx);
                                                        });
                                                    }
                                                }
                                            });
                                        })
                                );

                                // 7. Push
                                m = m.item(
                                    PopupMenuItem::new("Push").icon(IconName::ArrowUp)
                                        .on_click(move |_, _, cx| {
                                            let ws_s = ws_push.clone();
                                            let toast_s = toast_push.clone();
                                            ws_s.update(cx, |ws, cx| {
                                                match ws.git_push(cx) {
                                                    Ok(msg) => {
                                                        let trimmed = msg.trim();
                                                        toast_s.update(cx, |ts, cx| {
                                                            ts.push_success("Git Push", if trimmed.is_empty() { "Pushed commits to remote.".to_string() } else { trimmed.to_string() }, cx);
                                                        });
                                                    }
                                                    Err(e) => {
                                                        toast_s.update(cx, |ts, cx| {
                                                            ts.push_error("Git Push Failed", e, cx);
                                                        });
                                                    }
                                                }
                                            });
                                        })
                                );

                                m = m.separator();

                                // 8. Manage Remotes...
                                m = m.item(
                                    PopupMenuItem::new("Manage Remotes...").icon(IconName::Globe)
                                        .on_click(move |_, _, cx| {
                                            ws_remotes.update(cx, |ws, cx| {
                                                ws.open_git_modal(GitActiveModal::Remotes, cx);
                                            });
                                        })
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
                            .child(
                                div()
                                    .text_color(t.text_muted)
                                    .child("No Git"),
                            )
                            .into_any_element()
                    })
            });

        res
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

fn folder_has_matches(
    folder_id: &str,
    folders: &[Folder],
    requests: &[SidebarReqItem],
    query: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    if let Some(f) = folders.iter().find(|f| f.id == folder_id) {
        if f.name.to_lowercase().contains(query) {
            return true;
        }
    }
    if requests.iter().any(|r| {
        r.folder_id.as_deref() == Some(folder_id)
            && (r.name.to_lowercase().contains(query)
                || r.url.to_lowercase().contains(query)
                || r.examples.iter().any(|e| e.name.to_lowercase().contains(query)))
    }) {
        return true;
    }
    for child in folders.iter().filter(|f| f.parent_id.as_deref() == Some(folder_id)) {
        if folder_has_matches(&child.id, folders, requests, query) {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarGitStatus {
    New,      // File mới (Untracked '??', Staged Added 'A') -> Green
    Modified, // File có thay đổi ('M', 'R', 'C', '!', or unsaved) -> Yellow
    Clean,    // Đã commit -> Normal (White)
}

pub fn calculate_request_git_status(
    req_id: &str,
    git_status: Option<&restclient_core::git::GitStatusResult>,
    is_unsaved: bool,
) -> SidebarGitStatus {
    if is_unsaved {
        return SidebarGitStatus::Modified;
    }

    let Some(git) = git_status else {
        return SidebarGitStatus::Clean;
    };

    if !git.is_repo {
        return SidebarGitStatus::Clean;
    }

    let norm_req = req_id.replace('\\', "/");
    let coll_rel = format!("collections/{}", norm_req);

    let matches = |p: &str| -> bool {
        let p = p.replace('\\', "/");
        p == norm_req
            || p == coll_rel
            || p.ends_with(&format!("/{}", norm_req))
            || p.ends_with(&format!("/{}", coll_rel))
    };

    // 1. Untracked (new file)
    if git.untracked_files.iter().any(|item| matches(&item.path)) {
        return SidebarGitStatus::New;
    }

    // 2. Staged
    if let Some(item) = git.staged_files.iter().find(|item| matches(&item.path)) {
        return match item.status {
            restclient_core::git::GitFileStatus::Added => SidebarGitStatus::New,
            _ => SidebarGitStatus::Modified,
        };
    }

    // 3. Unstaged (working tree changes)
    if git.unstaged_files.iter().any(|item| matches(&item.path)) {
        return SidebarGitStatus::Modified;
    }

    SidebarGitStatus::Clean
}

pub fn calculate_folder_git_status(
    folder_id: &str,
    folders: &[Folder],
    requests: &[SidebarReqItem],
    git_status: Option<&restclient_core::git::GitStatusResult>,
    unsaved_requests: &std::collections::HashSet<String>,
) -> SidebarGitStatus {
    let mut descendant_folders = std::collections::HashSet::new();
    descendant_folders.insert(folder_id.to_string());

    let mut changed = true;
    while changed {
        changed = false;
        for f in folders {
            if let Some(ref pid) = f.parent_id {
                if descendant_folders.contains(pid) && descendant_folders.insert(f.id.clone()) {
                    changed = true;
                }
            }
        }
    }

    let mut has_new = false;
    for req in requests {
        if let Some(ref fid) = req.folder_id {
            if descendant_folders.contains(fid) {
                let st = calculate_request_git_status(&req.id, git_status, unsaved_requests.contains(&req.id));
                if st == SidebarGitStatus::Modified {
                    return SidebarGitStatus::Modified;
                }
                if st == SidebarGitStatus::New {
                    has_new = true;
                }
            }
        }
    }

    if has_new {
        SidebarGitStatus::New
    } else {
        SidebarGitStatus::Clean
    }
}

#[allow(clippy::too_many_arguments)]
fn render_folder_node(
    folder: &Folder,
    folders: &[Folder],
    requests: &[SidebarReqItem],
    query: &str,
    renaming_folder_id: Option<&str>,
    rename_folder_input: &Option<Entity<gpui_component::input::InputState>>,
    collapsed_folders: &std::collections::HashSet<String>,
    ws_clone: &Entity<WorkspaceState>,
    ed_clone: &Entity<EditorState>,
    render_req_item: &dyn Fn(&SidebarReqItem, bool) -> AnyElement,
    git_status: Option<&restclient_core::git::GitStatusResult>,
    unsaved_requests: &std::collections::HashSet<String>,
) -> Option<AnyElement> {
    if !folder_has_matches(&folder.id, folders, requests, query) {
        return None;
    }

    let t = theme();
    let git_green: gpui::Hsla = gpui::rgb(0x4ade80).into();
    let git_yellow: gpui::Hsla = gpui::rgb(0xfacc15).into();

    let folder_git_status = calculate_folder_git_status(
        &folder.id,
        folders,
        requests,
        git_status,
        unsaved_requests,
    );
    let folder_color = match folder_git_status {
        SidebarGitStatus::New => git_green,
        SidebarGitStatus::Modified => git_yellow,
        SidebarGitStatus::Clean => t.text_primary,
    };

    let f_id = folder.id.clone();
    let is_renaming_folder = renaming_folder_id == Some(f_id.as_str());
    let is_collapsed = collapsed_folders.contains(&f_id) && query.is_empty();
    let folder_name = folder.name.clone();

    let folder_requests: Vec<_> = requests
        .iter()
        .filter(|r| {
            r.folder_id.as_deref() == Some(&f_id)
                && (query.is_empty()
                    || r.name.to_lowercase().contains(query)
                    || r.url.to_lowercase().contains(query)
                    || r.examples.iter().any(|e| e.name.to_lowercase().contains(query)))
        })
        .collect();

    let child_folders: Vec<_> = folders
        .iter()
        .filter(|f| f.parent_id.as_deref() == Some(&f_id))
        .collect();

    let drag_folder_payload = DragFolderItem {
        id: f_id.clone(),
        name: folder_name.clone(),
    };
    let ws_drop_req = ws_clone.clone();
    let f_id_target_req = f_id.clone();
    let ws_drop_folder = ws_clone.clone();
    let f_id_target_folder = f_id.clone();

    let node = div()
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

                    let ws_new_sub = ws_clone.clone();
                    let f_id_new_sub = f_id.clone();

                    let ws_ren = ws_clone.clone();
                    let f_id_ren = f_id.clone();
                    let f_name_ren = folder_name.clone();

                    let ws_del = ws_clone.clone();
                    let f_id_del = f_id.clone();

                    let ws_settings = ws_clone.clone();
                    let f_id_settings = f_id.clone();

                    move |menu, _window, _cx| {
                        let ws_n = ws_new.clone();
                        let ed_n = ed_new.clone();
                        let f_id_n = f_id_new.clone();

                        let ws_ns = ws_new_sub.clone();
                        let f_id_ns = f_id_new_sub.clone();

                        let ws_r = ws_ren.clone();
                        let f_id_r = f_id_ren.clone();
                        let f_name_r = f_name_ren.clone();

                        let ws_set = ws_settings.clone();
                        let f_id_set = f_id_settings.clone();

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
                                gpui_component::menu::PopupMenuItem::new("New Folder")
                                    .on_click(move |_, window, cx| {
                                        let created = ws_ns.update(cx, |state, cx| {
                                            state.create_folder(Some(f_id_ns.clone()), "new-folder".to_string(), cx)
                                        });
                                        if let Some(new_id) = created {
                                            ws_ns.update(cx, |state, cx| {
                                                state.start_renaming_folder(new_id, "new-folder".to_string(), window, cx);
                                            });
                                        }
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
                            .item(
                                gpui_component::menu::PopupMenuItem::new("Folder Settings...")
                                    .icon(IconName::Settings)
                                    .on_click(move |_, _, cx| {
                                        ws_set.update(cx, |state, cx| {
                                            state.open_folder_settings(f_id_set.clone(), cx);
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
                                .w(px(14.))
                                .h(px(14.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(t.text_muted)
                                .child(
                                    Icon::new(if is_collapsed {
                                        IconName::ChevronRight
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .size(px(11.))
                                ),
                        )
                        .child(
                            Icon::new(if is_collapsed {
                                IconName::FolderClosed
                            } else {
                                IconName::FolderOpen
                            })
                            .size(px(14.))
                            .text_color(folder_color),
                        )
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
                                .text_color(folder_color)
                                .truncate()
                                .child(folder.name.clone())
                                .into_any_element()
                        }),
                )
                .when(folder_git_status != SidebarGitStatus::Clean, |this| {
                    match folder_git_status {
                        SidebarGitStatus::New => this.child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(git_green)
                                .child("U"),
                        ),
                        SidebarGitStatus::Modified => this.child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(git_yellow)
                                .child("M"),
                        ),
                        SidebarGitStatus::Clean => this,
                    }
                }),
        )
        // Folder items
        .when(!is_collapsed, |this| {
            let mut child_elements = Vec::new();

            for child_folder in child_folders {
                if let Some(child_el) = render_folder_node(
                    child_folder,
                    folders,
                    requests,
                    query,
                    renaming_folder_id,
                    rename_folder_input,
                    collapsed_folders,
                    ws_clone,
                    ed_clone,
                    render_req_item,
                    git_status,
                    unsaved_requests,
                ) {
                    child_elements.push(child_el);
                }
            }

            for req in folder_requests {
                child_elements.push(render_req_item(req, false));
            }

            if !child_elements.is_empty() {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .pl(px(14.))
                        .children(child_elements)
                )
            } else {
                this
            }
        });

    Some(node.into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;
    use restclient_core::git::{GitFileStatus, GitStatusItem, GitStatusResult};
    use restclient_core::models::HttpMethod;
    use std::collections::HashSet;

    #[test]
    fn test_calculate_request_git_status_untracked_is_new() {
        let status = GitStatusResult {
            is_repo: true,
            untracked_files: vec![GitStatusItem {
                path: "collections/users/get.yaml".to_string(),
                status: GitFileStatus::Untracked,
                staged: false,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("users/get.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::New);
    }

    #[test]
    fn test_calculate_request_git_status_staged_added_is_new() {
        let status = GitStatusResult {
            is_repo: true,
            staged_files: vec![GitStatusItem {
                path: "collections/login.yaml".to_string(),
                status: GitFileStatus::Added,
                staged: true,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("login.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::New);
    }

    #[test]
    fn test_calculate_request_git_status_unstaged_modified_is_modified() {
        let status = GitStatusResult {
            is_repo: true,
            unstaged_files: vec![GitStatusItem {
                path: "collections/auth/login.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: false,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("auth/login.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Modified);
    }

    #[test]
    fn test_calculate_request_git_status_staged_modified_is_modified() {
        let status = GitStatusResult {
            is_repo: true,
            staged_files: vec![GitStatusItem {
                path: "collections/checkout.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: true,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("checkout.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Modified);
    }

    #[test]
    fn test_calculate_request_git_status_unsaved_is_modified() {
        let status = GitStatusResult {
            is_repo: true,
            ..Default::default()
        };

        let result = calculate_request_git_status("req1.yaml", Some(&status), true);
        assert_eq!(result, SidebarGitStatus::Modified);
    }

    #[test]
    fn test_calculate_request_git_status_clean_is_clean() {
        let status = GitStatusResult {
            is_repo: true,
            staged_files: vec![GitStatusItem {
                path: "collections/other.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: true,
            }],
            ..Default::default()
        };

        let result = calculate_request_git_status("req_clean.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Clean);
    }

    #[test]
    fn test_calculate_request_git_status_non_repo_is_clean() {
        let status = GitStatusResult {
            is_repo: false,
            ..Default::default()
        };

        let result = calculate_request_git_status("req.yaml", Some(&status), false);
        assert_eq!(result, SidebarGitStatus::Clean);
    }

    #[test]
    fn test_calculate_folder_git_status() {
        let folders = vec![
            Folder {
                id: "auth".to_string(),
                workspace_id: "ws".to_string(),
                parent_id: None,
                name: "Auth".to_string(),
                sort_order: 0,
                headers: Vec::new(),
                auth: restclient_core::models::Auth::Inherit,
            },
            Folder {
                id: "auth/sub".to_string(),
                workspace_id: "ws".to_string(),
                parent_id: Some("auth".to_string()),
                name: "Sub".to_string(),
                sort_order: 0,
                headers: Vec::new(),
                auth: restclient_core::models::Auth::Inherit,
            },
        ];

        let requests = vec![
            SidebarReqItem {
                id: "auth/sub/token.yaml".to_string(),
                folder_id: Some("auth/sub".to_string()),
                name: "Token".to_string(),
                display_name: "Token".to_string(),
                method: HttpMethod::Post,
                url: "".to_string(),
                last_status_code: None,
                examples: vec![],
            },
            SidebarReqItem {
                id: "auth/login.yaml".to_string(),
                folder_id: Some("auth".to_string()),
                name: "Login".to_string(),
                display_name: "Login".to_string(),
                method: HttpMethod::Post,
                url: "".to_string(),
                last_status_code: None,
                examples: vec![],
            },
        ];

        let unsaved = HashSet::new();

        // 1. When child is modified, parent folder is modified
        let status_mod = GitStatusResult {
            is_repo: true,
            unstaged_files: vec![GitStatusItem {
                path: "collections/auth/sub/token.yaml".to_string(),
                status: GitFileStatus::Modified,
                staged: false,
            }],
            ..Default::default()
        };
        assert_eq!(
            calculate_folder_git_status("auth", &folders, &requests, Some(&status_mod), &unsaved),
            SidebarGitStatus::Modified
        );

        // 2. When child is new, parent folder is new
        let status_new = GitStatusResult {
            is_repo: true,
            untracked_files: vec![GitStatusItem {
                path: "collections/auth/login.yaml".to_string(),
                status: GitFileStatus::Untracked,
                staged: false,
            }],
            ..Default::default()
        };
        assert_eq!(
            calculate_folder_git_status("auth", &folders, &requests, Some(&status_new), &unsaved),
            SidebarGitStatus::New
        );

        // 3. When all clean, folder is clean
        let status_clean = GitStatusResult {
            is_repo: true,
            ..Default::default()
        };
        assert_eq!(
            calculate_folder_git_status("auth", &folders, &requests, Some(&status_clean), &unsaved),
            SidebarGitStatus::Clean
        );
    }
}
